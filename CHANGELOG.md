# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed (breaking)

- **Top-level navigation simplified.** `ViewMode` shrinks from
  `{ Table, Chart, Topology, ProcessDetail, Namespaces, SshHosts, Tunnels }`
  to three sections: `Connections`, `Processes`, `Ssh`.
  `Tab` / `Shift+Tab` cycles between sections.
- **Sub-tabs replace fullscreen modes.** Topology and ProcessDetail are
  sub-tabs of *Processes*; SSH Hosts and Tunnels are sub-tabs of *SSH*.
  Switch sub-tabs with `[` / `]`.
- **Sort moves off Tab.** `o` now picks the next sort column, `O` reverses
  direction. `Tab` is reserved for section navigation.
- **Per-action shortcuts collapse into a Space-key menu.** `b` (Block IP),
  `t` (Trace), `F` (SSH Forward), `p` (Copy PID), and the old fullscreen
  toggles `4`/`5`/`6`/`7`/`8`/`9` are removed. Use `Space` → choose
  action. Direct shortcuts remain only for `K` (Kill) and `c` (Copy).
- **Bottom Details panel is a single unified view** (no more 1/2/3
  Tree/Network/Connection tabs). Combines bind type, interface, remote,
  state, cmdline, related ports, and process tree in one scroll view.
- **Esc cascade is armed for filter clear.** First press shows
  "Esc again to clear filter"; a second press inside 1.5s clears.
  Same guard for the tunnel form when it has unsaved input.

### Added

- **Action menu** opened with `Space` — contextual list (Kill / Copy /
  Copy PID / Block IP / Trace / SSH forward) with j/k navigation, Enter
  to execute, 1..9 to jump.
- **Tunnel real status** — `TunnelStatus { Starting, Alive, Failed }`
  replaces the hard-coded "alive". Failed tunnels stay visible in the
  list (red) until the user restarts or removes them.
- **Tunnel edit mode** — `e` on the selected tunnel re-opens the form
  with all fields pre-filled; Enter replaces the tunnel in place.
- **Inline form validation** — bad fields turn red as you type, no need
  to wait for Enter.

### Removed

- **Chart fullscreen view** and its `4` shortcut.
- **Namespaces fullscreen view**, `7` shortcut, App `namespace_cache`,
  and the `prt_core::core::namespace` module.

## [0.3.0] - 2026-04-05

### Added

**Data Enrichment:**
- Known ports database — `Service` column shows http, ssh, postgres, etc. (~170 built-in entries, user overrides via `~/.config/prt/config.toml`)
- Connection aging — tracks `first_seen` per connection; ESTABLISHED >1h highlighted yellow, >24h red, CLOSE_WAIT always red
- Suspicious connection detector — `[!]` tag for non-root on privileged ports, scripting languages on sensitive ports, root outgoing to high ports; filter with `!`
- Container awareness — shows Docker/Podman container name per process (column auto-hides when no containers detected)
- Bandwidth estimation — system-wide RX/TX rate in header bar (Linux: `/proc/net/dev`, macOS: `netstat -ib`)

**New Views (fullscreen, press 4-7):**
- Chart view (`4`) — horizontal bar chart of connections per process
- Topology view (`5`) — ASCII graph: process → port → remote address
- Process detail view (`6`) — comprehensive page: CWD, CPU, RSS, open files, env vars, connections, network info, process tree
- Namespaces view (`7`) — network namespace grouping (Linux only)

**New Actions:**
- Firewall quick-block (`b`) — block remote IP via iptables/pfctl with confirmation dialog and undo command
- Strace/dtruss attach (`t`) — live syscall tracing in split panel with auto-detach
- SSH port forwarding (`F`) — create SSH -L tunnels from TUI with input dialog, active tunnel count in header, auto-cleanup on exit

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
