#!/usr/bin/env bash
# Joshify Uninstaller
#
# Usage: curl -fsSL https://raw.githubusercontent.com/bigknoxy/joshify/main/uninstall.sh | bash

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "⚡ Joshify Uninstaller ⚡"
echo "======================="
echo ""

BIN_NAME="joshify"
CONFIG_DIR="$HOME/.config/joshify"
CACHE_DIR="$HOME/.cache/joshify"

# Remove every copy we might have installed. install.sh can place the binary in
# ~/.cargo/bin, ~/.local/bin, or a JOSHIFY_INSTALL_DIR, and a cargo-installed
# copy needs cargo uninstall to clear its registry entry. Removing all of them
# keeps repeated runs idempotent.
REMOVED=0
for DIR in "${JOSHIFY_INSTALL_DIR:-}" "$HOME/.cargo/bin" "$HOME/.local/bin"; do
    [ -n "$DIR" ] || continue
    [ -e "$DIR/$BIN_NAME" ] || continue

    if [ "$DIR" = "$HOME/.cargo/bin" ] && command -v cargo &> /dev/null \
       && cargo install --list 2>/dev/null | grep -q "^$BIN_NAME "; then
        echo -e "${YELLOW}Removing cargo-installed binary from $DIR...${NC}"
        cargo uninstall "$BIN_NAME" > /dev/null 2>&1 || rm -f "$DIR/$BIN_NAME"
    else
        echo -e "${YELLOW}Removing $DIR/$BIN_NAME...${NC}"
        rm -f "$DIR/$BIN_NAME"
    fi
    REMOVED=$((REMOVED + 1))
done

# Anything left on PATH was installed some other way.
if command -v "$BIN_NAME" &> /dev/null; then
    BIN_PATH=$(command -v "$BIN_NAME")
    echo -e "${YELLOW}Removing $BIN_PATH...${NC}"
    rm -f "$BIN_PATH" 2>/dev/null || sudo rm -f "$BIN_PATH"
    REMOVED=$((REMOVED + 1))
fi

if [ "$REMOVED" -eq 0 ]; then
    echo "No installed binary found - nothing to remove"
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
