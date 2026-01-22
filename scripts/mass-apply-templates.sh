#!/bin/bash
# Mass Apply RSR Templates to All Repositories
# SPDX-License-Identifier: PMPL-1.0-or-later OR LicenseRef-Palimpsest-0.5
#
# Usage:
#   ./mass-apply-templates.sh [--dry-run] [--repos-dir /path/to/repos]
#
# This script applies RSR templates to all repositories in the unified directory.

set -euo pipefail

# === Configuration ===
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEMPLATE_DIR="${SCRIPT_DIR}/../templates"
REPOS_DIR="${REPOS_DIR:-/run/media/hyper/eclipse/gitprojects/unified}"
DRY_RUN=false
VERBOSE=false
AUTHOR="Jonathan D.A. Jewell"
AUTHOR_EMAIL="hyperpolymath@proton.me"
YEAR=$(date +%Y)

# Color schemes by project type
declare -A COLOR_SCHEMES=(
    ["conative-gating"]="conative"
    ["echidna"]="echidna"
    ["fogbinder"]="fogbinder"
    ["indieweb2-bastion"]="bastion"
    ["wordpress-wharf"]="wharf"
    ["wharf"]="wharf"
    ["vext"]="vext"
    ["zotero-nsai"]="zotero"
    ["zoterho-template"]="zotero"
    ["palimpsest-license"]="palimpsest"
    ["kith"]="kith"
    # Default for others: rhodium
)

# === Argument Parsing ===
while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --repos-dir)
            REPOS_DIR="$2"
            shift 2
            ;;
        --help|-h)
            echo "Usage: $0 [--dry-run] [--verbose] [--repos-dir /path/to/repos]"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# === Helper Functions ===

log() {
    echo "[$(date '+%H:%M:%S')] $*"
}

verbose() {
    if $VERBOSE; then
        echo "  → $*"
    fi
}

dry_run_or_exec() {
    if $DRY_RUN; then
        echo "  [DRY-RUN] $*"
    else
        eval "$@"
    fi
}

get_color_scheme() {
    local project="$1"
    echo "${COLOR_SCHEMES[$project]:-rhodium}"
}

get_project_slug() {
    local project="$1"
    echo "$project" | tr '[:upper:]' '[:lower:]' | tr ' ' '-' | tr -cd 'a-z0-9-'
}

