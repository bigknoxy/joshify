#!/usr/bin/env bash
# Joshify One-Line Installer
#
# Usage: curl -fsSL https://raw.githubusercontent.com/bigknoxy/joshify/main/install.sh | bash
#
# Environment variables:
#   TMPDIR              Preferred temp dir. Respected if it can execute binaries.
#   JOSHIFY_SKIP_DEPS   Set to 1 to skip the system dependency install entirely.

set -eu

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

REPO="https://github.com/bigknoxy/joshify.git"
BIN_NAME="joshify"

# Packages we need, per package manager. Also printed when we have to skip.
DEPS_APT="libasound2-dev pkg-config libssl-dev build-essential libchafa-dev libglib2.0-dev"
DEPS_DNF="alsa-lib-devel pkgconfig openssl-devel gcc chafa-devel glib2-devel"
DEPS_PACMAN="alsa-lib pkg-config openssl base-devel chafa glib2"
DEPS_BREW="pkgconf chafa"

# --- temp dir handling -------------------------------------------------------
# /tmp is mounted noexec on WSL2 and many hardened Linux hosts. rustup-init and
# build scripts need to execute binaries out of the temp dir, so probe it first.

# can_exec_in DIR -> 0 if a binary written to DIR can be executed
can_exec_in() {
    local dir="$1" probe
    [ -d "$dir" ] && [ -w "$dir" ] || return 1

    probe="$dir/.joshify-exec-probe.$$"
    printf '#!/bin/sh\nexit 0\n' > "$probe" 2>/dev/null || return 1
    chmod +x "$probe" 2>/dev/null || { rm -f "$probe"; return 1; }

    if "$probe" >/dev/null 2>&1; then
        rm -f "$probe"
        return 0
    fi
    rm -f "$probe"
    return 1
}

# Echo the first temp dir that can execute binaries: $TMPDIR, /tmp, then
# $HOME/.cache/joshify/tmp as a fallback we create ourselves.
find_exec_tmpdir() {
    local candidate fallback
    for candidate in "${TMPDIR:-}" /tmp; do
        [ -n "$candidate" ] || continue
        if can_exec_in "$candidate"; then
            echo "$candidate"
            return 0
        fi
    done

    fallback="${HOME}/.cache/joshify/tmp"
    mkdir -p "$fallback" 2>/dev/null || return 1
    if can_exec_in "$fallback"; then
        echo "$fallback"
        return 0
    fi
    return 1
}

# --- sudo handling -----------------------------------------------------------
# With `curl | bash`, stdin is the script, so sudo has nothing to prompt on.
# /dev/tty still works when a real terminal is attached, so prefer that.

have_tty() {
    [ -e /dev/tty ] && { : < /dev/tty; } 2>/dev/null
}

# SUDO_MODE is one of: none (running as root), passwordless, tty, unavailable
detect_sudo_mode() {
    if [ "$(id -u)" = "0" ]; then
        echo "none"
    elif ! command -v sudo > /dev/null 2>&1; then
        echo "unavailable"
    elif sudo -n true 2>/dev/null; then
        echo "passwordless"
    elif have_tty; then
        echo "tty"
    else
        echo "unavailable"
    fi
}

# Run a command with elevation, according to SUDO_MODE.
run_privileged() {
    case "$SUDO_MODE" in
        none)         "$@" ;;
        passwordless) sudo -n "$@" ;;
        tty)          sudo "$@" < /dev/tty ;;
        *)            return 1 ;;
    esac
}

print_manual_deps() {
    echo -e "${YELLOW}Skipping system dependency install.${NC}"
    echo "   Install these yourself before building:"
    echo "     Debian/Ubuntu: sudo apt-get install -y $DEPS_APT"
    echo "     Fedora/RHEL:   sudo dnf install -y $DEPS_DNF"
    echo "     Arch:          sudo pacman -S --noconfirm $DEPS_PACMAN"
    echo "     macOS:         brew install $DEPS_BREW"
}

# Sourced by tests to get the helpers without running the installer.
if [ -n "${JOSHIFY_INSTALL_LIB_ONLY:-}" ]; then
    return 0 2>/dev/null || exit 0
fi

echo "⚡ Joshify Installer ⚡"
echo "====================="
echo ""

# Pick a temp dir that can actually execute binaries, and use it everywhere.
if EXEC_TMPDIR=$(find_exec_tmpdir); then
    if [ "$EXEC_TMPDIR" != "${TMPDIR:-/tmp}" ]; then
        echo -e "${YELLOW}Default temp dir cannot execute binaries (noexec?).${NC}"
        echo "   Using $EXEC_TMPDIR instead."
        echo ""
    fi
    export TMPDIR="$EXEC_TMPDIR"
else
    echo -e "${YELLOW}Warning: could not find a temp dir that allows executing binaries.${NC}"
    echo "   Set TMPDIR to a path on an exec-enabled filesystem and re-run, e.g.:"
    echo "     export TMPDIR=\"\$HOME/.tmp-exec\" && mkdir -p \"\$TMPDIR\""
    echo ""
fi

