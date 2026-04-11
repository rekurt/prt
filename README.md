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

## Why prt?

Traditional tools like `lsof`, `ss`, and `netstat` give you a static snapshot that's already stale by the time you read it. `prt` gives you a **live, auto-refreshing terminal UI** with change tracking, so you can:

- **See connections appear and disappear** in real time (green = new, red = closing)
- **Find port conflicts instantly** — no more `lsof -i :8080` guessing games
- **Detect suspicious activity** — anomalous connections are auto-flagged with `[!]`
- **Block malicious IPs** directly from the TUI with one keypress
- **Debug containerized apps** — see which Docker/Podman container owns each port
- **Trace syscalls** on the fly — attach strace/dtruss without leaving the TUI
- **Monitor bandwidth** — system-wide throughput displayed in the header bar
- **Set up alerts** — get notified when specific ports open or processes exceed connection limits

## prt vs lsof vs ss vs netstat

| Feature | `prt` | `lsof -i` | `ss -tlnp` | `netstat -tlnp` |
|---------|:-----:|:---------:|:----------:|:---------------:|
| Live auto-refresh | **Yes** | No | No | No |
| Change tracking (new/gone) | **Yes** | No | No | No |
| Color-coded output | **Yes** | No | No | No |
| Interactive filtering | **Yes** | No | No | No |
| Process tree | **Yes** | No | No | No |
| Known port names (170+) | **Yes** | Partial | Partial | Partial |
| Suspicious connection detection | **Yes** | No | No | No |
| Container awareness (Docker) | **Yes** | No | No | No |
| Bandwidth monitoring | **Yes** | No | No | No |
| Firewall quick-block | **Yes** | No | No | No |
| Strace/dtruss attach | **Yes** | No | No | No |
| SSH tunnel creation | **Yes** | No | No | No |
| Alert rules (TOML config) | **Yes** | No | No | No |
| Export to JSON/CSV | **Yes** | No | No | No |
| NDJSON streaming for scripts | **Yes** | No | No | No |
| Multilingual (EN/RU/ZH) | **Yes** | No | No | No |
| macOS + Linux | **Yes** | macOS/Linux | Linux | Linux |
| No dependencies (single binary) | **Yes** | System | System | System |

## Features

### Live Table with Change Tracking

The main view displays all active network connections in a sortable, filterable table. Columns include port, service name, protocol, state, PID, process name, and user. New connections flash **green**; closed connections fade **red** for 5 seconds before disappearing. The table auto-refreshes every 2 seconds.

### Known Ports Database

The `Service` column maps well-known port numbers to human-readable names — http (80), ssh (22), postgres (5432), and ~170 more. You can override or extend with custom names in `~/.config/prt/config.toml`:

```toml
[known_ports]
3000 = "my-app"
9090 = "prometheus"
```

### Connection Aging

Every connection tracks its `first_seen` timestamp. ESTABLISHED connections older than 1 hour are highlighted yellow; older than 24 hours — red. CLOSE_WAIT connections are always red as they indicate potential resource leaks.

### Suspicious Connection Detector

Connections are scanned for anomalies and flagged with `[!]`:

- **Non-root on privileged port** — a non-root process listening on port < 1024
- **Script on sensitive port** — Python, Perl, Ruby, or Node.js listening on port 22, 80, or 443
- **Root outgoing to high port** — root process with an established connection to a remote port > 1024

Filter with `/` then type `!` to show only suspicious entries.

### Container Awareness

If Docker or Podman is running, the `Container` column shows which container owns each process. The column auto-hides when no containers are detected to save space. Resolution uses batched `docker ps` + `docker inspect` calls with a 2-second timeout to avoid blocking the TUI.

### Bandwidth Estimation

The header bar shows system-wide network throughput: `▼ 1.2 MB/s ▲ 340 KB/s`. Reads from `/proc/net/dev` on Linux or `netstat -ib` on macOS. Rates are calculated as deltas between refresh cycles.

