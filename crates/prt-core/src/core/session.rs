//! Session management — encapsulates the refresh/diff/retain/sort cycle.
//!
//! [`Session`] is the single point of truth for scan state. The TUI
//! delegates all data operations to it, keeping the UI layer thin.

use crate::config::PrtConfig;
use crate::core::bandwidth::BandwidthTracker;
use crate::core::history::ConnectionHistory;
use crate::core::{container, scanner, suspicious};
use crate::i18n;
use crate::known_ports;
use crate::model::{EntryStatus, SortState, TrackedEntry, GONE_RETENTION};
use std::collections::HashMap;
use std::time::Instant;

/// Shared scan session state used by the TUI app.
///
/// Encapsulates the full refresh cycle:
/// `scan → diff → enrich → retain(gone) → sort`
///
/// Stores sudo password (if elevated) so subsequent refreshes
/// can re-authenticate without user interaction.
pub struct Session {
    pub entries: Vec<TrackedEntry>,
    pub sort: SortState,
    pub is_elevated: bool,
    pub is_root: bool,
    pub config: PrtConfig,
    pub history: ConnectionHistory,
    pub bandwidth: BandwidthTracker,
    sudo_password: Option<String>,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            sort: SortState::default(),
            is_elevated: false,
            is_root: scanner::is_root(),
            config: crate::config::load_config(),
            history: ConnectionHistory::new(),
            bandwidth: BandwidthTracker::new(),
            sudo_password: None,
        }
    }

    /// Run a scan cycle: scan → diff → enrich → retain → sort.
    pub fn refresh(&mut self) -> Result<(), String> {
        let scan_result = if let Some(password) = &self.sudo_password {
            scanner::scan_with_sudo(password)
        } else {
            scanner::scan()
        };

        match scan_result {
            Ok(new_entries) => {
                let now = Instant::now();
                self.entries = scanner::diff_entries(&self.entries, new_entries, now);

                // ── Enrichment pipeline ──────────────────────────
                self.enrich_service_names();
                self.enrich_suspicious();
                self.enrich_containers();

                self.entries.retain(|e| {
                    e.status != EntryStatus::Gone || now.duration_since(e.seen_at) < GONE_RETENTION
                });

                // ── Metrics ─────────────────────────────────────
                self.update_history();
                self.bandwidth.sample();

                scanner::sort_entries(&mut self.entries, &self.sort);
                Ok(())
            }
            Err(e) => {
                let s = i18n::strings();
                Err(s.fmt_scan_error(&e.to_string()))
            }
        }
    }

    /// Populate service_name on all entries from the known ports DB.
    /// Skips Gone entries — they carry forward enrichment from `diff_entries`.
    fn enrich_service_names(&mut self) {
        for entry in &mut self.entries {
            if entry.status != EntryStatus::Gone {
                entry.service_name =
                    known_ports::lookup(entry.entry.local_port(), &self.config.known_ports);
            }
        }
    }

    /// Run suspicious-connection heuristics on all entries.
    /// Skips Gone entries — they carry forward enrichment from `diff_entries`.
    fn enrich_suspicious(&mut self) {
        for entry in &mut self.entries {
            if entry.status != EntryStatus::Gone {
                entry.suspicious = suspicious::check(&entry.entry);
            }
        }
    }

    /// Resolve Docker/Podman container names for all active entries.
    /// One batched CLI call per refresh cycle. Skips Gone entries.
    fn enrich_containers(&mut self) {
        let pids: Vec<u32> = self
            .entries
            .iter()
            .filter(|e| e.status != EntryStatus::Gone)
            .map(|e| e.entry.process.pid)
            .collect();

        let names = container::resolve_container_names(&pids);

        for entry in &mut self.entries {
            if entry.status != EntryStatus::Gone {
                entry.container_name = names.get(&entry.entry.process.pid).cloned();
            }
        }
    }

    /// Record connection counts per (port, pid) for sparkline history.
    fn update_history(&mut self) {
        let mut counts: HashMap<(u16, u32), u16> = HashMap::new();
        for e in &self.entries {
            if e.status != EntryStatus::Gone {
                *counts
                    .entry((e.entry.local_port(), e.entry.process.pid))
                    .or_insert(0) += 1;
            }
        }
        self.history.record(&counts);
    }

    /// Get cached sudo password (if elevated). Used by firewall block.
    pub fn sudo_password(&self) -> Option<&str> {
        self.sudo_password.as_deref()
    }

    pub fn filtered_indices(&self, query: &str) -> Vec<usize> {
        scanner::filter_indices(&self.entries, query)
    }

    /// Attempt sudo elevation with password. Returns status message.
    pub fn try_sudo(&mut self, password: &str) -> String {
        let s = i18n::strings();
        match scanner::scan_with_sudo(password) {
            Ok(new_entries) => {
                self.sudo_password = Some(password.to_string());
                self.is_elevated = true;
                self.is_root = true;
                let now = Instant::now();
                self.entries = scanner::diff_entries(&self.entries, new_entries, now);

                // Remove expired Gone entries (same as refresh)
                self.entries.retain(|e| {
                    e.status != EntryStatus::Gone || e.seen_at.elapsed() < GONE_RETENTION
                });

                // ── Enrichment pipeline (same as refresh) ───────
                self.enrich_service_names();
                self.enrich_suspicious();
                self.enrich_containers();

                // ── Metrics ─────────────────────────────────────
                self.update_history();
                self.bandwidth.sample();

                scanner::sort_entries(&mut self.entries, &self.sort);
                s.sudo_elevated.to_string()
            }
            Err(e) => {
                self.sudo_password = None;
                self.is_elevated = false;
                let msg = e.to_string();
                if msg.contains("incorrect password") || msg.contains("Sorry") {
                    s.sudo_wrong_password.to_string()
                } else {
                    s.fmt_sudo_error(&msg)
                }
            }
        }
    }
}