# Pick the package manager. On macOS that is Homebrew; on Linux the native
# manager wins even when linuxbrew is present, because brew alone does not
# provide the -dev packages the build needs.
PKG_MGR=""
if [ "$(uname -s)" = "Darwin" ]; then
    if command -v brew > /dev/null 2>&1; then PKG_MGR="brew"; fi
elif command -v apt-get > /dev/null 2>&1; then
    PKG_MGR="apt"
elif command -v dnf > /dev/null 2>&1; then
    PKG_MGR="dnf"
elif command -v pacman > /dev/null 2>&1; then
    PKG_MGR="pacman"
elif command -v brew > /dev/null 2>&1; then
    PKG_MGR="brew"
fi

# Work out how (or whether) we can elevate, and prime the credential cache now
# rather than after the slow Rust install. brew never needs sudo.
SUDO_MODE=$(detect_sudo_mode)
NEEDS_SUDO=no
if [ -n "$PKG_MGR" ] && [ "$PKG_MGR" != "brew" ] && [ -z "${JOSHIFY_SKIP_DEPS:-}" ]; then
    NEEDS_SUDO=yes
fi

if [ "$SUDO_MODE" = "tty" ] && [ "$NEEDS_SUDO" = "yes" ]; then
    echo "🔑 System dependencies need sudo. Authenticating up front..."
    if ! sudo -v < /dev/tty; then
        echo -e "${YELLOW}sudo authentication failed.${NC}"
        SUDO_MODE="unavailable"
    fi
    echo ""
fi

# Check for Rust installation
if ! command -v cargo > /dev/null 2>&1; then
    echo -e "${YELLOW}Rust not found. Installing Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env" 2>/dev/null || true
fi

if ! command -v cargo > /dev/null 2>&1; then
    echo -e "${YELLOW}Rust installed but 'cargo' is not on PATH.${NC}"
    echo "   Run: source \"\$HOME/.cargo/env\"  then re-run this installer."
    exit 1
fi

echo -e "${GREEN}Rust found: $(cargo --version)${NC}"
echo ""

# Install system dependencies for librespot
echo "📦 Checking system dependencies..."

# Everything except brew needs elevation.
if [ -n "${JOSHIFY_SKIP_DEPS:-}" ]; then
    echo -e "${YELLOW}JOSHIFY_SKIP_DEPS is set.${NC}"
    print_manual_deps
elif [ -z "$PKG_MGR" ]; then
    echo -e "${YELLOW}Unknown OS - no supported package manager found.${NC}"
    print_manual_deps
elif [ "$PKG_MGR" != "brew" ] && [ "$SUDO_MODE" = "unavailable" ]; then
    echo -e "${YELLOW}No TTY and no passwordless sudo available.${NC}"
    print_manual_deps
    echo ""
    echo "   Then re-run with JOSHIFY_SKIP_DEPS=1 to skip this step."
else
    case "$PKG_MGR" in
        brew)
            echo -e "${YELLOW}Detected Homebrew - installing terminal graphics dependencies...${NC}"
            # shellcheck disable=SC2086  # intentional word splitting into package args
            brew install $DEPS_BREW
            ;;
        apt)
            echo -e "${YELLOW}Detected Debian/Ubuntu - installing audio dependencies...${NC}"
            run_privileged apt-get update -qq
            # shellcheck disable=SC2086
            run_privileged env DEBIAN_FRONTEND=noninteractive apt-get install -y -qq $DEPS_APT
            ;;
        dnf)
            echo -e "${YELLOW}Detected Fedora/RHEL - installing audio dependencies...${NC}"
            # shellcheck disable=SC2086
            run_privileged dnf install -y $DEPS_DNF
            ;;
        pacman)
            echo -e "${YELLOW}Detected Arch - installing audio dependencies...${NC}"
            # shellcheck disable=SC2086
            run_privileged pacman -S --noconfirm $DEPS_PACMAN
            ;;
    esac
fi

# Clone and install
echo ""
echo "🔨 Building and installing Joshify..."
TEMP_DIR=$(mktemp -d)
cleanup() { rm -rf "$TEMP_DIR"; }
trap cleanup EXIT

git clone --depth 1 "$REPO" "$TEMP_DIR/joshify"
cargo install --path "$TEMP_DIR/joshify"

echo ""
echo -e "${GREEN}✓ Joshify installed successfully!${NC}"
echo ""
echo "Run '$BIN_NAME' to start the app."
echo ""
echo -e "${YELLOW}Optional: Non-interactive mode (skip browser auth)${NC}"
echo "Add these to your ~/.bashrc to skip OAuth setup:"
echo "  export SPOTIFY_CLIENT_ID=your_client_id"
echo "  export SPOTIFY_CLIENT_SECRET=your_client_secret"
echo "  export SPOTIFY_ACCESS_TOKEN=your_access_token"
echo ""
echo "🎵 Joshify now plays audio locally through your machine's speakers!"
echo "   Press 'd' to switch between local and remote devices."
echo ""
echo "Uninstall with: curl -fsSL https://raw.githubusercontent.com/bigknoxy/joshify/main/uninstall.sh | bash"
echo ""
