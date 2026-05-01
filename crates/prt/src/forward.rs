//! SSH tunnel manager.
//!
//! Spawns and supervises `ssh -N -L`/`-D` tunnels from within the TUI.
//! Tunnels are killed on `Drop` to prevent orphaned `ssh` processes.

use prt_core::core::ssh_tunnel::{SshTunnelSpec, TunnelKind};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

/// A single SSH tunnel: a running `ssh` child process plus its spec.
pub struct SshTunnel {
    pub spec: SshTunnelSpec,
    child: Child,
}

impl SshTunnel {
    /// Spawn an `ssh` process for `spec`. Validates startup by sleeping
    /// briefly and reading stderr if `ssh` exits immediately.
    pub fn spawn(spec: SshTunnelSpec) -> Result<Self, String> {
        spec.validate()?;
        let child = spawn_ssh(&spec)?;
        Ok(Self { spec, child })
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

    /// Check if the underlying ssh child is still alive.
    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Kill the tunnel (signal + reap).
    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    /// Kill and respawn from the current spec.
    pub fn restart(&mut self) -> Result<(), String> {
        self.kill();
        let new_child = spawn_ssh(&self.spec)?;
        self.child = new_child;
        Ok(())
    }
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        self.kill();
    }
}

fn spawn_ssh(spec: &SshTunnelSpec) -> Result<Child, String> {
    let args = spec.ssh_args();
    let mut child = Command::new("ssh")
        .args(&args)
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

    /// Add a tunnel from a full spec.
    pub fn add_spec(&mut self, spec: SshTunnelSpec) -> Result<usize, String> {
        let tunnel = SshTunnel::spawn(spec)?;
        self.tunnels.push(tunnel);
        Ok(self.tunnels.len() - 1)
    }

    /// Remove dead tunnels.
    pub fn cleanup(&mut self) {
        self.tunnels.retain_mut(|t| t.is_alive());
    }

    /// Kill the tunnel at `idx`. No-op if out of bounds.
    pub fn kill_at(&mut self, idx: usize) {
        if idx < self.tunnels.len() {
            self.tunnels[idx].kill();
            self.tunnels.remove(idx);
        }
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
