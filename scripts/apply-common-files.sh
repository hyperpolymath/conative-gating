#!/bin/bash
# Apply common files to all repos
# Usage: ./apply-common-files.sh [--dry-run]

set -e

UNIFIED_DIR="/run/media/hyper/eclipse/gitprojects/unified"
TEMPLATES_DIR="$UNIFIED_DIR/conative-gating/templates"
DRY_RUN=false
DATE=$(date +%Y-%m-%d)

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
        echo "unknown"
    fi
}

get_tier() {
    local lang="$1"
    case "$lang" in
        rust|rescript|elixir|zig|ada|haskell) echo "1" ;;
        nickel|racket|guile|nix) echo "2" ;;
        *) echo "N/A" ;;
    esac
}

cd "$UNIFIED_DIR"

# Counters
ec_added=0
wk_added=0
rsr_added=0

for repo in */; do
    name="${repo%/}"
    [ "$name" = "conative-gating" ] && continue  # Skip self

    lang=$(detect_language "$repo")
    tier=$(get_tier "$lang")

    # === 1. Apply .editorconfig ===
    if [ ! -f "$repo/.editorconfig" ]; then
        if $DRY_RUN; then
            log "[DRY] Would add .editorconfig to $name"
        else
            sed "s/{{PROJECT_NAME}}/$name/g" "$TEMPLATES_DIR/.editorconfig.template" > "$repo/.editorconfig"
            log "Added .editorconfig to $name"
        fi
        ((ec_added++)) || true
    fi

    # === 2. Apply .well-known/ files ===
    if [ ! -d "$repo/.well-known" ] || [ -z "$(ls -A "$repo/.well-known" 2>/dev/null)" ]; then
        if $DRY_RUN; then
            log "[DRY] Would add .well-known/ to $name"
        else
            mkdir -p "$repo/.well-known"

            # ai.txt
            sed -e "s/{{PROJECT_NAME}}/$name/g" \
                -e "s/{{DATE}}/$DATE/g" \
                "$TEMPLATES_DIR/.well-known/ai.txt.template" > "$repo/.well-known/ai.txt" 2>/dev/null || true

            # security.txt
            sed -e "s/{{PROJECT_NAME}}/$name/g" \
                -e "s/{{DATE}}/$DATE/g" \
                "$TEMPLATES_DIR/.well-known/security.txt.template" > "$repo/.well-known/security.txt" 2>/dev/null || true

            # humans.txt
            sed -e "s/{{PROJECT_NAME}}/$name/g" \
                "$TEMPLATES_DIR/.well-known/humans.txt.template" > "$repo/.well-known/humans.txt" 2>/dev/null || true

            # consent-required.txt
            cp "$TEMPLATES_DIR/.well-known/consent-required.txt.template" "$repo/.well-known/consent-required.txt" 2>/dev/null || true

            # provenance.json
            sed -e "s/{{PROJECT_NAME}}/$name/g" \
                -e "s/{{DATE}}/$DATE/g" \
                "$TEMPLATES_DIR/.well-known/provenance.json.template" > "$repo/.well-known/provenance.json" 2>/dev/null || true

            log "Added .well-known/ to $name"
        fi
        ((wk_added++)) || true
    fi

    # === 3. Apply RSR_COMPLIANCE.adoc ===
    if [ ! -f "$repo/RSR_COMPLIANCE.adoc" ]; then
        # Determine compliance status
        status="Partial"
        [ "$tier" = "1" ] && status="Compliant"
        [ "$tier" = "N/A" ] && status="Review Needed"

        # Check for files
        ec_check=$([ -f "$repo/.editorconfig" ] && echo "✓" || echo "✗")
        wk_check=$([ -d "$repo/.well-known" ] && echo "✓" || echo "✗")
        jf_check=$([ -f "$repo/justfile" ] && echo "✓" || echo "✗")
        lic_check=$([ -f "$repo/LICENSE" ] || [ -f "$repo/LICENSE.txt" ] && echo "✓" || echo "✗")
        cont_check=$([ -f "$repo/Containerfile" ] || [ -f "$repo/Dockerfile" ] && echo "✓" || echo "✗")
        flake_check=$([ -f "$repo/flake.nix" ] && echo "✓" || echo "✗")

        if $DRY_RUN; then
            log "[DRY] Would add RSR_COMPLIANCE.adoc to $name ($lang, tier $tier)"
        else
            # Generate RSR_COMPLIANCE.adoc directly
            cat > "$repo/RSR_COMPLIANCE.adoc" << RSREOF
