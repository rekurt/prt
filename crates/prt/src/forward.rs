//! SSH tunnel manager.
//!
//! Spawns and supervises `ssh -N -L`/`-D` tunnels from within the TUI.
//! Tunnels are killed on `Drop` to prevent orphaned `ssh` processes.

use prt_core::core::ssh_config::{SshHost, SshHostSource};
use prt_core::core::ssh_tunnel::{ResolvedHost, SshTunnelSpec, TunnelKind};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Lifecycle status of a tunnel, refreshed on each `cleanup()` tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TunnelStatus {
    #[default]
    Starting,
    Alive,
    Failed,
}

/// First delay before retrying a failed auto-reconnect tunnel.
const INITIAL_BACKOFF: Duration = Duration::from_secs(2);
/// Upper bound for the exponential reconnect backoff.
const MAX_BACKOFF: Duration = Duration::from_secs(60);
/// How long a tunnel must stay `Alive` before it's considered genuinely
/// recovered and its backoff is reset. Without this, a tunnel to an
/// unreachable host (where `ssh` survives the brief spawn check but dies a few
/// seconds later on TCP timeout) would have its backoff reset on every respawn,
/// defeating the exponential growth and hammering the host every couple of
/// seconds. The reset therefore lives in `refresh_status`, gated on uptime —
/// not in the respawn path.
const STABILITY_THRESHOLD: Duration = Duration::from_secs(30);
/// Give up auto-reconnecting after this many consecutive failed attempts so a
/// permanently unreachable host isn't retried forever. The tunnel stays
/// `Failed` (and `auto_reconnect` flips to `false`), which lets `save`/prune
/// remove it and lets the user restart it manually.
const MAX_RECONNECT_ATTEMPTS: u32 = 10;

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
    /// Flipped to `false` once `MAX_RECONNECT_ATTEMPTS` is exhausted.
    pub auto_reconnect: bool,
    /// Current exponential backoff between reconnect attempts.
    retry_backoff: Duration,
    /// Consecutive failed reconnect attempts; reset once the tunnel is stably
    /// `Alive`. Drives the give-up at `MAX_RECONNECT_ATTEMPTS`.
    retry_count: u32,
    /// Earliest instant the next reconnect attempt may run. `None` once the
    /// tunnel is healthy or no retry has been scheduled yet.
    next_retry_at: Option<Instant>,
}

impl SshTunnel {
    /// Assemble a tunnel from a freshly spawned child. Single place that seeds
    /// the reconnect/uptime bookkeeping so new fields only need adding once.
    fn from_child(spec: SshTunnelSpec, args: Vec<String>, child: Child) -> Self {
        Self {
            spec,
            args,
            child,
            last_status: TunnelStatus::Starting,
            started_at: Instant::now(),
            auto_reconnect: true,
            retry_backoff: INITIAL_BACKOFF,
            retry_count: 0,
            next_retry_at: None,
        }
    }

