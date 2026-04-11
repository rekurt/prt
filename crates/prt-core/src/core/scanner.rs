//! Port scanning, diffing, filtering, sorting, and export.
//!
//! This is the central business-logic module. The data pipeline is:
//!
//! ```text
//! scan() → diff_entries() → sort_entries() → filter_indices() → UI
//! ```
//!
//! Identity key for change tracking is:
//! `(pid, protocol, local_addr, remote_addr, state)`.

use crate::i18n;
use crate::model::{EntryStatus, ExportFormat, PortEntry, SortColumn, SortState, TrackedEntry};
use crate::platform;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// Scan all network ports visible to the current user.
pub fn scan() -> Result<Vec<PortEntry>> {
    platform::scan_ports()
}

/// Scan ports with cached sudo credentials (`sudo -n`).
/// Falls back to unprivileged scan if credentials are not cached.
pub fn scan_elevated() -> Result<Vec<PortEntry>> {
    platform::scan_ports_elevated()
}

/// Returns `true` when cached elevated access is still available.
pub fn has_elevated_access() -> bool {
    platform::has_elevated_access()
}

/// Scan ports using an explicit sudo password (`sudo -S`).
/// The password is piped to stdin; no tty required.
pub fn scan_with_sudo(password: &str) -> Result<Vec<PortEntry>> {
    platform::scan_ports_with_sudo(password)
}

/// Compute entry diffs between the previous and current scan.
///
/// Returns a merged list where:
/// - Entries present in `current` but not `prev` are [`EntryStatus::New`]
/// - Entries present in both are [`EntryStatus::Unchanged`]
/// - Entries in `prev` but not `current` are [`EntryStatus::Gone`]
///
/// Already-gone entries from `prev` are dropped (no double-gone).
/// Identity key: `(pid, protocol, local_addr, remote_addr, state)`.
pub fn diff_entries(
    prev: &[TrackedEntry],
    current: Vec<PortEntry>,
    now: Instant,
) -> Vec<TrackedEntry> {
    let current_keys: HashSet<EntryKey> = current.iter().map(entry_key).collect();

    // HashMap for O(1) lookup + carry-forward of first_seen from prev entries.
    let prev_map: HashMap<EntryKey, &TrackedEntry> = prev
        .iter()
        .filter(|e| e.status != EntryStatus::Gone)
        .map(|e| (tracked_entry_key(e), e))
        .collect();

    let mut result: Vec<TrackedEntry> = current
        .into_iter()
        .map(|entry| {
            let key = entry_key(&entry);
            let (status, first_seen) = if let Some(prev_e) = prev_map.get(&key) {
                // Carry forward first_seen; if prev had None (pre-upgrade),
                // start counting from now so aging kicks in eventually.
                (EntryStatus::Unchanged, prev_e.first_seen.or(Some(now)))
            } else {
                // New entry — first_seen is now
                (EntryStatus::New, Some(now))
            };
            TrackedEntry {
                entry,
                status,
                seen_at: now,
                first_seen,
                suspicious: Vec::new(),
                container_name: None,
                service_name: None,
            }
        })
        .collect();

    for prev_entry in prev {
        let key = tracked_entry_key(prev_entry);
        if !current_keys.contains(&key) && prev_entry.status != EntryStatus::Gone {
            result.push(TrackedEntry {
                entry: prev_entry.entry.clone(),
                status: EntryStatus::Gone,
                seen_at: now,
                first_seen: prev_entry.first_seen,
                // Carry forward enrichment data so Gone entries retain
                // their [!] tags and service names during the retention window.
                suspicious: prev_entry.suspicious.clone(),
                container_name: prev_entry.container_name.clone(),
                service_name: prev_entry.service_name.clone(),
            });
        }
    }

    result
}

type EntryKey = (
    u32,
    crate::model::Protocol,
    SocketAddr,
    Option<SocketAddr>,
    crate::model::ConnectionState,
);

fn entry_key(entry: &PortEntry) -> EntryKey {
    (
        entry.process.pid,
        entry.protocol,
        entry.local_addr,
        entry.remote_addr,
        entry.state,
    )
}

fn tracked_entry_key(entry: &TrackedEntry) -> EntryKey {
    entry_key(&entry.entry)
}

/// Format a duration as a human-readable short string.
///
/// Examples: "0s", "45s", "5m", "2h", "3d"
pub fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86400)
    }
}

