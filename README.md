<div align="center">

# prt

**Real-time network port monitor for your terminal**

<br>

<img src="docs/prt.gif" alt="prt demo" width="720">

<br>
<br>

[![CI](https://github.com/rekurt/prt/actions/workflows/ci.yml/badge.svg)](https://github.com/rekurt/prt/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

[English](README.md) | [Русский](README.ru.md) | [中文](README.zh.md)

</div>

---

## What is prt?

`prt` shows which processes occupy network ports on your machine — in real time, right in your terminal. Think of it as a live, interactive `lsof -i` / `ss -tlnp` with colors, filtering, and process trees.

## Features

| Feature | Description |
|---------|-------------|
| **Live table** | Ports, protocols, states, PIDs, processes, users. Auto-refreshes every 2s |
| **Change tracking** | New connections flash green; closed connections fade red for 5s |
| **Process tree** | See the full parent chain (e.g. `launchd → nginx → worker`) |
| **Detail tabs** | Tree / Network / Connection — toggle with `1` `2` `3` |
| **Search & filter** | Fuzzy search by port, process name, PID, protocol, state, user |
| **Sort** | By any column, ascending or descending |
| **Kill** | Select a process → `K` → confirm with `y` (SIGTERM) or `f` (SIGKILL) |
| **Sudo elevation** | Press `s`, enter password — see all system processes |
| **Clipboard** | Copy full line (`c`) or just the PID (`p`) |
| **Export** | `prt --export json` or `prt --export csv` for scripting |
| **Multilingual** | English, Russian, Chinese. Auto-detects locale, switch with `L` in TUI |

## Install

```bash
cargo install prt
```

<details>
<summary><b>Build from source</b></summary>

```bash
git clone https://github.com/rekurt/prt.git
cd prt
make install    # or: cargo install --path crates/prt
```

**Requirements:** Rust 1.75+ · macOS 10.15+ or Linux with `/proc` · `lsof` (macOS — preinstalled)

</details>

## Usage

```bash
prt                     # launch TUI
prt --lang ru           # Russian interface
prt --lang zh           # Chinese interface
prt --export json       # export snapshot to JSON
prt --export csv        # export snapshot to CSV
PRT_LANG=ru prt         # set language via environment
sudo prt                # run as root (see all processes)
```

## Keyboard Shortcuts

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

## Language

Language is resolved in this order:

1. `--lang en|ru|zh` CLI flag (highest priority)
2. `PRT_LANG` environment variable
3. System locale auto-detection
4. English (fallback)

Press `L` in the TUI to switch language at runtime — no restart needed.

## Architecture

```
crates/
├── prt-core/                  # Core library (platform-independent)
│   ├── model.rs               # PortEntry, TrackedEntry, SortState, enums
│   ├── core/
│   │   ├── scanner.rs         # scan → diff → sort → filter → export
│   │   ├── killer.rs          # SIGTERM / SIGKILL
│   │   └── session.rs         # Refresh cycle state machine
│   ├── i18n/                  # EN / RU / ZH, AtomicU8-backed runtime switching
│   └── platform/
│       ├── macos.rs           # lsof + batch ps (2 calls/cycle)
│       └── linux.rs           # /proc via procfs
└── prt/                       # TUI binary (ratatui + crossterm + clap)
```

**Data flow:**

```
platform::scan_ports() → Session::refresh()
    → diff_entries()        New / Unchanged / Gone
    → retain()              remove Gone after 5s
    → sort_entries()        by current SortState
    → filter_indices()      user's search query
    → UI renders
```

| Platform | Method | Performance |
|----------|--------|-------------|
| **macOS** | `lsof -F` structured output | 2 `ps` calls per scan cycle (batch) |
| **Linux** | `/proc/net/` via `procfs` | Zero subprocess overhead |

## Development

```bash
make check          # fmt + clippy + test (79 tests)
make bench          # criterion benchmarks
make doc-open       # generate and open rustdoc
make test-verbose   # tests with stdout
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

[MIT](LICENSE)
