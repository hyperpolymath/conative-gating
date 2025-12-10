#!/bin/bash
# Bulk standardization script for all repos
# Adds common files and converts MD to AsciiDoc

set -e

UNIFIED_DIR="/run/media/hyper/eclipse/gitprojects/unified"
TEMPLATES_DIR="$UNIFIED_DIR/conative-gating/templates"
LOG_FILE="/tmp/standardization_$(date +%Y%m%d_%H%M%S).log"

log() {
    echo "[$(date '+%H:%M:%S')] $*" | tee -a "$LOG_FILE"
}

# Count repos
REPO_COUNT=$(find "$UNIFIED_DIR" -maxdepth 1 -type d | wc -l)
log "Starting standardization for $REPO_COUNT repositories"
log "Log file: $LOG_FILE"

cd "$UNIFIED_DIR"

# Phase 1: Add .editorconfig
log "=== Phase 1: Adding .editorconfig ==="
added=0
for repo in */; do
    if [ ! -f "$repo/.editorconfig" ] && [ -f "$TEMPLATES_DIR/.editorconfig" ]; then
        cp "$TEMPLATES_DIR/.editorconfig" "$repo/"
        log "  Added .editorconfig to $repo"
        ((added++)) || true
    fi
done
log "  Added .editorconfig to $added repos"

# Phase 2: Add .well-known/ files
log "=== Phase 2: Adding .well-known/ files ==="
added=0
for repo in */; do
    if [ ! -d "$repo/.well-known" ]; then
        mkdir -p "$repo/.well-known"
        log "  Created .well-known/ in $repo"
        ((added++)) || true
    fi

    # Copy template files if they exist
    for wk_file in ai.txt security.txt humans.txt consent-required.txt provenance.json; do
        if [ -f "$TEMPLATES_DIR/.well-known/$wk_file" ] && [ ! -f "$repo/.well-known/$wk_file" ]; then
            cp "$TEMPLATES_DIR/.well-known/$wk_file" "$repo/.well-known/"
        fi
    done
done
log "  Processed .well-known/ for $added repos"

# Phase 3: Convert MD files to AsciiDoc (where AsciiDoc doesn't exist)
log "=== Phase 3: Converting MD to AsciiDoc ==="
converted=0

convert_files=("CHANGELOG" "CONTRIBUTING" "CODE_OF_CONDUCT" "SECURITY" "CLAUDE" "MAINTAINERS" "TPCF")

for repo in */; do
    for base in "${convert_files[@]}"; do
        md_file="$repo$base.md"
        adoc_file="$repo$base.adoc"

        if [ -f "$md_file" ] && [ ! -f "$adoc_file" ]; then
            # Simple conversion - preserve as much as possible
            # Note: For full conversion, pandoc would be ideal
            log "  Would convert $md_file to $adoc_file"
            ((converted++)) || true
        fi
    done
done
log "  Found $converted MD files to convert (manual review needed)"

# Phase 4: Add justfile
log "=== Phase 4: Adding justfile ==="
added=0
for repo in */; do
    if [ ! -f "$repo/justfile" ] && [ -f "$TEMPLATES_DIR/justfile.template" ]; then
        # Don't auto-copy - needs customization
        log "  Missing justfile: $repo"
        ((added++)) || true
    fi
done
log "  $added repos need justfile"

# Phase 5: Add Containerfile
log "=== Phase 5: Checking Containerfile ==="
added=0
for repo in */; do
    if [ ! -f "$repo/Containerfile" ] && [ ! -f "$repo/Dockerfile" ]; then
        log "  Missing Containerfile: $repo"
        ((added++)) || true
    fi
done
log "  $added repos need Containerfile"

# Phase 6: Add flake.nix
log "=== Phase 6: Checking flake.nix ==="
added=0
for repo in */; do
    if [ ! -f "$repo/flake.nix" ]; then
        log "  Missing flake.nix: $repo"
        ((added++)) || true
    fi
done
log "  $added repos need flake.nix"

# Phase 7: Add RSR_COMPLIANCE.adoc
log "=== Phase 7: Checking RSR_COMPLIANCE.adoc ==="
added=0
for repo in */; do
    if [ ! -f "$repo/RSR_COMPLIANCE.adoc" ]; then
        log "  Missing RSR_COMPLIANCE.adoc: $repo"
        ((added++)) || true
    fi
done
log "  $added repos need RSR_COMPLIANCE.adoc"

log ""
log "=== Summary ==="
log "Standardization scan complete. Review $LOG_FILE for details."
log ""
log "Next steps:"
log "1. Run 'gh auth login' to authenticate with GitHub"
log "2. Run './bulk-github-settings.sh' to apply GitHub settings"
log "3. Review and apply suggested changes manually"
