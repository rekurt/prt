//! Firewall quick-block: generate and execute commands to block a remote IP.
//!
//! - **Linux:** `iptables -A INPUT -s {IP} -j DROP`
//! - **macOS:** `pfctl -t prt_blocked -T add {IP}`
//!
//! Always requires confirmation dialog in the TUI. The generated commands
//! are shown to the user before execution.

use std::net::IpAddr;
use std::process::Command;

/// Generate the block command for the current platform.
pub fn block_command(ip: IpAddr) -> String {
    if cfg!(target_os = "linux") {
        format!("sudo iptables -A INPUT -s {ip} -j DROP")
    } else if cfg!(target_os = "macos") {
        format!("sudo pfctl -t prt_blocked -T add {ip}")
    } else {
        format!("# unsupported platform: block {ip}")
    }
}

/// Generate the undo (unblock) command for the current platform.
pub fn unblock_command(ip: IpAddr) -> String {
    if cfg!(target_os = "linux") {
        format!("sudo iptables -D INPUT -s {ip} -j DROP")
    } else if cfg!(target_os = "macos") {
        format!("sudo pfctl -t prt_blocked -T delete {ip}")
    } else {
        format!("# unsupported platform: unblock {ip}")
    }
}

/// Execute the block command. Requires sudo.
/// Returns Ok(()) on success, Err with message on failure.
pub fn execute_block(ip: IpAddr, sudo_password: Option<&str>) -> Result<(), String> {
    let ip_str = ip.to_string();
    let (cmd, args) = if cfg!(target_os = "linux") {
        (
            "iptables",
            vec!["-A", "INPUT", "-s", ip_str.as_str(), "-j", "DROP"],
        )
    } else if cfg!(target_os = "macos") {
        (
            "pfctl",
            vec!["-t", "prt_blocked", "-T", "add", ip_str.as_str()],
        )
    } else {
        return Err("unsupported platform".into());
    };

    let output = if let Some(password) = sudo_password {
        Command::new("sudo")
            .args(["-S", cmd])
            .args(&args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(ref mut stdin) = child.stdin {
                    let _ = writeln!(stdin, "{password}");
                }
                child.wait_with_output()
            })
            .map_err(|e| e.to_string())?
    } else {
        Command::new("sudo")
            .args(["-n", cmd])
            .args(&args)
            .output()
            .map_err(|e| e.to_string())?
    };

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("command failed: {}", stderr.trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn block_command_ipv4() {
        let cmd = block_command(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
        assert!(cmd.contains("10.0.0.1"));
        assert!(cmd.contains("sudo"));
    }

    #[test]
    fn block_command_ipv6() {
        let cmd = block_command(IpAddr::V6(Ipv6Addr::LOCALHOST));
        assert!(cmd.contains("::1"));
    }

    #[test]
    fn unblock_command_contains_ip() {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let cmd = unblock_command(ip);
        assert!(cmd.contains("192.168.1.1"));
    }

    #[test]
    fn block_and_unblock_are_different() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        assert_ne!(block_command(ip), unblock_command(ip));
    }
}
