//! Alert rule evaluation engine.
//!
//! Matches [`AlertRuleConfig`] conditions against port entries and
//! returns triggered alerts. The TUI layer handles the actual
//! actions (bell, highlight).

use crate::config::AlertRuleConfig;
use crate::model::{ConnectionState, EntryStatus, TrackedEntry};

/// The action to perform when an alert fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertAction {
    /// Ring terminal bell (BEL character).
    Bell,
    /// Highlight the row in the table.
    Highlight,
}

/// A triggered alert: the index of the matching entry and the action.
#[derive(Debug, Clone)]
pub struct FiredAlert {
    /// Index into the entries slice.
    pub entry_index: usize,
    /// What to do about it.
    pub action: AlertAction,
}

/// Evaluate all alert rules against the current entries.
///
/// Returns a list of fired alerts. Bell alerts only fire on `New`
/// entries to avoid ringing every 2 seconds.
pub fn evaluate(rules: &[AlertRuleConfig], entries: &[TrackedEntry]) -> Vec<FiredAlert> {
    let mut alerts = Vec::new();

    for (i, entry) in entries.iter().enumerate() {
        for rule in rules {
            if matches_rule(rule, entry, entries) {
                let action = parse_action(&rule.action);
                // Bell only on New entries (not every tick)
                if action == AlertAction::Bell && entry.status != EntryStatus::New {
                    continue;
                }
                alerts.push(FiredAlert {
                    entry_index: i,
                    action,
                });
            }
        }
    }

    alerts
}

/// Check if a single entry matches a rule's conditions.
/// All specified conditions must match (AND logic).
fn matches_rule(
    rule: &AlertRuleConfig,
    entry: &TrackedEntry,
    all_entries: &[TrackedEntry],
) -> bool {
    if let Some(port) = rule.port {
        if entry.entry.local_port() != port {
            return false;
        }
    }

    if let Some(ref process) = rule.process {
        if !entry
            .entry
            .process
            .name
            .to_lowercase()
            .contains(&process.to_lowercase())
        {
            return false;
        }
    }

    if let Some(ref state) = rule.state {
        let entry_state = parse_state(state);
        if let Some(expected) = entry_state {
            if entry.entry.state != expected {
                return false;
            }
        }
    }

    if let Some(threshold) = rule.connections_gt {
        let pid = entry.entry.process.pid;
        let count = all_entries
            .iter()
            .filter(|e| e.entry.process.pid == pid)
            .count();
        if count <= threshold {
            return false;
        }
    }

    true
}

fn parse_action(s: &str) -> AlertAction {
    match s.to_lowercase().as_str() {
        "bell" => AlertAction::Bell,
        _ => AlertAction::Highlight,
    }
}