/// Sort entries in-place by the given column and direction.
pub fn sort_entries(entries: &mut [TrackedEntry], state: &SortState) {
    entries.sort_by(|a, b| {
        let cmp = match state.column {
            SortColumn::Port => a.entry.local_port().cmp(&b.entry.local_port()),
            SortColumn::Service => {
                // Sort None (unknown service) after all named services
                let a_s = a.service_name.as_deref().unwrap_or("\u{FFFF}");
                let b_s = b.service_name.as_deref().unwrap_or("\u{FFFF}");
                a_s.cmp(b_s)
            }
            SortColumn::Protocol => a.entry.protocol.cmp(&b.entry.protocol),
            SortColumn::State => a.entry.state.cmp(&b.entry.state),
            SortColumn::Pid => a.entry.process.pid.cmp(&b.entry.process.pid),
            SortColumn::ProcessName => a.entry.process.name.cmp(&b.entry.process.name),
            SortColumn::User => a.entry.process.user.cmp(&b.entry.process.user),
        };
        if state.ascending {
            cmp
        } else {
            cmp.reverse()
        }
    });
}

/// Returns `true` if the entry matches the query string.
///
/// Matches against: port number, process name, PID, protocol, state, user.
/// All comparisons are case-insensitive.
fn matches_query(e: &TrackedEntry, q: &str) -> bool {
    // Special filter: "!" shows only suspicious entries
    if q == "!" {
        return !e.suspicious.is_empty();
    }

    e.entry.local_port().to_string().contains(q)
        || e.entry.process.name.to_lowercase().contains(q)
        || e.entry.process.pid.to_string().contains(q)
        || e.entry.protocol.to_string().to_lowercase().contains(q)
        || e.entry.state.to_string().to_lowercase().contains(q)
        || e.entry
            .process
            .user
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains(q)
        || e.service_name
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains(q)
}

/// Filter entries by query string, returning matching entries.
/// Empty query returns all entries.
#[cfg(test)]
pub fn filter_entries<'a>(entries: &'a [TrackedEntry], query: &str) -> Vec<&'a TrackedEntry> {
    if query.is_empty() {
        return entries.iter().collect();
    }
    let q = query.to_lowercase();
    entries.iter().filter(|e| matches_query(e, &q)).collect()
}

/// Filter entries by query, returning indices into the original slice.
/// Empty query returns all indices `0..len`.
pub fn filter_indices(entries: &[TrackedEntry], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..entries.len()).collect();
    }
    let q = query.to_lowercase();
    entries
        .iter()
        .enumerate()
        .filter(|(_, e)| matches_query(e, &q))
        .map(|(i, _)| i)
        .collect()
}

/// Export port entries to JSON or CSV string.
///
/// JSON uses `serde_json::to_string_pretty`.
/// CSV includes a header row with all fields.
pub fn export(entries: &[PortEntry], format: ExportFormat) -> Result<String> {
    match format {
        ExportFormat::Json => Ok(serde_json::to_string_pretty(entries)?),
        ExportFormat::Csv => {
            let mut buf = Vec::new();
            {
                let mut wtr = csv::Writer::from_writer(&mut buf);
                wtr.write_record([
                    "protocol",
                    "local_addr",
                    "remote_addr",
                    "state",
                    "pid",
                    "process",
                    "user",
                    "parent_pid",
                    "parent_name",
                    "cmdline",
                ])?;
                for e in entries {
                    let process_name = sanitize_csv_cell(&e.process.name);
                    let user = sanitize_csv_cell(e.process.user.as_deref().unwrap_or(""));
                    let parent_name =
                        sanitize_csv_cell(e.process.parent_name.as_deref().unwrap_or(""));
                    let cmdline = sanitize_csv_cell(e.process.cmdline.as_deref().unwrap_or(""));
                    wtr.write_record([
                        &e.protocol.to_string(),
                        &e.local_addr.to_string(),
                        &e.remote_addr.map(|a| a.to_string()).unwrap_or_default(),
                        &e.state.to_string(),
                        &e.process.pid.to_string(),
                        &process_name,
                        &user,
                        &e.process
                            .parent_pid
                            .map(|p| p.to_string())
                            .unwrap_or_default(),
                        &parent_name,
                        &cmdline,
                    ])?;
                }
                wtr.flush()?;
            }
            Ok(String::from_utf8(buf)?)
        }
    }
}

fn sanitize_csv_cell(value: &str) -> String {
    let formula_start = value
        .trim_start_matches(|c: char| c.is_ascii_control())
        .chars()
        .next();
    match formula_start {
        Some('=' | '+' | '-' | '@') => format!("'{value}"),
        _ => value.to_owned(),
    }
}

/// Returns `true` if the current process is running as root (UID 0).
pub fn is_root() -> bool {
    nix::unistd::geteuid().is_root()
}

