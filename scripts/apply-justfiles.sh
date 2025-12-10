#!/bin/bash
# Apply language-specific justfiles to repos
# Usage: ./apply-justfiles.sh [--dry-run]

set -e

UNIFIED_DIR="/run/media/hyper/eclipse/gitprojects/unified"
DRY_RUN=false

[ "$1" = "--dry-run" ] && DRY_RUN=true && echo "=== DRY RUN MODE ==="

log() { echo "[$(date '+%H:%M:%S')] $*"; }

detect_language() {
    local repo="$1"
    if [ -f "$repo/Cargo.toml" ]; then
        echo "rust"
    elif [ -f "$repo/rescript.json" ] || [ -f "$repo/bsconfig.json" ]; then
        echo "rescript"
    elif [ -f "$repo/mix.exs" ]; then
        echo "elixir"
    elif [ -f "$repo/package.json" ]; then
        echo "javascript"
    elif [ -f "$repo/pyproject.toml" ] || [ -f "$repo/setup.py" ]; then
        echo "python"
    elif [ -f "$repo/flake.nix" ]; then
        echo "nix"
    else
        echo "generic"
    fi
}

generate_rust_justfile() {
    local name="$1"
    cat << 'RUSTJUST'
# {{NAME}} - Rust Development Tasks
set shell := ["bash", "-uc"]
set dotenv-load := true

project := "{{NAME}}"

# Show all recipes
default:
    @just --list --unsorted

# Build debug
build:
    cargo build

# Build release
build-release:
    cargo build --release

# Run tests
test:
    cargo test

# Run tests verbose
test-verbose:
    cargo test -- --nocapture

# Format code
fmt:
    cargo fmt

# Check formatting
fmt-check:
    cargo fmt -- --check

# Run clippy lints
lint:
    cargo clippy -- -D warnings

# Check without building
check:
    cargo check

# Clean build artifacts
clean:
    cargo clean

# Run the project
run *ARGS:
    cargo run -- {{ARGS}}

# Generate docs
doc:
    cargo doc --no-deps --open

# Update dependencies
update:
    cargo update

# Audit dependencies
audit:
    cargo audit

# All checks before commit
pre-commit: fmt-check lint test
    @echo "All checks passed!"
RUSTJUST
}

generate_rescript_justfile() {
    local name="$1"
    cat << 'RESCRIPTJUST'
# {{NAME}} - ReScript Development Tasks
set shell := ["bash", "-uc"]
set dotenv-load := true

project := "{{NAME}}"

# Show all recipes
default:
    @just --list --unsorted

# Build
build:
    npx rescript build

# Build clean
build-clean:
    npx rescript build -clean-world

# Watch mode
watch:
    npx rescript build -w

# Format code
fmt:
    npx rescript format -all

# Clean
clean:
    npx rescript clean

# Install dependencies
install:
    npm install

# Run tests
test:
    npm test

# All checks before commit
pre-commit: build test
    @echo "All checks passed!"
RESCRIPTJUST
}

generate_elixir_justfile() {
    local name="$1"
    cat << 'ELIXIRJUST'
# {{NAME}} - Elixir Development Tasks
set shell := ["bash", "-uc"]
set dotenv-load := true

project := "{{NAME}}"

# Show all recipes
default:
    @just --list --unsorted

# Get dependencies
deps:
    mix deps.get

# Compile
build:
    mix compile

# Run tests
test:
    mix test

# Run tests verbose
test-verbose:
    mix test --trace

# Format code
fmt:
    mix format

# Check formatting
fmt-check:
    mix format --check-formatted

# Run credo lints
lint:
    mix credo --strict

# Clean
clean:
    mix clean

# Run the app
run:
    mix run

# Start IEx session
iex:
    iex -S mix

# Generate docs
doc:
    mix docs

# All checks before commit
pre-commit: fmt-check lint test
    @echo "All checks passed!"
ELIXIRJUST
}

generate_javascript_justfile() {
    local name="$1"
    cat << 'JSJUST'
# {{NAME}} - JavaScript/TypeScript Development Tasks
set shell := ["bash", "-uc"]
set dotenv-load := true

project := "{{NAME}}"

# Show all recipes
default:
    @just --list --unsorted

# Install dependencies
install:
    npm install

# Build
build:
    npm run build

# Run tests
test:
    npm test

# Lint
lint:
    npm run lint

# Format (if prettier configured)
fmt:
    npm run format || npx prettier --write .

# Clean
clean:
    rm -rf node_modules dist build .next

# Start dev server
dev:
    npm run dev

# Type check (if TypeScript)
typecheck:
    npm run typecheck || npx tsc --noEmit

# All checks before commit
pre-commit: lint test
    @echo "All checks passed!"
JSJUST
}

generate_nix_justfile() {
    local name="$1"
    cat << 'NIXJUST'
# {{NAME}} - Nix Development Tasks
set shell := ["bash", "-uc"]
set dotenv-load := true

project := "{{NAME}}"

# Show all recipes
default:
    @just --list --unsorted

# Build with nix
build:
    nix build

# Build and show output path
build-show:
    nix build --print-out-paths

# Enter dev shell
develop:
    nix develop

# Check flake
check:
    nix flake check

# Update flake inputs
update:
    nix flake update

# Show flake info
info:
    nix flake info

# Format nix files
fmt:
    nixfmt *.nix || nix fmt

# Run nix linter
lint:
    statix check . || true

# Clean
clean:
    rm -rf result

# Show derivation
show-drv:
    nix derivation show

# All checks before commit
pre-commit: check
    @echo "All checks passed!"
NIXJUST
}

generate_generic_justfile() {
    local name="$1"
    cat << 'GENERICJUST'
# {{NAME}} - Development Tasks
set shell := ["bash", "-uc"]
set dotenv-load := true

project := "{{NAME}}"

# Show all recipes
default:
    @just --list --unsorted

# Build
build:
    @echo "TODO: Add build command"

# Test
test:
    @echo "TODO: Add test command"

# Clean
clean:
    @echo "TODO: Add clean command"

# Format
fmt:
    @echo "TODO: Add format command"

# Lint
lint:
    @echo "TODO: Add lint command"
GENERICJUST
}

cd "$UNIFIED_DIR"
added=0

for repo in */; do
    name="${repo%/}"
    [ "$name" = "conative-gating" ] && continue

    # Skip if justfile exists (file or directory)
    [ -e "$repo/justfile" ] && continue

    lang=$(detect_language "$repo")

    if $DRY_RUN; then
        log "[DRY] Would add justfile to $name ($lang)"
        ((added++)) || true
        continue
    fi

    case "$lang" in
        rust)
            generate_rust_justfile "$name" | sed "s/{{NAME}}/$name/g" > "$repo/justfile"
            ;;
        rescript)
            generate_rescript_justfile "$name" | sed "s/{{NAME}}/$name/g" > "$repo/justfile"
            ;;
        elixir)
            generate_elixir_justfile "$name" | sed "s/{{NAME}}/$name/g" > "$repo/justfile"
            ;;
        javascript)
            generate_javascript_justfile "$name" | sed "s/{{NAME}}/$name/g" > "$repo/justfile"
            ;;
        nix)
            generate_nix_justfile "$name" | sed "s/{{NAME}}/$name/g" > "$repo/justfile"
            ;;
        *)
            generate_generic_justfile "$name" | sed "s/{{NAME}}/$name/g" > "$repo/justfile"
            ;;
    esac

    log "Added justfile to $name ($lang)"
    ((added++)) || true
done

echo ""
echo "=== SUMMARY ==="
echo "Justfiles added: $added repos"