= RSR Compliance: $name
:toc:
:sectnums:

== Overview

This document describes the Rhodium Standard Repository (RSR) compliance status for *$name*.

== Classification

[cols="1,2"]
|===
|Attribute |Value

|Project |$name
|Primary Language |$lang
|RSR Tier |$tier
|Compliance Status |$status
|Last Updated |$DATE
|===

== Language Tier Classification

=== Tier 1 Languages (Preferred)
* Rust
* Elixir
* Zig
* Ada
* Haskell
* ReScript

=== Tier 2 Languages (Acceptable)
* Nickel (configuration)
* Racket (scripting)
* Guile Scheme (state management)
* Nix (derivations)

=== Restricted Languages
* Python - Only allowed in salt/ directories for SaltStack
* TypeScript/JavaScript - Legacy only, convert to ReScript
* CUE - Not permitted, use Nickel or Guile

== Compliance Checklist

[cols="1,1,2"]
|===
|Requirement |Status |Notes

|Primary language is Tier 1/2 |✓ |$lang
|No restricted languages outside exemptions |✓ |
|.editorconfig present |$ec_check |
|.well-known/ directory |$wk_check |
|justfile present |$jf_check |
|LICENSE.txt (AGPL + Palimpsest) |$lic_check |
|Containerfile present |$cont_check |
|flake.nix present |$flake_check |
|===

== Exemptions

RSREOF
            # Add exemptions
            if [ "$lang" = "python" ]; then
                echo "* Python permitted in salt/ directories only" >> "$repo/RSR_COMPLIANCE.adoc"
            elif [ "$lang" = "javascript" ]; then
                echo "* JavaScript/TypeScript: Legacy code, conversion to ReScript planned" >> "$repo/RSR_COMPLIANCE.adoc"
            else
                echo "None" >> "$repo/RSR_COMPLIANCE.adoc"
            fi

            # Add action items section
            cat >> "$repo/RSR_COMPLIANCE.adoc" << 'ACTIONEOF'

== Action Items

ACTIONEOF
            # Add specific action items
            action_count=0
            [ "$ec_check" = "✗" ] && echo "* Add .editorconfig" >> "$repo/RSR_COMPLIANCE.adoc" && ((action_count++)) || true
            [ "$wk_check" = "✗" ] && echo "* Add .well-known/ directory" >> "$repo/RSR_COMPLIANCE.adoc" && ((action_count++)) || true
            [ "$jf_check" = "✗" ] && echo "* Add justfile" >> "$repo/RSR_COMPLIANCE.adoc" && ((action_count++)) || true
            [ "$cont_check" = "✗" ] && echo "* Add Containerfile" >> "$repo/RSR_COMPLIANCE.adoc" && ((action_count++)) || true
            [ "$flake_check" = "✗" ] && echo "* Add flake.nix" >> "$repo/RSR_COMPLIANCE.adoc" && ((action_count++)) || true
            [ "$action_count" -eq 0 ] && echo "None - fully compliant" >> "$repo/RSR_COMPLIANCE.adoc"

            # Add references
            cat >> "$repo/RSR_COMPLIANCE.adoc" << 'REFEOF'

== References

* link:https://github.com/hyperpolymath/RSR-template-repo[RSR Template Repository]
* link:../CONTRIBUTING.adoc[Contributing Guidelines]
* link:../CODE_OF_CONDUCT.adoc[Code of Conduct]
REFEOF
            log "Added RSR_COMPLIANCE.adoc to $name"
        fi
        ((rsr_added++)) || true
    fi
done

echo ""
echo "=== SUMMARY ==="
echo ".editorconfig added: $ec_added repos"
echo ".well-known/ added:  $wk_added repos"
echo "RSR_COMPLIANCE.adoc: $rsr_added repos"
