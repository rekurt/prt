# prt

[![Crates.io](https://img.shields.io/crates/v/prt.svg)](https://crates.io/crates/prt)
[![CI](https://github.com/rekurt/prt/actions/workflows/ci.yml/badge.svg)](https://github.com/rekurt/prt/actions/workflows/ci.yml)
[![MIT License](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/rekurt/prt/blob/master/LICENSE)

`prt` is a real-time terminal interface for discovering which processes own network ports on macOS and Linux. It adds filtering, lifecycle tracking, process details, network topology, SSH tunnels, alerts, and automation-friendly output to the usual `lsof` or `/proc` snapshot.

<img src="https://raw.githubusercontent.com/rekurt/prt/master/docs/prt.gif" alt="Animated prt demo showing live connections, process details, topology, command palette, and contextual actions" width="960">

[View a static demo frame](https://github.com/rekurt/prt/blob/master/docs/prt-demo.png) · [Read the demo transcript](https://github.com/rekurt/prt/blob/master/docs/demo-transcript.md) · [Open the full documentation](https://github.com/rekurt/prt#readme)

## Install

```sh
cargo install prt
prt
```

Requirements: Rust 1.75+ for Cargo installation; macOS 10.15+ with `lsof`, or Linux with `/proc`.

## Command-line modes

```sh
prt                         # interactive TUI
prt --lang ru               # language: en, ru, or zh
prt --export json           # one JSON snapshot
prt --export csv            # one CSV snapshot
prt --json                  # continuous NDJSON stream
prt watch 80 443            # compact port monitor
sudo prt                    # include processes hidden from this user
```

Use `--export` for a finite snapshot and `--json` for a continuous stream.

## Interface

- `Tab` / `Shift+Tab`: Connections, Processes, and SSH sections
- `?`: complete in-app help
- `:`: searchable command palette
- `/`: search and filter
- `Space`: contextual actions
- `Enter`: open the selected process
- `d`: toggle the Connections details panel
- `[` / `]`: switch Processes or SSH sub-tabs
- `p`: pause or resume automatic refresh
- `L`: switch language

The TUI is keyboard-driven. A static preview and text transcript are provided for readers who prefer not to view an autoplaying animation. JSON, CSV, NDJSON, and watch modes provide non-full-screen alternatives.

## Crate layout

This binary crate contains the clap CLI, ratatui interface, input handling, NDJSON and watch modes, system-call tracing, and SSH tunnel management. Scanning and domain logic live in [`prt-core`](https://crates.io/crates/prt-core).

## License

[MIT](https://github.com/rekurt/prt/blob/master/LICENSE)
