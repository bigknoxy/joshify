#!/bin/bash
# Capture Screenshots Script
# Generates reference screenshots for VHS visual testing
# Usage: ./scripts/capture-screenshots.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}📸 Joshify Screenshot Capture${NC}"
echo ""

# Check if VHS is installed
if ! command -v vhs &> /dev/null; then
    echo -e "${YELLOW}⚠️  VHS not found. Running setup...${NC}"
    "$SCRIPT_DIR/vhs-setup.sh"
fi

# Verify VHS is now available
if ! command -v vhs &> /dev/null; then
    echo -e "${RED}❌ VHS installation failed${NC}"
    echo "Please install manually: https://github.com/charmbracelet/vhs"
    exit 1
fi

echo -e "${GREEN}✅ VHS found: $(vhs version 2>&1 | head -1)${NC}"
echo ""

# Create output directory
mkdir -p "$PROJECT_ROOT/screenshots/current"

# Get list of tape files
TAPE_DIR="$PROJECT_ROOT/tapes"
if [ ! -d "$TAPE_DIR" ]; then
    echo -e "${RED}❌ Tapes directory not found: $TAPE_DIR${NC}"
    exit 1
fi

# Count tape files
TAPE_COUNT=$(find "$TAPE_DIR" -name "*.tape" | wc -l)
if [ "$TAPE_COUNT" -eq 0 ]; then
    echo -e "${YELLOW}⚠️  No tape files found in $TAPE_DIR${NC}"
    exit 0
fi

echo -e "${BLUE}🎬 Found $TAPE_COUNT tape files${NC}"
echo ""

# Build release binary first
echo -e "${BLUE}🔨 Building release binary...${NC}"
cd "$PROJECT_ROOT"
if ! cargo build --release 2>&1 | tail -20; then
    echo -e "${RED}❌ Build failed${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Build complete${NC}"
echo ""

# Run each tape file
SUCCESS_COUNT=0
FAILED_COUNT=0

for tape in "$TAPE_DIR"/*.tape; do
    if [ -f "$tape" ]; then
        TAPE_NAME=$(basename "$tape")
        echo -e "${BLUE}▶️  Running: $TAPE_NAME${NC}"

        if vhs "$tape" 2>&1; then
            echo -e "${GREEN}✅ Success: $TAPE_NAME${NC}"
            SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
        else
            echo -e "${RED}❌ Failed: $TAPE_NAME${NC}"
            FAILED_COUNT=$((FAILED_COUNT + 1))
        fi
        echo ""
    fi
done

# Summary
echo ""
echo -e "${BLUE}═══════════════════════════════════════${NC}"
echo -e "${BLUE}📊 Summary${NC}"
echo -e "${BLUE}═══════════════════════════════════════${NC}"
echo -e "${GREEN}✅ Successful: $SUCCESS_COUNT${NC}"
echo -e "${RED}❌ Failed: $FAILED_COUNT${NC}"
echo ""

# List generated screenshots
echo -e "${BLUE}📁 Generated screenshots:${NC}"
PNG_COUNT=0
GIF_COUNT=0
if [ -d "$PROJECT_ROOT/screenshots" ]; then
    while IFS= read -r file; do
        if [ -f "$file" ]; then
            SIZE=$(du -h "$file" | cut -f1)
            echo "   $(basename "$file") ($SIZE)"
            if [[ "$file" == *.png ]]; then
                PNG_COUNT=$((PNG_COUNT + 1))
            elif [[ "$file" == *.gif ]]; then
                GIF_COUNT=$((GIF_COUNT + 1))
            fi
        fi
    done < <(find "$PROJECT_ROOT/screenshots" \( -name "*.png" -o -name "*.gif" \) 2>/dev/null)
fi

if [ $PNG_COUNT -eq 0 ] && [ $GIF_COUNT -eq 0 ]; then
    echo "   No screenshots found"
else
    echo ""
    echo "   Total: $PNG_COUNT PNG files, $GIF_COUNT GIF files"
fi

echo ""
if [ $FAILED_COUNT -eq 0 ]; then
    echo -e "${GREEN}🎉 All screenshots captured successfully!${NC}"
    exit 0
else
    echo -e "${YELLOW}⚠️  Some tapes had issues. Check output above.${NC}"
    exit 1
fi
