<div align="center">

# prt

**Real-time network port monitor for your terminal**

<br>

<img src="docs/prt.gif" alt="prt demo" width="720">

<br>
<br>

[![Crates.io](https://img.shields.io/crates/v/prt.svg)](https://crates.io/crates/prt)
[![Downloads](https://img.shields.io/crates/d/prt.svg)](https://crates.io/crates/prt)
[![CI](https://github.com/rekurt/prt/actions/workflows/ci.yml/badge.svg)](https://github.com/rekurt/prt/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![docs.rs](https://docs.rs/prt-core/badge.svg)](https://docs.rs/prt-core)

[English](README.md) | [Русский](README.ru.md) | [中文](README.zh.md)

</div>

---

## What is prt?

`prt` shows which processes occupy network ports on your machine — in real time, right in your terminal. Think of it as a live, interactive `lsof -i` / `ss -tlnp` with colors, filtering, and process trees.

## Features

| Feature | Description |
|---------|-------------|
| **Live table** | Ports, services, protocols, states, PIDs, processes, users. Auto-refreshes every 2s |
| **Change tracking** | New connections flash green; closed connections fade red for 5s |
| **Suspicious detector** | `[!]` flags for non-root on privileged ports, scripts on sensitive ports |
| **Process tree** | Full parent chain (e.g. `launchd → nginx → worker`) |
| **Detail panel** | Tree / Network / Connection tabs for selected process (`1` `2` `3`) |
| **Fullscreen views** | Chart (`4`), Topology (`5`), Process detail (`6`), Namespaces (`7`) |
| **Search & filter** | By port, service, process name, PID, protocol, state, user. `!` = suspicious only |
| **Sort** | By any column, ascending or descending |
| **Kill** | Select a process → `K` → confirm with `y` (SIGTERM) or `f` (SIGKILL) |
| **Firewall block** | `b` → block remote IP via iptables/pfctl with undo command |
| **Strace attach** | `t` → live syscall tracing in a split panel |
| **Sudo elevation** | Press `s`, enter password — see all system processes |
| **Clipboard** | Copy full line (`c`) or just the PID (`p`) |
| **Container awareness** | Shows Docker/Podman container name (auto-hides when not applicable) |
| **Bandwidth** | System-wide RX/TX rate in the header |
| **Export** | `prt --export json`, `prt --export csv`, `prt --json` (NDJSON streaming) |
| **Watch mode** | `prt watch 3000 8080` — compact UP/DOWN monitor |
| **Alert rules** | TOML config with bell/highlight on port, process, or connection count |
| **Multilingual** | English, Russian, Chinese. Auto-detects locale, switch with `L` |
| **Config** | `~/.config/prt/config.toml` — known port overrides, alert rules |

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
prt --export json       # export snapshot to JSON
prt --export csv        # export snapshot to CSV
prt --json              # NDJSON streaming to stdout
prt watch 80 443 5432   # compact port watch mode
sudo prt                # run as root (see all processes)
```

## Keyboard Shortcuts

**Navigation:**

| Key | Action |
|-----|--------|
| `j`/`k` `↑`/`↓` | Move selection / scroll |
| `g` / `G` | Jump to top / bottom |
| `/` | Search & filter (`!` = suspicious only) |
| `Esc` | Back to table / clear filter |
| `q` | Quit |

**Bottom panel (Table mode):**

| Key | Action |
|-----|--------|
| `Enter` / `d` | Toggle detail panel |
| `1` `2` `3` | Tree / Network / Connection tab |
| `←`/`→` `h`/`l` | Switch detail tab |

**Fullscreen views:**

| Key | Action |
|-----|--------|
| `4` | Chart — connections per process |
| `5` | Topology — process → port → remote |
| `6` | Process detail — info, files, env |
| `7` | Namespaces (Linux only) |

**Actions:**

| Key | Action |
|-----|--------|
| `K` / `Del` | Kill process |
| `c` | Copy line to clipboard |
| `p` | Copy PID to clipboard |
| `b` | Block remote IP (firewall) |
| `t` | Attach/detach strace |
| `r` | Refresh |
| `s` | Sudo prompt |
| `Tab` | Next sort column |
| `Shift+Tab` | Reverse sort direction |
| `L` | Cycle language |
| `?` | Help |

## Configuration

Create `~/.config/prt/config.toml`:

```toml
# Override known port names
[known_ports]
3000 = "my-app"
9090 = "prometheus"

# Alert rules
[[alerts]]
port = 22
action = "bell"

[[alerts]]
process = "python"
state = "LISTEN"
action = "highlight"

[[alerts]]
connections_gt = 100
action = "bell"
```

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
│   ├── model.rs               # PortEntry, TrackedEntry, ViewMode, DetailTab, enums
│   ├── config.rs              # TOML config loading (~/.config/prt/)
│   ├── known_ports.rs         # Well-known port → service name database
│   ├── core/
│   │   ├── scanner.rs         # scan → diff → sort → filter → export
│   │   ├── session.rs         # Refresh cycle state machine
│   │   ├── killer.rs          # SIGTERM / SIGKILL
│   │   ├── alerts.rs          # Alert rule evaluation
│   │   ├── suspicious.rs      # Suspicious connection heuristics
│   │   ├── bandwidth.rs       # System-wide RX/TX rate tracking
│   │   ├── container.rs       # Docker/Podman container resolution
│   │   ├── history.rs         # Connection count sparkline history
│   │   ├── namespace.rs       # Network namespace grouping (Linux)
│   │   ├── process_detail.rs  # CWD, env, files, CPU, RSS
│   │   └── firewall.rs        # iptables/pfctl block/unblock
│   ├── i18n/                  # EN / RU / ZH, AtomicU8-backed runtime switching
│   └── platform/
│       ├── macos.rs           # lsof + batch ps (2 calls/cycle)
│       └── linux.rs           # /proc via procfs
└── prt/                       # TUI binary (ratatui + crossterm + clap)
    ├── app.rs                 # App state, main loop, caching
    ├── ui.rs                  # ViewMode-based rendering, fullscreen views
    ├── input.rs               # Key dispatch by view mode
    ├── stream.rs              # NDJSON streaming mode
    ├── watch.rs               # Port watch mode
    ├── tracer.rs              # Strace/dtruss session management
    └── forward.rs             # SSH tunnel manager
```

**Data flow:**

```
platform::scan_ports() → Session::refresh()
    → diff_entries()        New / Unchanged / Gone (with first_seen carry-forward)
    → retain()              remove Gone after 5s
    → enrich()              service names, suspicious flags, containers
    → sort_entries()        by current SortState
    → filter_indices()      user's search query
    → alerts::evaluate()    fire bell/highlight alerts
    → UI renders            ViewMode-based routing
```

| Platform | Method | Performance |
|----------|--------|-------------|
| **macOS** | `lsof -F` structured output | 2 `ps` calls per scan cycle (batch) |
| **Linux** | `/proc/net/` via `procfs` | Zero subprocess overhead |

## Development

```bash
cargo build --workspace          # build everything
cargo test --workspace           # run all tests (188 tests)
cargo clippy --workspace         # lint
cargo fmt --all -- --check       # format check
cargo bench -p prt-core          # criterion benchmarks
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

[MIT](LICENSE)
