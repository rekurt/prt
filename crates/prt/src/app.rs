use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use prt_core::config;
use prt_core::core::alerts::{self, AlertAction, FiredAlert};
use prt_core::core::firewall;
use prt_core::core::process_detail::ProcessDetail;
use prt_core::core::ssh_config::{self, SshHost};
use prt_core::core::ssh_tunnel::SshTunnelSpec;
use prt_core::core::{killer, process_detail, session::Session};
use prt_core::i18n;
use prt_core::model::{ConnectionState, ProcessesTab, SshTab, TrackedEntry, ViewMode, TICK_RATE};

use crate::forward::ForwardManager;
use crate::tracer::StraceSession;
use crate::views::action_menu::ActionMenu;
use crate::views::command_palette::CommandPalette;
use crate::views::tunnel_form::TunnelFormState;
use ratatui::prelude::*;
use std::collections::HashSet;
use std::io::stdout;
use std::time::Instant;

use crate::input::handle_key;
use crate::ui::draw;

#[derive(Clone, Copy)]
pub(crate) enum SudoPurpose {
    Refresh,
    Block(std::net::IpAddr),
}

pub struct App {
    pub session: Session,
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub filter: String,
    pub filter_mode: bool,
    pub show_help: bool,
    pub show_details: bool,
    pub auto_refresh_paused: bool,
    /// Top-level section.
    pub view_mode: ViewMode,
    /// Sub-tab inside the Processes section.
    pub processes_tab: ProcessesTab,
    /// Sub-tab inside the SSH section.
    pub ssh_tab: SshTab,
    pub confirm_kill: Option<(u32, String)>,
    pub sudo_prompt: bool,
    pub sudo_password: String,
    sudo_purpose: SudoPurpose,
    pub status_msg: Option<(String, Instant)>,
    pub should_quit: bool,
    /// Alert results from the last refresh cycle.
    pub active_alerts: Vec<FiredAlert>,
    /// Firewall block confirmation: (IP, block command string).
    pub confirm_block: Option<(std::net::IpAddr, String)>,
    /// SSH tunnel manager.
    pub forwards: ForwardManager,
    /// Forward dialog: user is typing "host:port" to create SSH tunnel.
    pub forward_prompt: bool,
    /// Forward dialog input buffer: "user@host:port" or "host:port".
    pub forward_input: String,
    /// Active strace/dtruss session.
    pub tracer: Option<StraceSession>,
    /// Cached process detail (PID → detail). Refreshed on PID change or refresh.
    pub detail_cache: Option<(u32, ProcessDetail)>,
    /// Scroll offset for fullscreen views (Topology).
    pub scroll_offset: u16,
    /// Saved SSH hosts (parsed from `~/.ssh/config` + prt config).
    pub ssh_hosts: Vec<SshHost>,
    /// Selected index in the SSH Hosts view.
    pub ssh_hosts_selected: usize,
    /// Selected index in the Tunnels view.
    pub tunnels_selected: usize,
    /// Active "new tunnel" form, if any.
    pub tunnel_form: Option<TunnelFormState>,
    /// Active action menu overlay (Space-key popup), if any.
    pub action_menu: Option<ActionMenu>,
    pub command_palette: Option<CommandPalette>,
    /// Timestamp of the last Esc press; used to arm the cascade
    /// (e.g. press Esc once to be warned, twice in <1.5s to clear filter).
    pub last_esc: Option<Instant>,
}

