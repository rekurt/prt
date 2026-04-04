//! Platform-specific port scanning.
//!
//! - **macOS**: parses `lsof -F` output + batch `ps` calls (2 per cycle)
//! - **Linux**: reads `/proc/net/{tcp,tcp6,udp,udp6}` via `procfs` crate

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

use crate::model::PortEntry;
use anyhow::Result;

/// Scan all visible network ports (unprivileged).
pub fn scan_ports() -> Result<Vec<PortEntry>> {
    #[cfg(target_os = "linux")]
    {
        linux::scan()
    }
    #[cfg(target_os = "macos")]
    {
        macos::scan()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        anyhow::bail!("unsupported platform")
    }
}

/// Scan with cached sudo credentials (`sudo -n`).
pub fn scan_ports_elevated() -> Result<Vec<PortEntry>> {
    #[cfg(target_os = "macos")]
    {
        macos::scan_elevated()
    }
    #[cfg(target_os = "linux")]
    {
        linux::scan()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        anyhow::bail!("unsupported platform")
    }
}

/// Scan with explicit sudo password piped via stdin (`sudo -S`).
pub fn scan_ports_with_sudo(password: &str) -> Result<Vec<PortEntry>> {
    #[cfg(target_os = "macos")]
    {
        macos::scan_with_sudo(password)
    }
    #[cfg(target_os = "linux")]
    {
        let _ = password;
        linux::scan()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = password;
        anyhow::bail!("unsupported platform")
    }
}
