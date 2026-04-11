#!/usr/bin/env bash
#
# Kerrigan — multi-agent orchestration framework (all components)
# One-line installer:
#   curl -fsSL https://github.com/YoungY620/harness/kerrigan/-/raw/main/install.sh | bash
#
# Individual components:
#   curl -fsSL https://github.com/YoungY620/harness/kerrigan-storage/-/raw/main/install.sh | bash
#   curl -fsSL https://github.com/YoungY620/harness/kerrigan-cli/-/raw/main/install.sh | bash
#
# Environment variables:
#   INSTALL_DIR   — where to place binaries     (default: ~/.local/bin)
#   CLONE_METHOD  — "ssh" or "https"             (default: auto-detect)
#   KERRIGAN_REF  — git ref to checkout          (default: main)
#   KEEP_SOURCE   — set to "1" to keep cloned source after build
#
set -euo pipefail

# ── Constants ──

REPO_SSH="ssh://git@github.com/YoungY620:30022/harness/kerrigan.git"
REPO_HTTPS="https://github.com/YoungY620/Kerrigan.git"
BINARIES=("kerrigan-cli" "kerrigan-storage")
VERSION="0.1.0"

# ── Configurable ──

INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
KERRIGAN_REF="${KERRIGAN_REF:-main}"
KEEP_SOURCE="${KEEP_SOURCE:-}"

# ── Helpers ──

info()  { printf '\033[1;34m==>\033[0m \033[1m%s\033[0m\n' "$*"; }
warn()  { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }
error() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

need_cmd() {
    if ! command -v "$1" &>/dev/null; then
        error "required command not found: $1"
    fi
}

# ── Platform detection ──

detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)  OS="linux" ;;
        Darwin) OS="macos" ;;
        *)      error "unsupported OS: $os" ;;
    esac

    case "$arch" in
        x86_64|amd64)   ARCH="x86_64" ;;
        aarch64|arm64)  ARCH="aarch64" ;;
        *)              error "unsupported architecture: $arch" ;;
    esac

    info "Platform: $OS/$ARCH"
}

# ── Rust toolchain ──

ensure_rust() {
    if command -v cargo &>/dev/null; then
        info "Rust toolchain found: $(rustc --version)"
        return
    fi

    info "Rust not found — installing via rustup..."
    need_cmd curl
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
    info "Rust installed: $(rustc --version)"
}

# ── Git clone ──

clone_repo() {
    local method="${CLONE_METHOD:-}"
    local tmpdir
    tmpdir="$(mktemp -d)"
    CLONE_DIR="$tmpdir/kerrigan"

    # Auto-detect clone method: try SSH first (dev machines typically have keys),
    # fall back to HTTPS
    if [ -z "$method" ]; then
        if ssh -o BatchMode=yes -o ConnectTimeout=5 -p 30022 git@github.com/YoungY620 2>&1 | grep -qi "welcome\|gitlab\|success\|PTY"; then
            method="ssh"
        else
            method="https"
        fi
    fi

    local repo_url
    case "$method" in
        ssh)   repo_url="$REPO_SSH" ;;
        https) repo_url="$REPO_HTTPS" ;;
        *)     error "unknown CLONE_METHOD: $method (use 'ssh' or 'https')" ;;
    esac

    info "Cloning $repo_url (ref: $KERRIGAN_REF)..."
    need_cmd git
    git clone --recurse-submodules --depth 1 --branch "$KERRIGAN_REF" "$repo_url" "$CLONE_DIR" 2>&1 \
        || error "git clone failed. If using SSH, ensure your key is configured.\n  Try: CLONE_METHOD=https curl -fsSL ... | bash"
}

# ── Build ──

build_release() {
    info "Building kerrigan (release mode)..."

    # Build kerrigan-storage first (it's a dependency)
    (cd "$CLONE_DIR/kerrigan-storage" && cargo build --release --quiet)
    info "Built kerrigan-storage"

    # Build kerrigan-cli (depends on kerrigan-storage via path dep)
    (cd "$CLONE_DIR/kerrigan-cli" && cargo build --release --quiet)
    info "Built kerrigan-cli"
}

# ── Install ──

install_binaries() {
    info "Installing to $INSTALL_DIR..."
    mkdir -p "$INSTALL_DIR"

    for bin in "${BINARIES[@]}"; do
        local src
        # kerrigan-cli builds inside its own target/, kerrigan-storage in its own
        src="$CLONE_DIR/$bin/target/release/$bin"
        if [ ! -f "$src" ]; then
            error "binary not found: $src"
        fi
        cp "$src" "$INSTALL_DIR/$bin"
        chmod +x "$INSTALL_DIR/$bin"
        info "  $INSTALL_DIR/$bin"
    done
}

# ── Post-install ──

post_install() {
    # Initialize ~/.kerrigan/ (idempotent)
    info "Initializing ~/.kerrigan/..."
    "$INSTALL_DIR/kerrigan-storage" init

    # Check if INSTALL_DIR is in PATH
    case ":$PATH:" in
        *":$INSTALL_DIR:"*) ;;
        *)
            warn "$INSTALL_DIR is not in your PATH"
            echo ""
            echo "  Add to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
            echo "    export PATH=\"$INSTALL_DIR:\$PATH\""
            echo ""
            ;;
    esac

    # Check for agent CLI
    if command -v claude &>/dev/null; then
        info "Agent backend: claude (default)"
    elif command -v kimi &>/dev/null; then
        info "Agent backend: kimi (use --adapter kimi to select)"
    else
        warn "No agent CLI found"
        echo ""
        echo "  Kerrigan requires at least one agent backend to run tasks:"
        echo "    Claude Code: https://docs.anthropic.com/en/docs/claude-code"
        echo "    Kimi CLI:    (internal tool)"
        echo ""
        echo "  Default: --adapter claude"
    fi
}

# ── Cleanup ──

cleanup() {
    if [ -n "${CLONE_DIR:-}" ] && [ -z "$KEEP_SOURCE" ]; then
        info "Cleaning up build artifacts..."
        rm -rf "$(dirname "$CLONE_DIR")"
    elif [ -n "${CLONE_DIR:-}" ]; then
        info "Source kept at: $CLONE_DIR"
    fi
}

# ── Main ──

main() {
    echo ""
    echo "  ╔═══════════════════════════════════════╗"
    echo "  ║   Kerrigan Installer  v${VERSION}          ║"
    echo "  ║   Multi-Agent Orchestration Framework ║"
    echo "  ╚═══════════════════════════════════════╝"
    echo ""

    detect_platform
    ensure_rust
    clone_repo

    trap cleanup EXIT

    build_release
    install_binaries
    post_install

    echo ""
    info "Kerrigan installed successfully!"
    echo ""
    echo "  Binaries:"
    echo "    kerrigan-storage  — DAG engine & storage management"
    echo "    kerrigan-cli      — orchestration runner"
    echo ""
    echo "  Quick start:"
    echo "    kerrigan-storage task create --summary \"my task\" --workdir ."
    echo "    kerrigan-cli run <task-id> --adapter claude"
    echo ""
}

main "$@"