# Substitute template variables
substitute_vars() {
    local template="$1"
    local project_name="$2"
    local project_slug="$3"
    local color_scheme="$4"
    local domain="${5:-}"
    local description="${6:-RSR-compliant project}"

    # Get color values based on scheme
    local primary_color secondary_color accent_color bg_color text_color
    case "$color_scheme" in
        rhodium)
            primary_color="#cd7f32"; secondary_color="#2d3436"; accent_color="#00cec9"
            bg_color="#ffffff"; text_color="#2d3436"
            ;;
        conative)
            primary_color="#6c5ce7"; secondary_color="#2d3436"; accent_color="#00b894"
            bg_color="#f8f9fa"; text_color="#2d3436"
            ;;
        zotero)
            primary_color="#cc2936"; secondary_color="#2d3436"; accent_color="#0984e3"
            bg_color="#ffffff"; text_color="#2d3436"
            ;;
        fogbinder)
            primary_color="#9b59b6"; secondary_color="#1a1a2e"; accent_color="#e17055"
            bg_color="#0f0f23"; text_color="#f8f9fa"
            ;;
        echidna)
            primary_color="#2980b9"; secondary_color="#2d3436"; accent_color="#f39c12"
            bg_color="#ffffff"; text_color="#2d3436"
            ;;
        wharf)
            primary_color="#27ae60"; secondary_color="#2c3e50"; accent_color="#e74c3c"
            bg_color="#ecf0f1"; text_color="#2c3e50"
            ;;
        bastion)
            primary_color="#8e44ad"; secondary_color="#2c3e50"; accent_color="#3498db"
            bg_color="#ffffff"; text_color="#2c3e50"
            ;;
        vext)
            primary_color="#1abc9c"; secondary_color="#2d3436"; accent_color="#e74c3c"
            bg_color="#f8f9fa"; text_color="#2d3436"
            ;;
        palimpsest)
            primary_color="#5f27cd"; secondary_color="#222f3e"; accent_color="#ff9f43"
            bg_color="#f5f6fa"; text_color="#222f3e"
            ;;
        kith)
            primary_color="#10ac84"; secondary_color="#222f3e"; accent_color="#ee5253"
            bg_color="#ffffff"; text_color="#222f3e"
            ;;
        *)
            primary_color="#cd7f32"; secondary_color="#2d3436"; accent_color="#00cec9"
            bg_color="#ffffff"; text_color="#2d3436"
            ;;
    esac

    sed -e "s|{{PROJECT_NAME}}|$project_name|g" \
        -e "s|{{PROJECT_SLUG}}|$project_slug|g" \
        -e "s|{{COLOR_SCHEME}}|$color_scheme|g" \
        -e "s|{{PRIMARY_COLOR}}|$primary_color|g" \
        -e "s|{{SECONDARY_COLOR}}|$secondary_color|g" \
        -e "s|{{ACCENT_COLOR}}|$accent_color|g" \
        -e "s|{{BG_COLOR}}|$bg_color|g" \
        -e "s|{{TEXT_COLOR}}|$text_color|g" \
        -e "s|{{TEXT_MUTED_COLOR}}|#636e72|g" \
        -e "s|{{BORDER_COLOR}}|#dfe6e9|g" \
        -e "s|{{DOMAIN}}|${domain:-$project_slug}|g" \
        -e "s|{{DESCRIPTION}}|$description|g" \
        -e "s|{{AUTHOR}}|$AUTHOR|g" \
        -e "s|{{AUTHOR_EMAIL}}|$AUTHOR_EMAIL|g" \
        -e "s|{{YEAR}}|$YEAR|g" \
        -e "s|{{RSR_TIER}}|1|g" \
        -e "s|{{VERSION}}|0.1.0|g" \
        -e "s|{{CONTACT_EMAIL}}|$AUTHOR_EMAIL|g" \
        -e "s|{{TRAINING_POLICY}}|conditional|g" \
        "$template"
}

# === Template Application Functions ===

apply_well_known() {
    local repo_dir="$1"
    local project_name="$2"
    local project_slug="$3"

    verbose "Applying .well-known/ templates"

    dry_run_or_exec "mkdir -p '$repo_dir/.well-known'"

    # AIBDP manifest
    if [[ -f "$TEMPLATE_DIR/aibdp.json.template" ]]; then
        verbose "  Creating aibdp.json"
        if ! $DRY_RUN; then
            substitute_vars "$TEMPLATE_DIR/aibdp.json.template" "$project_name" "$project_slug" "$(get_color_scheme "$project_name")" > "$repo_dir/.well-known/aibdp.json"
        fi
    fi

    # Dublin Core
    if [[ -f "$TEMPLATE_DIR/dc.xml.template" ]]; then
        verbose "  Creating dc.xml"
        if ! $DRY_RUN; then
            substitute_vars "$TEMPLATE_DIR/dc.xml.template" "$project_name" "$project_slug" "$(get_color_scheme "$project_name")" > "$repo_dir/.well-known/dc.xml"
        fi
    fi

    # Security.txt
    if [[ ! -f "$repo_dir/.well-known/security.txt" ]]; then
        verbose "  Creating security.txt"
        if ! $DRY_RUN; then
            cat > "$repo_dir/.well-known/security.txt" << EOF
Contact: mailto:$AUTHOR_EMAIL
Preferred-Languages: en
Canonical: https://github.com/hyperpolymath/$project_slug/.well-known/security.txt
EOF
        fi
    fi

    # AI.txt
    if [[ ! -f "$repo_dir/.well-known/ai.txt" ]]; then
        verbose "  Creating ai.txt"
        if ! $DRY_RUN; then
            cat > "$repo_dir/.well-known/ai.txt" << EOF
# AI Training Preferences for $project_name
# See also: .well-known/aibdp.json

User-agent: *
Allow: /

# Training preferences
Training: conditional
Conditions: Attribution required, Non-commercial, Share-alike
EOF
        fi
    fi
}

