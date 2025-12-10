#!/bin/bash
# Deploy comprehensive templates to a repository
# Usage: ./deploy-templates.sh <repo_path> <project_name> <language> [tier]

set -e

REPO_PATH="${1:-.}"
PROJECT_NAME="${2:-$(basename "$REPO_PATH")}"
LANGUAGE="${3:-rust}"
TIER="${4:-1}"

TEMPLATE_DIR="$(dirname "$0")/../templates"

echo "Deploying templates to: $REPO_PATH"
echo "  Project: $PROJECT_NAME"
echo "  Language: $LANGUAGE"
echo "  Tier: $TIER"
echo ""

# Helper: substitute placeholders
substitute() {
  local template="$1"
  local output="$2"

  sed -e "s/{{PROJECT_NAME}}/$PROJECT_NAME/g" \
      -e "s/{{PROJECT_NAME_SNAKE}}/$(echo "$PROJECT_NAME" | tr '-' '_')/g" \
      -e "s/{{LANGUAGE}}/$LANGUAGE/g" \
      -e "s/{{TIER}}/$TIER/g" \
      -e "s/{{BINARY_NAME}}/$(echo "$PROJECT_NAME" | tr '-' '_')/g" \
      "$template" > "$output"

  echo "  Created: $output"
}

cd "$REPO_PATH"

# .editorconfig
if [ ! -f ".editorconfig" ]; then
  substitute "$TEMPLATE_DIR/.editorconfig.template" ".editorconfig"
fi

# .gitignore (append if exists, create if not)
if [ ! -f ".gitignore" ]; then
  substitute "$TEMPLATE_DIR/.gitignore.template" ".gitignore"
fi

# CONTRIBUTING.md
if [ ! -f "CONTRIBUTING.md" ]; then
  substitute "$TEMPLATE_DIR/CONTRIBUTING.md.template" "CONTRIBUTING.md"
fi

# .gitlab-ci.yml
if [ ! -f ".gitlab-ci.yml" ]; then
  substitute "$TEMPLATE_DIR/.gitlab-ci.yml.template" ".gitlab-ci.yml"
fi

# .github/workflows/ci.yml
mkdir -p .github/workflows
if [ ! -f ".github/workflows/ci.yml" ]; then
  substitute "$TEMPLATE_DIR/.github/workflows/ci.yml.template" ".github/workflows/ci.yml"
fi

# Justfile (only if missing)
if [ ! -f "justfile" ] && [ ! -f "Justfile" ]; then
  substitute "$TEMPLATE_DIR/justfile.template" "justfile"
fi

# Salt states (only if missing)
if [ ! -d "salt" ]; then
  mkdir -p salt/states salt/minion.d
  substitute "$TEMPLATE_DIR/salt/states/development.sls.template" "salt/states/development.sls"
  substitute "$TEMPLATE_DIR/salt/states/cicd.sls.template" "salt/states/cicd.sls"
  substitute "$TEMPLATE_DIR/salt/minion.d/minion.conf.template" "salt/minion.d/minion.conf"
fi

# Guix channel
if [ ! -f "guix.scm" ]; then
  substitute "$TEMPLATE_DIR/guix-channel.scm.template" "guix.scm"
fi

# Nix flake
if [ ! -f "flake.nix" ]; then
  substitute "$TEMPLATE_DIR/flake.nix.template" "flake.nix"
fi

# Container files
if [ ! -f "Containerfile" ]; then
  if [ "$LANGUAGE" = "rust" ] || [ "$LANGUAGE" = "zig" ]; then
    substitute "$TEMPLATE_DIR/Containerfile.distroless.template" "Containerfile"
  else
    substitute "$TEMPLATE_DIR/Containerfile.wolfi.template" "Containerfile"
  fi
fi

echo ""
echo "Template deployment complete!"
echo "Run 'just check' to validate the setup."