### Process Tree

Press `Enter` or `d` to open the detail panel, then `1` to see the full parent chain for the selected process (e.g., `launchd → nginx → worker`). Built by traversing PPID relationships.

### Detail Panel Tabs

The bottom panel (toggle with `Enter`/`d`) has three tabs:

| Tab | Key | Content |
|-----|-----|---------|
| **Tree** | `1` | Process parent chain |
| **Network** | `2` | Interface details, IP addresses, MTU |
| **Connection** | `3` | All connections for the selected PID |

### Fullscreen Views

Four dedicated views accessible with keys `4`-`7`:

| View | Key | Description |
|------|-----|-------------|
| **Chart** | `4` | Horizontal bar chart showing connection count per process |
| **Topology** | `5` | ASCII network graph: process → local port → remote host |
| **Process Detail** | `6` | Comprehensive info page: CWD, CPU %, RSS, open files, environment variables, all connections, network interfaces, process tree |
| **Namespaces** | `7` | Network namespace grouping (Linux only). Shows named namespaces from `/run/netns/` or raw inode numbers |

All fullscreen views support scrolling with `j`/`k` and `g`/`G`. Press `Esc` to return to the table.

### Firewall Quick-Block

Press `b` on a connection with a remote address to block that IP. A confirmation dialog shows the exact command that will be executed:

- **Linux:** `iptables -A INPUT -s <IP> -j DROP`
- **macOS:** `pfctl -t prt_blocked -T add <IP>`

The status bar shows the undo command after blocking. Requires sudo privileges.

### Strace / Dtruss Attach

Press `t` to attach a system call tracer to the selected process. The detail panel splits to show a live stream of network-related syscalls:

- **Linux:** `strace -p <PID> -e trace=network -f`
- **macOS:** `dtruss -p <PID>` (requires SIP disabled or root)

Press `t` again to detach. The tracer process is automatically killed on exit.

### SSH Port Forwarding

Press `F` (Shift+F) to create an SSH tunnel for the selected port. A dialog prompts for the remote host:

```
localhost:5432 →
host:port → user@server.io:5432█
```

The tunnel is created via `ssh -N -L <local>:localhost:<remote> <host>`. Active tunnels are shown in the header bar (`⇄ localhost:5432 → server:22`). Tunnels are health-checked each tick and automatically killed on exit via `Drop`.

### Alert Rules

Define rules in `~/.config/prt/config.toml` to get notified when specific conditions are met:

```toml
[[alerts]]
port = 22
action = "bell"        # terminal bell on new SSH connections

[[alerts]]
process = "python"
state = "LISTEN"
action = "highlight"   # highlight row in yellow

[[alerts]]
connections_gt = 100
action = "bell"        # alert when a process exceeds 100 connections
```

Alerts fire only on NEW entries (not every refresh cycle). Available conditions: `port`, `process`, `state`, `connections_gt`. Actions: `bell`, `highlight`.

### NDJSON Streaming

```bash
prt --json | jq '.process.name'
```

Outputs one JSON object per connection per refresh cycle to stdout. Handles SIGPIPE gracefully (no panics when piped to `head`). No TUI initialization — safe for scripts and pipelines.

### Watch Mode

```bash
prt watch 3000 8080 5432
```

Compact non-TUI display showing UP/DOWN status for specific ports. Emits BEL (`\x07`) on state changes. Supports ANSI colors when connected to a TTY, plain text when piped.

```
:3000 ● UP   nginx (1234)  since 42s
:8080 ○ DOWN               since 7m
:5432 ● UP   postgres (567) since 42s
```

### Export

```bash
prt --export json    # JSON snapshot of all connections
prt --export csv     # CSV snapshot
```

### Multilingual Interface

English, Russian, and Chinese. Language is resolved:

1. `--lang en|ru|zh` CLI flag (highest priority)
2. `PRT_LANG` environment variable
3. System locale auto-detection
4. English (fallback)