apply_state_scm() {
    local repo_dir="$1"
    local project_name="$2"
    local project_slug="$3"
    local color_scheme="$4"

    if [[ -f "$repo_dir/STATE.scm" ]]; then
        verbose "STATE.scm already exists, skipping"
        return
    fi

    verbose "Creating STATE.scm"

    if ! $DRY_RUN; then
        cat > "$repo_dir/STATE.scm" << EOF
;;; STATE.scm - Project State Checkpoint for $project_name
;;; SPDX-License-Identifier: PMPL-1.0-or-later OR LicenseRef-Palimpsest-0.5
;;; Generated: $(date -Iseconds)

(define-module ($project_slug state)
  #:export (project-state))

(define project-state
  '((metadata
     (name . "$project_name")
     (slug . "$project_slug")
     (version . "0.1.0")
     (author . "$AUTHOR")
     (license . "AGPL-3.0-or-later OR LicenseRef-Palimpsest-0.5")
     (created . "$YEAR"))

    (rsr
     (tier . 1)
     (compliance . "bronze")
     (color-scheme . "$color_scheme"))

    (ecosystem
     (part-of . ("RSR Framework" "MAAF"))
     (depends-on . ())
     (integrates-with . ()))

    (status
     (phase . "active")
     (last-checkpoint . "$(date -Iseconds)")
     (next-milestone . "silver-compliance"))))

;;; End of STATE.scm
EOF
    fi
}

apply_citation_cff() {
    local repo_dir="$1"
    local project_name="$2"
    local project_slug="$3"

    if [[ -f "$repo_dir/CITATION.cff" ]]; then
        verbose "CITATION.cff already exists, skipping"
        return
    fi

    verbose "Creating CITATION.cff"

    if ! $DRY_RUN; then
        cat > "$repo_dir/CITATION.cff" << EOF
cff-version: 1.2.0
message: "If you use this software, please cite it as below."
type: software
title: "$project_name"
version: "0.1.0"
date-released: $YEAR-01-01
authors:
  - family-names: "Jewell"
    given-names: "Jonathan D.A."
    email: "$AUTHOR_EMAIL"
    orcid: "https://orcid.org/0000-0000-0000-0000"
license: "AGPL-3.0-or-later"
repository-code: "https://github.com/hyperpolymath/$project_slug"
keywords:
  - RSR
  - "Rhodium Standard"
EOF
    fi
}

apply_codemeta() {
    local repo_dir="$1"
    local project_name="$2"
    local project_slug="$3"
    local description="${4:-RSR-compliant software}"

    if [[ -f "$repo_dir/codemeta.json" ]]; then
        verbose "codemeta.json already exists, skipping"
        return
    fi

    verbose "Creating codemeta.json"

    if ! $DRY_RUN; then
        cat > "$repo_dir/codemeta.json" << EOF
{
  "@context": "https://doi.org/10.5063/schema/codemeta-2.0",
  "@type": "SoftwareSourceCode",
  "name": "$project_name",
  "description": "$description",
  "version": "0.1.0",
  "license": "https://spdx.org/licenses/AGPL-3.0-or-later.html",
  "codeRepository": "https://github.com/hyperpolymath/$project_slug",
  "author": [{
    "@type": "Person",
    "givenName": "Jonathan D.A.",
    "familyName": "Jewell",
    "email": "$AUTHOR_EMAIL"
  }],
  "programmingLanguage": ["Rust", "ReScript", "Guile Scheme"],
  "keywords": ["RSR", "Rhodium Standard"]
}
EOF
    fi
}

