//! SSH tunnel manager.
//!
//! Spawns and supervises `ssh -N -L`/`-D` tunnels from within the TUI.
//! Tunnels are killed on `Drop` to prevent orphaned `ssh` processes.

use prt_core::core::ssh_config::{SshHost, SshHostSource};
use prt_core::core::ssh_tunnel::{ResolvedHost, SshTunnelSpec, TunnelKind};
use std::collections::HashSet;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Lifecycle status of a tunnel, refreshed on each `cleanup()` tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TunnelStatus {
    #[default]
    Starting,
    Alive,
    /// `ssh` is running but its local port is not actually being listened on
    /// (e.g. a `-D`/`-L` bind failure because the port is already in use).
    /// Surfaced as a warning; deliberately *not* auto-reconnected because
    /// restarting `ssh` won't free a port another process holds.
    Unhealthy,
    Failed,
}

/// First delay before retrying a failed auto-reconnect tunnel.
const INITIAL_BACKOFF: Duration = Duration::from_secs(2);
/// Upper bound for the exponential reconnect backoff.
const MAX_BACKOFF: Duration = Duration::from_secs(60);
/// Minimum time a tunnel must stay `Alive` before its reconnect backoff is
/// considered recovered and reset. Must exceed `HEALTH_GRACE` and one scan.
const STABLE_THRESHOLD: Duration = Duration::from_secs(30);
/// Grace period after (re)spawn during which we report `Starting` rather than
/// judging the listener — gives `ssh` time to bind and the scan to observe it.
/// Kept above `TICK_RATE` (2s) so at least one scan happens first.
const HEALTH_GRACE: Duration = Duration::from_secs(3);
/// After this many reconnect attempts without stabilising, give up and mark the
/// tunnel permanently failed (`auto_reconnect = false`) so it can be pruned.
const MAX_RETRIES: u32 = 8;

/// Next exponential backoff step, capped at [`MAX_BACKOFF`].
fn next_backoff(cur: Duration) -> Duration {
    (cur * 2).min(MAX_BACKOFF)
}

/// Pure status decision from the inputs `refresh_health` gathers — split out so
/// it can be unit-tested without spawning a real `ssh` child.
///
/// * dead process            → `Failed`
/// * within the spawn grace  → `Starting` (don't judge the listener yet)
/// * scan not usable (paused/stale) → `Alive` (avoid a false `no listener`)
/// * listener present        → `Alive`
/// * listener absent         → `Unhealthy`
fn decide_status(
    alive_proc: bool,
    uptime: Duration,
    listener_ok: bool,
    scan_usable: bool,
) -> TunnelStatus {
    if !alive_proc {
        return TunnelStatus::Failed;
    }
    if uptime < HEALTH_GRACE {
        return TunnelStatus::Starting;
    }
    if !scan_usable {
        return TunnelStatus::Alive;
    }
    if listener_ok {
        TunnelStatus::Alive
    } else {
        TunnelStatus::Unhealthy
    }
}

/// A single SSH tunnel: a running `ssh` child process plus the spec and
/// resolved argument list (kept so `restart()` reuses the same resolution).
pub struct SshTunnel {
    pub spec: SshTunnelSpec,
    args: Vec<String>,
    child: Child,
    pub last_status: TunnelStatus,
    /// When the current `ssh` child was spawned — reset on `restart()`.
    started_at: Instant,
    /// Whether to auto-restart this tunnel after it fails on its own.
    /// Manual `kill_at` removes the tunnel entirely, so it never reconnects.
    pub auto_reconnect: bool,
    /// Current exponential backoff between reconnect attempts.
    retry_backoff: Duration,
    /// Earliest instant the next reconnect attempt may run. `None` once the
    /// tunnel is healthy or no retry has been scheduled yet.
    next_retry_at: Option<Instant>,
    /// Consecutive reconnect attempts since the tunnel was last stable.
    retry_count: u32,
}

