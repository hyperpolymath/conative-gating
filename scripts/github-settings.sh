#!/bin/bash
# Configure GitHub repository settings consistently across all repos
# Usage: ./github-settings.sh <repo_name> [description] [topics]

set -e

OWNER="hyperpolymath"
REPO="$1"
DESCRIPTION="${2:-}"
TOPICS="${3:-}"

if [ -z "$REPO" ]; then
  echo "Usage: $0 <repo_name> [description] [topics_comma_separated]"
  exit 1
fi

echo "Configuring GitHub settings for $OWNER/$REPO..."

# Update repository settings
gh api -X PATCH "repos/$OWNER/$REPO" \
  --field has_issues=true \
  --field has_projects=false \
  --field has_wiki=false \
  --field has_downloads=true \
  --field allow_squash_merge=true \
  --field allow_merge_commit=false \
  --field allow_rebase_merge=true \
  --field delete_branch_on_merge=true \
  --field allow_auto_merge=true \
  --field allow_update_branch=true \
  --field squash_merge_commit_title="PR_TITLE" \
  --field squash_merge_commit_message="PR_BODY" \
  ${DESCRIPTION:+--field description="$DESCRIPTION"} \
  2>/dev/null && echo "  ✓ Repository settings updated"

# Set topics if provided
if [ -n "$TOPICS" ]; then
  # Convert comma-separated to JSON array
  TOPICS_JSON=$(echo "$TOPICS" | tr ',' '\n' | jq -R . | jq -s .)
  gh api -X PUT "repos/$OWNER/$REPO/topics" \
    --input - <<< "{\"names\": $TOPICS_JSON}" \
    2>/dev/null && echo "  ✓ Topics set: $TOPICS"
fi

# Enable vulnerability alerts
gh api -X PUT "repos/$OWNER/$REPO/vulnerability-alerts" \
  2>/dev/null && echo "  ✓ Vulnerability alerts enabled"

# Enable automated security fixes
gh api -X PUT "repos/$OWNER/$REPO/automated-security-fixes" \
  2>/dev/null && echo "  ✓ Automated security fixes enabled"

# Set up branch protection for main
gh api -X PUT "repos/$OWNER/$REPO/branches/main/protection" \
  --input - << 'EOF' 2>/dev/null && echo "  ✓ Branch protection configured"
{
  "required_status_checks": {
    "strict": true,
    "contexts": ["lint", "test"]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "dismiss_stale_reviews": true,
    "require_code_owner_reviews": false,
    "required_approving_review_count": 1
  },
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false
}
EOF

echo "Done configuring $REPO"