apply_justfile_hooks() {
    local repo_dir="$1"

    if [[ -f "$repo_dir/justfile" ]]; then
        # Check if hooks recipes already exist
        if grep -q "hooks-install" "$repo_dir/justfile" 2>/dev/null; then
            verbose "Justfile already has hooks recipes, skipping"
            return
        fi

        verbose "Appending hooks recipes to existing justfile"
        if ! $DRY_RUN && [[ -f "$TEMPLATE_DIR/justfile-hooks.template" ]]; then
            echo "" >> "$repo_dir/justfile"
            cat "$TEMPLATE_DIR/justfile-hooks.template" >> "$repo_dir/justfile"
        fi
    else
        verbose "Creating justfile with hooks"
        if ! $DRY_RUN && [[ -f "$TEMPLATE_DIR/justfile-hooks.template" ]]; then
            cat > "$repo_dir/justfile" << 'EOF'
# RSR Project Justfile
# SPDX-License-Identifier: PMPL-1.0-or-later

default: check

# Run all checks
check: check-state check-aibdp
    @echo "✓ All checks passed"

EOF
            cat "$TEMPLATE_DIR/justfile-hooks.template" >> "$repo_dir/justfile"
        fi
    fi
}

apply_git_hooks() {
    local repo_dir="$1"

    if [[ ! -d "$repo_dir/.git" ]]; then
        verbose "Not a git repo, skipping hooks"
        return
    fi

    verbose "Installing git hooks"
    dry_run_or_exec "mkdir -p '$repo_dir/.git/hooks'"

    # Pre-commit hook
    if [[ -f "$TEMPLATE_DIR/hooks/pre-commit.template" ]] && [[ ! -f "$repo_dir/.git/hooks/pre-commit" ]]; then
        verbose "  Installing pre-commit hook"
        if ! $DRY_RUN; then
            cp "$TEMPLATE_DIR/hooks/pre-commit.template" "$repo_dir/.git/hooks/pre-commit"
            chmod +x "$repo_dir/.git/hooks/pre-commit"
        fi
    fi

    # Post-receive hook
    if [[ -f "$TEMPLATE_DIR/hooks/post-receive.template" ]] && [[ ! -f "$repo_dir/.git/hooks/post-receive" ]]; then
        verbose "  Installing post-receive hook"
        if ! $DRY_RUN; then
            cp "$TEMPLATE_DIR/hooks/post-receive.template" "$repo_dir/.git/hooks/post-receive"
            chmod +x "$repo_dir/.git/hooks/post-receive"
        fi
    fi
}

# === Main Processing ===

process_repo() {
    local repo_dir="$1"
    local project_name
    local project_slug
    local color_scheme

    project_name=$(basename "$repo_dir")
    project_slug=$(get_project_slug "$project_name")
    color_scheme=$(get_color_scheme "$project_name")

    log "Processing: $project_name (scheme: $color_scheme)"

    apply_well_known "$repo_dir" "$project_name" "$project_slug"
    apply_state_scm "$repo_dir" "$project_name" "$project_slug" "$color_scheme"
    apply_citation_cff "$repo_dir" "$project_name" "$project_slug"
    apply_codemeta "$repo_dir" "$project_name" "$project_slug"
    apply_justfile_hooks "$repo_dir"
    apply_git_hooks "$repo_dir"
}

# === Entry Point ===

main() {
    log "RSR Template Mass Apply Script"
    log "==============================="
    log "Repos directory: $REPOS_DIR"
    log "Template directory: $TEMPLATE_DIR"
    log "Dry run: $DRY_RUN"
    echo ""

    if [[ ! -d "$REPOS_DIR" ]]; then
        echo "Error: Repos directory not found: $REPOS_DIR"
        exit 1
    fi

    if [[ ! -d "$TEMPLATE_DIR" ]]; then
        echo "Error: Template directory not found: $TEMPLATE_DIR"
        exit 1
    fi

    local count=0
    local skipped=0

    for repo in "$REPOS_DIR"/*/; do
        if [[ -d "$repo" ]]; then
            # Skip non-project directories
            local name=$(basename "$repo")
            case "$name" in
                .git|node_modules|vendor|dist|build|target|__pycache__)
                    verbose "Skipping: $name (system directory)"
                    ((skipped++))
                    continue
                    ;;
            esac

            process_repo "$repo"
            ((count++))
        fi
    done

    echo ""
    log "==============================="
    log "Processed: $count repositories"
    log "Skipped: $skipped directories"

    if $DRY_RUN; then
        log ""
        log "This was a dry run. No files were modified."
        log "Run without --dry-run to apply changes."
    fi
}

main "$@"