impl SshTunnel {
    /// Assemble a freshly-spawned tunnel with default supervision state.
    fn from_parts(spec: SshTunnelSpec, args: Vec<String>, child: Child) -> Self {
        Self {
            spec,
            args,
            child,
            last_status: TunnelStatus::Starting,
            started_at: Instant::now(),
            auto_reconnect: true,
            retry_backoff: INITIAL_BACKOFF,
            next_retry_at: None,
            retry_count: 0,
        }
    }

    /// Spawn an `ssh` process for `spec` with no extra host resolution
    /// (relies on `~/.ssh/config` or DNS for the alias).
    pub fn spawn(spec: SshTunnelSpec) -> Result<Self, String> {
        spec.validate()?;
        let args = spec.ssh_args();
        let child = spawn_ssh_args(&args)?;
        Ok(Self::from_parts(spec, args, child))
    }

    /// Spawn an `ssh` process for `spec`. For aliases defined only in prt's
    /// `[[ssh_hosts]]` (which `ssh` itself doesn't know about), inject
    /// explicit `-l/-p/-i hostname` flags. For aliases parsed from
    /// `~/.ssh/config`, keep the alias as the positional target so that
    /// host-scoped directives (`ProxyJump`, `ProxyCommand`, `ForwardAgent`,
    /// etc.) are honoured by `ssh`.
    pub fn spawn_with_host(spec: SshTunnelSpec, host: Option<&SshHost>) -> Result<Self, String> {
        spec.validate()?;
        let args = match host {
            Some(h) if h.source == SshHostSource::PrtConfig => {
                spec.ssh_args_with(&resolved_from(h))
            }
            _ => spec.ssh_args(),
        };
        let child = spawn_ssh_args(&args)?;
        Ok(Self::from_parts(spec, args, child))
    }

    /// Backwards-compat shortcut: spawn a Local tunnel matching the legacy
    /// signature used by the F-key prompt.
    pub fn new(local_port: u16, remote_host: &str, remote_port: u16) -> Result<Self, String> {
        let spec = SshTunnelSpec {
            name: None,
            kind: TunnelKind::Local,
            local_port,
            remote_host: Some("localhost".into()),
            remote_port: Some(remote_port),
            host_alias: remote_host.to_string(),
        };
        Self::spawn(spec)
    }

    /// Human-readable summary.
    pub fn summary(&self) -> String {
        self.spec.summary()
    }

    /// Refresh `last_status` from the child process state and the latest scan.
    ///
    /// `listening` is the set of local ports currently observed in `LISTEN`;
    /// `scan_usable` is false when the scan is paused/stale and can't be
    /// trusted to judge the listener. On stabilisation (`Alive` for at least
    /// [`STABLE_THRESHOLD`]) the reconnect backoff is reset.
    pub fn refresh_health(&mut self, listening: &HashSet<u16>, scan_usable: bool) -> TunnelStatus {
        let alive_proc = matches!(self.child.try_wait(), Ok(None));
        let listener_ok = listening.contains(&self.spec.local_port);
        let new = decide_status(alive_proc, self.uptime(), listener_ok, scan_usable);

        if new == TunnelStatus::Alive && self.uptime() >= STABLE_THRESHOLD {
            // Recovered and stable: clear the reconnect penalty.
            self.retry_backoff = INITIAL_BACKOFF;
            self.retry_count = 0;
            self.next_retry_at = None;
        }
        self.last_status = new;
        new
    }

    /// Kill the tunnel (signal + reap).
    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    /// Kill and respawn using the previously resolved arg list (blocking).
    ///
    /// Used for user-initiated restarts: validates the spawn synchronously so
    /// an immediate failure is surfaced, and clears the reconnect penalty since
    /// the user explicitly asked to try again.
    pub fn restart(&mut self) -> Result<(), String> {
        self.kill();
        self.child = spawn_ssh_args(&self.args)?;
        self.last_status = TunnelStatus::Starting;
        self.started_at = Instant::now();
        self.retry_backoff = INITIAL_BACKOFF;
        self.retry_count = 0;
        self.next_retry_at = None;
        Ok(())
    }

