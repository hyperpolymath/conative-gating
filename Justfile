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

# [AUTO-GENERATED] Multi-arch / RISC-V target
build-riscv:
	@echo "Building for RISC-V..."
	cross build --target riscv64gc-unknown-linux-gnu

# Run panic-attacker pre-commit scan
assail:
    @command -v panic-attack >/dev/null 2>&1 && panic-attack assail . || echo "panic-attack not found — install from https://github.com/hyperpolymath/panic-attacker"

# ═══════════════════════════════════════════════════════════════════════════════
# ONBOARDING & DIAGNOSTICS
# ═══════════════════════════════════════════════════════════════════════════════

# Check all required toolchain dependencies and report health
doctor:
    #!/usr/bin/env bash
    echo "═══════════════════════════════════════════════════"
    echo "  Conative Gating Doctor — Toolchain Health Check"
    echo "═══════════════════════════════════════════════════"
    echo ""
    PASS=0; FAIL=0; WARN=0
    check() {
        local name="$1" cmd="$2" min="$3"
        if command -v "$cmd" >/dev/null 2>&1; then
            VER=$("$cmd" --version 2>&1 | head -1)
            echo "  [OK]   $name — $VER"
            PASS=$((PASS + 1))
        else
            echo "  [FAIL] $name — not found (need $min+)"
            FAIL=$((FAIL + 1))
        fi
    }
    check "just"              just      "1.25" 
    check "git"               git       "2.40" 
    check "Rust (cargo)"      cargo     "1.80" 
    check "Zig"               zig       "0.13" 
# Optional tools
if command -v panic-attack >/dev/null 2>&1; then
    echo "  [OK]   panic-attack — available"
    PASS=$((PASS + 1))
else
    echo "  [WARN] panic-attack — not found (pre-commit scanner)"
    WARN=$((WARN + 1))
fi
    echo ""
    echo "  Result: $PASS passed, $FAIL failed, $WARN warnings"
    if [ "$FAIL" -gt 0 ]; then
        echo "  Run 'just heal' to attempt automatic repair."
        exit 1
    fi
    echo "  All required tools present."

# Attempt to automatically install missing tools
heal:
    #!/usr/bin/env bash
    echo "═══════════════════════════════════════════════════"
    echo "  Conative Gating Heal — Automatic Tool Installation"
    echo "═══════════════════════════════════════════════════"
    echo ""
if ! command -v cargo >/dev/null 2>&1; then
    echo "Installing Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi
if ! command -v just >/dev/null 2>&1; then
    echo "Installing just..."
    cargo install just 2>/dev/null || echo "Install just from https://just.systems"
fi
    echo ""
    echo "Heal complete. Run 'just doctor' to verify."

# Guided tour of the project structure and key concepts
tour:
    #!/usr/bin/env bash
    echo "═══════════════════════════════════════════════════"
    echo "  Conative Gating — Guided Tour"
    echo "═══════════════════════════════════════════════════"
    echo ""
    echo 'Jonathan D.A. Jewell <jonathan@hyperpolymath.org>'
    echo ""
    echo "Key directories:"
    echo "  src/                      Source code" 
    echo "  ffi/                      Foreign function interface (Zig)" 
    echo "  src/abi/                  Idris2 ABI definitions" 
    echo "  docs/                     Documentation" 
    echo "  .github/workflows/        CI/CD workflows" 
    echo "  contractiles/             Must/Trust/Dust contracts" 
    echo "  .machine_readable/        Machine-readable metadata" 
    echo "  examples/                 Usage examples" 
    echo ""
    echo "Quick commands:"
    echo "  just doctor    Check toolchain health"
    echo "  just heal      Fix missing tools"
    echo "  just help-me   Common workflows"
    echo "  just default   List all recipes"
    echo ""
    echo "Read more: README.adoc, EXPLAINME.adoc"

# Show help for common workflows
help-me:
    #!/usr/bin/env bash
    echo "═══════════════════════════════════════════════════"
    echo "  Conative Gating — Common Workflows"
    echo "═══════════════════════════════════════════════════"
    echo ""
echo "FIRST TIME SETUP:"
echo "  just doctor           Check toolchain"
echo "  just heal             Fix missing tools"
echo "" 
    echo "DEVELOPMENT:" 
    echo "  cargo build           Build the project" 
    echo "  cargo test            Run tests" 
    echo "" 
echo "PRE-COMMIT:"
echo "  just assail           Run panic-attacker scan"
echo ""
echo "LEARN:"
echo "  just tour             Guided project tour"
echo "  just default          List all recipes" 
