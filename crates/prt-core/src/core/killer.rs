//! Process termination (SIGTERM / SIGKILL).
//!
//! Provides a safe wrapper around `nix::sys::signal::kill` with a
//! pre-check that the target process is still alive.

use anyhow::{Context, Result};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;

/// Send a signal to a process.
///
/// - `force = false` → SIGTERM (graceful shutdown)
/// - `force = true`  → SIGKILL (immediate kill)
///
/// Returns an error if the process no longer exists or the signal fails.
pub fn kill_process(pid: u32, force: bool) -> Result<()> {
    if !is_running(pid) {
        anyhow::bail!("process {pid} is no longer running");
    }
    let sig = if force {
        Signal::SIGKILL
    } else {
        Signal::SIGTERM
    };
    signal::kill(Pid::from_raw(pid as i32), sig)
        .with_context(|| format!("failed to send {sig} to pid {pid}"))
}

/// Check if a process with the given PID is alive.
///
/// Uses `kill(pid, 0)` — sends no signal but checks process existence.
pub fn is_running(pid: u32) -> bool {
    signal::kill(Pid::from_raw(pid as i32), None).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_process_is_running() {
        assert!(is_running(std::process::id()));
    }

    #[test]
    fn nonexistent_process_is_not_running() {
        assert!(!is_running(4_000_000));
    }

    #[test]
    fn kill_nonexistent_process_returns_error() {
        assert!(kill_process(4_000_000, false).is_err());
    }

    #[test]
    fn kill_error_message_contains_pid() {
        let err = kill_process(4_000_000, false).unwrap_err();
        assert!(err.to_string().contains("4000000"));
    }
}
