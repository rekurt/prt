# prt-core

[![Crates.io](https://img.shields.io/crates/v/prt-core.svg)](https://crates.io/crates/prt-core)
[![docs.rs](https://docs.rs/prt-core/badge.svg)](https://docs.rs/prt-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/rekurt/prt/blob/master/LICENSE)

Core library for [**prt**](https://crates.io/crates/prt) — a real-time network port monitor for macOS and Linux.

## What it does

`prt-core` provides platform-independent logic for:

- **Scanning** network ports (TCP/UDP) via `lsof` on macOS or `/proc` on Linux
- **Tracking** connection changes over time (New → Unchanged → Gone)
- **Filtering** by port, PID, process name, protocol, state, or user
- **Sorting** by any column, ascending or descending
- **Exporting** to JSON or CSV
- **Killing** processes by PID (SIGTERM / SIGKILL)
- **i18n** — runtime-switchable localization (English, Russian, Chinese)

## Architecture

```text
platform::scan_ports()
    → Session::refresh()
        → scanner::diff_entries()   (New / Unchanged / Gone)
        → scanner::sort_entries()
        → scanner::filter_indices()
    → UI renders
```

| Platform | Method | Performance |
|----------|--------|-------------|
| **macOS** | `lsof -F` structured output | 2 batch `ps` calls per cycle |
| **Linux** | `/proc/net/` via `procfs` crate | Zero subprocess overhead |

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
    println!("{} :{} ({})", entry.entry.process_name, entry.entry.local_addr.port(), entry.status);
}
```

## i18n

```rust
use prt_core::i18n::{set_lang, strings, Lang};

set_lang(Lang::Zh);
let s = strings();
println!("{}", s.app_name); // "PRT"
println!("{}", s.hint_quit); // "退出"
```

Language resolution: `--lang` flag → `PRT_LANG` env → system locale → English.

## Platform support

| OS | Method | Notes |
|----|--------|-------|
| macOS 10.15+ | `lsof -F` + batch `ps` | Pre-installed, no extra deps |
| Linux | `/proc/net/tcp`, `/proc/net/udp` via `procfs` | Requires `/proc` filesystem |

## License

[MIT](https://github.com/rekurt/prt/blob/master/LICENSE)
