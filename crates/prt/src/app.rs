use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use prt_core::core::alerts::{self, AlertAction, FiredAlert};
use prt_core::core::firewall;
use prt_core::core::namespace::NetNamespace;
use prt_core::core::process_detail::ProcessDetail;
use prt_core::core::{killer, namespace, process_detail, session::Session};
use prt_core::i18n;
use prt_core::model::{DetailTab, EntryStatus, TrackedEntry, ViewMode, TICK_RATE};

use crate::forward::ForwardManager;
use crate::tracer::StraceSession;
use ratatui::prelude::*;
use std::io::stdout;
use std::time::Instant;

use crate::input::handle_key;
use crate::ui::draw;

pub struct App {
    pub session: Session,
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub filter: String,
    pub filter_mode: bool,
    pub show_help: bool,
    pub show_details: bool,
    pub detail_tab: DetailTab,
    /// Main view mode: Table, Chart, Topology, ProcessDetail, Namespaces.
    pub view_mode: ViewMode,
    pub confirm_kill: Option<(u32, String)>,
    pub sudo_prompt: bool,
    pub sudo_password: String,
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
    /// Cached namespace data. Refreshed each scan cycle.
    pub namespace_cache: Vec<(NetNamespace, Vec<u32>)>,
    /// Scroll offset for fullscreen views (Chart, Topology, Namespaces).
    pub scroll_offset: u16,
}

impl App {
    pub fn new() -> Self {
        Self {
            session: Session::new(),
            filtered_indices: Vec::new(),
            selected: 0,
            filter: String::new(),
            filter_mode: false,
            show_help: false,
            show_details: true,
            detail_tab: DetailTab::Tree,
            view_mode: ViewMode::default(),
            confirm_kill: None,
            sudo_prompt: false,
            sudo_password: String::new(),
            status_msg: None,
            should_quit: false,
            active_alerts: Vec::new(),
            confirm_block: None,
            forwards: ForwardManager::new(),
            forward_prompt: false,
            forward_input: String::new(),
            tracer: None,
            detail_cache: None,
            namespace_cache: Vec::new(),
            scroll_offset: 0,
        }
    }

    pub fn refresh(&mut self) {
        if let Err(msg) = self.session.refresh() {
            self.set_status(msg);
        }
        // Evaluate alert rules
        self.active_alerts = alerts::evaluate(&self.session.config.alerts, &self.session.entries);
        // Refresh caches
        self.refresh_namespace_cache();
        // Invalidate detail cache to pick up fresh data
        self.detail_cache = None;
        self.update_filtered();
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
        // Remember which entry was focused before the update
        let prev_key = self.selected_key();

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
        if let Some((ip, _)) = self.confirm_block.take() {
            let sudo_pw = self.session.sudo_password();
            match firewall::execute_block(ip, sudo_pw) {
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

    /// Refresh namespace cache (called once per refresh cycle, not per frame).
    fn refresh_namespace_cache(&mut self) {
        let pids: Vec<u32> = self
            .session
            .entries
            .iter()
            .filter(|e| e.status != EntryStatus::Gone)
            .map(|e| e.entry.process.pid)
            .collect();

        let ns_map = namespace::resolve_namespaces(&pids);
        self.namespace_cache = namespace::group_by_namespace(&ns_map);
    }

    pub fn try_sudo(&mut self) {
        let password = std::mem::take(&mut self.sudo_password);
        self.sudo_prompt = false;
        let msg = self.session.try_sudo(&password);
        self.set_status(msg);
        self.update_filtered();
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
        // Populate detail cache if ProcessDetail view is visible (avoids per-frame fetch)
        if app.view_mode == ViewMode::ProcessDetail {
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

        // Cleanup dead tunnels
        app.forwards.cleanup();

        if last_tick.elapsed() >= TICK_RATE {
            app.refresh();
            // Bell on alert (BEL char to terminal)
            if app.should_bell() {
                print!("\x07");
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
}
