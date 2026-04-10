//! SSH port forwarding manager.
//!
//! Creates and manages SSH -L tunnels from within the TUI.
//! Tunnels are killed on Drop to prevent orphaned SSH processes.

use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

/// A single SSH tunnel.
pub struct SshTunnel {
    pub local_port: u16,
    pub remote: String,
    child: Child,
}

impl SshTunnel {
    /// Create a new SSH -L tunnel.
    /// `remote` format: `host:port` or `user@host:port`
    pub fn new(local_port: u16, remote_host: &str, remote_port: u16) -> Result<Self, String> {
        let forward_spec = format!("{local_port}:localhost:{remote_port}");
        let mut child = Command::new("ssh")
            .args(["-N", "-L", &forward_spec, remote_host])
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

        Ok(Self {
            local_port,
            remote: format!("{remote_host}:{remote_port}"),
            child,
        })
    }

    /// Human-readable summary: "localhost:8080 → server:22".
    pub fn summary(&self) -> String {
        format!("localhost:{} → {}", self.local_port, self.remote)
    }

    /// Check if the tunnel is still alive.
    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Kill the tunnel.
    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        self.kill();
    }
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

    /// Add a new tunnel. Returns index.
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

    /// Remove dead tunnels.
    pub fn cleanup(&mut self) {
        self.tunnels.retain_mut(|t| t.is_alive());
    }

    /// Kill all tunnels.
    pub fn kill_all(&mut self) {
        for tunnel in &mut self.tunnels {
            tunnel.kill();
        }
        self.tunnels.clear();
    }

    /// Number of active tunnels.
    pub fn count(&self) -> usize {
        self.tunnels.len()
    }

    /// List summaries of all active tunnels.
    pub fn summaries(&self) -> Vec<String> {
        self.tunnels.iter().map(|t| t.summary()).collect()
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

    // Note: SSH tunnel creation tests require an actual SSH server,
    // so we only test the manager state logic here.
}
