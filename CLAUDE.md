# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build --workspace                    # build everything
cargo test --workspace                     # run all tests
cargo test -p prt-core core::scanner       # tests for specific module
cargo test -- --nocapture                  # show println output
cargo clippy --workspace --all-targets     # lint
cargo fmt --all -- --check                 # format check
```

Note: `cargo` may require `export PATH="$HOME/.cargo/bin:$PATH"` on this machine.

## Architecture

Network port monitor with TUI interface (ratatui) for macOS and Linux. Workspace with 2 crates:

- **prt-core** — library: model, scanner, killer, platform abstraction, i18n, session
- **prt** — TUI binary (ratatui + crossterm + clap)

**Data flow:** `platform::scan_ports()` → `Session::refresh()` → `scanner::diff_entries()` (tracks New/Unchanged/Gone) → `scanner::sort_entries()` → `scanner::filter_indices()` → UI renders

**Key design decisions:**
- Platform abstraction via `platform/mod.rs` with `#[cfg(target_os)]` — macOS uses `lsof` output parsing, Linux uses `/proc` via `procfs` crate
- `PortEntry` is the core data type; `TrackedEntry` wraps it with status (New/Unchanged/Gone) and timestamp for change tracking
- Entry identity key is `(port, pid)` tuple — used in `diff_entries()` to detect changes between scan cycles
- `Session` struct encapsulates the refresh/diff/retain/sort cycle — shared logic that UI delegates to
- `ExportFormat` in core has no clap dependency; binary crate wraps it with `clap::ValueEnum`
- Gone entries are retained for 5 seconds before removal; auto-refresh every 2 seconds
- Error handling: `anyhow::Result` throughout, UI shows errors as status messages

**i18n system:** `prt-core/src/i18n/` — static `Strings` structs per language (en, ru, zh), `OnceLock<Lang>` for global state. Language set via `--lang` flag, `PRT_LANG` env, or auto-detected from system locale. Compile-time completeness check: adding a field to `Strings` forces all language files to be updated.

**macOS performance:** `platform/macos.rs` uses batch `ps` calls (`batch_ps_info`, `batch_parent_names`) — 2 total ps invocations per scan cycle instead of 4*N. This is critical for responsiveness with many connections.

**Shared constants:** `TICK_RATE` and `GONE_RETENTION` are defined in `model.rs`.

## Testing Patterns

Tests are inline `#[cfg(test)] mod tests` in each module. Helper functions `make_entry()` / `make_tracked()` create test data with minimal required fields. Platform-specific parsing tests (macos.rs) run only on macOS via `#[cfg(target_os = "macos")]`.
