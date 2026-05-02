# prt

[![Crates.io](https://img.shields.io/crates/v/prt.svg)](https://crates.io/crates/prt)
[![CI](https://github.com/rekurt/prt/actions/workflows/ci.yml/badge.svg)](https://github.com/rekurt/prt/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/rekurt/prt/blob/master/LICENSE)

**Real-time terminal UI for monitoring network ports — interactive alternative to lsof/ss with colors, filtering and process trees.**

<img src="https://raw.githubusercontent.com/rekurt/prt/master/docs/prt.gif" alt="prt demo" width="720">

## Install

```bash
cargo install prt
```

**Requirements:** Rust 1.75+ · macOS 10.15+ or Linux with `/proc` · `lsof` (macOS — preinstalled)

## Features

| Feature | Description |
|---------|-------------|
| **Live table** | Ports, services, protocols, states, PIDs, processes, users. Auto-refreshes every 2s |
| **Change tracking** | New connections green; closed fade red for 5s |
| **Known ports** | Service column with ~170 built-in names + config overrides |
| **Connection aging** | Color-coded by age (>1h yellow, >24h red, CLOSE_WAIT always red) |
| **Suspicious detector** | `[!]` flags for non-root on privileged ports, scripts on sensitive ports |
| **Process tree** | Full parent chain (e.g. `launchd → nginx → worker`) |
| **Sections** | `Tab` cycles Connections / Processes / SSH; sub-tabs with `[` / `]` |
| **Details panel** | Single unified panel under the table — bind, iface, remote, state, cmdline, related ports, process tree |
| **Action menu** | `Space` → contextual list (Kill / Copy / Block / Trace / Forward) |
| **Search & filter** | By port, service, process, PID, protocol, state, user. `!` = suspicious. `Esc` twice to clear |
| **Kill** | `K` → `y` (SIGTERM) or `f` (SIGKILL) |
| **Firewall block** | `Space → Block IP` — adds rule + status-bar undo command |
| **Strace** | `Space → Trace` — live syscall stream in split panel |
| **SSH Forward** | `Space → SSH forward` — opens tunnel form with inline validation |
| **Containers** | Docker/Podman container name column (auto-hides) |
| **Bandwidth** | System-wide RX/TX in header |
| **Export** | `--export json/csv`, `--json` (NDJSON stream) |
| **Watch mode** | `prt watch 80 443` — compact UP/DOWN with BEL alerts |
| **Alerts** | TOML config: bell/highlight on port, process, connection count |
| **Multilingual** | English, Russian, Chinese. Switch with `L` |
| **Config** | `~/.config/prt/config.toml` — port overrides, alert rules |

## Usage

```bash
prt                     # launch TUI
prt --lang ru           # Russian interface
prt --export json       # export snapshot to JSON
prt --json              # NDJSON streaming
prt watch 80 443        # compact port watch
sudo prt                # run as root
```

## Keyboard shortcuts

**Global:** `?` help, `q` quit, `Tab`/`Shift+Tab` next/prev section, `Space` action menu, `/` filter, `r` refresh, `s` sudo, `L` language

**Direct:** `K`/`Del` kill, `c` copy line

**Connections:** `Enter`/`d` toggle Details panel, `o`/`O` sort column / reverse

**Processes / SSH:** `[`/`]` switch sub-tab. SSH/Tunnels: `n` new · `e` edit · `K` kill · `r` restart · `s` save

## Architecture

`prt` is the TUI frontend built on [ratatui](https://ratatui.rs). All core logic lives in [prt-core](https://crates.io/crates/prt-core).

```
crates/
├── prt-core/    # Core library: scanner, tracker, alerts, known ports, i18n, platform
└── prt/         # TUI binary (ratatui + crossterm + clap)
    ├── app.rs       # App state, main loop, caching
    ├── ui.rs        # ViewMode-based rendering
    ├── input.rs     # Key dispatch
    ├── stream.rs    # NDJSON streaming mode
    ├── watch.rs     # Port watch mode
    ├── tracer.rs    # Strace/dtruss session management
    └── forward.rs   # SSH tunnel manager
```

## License

[MIT](https://github.com/rekurt/prt/blob/master/LICENSE)
