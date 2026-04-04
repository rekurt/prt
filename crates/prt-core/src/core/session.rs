//! Session management — encapsulates the refresh/diff/retain/sort cycle.
//!
//! [`Session`] is the single point of truth for scan state. The TUI
//! delegates all data operations to it, keeping the UI layer thin.

use crate::core::scanner;
use crate::i18n;
use crate::model::{EntryStatus, SortState, TrackedEntry, GONE_RETENTION};
use std::time::Instant;

/// Shared scan session state used by the TUI app.
///
/// Encapsulates the full refresh cycle:
/// `scan → diff → retain(gone) → sort`
///
/// Stores sudo password (if elevated) so subsequent refreshes
/// can re-authenticate without user interaction.
pub struct Session {
    pub entries: Vec<TrackedEntry>,
    pub sort: SortState,
    pub is_elevated: bool,
    pub is_root: bool,
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
            sudo_password: None,
        }
    }

    /// Run a scan cycle: scan -> diff -> retain -> sort.
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
                self.entries.retain(|e| {
                    e.status != EntryStatus::Gone || now.duration_since(e.seen_at) < GONE_RETENTION
                });
                scanner::sort_entries(&mut self.entries, &self.sort);
                Ok(())
            }
            Err(e) => {
                let s = i18n::strings();
                Err(s.fmt_scan_error(&e.to_string()))
            }
        }
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
