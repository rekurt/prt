# prt

[![Crates.io](https://img.shields.io/crates/v/prt.svg)](https://crates.io/crates/prt)
[![CI](https://github.com/rekurt/prt/actions/workflows/ci.yml/badge.svg)](https://github.com/rekurt/prt/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/rekurt/prt/blob/master/LICENSE)

**Real-time network port monitor for your terminal.**

An interactive alternative to `lsof -i` / `ss -tlnp` with colors, filtering, and process trees.

<img src="https://raw.githubusercontent.com/rekurt/prt/master/docs/prt.gif" alt="prt demo" width="720">

## Install

```bash
cargo install prt
```

**Requirements:** Rust 1.75+ · macOS 10.15+ or Linux with `/proc` · `lsof` (macOS — preinstalled)

## Features

| Feature | Description |
|---------|-------------|
| **Live table** | Ports, protocols, states, PIDs, processes, users. Auto-refreshes every 2s |
| **Change tracking** | New connections flash green; closed ones fade red for 5s |
| **Process tree** | Full parent chain (e.g. `launchd → nginx → worker`) |
| **Detail tabs** | Tree / Network / Connection — toggle with `1` `2` `3` |
| **Search & filter** | By port, process name, PID, protocol, state, user |
| **Sort** | By any column, ascending or descending |
| **Kill** | Select → `K` → confirm with `y` (SIGTERM) or `f` (SIGKILL) |
| **Sudo elevation** | Press `s` — see all system processes |
| **Clipboard** | Copy full line (`c`) or just PID (`p`) |
| **Export** | `prt --export json` or `prt --export csv` |
| **Multilingual** | English, Russian, Chinese. Auto-detects locale, switch with `L` |

## Usage

```bash
prt                     # launch TUI
prt --lang ru           # Russian interface
prt --lang zh           # Chinese interface
prt --export json       # export snapshot to JSON
prt --export csv        # export snapshot to CSV
sudo prt                # run as root (see all processes)
```

## Keyboard shortcuts

| Key | Action | | Key | Action |
|-----|--------|-|-----|--------|
| `q` | Quit | | `K` / `Del` | Kill process |
| `?` | Help | | `c` | Copy line |
| `/` | Search | | `p` | Copy PID |
| `Esc` | Clear filter | | `Tab` | Next sort column |
| `r` | Refresh | | `Shift+Tab` | Reverse sort |
| `s` | Sudo prompt | | `L` | Cycle language |
| `j`/`k` `↑`/`↓` | Navigate | | `1` `2` `3` | Detail tabs |
| `g` / `G` | Top / bottom | | `Enter` / `d` | Toggle details |

## Architecture

`prt` is the TUI frontend built on [ratatui](https://ratatui.rs). All core logic lives in [prt-core](https://crates.io/crates/prt-core).

```
crates/
├── prt-core/    # Core library: scanner, tracker, killer, i18n, platform
└── prt/         # TUI binary (ratatui + crossterm + clap)
```

## License

[MIT](https://github.com/rekurt/prt/blob/master/LICENSE)
