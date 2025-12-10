#!/usr/bin/env bash
# RSR Git Hook Installer
# SPDX-License-Identifier: AGPL-3.0-or-later OR LicenseRef-Palimpsest-0.5
#
# Installs RSR standard git hooks from templates.
# Usage: ./install-hooks.sh [--all | hook-name ...]
#
# Examples:
#   ./install-hooks.sh --all           # Install all hooks
#   ./install-hooks.sh pre-commit      # Install specific hook
#   ./install-hooks.sh pre-commit pre-push  # Install multiple

set -euo pipefail

# Find template directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEMPLATE_DIR="${SCRIPT_DIR}"

# Find git hooks directory
if [ -d ".git/hooks" ]; then
    HOOKS_DIR=".git/hooks"
elif [ -d "hooks" ]; then
    HOOKS_DIR="hooks"
else
    echo "Error: Not in a git repository"
    exit 1
fi

# Available hooks
AVAILABLE_HOOKS=("pre-commit" "post-receive" "pre-push")

install_hook() {
    local hook="$1"
    local template="${TEMPLATE_DIR}/${hook}.template"
    local target="${HOOKS_DIR}/${hook}"

    if [ ! -f "$template" ]; then
        echo "Warning: Template not found: $template"
        return 1
    fi

    # Backup existing hook
    if [ -f "$target" ] && [ ! -f "${target}.backup" ]; then
        cp "$target" "${target}.backup"
        echo "  Backed up existing ${hook} to ${hook}.backup"
    fi

    # Install hook
    cp "$template" "$target"
    chmod +x "$target"
    echo "✓ Installed ${hook}"
}

show_help() {
    echo "RSR Git Hook Installer"
    echo ""
    echo "Usage: $0 [--all | hook-name ...]"
    echo ""
    echo "Options:"
    echo "  --all       Install all available hooks"
    echo "  --list      List available hooks"
    echo "  --help      Show this help"
    echo ""
    echo "Available hooks:"
    for hook in "${AVAILABLE_HOOKS[@]}"; do
        echo "  - ${hook}"
    done
    echo ""
    echo "Examples:"
    echo "  $0 --all                    # Install all hooks"
    echo "  $0 pre-commit               # Install specific hook"
    echo "  $0 pre-commit pre-push      # Install multiple"
}

# Parse arguments
if [ $# -eq 0 ]; then
    show_help
    exit 0
fi

case "$1" in
    --help|-h)
        show_help
        exit 0
        ;;
    --list)
        echo "Available hooks:"
        for hook in "${AVAILABLE_HOOKS[@]}"; do
            echo "  - ${hook}"
        done
        exit 0
        ;;
    --all)
        echo "Installing all RSR git hooks..."
        for hook in "${AVAILABLE_HOOKS[@]}"; do
            install_hook "$hook" || true
        done
        echo ""
        echo "Done! Hooks installed to ${HOOKS_DIR}/"
        ;;
    *)
        echo "Installing specified hooks..."
        for hook in "$@"; do
            if [[ " ${AVAILABLE_HOOKS[*]} " =~ " ${hook} " ]]; then
                install_hook "$hook"
            else
                echo "Warning: Unknown hook: ${hook}"
            fi
        done
        ;;
esac