/// Build a text-based process tree for the given PID.
///
/// Shows up to 2 ancestor levels (grandparent → parent → process)
/// and all network connections belonging to the process.
pub fn build_process_tree(entries: &[TrackedEntry], pid: u32) -> Vec<String> {
    let s = i18n::strings();
    let entry = entries.iter().find(|e| e.entry.process.pid == pid);
    let Some(entry) = entry else {
        return vec![s.process_not_found.into()];
    };

    let mut lines = Vec::new();
    let p = &entry.entry.process;

    let mut ancestors: Vec<(u32, String)> = Vec::new();
    if let (Some(ppid), Some(pname)) = (p.parent_pid, p.parent_name.as_ref()) {
        ancestors.push((ppid, pname.clone()));
        if let Some(parent_entry) = entries.iter().find(|e| e.entry.process.pid == ppid) {
            if let (Some(gppid), Some(gpname)) = (
                parent_entry.entry.process.parent_pid,
                parent_entry.entry.process.parent_name.as_ref(),
            ) {
                ancestors.push((gppid, gpname.clone()));
            }
        }
    }

    ancestors.reverse();
    for (i, (apid, aname)) in ancestors.iter().enumerate() {
        let indent = "  ".repeat(i);
        let connector = if i == 0 { "" } else { "└─ " };
        lines.push(format!("{indent}{connector}{aname} ({apid})"));
    }

    let depth = ancestors.len();
    let indent = "  ".repeat(depth);
    let connector = if depth == 0 { "" } else { "└─ " };
    let user_str = p.user.as_deref().unwrap_or("");
    lines.push(format!(
        "{indent}{connector}{} ({}) [{}]",
        p.name, p.pid, user_str
    ));

    let child_indent = "  ".repeat(depth + 1);
    for e in entries.iter().filter(|e| e.entry.process.pid == pid) {
        let arrow = e
            .entry
            .remote_addr
            .map(|a| format!(" → {a}"))
            .unwrap_or_default();
        lines.push(format!(
            "{child_indent}├─ :{} {} {}{}",
            e.entry.local_port(),
            e.entry.protocol,
            e.entry.state,
            arrow,
        ));
    }

    lines
}

/// Collect all entries belonging to a given PID.
pub fn process_connections(entries: &[TrackedEntry], pid: u32) -> Vec<&TrackedEntry> {
    entries
        .iter()
        .filter(|e| e.entry.process.pid == pid)
        .collect()
}

