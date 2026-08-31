# Contributing to prt

Thank you for helping improve `prt`. Contributions to code, tests, documentation, translations, and accessibility are welcome.

## Before you start

For a defect, include the operating system, `prt --version`, terminal application, expected behavior, and the smallest reliable reproduction. For a feature, explain the user problem before proposing an interface.

Security vulnerabilities must follow [SECURITY.md](SECURITY.md) rather than a public issue.

## Set up the repository

```sh
git clone https://github.com/rekurt/prt.git
cd prt
cargo build --workspace
cargo test --workspace
```

The workspace requires Rust 1.75 or newer. Platform-specific scanner tests run only on their matching operating system.

## Repository map

```text
crates/
├── prt-core/
│   └── src/
│       ├── core/       scanning, sessions, alerts, process details,
│       │               containers, firewall, bandwidth, and SSH config
│       ├── i18n/       English, Russian, and Chinese strings
│       ├── platform/   macOS and Linux scanners
│       ├── config.rs   TOML configuration
│       └── model.rs    shared domain and UI state types
└── prt/
    └── src/
        ├── views/      Connections, Processes, SSH, forms, and menus
        ├── app.rs      application state and refresh loop
        ├── input.rs    global key dispatch
        ├── ui.rs       top-level rendering
        ├── stream.rs   NDJSON mode
        ├── watch.rs    compact port monitor
        ├── tracer.rs   strace/dtruss sessions
        └── forward.rs  SSH tunnel lifecycle
```

The primary data flow is `platform scan → Session::refresh → diff/enrich/retain/sort → alerts/filter → UI`.

## Make a focused change

- Follow `rustfmt.toml` and keep Clippy warning-free.
- Put user-visible UI text in the `i18n` system rather than inline Rust strings.
- Update English, Russian, and Chinese strings together; the `Strings` struct enforces completeness at compile time.
- Preserve platform boundaries with `#[cfg(target_os = "macos")]` and `#[cfg(target_os = "linux")]`.
- Add or update tests for behavior changes. Tests normally live beside the code in `#[cfg(test)]` modules.
- Avoid unrelated refactors in the same pull request.

When adding a top-level section, update `ViewMode`, rendering, input routing, all localizations, help text, and the README keyboard reference. When adding a sub-tab, update the matching `ProcessesTab` or `SshTab` enum and its view-specific handler.

## Run checks

Run the same core checks expected by CI:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --workspace --doc
```

Optional checks:

```sh
cargo bench -p prt-core          # performance-sensitive scanner work
cargo deny check                 # dependency policy and advisories
cargo doc --workspace --no-deps  # public API documentation
```

If a check is platform-specific or unavailable, state that clearly in the pull request.

## Documentation changes

- Verify commands against the current CLI with `cargo run -p prt -- --help`.
- Keep `README.md`, `README.ru.md`, and `README.zh.md` structurally aligned when user-facing behavior changes.
- Use descriptive link text, meaningful image alternative text, and correct heading order.
- Do not rely on color alone when explaining UI state.
- Update [`docs/demo-transcript.md`](docs/demo-transcript.md) when the demo sequence changes.

The demo requires [VHS](https://github.com/charmbracelet/vhs):

```sh
cargo build --release -p prt
vhs docs/demo.tape
ffmpeg -y -ss 1 -i docs/prt.gif -frames:v 1 docs/prt-demo.png
```

Inspect both assets before committing them and avoid recording secrets, private hosts, or identifying data.

## Pull request checklist

- Explain what changed and why.
- List the checks you ran.
- Include screenshots or a recording for visible TUI changes.
- Update documentation and translations affected by the change.
- Link related issues and follow-up work.

By contributing, you agree that your contribution is licensed under the [MIT License](LICENSE).
