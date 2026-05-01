//! Core data types for the port monitor.
//!
//! This module defines all shared types used across the scanner, session,
//! and UI layers. The key types are:
//!
//! - [`PortEntry`] — a single network connection with process info
//! - [`TrackedEntry`] — a `PortEntry` enriched with change-tracking status
//! - [`SortState`] — current sort column and direction
//! - [`ExportFormat`] — output format for CLI export

use serde::Serialize;
use std::fmt;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Auto-refresh interval for the TUI. The UI polls for new scan data
/// at this rate.
pub const TICK_RATE: Duration = Duration::from_secs(2);

/// How long a "Gone" entry stays visible before removal.
/// Gives the user time to notice a connection disappeared.
pub const GONE_RETENTION: Duration = Duration::from_secs(5);

/// Network transport protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum Protocol {
    Tcp,
    Udp,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::Tcp => write!(f, "TCP"),
            Protocol::Udp => write!(f, "UDP"),
        }
    }
}

/// TCP connection state.
///
/// Matches standard TCP FSM states plus `Unknown` for UDP or unparsable states.
/// Display format uses uppercase with underscores (e.g. `TIME_WAIT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum ConnectionState {
    Listen,
    Established,
    TimeWait,
    CloseWait,
    SynSent,
    SynRecv,
    FinWait1,
    FinWait2,
    Closing,
    LastAck,
    Closed,
    Unknown,
}

impl fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ConnectionState::Listen => "LISTEN",
            ConnectionState::Established => "ESTABLISHED",
            ConnectionState::TimeWait => "TIME_WAIT",
            ConnectionState::CloseWait => "CLOSE_WAIT",
            ConnectionState::SynSent => "SYN_SENT",
            ConnectionState::SynRecv => "SYN_RECV",
            ConnectionState::FinWait1 => "FIN_WAIT1",
            ConnectionState::FinWait2 => "FIN_WAIT2",
            ConnectionState::Closing => "CLOSING",
            ConnectionState::LastAck => "LAST_ACK",
            ConnectionState::Closed => "CLOSED",
            ConnectionState::Unknown => "UNKNOWN",
        };
        write!(f, "{s}")
    }
}

/// Information about the process that owns a network connection.
///
/// Fields like `path`, `cmdline`, `parent_pid`, and `parent_name` are
/// populated via a secondary `ps` call (macOS) or `/proc` (Linux)
/// and may be `None` if the process has exited or access is denied.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    /// Process ID.
    pub pid: u32,
    /// Short process name (e.g. "nginx").
    pub name: String,
    /// Full path to the executable, if available.
    pub path: Option<PathBuf>,
    /// Full command line, if available.
    pub cmdline: Option<String>,
    /// Username of the process owner.
    pub user: Option<String>,
    /// Parent process ID.
    pub parent_pid: Option<u32>,
    /// Parent process name, resolved from `parent_pid`.
    pub parent_name: Option<String>,
}

/// A single network port entry.
///
/// This is the fundamental data unit — one row in the port table.
/// Identity key for change tracking is `(local_port, pid)`.
#[derive(Debug, Clone, Serialize)]
pub struct PortEntry {
    /// Transport protocol (TCP or UDP).
    pub protocol: Protocol,
    /// Local socket address (ip:port).
    pub local_addr: SocketAddr,
    /// Remote socket address, if connected.
    pub remote_addr: Option<SocketAddr>,
    /// Connection state (LISTEN, ESTABLISHED, etc.).
    pub state: ConnectionState,
    /// Process that owns this connection.
    pub process: ProcessInfo,
}

impl PortEntry {
    /// Returns the local port number.
    pub fn local_port(&self) -> u16 {
        self.local_addr.port()
    }
}

/// Change-tracking status for a port entry between scan cycles.
///
/// Used by [`crate::core::scanner::diff_entries`] to compute what changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryStatus {
    /// Entry existed in previous scan and still exists.
    Unchanged,
    /// Entry is new since last scan.
    New,
    /// Entry disappeared since last scan. Will be retained for
    /// [`GONE_RETENTION`] seconds before removal.
    Gone,
}