    /// Spawn an `ssh` process for `spec` with no extra host resolution
    /// (relies on `~/.ssh/config` or DNS for the alias).
    pub fn spawn(spec: SshTunnelSpec) -> Result<Self, String> {
        spec.validate()?;
        let args = spec.ssh_args();
        let child = spawn_ssh_args(&args)?;
        Ok(Self::from_child(spec, args, child))
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
        Ok(Self::from_child(spec, args, child))
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

    /// Refresh `last_status` based on the child process state.
    pub fn refresh_status(&mut self) -> TunnelStatus {
        let new = match self.child.try_wait() {
            Ok(None) => match self.last_status {
                TunnelStatus::Starting => {
                    // After surviving the 150ms spawn check + at least one tick,
                    // promote to Alive.
                    TunnelStatus::Alive
                }
                TunnelStatus::Alive => {
                    // Only a sustained `Alive` period proves the tunnel really
                    // came up (an unreachable host keeps `ssh` alive for a few
                    // seconds before TCP timeout). Reset the backoff here rather
                    // than on respawn so the exponential growth survives a host
                    // that flaps every few seconds. Also re-arm `auto_reconnect`:
                    // if the give-up at `MAX_RECONNECT_ATTEMPTS` happened to land
                    // on an attempt that actually recovered, a now-healthy tunnel
                    // must be eligible to reconnect again if it later drops.
                    if self.started_at.elapsed() >= STABILITY_THRESHOLD {
                        self.retry_backoff = INITIAL_BACKOFF;
                        self.retry_count = 0;
                        self.next_retry_at = None;
                        self.auto_reconnect = true;
                    }
                    TunnelStatus::Alive
                }
                // `Failed` with a still-running child shouldn't happen.
                other => other,
            },
            Ok(Some(_)) => TunnelStatus::Failed,
            Err(_) => TunnelStatus::Failed,
        };
        self.last_status = new;
        new
    }

    /// Kill the tunnel (signal + reap).
    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    /// Kill the current child and spawn a fresh one with the same arg list.
    /// `validate` runs the brief blocking liveness check (good for interactive
    /// restart, which wants immediate error feedback); auto-reconnect passes
    /// `false` so it never blocks the UI thread on the spawn sleep.
    fn respawn(&mut self, validate: bool) -> Result<(), String> {
        self.kill();
        self.child = if validate {
            spawn_ssh_args(&self.args)?
        } else {
            spawn_ssh_args_nowait(&self.args)?
        };
        self.last_status = TunnelStatus::Starting;
        self.started_at = Instant::now();
        Ok(())
    }

    /// Manual restart: kill and respawn, then wipe the reconnect bookkeeping so
    /// the user's explicit action gives the tunnel a clean slate.
    pub fn restart(&mut self) -> Result<(), String> {
        self.respawn(true)?;
        self.retry_backoff = INITIAL_BACKOFF;
        self.retry_count = 0;
        self.next_retry_at = None;
        // Re-engage auto-reconnect: a manual restart means the user wants this
        // tunnel back, even if a prior run had exhausted its retries.
        self.auto_reconnect = true;
        Ok(())
    }

    /// How long the current `ssh` child has been running.
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// PID of the current `ssh` child. For `-L`/`-D` tunnels this is the
    /// process that binds the local port, so the listener health check can
    /// confirm a `LISTEN` socket really belongs to *this* tunnel.
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// The full `ssh` command line this tunnel was spawned with — handy for
    /// copying to the clipboard and reproducing the tunnel outside prt. Each
    /// argument is shell-quoted so paths with spaces (e.g. an identity file
    /// under `/Users/x/my keys/id_rsa`) paste back as a single argument.
    pub fn command_string(&self) -> String {
        let mut cmd = String::from("ssh");
        for arg in &self.args {
            cmd.push(' ');
            cmd.push_str(&shell_quote(arg));
        }
        cmd
    }
}

/// Quote a single argument for safe pasting into a POSIX shell. Returns the
/// argument unchanged when it contains only shell-safe characters; otherwise
/// wraps it in single quotes, escaping any embedded single quote as `'\''`.
fn shell_quote(arg: &str) -> String {
    let safe = !arg.is_empty()
        && arg.bytes().all(|b| {
            b.is_ascii_alphanumeric()
                || matches!(
                    b,
                    b'_' | b'-' | b'.' | b'/' | b':' | b'=' | b'@' | b',' | b'+'
                )
        });
    if safe {
        return arg.to_string();
    }
    let mut out = String::with_capacity(arg.len() + 2);
    out.push('\'');
    for ch in arg.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
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

fn spawn_ssh_args(args: &[String]) -> Result<Child, String> {
    let mut child = Command::new("ssh")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to start ssh: {e}"))?;

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

/// Spawn `ssh` without the blocking liveness check. Used by auto-reconnect,
/// which runs on the UI thread every loop iteration and must not sleep; a
/// respawn that dies immediately is caught on the next `cleanup()` tick.
fn spawn_ssh_args_nowait(args: &[String]) -> Result<Child, String> {
    Command::new("ssh")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to start ssh: {e}"))
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

    /// Refresh each tunnel's `last_status`. Dead tunnels remain in the list
    /// (with `last_status = Failed`) so the user can see what happened and
    /// either restart or remove them; previously they were silently dropped.
    pub fn cleanup(&mut self) {
        for tunnel in &mut self.tunnels {
            tunnel.refresh_status();
        }
    }

    /// Auto-restart tunnels that have failed on their own, with exponential
    /// backoff so an unreachable host isn't hammered. Call after `cleanup()`
    /// (which is what marks tunnels `Failed`). Returns the number of restart
    /// attempts that succeeded this tick.
    ///
    /// Scheduling: the first time a failure is observed, the next attempt is
    /// deferred by `INITIAL_BACKOFF`; the backoff doubles on each attempt up to
    /// `MAX_BACKOFF` and is only reset once the tunnel stays `Alive` for
    /// `STABILITY_THRESHOLD` (see `refresh_status`). After
    /// `MAX_RECONNECT_ATTEMPTS` consecutive failures the tunnel gives up
    /// (`auto_reconnect = false`) so an unreachable host isn't retried forever.
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
                // Due: try to respawn (non-blocking — never sleeps the UI).
                Some(_) => {
                    tunnel.retry_count += 1;
                    let outcome = tunnel.respawn(false);
                    // Grow the backoff for the *next* attempt regardless of
                    // whether this spawn launched; only a sustained `Alive`
                    // period resets it.
                    tunnel.retry_backoff = (tunnel.retry_backoff * 2).min(MAX_BACKOFF);
                    match outcome {
                        Ok(()) => {
                            reconnected += 1;
                            // Reschedule from the next observed failure.
                            tunnel.next_retry_at = None;
                        }
                        Err(_) => {
                            tunnel.next_retry_at = Some(now + tunnel.retry_backoff);
                        }
                    }
                    if tunnel.retry_count >= MAX_RECONNECT_ATTEMPTS {
                        tunnel.auto_reconnect = false;
                    }
                }
            }
        }
        reconnected
    }

    /// Drop tunnels that have permanently failed — i.e. `Failed` *and* no
    /// longer auto-reconnecting (retries exhausted or explicitly disabled).
    /// Tunnels still cycling through reconnect attempts are kept, so saving the
    /// list (which calls this) never silently deletes a tunnel that just
    /// happens to be between retries.
    pub fn drop_failed(&mut self) {
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

    #[test]
    fn shell_quote_leaves_safe_args_untouched() {
        assert_eq!(shell_quote("-L"), "-L");
        assert_eq!(shell_quote("8080:localhost:80"), "8080:localhost:80");
        assert_eq!(
            shell_quote("/home/user/.ssh/id_rsa"),
            "/home/user/.ssh/id_rsa"
        );
        assert_eq!(shell_quote("user@host"), "user@host");
    }

    #[test]
    fn shell_quote_wraps_paths_with_spaces() {
        assert_eq!(
            shell_quote("/Users/x/my keys/id_rsa"),
            "'/Users/x/my keys/id_rsa'"
        );
    }

    #[test]
    fn shell_quote_escapes_embedded_single_quote() {
        assert_eq!(shell_quote("a'b"), "'a'\\''b'");
    }

    #[test]
    fn shell_quote_quotes_empty_arg() {
        assert_eq!(shell_quote(""), "''");
    }
}