    /// Like [`restart`] but non-blocking: skips the 150ms spawn validation so
    /// the auto-reconnect loop never stalls the UI thread. A spawn that dies
    /// immediately is caught by the next `refresh_health` tick. Backoff/retry
    /// bookkeeping is left to the caller (`reconnect_failed`).
    pub fn restart_async(&mut self) -> Result<(), String> {
        self.kill();
        self.child = spawn_ssh_args_nowait(&self.args)?;
        self.last_status = TunnelStatus::Starting;
        self.started_at = Instant::now();
        Ok(())
    }

    /// How long the current `ssh` child has been running.
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// The full `ssh` command line this tunnel was spawned with — handy for
    /// copying to the clipboard and reproducing the tunnel outside prt.
    /// Arguments are shell-quoted so paths with spaces survive a paste.
    pub fn command_string(&self) -> String {
        ssh_command_string(&self.args)
    }
}

/// Render `ssh <args...>` with each argument shell-quoted, so an identity path
/// like `/home/u/my keys/id` doesn't split when pasted into a shell.
fn ssh_command_string(args: &[String]) -> String {
    let mut cmd = String::from("ssh");
    for arg in args {
        cmd.push(' ');
        match shlex::try_quote(arg) {
            Ok(quoted) => cmd.push_str(&quoted),
            // try_quote only errors on interior NUL, which can't occur in our
            // args; fall back to the raw arg to stay infallible.
            Err(_) => cmd.push_str(arg),
        }
    }
    cmd
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        self.kill();
    }
}

fn resolved_from(h: &SshHost) -> ResolvedHost<'_> {
    ResolvedHost {
        hostname: h.hostname.as_deref(),
        user: h.user.as_deref(),
        port: h.port,
        identity_file: h.identity_file.as_deref().and_then(|p| p.to_str()),
    }
}

/// Spawn the `ssh` child without any synchronous validation. Non-blocking:
/// safe to call from the render thread (used by auto-reconnect).
fn spawn_ssh_args_nowait(args: &[String]) -> Result<Child, String> {
    Command::new("ssh")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to start ssh: {e}"))
}

fn spawn_ssh_args(args: &[String]) -> Result<Child, String> {
    let mut child = spawn_ssh_args_nowait(args)?;

    // Quick validation: if ssh exits immediately, surface stderr as an error.
    thread::sleep(Duration::from_millis(150));
    if let Ok(Some(status)) = child.try_wait() {
        use std::io::Read;
        let mut stderr = String::new();
        if let Some(mut err) = child.stderr.take() {
            let _ = err.read_to_string(&mut stderr);
        }
        let stderr = stderr.trim();
        let details = if stderr.is_empty() {
            format!("ssh exited with status {status}")
        } else {
            stderr.to_string()
        };
        return Err(format!("failed to establish ssh tunnel: {details}"));
    }
    Ok(child)
}

/// Manages multiple SSH tunnels.
pub struct ForwardManager {
    pub tunnels: Vec<SshTunnel>,
}

impl Default for ForwardManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ForwardManager {
    pub fn new() -> Self {
        Self {
            tunnels: Vec::new(),
        }
    }

    /// Backwards-compat: add a Local tunnel (used by the F-key prompt).
    pub fn add(
        &mut self,
        local_port: u16,
        remote_host: &str,
        remote_port: u16,
    ) -> Result<usize, String> {
        let tunnel = SshTunnel::new(local_port, remote_host, remote_port)?;
        self.tunnels.push(tunnel);
        Ok(self.tunnels.len() - 1)
    }

    /// Add a tunnel from a spec, optionally resolving extra connection
    /// settings (`hostname`, `user`, `port`, `identity_file`) from a known host.
    pub fn add_spec_with_host(
        &mut self,
        spec: SshTunnelSpec,
        host: Option<&SshHost>,
    ) -> Result<usize, String> {
        let tunnel = SshTunnel::spawn_with_host(spec, host)?;
        self.tunnels.push(tunnel);
        Ok(self.tunnels.len() - 1)
    }

