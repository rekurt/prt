# Contributing to prt

Thank you for your interest in contributing to **prt**!

## Getting Started

```bash
git clone https://github.com/rekurt/prt.git
cd prt
cargo build --workspace
cargo test --workspace
```

## Development

### Project Structure

```
crates/
  prt-core/    # Core library: model, scanner, session, alerts, suspicious, bandwidth,
               #   containers, namespaces, process_detail, firewall, known_ports,
               #   config, killer, i18n, platform
  prt/         # TUI binary: app, ui (ViewMode routing), input, stream, watch, tracer, forward
```

### Build & Test

```bash
cargo build --workspace          # build everything
cargo test --workspace           # run all tests (188 tests)
cargo clippy --workspace         # lint
cargo fmt --all -- --check       # format check
cargo bench -p prt-core          # criterion benchmarks
```

### Code Style

- Follow `rustfmt.toml` settings (run `cargo fmt --all`)
- No warnings allowed (`RUSTFLAGS="-Dwarnings"`)
- Keep error messages in English in core; UI strings go through `i18n`
- Tests are inline `#[cfg(test)] mod tests` in each module
- New enrichment fields in `TrackedEntry` should be `Option` or `Vec` (zero-cost when unused)
- Platform-specific code uses `#[cfg(target_os)]` guards

### Adding a Language

1. Create `crates/prt-core/src/i18n/<lang>.rs`
2. Implement `static STRINGS: Strings = Strings { ... }`
3. Add variant to `Lang` enum in `i18n/mod.rs`
4. Update `detect_locale()` and `parse_lang()` matchers
5. Compile — any missing `Strings` fields will be caught at compile time

### Adding a View Mode

1. Add variant to `ViewMode` enum in `model.rs`
2. Add i18n label to `Strings` (all 3 language files)
3. Add `draw_*_fullscreen()` function in `ui.rs`
4. Add key binding in `input.rs` (keys 4-7 toggle fullscreen views)
5. Add routing in `draw()` match on `app.view_mode`

### Adding a Detail Tab

1. Add variant to `DetailTab` enum and `ALL` const in `model.rs`
2. Add i18n label to `Strings`
3. Add `draw_tab_*()` function in `ui.rs`
4. Update `draw_detail_panel()` match and `tab_label()`

## Pull Requests

- Keep PRs focused on a single change
- Include tests for new functionality
- Ensure `cargo test --workspace` and `cargo clippy --workspace` pass
- Write clear commit messages

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
