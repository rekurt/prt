# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-04-05

### Added

**Data Enrichment:**
- Known ports database — `Service` column shows http, ssh, postgres, etc. (~200 built-in entries, user overrides via `~/.config/prt/config.toml`)
- Connection aging — tracks `first_seen` per connection; ESTABLISHED >1h highlighted yellow, >24h red, CLOSE_WAIT always red
- Suspicious connection detector — `[!]` tag for non-root on privileged ports, scripting languages on sensitive ports, root outgoing to high ports; filter with `!`
- Container awareness — shows Docker/Podman container name per process (column auto-hides when no containers detected)
- Bandwidth estimation — system-wide RX/TX rate in header bar (Linux: `/proc/net/dev`, macOS: `netstat -ib`)
- Sparkline history — per-connection trend mini-chart (visible at terminal width >140)

**New Views (fullscreen, press 4-7):**
- Chart view (`4`) — horizontal bar chart of connections per process
- Topology view (`5`) — ASCII graph: process → port → remote address
- Process detail view (`6`) — comprehensive page: CWD, CPU, RSS, open files, env vars, connections, network info, process tree
- Namespaces view (`7`) — network namespace grouping (Linux only)

**New Actions:**
- Firewall quick-block (`b`) — block remote IP via iptables/pfctl with confirmation dialog and undo command
- Strace/dtruss attach (`t`) — live syscall tracing in split panel with auto-detach
- SSH port forwarding (`f`) — tunnel manager with health monitoring

**New CLI Modes:**
- `prt --json` — NDJSON streaming mode for scripting/piping
- `prt watch <ports>` — compact UP/DOWN monitor with BEL on state changes

**Configuration:**
- `~/.config/prt/config.toml` — known port overrides, alert rules
- Alert rules — `[[alerts]]` sections with port/process/state/connections_gt conditions, bell or highlight actions

**UI Improvements:**
- ViewMode architecture — Table (default) + 4 fullscreen views, Esc returns to table
- Context-sensitive footer — hints change based on current view mode and state
- Scrollable fullscreen views — j/k navigation in all fullscreen modes
- Focus stability — selection stays on same process after refresh/re-sort (identity-based tracking via port+pid key)
- Adaptive column layout — Service and Container columns appear based on terminal width

### Changed
- DetailTab reduced from 7 to 3 variants (Tree, Interface, Connection) — other views promoted to fullscreen
- Bottom detail panel now only shows in Table mode for the selected process
- Keys 1-3 switch detail tabs; keys 4-7 toggle fullscreen views
- Navigation (j/k) works in all view modes (scroll in fullscreen, selection in table)
- Scroll offset resets when switching between views

### Fixed
- Focus jumping on refresh — selection now tracks by (port, pid) identity instead of raw index
- Firewall block always passed None for sudo password — now correctly uses Session::sudo_password()
- Tracer stderr capture used expect() — replaced with proper error handling

## [0.2.0] - 2025-03-20

### Added
- Workspace architecture: `prt-core` library + `prt` TUI binary
- Multilingual support: English (default), Russian, Chinese
- Auto-detect system locale (`PRT_LANG` env or `--lang` flag override)
- Process tree view with parent chain
- Network interface details tab
- Connection details tab with all process ports
- Sudo password input from TUI for elevated scanning
- Session struct for shared scan/diff/sort logic
- Live change tracking: new (green), gone (red, 5s fade)
- Filter by port, process, PID, protocol, state, user
- Sort by any column (Tab/Shift+Tab)
- Kill process with confirmation (SIGTERM/SIGKILL)
- Copy to clipboard (line or PID)
- Export to JSON/CSV (`--export json|csv`)
- Batch `ps` calls on macOS (2 calls per scan, not 4*N)
- Panic hook for terminal recovery
- CI/CD with GitHub Actions (lint, test, release)
- cargo-deny for dependency auditing