/// A [`PortEntry`] with change-tracking metadata and enrichment data.
///
/// The `status` field indicates whether the entry is new, unchanged,
/// or gone since the last scan. `seen_at` records when the status
/// was last updated.
///
/// Enrichment fields (`first_seen`, `suspicious`, `container_name`,
/// `service_name`) are populated lazily by the corresponding modules
/// after the diff step. They use `Option` / `Vec` to be zero-cost
/// when the corresponding feature is not active.
#[derive(Debug, Clone)]
pub struct TrackedEntry {
    /// The underlying port entry.
    pub entry: PortEntry,
    /// Current change status.
    pub status: EntryStatus,
    /// When this status was assigned.
    pub seen_at: Instant,

    // ── Enrichment fields ────────────────────────────────────────
    /// When this connection was first observed (carried forward across
    /// scan cycles for Unchanged entries). Used for connection aging.
    pub first_seen: Option<Instant>,
    /// Suspicious activity reasons detected by heuristics.
    pub suspicious: Vec<SuspiciousReason>,
    /// Docker/Podman container name, if the process runs inside one.
    pub container_name: Option<String>,
    /// Well-known service name for the port (e.g. "http", "ssh").
    pub service_name: Option<String>,
}

/// Reason why a connection was flagged as suspicious.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuspiciousReason {
    /// Non-root process listening on a privileged port (< 1024).
    NonRootPrivileged,
    /// Scripting language (python, perl, ruby, node) on a sensitive port.
    ScriptOnSensitive,
    /// Root process making outgoing connection to a high port.
    RootHighPortOutgoing,
}

/// Column by which the port table can be sorted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Port,
    Service,
    Protocol,
    State,
    Pid,
    ProcessName,
    User,
}

/// Current sorting configuration: which column and direction.
///
/// Toggle behavior: pressing the same column flips direction;
/// pressing a different column switches to it ascending.
#[derive(Debug, Clone, Copy)]
pub struct SortState {
    /// Column to sort by.
    pub column: SortColumn,
    /// `true` = ascending (A→Z, 0→9), `false` = descending.
    pub ascending: bool,
}

impl Default for SortState {
    fn default() -> Self {
        Self {
            column: SortColumn::Port,
            ascending: true,
        }
    }
}

impl SortState {
    /// Toggle sorting: same column flips direction, different column
    /// switches to ascending.
    pub fn toggle(&mut self, col: SortColumn) {
        if self.column == col {
            self.ascending = !self.ascending;
        } else {
            self.column = col;
            self.ascending = true;
        }
    }
}

/// Top-level section. Tab / Shift+Tab cycles between sections.
///
/// `Connections` is the default: port table + bottom Details panel.
/// `Processes` shows the selected entry's process detail / topology.
/// `Ssh` aggregates SSH hosts and active tunnels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    /// Connections: port table + Details panel.
    #[default]
    Connections,
    /// Processes: Detail / Topology sub-tabs.
    Processes,
    /// SSH: Hosts / Tunnels sub-tabs.
    Ssh,
}

impl ViewMode {
    pub const ALL: &[ViewMode] = &[ViewMode::Connections, ViewMode::Processes, ViewMode::Ssh];

    fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|&m| m == self)
            .expect("all ViewMode variants must be listed in ALL")
    }

    pub fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

/// Sub-tab inside the Processes section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcessesTab {
    #[default]
    Detail,
    Topology,
}

impl ProcessesTab {
    pub const ALL: &[ProcessesTab] = &[ProcessesTab::Detail, ProcessesTab::Topology];

    pub fn next(self) -> Self {
        match self {
            ProcessesTab::Detail => ProcessesTab::Topology,
            ProcessesTab::Topology => ProcessesTab::Detail,
        }
    }

    pub fn prev(self) -> Self {
        self.next()
    }
}

/// Sub-tab inside the SSH section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SshTab {
    #[default]
    Hosts,
    Tunnels,
}

impl SshTab {
    pub const ALL: &[SshTab] = &[SshTab::Hosts, SshTab::Tunnels];

    pub fn next(self) -> Self {
        match self {
            SshTab::Hosts => SshTab::Tunnels,
            SshTab::Tunnels => SshTab::Hosts,
        }
    }

    pub fn prev(self) -> Self {
        self.next()
    }
}