Press `L` in the TUI to switch language at runtime — no restart needed.

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
| `F` | SSH port forward (tunnel) |
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
    → enrich()              service names, suspicious flags, containers
    → retain()              remove Gone after 5s
    → bandwidth.sample()    RX/TX delta since previous cycle
    → sort_entries()        by current SortState
App::refresh()
    → alerts::evaluate()    fire bell/highlight alerts
    → filter_indices()      user's search query
    → UI renders            ViewMode-based routing
```

| Platform | Method | Performance |
|----------|--------|-------------|
| **macOS** | `lsof -F` structured output | 2 `ps` calls per scan cycle (batch) |
| **Linux** | `/proc/net/` via `procfs` | Zero subprocess overhead |

## High-ROI Use Cases (beyond basic monitoring)

### 1) Pre-deploy network regression guard

Run `prt --json` before and after a deploy and diff the connection profile.  
This catches accidental new egress paths, wrong bind addresses, and hidden side effects.

```bash
prt --json | jq -c '{pid: .process.pid, name: .process.name, local: .local_addr, remote: .remote_addr, state: .state}'
```

### 2) Live incident response loop in terminal

Use suspicious filter + block + trace without leaving the TUI:

1. `/` then `!` to show suspicious only
2. `b` to block remote IP
3. `t` to attach strace/dtruss and inspect behavior

This provides a fast “observe → contain → inspect” workflow.

### 3) Container port exposure audit

In container-heavy hosts, use **Topology** (`5`) and **Namespaces** (`7`) to spot
unexpected exposure (e.g., debug ports, admin APIs, accidental public binds).

### 4) Runtime feature-flag verification

During feature rollout, track whether enabling a flag introduces new outbound
connections or state churn (`SYN_SENT`, `CLOSE_WAIT` spikes, etc.).

### 5) Lightweight host canary for script-heavy services

For Python/Node/Ruby-heavy stacks, suspicious heuristics + alerts can act as a
cheap canary for anomalous listener behavior on sensitive ports.

## Development

```bash
cargo build --workspace          # build everything
cargo test --workspace           # run all tests
cargo clippy --workspace         # lint
cargo fmt --all -- --check       # format check
cargo bench -p prt-core          # criterion benchmarks
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## FAQ

### How do I see all processes? Some ports show as "unknown".

Run with `sudo prt` to see processes owned by other users. Without root, the OS hides PIDs you don't own.

### Does prt work on Windows?

Not yet. `prt` currently supports **macOS** (10.15+) and **Linux** (with `/proc`). Windows support is tracked in the issue tracker.

### How is prt different from `htop` or `btop`?

`htop`/`btop` are general-purpose process monitors. `prt` focuses specifically on **network connections and ports** — showing which process uses which port, tracking connection lifecycle, detecting anomalies, and providing network-specific actions (firewall block, strace, SSH tunnels).

### Can I use prt in scripts and pipelines?

Yes! Use `prt --json` for NDJSON streaming output, `prt --export json|csv` for snapshots, or `prt watch <ports>` for simple UP/DOWN monitoring. All non-TUI modes work cleanly when piped.

### How do I add custom port names?

Edit `~/.config/prt/config.toml`:

```toml
[known_ports]
3000 = "my-frontend"
8080 = "my-api"
9090 = "prometheus"
```

### Is prt safe to use in production?

`prt` is a **read-only diagnostic tool** by default. Destructive actions (kill process, block IP, attach strace) always require explicit confirmation. The TUI never modifies system state without your approval.

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=rekurt/prt&type=Date)](https://star-history.com/#rekurt/prt&Date)

## License

[MIT](LICENSE)

---

<div align="center">

**If `prt` is useful to you, consider giving it a star on GitHub!**

[![GitHub stars](https://img.shields.io/github/stars/rekurt/prt?style=social)](https://github.com/rekurt/prt)

</div>
