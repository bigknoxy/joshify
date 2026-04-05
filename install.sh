#!/usr/bin/env bash
# Joshify One-Line Installer
#
# Usage: curl -fsSL https://raw.githubusercontent.com/bigknoxy/joshify/main/install.sh | bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

REPO="https://github.com/bigknoxy/joshify.git"
BIN_NAME="joshify"

echo "⚡ Joshify Installer ⚡"
echo "====================="
echo ""

# Check for Rust installation
if ! command -v cargo &> /dev/null; then
    echo -e "${YELLOW}Rust not found. Installing Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env" 2>/dev/null || true
fi

echo -e "${GREEN}Rust found: $(cargo --version)${NC}"
echo ""

# Install system dependencies for librespot (Linux only)
echo "📦 Checking system dependencies..."
if command -v apt-get &> /dev/null; then
    echo -e "${YELLOW}Detected Debian/Ubuntu - installing audio dependencies...${NC}"
    sudo apt-get update -qq
    sudo apt-get install -y -qq libasound2-dev pkg-config libssl-dev build-essential
elif command -v dnf &> /dev/null; then
    echo -e "${YELLOW}Detected Fedora/RHEL - installing audio dependencies...${NC}"
    sudo dnf install -y alsa-lib-devel pkgconfig openssl-devel gcc
elif command -v pacman &> /dev/null; then
    echo -e "${YELLOW}Detected Arch - installing audio dependencies...${NC}"
    sudo pacman -S --noconfirm alsa-lib pkg-config openssl base-devel
elif command -v brew &> /dev/null; then
    echo -e "${GREEN}Detected macOS - no additional system dependencies needed${NC}"
else
    echo -e "${YELLOW}Unknown OS - you may need to install audio dependencies manually${NC}"
    echo "   Linux: libasound2-dev pkg-config libssl-dev build-essential"
    echo "   macOS: No extra deps needed"
fi

# Clone and install
echo ""
echo "🔨 Building and installing Joshify..."
TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

git clone "$REPO"
cd joshify

cargo install --path .

# Clean up
cd - > /dev/null
rm -rf "$TEMP_DIR"

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