impl App {
    pub fn new() -> Self {
        let session = Session::new();
        let ssh_hosts = ssh_config::load_known_hosts(&session.config.ssh_hosts);
        let mut app = Self {
            session,
            filtered_indices: Vec::new(),
            selected: 0,
            filter: String::new(),
            filter_mode: false,
            show_help: false,
            show_details: true,
            auto_refresh_paused: false,
            view_mode: ViewMode::default(),
            processes_tab: ProcessesTab::default(),
            ssh_tab: SshTab::default(),
            confirm_kill: None,
            sudo_prompt: false,
            sudo_password: String::new(),
            sudo_purpose: SudoPurpose::Refresh,
            status_msg: None,
            should_quit: false,
            active_alerts: Vec::new(),
            confirm_block: None,
            forwards: ForwardManager::new(),
            forward_prompt: false,
            forward_input: String::new(),
            tracer: None,
            detail_cache: None,
            scroll_offset: 0,
            ssh_hosts,
            ssh_hosts_selected: 0,
            tunnels_selected: 0,
            tunnel_form: None,
            action_menu: None,
            command_palette: None,
            last_esc: None,
        };
        app.autostart_tunnels();
        app
    }

    /// Best-effort: spawn each tunnel listed in the loaded prt config.
    /// Failures are recorded as status messages but never abort startup.
    fn autostart_tunnels(&mut self) {
        let configs = self.session.config.ssh_tunnels.clone();
        if configs.is_empty() {
            return;
        }
        let mut started = 0usize;
        let mut failed = 0usize;
        for cfg in &configs {
            let spec = match cfg.to_spec() {
                Some(s) => s,
                None => {
                    failed += 1;
                    continue;
                }
            };
            let host = self.host_for_alias(&spec.host_alias).cloned();
            match self.forwards.add_spec_with_host(spec, host.as_ref()) {
                Ok(_) => started += 1,
                Err(_) => failed += 1,
            }
        }
        if started > 0 || failed > 0 {
            self.set_status(format!("tunnels: {started} started, {failed} failed"));
        }
    }

    /// Look up a known host by alias.
    fn host_for_alias(&self, alias: &str) -> Option<&SshHost> {
        self.ssh_hosts.iter().find(|h| h.alias == alias)
    }

    /// Reload SSH hosts: re-reads `~/.config/prt/config.toml` from disk so
    /// edits made while prt is running take effect, then merges with
    /// `~/.ssh/config`.
    pub fn reload_ssh_hosts(&mut self) {
        self.session.config = config::load_config();
        self.ssh_hosts = ssh_config::load_known_hosts(&self.session.config.ssh_hosts);
        if self.ssh_hosts_selected >= self.ssh_hosts.len() {
            self.ssh_hosts_selected = self.ssh_hosts.len().saturating_sub(1);
        }
        let s = i18n::strings();
        self.set_status(s.ssh_hosts_reloaded.into());
    }

    /// Open the new-tunnel form, optionally pre-filling the SSH host alias.
    pub fn open_tunnel_form(&mut self, prefill_alias: Option<String>) {
        self.tunnel_form = Some(match prefill_alias {
            Some(alias) => TunnelFormState::new_from_host(alias),
            None => TunnelFormState::new(None),
        });
    }

    /// Open the tunnel form pre-populated from an existing tunnel for editing.
    /// On submit, the form will replace the tunnel at `idx` (kill + spawn).
    pub fn open_tunnel_form_edit(&mut self, idx: usize) {
        if let Some(tunnel) = self.forwards.tunnels.get(idx) {
            self.tunnel_form = Some(TunnelFormState::edit(&tunnel.spec, idx));
        }
    }

    /// Replace the tunnel at `idx` with a new spec (kill old, spawn new).
    pub fn replace_tunnel(&mut self, idx: usize, spec: SshTunnelSpec) {
        let summary = spec.summary();
        let host = self.host_for_alias(&spec.host_alias).cloned();
        let s = i18n::strings();
        match self.forwards.replace_at(idx, spec, host.as_ref()) {
            Ok(()) => self.set_status(format!("tunnel: {summary}")),
            Err(e) => self.set_status(format!("{}: {e}", s.tunnel_create_failed)),
        }
    }

