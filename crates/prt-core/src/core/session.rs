//! Session management — encapsulates the refresh/diff/retain/sort cycle.
//!
//! [`Session`] is the single point of truth for scan state. The TUI
//! delegates all data operations to it, keeping the UI layer thin.

use crate::config::PrtConfig;
use crate::core::bandwidth::BandwidthTracker;
use crate::core::{container, scanner, suspicious};
use crate::i18n;
use crate::known_ports;
use crate::model::{EntryStatus, SortState, TrackedEntry, GONE_RETENTION};
use std::time::Instant;

/// Shared scan session state used by the TUI app.
///
/// Encapsulates the full refresh cycle:
/// `scan → diff → enrich → retain(gone) → sort`
///
/// Tracks whether sudo authentication was completed successfully.
/// Subsequent refreshes use cached sudo credentials (`sudo -n`)
/// without storing the password in memory.
pub struct Session {
    pub entries: Vec<TrackedEntry>,
    pub sort: SortState,
    pub is_elevated: bool,
    pub is_root: bool,
    pub config: PrtConfig,
    pub bandwidth: BandwidthTracker,
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
            bandwidth: BandwidthTracker::new(),
        }
    }

    /// Run a scan cycle: scan → diff → enrich → retain → sort.
    pub fn refresh(&mut self) -> Result<(), String> {
        self.sync_elevation_state(scanner::has_elevated_access());

        let scan_result = if self.is_elevated {
            scanner::scan_elevated()
        } else {
            scanner::scan()
        };

        match scan_result {
            Ok(new_entries) => {
                self.sync_elevation_state(scanner::has_elevated_access());
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

    pub fn filtered_indices(&self, query: &str) -> Vec<usize> {
        scanner::filter_indices(&self.entries, query)
    }

    fn sync_elevation_state(&mut self, has_elevated_access: bool) {
        if self.is_elevated && !has_elevated_access {
            self.is_elevated = false;
            self.is_root = scanner::is_root();
        }
    }

    /// Attempt sudo elevation with password. Returns status message.
    pub fn try_sudo(&mut self, password: &str) -> String {
        let s = i18n::strings();
        match scanner::scan_with_sudo(password) {
            Ok(new_entries) => {
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
                self.bandwidth.sample();

                scanner::sort_entries(&mut self.entries, &self.sort);
                s.sudo_elevated.to_string()
            }
            Err(e) => {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_elevation_state_clears_expired_sudo_cache() {
        let mut session = Session::new();
        session.is_elevated = true;
        session.is_root = true;

        session.sync_elevation_state(false);

        assert!(!session.is_elevated);
        assert_eq!(session.is_root, scanner::is_root());
    }
}