    /// Refresh each tunnel's status from the latest scan. `listening` is the
    /// set of local ports currently in `LISTEN`; `scan_usable` is false when
    /// the scan is paused/stale. Dead tunnels stay in the list (as `Failed`)
    /// so the user can see what happened and restart or remove them.
    pub fn cleanup(&mut self, listening: &HashSet<u16>, scan_usable: bool) {
        for tunnel in &mut self.tunnels {
            tunnel.refresh_health(listening, scan_usable);
        }
    }

    /// Auto-restart tunnels whose process died on their own, with exponential
    /// backoff so an unreachable host isn't hammered. Call after `cleanup()`.
    /// Returns the number of restart attempts that spawned this tick.
    ///
    /// Backoff grows on *every* attempt (not just hard failures), so a host
    /// that accepts the connection but drops it seconds later still backs off.
    /// The penalty is cleared only once a tunnel stays `Alive` for
    /// [`STABLE_THRESHOLD`] (handled in `refresh_health`). After [`MAX_RETRIES`]
    /// attempts without recovery the tunnel is marked permanently failed
    /// (`auto_reconnect = false`) so it stops retrying and can be pruned.
    ///
    /// Only `Failed` (dead process) is reconnected — `Unhealthy` is left alone
    /// because restarting `ssh` can't free a port another process holds.
    pub fn reconnect_failed(&mut self) -> usize {
        let now = Instant::now();
        let mut reconnected = 0;
        for tunnel in &mut self.tunnels {
            if tunnel.last_status != TunnelStatus::Failed || !tunnel.auto_reconnect {
                continue;
            }
            match tunnel.next_retry_at {
                // No attempt scheduled yet: schedule the first one and wait.
                None => tunnel.next_retry_at = Some(now + tunnel.retry_backoff),
                // Scheduled but not yet due: keep waiting.
                Some(at) if at > now => {}
                // Due: attempt a non-blocking restart and grow the backoff.
                Some(_) => {
                    tunnel.retry_count += 1;
                    tunnel.retry_backoff = next_backoff(tunnel.retry_backoff);
                    tunnel.next_retry_at = Some(now + tunnel.retry_backoff);
                    if tunnel.restart_async().is_ok() {
                        reconnected += 1;
                    }
                    if tunnel.retry_count >= MAX_RETRIES {
                        tunnel.auto_reconnect = false;
                    }
                }
            }
        }
        reconnected
    }

    /// Drop tunnels that are permanently dead. A `Failed` tunnel that is still
    /// auto-reconnecting is kept (it's live config the user expects to persist);
    /// only those that exhausted their retries (`auto_reconnect == false`) are
    /// pruned. Called before `save_tunnels` persists the surviving specs.
    pub fn drop_failed(&mut self) {
        // Keep everything except permanently-dead tunnels (failed *and* no
        // longer auto-reconnecting).
        self.tunnels
            .retain(|t| t.last_status != TunnelStatus::Failed || t.auto_reconnect);
    }

    /// Kill the tunnel at `idx`. No-op if out of bounds.
    pub fn kill_at(&mut self, idx: usize) {
        if idx < self.tunnels.len() {
            self.tunnels[idx].kill();
            self.tunnels.remove(idx);
        }
    }

    /// Replace the tunnel at `idx` with a fresh spec (kills + respawns).
    /// Used by the form's edit-mode.
    pub fn replace_at(
        &mut self,
        idx: usize,
        spec: SshTunnelSpec,
        host: Option<&SshHost>,
    ) -> Result<(), String> {
        if idx >= self.tunnels.len() {
            return Err("no such tunnel".into());
        }
        let new_tunnel = SshTunnel::spawn_with_host(spec, host)?;
        // Old tunnel is killed via Drop when replaced.
        self.tunnels[idx] = new_tunnel;
        Ok(())
    }