    /// Spawn a tunnel from a fully-validated spec.
    pub fn create_tunnel(&mut self, spec: SshTunnelSpec) {
        let summary = spec.summary();
        let host = self.host_for_alias(&spec.host_alias).cloned();
        match self.forwards.add_spec_with_host(spec, host.as_ref()) {
            Ok(_) => {
                self.tunnels_selected = self.forwards.tunnels.len().saturating_sub(1);
                self.set_status(format!("tunnel: {summary}"));
            }
            Err(e) => {
                let s = i18n::strings();
                self.set_status(format!("{}: {e}", s.tunnel_create_failed));
            }
        }
    }

    /// Clamp `tunnels_selected` into the current tunnel list. Called before
    /// dispatching kill/restart actions so async `cleanup()` can't leave the
    /// stored index pointing past the end.
    fn clamp_tunnels_selected(&mut self) -> Option<usize> {
        let count = self.forwards.tunnels.len();
        if count == 0 {
            self.tunnels_selected = 0;
            return None;
        }
        if self.tunnels_selected >= count {
            self.tunnels_selected = count - 1;
        }
        Some(self.tunnels_selected)
    }

    /// Kill the currently selected tunnel and adjust the selection.
    pub fn kill_selected_tunnel(&mut self) {
        let idx = match self.clamp_tunnels_selected() {
            Some(i) => i,
            None => return,
        };
        self.forwards.kill_at(idx);
        if self.tunnels_selected >= self.forwards.tunnels.len() {
            self.tunnels_selected = self.forwards.tunnels.len().saturating_sub(1);
        }
        let s = i18n::strings();
        self.set_status(s.tunnel_killed.into());
    }

    /// Restart the currently selected tunnel.
    pub fn restart_selected_tunnel(&mut self) {
        let idx = match self.clamp_tunnels_selected() {
            Some(i) => i,
            None => return,
        };
        let s = i18n::strings();
        match self.forwards.restart_at(idx) {
            Ok(()) => self.set_status(s.tunnel_restarted.into()),
            Err(e) => self.set_status(format!("{}: {e}", s.tunnel_create_failed)),
        }
    }

    /// Copy the `ssh` command line of the selected tunnel to the clipboard.
    pub fn copy_selected_tunnel_command(&mut self) {
        let idx = match self.clamp_tunnels_selected() {
            Some(i) => i,
            None => return,
        };
        let cmd = match self.forwards.tunnels.get(idx) {
            Some(t) => t.command_string(),
            None => return,
        };
        self.copy_to_clipboard(&cmd);
    }

    /// Persist the current set of active tunnels to the user's config file.
    /// Failed tunnels are pruned first so the on-disk config stays clean.
    pub fn save_tunnels(&mut self) {
        let path = match config::config_path() {
            Some(p) => p,
            None => {
                self.set_status("config path unavailable".into());
                return;
            }
        };
        self.forwards.drop_failed();
        let specs = self.forwards.specs();
        let s = i18n::strings();
        match config::write_tunnels(&path, &specs) {
            Ok(()) => self.set_status(format!("{} ({})", s.tunnels_saved, specs.len())),
            Err(e) => self.set_status(format!("save failed: {e}")),
        }
    }

    pub fn refresh(&mut self) {
        let prev_key = self.selected_key();
        if let Err(msg) = self.session.refresh() {
            self.set_status(msg);
        }
        // Evaluate alert rules
        self.active_alerts = alerts::evaluate(&self.session.config.alerts, &self.session.entries);
        if let Some((pid, _)) = self.detail_cache.as_ref() {
            if !self
                .session
                .entries
                .iter()
                .any(|entry| entry.entry.process.pid == *pid)
            {
                self.detail_cache = None;
            }
        }
        self.update_filtered_preserving(prev_key);
    }

    /// Whether any bell alerts fired this cycle (for TUI to emit BEL).
    pub fn should_bell(&self) -> bool {
        self.active_alerts
            .iter()
            .any(|a| a.action == AlertAction::Bell)
    }

