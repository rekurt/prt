# prt — Network Port Monitor
# Usage: make [target]

.PHONY: all build test bench lint fmt check doc clean install run help

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

# ── Clean ──────────────────────────────────────

## Remove build artifacts
clean:
	cargo clean

# ── Help ───────────────────────────────────────

## Show this help
help:
	@echo "prt — Network Port Monitor"
	@echo ""
	@echo "Build:"
	@echo "  make build       Build all crates (debug)"
	@echo "  make release     Build release binary"
	@echo "  make install     Install to ~/.cargo/bin"
	@echo ""
	@echo "Test:"
	@echo "  make test        Run all tests"
	@echo "  make test-verbose  Run tests with output"
	@echo "  make bench       Run criterion benchmarks"
	@echo ""
	@echo "Quality:"
	@echo "  make check       Run fmt + clippy + test"
	@echo "  make lint        Clippy lint"
	@echo "  make fmt         Auto-format code"
	@echo "  make audit       Check deps for vulnerabilities"
	@echo ""
	@echo "Documentation:"
	@echo "  make doc         Generate rustdoc"
	@echo "  make doc-open    Generate and open in browser"
	@echo ""
	@echo "Run:"
	@echo "  make run         Run TUI (release)"
	@echo "  make export-json Export to JSON"
	@echo "  make export-csv  Export to CSV"
