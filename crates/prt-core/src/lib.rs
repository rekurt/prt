//! # prt-core
//!
//! Core library for **prt** — a real-time network port monitor.
//!
//! This crate provides the platform-independent logic for scanning, tracking,
//! filtering, sorting, and exporting network port information. It is designed
//! to be consumed by any frontend (TUI, GUI, CLI).
//!
//! # Architecture
//!
//! ```text
//! platform::scan_ports()
//!     → Session::refresh()
//!         → scanner::diff_entries()   (New / Unchanged / Gone)
//!         → scanner::sort_entries()
//!         → scanner::filter_indices()
//!     → UI renders
//! ```
//!
//! # Modules
//!
//! - [`model`] — Core data types: [`PortEntry`](model::PortEntry),
//!   [`TrackedEntry`](model::TrackedEntry), [`SortState`](model::SortState), enums.
//! - [`core`] — Business logic: scanning, diffing, filtering, sorting, killing,
//!   session management.
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

pub mod core;
pub mod i18n;
pub mod model;
pub mod platform;