fn parse_state(s: &str) -> Option<ConnectionState> {
    match s.to_uppercase().as_str() {
        "LISTEN" => Some(ConnectionState::Listen),
        "ESTABLISHED" => Some(ConnectionState::Established),
        "TIME_WAIT" => Some(ConnectionState::TimeWait),
        "CLOSE_WAIT" => Some(ConnectionState::CloseWait),
        "SYN_SENT" => Some(ConnectionState::SynSent),
        "SYN_RECV" => Some(ConnectionState::SynRecv),
        "FIN_WAIT1" => Some(ConnectionState::FinWait1),
        "FIN_WAIT2" => Some(ConnectionState::FinWait2),
        "CLOSING" => Some(ConnectionState::Closing),
        "LAST_ACK" => Some(ConnectionState::LastAck),
        "CLOSED" => Some(ConnectionState::Closed),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ConnectionState, EntryStatus, PortEntry, ProcessInfo, Protocol};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::time::Instant;

    fn make_entry(port: u16, pid: u32, name: &str, state: ConnectionState) -> TrackedEntry {
        TrackedEntry {
            entry: PortEntry {
                protocol: Protocol::Tcp,
                local_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
                remote_addr: None,
                state,
                process: ProcessInfo {
                    pid,
                    name: name.into(),
                    path: None,
                    cmdline: None,
                    user: None,
                    parent_pid: None,
                    parent_name: None,
                },
            },
            status: EntryStatus::Unchanged,
            seen_at: Instant::now(),
            first_seen: None,
            suspicious: Vec::new(),
            container_name: None,
            service_name: None,
        }
    }

    fn make_new_entry(port: u16, pid: u32, name: &str, state: ConnectionState) -> TrackedEntry {
        let mut e = make_entry(port, pid, name, state);
        e.status = EntryStatus::New;
        e
    }

    #[test]
    fn port_rule_matches() {
        let rule = AlertRuleConfig {
            port: Some(22),
            action: "highlight".into(),
            ..Default::default()
        };
        let entries = vec![make_entry(22, 1, "sshd", ConnectionState::Listen)];
        let alerts = evaluate(&[rule], &entries);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].action, AlertAction::Highlight);
    }

    #[test]
    fn port_rule_no_match() {
        let rule = AlertRuleConfig {
            port: Some(22),
            action: "highlight".into(),
            ..Default::default()
        };
        let entries = vec![make_entry(80, 1, "nginx", ConnectionState::Listen)];
        assert!(evaluate(&[rule], &entries).is_empty());
    }

    #[test]
    fn process_rule_matches_case_insensitive() {
        let rule = AlertRuleConfig {
            process: Some("Python".into()),
            action: "highlight".into(),
            ..Default::default()
        };
        let entries = vec![make_entry(8000, 1, "python3", ConnectionState::Listen)];
        assert_eq!(evaluate(&[rule], &entries).len(), 1);
    }

    #[test]
    fn state_rule_matches() {
        let rule = AlertRuleConfig {
            state: Some("LISTEN".into()),
            action: "highlight".into(),
            ..Default::default()
        };
        let entries = vec![
            make_entry(80, 1, "nginx", ConnectionState::Listen),
            make_entry(81, 2, "curl", ConnectionState::Established),
        ];
        let alerts = evaluate(&[rule], &entries);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].entry_index, 0);
    }

    #[test]
    fn connections_gt_rule() {
        let rule = AlertRuleConfig {
            connections_gt: Some(1),
            action: "highlight".into(),
            ..Default::default()
        };
        // PID 1 has 2 connections, PID 2 has 1
        let entries = vec![
            make_entry(80, 1, "nginx", ConnectionState::Listen),
            make_entry(443, 1, "nginx", ConnectionState::Listen),
            make_entry(8080, 2, "node", ConnectionState::Listen),
        ];
        let alerts = evaluate(&[rule], &entries);
        // PID 1's entries match (2 > 1), PID 2 does not (1 <= 1)
        assert_eq!(alerts.len(), 2);
    }

    #[test]
    fn bell_only_on_new_entries() {
        let rule = AlertRuleConfig {
            port: Some(22),
            action: "bell".into(),
            ..Default::default()
        };
        // Unchanged entry — bell should NOT fire
        let entries = vec![make_entry(22, 1, "sshd", ConnectionState::Listen)];
        assert!(evaluate(std::slice::from_ref(&rule), &entries).is_empty());

        // New entry — bell should fire
        let entries = vec![make_new_entry(22, 1, "sshd", ConnectionState::Listen)];
        let alerts = evaluate(std::slice::from_ref(&rule), &entries);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].action, AlertAction::Bell);
    }

    #[test]
    fn combined_conditions_are_and() {
        let rule = AlertRuleConfig {
            port: Some(80),
            process: Some("python".into()),
            state: Some("LISTEN".into()),
            action: "highlight".into(),
            ..Default::default()
        };
        // Matches all conditions
        let entries = vec![make_entry(80, 1, "python3", ConnectionState::Listen)];
        assert_eq!(evaluate(std::slice::from_ref(&rule), &entries).len(), 1);

        // Wrong port
        let entries = vec![make_entry(8080, 1, "python3", ConnectionState::Listen)];
        assert!(evaluate(std::slice::from_ref(&rule), &entries).is_empty());

        // Wrong process
        let entries = vec![make_entry(80, 1, "nginx", ConnectionState::Listen)];
        assert!(evaluate(std::slice::from_ref(&rule), &entries).is_empty());
    }

    #[test]
    fn empty_rules_no_alerts() {
        let entries = vec![make_entry(80, 1, "nginx", ConnectionState::Listen)];
        assert!(evaluate(&[], &entries).is_empty());
    }

    #[test]
    fn empty_entries_no_alerts() {
        let rule = AlertRuleConfig {
            port: Some(22),
            action: "bell".into(),
            ..Default::default()
        };
        assert!(evaluate(&[rule], &[]).is_empty());
    }

    #[test]
    fn parse_action_variants() {
        assert_eq!(parse_action("bell"), AlertAction::Bell);
        assert_eq!(parse_action("BELL"), AlertAction::Bell);
        assert_eq!(parse_action("highlight"), AlertAction::Highlight);
        assert_eq!(parse_action("unknown"), AlertAction::Highlight);
        assert_eq!(parse_action(""), AlertAction::Highlight);
    }
}
