//! # prt-core
//!
//! Core library for **prt** — a real-time network port monitor.
//!
//! This crate provides the platform-independent logic for scanning, tracking,
//! enriching, filtering, sorting, and exporting network port information. It is
//! designed to be consumed by any frontend (TUI, GUI, CLI).
//!
//! # Architecture
//!
//! ```text
//! platform::scan_ports()
//!     → Session::refresh()
//!         → scanner::diff_entries()       (New / Unchanged / Gone + first_seen)
//!         → enrich: service names, suspicious flags, containers
//!         → retain: drop Gone entries older than 5s
//!         → bandwidth.sample()
//!         → scanner::sort_entries()
//!     → (frontend layer)
//!         → alerts::evaluate()
//!         → scanner::filter_indices()
//!         → UI renders
//! ```
//!
//! # Modules
//!
//! - [`model`] — Core data types: [`PortEntry`](model::PortEntry),
//!   [`TrackedEntry`](model::TrackedEntry), [`ViewMode`](model::ViewMode),
//!   [`DetailTab`](model::DetailTab), [`SortState`](model::SortState).
//! - [`core`] — Business logic: scanning, diffing, filtering, sorting, killing,
//!   session management, alerts, suspicious detection, bandwidth tracking,
//!   container resolution, namespaces, process detail, firewall.
//! - [`config`] — TOML configuration from `~/.config/prt/` (known port overrides, alert rules).
//! - [`known_ports`] — Well-known port → service name database (~170 entries + user overrides).
//! - [`i18n`] — Internationalization: runtime-switchable language support
//!   (English, Russian, Chinese) backed by `AtomicU8`.
//! - [`platform`] — OS-specific port scanning: macOS (`lsof`), Linux (`/proc`).
//!
//! # Example
//!
//! ```no_run
//! use prt_core::core::scanner;
//! use prt_core::model::ExportFormat;
//!
//! let entries = scanner::scan().expect("scan failed");
//! let json = scanner::export(&entries, ExportFormat::Json).unwrap();
//! println!("{json}");
//! ```

pub mod config;
pub mod core;
pub mod i18n;
pub mod known_ports;
pub mod model;
pub mod platform;
