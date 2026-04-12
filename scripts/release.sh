#!/usr/bin/env bash
#
# Release script: generate changelog via kimi, update CHANGELOG.md, tag, push.
#
# Usage:
#   make release VERSION=0.2.0
#   # or directly:
#   ./scripts/release.sh 0.2.0
#
set -euo pipefail

VERSION="${1:?Usage: $0 <version>}"
TAG="v${VERSION}"
DATE="$(date +%Y-%m-%d)"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SKILL_DIR="$PROJECT_DIR/skills/changelog"
CHANGELOG="$PROJECT_DIR/CHANGELOG.md"

info()  { printf '\033[1;34m==>\033[0m \033[1m%s\033[0m\n' "$*"; }
error() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

# ── Checks ──

cd "$PROJECT_DIR"

# Ensure working tree is clean
if [ -n "$(git status --porcelain)" ]; then
    error "Working tree is dirty. Commit or stash changes first."
fi

# Ensure tag doesn't already exist
if git rev-parse "$TAG" &>/dev/null; then
    error "Tag $TAG already exists."
fi

# Ensure kimi is available
if ! command -v kimi &>/dev/null; then
    error "kimi CLI not found in PATH. Required for changelog generation."
fi

# ── Collect git log ──

LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")

if [ -n "$LAST_TAG" ]; then
    info "Generating changelog: $LAST_TAG → $TAG"
    GIT_LOG=$(git log "${LAST_TAG}..HEAD" --pretty=format:"%H|%an|%ae|%ad|%s" --date=short)
else
    info "Generating changelog: (all commits) → $TAG"
    GIT_LOG=$(git log --pretty=format:"%H|%an|%ae|%ad|%s" --date=short)
fi

if [ -z "$GIT_LOG" ]; then
    error "No commits found since last tag."
fi

# ── Call kimi to generate changelog entry ──

PROMPT="Generate a CHANGELOG entry for version ${VERSION} released on ${DATE}.

Here are the commits since the last release:

${GIT_LOG}"

info "Calling kimi to analyze commits..."
CHANGELOG_ENTRY=$(kimi --print --yolo --skills-dir "$SKILL_DIR" --prompt "$PROMPT" 2>/dev/null)

if [ -z "$CHANGELOG_ENTRY" ]; then
    error "kimi returned empty output."
fi

# ── Update CHANGELOG.md ──

info "Updating CHANGELOG.md..."

if [ -f "$CHANGELOG" ]; then
    # Prepend new entry after the title line
    EXISTING=$(cat "$CHANGELOG")
    cat > "$CHANGELOG" <<EOF
# Changelog

${CHANGELOG_ENTRY}

$(echo "$EXISTING" | tail -n +3)
EOF
else
    cat > "$CHANGELOG" <<EOF
# Changelog

${CHANGELOG_ENTRY}
EOF
fi

# ── Commit, tag, push ──

info "Committing release..."
git add CHANGELOG.md
git commit -m "release: ${TAG}

$(echo "$CHANGELOG_ENTRY" | head -20)"

info "Tagging ${TAG}..."
git tag -a "$TAG" -m "Release ${TAG}"

info "Pushing..."
git push origin main "$TAG"

echo ""
info "Released ${TAG} successfully!"
echo "  CHANGELOG.md updated"
echo "  Tag $TAG pushed — CI will build release binaries"
