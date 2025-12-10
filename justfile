# Conative Gating - Development Tasks
# Run `just --list` to see all available recipes

set shell := ["bash", "-uc"]
set dotenv-load := true

# Default recipe - show help
default:
    @just --list

# ─────────────────────────────────────────────────────────────
# BUILD & TEST
# ─────────────────────────────────────────────────────────────

# Build in debug mode
build:
    cargo build

# Build in release mode
build-release:
    cargo build --release

# Run all tests
test:
    cargo test --workspace

# Run tests with output
test-verbose:
    cargo test --workspace -- --nocapture

# Run specific test
test-one NAME:
    cargo test --workspace {{NAME}}

# ─────────────────────────────────────────────────────────────
# QUALITY & LINTING
# ─────────────────────────────────────────────────────────────

# Format all code
fmt:
    cargo fmt --all

# Check formatting without changes
fmt-check:
    cargo fmt --all -- --check

# Run clippy linter
lint:
    cargo clippy --workspace -- -D warnings

# Run all quality checks
check: fmt-check lint test
    @echo "All checks passed!"

# Fix common issues automatically
fix:
    cargo fix --workspace --allow-dirty
    cargo fmt --all

# ─────────────────────────────────────────────────────────────
# CLI OPERATIONS
# ─────────────────────────────────────────────────────────────

# Run the CLI (debug build)
run *ARGS:
    cargo run -- {{ARGS}}

# Scan current directory
scan PATH=".":
    cargo run -- scan {{PATH}}

# Scan with JSON output
scan-json PATH=".":
    cargo run -- scan {{PATH}} --format json

# Check a file
check-file FILE:
    cargo run -- check --file {{FILE}}

# Show policy
policy:
    cargo run -- policy

# Show policy as JSON
policy-json:
    cargo run -- policy --format json

# Dry run scan
dry-scan PATH=".":
    cargo run -- --dry-run scan {{PATH}}

# ─────────────────────────────────────────────────────────────
# DOCUMENTATION
# ─────────────────────────────────────────────────────────────

# Generate man page
man:
    cargo run -- man > docs/conative.1

# Generate shell completions
completions-bash:
    cargo run -- completions bash > completions/conative.bash

completions-zsh:
    cargo run -- completions zsh > completions/_conative

completions-fish:
    cargo run -- completions fish > completions/conative.fish

# Generate all completions
completions: completions-bash completions-zsh completions-fish
    @echo "Generated completions in completions/"

# Build documentation
docs:
    cargo doc --workspace --no-deps --open

# ─────────────────────────────────────────────────────────────
# RELEASE & PACKAGING
# ─────────────────────────────────────────────────────────────

# Create release build
release: check build-release
    @echo "Release build complete: target/release/conative"

# Package for distribution
package: release
    mkdir -p dist
    cp target/release/conative dist/
    cp README.adoc dist/
    cp LICENSE dist/
    @echo "Package created in dist/"

# Install locally
install: build-release
    cp target/release/conative ~/.local/bin/
    @echo "Installed to ~/.local/bin/conative"

# ─────────────────────────────────────────────────────────────
# DEVELOPMENT UTILITIES
# ─────────────────────────────────────────────────────────────

# Watch for changes and rebuild
watch:
    cargo watch -x build

# Watch and run tests on change
watch-test:
    cargo watch -x test

# Clean build artifacts
clean:
    cargo clean

# Show dependency tree
deps:
    cargo tree

# Check for outdated dependencies
outdated:
    cargo outdated

# Update dependencies
update:
    cargo update

# ─────────────────────────────────────────────────────────────
# NICKEL POLICY
# ─────────────────────────────────────────────────────────────

# Validate Nickel policy
nickel-check:
    nickel typecheck config/policy.ncl

# Export Nickel policy to JSON
nickel-export:
    nickel export config/policy.ncl > config/policy.json

# ─────────────────────────────────────────────────────────────
# CI SIMULATION
# ─────────────────────────────────────────────────────────────

# Run full CI pipeline locally
ci: fmt-check lint test build-release
    @echo "CI pipeline complete!"

# ─────────────────────────────────────────────────────────────
# HELP & INFO
# ─────────────────────────────────────────────────────────────

# Show version
version:
    @cargo run -- --version

# Show CLI help
help:
    @cargo run -- --help

# Show help for a specific command
help-cmd CMD:
    @cargo run -- {{CMD}} --help

# ─────────────────────────────────────────────────────────────
# REVERSIBILITY NOTES
# ─────────────────────────────────────────────────────────────
#
# All recipes in this justfile are non-destructive except:
# - clean: removes target/ (rebuild with `just build`)
# - install: copies to ~/.local/bin (remove manually)
# - fix: modifies source files (use git to revert)
#
# To see what a recipe would do, use `just --dry-run RECIPE`
# ─────────────────────────────────────────────────────────────
