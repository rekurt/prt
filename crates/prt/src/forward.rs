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

/// A single SSH tunnel: a running `ssh` child process plus the spec and
/// resolved argument list (kept so `restart()` reuses the same resolution).
pub struct SshTunnel {
    pub spec: SshTunnelSpec,
    args: Vec<String>,
    child: Child,
    pub last_status: TunnelStatus,
    /// When the current `ssh` child was spawned — reset on `restart()`.
    started_at: Instant,
}

impl SshTunnel {
    /// Spawn an `ssh` process for `spec` with no extra host resolution
    /// (relies on `~/.ssh/config` or DNS for the alias).
    pub fn spawn(spec: SshTunnelSpec) -> Result<Self, String> {
        spec.validate()?;
        let args = spec.ssh_args();
        let child = spawn_ssh_args(&args)?;
        Ok(Self {
            spec,
            args,
            child,
            last_status: TunnelStatus::Starting,
            started_at: Instant::now(),
        })
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
        Ok(Self {
            spec,
            args,
            child,
            last_status: TunnelStatus::Starting,
            started_at: Instant::now(),
        })
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

    /// Kill and respawn using the previously resolved arg list.
    pub fn restart(&mut self) -> Result<(), String> {
        self.kill();
        self.child = spawn_ssh_args(&self.args)?;
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
    pub fn command_string(&self) -> String {
        let mut cmd = String::from("ssh");
        for arg in &self.args {
            cmd.push(' ');
            cmd.push_str(arg);
        }
        cmd
    }
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

    /// Drop tunnels that have already failed. Called when the user asks to
    /// prune the list (e.g. via "save" which only persists running tunnels).
    pub fn drop_failed(&mut self) {
        self.tunnels
            .retain(|t| t.last_status != TunnelStatus::Failed);
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
}
