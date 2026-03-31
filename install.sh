#!/usr/bin/env bash
# Joshify One-Line Installer
#
# Usage: curl -fsSL https://raw.githubusercontent.com/bigknoxy/joshify/master/install.sh | bash

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

# Clone and install
echo "Installing Joshify..."
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
echo "Uninstall with: curl -fsSL https://raw.githubusercontent.com/bigknoxy/joshify/main/uninstall.sh | bash"
echo ""
