#!/usr/bin/env bash
# Joshify Uninstaller
#
# Usage: curl -fsSL https://raw.githubusercontent.com/bigknoxy/joshify/master/uninstall.sh | bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "⚡ Joshify Uninstaller ⚡"
echo "======================="
echo ""

BIN_NAME="joshify"
CONFIG_DIR="$HOME/.config/joshify"
CACHE_DIR="$HOME/.cache/joshify"

# Find and remove binary
if command -v "$BIN_NAME" &> /dev/null; then
    BIN_PATH=$(which "$BIN_NAME")
    echo "Found binary at: $BIN_PATH"

    if [[ "$BIN_PATH" == *"/.cargo/bin/"* ]]; then
        echo -e "${YELLOW}Removing cargo-installed binary...${NC}"
        cargo uninstall "$BIN_NAME"
    else
        echo -e "${YELLOW}Removing system binary...${NC}"
        sudo rm -f "$BIN_PATH"
    fi
else
    echo "Binary not found in PATH"
fi

# Remove config
if [ -d "$CONFIG_DIR" ]; then
    echo "Removing config directory: $CONFIG_DIR"
    rm -rf "$CONFIG_DIR"
fi

# Remove cache
if [ -d "$CACHE_DIR" ]; then
    echo "Removing cache directory: $CACHE_DIR"
    rm -rf "$CACHE_DIR"
fi

# Check for npm/bun installations
if command -v npm &> /dev/null && [ -d "$HOME/.npm/packages/$BIN_NAME" ]; then
    echo -e "${YELLOW}Found npm installation, removing...${NC}"
    npm uninstall -g "$BIN_NAME" 2>/dev/null || true
fi

if command -v bun &> /dev/null && [ -d "$HOME/.bun/install/global/$BIN_NAME" ]; then
    echo -e "${YELLOW}Found bun installation, removing...${NC}"
    bun remove -g "$BIN_NAME" 2>/dev/null || true
fi

echo ""
echo -e "${GREEN}✓ Joshify uninstalled successfully!${NC}"
echo ""
