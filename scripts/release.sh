#!/usr/bin/env bash
#
# Release script: auto-determine next version from commit types,
# generate changelog via kimi, update CHANGELOG.md, tag, push.
#
# Usage:
#   make release
#   # or directly:
#   ./scripts/release.sh
#
# Override auto-detected version:
#   make release VERSION=1.0.0
#   ./scripts/release.sh 1.0.0
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SKILL_DIR="$PROJECT_DIR/skills/changelog"
CHANGELOG="$PROJECT_DIR/CHANGELOG.md"

info()  { printf '\033[1;34m==>\033[0m \033[1m%s\033[0m\n' "$*"; }
warn()  { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }
error() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

cd "$PROJECT_DIR"

# ── Checks ──

if [ -n "$(git status --porcelain)" ]; then
    error "Working tree is dirty. Commit or stash changes first."
fi

if ! command -v kimi &>/dev/null; then
    error "kimi CLI not found in PATH. Required for changelog generation."
fi

# ── Determine last version ──

LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")

if [ -n "$LAST_TAG" ]; then
    # Parse last version: strip leading 'v'
    LAST_VERSION="${LAST_TAG#v}"
    IFS='.' read -r MAJOR MINOR PATCH <<< "$LAST_VERSION"
else
    MAJOR=0
    MINOR=0
    PATCH=0
fi

# ── Collect commits since last tag ──

if [ -n "$LAST_TAG" ]; then
    COMMITS=$(git log "${LAST_TAG}..HEAD" --pretty=format:"%s" 2>/dev/null)
    GIT_LOG=$(git log "${LAST_TAG}..HEAD" --pretty=format:"%H|%an|%ae|%ad|%s" --date=short)
else
    COMMITS=$(git log --pretty=format:"%s" 2>/dev/null)
    GIT_LOG=$(git log --pretty=format:"%H|%an|%ae|%ad|%s" --date=short)
fi

if [ -z "$COMMITS" ]; then
    error "No commits found since last tag."
fi

# ── Auto-determine version bump ──

# Check if there are any releasable commits (feat, fix, release, BREAKING CHANGE)
HAS_RELEASABLE=$(echo "$COMMITS" | grep -cE "^(feat|fix|release|BREAKING CHANGE):" || true)

if [ "$HAS_RELEASABLE" -eq 0 ] && [ -z "${1:-}" ]; then
    info "No releasable commits since $LAST_TAG (only docs/chore/ci/refactor/test). Skipping release."
    exit 0
fi

BUMP="patch"  # default

# Check for BREAKING CHANGE (major bump)
if echo "$COMMITS" | grep -qi "BREAKING CHANGE"; then
    BUMP="major"
# Check for release: prefix (minor bump)
elif echo "$COMMITS" | grep -q "^release:"; then
    BUMP="minor"
fi

# Apply bump
case "$BUMP" in
    major)
        MAJOR=$((MAJOR + 1))
        MINOR=0
        PATCH=0
        ;;
    minor)
        MINOR=$((MINOR + 1))
        PATCH=0
        ;;
    patch)
        PATCH=$((PATCH + 1))
        ;;
esac

AUTO_VERSION="${MAJOR}.${MINOR}.${PATCH}"

# Allow manual override
VERSION="${1:-$AUTO_VERSION}"
TAG="v${VERSION}"
DATE="$(date +%Y-%m-%d)"

# Verify tag doesn't exist
if git rev-parse "$TAG" &>/dev/null; then
    error "Tag $TAG already exists."
fi

info "Version bump: $BUMP"
info "Last version: ${LAST_TAG:-none} → Next: $TAG"

# Show commits that determined the bump
if [ "$BUMP" = "major" ]; then
    info "BREAKING CHANGE detected in commits"
elif [ "$BUMP" = "minor" ]; then
    info "release: commit detected"
fi

# ── Call kimi to generate changelog entry ──

PROMPT="Generate a CHANGELOG entry for version ${VERSION} released on ${DATE}.

Here are the commits since the last release (${LAST_TAG:-initial}):

${GIT_LOG}"

info "Calling kimi to analyze commits..."
CHANGELOG_ENTRY=$(kimi --print --yolo --skills-dir "$SKILL_DIR" --prompt "$PROMPT" 2>/dev/null)

if [ -z "$CHANGELOG_ENTRY" ]; then
    error "kimi returned empty output."
fi

# ── Update CHANGELOG.md ──

info "Updating CHANGELOG.md..."

if [ -f "$CHANGELOG" ]; then
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
echo "  Version bump: $BUMP ($LAST_TAG → $TAG)"
echo "  CHANGELOG.md updated"
echo "  Tag $TAG pushed — CI will build release binaries"