/// Output format for CLI export mode (`--export`).
///
/// Note: this enum intentionally does not derive `clap::ValueEnum` to keep
/// `prt-core` free of CLI dependencies. The binary crate wraps it with
/// `CliExportFormat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Csv,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn make_process() -> ProcessInfo {
        ProcessInfo {
            pid: 1,
            name: "test".into(),
            path: None,
            cmdline: None,
            user: None,
            parent_pid: None,
            parent_name: None,
        }
    }

    #[test]
    fn local_port_returns_port_from_addr() {
        let entry = PortEntry {
            protocol: Protocol::Tcp,
            local_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080),
            remote_addr: None,
            state: ConnectionState::Listen,
            process: make_process(),
        };
        assert_eq!(entry.local_port(), 8080);
    }

    #[test]
    fn sort_state_default_is_port_ascending() {
        let s = SortState::default();
        assert_eq!(s.column, SortColumn::Port);
        assert!(s.ascending);
    }

    #[test]
    fn sort_state_toggle_same_column_flips_direction() {
        let mut s = SortState::default();
        s.toggle(SortColumn::Port);
        assert!(!s.ascending);
        s.toggle(SortColumn::Port);
        assert!(s.ascending);
    }

    #[test]
    fn sort_state_toggle_different_column_resets_ascending() {
        let mut s = SortState::default();
        s.toggle(SortColumn::Port);
        s.toggle(SortColumn::Pid);
        assert_eq!(s.column, SortColumn::Pid);
        assert!(s.ascending);
    }

    #[test]
    fn protocol_display() {
        assert_eq!(Protocol::Tcp.to_string(), "TCP");
        assert_eq!(Protocol::Udp.to_string(), "UDP");
    }

    #[test]
    fn connection_state_display() {
        let cases = [
            (ConnectionState::Listen, "LISTEN"),
            (ConnectionState::Established, "ESTABLISHED"),
            (ConnectionState::TimeWait, "TIME_WAIT"),
            (ConnectionState::CloseWait, "CLOSE_WAIT"),
            (ConnectionState::SynSent, "SYN_SENT"),
            (ConnectionState::SynRecv, "SYN_RECV"),
            (ConnectionState::FinWait1, "FIN_WAIT1"),
            (ConnectionState::FinWait2, "FIN_WAIT2"),
            (ConnectionState::Closing, "CLOSING"),
            (ConnectionState::LastAck, "LAST_ACK"),
            (ConnectionState::Closed, "CLOSED"),
            (ConnectionState::Unknown, "UNKNOWN"),
        ];
        for (state, expected) in cases {
            assert_eq!(state.to_string(), expected, "state {:?}", state);
        }
    }

    #[test]
    fn view_mode_default_is_connections() {
        assert_eq!(ViewMode::default(), ViewMode::Connections);
    }

    #[test]
    fn view_mode_next_prev_cycle() {
        let cases = [
            (ViewMode::Connections, ViewMode::Processes),
            (ViewMode::Processes, ViewMode::Ssh),
            (ViewMode::Ssh, ViewMode::Connections),
        ];
        for (from, expected) in cases {
            assert_eq!(from.next(), expected);
            assert_eq!(expected.prev(), from);
        }
    }

    #[test]
    fn processes_tab_cycle() {
        assert_eq!(ProcessesTab::Detail.next(), ProcessesTab::Topology);
        assert_eq!(ProcessesTab::Topology.next(), ProcessesTab::Detail);
        assert_eq!(ProcessesTab::default(), ProcessesTab::Detail);
    }

    #[test]
    fn ssh_tab_cycle() {
        assert_eq!(SshTab::Hosts.next(), SshTab::Tunnels);
        assert_eq!(SshTab::Tunnels.next(), SshTab::Hosts);
        assert_eq!(SshTab::default(), SshTab::Hosts);
    }

    // ── SortState toggle table ────────────────────────────────────

    #[test]
    fn sort_state_toggle_all_columns() {
        let columns = [
            SortColumn::Port,
            SortColumn::Service,
            SortColumn::Protocol,
            SortColumn::State,
            SortColumn::Pid,
            SortColumn::ProcessName,
            SortColumn::User,
        ];
        for col in columns {
            let mut s = SortState::default();
            s.toggle(col);
            if col == SortColumn::Port {
                assert!(!s.ascending, "toggling same column should flip");
            } else {
                assert_eq!(s.column, col);
                assert!(s.ascending, "switching to {:?} should be ascending", col);
            }
        }
    }
}
