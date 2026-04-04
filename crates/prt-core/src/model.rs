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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
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

/// A [`PortEntry`] with change-tracking metadata.
///
/// The `status` field indicates whether the entry is new, unchanged,
/// or gone since the last scan. `seen_at` records when the status
/// was last updated.
#[derive(Debug, Clone)]
pub struct TrackedEntry {
    /// The underlying port entry.
    pub entry: PortEntry,
    /// Current change status.
    pub status: EntryStatus,
    /// When this status was assigned.
    pub seen_at: Instant,
}

/// Column by which the port table can be sorted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Port,
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

/// Tab in the detail panel below the port table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailTab {
    /// Process tree view.
    Tree,
    /// Network interface info.
    Interface,
    /// Connection details.
    Connection,
}

impl DetailTab {
    /// Cycle to the next tab: Tree → Interface → Connection → Tree.
    pub fn next(self) -> Self {
        match self {
            Self::Tree => Self::Interface,
            Self::Interface => Self::Connection,
            Self::Connection => Self::Tree,
        }
    }

    /// Cycle to the previous tab: Tree → Connection → Interface → Tree.
    pub fn prev(self) -> Self {
        match self {
            Self::Tree => Self::Connection,
            Self::Interface => Self::Tree,
            Self::Connection => Self::Interface,
        }
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

    // ── DetailTab cycling ─────────────────────────────────────────

    #[test]
    fn detail_tab_next_cycles_forward() {
        let cases = [
            (DetailTab::Tree, DetailTab::Interface),
            (DetailTab::Interface, DetailTab::Connection),
            (DetailTab::Connection, DetailTab::Tree),
        ];
        for (from, expected) in cases {
            assert_eq!(from.next(), expected, "next of {:?}", from);
        }
    }

    #[test]
    fn detail_tab_prev_cycles_backward() {
        let cases = [
            (DetailTab::Tree, DetailTab::Connection),
            (DetailTab::Interface, DetailTab::Tree),
            (DetailTab::Connection, DetailTab::Interface),
        ];
        for (from, expected) in cases {
            assert_eq!(from.prev(), expected, "prev of {:?}", from);
        }
    }

    #[test]
    fn detail_tab_next_prev_roundtrip() {
        for tab in [DetailTab::Tree, DetailTab::Interface, DetailTab::Connection] {
            assert_eq!(tab.next().prev(), tab, "roundtrip {:?}", tab);
            assert_eq!(tab.prev().next(), tab, "reverse roundtrip {:?}", tab);
        }
    }

    // ── SortState toggle table ────────────────────────────────────

    #[test]
    fn sort_state_toggle_all_columns() {
        let columns = [
            SortColumn::Port,
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
