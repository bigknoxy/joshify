#!/bin/bash
# VHS Setup Script
# Installs VHS and ttyd for visual TUI testing
# Usage: ./scripts/vhs-setup.sh

set -euo pipefail

VHS_VERSION="0.11.0"
TTYD_VERSION="1.7.7"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "🎬 Setting up VHS for Joshify visual testing..."

# Detect architecture
ARCH=$(uname -m)
if [ "$ARCH" = "x86_64" ]; then
    VHS_ARCH="amd64"
    TTYD_ARCH="x86_64"
elif [ "$ARCH" = "aarch64" ]; then
    VHS_ARCH="arm64"
    TTYD_ARCH="aarch64"
else
    echo -e "${RED}❌ Unsupported architecture: $ARCH${NC}"
    exit 1
fi

echo "📦 Detected architecture: $ARCH"

# Check for required dependencies
echo "🔍 Checking dependencies..."

if ! command -v ffmpeg &> /dev/null; then
    echo -e "${YELLOW}⚠️  ffmpeg not found. Installing...${NC}"
    if command -v apt-get &> /dev/null; then
        sudo apt-get update && sudo apt-get install -y ffmpeg
    elif command -v brew &> /dev/null; then
        brew install ffmpeg
    else
        echo -e "${RED}❌ Please install ffmpeg manually${NC}"
        exit 1
    fi
fi

# Create temporary directory
TMP_DIR=$(mktemp -d)
trap "rm -rf $TMP_DIR" EXIT

# Download VHS
echo "⬇️  Downloading VHS v${VHS_VERSION}..."
VHS_URL="https://github.com/charmbracelet/vhs/releases/download/v${VHS_VERSION}/vhs_${VHS_VERSION}_linux_${VHS_ARCH}.deb"
VHS_DEB="$TMP_DIR/vhs.deb"

if ! wget -q "$VHS_URL" -O "$VHS_DEB" 2>/dev/null; then
    echo -e "${RED}❌ Failed to download VHS${NC}"
    exit 1
fi

# Extract VHS binary
echo "📂 Extracting VHS..."
dpkg-deb -x "$VHS_DEB" "$TMP_DIR/vhs_extract" 2>/dev/null || {
    # Fallback: try to extract with ar
    cd "$TMP_DIR"
    ar x "$VHS_DEB" data.tar.xz
    tar -xf data.tar.xz
    mv usr vhs_extract/
}

# Download ttyd
echo "⬇️  Downloading ttyd v${TTYD_VERSION}..."
TTYD_URL="https://github.com/tsl0922/ttyd/releases/download/${TTYD_VERSION}/ttyd.${TTYD_ARCH}"
TTYD_BIN="$TMP_DIR/ttyd"

if ! wget -q "$TTYD_URL" -O "$TTYD_BIN" 2>/dev/null; then
    echo -e "${RED}❌ Failed to download ttyd${NC}"
    exit 1
fi

chmod +x "$TTYD_BIN"

# Install binaries
echo "🔧 Installing binaries..."

# Determine install location
if [ -w "/usr/local/bin" ]; then
    INSTALL_DIR="/usr/local/bin"
    INSTALL_CMD="cp"
else
    INSTALL_DIR="$HOME/.local/bin"
    INSTALL_CMD="cp"
    mkdir -p "$INSTALL_DIR"
fi

# Install VHS
VHS_BIN="$TMP_DIR/vhs_extract/usr/bin/vhs"
if [ -f "$VHS_BIN" ]; then
    $INSTALL_CMD "$VHS_BIN" "$INSTALL_DIR/vhs"
    chmod +x "$INSTALL_DIR/vhs"
    echo -e "${GREEN}✅ VHS installed to $INSTALL_DIR/vhs${NC}"
else
    echo -e "${RED}❌ VHS binary not found in package${NC}"
    exit 1
fi

# Install ttyd
$INSTALL_CMD "$TTYD_BIN" "$INSTALL_DIR/ttyd"
chmod +x "$INSTALL_DIR/ttyd"
echo -e "${GREEN}✅ ttyd installed to $INSTALL_DIR/ttyd${NC}"

# Update PATH if necessary
if [ "$INSTALL_DIR" = "$HOME/.local/bin" ]; then
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        echo -e "${YELLOW}⚠️  Please add $INSTALL_DIR to your PATH:${NC}"
        echo "   export PATH=\"$INSTALL_DIR:\$PATH\""
        echo ""
        echo "Or add this to your ~/.bashrc or ~/.zshrc:"
        echo "   echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.bashrc"
    fi
fi

# Verify installation
echo ""
echo "🔍 Verifying installation..."

if command -v vhs &> /dev/null; then
    VHS_VER=$(vhs version 2>&1 | head -1)
    echo -e "${GREEN}✅ VHS: $VHS_VER${NC}"
else
    echo -e "${RED}❌ VHS not found in PATH${NC}"
    exit 1
fi

if command -v ttyd &> /dev/null; then
    TTYD_VER=$(ttyd --version 2>&1)
    echo -e "${GREEN}✅ ttyd: $TTYD_VER${NC}"
else
    echo -e "${RED}❌ ttyd not found in PATH${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}🎉 VHS setup complete!${NC}"
echo ""
echo "Next steps:"
echo "  1. Create your first tape: vhs new my-test.tape"
echo "  2. Run a tape: vhs my-test.tape"
echo "  3. Check the generated GIF"
echo ""
echo "For Joshify testing:"
echo "  ./scripts/capture-screenshots.sh"
