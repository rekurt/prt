# prt-core

[![Crates.io](https://img.shields.io/crates/v/prt-core.svg)](https://crates.io/crates/prt-core)
[![docs.rs](https://docs.rs/prt-core/badge.svg)](https://docs.rs/prt-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/rekurt/prt/blob/master/LICENSE)

Core library for [**prt**](https://crates.io/crates/prt) — a real-time network port monitor for macOS and Linux.

## What it does

`prt-core` provides platform-independent logic for:

- **Scanning** network ports (TCP/UDP) via `lsof` on macOS or `/proc` on Linux
- **Tracking** connection changes over time (New → Unchanged → Gone) with `first_seen` aging
- **Enrichment** — known port names (~170 built-in + user overrides), suspicious connection detection, container awareness
- **Filtering** by port, PID, process name, service, protocol, state, user, or `!` (suspicious)
- **Sorting** by any column, ascending or descending
- **Exporting** to JSON or CSV
- **Killing** processes by PID (SIGTERM / SIGKILL)
- **Alerts** — configurable rules with bell/highlight actions
- **Firewall** — generate iptables/pfctl block/unblock commands
- **Bandwidth** — system-wide RX/TX rate tracking
- **Containers** — Docker/Podman container name resolution
- **Namespaces** — Linux network namespace grouping
- **Process detail** — CWD, environment, open files, CPU, RSS
- **i18n** — runtime-switchable localization (English, Russian, Chinese) backed by `AtomicU8`
- **Config** — TOML-based configuration from `~/.config/prt/`

## Architecture

```text
platform::scan_ports()
    → Session::refresh()
        → scanner::diff_entries()   (New / Unchanged / Gone + first_seen carry-forward)
        → enrich: service names, suspicious flags, containers
        → retain: drop Gone entries older than 5s
        → bandwidth.sample(): RX/TX rate delta
        → scanner::sort_entries()
    → (frontend layer)
        → alerts::evaluate()
        → scanner::filter_indices()
        → UI renders
```

| Platform | Method | Performance |
|----------|--------|-------------|
| **macOS** | `lsof -F` structured output | 2 batch `ps` calls per cycle |
| **Linux** | `/proc/net/tcp`, `/proc/net/udp` via `procfs` crate | Zero subprocess overhead |

## Quick start

```rust
use prt_core::core::scanner;
use prt_core::model::ExportFormat;

let entries = scanner::scan().expect("scan failed");
let json = scanner::export(&entries, ExportFormat::Json).unwrap();
println!("{json}");
```

## Session-based scanning

For continuous monitoring with change tracking:

```rust
use prt_core::core::session::Session;

let mut session = Session::new();
session.refresh().expect("refresh failed");

for entry in &session.entries {
    println!(":{} {} (PID {}) {:?}",
        entry.entry.local_port(),
        entry.entry.process.name,
        entry.entry.process.pid,
        entry.status);
}
```

## Modules

| Module | Description |
|--------|-------------|
| `model` | Core types: PortEntry, TrackedEntry, ViewMode, DetailTab, SortState |
| `core::scanner` | Scan, diff, sort, filter, export |
| `core::session` | Refresh cycle state machine with enrichment pipeline |
| `core::alerts` | Alert rule evaluation (port, process, state, connections_gt) |
| `core::suspicious` | Suspicious connection heuristics (3 rules) |
| `core::bandwidth` | System-wide RX/TX rate (Linux: /proc/net/dev, macOS: netstat -ib) |
| `core::container` | Docker/Podman resolution via batched CLI calls |
| `core::namespace` | Linux network namespace grouping |
| `core::process_detail` | CWD, env, open files, CPU %, RSS |
| `core::firewall` | iptables/pfctl block/unblock command generation |
| `core::killer` | SIGTERM / SIGKILL |
| `known_ports` | Port → service name database (~170 entries + config overrides) |
| `config` | TOML config loading (known_ports, alert rules) |
| `i18n` | EN / RU / ZH runtime switching |
| `platform` | macOS (lsof) / Linux (/proc) |

## i18n

```rust
use prt_core::i18n::{set_lang, strings, Lang};

set_lang(Lang::Zh);
let s = strings();
println!("{}", s.hint_quit); // "退出"
```

## Platform support

| OS | Method | Notes |
|----|--------|-------|
| macOS 10.15+ | `lsof -F` + batch `ps` | Pre-installed, no extra deps |
| Linux | `/proc/net/tcp`, `/proc/net/udp` via `procfs` | Requires `/proc` filesystem |

## License

[MIT](https://github.com/rekurt/prt/blob/master/LICENSE)
