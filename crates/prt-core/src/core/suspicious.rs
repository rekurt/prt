//! Heuristic detection of suspicious network connections.
//!
//! Each heuristic is a pure function that examines a [`PortEntry`]
//! and returns `true` if the pattern matches. The main entry point
//! [`check`] runs all heuristics and collects matching reasons.
//!
//! # Heuristics
//!
//! 1. **NonRootPrivileged** — non-root process listening on port < 1024
//! 2. **ScriptOnSensitive** — scripting language on a sensitive port (22, 80, 443)
//! 3. **RootHighPortOutgoing** — root process with outgoing connection to high port

use crate::model::{ConnectionState, PortEntry, SuspiciousReason};

/// Well-known scripting language process names.
const SCRIPT_NAMES: &[&str] = &["python", "python3", "perl", "ruby", "node"];

/// Sensitive ports that should not normally be served by scripting languages.
const SENSITIVE_PORTS: &[u16] = &[22, 80, 443];

/// Run all heuristics on a single port entry and return matching reasons.
pub fn check(entry: &PortEntry) -> Vec<SuspiciousReason> {
    let mut reasons = Vec::new();

    if is_non_root_privileged(entry) {
        reasons.push(SuspiciousReason::NonRootPrivileged);
    }
    if is_script_on_sensitive(entry) {
        reasons.push(SuspiciousReason::ScriptOnSensitive);
    }
    if is_root_high_port_outgoing(entry) {
        reasons.push(SuspiciousReason::RootHighPortOutgoing);
    }

    reasons
}

/// Non-root process listening on a privileged port (< 1024).
///
/// On Unix, ports below 1024 traditionally require root. A non-root
/// process on such a port may indicate capability escalation or
/// misconfigured setuid.
fn is_non_root_privileged(entry: &PortEntry) -> bool {
    entry.state == ConnectionState::Listen
        && entry.local_addr.port() < 1024
        && entry.process.user.as_deref().unwrap_or("") != "root"
        && entry.process.user.is_some()
}

/// Scripting language (python, perl, ruby, node) listening on a
/// sensitive port (22, 80, 443).
///
/// Legitimate web servers are typically compiled binaries (nginx,
/// apache). A scripting language binding directly to these ports
/// may indicate a backdoor or debug server left in production.
fn is_script_on_sensitive(entry: &PortEntry) -> bool {
    if entry.state != ConnectionState::Listen {
        return false;
    }
    let port = entry.local_addr.port();
    if !SENSITIVE_PORTS.contains(&port) {
        return false;
    }
    let name = entry.process.name.to_lowercase();
    SCRIPT_NAMES.iter().any(|&s| name.contains(s))
}