    /// Whether the given entry index has a highlight alert.
    pub fn is_alert_highlighted(&self, entry_index: usize) -> bool {
        self.active_alerts
            .iter()
            .any(|a| a.entry_index == entry_index && a.action == AlertAction::Highlight)
    }

    /// Identity key of the currently selected entry: (port, pid).
    /// Used to restore focus after refresh/re-sort.
    fn selected_key(&self) -> Option<(u16, u32)> {
        self.selected_entry()
            .map(|e| (e.entry.local_port(), e.entry.process.pid))
    }

    pub fn update_filtered(&mut self) {
        let prev_key = self.selected_key();
        self.update_filtered_preserving(prev_key);
    }

    fn update_filtered_preserving(&mut self, prev_key: Option<(u16, u32)>) {
        self.filtered_indices = self.session.filtered_indices(&self.filter);

        // Try to restore focus to the same (port, pid) entry
        if let Some((port, pid)) = prev_key {
            if let Some(new_pos) = self.filtered_indices.iter().position(|&i| {
                let e = &self.session.entries[i];
                e.entry.local_port() == port && e.entry.process.pid == pid
            }) {
                self.selected = new_pos;
                return;
            }
        }

        // Entry disappeared or first run — clamp index
        if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len().saturating_sub(1);
        }
    }

    pub fn selected_entry(&self) -> Option<&TrackedEntry> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&i| self.session.entries.get(i))
    }

    pub fn kill_selected(&mut self, force: bool) {
        if let Some(entry) = self.selected_entry() {
            let pid = entry.entry.process.pid;
            let name = entry.entry.process.name.clone();
            let s = i18n::strings();
            match killer::kill_process(pid, force) {
                Ok(()) => {
                    let sig = if force { "SIGKILL" } else { "SIGTERM" };
                    self.set_status(s.fmt_kill_sent(sig, &name, pid));
                    self.confirm_kill = None;
                }
                Err(e) => {
                    self.set_status(s.fmt_kill_failed(&e.to_string()));
                    self.confirm_kill = None;
                }
            }
        }
    }

    pub fn copy_to_clipboard(&mut self, text: &str) {
        let s = i18n::strings();
        match arboard::Clipboard::new().and_then(|mut clip| clip.set_text(text)) {
            Ok(()) => self.set_status(s.copied.into()),
            Err(_) => self.set_status(s.clipboard_unavailable.into()),
        }
    }

    pub fn set_status(&mut self, msg: String) {
        self.status_msg = Some((msg, Instant::now()));
    }

    /// Initiate firewall block for the selected entry's remote IP.
    pub fn initiate_block(&mut self) {
        if let Some(entry) = self.selected_entry() {
            if let Some(remote) = entry.entry.remote_addr {
                let ip = remote.ip();
                let cmd = firewall::block_command(ip);
                self.confirm_block = Some((ip, cmd));
            } else {
                self.set_status("no remote address to block".into());
            }
        }
    }

    /// Execute the confirmed firewall block.
    pub fn execute_block(&mut self) {
        if let Some((ip, cmd)) = self.confirm_block.take() {
            if !self.session.is_root && !self.session.is_elevated {
                self.confirm_block = Some((ip, cmd));
                self.open_sudo_prompt(SudoPurpose::Block(ip));
                return;
            }

            match firewall::execute_block(ip, None) {
                Ok(()) => {
                    let undo = firewall::unblock_command(ip);
                    self.set_status(format!("blocked {ip} — undo: {undo}"));
                }
                Err(e) => self.set_status(format!("block failed: {e}")),
            }
        }
    }

    /// Create SSH forward from selected port using the input string.
    /// Input format: "host:port" or "user@host:port".
    /// The local port is the selected entry's port.
    pub fn create_forward(&mut self) {
        let input = std::mem::take(&mut self.forward_input);
        self.forward_prompt = false;

        // Parse input: "host:port" or "user@host:port"
        let (remote_host, remote_port) = match parse_forward_input(&input) {
            Some(parsed) => parsed,
            None => {
                self.set_status(format!("invalid format: use host:port — got '{input}'"));
                return;
            }
        };

        // Use selected entry's local port
        let local_port = match self.selected_entry() {
            Some(e) => e.entry.local_port(),
            None => {
                self.set_status("no port selected for forwarding".into());
                return;
            }
        };

        match self.forwards.add(local_port, &remote_host, remote_port) {
            Ok(_) => {
                self.set_status(format!(
                    "tunnel: localhost:{local_port} → {remote_host}:{remote_port} ({} active)",
                    self.forwards.count()
                ));
            }
            Err(e) => {
                self.set_status(format!("forward failed: {e}"));
            }
        }
    }

    /// Toggle strace attachment on the selected process.
    pub fn toggle_tracer(&mut self) {
        if self.tracer.is_some() {
            // Detach
            self.tracer = None;
            self.set_status("tracer detached".into());
        } else if let Some(entry) = self.selected_entry() {
            let pid = entry.entry.process.pid;
            match StraceSession::attach(pid) {
                Ok(session) => {
                    self.set_status(format!("tracing PID {pid}"));
                    self.tracer = Some(session);
                }
                Err(e) => self.set_status(format!("trace failed: {e}")),
            }
        }
    }

    /// Get process detail for the selected entry, using cache.
    /// Only re-fetches when the selected PID changes.
    pub fn get_process_detail(&mut self) -> Option<&ProcessDetail> {
        let pid = self.selected_entry()?.entry.process.pid;
        if self.detail_cache.as_ref().map(|(p, _)| *p) != Some(pid) {
            if let Some(detail) = process_detail::fetch(pid) {
                self.detail_cache = Some((pid, detail));
            } else {
                self.detail_cache = None;
                return None;
            }
        }
        self.detail_cache.as_ref().map(|(_, d)| d)
    }

    pub fn open_sudo_prompt(&mut self, purpose: SudoPurpose) {
        self.sudo_purpose = purpose;
        self.sudo_prompt = true;
        self.sudo_password.clear();
    }

    pub fn try_sudo(&mut self) {
        let password = std::mem::take(&mut self.sudo_password);
        self.sudo_prompt = false;
        match self.sudo_purpose {
            SudoPurpose::Refresh => {
                let msg = self.session.try_sudo(&password);
                self.set_status(msg);
                self.update_filtered();
            }
            SudoPurpose::Block(ip) => match firewall::execute_block(ip, Some(&password)) {
                Ok(()) => {
                    self.session.is_elevated = true;
                    self.session.is_root = true;
                    self.confirm_block = None;
                    let undo = firewall::unblock_command(ip);
                    self.set_status(format!("blocked {ip} — undo: {undo}"));
                }
                Err(e) => self.set_status(format!("block failed: {e}")),
            },
        }
    }
}

