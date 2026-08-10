#!/usr/bin/env bash
# Joshify One-Line Installer
#
# Usage: curl -fsSL https://raw.githubusercontent.com/bigknoxy/joshify/main/install.sh | bash
#
# Strategy:
#   1. Download a prebuilt binary from the latest GitHub release (fast path).
#   2. If no matching prebuilt asset exists, fall back to building from source,
#      installing the required native dependencies first (chafa is REQUIRED by
#      ratatui-image at build time — missing it fails the build).

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

REPO_OWNER="bigknoxy"
REPO_NAME="joshify"
BIN_NAME="joshify"
BIN_DIR="$HOME/.local/bin"

echo "⚡ Joshify Installer ⚡"
echo "====================="
echo ""

# Detect OS and architecture for the prebuilt asset name
detect_os_arch() {
    OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
    ARCH="$(uname -m)"

    case "$OS" in
        linux) OS_STR="linux" ;;
        darwin) OS_STR="macos" ;;
        *)
            echo -e "${RED}Unsupported OS: $OS (prebuilt binaries unavailable; falling back to source build)${NC}"
            return 1
            ;;
    esac

    case "$ARCH" in
        x86_64|amd64) ARCH_STR="x86_64" ;;
        aarch64|arm64) ARCH_STR="aarch64" ;;
        *)
            echo -e "${RED}Unsupported arch: $ARCH (prebuilt binaries unavailable; falling back to source build)${NC}"
            return 1
            ;;
    esac
}

# Fetch the latest release tag from the GitHub API
get_latest_release_tag() {
    curl -fsSL "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest" \
        | grep '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\(.*\)".*/\1/'
}

# Download and install a prebuilt binary. Returns 0 on success.
install_prebuilt() {
    echo "🦀 Attempting to download a prebuilt binary..."

    if ! detect_os_arch; then
        return 1
    fi

    # librespot needs an audio backend; macOS builds need chafa compiled against
    # native libs. Prebuilt assets are only published for supported combos, so
    # probe for the exact asset name before downloading.
    ASSET_NAME="${BIN_NAME}-${OS_STR}-${ARCH_STR}.tar.gz"

    TAG="$(get_latest_release_tag 2>/dev/null || true)"
    if [ -z "$TAG" ]; then
        echo -e "${YELLOW}Could not determine latest release tag. Falling back to source build.${NC}"
        return 1
    fi

    URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${TAG}/${ASSET_NAME}"
    echo -e "Fetching ${GREEN}${URL}${NC}"

    if ! curl -fsSL "$URL" -o /tmp/"${ASSET_NAME}"; then
        echo -e "${YELLOW}No prebuilt asset '${ASSET_NAME}' in release ${TAG}. Falling back to source build.${NC}"
        return 1
    fi

    mkdir -p "$BIN_DIR" /tmp/joshify-install
    tar xzf /tmp/"${ASSET_NAME}" -C /tmp/joshify-install

    # The tarball contains a single binary named "joshify-<os>-<arch>"
    BINARY_PATH="$(find /tmp/joshify-install -maxdepth 1 -type f -name "${BIN_NAME}-*" | head -1)"
    if [ -z "$BINARY_PATH" ]; then
        BINARY_PATH="$(find /tmp/joshify-install -maxdepth 1 -type f -name "${BIN_NAME}" | head -1)"
    fi
    if [ -z "$BINARY_PATH" ] || [ ! -x "$BINARY_PATH" ]; then
        echo -e "${YELLOW}Prebuilt archive did not contain a usable binary. Falling back to source build.${NC}"
        rm -rf /tmp/joshify-install /tmp/"${ASSET_NAME}"
        return 1
    fi

    install -m 0755 "$BINARY_PATH" "$BIN_DIR/${BIN_NAME}"
    rm -rf /tmp/joshify-install /tmp/"${ASSET_NAME}"

    # Ensure BIN_DIR is on PATH
    case ":$PATH:" in
        *":$BIN_DIR:"*) ;;
        *) export PATH="$BIN_DIR:$PATH"
           echo -e "${YELLOW}Note: add ${BIN_DIR} to your PATH (e.g. in ~/.bashrc):${NC}"
           echo "  export PATH=\"\$HOME/.local/bin:\$PATH\"" ;;
    esac

    echo -e "${GREEN}✓ Joshify ${TAG} installed to ${BIN_DIR}/${BIN_NAME}${NC}"
    return 0
}

# Install native dependencies needed for a source build (Linux only).
install_build_deps() {
    echo "📦 Checking build dependencies..."
    if command -v apt-get &> /dev/null; then
        echo -e "${YELLOW}Detected Debian/Ubuntu - installing build dependencies...${NC}"
        sudo apt-get update -qq
        sudo apt-get install -y -qq libasound2-dev pkg-config libssl-dev build-essential libchafa-dev libglib2.0-dev
    elif command -v dnf &> /dev/null; then
        echo -e "${YELLOW}Detected Fedora/RHEL - installing build dependencies...${NC}"
        sudo dnf install -y alsa-lib-devel pkgconfig openssl-devel gcc chafa-devel glib2-devel
    elif command -v pacman &> /dev/null; then
        echo -e "${YELLOW}Detected Arch - installing build dependencies...${NC}"
        sudo pacman -S --noconfirm alsa-lib pkg-config openssl base-devel chafa glib2
    elif command -v brew &> /dev/null; then
        echo -e "${GREEN}Detected macOS - installing build dependencies...${NC}"
        brew install pkg-config chafa || true
    else
        echo -e "${YELLOW}Unknown OS - you may need to install build dependencies manually${NC}"
        echo "   Debian/Ubuntu: libasound2-dev pkg-config libssl-dev build-essential libchafa-dev libglib2.0-dev"
        echo "   Fedora:        alsa-lib-devel pkgconfig openssl-devel gcc chafa-devel glib2-devel"
        echo "   Arch:          alsa-lib pkg-config openssl base-devel chafa glib2"
        echo "   macOS:         brew install pkg-config chafa"
    fi
}

# Build from source. Requires cargo + native deps.
install_from_source() {
    echo "🔨 Building from source..."

    # Check for Rust installation
    if ! command -v cargo &> /dev/null; then
        echo -e "${YELLOW}Rust not found. Installing Rust...${NC}"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env" 2>/dev/null || true
    fi
    echo -e "${GREEN}Rust found: $(cargo --version)${NC}"

    install_build_deps

    TEMP_DIR=$(mktemp -d)
    cd "$TEMP_DIR"

    echo "Cloning ${REPO_OWNER}/${REPO_NAME}..."
    git clone --depth 1 "https://github.com/${REPO_OWNER}/${REPO_NAME}.git"
    cd "${REPO_NAME}"

    cargo install --path . --locked

    # cargo install places the binary in ~/.cargo/bin
    cd - > /dev/null
    rm -rf "$TEMP_DIR"

    case ":$PATH:" in
        *":$HOME/.cargo/bin:"*) ;;
        *) export PATH="$HOME/.cargo/bin:$PATH"
           echo -e "${YELLOW}Note: add ~/.cargo/bin to your PATH (e.g. in ~/.bashrc):${NC}"
           echo "  export PATH=\"\$HOME/.cargo/bin:\$PATH\"" ;;
    esac
}

# Main: prefer prebuilt, fall back to source.
if install_prebuilt; then
    echo ""
else
    echo ""
    echo -e "${YELLOW}Falling back to building from source (this takes a few minutes).${NC}"
    install_from_source
fi

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
