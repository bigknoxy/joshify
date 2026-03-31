#!/usr/bin/env bash
# Joshify One-Line Installer
#
# Usage: curl -fsSL https://raw.githubusercontent.com/joshify/joshify/main/install.sh | bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "⚡ Joshify Installer ⚡"
echo "====================="
echo ""

# Detect OS
OS="$(uname -s)"
ARCH="$(uname -m)"

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

git clone https://github.com/joshify/joshify.git
cd joshify

cargo install --path . --locked

# Clean up
cd - > /dev/null
rm -rf "$TEMP_DIR"

echo ""
echo -e "${GREEN}✓ Joshify installed successfully!${NC}"
echo ""
echo "Run 'joshify' to start the app."
echo ""
