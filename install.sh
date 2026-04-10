#!/usr/bin/env bash
#
# kerrigan-storage — DAG engine & storage management
# One-line install:
#   curl -fsSL https://github.com/YoungY620/harness/kerrigan-storage/-/raw/main/install.sh | bash
#
# Environment variables:
#   INSTALL_DIR   — where to place the binary  (default: ~/.local/bin)
#   CLONE_METHOD  — "ssh" or "https"            (default: auto-detect)
#   KERRIGAN_REF  — git ref to checkout         (default: main)
#   KEEP_SOURCE   — set to "1" to keep source after build
#
set -euo pipefail

# ── Constants ──

REPO_SSH="ssh://git@github.com/YoungY620:30022/harness/kerrigan-storage.git"
REPO_HTTPS="https://github.com/YoungY620/harness/kerrigan-storage.git"
BINARY="kerrigan-storage"

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

repo_url_for() {
    local method="$1"
    case "$method" in
        ssh)   echo "$REPO_SSH" ;;
        https) echo "$REPO_HTTPS" ;;
        *)     error "unknown CLONE_METHOD: $method (use 'ssh' or 'https')" ;;
    esac
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
    echo "  kerrigan-storage installer"
    echo "  DAG engine & storage management"
    echo ""

    detect_platform
    ensure_rust

    local method tmpdir clone_dir repo_url
    method="$(detect_clone_method)"
    repo_url="$(repo_url_for "$method")"
    tmpdir="$(mktemp -d)"
    clone_dir="$tmpdir/kerrigan-storage"

    info "Cloning $repo_url (ref: $KERRIGAN_REF)..."
    need_cmd git
    git clone --depth 1 --branch "$KERRIGAN_REF" "$repo_url" "$clone_dir" 2>&1 \
        || error "git clone failed. Try: CLONE_METHOD=https curl -fsSL ... | bash"

    info "Building $BINARY (release mode)..."
    (cd "$clone_dir" && cargo build --release --quiet)

    mkdir -p "$INSTALL_DIR"
    cp "$clone_dir/target/release/$BINARY" "$INSTALL_DIR/$BINARY"
    chmod +x "$INSTALL_DIR/$BINARY"
    info "Installed $INSTALL_DIR/$BINARY"

    info "Initializing ~/.kerrigan/..."
    "$INSTALL_DIR/$BINARY" init

    # Cleanup
    if [ -z "$KEEP_SOURCE" ]; then
        rm -rf "$tmpdir"
    else
        info "Source kept at: $clone_dir"
    fi

    check_path

    echo ""
    info "kerrigan-storage installed successfully!"
    echo ""
    echo "  Usage:"
    echo "    kerrigan-storage init"
    echo "    kerrigan-storage task create --summary \"my task\" --workdir ."
    echo "    kerrigan-storage task list"
    echo ""
}

main "$@"
