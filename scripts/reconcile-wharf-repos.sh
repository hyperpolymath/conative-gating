#!/bin/bash
# reconcile-wharf-repos.sh - Consolidate wharf repositories
# SPDX-License-Identifier: PMPL-1.0-or-later OR LicenseRef-Palimpsest-0.5
#
# This script reconciles the wharf repository situation:
# - wharf (GitHub) = Rust infrastructure (KEEP SEPARATE)
# - sinople-wharf (GitLab) = WordPress MVP (DEPRECATE)
# - wordpress-wharf (GitLab) = WordPress MVP (CANONICAL)

set -euo pipefail

UNIFIED_DIR="/run/media/hyper/eclipse/gitprojects/unified"
SINOPLE_WHARF="$UNIFIED_DIR/sinople-wharf"
WORDPRESS_WHARF="$UNIFIED_DIR/wordpress-wharf"
WHARF_INFRA="$UNIFIED_DIR/wharf"

echo "=== Wharf Repository Reconciliation ==="
echo ""

# Step 1: Verify all repos exist
echo "[1/5] Verifying repositories..."
for repo in "$SINOPLE_WHARF" "$WORDPRESS_WHARF" "$WHARF_INFRA"; do
    if [ -d "$repo/.git" ]; then
        echo "  ✓ $(basename $repo) exists"
    else
        echo "  ✗ $(basename $repo) NOT FOUND"
        exit 1
    fi
done

# Step 2: Check for unique files in sinople-wharf
echo ""
echo "[2/5] Checking for unique files in sinople-wharf..."
UNIQUE_FILES=$(diff -rq "$SINOPLE_WHARF" "$WORDPRESS_WHARF" --exclude=.git 2>/dev/null | grep "Only in $SINOPLE_WHARF" || true)
if [ -n "$UNIQUE_FILES" ]; then
    echo "  Found unique files to migrate:"
    echo "$UNIQUE_FILES" | sed 's/^/    /'
    echo ""
    echo "  Copying unique files to wordpress-wharf..."
    while IFS= read -r line; do
        src_dir=$(echo "$line" | sed 's/Only in //' | cut -d: -f1)
        file=$(echo "$line" | cut -d: -f2 | xargs)
        dest_dir="${src_dir/$SINOPLE_WHARF/$WORDPRESS_WHARF}"
        mkdir -p "$dest_dir"
        cp -r "$src_dir/$file" "$dest_dir/"
        echo "    Copied: $file"
    done <<< "$UNIQUE_FILES"
else
    echo "  ✓ No unique files (repos have identical structure)"
fi

# Step 3: Create deprecation notice for sinople-wharf
echo ""
echo "[3/5] Creating deprecation notice for sinople-wharf..."
cat > "$SINOPLE_WHARF/DEPRECATED.adoc" << 'EOF'
= DEPRECATED: sinople-wharf
:important-caption: ⚠️

[IMPORTANT]
====
**This repository has been deprecated and archived.**

Please use the canonical repository:

* **GitLab**: https://gitlab.com/maa-framework/3-applications/wordpress-wharf
* **Local**: `wordpress-wharf/`

This repository (sinople-wharf) was consolidated into wordpress-wharf as part of
RSR repository standardization. All future development occurs in wordpress-wharf.
====

== Migration

No migration required. Both repositories had identical content.

== History

* 2025-01: Consolidated into wordpress-wharf
* 2024: Original sinople MVP scaffold

== See Also

* link:https://gitlab.com/maa-framework/3-applications/wordpress-wharf[wordpress-wharf (canonical)]
* link:https://github.com/hyperpolymath/wharf[wharf (Rust infrastructure - different project)]
EOF

echo "  ✓ Created DEPRECATED.adoc"

# Step 4: Update sinople-wharf README with deprecation header
echo ""
echo "[4/5] Updating sinople-wharf README with deprecation notice..."
if [ -f "$SINOPLE_WHARF/README.adoc" ]; then
    # Check if deprecation notice already exists
    if ! grep -q "DEPRECATED" "$SINOPLE_WHARF/README.adoc"; then
        TEMP_FILE=$(mktemp)
        cat > "$TEMP_FILE" << 'HEADER'
[IMPORTANT]
====
**⚠️ DEPRECATED**: This repository has been merged into
https://gitlab.com/maa-framework/3-applications/wordpress-wharf[wordpress-wharf].
See link:DEPRECATED.adoc[DEPRECATED.adoc] for details.
====

'''

HEADER
        cat "$SINOPLE_WHARF/README.adoc" >> "$TEMP_FILE"
        mv "$TEMP_FILE" "$SINOPLE_WHARF/README.adoc"
        echo "  ✓ Added deprecation header to README.adoc"
    else
        echo "  ✓ README.adoc already has deprecation notice"
    fi
fi

# Step 5: Summary
echo ""
echo "[5/5] Summary"
echo ""
echo "Repository Status:"
echo "  ┌─────────────────┬─────────────────────────────┬─────────────┐"
echo "  │ Repository      │ Purpose                     │ Status      │"
echo "  ├─────────────────┼─────────────────────────────┼─────────────┤"
echo "  │ wharf           │ Rust infrastructure         │ ✓ Keep      │"
echo "  │ wordpress-wharf │ WordPress MVP (canonical)   │ ✓ Primary   │"
echo "  │ sinople-wharf   │ WordPress MVP               │ ⚠ Deprecated│"
echo "  └─────────────────┴─────────────────────────────┴─────────────┘"
echo ""
echo "Next steps:"
echo "  1. Review changes in sinople-wharf"
echo "  2. Commit deprecation notice: cd sinople-wharf && git add -A && git commit -m 'Deprecate in favor of wordpress-wharf'"
echo "  3. Archive sinople-wharf on GitLab (Settings → General → Archive)"
echo "  4. Update any ecosystem references"
echo ""
echo "✓ Reconciliation complete"
