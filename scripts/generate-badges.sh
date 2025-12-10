#!/bin/bash
# Generate badges from STATE.scm and project metadata
# Usage: ./scripts/generate-badges.sh [minimal|standard|full]

set -euo pipefail

LEVEL="${1:-standard}"
BASE_URL="https://img.shields.io/badge"

# Detect RSR Tier from language
detect_tier() {
    if [ -f "Cargo.toml" ]; then echo "1"; return; fi
    if [ -f "mix.exs" ]; then echo "1"; return; fi
    if [ -f "rescript.json" ] || [ -f "bsconfig.json" ]; then echo "1"; return; fi
    if [ -f "*.zig" ] 2>/dev/null; then echo "1"; return; fi
    if [ -f "*.adb" ] 2>/dev/null; then echo "1"; return; fi
    if [ -f "*.hs" ] 2>/dev/null; then echo "1"; return; fi
    if ls *.ncl >/dev/null 2>&1 || [ -d "config" ] && ls config/*.ncl >/dev/null 2>&1; then echo "2"; return; fi
    if [ -f "flake.nix" ] || [ -f "guix.scm" ]; then echo "2"; return; fi
    echo "Infra"
}

# Detect primary language
detect_language() {
    if [ -f "Cargo.toml" ]; then echo "Rust"; return; fi
    if [ -f "mix.exs" ]; then echo "Elixir"; return; fi
    if [ -f "rescript.json" ] || [ -f "bsconfig.json" ]; then echo "ReScript"; return; fi
    if [ -f "package.json" ]; then echo "JavaScript"; return; fi
    if [ -f "pyproject.toml" ]; then echo "Python"; return; fi
    echo "Guile"
}

# Read phase from STATE.scm
read_phase() {
    if [ -f "STATE.scm" ]; then
        grep -oP '\(phase\s+\.\s+"?\K[^")]+' STATE.scm 2>/dev/null | head -1 || echo "development"
    else
        echo "development"
    fi
}

# Read maturity from STATE.scm
read_maturity() {
    if [ -f "STATE.scm" ]; then
        grep -oP '\(maturity\s+\.\s+"?\K[^")]+' STATE.scm 2>/dev/null | head -1 || echo "experimental"
    else
        echo "experimental"
    fi
}

# Color mappings
tier_color() {
    case "$1" in
        1) echo "gold" ;;
        2) echo "silver" ;;
        *) echo "cd7f32" ;;
    esac
}

phase_color() {
    case "$1" in
        design) echo "lightgrey" ;;
        implementation) echo "blue" ;;
        testing) echo "yellow" ;;
        maintenance) echo "brightgreen" ;;
        archived) echo "red" ;;
        *) echo "blue" ;;
    esac
}

maturity_color() {
    case "$1" in
        experimental) echo "orange" ;;
        alpha) echo "yellow" ;;
        beta) echo "blue" ;;
        production) echo "brightgreen" ;;
        lts) echo "green" ;;
        *) echo "blue" ;;
    esac
}

language_logo() {
    case "$1" in
        Rust) echo "?logo=rust" ;;
        Elixir) echo "?logo=elixir" ;;
        ReScript) echo "?logo=rescript" ;;
        JavaScript) echo "?logo=javascript" ;;
        Python) echo "?logo=python" ;;
        *) echo "" ;;
    esac
}

# URL encode
urlencode() {
    echo "$1" | sed 's/ /%20/g; s/+/%2B/g; s/|/%7C/g'
}

# Generate badge markdown
badge() {
    local label="$1"
    local value="$2"
    local color="$3"
    local extra="${4:-}"
    local encoded_label=$(urlencode "$label")
    local encoded_value=$(urlencode "$value")
    echo "image:${BASE_URL}/${encoded_label}-${encoded_value}-${color}${extra}[${label}: ${value}]"
}

# Get project name
PROJECT_NAME=$(basename "$(pwd)")

# Gather metadata
TIER=$(detect_tier)
TIER_COLOR=$(tier_color "$TIER")
LANG=$(detect_language)
LANG_LOGO=$(language_logo "$LANG")
PHASE=$(read_phase)
PHASE_COLOR=$(phase_color "$PHASE")
MATURITY=$(read_maturity)
MAT_COLOR=$(maturity_color "$MATURITY")

# Check infrastructure
HAS_GUIX=$( [ -f "guix.scm" ] || [ -f ".guix-channel" ] && echo "yes" || echo "no" )
HAS_NIX=$( [ -f "flake.nix" ] && echo "yes" || echo "no" )
HAS_CONTAINER=$( [ -f "Containerfile" ] || [ -f "Dockerfile" ] && echo "yes" || echo "no" )
HAS_GITHUB=$( [ -d ".github/workflows" ] && echo "yes" || echo "no" )
HAS_GITLAB=$( [ -f ".gitlab-ci.yml" ] && echo "yes" || echo "no" )

echo "// Auto-generated badges for ${PROJECT_NAME}"
echo "// Generated: $(date -Iseconds)"
echo "// Level: ${LEVEL}"
echo ""

# === MINIMAL ===
echo "// Identity"
badge "RSR" "Tier ${TIER}" "$TIER_COLOR"
badge "Phase" "$PHASE" "$PHASE_COLOR"
badge "Maturity" "$MATURITY" "$MAT_COLOR"
echo ""
echo "// License"
badge "License" "AGPL OR Palimpsest" "blue"
echo ""

if [ "$LEVEL" = "minimal" ]; then
    exit 0
fi

# === STANDARD ===
echo "// Language"
badge "$LANG" "Latest" "orange" "$LANG_LOGO"

# Secondary languages
SECONDARY=""
[ -f "config.ncl" ] || [ -d "config" ] && ls config/*.ncl >/dev/null 2>&1 && SECONDARY="${SECONDARY}Nickel | "
[ -f "STATE.scm" ] && SECONDARY="${SECONDARY}Guile | "
[ -f "flake.nix" ] && SECONDARY="${SECONDARY}Nix | "
SECONDARY="${SECONDARY% | }"
if [ -n "$SECONDARY" ]; then
    badge "Also" "$SECONDARY" "blue"
fi
echo ""

echo "// Infrastructure"
if [ "$HAS_GUIX" = "yes" ]; then
    badge "Guix" "Primary" "purple" "?logo=gnu"
fi
if [ "$HAS_NIX" = "yes" ]; then
    badge "Nix" "Fallback" "5277C3" "?logo=nixos"
fi
if [ "$HAS_CONTAINER" = "yes" ]; then
    badge "Container" "nerdctl + Wolfi" "blue"
fi
echo ""

echo "// CI/CD"
if [ "$HAS_GITHUB" = "yes" ] && [ "$HAS_GITLAB" = "yes" ]; then
    badge "CI" "GitHub + GitLab" "green"
elif [ "$HAS_GITHUB" = "yes" ]; then
    badge "CI" "GitHub Actions" "2088FF" "?logo=github-actions"
elif [ "$HAS_GITLAB" = "yes" ]; then
    badge "CI" "GitLab CI" "FC6D26" "?logo=gitlab"
fi
echo ""

if [ "$LEVEL" = "standard" ]; then
    exit 0
fi

# === FULL ===
echo "// Security & Privacy"
if [ -f ".well-known/security.txt" ]; then
    badge "Security" "RFC 9116" "green"
fi
badge "Privacy" "GDPR Ready" "brightgreen"
badge "SBOM" "Available" "blue"
echo ""

echo "// Ecosystem"
# Would parse ECOSYSTEM.scm for relationships
badge "Part Of" "RSR Framework" "green"
echo ""
