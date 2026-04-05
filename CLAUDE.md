# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build --workspace                    # build everything
cargo test --workspace                     # run all tests (188 tests)
cargo test -p prt-core core::scanner       # tests for specific module
cargo test -- --nocapture                  # show println output
cargo clippy --workspace --all-targets     # lint
cargo fmt --all -- --check                 # format check
cargo bench -p prt-core                    # criterion benchmarks
```

Note: `cargo` may require `export PATH="$HOME/.cargo/bin:$PATH"` on this machine.

## Architecture

Network port monitor with TUI interface (ratatui) for macOS and Linux. Workspace with 2 crates:

- **prt-core** — library: model, scanner, killer, platform abstraction, i18n, session, config, known ports, alerts, suspicious detection, bandwidth, containers, namespaces, process detail, firewall, history
- **prt** — TUI binary (ratatui + crossterm + clap) with stream/watch/tracer/forward modules

**Data flow:** `platform::scan_ports()` → `Session::refresh()` → `scanner::diff_entries()` (tracks New/Unchanged/Gone with first_seen carry-forward) → enrich (service names, suspicious, containers) → `scanner::sort_entries()` → `scanner::filter_indices()` → `alerts::evaluate()` → UI renders (ViewMode-based routing)

**Key design decisions:**
- Platform abstraction via `platform/mod.rs` with `#[cfg(target_os)]` — macOS uses `lsof` output parsing, Linux uses `/proc` via `procfs` crate
- `PortEntry` is the core data type; `TrackedEntry` wraps it with status (New/Unchanged/Gone), timestamp, and enrichment fields (first_seen, suspicious, container_name, service_name)
- Entry identity key is `(port, pid)` tuple — used in `diff_entries()` and focus stability (selection tracks by identity, not index)
- `Session` struct encapsulates the refresh/diff/retain/sort cycle — shared logic that UI delegates to
- `ViewMode` enum controls fullscreen views (Table/Chart/Topology/ProcessDetail/Namespaces); `DetailTab` enum controls bottom panel tabs (Tree/Interface/Connection)
- `ExportFormat` in core has no clap dependency; binary crate wraps it with `clap::ValueEnum`
- Gone entries are retained for 5 seconds before removal; auto-refresh every 2 seconds
- Config from `~/.config/prt/config.toml` — optional, missing file = defaults, parse error = stderr warning + defaults
- Error handling: `anyhow::Result` throughout, UI shows errors as status messages
- Caching: process detail and namespace data cached per-refresh (not per-frame) in App

**i18n system:** `prt-core/src/i18n/` — static `Strings` structs per language (en, ru, zh), `AtomicU8` for global state. Language set via `--lang` flag, `PRT_LANG` env, or auto-detected from system locale. Compile-time completeness check: adding a field to `Strings` forces all language files to be updated.

**macOS performance:** `platform/macos.rs` uses batch `ps` calls (`batch_ps_info`, `batch_parent_names`) — 2 total ps invocations per scan cycle instead of 4*N. This is critical for responsiveness with many connections.

**Shared constants:** `TICK_RATE` and `GONE_RETENTION` are defined in `model.rs`.

## Testing Patterns

Tests are inline `#[cfg(test)] mod tests` in each module (180 in prt-core, 8 in prt). Helper functions `make_entry()` / `make_tracked()` create test data with minimal required fields. Platform-specific parsing tests (macos.rs) run only on macOS via `#[cfg(target_os = "macos")]`.
