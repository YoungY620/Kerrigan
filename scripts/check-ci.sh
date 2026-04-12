#!/usr/bin/env bash
#
# Check CI status for the latest push on GitHub.
# Intended to be called after git push (e.g., from a hook or manually).
#
# Usage: ./scripts/check-ci.sh [--wait]
#   --wait: poll until the workflow completes (default: just show current status)
#
set -euo pipefail

WAIT="${1:-}"
REPO="YoungY620/Kerrigan"
BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "main")

info()  { printf '\033[1;34m==>\033[0m \033[1m%s\033[0m\n' "$*"; }
warn()  { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }

if ! command -v gh &>/dev/null; then
    warn "gh CLI not found, skipping CI check"
    exit 0
fi

info "Checking CI status for $REPO ($BRANCH)..."

if [ "$WAIT" = "--wait" ]; then
    # Wait for the run to complete (blocks up to ~10 min)
    gh run watch --repo "$REPO" --exit-status 2>/dev/null || {
        warn "CI failed or timed out. Check: https://github.com/$REPO/actions"
    }
else
    # Just show recent runs
    gh run list --repo "$REPO" --branch "$BRANCH" --limit 3 2>/dev/null || {
        warn "Could not fetch CI status. Check: https://github.com/$REPO/actions"
    }
fi