/// Root process making an outgoing (ESTABLISHED) connection where
/// the remote port is > 1024.
///
/// Root processes initiating connections to high ports could indicate
/// a reverse shell or data exfiltration channel.
fn is_root_high_port_outgoing(entry: &PortEntry) -> bool {
    if entry.state != ConnectionState::Established {
        return false;
    }
    if entry.process.user.as_deref() != Some("root") {
        return false;
    }
    match entry.remote_addr {
        Some(addr) => addr.port() >= 1024,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ProcessInfo, Protocol};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn make_process(name: &str, user: Option<&str>) -> ProcessInfo {
        ProcessInfo {
            pid: 1,
            name: name.to_string(),
            path: None,
            cmdline: None,
            user: user.map(String::from),
            parent_pid: None,
            parent_name: None,
        }
    }

    fn listen_entry(port: u16, name: &str, user: Option<&str>) -> PortEntry {
        PortEntry {
            protocol: Protocol::Tcp,
            local_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            remote_addr: None,
            state: ConnectionState::Listen,
            process: make_process(name, user),
        }
    }

    fn established_entry(
        local_port: u16,
        remote_port: u16,
        name: &str,
        user: Option<&str>,
    ) -> PortEntry {
        PortEntry {
            protocol: Protocol::Tcp,
            local_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), local_port),
            remote_addr: Some(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
                remote_port,
            )),
            state: ConnectionState::Established,
            process: make_process(name, user),
        }
    }

    // ── NonRootPrivileged ────────────────────────────────────────

    #[test]
    fn non_root_on_privileged_port_is_suspicious() {
        let entry = listen_entry(80, "nginx", Some("www-data"));
        assert!(is_non_root_privileged(&entry));
        assert!(check(&entry).contains(&SuspiciousReason::NonRootPrivileged));
    }

    #[test]
    fn root_on_privileged_port_is_not_suspicious() {
        let entry = listen_entry(80, "nginx", Some("root"));
        assert!(!is_non_root_privileged(&entry));
    }

    #[test]
    fn non_root_on_high_port_is_not_suspicious() {
        let entry = listen_entry(8080, "nginx", Some("www-data"));
        assert!(!is_non_root_privileged(&entry));
    }

    #[test]
    fn established_on_privileged_port_is_not_suspicious() {
        // Only LISTEN triggers this heuristic
        let entry = established_entry(80, 12345, "nginx", Some("www-data"));
        assert!(!is_non_root_privileged(&entry));
    }

    #[test]
    fn no_user_not_flagged_as_non_root_privileged() {
        // If we don't know the user, don't flag (avoid false positives)
        let entry = listen_entry(80, "nginx", None);
        assert!(!is_non_root_privileged(&entry));
    }

    // ── ScriptOnSensitive ────────────────────────────────────────

    #[test]
    fn python_on_port_80_is_suspicious() {
        let entry = listen_entry(80, "python3", Some("user"));
        assert!(is_script_on_sensitive(&entry));
        assert!(check(&entry).contains(&SuspiciousReason::ScriptOnSensitive));
    }

    #[test]
    fn node_on_port_443_is_suspicious() {
        let entry = listen_entry(443, "node", Some("user"));
        assert!(is_script_on_sensitive(&entry));
    }

    #[test]
    fn perl_on_port_22_is_suspicious() {
        let entry = listen_entry(22, "perl", Some("user"));
        assert!(is_script_on_sensitive(&entry));
    }

    #[test]
    fn nginx_on_port_80_is_not_suspicious() {
        let entry = listen_entry(80, "nginx", Some("root"));
        assert!(!is_script_on_sensitive(&entry));
    }

    #[test]
    fn python_on_port_8080_is_not_suspicious() {
        // Port 8080 is not in the sensitive list
        let entry = listen_entry(8080, "python3", Some("user"));
        assert!(!is_script_on_sensitive(&entry));
    }

    #[test]
    fn python_established_on_port_80_is_not_suspicious() {
        // Only LISTEN triggers this heuristic
        let entry = established_entry(80, 12345, "python3", Some("user"));
        assert!(!is_script_on_sensitive(&entry));
    }

    #[test]
    fn ruby_on_port_443_is_suspicious() {
        let entry = listen_entry(443, "ruby", Some("deploy"));
        assert!(is_script_on_sensitive(&entry));
    }

    #[test]
    fn script_on_sensitive_fires_even_without_user() {
        // Intentional: ScriptOnSensitive is name+port based, not user-based.
        // Unlike NonRootPrivileged, user info is irrelevant to this heuristic.
        let entry = listen_entry(80, "python3", None);
        assert!(is_script_on_sensitive(&entry));
    }

    // ── RootHighPortOutgoing ─────────────────────────────────────

    #[test]
    fn root_outgoing_to_high_port_is_suspicious() {
        let entry = established_entry(54321, 4444, "bash", Some("root"));
        assert!(is_root_high_port_outgoing(&entry));
        assert!(check(&entry).contains(&SuspiciousReason::RootHighPortOutgoing));
    }

    #[test]
    fn root_outgoing_to_port_1024_is_suspicious() {
        // Port 1024 is the first registered/high port (IANA: 0-1023 well-known)
        let entry = established_entry(54321, 1024, "bash", Some("root"));
        assert!(is_root_high_port_outgoing(&entry));
    }

    #[test]
    fn root_outgoing_to_port_1023_is_not_suspicious() {
        // Port 1023 is the last well-known port
        let entry = established_entry(54321, 1023, "curl", Some("root"));
        assert!(!is_root_high_port_outgoing(&entry));
    }

    #[test]
    fn root_outgoing_to_low_port_is_not_suspicious() {
        let entry = established_entry(54321, 443, "curl", Some("root"));
        assert!(!is_root_high_port_outgoing(&entry));
    }

    #[test]
    fn non_root_outgoing_to_high_port_is_not_suspicious() {
        let entry = established_entry(54321, 4444, "bash", Some("user"));
        assert!(!is_root_high_port_outgoing(&entry));
    }

    #[test]
    fn root_listen_not_flagged_as_outgoing() {
        let entry = listen_entry(4444, "sshd", Some("root"));
        assert!(!is_root_high_port_outgoing(&entry));
    }

    #[test]
    fn root_established_no_remote_not_flagged() {
        let mut entry = listen_entry(80, "nginx", Some("root"));
        entry.state = ConnectionState::Established;
        // remote_addr is None
        assert!(!is_root_high_port_outgoing(&entry));
    }

    // ── Combined / edge cases ────────────────────────────────────

    #[test]
    fn multiple_reasons_possible() {
        // python3 as www-data listening on port 80:
        // - NonRootPrivileged (port 80 < 1024, user != root)
        // - ScriptOnSensitive (python3 on port 80)
        let entry = listen_entry(80, "python3", Some("www-data"));
        let reasons = check(&entry);
        assert!(reasons.contains(&SuspiciousReason::NonRootPrivileged));
        assert!(reasons.contains(&SuspiciousReason::ScriptOnSensitive));
        assert_eq!(reasons.len(), 2);
    }

    #[test]
    fn clean_entry_has_no_reasons() {
        let entry = listen_entry(8080, "nginx", Some("www-data"));
        assert!(check(&entry).is_empty());
    }

    #[test]
    fn root_on_standard_port_is_clean() {
        let entry = listen_entry(443, "nginx", Some("root"));
        assert!(check(&entry).is_empty());
    }
}
