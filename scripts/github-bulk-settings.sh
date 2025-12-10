#!/bin/bash
# Apply consistent GitHub settings to all repos in a directory
# Usage: ./github-bulk-settings.sh <repos_directory>

set -e

REPOS_DIR="${1:-/run/media/hyper/eclipse/gitprojects/unified}"
SCRIPT_DIR="$(dirname "$0")"
OWNER="hyperpolymath"

# Common topics for all RSR projects
BASE_TOPICS="rhodium-standard,rsr,agpl-3-0,palimpsest-license"

echo "=== Bulk GitHub Settings Configuration ==="
echo "Repos directory: $REPOS_DIR"
echo ""

# Check gh auth
if ! gh auth status &>/dev/null; then
  echo "ERROR: Not authenticated with GitHub. Run: gh auth login"
  exit 1
fi

cd "$REPOS_DIR"

for dir in */; do
  repo="${dir%/}"

  # Skip non-git directories
  if [ ! -d "$repo/.git" ]; then
    continue
  fi

  # Check if repo exists on GitHub
  if ! gh repo view "$OWNER/$repo" &>/dev/null; then
    echo "  SKIP: $repo (not on GitHub)"
    continue
  fi

  echo "Processing: $repo"

  # Detect language and add appropriate topics
  TOPICS="$BASE_TOPICS"

  if [ -f "$repo/Cargo.toml" ]; then
    TOPICS="$TOPICS,rust"
  fi
  if [ -f "$repo/mix.exs" ]; then
    TOPICS="$TOPICS,elixir"
  fi
  if [ -f "$repo/build.zig" ]; then
    TOPICS="$TOPICS,zig"
  fi
  if ls "$repo"/*.adb &>/dev/null 2>&1 || ls "$repo"/*.ads &>/dev/null 2>&1; then
    TOPICS="$TOPICS,ada"
  fi
  if [ -f "$repo/bsconfig.json" ] || [ -f "$repo/rescript.json" ]; then
    TOPICS="$TOPICS,rescript"
  fi
  if [ -f "$repo/config.ncl" ] || ls "$repo"/*.ncl &>/dev/null 2>&1; then
    TOPICS="$TOPICS,nickel"
  fi

  # Get description from README if available
  DESC=""
  if [ -f "$repo/README.adoc" ]; then
    DESC=$(head -10 "$repo/README.adoc" | grep -E "^[^=]" | head -1 | sed 's/\*//g' | xargs)
  fi

  # Apply settings
  "$SCRIPT_DIR/github-settings.sh" "$repo" "$DESC" "$TOPICS" || true

  echo ""
done

echo "=== Bulk configuration complete ==="