pub fn run() -> Result<()> {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = stdout().execute(LeaveAlternateScreen);
        original_hook(info);
    }));

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new();
    let mut last_tick = Instant::now();

    app.refresh();

    loop {
        // Populate detail cache when Processes/Detail view is visible (avoids per-frame fetch)
        if app.view_mode == ViewMode::Processes
            && app.processes_tab == prt_core::model::ProcessesTab::Detail
        {
            app.get_process_detail();
        }

        terminal.draw(|f| draw(f, &app))?;

        let timeout = TICK_RATE.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    app.should_quit = true;
                }
                handle_key(&mut app, key);
            }
        }

        // Poll tracer for new output
        if let Some(ref mut tracer) = app.tracer {
            tracer.poll();
            if !tracer.is_alive() {
                app.tracer = None;
                app.set_status("tracer process exited".into());
            }
        }

        // Refresh tunnel statuses from the latest scan (which local ports are
        // listening), then auto-reconnect any whose process died (with backoff,
        // so an unreachable host isn't hammered). When auto-refresh is paused
        // the scan is stale, so the listener health can't be trusted.
        let listening: HashSet<u16> = app
            .session
            .entries
            .iter()
            .filter(|e| e.entry.state == ConnectionState::Listen)
            .map(|e| e.entry.local_addr.port())
            .collect();
        app.forwards.cleanup(&listening, !app.auto_refresh_paused);
        app.forwards.reconnect_failed();

        if last_tick.elapsed() >= TICK_RATE {
            if !app.auto_refresh_paused {
                app.refresh();
                // Bell on alert (BEL char to terminal)
                if app.should_bell() {
                    print!("\x07");
                }
            }
            last_tick = Instant::now();
        }

        if app.should_quit {
            break;
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

/// Parse forward input: "host:port" or "user@host:port".
/// Returns (remote_host_string, remote_port).
fn parse_forward_input(input: &str) -> Option<(String, u16)> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    // Find the last ':' — port is always after it
    let colon_pos = input.rfind(':')?;
    let host_part = &input[..colon_pos];
    let port_part = &input[colon_pos + 1..];

    if host_part.is_empty() {
        return None;
    }

    let port: u16 = port_part.parse().ok()?;
    Some((host_part.to_string(), port))
}

