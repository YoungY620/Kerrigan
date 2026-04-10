#!/usr/bin/env bash
#
# kerrigan-cli — multi-agent orchestration runner
# One-line install:
#   curl -fsSL https://github.com/YoungY620/harness/kerrigan-cli/-/raw/main/install.sh | bash
#
# This script also installs kerrigan-storage (build dependency + runtime requirement).
#
# Environment variables:
#   INSTALL_DIR   — where to place binaries     (default: ~/.local/bin)
#   CLONE_METHOD  — "ssh" or "https"             (default: auto-detect)
#   KERRIGAN_REF  — git ref to checkout          (default: main)
#   KEEP_SOURCE   — set to "1" to keep source after build
#
set -euo pipefail

# ── Constants ──
# kerrigan-cli depends on kerrigan-storage via path = "../kerrigan-storage",
# so both repos are cloned side-by-side.

STORAGE_REPO_SSH="ssh://git@github.com/YoungY620:30022/harness/kerrigan-storage.git"
STORAGE_REPO_HTTPS="https://github.com/YoungY620/harness/kerrigan-storage.git"
CLI_REPO_SSH="ssh://git@github.com/YoungY620:30022/harness/kerrigan-cli.git"
CLI_REPO_HTTPS="https://github.com/YoungY620/harness/kerrigan-cli.git"
BINARIES=("kerrigan-cli" "kerrigan-storage")

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

detect_platform() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"
    case "$os" in
        Linux|Darwin) ;;
        *) error "unsupported OS: $os" ;;
    esac
    case "$arch" in
        x86_64|amd64|aarch64|arm64) ;;
        *) error "unsupported architecture: $arch" ;;
    esac
    info "Platform: $os/$arch"
}

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

detect_clone_method() {
    local method="${CLONE_METHOD:-}"
    if [ -z "$method" ]; then
        if ssh -o BatchMode=yes -o ConnectTimeout=5 -p 30022 git@github.com/YoungY620 2>&1 | grep -qi "welcome\|gitlab\|success\|PTY"; then
            method="ssh"
        else
            method="https"
        fi
    fi
    echo "$method"
}

check_path() {
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
}

# ── Main ──

main() {
    echo ""
    echo "  kerrigan-cli installer"
    echo "  Multi-agent orchestration runner"
    echo ""

    detect_platform
    ensure_rust
    need_cmd git

    local method tmpdir
    method="$(detect_clone_method)"
    tmpdir="$(mktemp -d)"

    # Clone both repos side-by-side (kerrigan-cli depends on ../kerrigan-storage)
    local storage_url cli_url
    case "$method" in
        ssh)
            storage_url="$STORAGE_REPO_SSH"
            cli_url="$CLI_REPO_SSH"
            ;;
        https)
            storage_url="$STORAGE_REPO_HTTPS"
            cli_url="$CLI_REPO_HTTPS"
            ;;
        *) error "unknown CLONE_METHOD: $method" ;;
    esac

    info "Cloning kerrigan-storage (ref: $KERRIGAN_REF)..."
    git clone --depth 1 --branch "$KERRIGAN_REF" "$storage_url" "$tmpdir/kerrigan-storage" 2>&1 \
        || error "git clone kerrigan-storage failed. Try: CLONE_METHOD=https curl -fsSL ... | bash"

    info "Cloning kerrigan-cli (ref: $KERRIGAN_REF)..."
    git clone --depth 1 --branch "$KERRIGAN_REF" "$cli_url" "$tmpdir/kerrigan-cli" 2>&1 \
        || error "git clone kerrigan-cli failed."

    # Build kerrigan-storage
    info "Building kerrigan-storage (release mode)..."
    (cd "$tmpdir/kerrigan-storage" && cargo build --release --quiet)

    # Build kerrigan-cli (picks up ../kerrigan-storage via path dep)
    info "Building kerrigan-cli (release mode)..."
    (cd "$tmpdir/kerrigan-cli" && cargo build --release --quiet)

    # Install both binaries
    mkdir -p "$INSTALL_DIR"
    for bin in "${BINARIES[@]}"; do
        cp "$tmpdir/$bin/target/release/$bin" "$INSTALL_DIR/$bin"
        chmod +x "$INSTALL_DIR/$bin"
        info "Installed $INSTALL_DIR/$bin"
    done

    # Initialize storage
    info "Initializing ~/.kerrigan/..."
    "$INSTALL_DIR/kerrigan-storage" init

    # Cleanup
    if [ -z "$KEEP_SOURCE" ]; then
        rm -rf "$tmpdir"
    else
        info "Source kept at: $tmpdir"
    fi

    check_path

    echo ""
    info "kerrigan-cli installed successfully!"
    echo ""
    echo "  Usage:"
    echo "    kerrigan-storage task create --summary \"my task\" --workdir ."
    echo "    kerrigan-cli run <task-id> --adapter claude"
    echo ""
}

main "$@"
