# prt — Network Port Monitor
# Usage: make [target]

VERSION := $(shell grep '^version' crates/prt/Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')

.PHONY: all build test bench lint fmt check doc clean install run help \
        publish publish-core publish-prt tag release

# Default target
all: check test

# ── Build ──────────────────────────────────────

## Build all crates (debug)
build:
	cargo build --workspace

## Build release binary
release:
	cargo build --release --workspace

## Install prt binary to ~/.cargo/bin
install:
	cargo install --path crates/prt

# ── Test ───────────────────────────────────────

## Run all tests
test:
	cargo test --workspace

## Run tests with output
test-verbose:
	cargo test --workspace -- --nocapture

## Run tests for a specific module (e.g. make test-mod MOD=scanner)
test-mod:
	cargo test -p prt-core $(MOD)

# ── Benchmarks ─────────────────────────────────

## Run criterion benchmarks
bench:
	cargo bench -p prt-core

# ── Quality ────────────────────────────────────

## Run all checks (fmt + clippy + test)
check: fmt-check lint test

## Clippy lint
lint:
	cargo clippy --workspace --all-targets -- -D warnings

## Format check
fmt-check:
	cargo fmt --all -- --check

## Auto-format code
fmt:
	cargo fmt --all

## Check dependencies for known vulnerabilities
audit:
	cargo deny check

# ── Documentation ──────────────────────────────

## Generate rustdoc documentation
doc:
	cargo doc --workspace --no-deps --document-private-items

## Generate and open docs in browser
doc-open:
	cargo doc --workspace --no-deps --document-private-items --open

# ── Run ────────────────────────────────────────

## Run TUI
run:
	cargo run --release

## Export JSON
export-json:
	cargo run --release -- --export json

## Export CSV
export-csv:
	cargo run --release -- --export csv

# ── Publish ────────────────────────────────────

## Dry-run publish (verify packaging)
publish-dry:
	cargo publish -p prt-core --dry-run
	cargo publish -p prt --dry-run

## Publish prt-core to crates.io
publish-core:
	cargo publish -p prt-core

## Publish prt binary to crates.io (requires prt-core published first)
publish-prt:
	cargo publish -p prt

## Publish all crates to crates.io (core first, then binary)
publish: publish-core
	@echo "Waiting for crates.io to index prt-core..."
	@sleep 30
	$(MAKE) publish-prt

## Create git tag v$(VERSION) and push it
tag:
	@echo "Tagging v$(VERSION)..."
	git tag -a "v$(VERSION)" -m "Release v$(VERSION)"
	git push origin "v$(VERSION)"

## Full release: check → publish → tag → GitHub release
release: check
	$(MAKE) publish
	$(MAKE) tag
	@echo ""
	@echo "Release v$(VERSION) complete!"
	@echo "  - crates.io: https://crates.io/crates/prt/$(VERSION)"
	@echo "  - GitHub release will be created by CI from tag v$(VERSION)"

# ── Clean ──────────────────────────────────────

## Remove build artifacts
clean:
	cargo clean

# ── Help ───────────────────────────────────────

## Show this help
help:
	@echo "prt — Network Port Monitor (v$(VERSION))"
	@echo ""
	@echo "Build:"
	@echo "  make build         Build all crates (debug)"
	@echo "  make release       Build release binary"
	@echo "  make install       Install to ~/.cargo/bin"
	@echo ""
	@echo "Test:"
	@echo "  make test          Run all tests"
	@echo "  make test-verbose  Run tests with output"
	@echo "  make bench         Run criterion benchmarks"
	@echo ""
	@echo "Quality:"
	@echo "  make check         Run fmt + clippy + test"
	@echo "  make lint          Clippy lint"
	@echo "  make fmt           Auto-format code"
	@echo "  make audit         Check deps for vulnerabilities"
	@echo ""
	@echo "Documentation:"
	@echo "  make doc           Generate rustdoc"
	@echo "  make doc-open      Generate and open in browser"
	@echo ""
	@echo "Run:"
	@echo "  make run           Run TUI (release)"
	@echo "  make export-json   Export to JSON"
	@echo "  make export-csv    Export to CSV"
	@echo ""
	@echo "Publish:"
	@echo "  make publish-dry   Dry-run publish (verify packaging)"
	@echo "  make publish       Publish all crates to crates.io"
	@echo "  make tag           Create and push git tag v$(VERSION)"
	@echo "  make release       Full release: check + publish + tag"
