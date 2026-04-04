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
  prt-core/    # Core library: model, scanner, killer, platform, i18n
  prt/         # TUI binary (ratatui + crossterm)
```

### Build & Test

```bash
cargo build --workspace          # build everything
cargo test --workspace           # run all tests
cargo clippy --workspace         # lint
cargo fmt --all -- --check       # format check
```

### Code Style

- Follow `rustfmt.toml` settings (run `cargo fmt --all`)
- No warnings allowed (`RUSTFLAGS="-Dwarnings"`)
- Keep error messages in English in core; UI strings go through `i18n`
- Tests are inline `#[cfg(test)] mod tests` in each module

### Adding a Language

1. Create `crates/prt-core/src/i18n/<lang>.rs`
2. Implement `static STRINGS: Strings = Strings { ... }`
3. Add variant to `Lang` enum in `i18n/mod.rs`
4. Update `detect_locale()` and `parse_lang()` matchers

## Pull Requests

- Keep PRs focused on a single change
- Include tests for new functionality
- Ensure `cargo test --workspace` and `cargo clippy --workspace` pass
- Write clear commit messages

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
