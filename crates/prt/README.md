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
| **Suspicious detector** | `[!]` flags for non-root on privileged ports, scripts on sensitive ports |
| **Process tree** | Full parent chain (e.g. `launchd → nginx → worker`) |
| **Detail panel** | Tree / Network / Connection tabs (`1` `2` `3`) |
| **Fullscreen views** | Chart (`4`), Topology (`5`), Process detail (`6`), Namespaces (`7`) |
| **Search & filter** | By port, service, process, PID, protocol, state, user. `!` = suspicious |
| **Kill** | Select → `K` → `y` (SIGTERM) or `f` (SIGKILL) |
| **Firewall block** | `b` → block remote IP with undo command |
| **Strace** | `t` → live syscall tracing |
| **Containers** | Docker/Podman container name column |
| **Bandwidth** | System-wide RX/TX in header |
| **Export** | `--export json/csv`, `--json` (NDJSON stream) |
| **Watch mode** | `prt watch 80 443` — compact UP/DOWN |
| **Alerts** | TOML config: bell/highlight on port, process, connection count |
| **Multilingual** | English, Russian, Chinese. Switch with `L` |
| **Config** | `~/.config/prt/config.toml` |

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

**Navigation:** `j`/`k` move, `g`/`G` top/bottom, `/` search, `Esc` back/clear, `q` quit

**Panel:** `Enter`/`d` toggle details, `1`-`3` tabs, `←`/`→` switch tabs

**Views:** `4` chart, `5` topology, `6` process detail, `7` namespaces

**Actions:** `K` kill, `c` copy, `p` copy PID, `b` block IP, `t` strace, `Tab` sort, `L` language

## Architecture

`prt` is the TUI frontend built on [ratatui](https://ratatui.rs). All core logic lives in [prt-core](https://crates.io/crates/prt-core).

```
crates/
├── prt-core/    # Core library: scanner, tracker, alerts, known ports, i18n, platform
└── prt/         # TUI binary (ratatui + crossterm + clap)
```

## License

[MIT](https://github.com/rekurt/prt/blob/master/LICENSE)
