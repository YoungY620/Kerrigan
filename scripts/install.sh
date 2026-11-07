#!/usr/bin/env sh
#
# Kerrigan installer — run with:
#
#   # clone from GitHub (public)
#   curl -fsSL \
#     "https://raw.githubusercontent.com/YoungY620/Kerrigan/main/scripts/install.sh" | sh
#
# What it does:
#   0. Clone (or pull) the kerrigan repo into ~/.kerrigan/.source/
#   1. Copy default-agent config to ~/.kerrigan/agents/kerrigan-agent/
#   2. Copy skills to ~/.kerrigan/skills/
#   3. Build kerrigan CLI (if cargo is available)
#
set -e

# ── Helpers ──────────────────────────────────────────────────────────────────

info()  { printf '\033[1;34m==>\033[0m \033[1m%s\033[0m\n' "$*"; }
ok()    { printf '\033[1;32m ✓\033[0m  %s\n' "$*"; }
warn()  { printf '\033[1;33m ⚠\033[0m  %s\n' "$*" >&2; }
error() { printf '\033[1;31m ✗\033[0m  %s\n' "$*" >&2; exit 1; }

# ── Configuration ────────────────────────────────────────────────────────────

REPO_URL="https://github.com/YoungY620/Kerrigan.git"
SOURCE_DIR="$HOME/.kerrigan/.source"
AGENT_DIR="$HOME/.kerrigan/agents/kerrigan-agent"
SKILLS_DIR="$HOME/.kerrigan/skills"

# ── Pre-flight checks ───────────────────────────────────────────────────────

command -v git >/dev/null 2>&1 || error "git is required but not found in PATH"

# ── Step 0: Clone or update source ──────────────────────────────────────────

info "Step 0: Source repository → $SOURCE_DIR"

if [ -d "$SOURCE_DIR/.git" ]; then
    info "Repository exists, pulling latest..."
    git -C "$SOURCE_DIR" pull --ff-only 2>/dev/null || {
        warn "Fast-forward pull failed. Resetting to origin/main..."
        git -C "$SOURCE_DIR" fetch origin
        git -C "$SOURCE_DIR" reset --hard origin/main
    }
    ok "Updated to latest"
else
    mkdir -p "$(dirname "$SOURCE_DIR")"
    info "Cloning repository..."
    git clone "$REPO_URL" "$SOURCE_DIR"
    ok "Cloned successfully"
fi

# ── Step 1: Deploy default-agent ────────────────────────────────────────────

info "Step 1: Agent config → $AGENT_DIR"

mkdir -p "$AGENT_DIR"
cp "$SOURCE_DIR/default-agent/agent.yaml"  "$AGENT_DIR/agent.yaml"
cp "$SOURCE_DIR/default-agent/sub.yaml"    "$AGENT_DIR/sub.yaml"
cp "$SOURCE_DIR/default-agent/system.md"   "$AGENT_DIR/system.md"
ok "kerrigan-agent deployed"

# ── Step 2: Deploy skills ───────────────────────────────────────────────────

info "Step 2: Skills → $SKILLS_DIR"

mkdir -p "$SKILLS_DIR"
# Copy each skill directory (kerrigan, feishu-research, changelog, etc.)
for skill_dir in "$SOURCE_DIR"/skills/*/; do
    skill_name="$(basename "$skill_dir")"
    # Only copy directories that contain a SKILL.md
    if [ -f "$skill_dir/SKILL.md" ]; then
        rm -rf "${SKILLS_DIR:?}/$skill_name"
        cp -r "$skill_dir" "$SKILLS_DIR/$skill_name"
        ok "Skill: $skill_name"
    fi
done

# ── Step 3: Build kerrigan CLI (optional) ───────────────────────────────────

info "Step 3: Build kerrigan CLI"

if command -v cargo >/dev/null 2>&1; then
    info "cargo found, building kerrigan..."
    cargo install --path "$SOURCE_DIR" --quiet 2>/dev/null && {
        # Copy to ~/.local/bin if it exists (common on Linux)
        if [ -d "$HOME/.local/bin" ]; then
            cp "$HOME/.cargo/bin/kerrigan" "$HOME/.local/bin/kerrigan" 2>/dev/null || true
        fi
        ok "kerrigan CLI installed ($(kerrigan --version 2>/dev/null || echo 'unknown version'))"
    } || {
        warn "cargo install failed — you can build manually later with: cargo install --path $SOURCE_DIR"
    }
else
    warn "cargo not found — skipping CLI build"
    warn "Install Rust (https://rustup.rs) then run: cargo install --path $SOURCE_DIR"
fi

# ── Summary ──────────────────────────────────────────────────────────────────

echo ""
info "Kerrigan installed successfully!"
echo ""
echo "  Source:  $SOURCE_DIR"
echo "  Agent:   $AGENT_DIR"
echo "  Skills:  $SKILLS_DIR"
echo ""
echo "  Make the skill visible to Claude Code:"
echo "    ln -sfn $SKILLS_DIR/kerrigan ~/.claude/skills/kerrigan"
echo ""
echo "  To update, re-run this script."
echo ""
