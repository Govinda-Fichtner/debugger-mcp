#!/bin/bash
# Setup branch protection rules for main branch
# Requires: gh CLI authenticated

set -e

REPO="Govinda-Fichtner/debugger-mcp"
BRANCH="main"

echo "Setting up branch protection for $REPO:$BRANCH..."

# Configure branch protection
gh api -X PUT "repos/$REPO/branches/$BRANCH/protection" \
  --input - <<EOF
{
  "required_status_checks": {
    "strict": true,
    "contexts": [
      "Code Quality",
      "Test Suite",
      "Code Coverage",
      "Build (x86_64-unknown-linux-gnu)",
      "Build (aarch64-unknown-linux-gnu)"
    ]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "dismiss_stale_reviews": true,
    "require_code_owner_reviews": false,
    "required_approving_review_count": 1,
    "require_last_push_approval": false
  },
  "restrictions": null,
  "required_linear_history": false,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "block_creations": false,
  "required_conversation_resolution": true,
  "lock_branch": false,
  "allow_fork_syncing": true
}
EOF

echo "✅ Branch protection configured successfully!"
echo ""
echo "Protection rules:"
echo "  - Require PR reviews: 1 approval"
echo "  - Require status checks: CI jobs must pass"
echo "  - No force pushes allowed"
echo "  - Conversations must be resolved"
echo ""
echo "View settings: https://github.com/$REPO/settings/branches"