/// Resolve a socket address to a human-readable interface description.
///
/// Returns localized strings for loopback, wildcard, or the raw IP.
pub fn resolve_interface(addr: &std::net::SocketAddr) -> String {
    let s = i18n::strings();
    let ip = addr.ip();
    if ip.is_loopback() {
        s.iface_loopback.into()
    } else if ip.is_unspecified() {
        s.iface_all.into()
    } else {
        format!("{ip}")
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ConnectionState, ProcessInfo, Protocol};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    // ── Helpers ───────────────────────────────────────────────────

    fn make_entry(port: u16, pid: u32, name: &str) -> PortEntry {
        make_entry_full(
            port,
            pid,
            name,
            Protocol::Tcp,
            ConnectionState::Listen,
            None,
        )
    }

    fn make_entry_full(
        port: u16,
        pid: u32,
        name: &str,
        proto: Protocol,
        state: ConnectionState,
        user: Option<&str>,
    ) -> PortEntry {
        PortEntry {
            protocol: proto,
            local_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            remote_addr: None,
            state,
            process: ProcessInfo {
                pid,
                name: name.to_string(),
                path: None,
                cmdline: None,
                user: user.map(String::from),
                parent_pid: None,
                parent_name: None,
            },
        }
    }

    fn make_entry_with_remote(
        port: u16,
        pid: u32,
        name: &str,
        remote_port: u16,
        state: ConnectionState,
    ) -> PortEntry {
        let mut entry = make_entry_full(port, pid, name, Protocol::Tcp, state, None);
        entry.remote_addr = Some(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            remote_port,
        ));
        entry
    }

    fn make_tracked(port: u16, pid: u32, name: &str, status: EntryStatus) -> TrackedEntry {
        TrackedEntry {
            entry: make_entry(port, pid, name),
            status,
            seen_at: Instant::now(),
            first_seen: None,
            suspicious: Vec::new(),
            container_name: None,
            service_name: None,
        }
    }

    /// Create a TrackedEntry with a custom PortEntry (for sort/filter tests
    /// that need non-default protocol/state/user).
    fn make_tracked_custom(entry: PortEntry, status: EntryStatus) -> TrackedEntry {
        TrackedEntry {
            entry,
            status,
            seen_at: Instant::now(),
            first_seen: None,
            suspicious: Vec::new(),
            container_name: None,
            service_name: None,
        }
    }

    // ── diff_entries: table-driven ────────────────────────────────

    #[test]
    fn diff_empty_prev_all_new() {
        let current = vec![make_entry(80, 1, "nginx"), make_entry(443, 2, "nginx")];
        let result = diff_entries(&[], current, Instant::now());
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|e| e.status == EntryStatus::New));
    }

    #[test]
    fn diff_empty_current_all_gone() {
        let prev = vec![make_tracked(80, 1, "nginx", EntryStatus::Unchanged)];
        let result = diff_entries(&prev, vec![], Instant::now());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].status, EntryStatus::Gone);
    }

    #[test]
    fn diff_unchanged_entries() {
        let prev = vec![make_tracked(80, 1, "nginx", EntryStatus::Unchanged)];
        let current = vec![make_entry(80, 1, "nginx")];
        let result = diff_entries(&prev, current, Instant::now());
        assert_eq!(result[0].status, EntryStatus::Unchanged);
    }

    #[test]
    fn diff_already_gone_not_duplicated() {
        let prev = vec![make_tracked(80, 1, "nginx", EntryStatus::Gone)];
        let result = diff_entries(&prev, vec![], Instant::now());
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn diff_mixed_new_unchanged_gone() {
        let prev = vec![
            make_tracked(80, 1, "nginx", EntryStatus::Unchanged),
            make_tracked(443, 2, "apache", EntryStatus::Unchanged),
        ];
        // Port 80/pid1 stays, port 443/pid2 gone, port 8080/pid3 new
        let current = vec![make_entry(80, 1, "nginx"), make_entry(8080, 3, "node")];
        let result = diff_entries(&prev, current, Instant::now());
        assert_eq!(result.len(), 3);

        let statuses: Vec<(u16, EntryStatus)> = result
            .iter()
            .map(|e| (e.entry.local_port(), e.status))
            .collect();
        assert!(statuses.contains(&(80, EntryStatus::Unchanged)));
        assert!(statuses.contains(&(8080, EntryStatus::New)));
        assert!(statuses.contains(&(443, EntryStatus::Gone)));
    }

    #[test]
    fn diff_same_port_different_pid_is_new() {
        // Port 80 with pid 1 goes away, port 80 with pid 2 appears — different identity
        let prev = vec![make_tracked(80, 1, "nginx", EntryStatus::Unchanged)];
        let current = vec![make_entry(80, 2, "apache")];
        let result = diff_entries(&prev, current, Instant::now());
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|e| e.status == EntryStatus::New));
        assert!(result.iter().any(|e| e.status == EntryStatus::Gone));
    }

    #[test]
    fn diff_same_pid_different_port() {
        let prev = vec![make_tracked(80, 1, "nginx", EntryStatus::Unchanged)];
        let current = vec![make_entry(443, 1, "nginx")];
        let result = diff_entries(&prev, current, Instant::now());
        assert!(result
            .iter()
            .any(|e| e.entry.local_port() == 443 && e.status == EntryStatus::New));
        assert!(result
            .iter()
            .any(|e| e.entry.local_port() == 80 && e.status == EntryStatus::Gone));
    }

    #[test]
    fn diff_same_pid_port_but_different_remote_is_new() {
        let prev = vec![TrackedEntry {
            entry: make_entry_with_remote(443, 42, "nginx", 51000, ConnectionState::Established),
            status: EntryStatus::Unchanged,
            seen_at: Instant::now(),
            first_seen: None,
            suspicious: Vec::new(),
            container_name: None,
            service_name: None,
        }];
        let current = vec![make_entry_with_remote(
            443,
            42,
            "nginx",
            51001,
            ConnectionState::Established,
        )];
        let result = diff_entries(&prev, current, Instant::now());
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|e| e.status == EntryStatus::New));
        assert!(result.iter().any(|e| e.status == EntryStatus::Gone));
    }

    // ── sort_entries: table-driven per column ─────────────────────

    #[test]
    fn sort_by_port_ascending() {
        let mut entries = vec![
            make_tracked(443, 1, "nginx", EntryStatus::Unchanged),
            make_tracked(80, 2, "apache", EntryStatus::Unchanged),
            make_tracked(8080, 3, "node", EntryStatus::Unchanged),
        ];
        sort_entries(
            &mut entries,
            &SortState {
                column: SortColumn::Port,
                ascending: true,
            },
        );
        let ports: Vec<u16> = entries.iter().map(|e| e.entry.local_port()).collect();
        assert_eq!(ports, vec![80, 443, 8080]);
    }

    #[test]
    fn sort_by_port_descending() {
        let mut entries = vec![
            make_tracked(80, 1, "a", EntryStatus::Unchanged),
            make_tracked(443, 2, "b", EntryStatus::Unchanged),
            make_tracked(8080, 3, "c", EntryStatus::Unchanged),
        ];
        sort_entries(
            &mut entries,
            &SortState {
                column: SortColumn::Port,
                ascending: false,
            },
        );
        let ports: Vec<u16> = entries.iter().map(|e| e.entry.local_port()).collect();
        assert_eq!(ports, vec![8080, 443, 80]);
    }

    #[test]
    fn sort_by_pid() {
        let mut entries = vec![
            make_tracked(80, 300, "c", EntryStatus::Unchanged),
            make_tracked(443, 100, "a", EntryStatus::Unchanged),
            make_tracked(8080, 200, "b", EntryStatus::Unchanged),
        ];
        sort_entries(
            &mut entries,
            &SortState {
                column: SortColumn::Pid,
                ascending: true,
            },
        );
        let pids: Vec<u32> = entries.iter().map(|e| e.entry.process.pid).collect();
        assert_eq!(pids, vec![100, 200, 300]);
    }

    #[test]
    fn sort_by_process_name() {
        let mut entries = vec![
            make_tracked(80, 1, "nginx", EntryStatus::Unchanged),
            make_tracked(443, 2, "apache", EntryStatus::Unchanged),
            make_tracked(8080, 3, "caddy", EntryStatus::Unchanged),
        ];
        sort_entries(
            &mut entries,
            &SortState {
                column: SortColumn::ProcessName,
                ascending: true,
            },
        );
        let names: Vec<&str> = entries
            .iter()
            .map(|e| e.entry.process.name.as_str())
            .collect();
        assert_eq!(names, vec!["apache", "caddy", "nginx"]);
    }

    #[test]
    fn sort_by_protocol() {
        let mut entries = vec![
            make_tracked_custom(
                make_entry_full(80, 1, "a", Protocol::Udp, ConnectionState::Unknown, None),
                EntryStatus::Unchanged,
            ),
            make_tracked_custom(
                make_entry_full(443, 2, "b", Protocol::Tcp, ConnectionState::Listen, None),
                EntryStatus::Unchanged,
            ),
        ];
        sort_entries(
            &mut entries,
            &SortState {
                column: SortColumn::Protocol,
                ascending: true,
            },
        );
        assert_eq!(entries[0].entry.protocol, Protocol::Tcp);
        assert_eq!(entries[1].entry.protocol, Protocol::Udp);
    }

    #[test]
    fn sort_by_user() {
        let mut entries = vec![
            make_tracked_custom(
                make_entry_full(
                    80,
                    1,
                    "a",
                    Protocol::Tcp,
                    ConnectionState::Listen,
                    Some("zoe"),
                ),
                EntryStatus::Unchanged,
            ),
            make_tracked_custom(
                make_entry_full(
                    443,
                    2,
                    "b",
                    Protocol::Tcp,
                    ConnectionState::Listen,
                    Some("alice"),
                ),
                EntryStatus::Unchanged,
            ),
        ];
        sort_entries(
            &mut entries,
            &SortState {
                column: SortColumn::User,
                ascending: true,
            },
        );
        assert_eq!(entries[0].entry.process.user.as_deref(), Some("alice"));
        assert_eq!(entries[1].entry.process.user.as_deref(), Some("zoe"));
    }

    #[test]
    fn sort_empty_slice_no_panic() {
        let mut entries: Vec<TrackedEntry> = vec![];
        sort_entries(&mut entries, &SortState::default());
        assert!(entries.is_empty());
    }

    // ── filter: table-driven ──────────────────────────────────────

    #[test]
    fn filter_empty_query_returns_all() {
        let entries = vec![
            make_tracked(80, 1, "nginx", EntryStatus::Unchanged),
            make_tracked(443, 2, "apache", EntryStatus::Unchanged),
        ];
        assert_eq!(filter_entries(&entries, "").len(), 2);
    }

    #[test]
    fn filter_by_port() {
        let entries = vec![
            make_tracked(80, 1, "nginx", EntryStatus::Unchanged),
            make_tracked(443, 2, "apache", EntryStatus::Unchanged),
            make_tracked(8080, 3, "node", EntryStatus::Unchanged),
        ];
        // "80" matches port 80 and 8080
        assert_eq!(filter_entries(&entries, "80").len(), 2);
    }

    #[test]
    fn filter_case_insensitive() {
        let entries = vec![make_tracked(80, 1, "Nginx", EntryStatus::Unchanged)];
        assert_eq!(filter_entries(&entries, "NGINX").len(), 1);
        assert_eq!(filter_entries(&entries, "nginx").len(), 1);
        assert_eq!(filter_entries(&entries, "nGiNx").len(), 1);
    }

    #[test]
    fn filter_by_pid() {
        let entries = vec![
            make_tracked(80, 1234, "nginx", EntryStatus::Unchanged),
            make_tracked(443, 5678, "apache", EntryStatus::Unchanged),
        ];
        assert_eq!(filter_entries(&entries, "1234").len(), 1);
        assert_eq!(filter_entries(&entries, "5678").len(), 1);
    }

    #[test]
    fn filter_by_protocol() {
        let entries = vec![
            make_tracked_custom(
                make_entry_full(80, 1, "a", Protocol::Tcp, ConnectionState::Listen, None),
                EntryStatus::Unchanged,
            ),
            make_tracked_custom(
                make_entry_full(53, 2, "b", Protocol::Udp, ConnectionState::Unknown, None),
                EntryStatus::Unchanged,
            ),
        ];
        assert_eq!(filter_entries(&entries, "udp").len(), 1);
        assert_eq!(filter_entries(&entries, "tcp").len(), 1);
    }

    #[test]
    fn filter_by_state() {
        let entries = vec![
            make_tracked_custom(
                make_entry_full(80, 1, "a", Protocol::Tcp, ConnectionState::Listen, None),
                EntryStatus::Unchanged,
            ),
            make_tracked_custom(
                make_entry_full(
                    81,
                    2,
                    "b",
                    Protocol::Tcp,
                    ConnectionState::Established,
                    None,
                ),
                EntryStatus::Unchanged,
            ),
        ];
        assert_eq!(filter_entries(&entries, "listen").len(), 1);
        assert_eq!(filter_entries(&entries, "established").len(), 1);
    }

    #[test]
    fn filter_by_user() {
        let entries = vec![
            make_tracked_custom(
                make_entry_full(
                    80,
                    1,
                    "a",
                    Protocol::Tcp,
                    ConnectionState::Listen,
                    Some("root"),
                ),
                EntryStatus::Unchanged,
            ),
            make_tracked_custom(
                make_entry_full(
                    81,
                    2,
                    "b",
                    Protocol::Tcp,
                    ConnectionState::Listen,
                    Some("www-data"),
                ),
                EntryStatus::Unchanged,
            ),
        ];
        assert_eq!(filter_entries(&entries, "root").len(), 1);
        assert_eq!(filter_entries(&entries, "www").len(), 1);
    }

    #[test]
    fn filter_no_match_returns_empty() {
        let entries = vec![make_tracked(80, 1, "nginx", EntryStatus::Unchanged)];
        assert_eq!(filter_entries(&entries, "zzz_no_match").len(), 0);
    }

    #[test]
    fn filter_bang_shows_only_suspicious() {
        use crate::model::SuspiciousReason;
        let mut suspicious_entry = make_tracked(80, 1, "python3", EntryStatus::Unchanged);
        suspicious_entry
            .suspicious
            .push(SuspiciousReason::ScriptOnSensitive);
        let clean_entry = make_tracked(8080, 2, "nginx", EntryStatus::Unchanged);
        let entries = vec![suspicious_entry, clean_entry];
        let filtered = filter_entries(&entries, "!");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].entry.process.name, "python3");
    }

    #[test]
    fn filter_indices_returns_correct_positions() {
        let entries = vec![
            make_tracked(80, 1, "nginx", EntryStatus::Unchanged),
            make_tracked(443, 2, "apache", EntryStatus::Unchanged),
            make_tracked(8080, 3, "nginx-proxy", EntryStatus::Unchanged),
        ];
        assert_eq!(filter_indices(&entries, "nginx"), vec![0, 2]);
    }

    #[test]
    fn filter_indices_empty_query() {
        let entries = vec![
            make_tracked(80, 1, "a", EntryStatus::Unchanged),
            make_tracked(443, 2, "b", EntryStatus::Unchanged),
        ];
        assert_eq!(filter_indices(&entries, ""), vec![0, 1]);
    }

    // ── export: table-driven ──────────────────────────────────────

    #[test]
    fn export_json_valid() {
        let entries = vec![make_entry(80, 1, "nginx")];
        let json = export(&entries, ExportFormat::Json).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 1);
    }

    #[test]
    fn export_json_empty() {
        let json = export(&[], ExportFormat::Json).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.as_array().unwrap().len(), 0);
    }

    #[test]
    fn export_csv_has_header_and_data() {
        let entries = vec![make_entry(80, 1, "nginx")];
        let csv_out = export(&entries, ExportFormat::Csv).unwrap();
        let lines: Vec<&str> = csv_out.lines().collect();
        assert!(lines.len() >= 2);
        assert!(lines[0].contains("protocol"));
        assert!(lines[0].contains("local_addr"));
        assert!(lines[1].contains("nginx"));
    }

    #[test]
    fn export_csv_empty() {
        let csv_out = export(&[], ExportFormat::Csv).unwrap();
        let lines: Vec<&str> = csv_out.lines().collect();
        // Header only
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("protocol"));
    }

    #[test]
    fn export_csv_multiple_entries() {
        let entries = vec![
            make_entry(80, 1, "nginx"),
            make_entry(443, 2, "apache"),
            make_entry(8080, 3, "node"),
        ];
        let csv_out = export(&entries, ExportFormat::Csv).unwrap();
        let lines: Vec<&str> = csv_out.lines().collect();
        assert_eq!(lines.len(), 4); // header + 3 rows
    }

    #[test]
    fn export_csv_sanitizes_formula_cells() {
        let mut entry = make_entry(80, 1, "=calc");
        entry.process.user = Some("+user".to_string());
        entry.process.parent_name = Some("-parent".to_string());
        entry.process.cmdline = Some("@cmd".to_string());

        let csv_out = export(&[entry], ExportFormat::Csv).unwrap();
        assert!(csv_out.contains("'=calc"));
        assert!(csv_out.contains("'+user"));
        assert!(csv_out.contains("'-parent"));
        assert!(csv_out.contains("'@cmd"));
    }

    #[test]
    fn export_csv_sanitizes_control_prefixed_formula_cells() {
        let mut entry = make_entry(80, 1, "\t=calc");
        entry.process.user = Some("\r+user".to_string());
        entry.process.parent_name = Some("\n-parent".to_string());
        entry.process.cmdline = Some("\u{0008}@cmd".to_string());

        let csv_out = export(&[entry], ExportFormat::Csv).unwrap();
        assert!(csv_out.contains("'\t=calc"));
        assert!(csv_out.contains("'\r+user"));
        assert!(csv_out.contains("'\n-parent"));
        assert!(csv_out.contains("'\u{0008}@cmd"));
    }

    #[test]
    fn export_json_contains_all_fields() {
        let entry = make_entry_full(
            443,
            42,
            "nginx",
            Protocol::Tcp,
            ConnectionState::Established,
            Some("www"),
        );
        let json = export(&[entry], ExportFormat::Json).unwrap();
        assert!(json.contains("443"));
        assert!(json.contains("42"));
        assert!(json.contains("nginx"));
        assert!(json.contains("Tcp"));
        assert!(json.contains("Established"));
        assert!(json.contains("www"));
    }

    // ── process_connections ────────────────────────────────────────

    #[test]
    fn process_connections_filters_by_pid() {
        let entries = vec![
            make_tracked(80, 1, "nginx", EntryStatus::Unchanged),
            make_tracked(443, 1, "nginx", EntryStatus::Unchanged),
            make_tracked(8080, 2, "node", EntryStatus::Unchanged),
        ];
        assert_eq!(process_connections(&entries, 1).len(), 2);
        assert_eq!(process_connections(&entries, 2).len(), 1);
        assert_eq!(process_connections(&entries, 999).len(), 0);
    }

    // ── resolve_interface ─────────────────────────────────────────

    #[test]
    fn resolve_interface_loopback() {
        let addr: SocketAddr = "127.0.0.1:80".parse().unwrap();
        let result = resolve_interface(&addr);
        assert!(!result.is_empty());
    }

    #[test]
    fn resolve_interface_wildcard() {
        let addr: SocketAddr = "0.0.0.0:80".parse().unwrap();
        let result = resolve_interface(&addr);
        assert!(!result.is_empty());
    }

    #[test]
    fn resolve_interface_specific_ip() {
        let addr: SocketAddr = "192.168.1.1:80".parse().unwrap();
        let result = resolve_interface(&addr);
        assert!(result.contains("192.168.1.1"));
    }

    #[test]
    fn resolve_interface_ipv6_loopback() {
        let addr: SocketAddr = "[::1]:80".parse().unwrap();
        let result = resolve_interface(&addr);
        assert!(!result.is_empty());
    }

    #[test]
    fn resolve_interface_ipv6_wildcard() {
        let addr: SocketAddr = "[::]:80".parse().unwrap();
        let result = resolve_interface(&addr);
        assert!(!result.is_empty());
    }

    // ── diff_entries: first_seen carry-forward ─────────────────────

    #[test]
    fn diff_new_entry_gets_first_seen() {
        let now = Instant::now();
        let result = diff_entries(&[], vec![make_entry(80, 1, "nginx")], now);
        assert_eq!(result[0].first_seen, Some(now));
    }

    #[test]
    fn diff_unchanged_carries_first_seen_forward() {
        let original_time = Instant::now();
        let mut prev = make_tracked(80, 1, "nginx", EntryStatus::Unchanged);
        prev.first_seen = Some(original_time);

        let later = original_time + Duration::from_secs(10);
        let result = diff_entries(&[prev], vec![make_entry(80, 1, "nginx")], later);
        assert_eq!(result[0].status, EntryStatus::Unchanged);
        assert_eq!(result[0].first_seen, Some(original_time));
    }

    #[test]
    fn diff_gone_preserves_first_seen() {
        let original_time = Instant::now();
        let mut prev = make_tracked(80, 1, "nginx", EntryStatus::Unchanged);
        prev.first_seen = Some(original_time);

        let later = original_time + Duration::from_secs(10);
        let result = diff_entries(&[prev], vec![], later);
        assert_eq!(result[0].status, EntryStatus::Gone);
        assert_eq!(result[0].first_seen, Some(original_time));
    }

    #[test]
    fn sort_by_service_none_sorts_last() {
        let mut entries = vec![
            make_tracked(80, 1, "nginx", EntryStatus::Unchanged),
            make_tracked(9999, 2, "custom", EntryStatus::Unchanged),
            make_tracked(443, 3, "nginx", EntryStatus::Unchanged),
        ];
        entries[0].service_name = Some("http".into());
        entries[1].service_name = None; // unknown
        entries[2].service_name = Some("https".into());
        sort_entries(
            &mut entries,
            &SortState {
                column: SortColumn::Service,
                ascending: true,
            },
        );
        // Named services first, None last
        assert_eq!(entries[0].service_name.as_deref(), Some("http"));
        assert_eq!(entries[1].service_name.as_deref(), Some("https"));
        assert_eq!(entries[2].service_name, None);
    }

    #[test]
    fn diff_unchanged_with_none_first_seen_gets_now() {
        // Simulates pre-upgrade entry with first_seen = None
        let mut prev = make_tracked(80, 1, "nginx", EntryStatus::Unchanged);
        prev.first_seen = None; // pre-upgrade: no first_seen

        let now = Instant::now();
        let result = diff_entries(&[prev], vec![make_entry(80, 1, "nginx")], now);
        assert_eq!(result[0].status, EntryStatus::Unchanged);
        // Should fill in `now` rather than leaving None forever
        assert_eq!(result[0].first_seen, Some(now));
    }

    // ── format_duration ──────────────────────────────────────────

    #[test]
    fn format_duration_table() {
        let cases = [
            (Duration::from_secs(0), "0s"),
            (Duration::from_secs(45), "45s"),
            (Duration::from_secs(60), "1m"),
            (Duration::from_secs(300), "5m"),
            (Duration::from_secs(3600), "1h"),
            (Duration::from_secs(7200), "2h"),
            (Duration::from_secs(86400), "1d"),
            (Duration::from_secs(259200), "3d"),
        ];
        for (dur, expected) in cases {
            assert_eq!(format_duration(dur), expected, "duration {:?}", dur);
        }
    }

    // ── build_process_tree ────────────────────────────────────────

    #[test]
    fn build_tree_unknown_pid() {
        let entries = vec![make_tracked(80, 1, "nginx", EntryStatus::Unchanged)];
        let tree = build_process_tree(&entries, 999);
        assert_eq!(tree.len(), 1); // "process not found"
    }

    #[test]
    fn build_tree_single_process() {
        let entries = vec![make_tracked(80, 1, "nginx", EntryStatus::Unchanged)];
        let tree = build_process_tree(&entries, 1);
        assert!(tree.len() >= 2); // process line + connection line
        assert!(tree[0].contains("nginx"));
        assert!(tree[1].contains(":80"));
    }

    #[test]
    fn build_tree_with_parent() {
        let mut entry = make_tracked(80, 2, "worker", EntryStatus::Unchanged);
        entry.entry.process.parent_pid = Some(1);
        entry.entry.process.parent_name = Some("master".into());
        let entries = vec![entry];
        let tree = build_process_tree(&entries, 2);
        assert!(tree.iter().any(|l| l.contains("master")));
        assert!(tree.iter().any(|l| l.contains("worker")));
    }

    #[test]
    fn build_tree_multiple_ports() {
        let entries = vec![
            make_tracked(80, 1, "nginx", EntryStatus::Unchanged),
            make_tracked(443, 1, "nginx", EntryStatus::Unchanged),
        ];
        let tree = build_process_tree(&entries, 1);
        assert!(tree.iter().any(|l| l.contains(":80")));
        assert!(tree.iter().any(|l| l.contains(":443")));
    }
}