#[cfg(test)]
mod tests {
    use super::*;
    use prt_core::model::{ConnectionState, EntryStatus, PortEntry, ProcessInfo, Protocol};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn tracked(port: u16, pid: u32, name: &str) -> TrackedEntry {
        TrackedEntry {
            entry: PortEntry {
                protocol: Protocol::Tcp,
                local_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
                remote_addr: None,
                state: ConnectionState::Listen,
                process: ProcessInfo {
                    pid,
                    name: name.into(),
                    path: None,
                    cmdline: None,
                    user: None,
                    parent_pid: None,
                    parent_name: None,
                },
            },
            status: EntryStatus::Unchanged,
            seen_at: Instant::now(),
            first_seen: None,
            suspicious: Vec::new(),
            container_name: None,
            service_name: None,
        }
    }

    #[test]
    fn parse_forward_simple_host_port() {
        let (host, port) = parse_forward_input("example.com:8080").unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(port, 8080);
    }

    #[test]
    fn parse_forward_user_at_host() {
        let (host, port) = parse_forward_input("user@server.io:22").unwrap();
        assert_eq!(host, "user@server.io");
        assert_eq!(port, 22);
    }

    #[test]
    fn parse_forward_ip_address() {
        let (host, port) = parse_forward_input("192.168.1.1:3000").unwrap();
        assert_eq!(host, "192.168.1.1");
        assert_eq!(port, 3000);
    }

    #[test]
    fn parse_forward_empty() {
        assert!(parse_forward_input("").is_none());
    }

    #[test]
    fn parse_forward_no_port() {
        assert!(parse_forward_input("host").is_none());
    }

    #[test]
    fn parse_forward_no_host() {
        assert!(parse_forward_input(":8080").is_none());
    }

    #[test]
    fn parse_forward_invalid_port() {
        assert!(parse_forward_input("host:abc").is_none());
        assert!(parse_forward_input("host:99999").is_none());
    }

    #[test]
    fn update_filtered_preserves_selection_from_pre_refresh_key() {
        let mut app = App::new();
        app.session.entries = vec![tracked(80, 1, "old-a"), tracked(443, 2, "old-b")];
        app.filtered_indices = vec![0, 1];
        app.selected = 1;

        let prev_key = app.selected_key();
        app.session.entries = vec![
            tracked(22, 3, "new-a"),
            tracked(80, 1, "old-a"),
            tracked(443, 2, "old-b"),
        ];

        app.update_filtered_preserving(prev_key);

        assert_eq!(app.selected, 2);
        assert_eq!(app.selected_entry().unwrap().entry.process.pid, 2);
    }
}