    /// Restart the tunnel at `idx`.
    pub fn restart_at(&mut self, idx: usize) -> Result<(), String> {
        self.tunnels
            .get_mut(idx)
            .ok_or_else(|| "no such tunnel".to_string())?
            .restart()
    }

    /// Kill all tunnels.
    pub fn kill_all(&mut self) {
        for tunnel in &mut self.tunnels {
            tunnel.kill();
        }
        self.tunnels.clear();
    }

    /// Number of tunnels.
    pub fn count(&self) -> usize {
        self.tunnels.len()
    }

    /// List summaries of all tunnels.
    pub fn summaries(&self) -> Vec<String> {
        self.tunnels.iter().map(|t| t.summary()).collect()
    }

    /// Snapshot specs for persistence.
    pub fn specs(&self) -> Vec<SshTunnelSpec> {
        self.tunnels.iter().map(|t| t.spec.clone()).collect()
    }
}

impl Drop for ForwardManager {
    fn drop(&mut self) {
        self.kill_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_manager_new_is_empty() {
        let fm = ForwardManager::new();
        assert_eq!(fm.count(), 0);
    }

    #[test]
    fn forward_manager_default_is_empty() {
        let fm = ForwardManager::default();
        assert_eq!(fm.count(), 0);
    }

    #[test]
    fn specs_snapshot_is_empty_when_no_tunnels() {
        let fm = ForwardManager::new();
        assert!(fm.specs().is_empty());
    }

    // Tunnel creation tests require an actual SSH server, so we only test
    // the manager state logic here. The spec-level tests cover argument
    // generation in `prt_core::core::ssh_tunnel`.

    // ── decide_status (pure) ─────────────────────────────────────

    #[test]
    fn decide_status_dead_process_is_failed() {
        let s = decide_status(false, Duration::from_secs(100), true, true);
        assert_eq!(s, TunnelStatus::Failed);
    }

    #[test]
    fn decide_status_within_grace_is_starting() {
        let s = decide_status(true, Duration::from_secs(1), false, true);
        assert_eq!(s, TunnelStatus::Starting);
    }

    #[test]
    fn decide_status_paused_scan_assumes_alive() {
        // No usable scan → don't emit a false "no listener".
        let s = decide_status(true, Duration::from_secs(100), false, false);
        assert_eq!(s, TunnelStatus::Alive);
    }

    #[test]
    fn decide_status_listener_present_is_alive() {
        let s = decide_status(true, Duration::from_secs(100), true, true);
        assert_eq!(s, TunnelStatus::Alive);
    }

    #[test]
    fn decide_status_listener_absent_is_unhealthy() {
        let s = decide_status(true, Duration::from_secs(100), false, true);
        assert_eq!(s, TunnelStatus::Unhealthy);
    }

    // ── next_backoff (pure) ──────────────────────────────────────

    #[test]
    fn next_backoff_doubles_then_caps() {
        assert_eq!(next_backoff(INITIAL_BACKOFF), Duration::from_secs(4));
        assert_eq!(next_backoff(Duration::from_secs(4)), Duration::from_secs(8));
        assert_eq!(next_backoff(Duration::from_secs(32)), MAX_BACKOFF);
        // Never exceeds the cap.
        assert_eq!(next_backoff(MAX_BACKOFF), MAX_BACKOFF);
    }

    // ── ssh_command_string (pure) ────────────────────────────────

    #[test]
    fn command_string_quotes_spaced_args() {
        let args = vec![
            "-N".to_string(),
            "-i".to_string(),
            "/home/u/my keys/id_rsa".to_string(),
            "prod".to_string(),
        ];
        let cmd = ssh_command_string(&args);
        // The spaced path must be quoted so a paste stays one argument.
        assert!(cmd.contains("'/home/u/my keys/id_rsa'"));
        assert!(cmd.starts_with("ssh -N -i "));
        assert!(cmd.ends_with(" prod"));
    }
}
