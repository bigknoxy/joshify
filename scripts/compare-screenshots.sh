#!/bin/bash
# Compare Screenshots Script
# Compares reference screenshots with current screenshots for visual regression testing
# Usage: ./scripts/compare-screenshots.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

REF_DIR="$PROJECT_ROOT/screenshots/reference"
CUR_DIR="$PROJECT_ROOT/screenshots/current"
DIFF_DIR="$PROJECT_ROOT/screenshots/diffs"

# Ensure directories exist
mkdir -p "$DIFF_DIR"

echo -e "${BLUE}🔍 Joshify Visual Regression Comparison${NC}"
echo ""

# Check for ImageMagick
if ! command -v compare &> /dev/null; then
    echo -e "${YELLOW}⚠️  ImageMagick 'compare' not found${NC}"
    echo "   Install with: sudo apt-get install imagemagick"
    echo ""
    echo -e "${BLUE}Falling back to file existence check only${NC}"
    FALLBACK_MODE=true
else
    FALLBACK_MODE=false
fi

# Count reference screenshots
if [ ! -d "$REF_DIR" ]; then
    echo -e "${YELLOW}⚠️  Reference directory not found: $REF_DIR${NC}"
    echo "   Run: ./scripts/capture-screenshots.sh"
    echo "   Then: cp screenshots/current/*.png screenshots/reference/"
    exit 1
fi

REF_COUNT=$(find "$REF_DIR" -name "*.png" | wc -l)
if [ "$REF_COUNT" -eq 0 ]; then
    echo -e "${YELLOW}⚠️  No reference screenshots found${NC}"
    echo "   Run capture-screenshots.sh first, then copy to reference/"
    exit 1
fi

echo -e "${BLUE}📊 Reference screenshots: $REF_COUNT${NC}"
echo ""

# Track results
MATCH_COUNT=0
DIFF_COUNT=0
MISSING_COUNT=0

# Compare each reference screenshot
for ref in "$REF_DIR"/*.png; do
    if [ -f "$ref" ]; then
        BASENAME=$(basename "$ref")
        cur="$CUR_DIR/$BASENAME"
        diff="$DIFF_DIR/$BASENAME"

        if [ ! -f "$cur" ]; then
            echo -e "${RED}❌ MISSING: $BASENAME${NC}"
            MISSING_COUNT=$((MISSING_COUNT + 1))
            continue
        fi

        if [ "$FALLBACK_MODE" = true ]; then
            # Fallback: just check if files are identical
            if cmp -s "$ref" "$cur"; then
                echo -e "${GREEN}✅ IDENTICAL: $BASENAME${NC}"
                MATCH_COUNT=$((MATCH_COUNT + 1))
            else
                echo -e "${YELLOW}⚠️  DIFFERENT: $BASENAME${NC}"
                DIFF_COUNT=$((DIFF_COUNT + 1))
            fi
        else
            # Use ImageMagick compare - single call, capture metric
            # compare returns non-zero if images differ, so don't use if
            DIFF_PIXELS=$(compare -metric AE "$ref" "$cur" "$diff" 2>&1 || true)
            
            if [ -z "$DIFF_PIXELS" ] || [ "$DIFF_PIXELS" = "0" ]; then
                echo -e "${GREEN}✅ MATCH: $BASENAME${NC}"
                MATCH_COUNT=$((MATCH_COUNT + 1))
                rm -f "$diff"  # Remove empty diff
            else
                echo -e "${YELLOW}⚠️  DIFF: $BASENAME ($DIFF_PIXELS pixels different)${NC}"
                DIFF_COUNT=$((DIFF_COUNT + 1))
            fi
        fi
    fi
done

# Summary
echo ""
echo -e "${BLUE}═══════════════════════════════════════${NC}"
echo -e "${BLUE}📊 Visual Regression Summary${NC}"
echo -e "${BLUE}═══════════════════════════════════════${NC}"
echo -e "${GREEN}✅ Matching: $MATCH_COUNT${NC}"
echo -e "${YELLOW}⚠️  Different: $DIFF_COUNT${NC}"
echo -e "${RED}❌ Missing: $MISSING_COUNT${NC}"
echo ""

if [ $DIFF_COUNT -gt 0 ] && [ "$FALLBACK_MODE" = false ]; then
    echo -e "${BLUE}📁 Differences saved to: $DIFF_DIR${NC}"
fi

# Exit code
if [ $DIFF_COUNT -eq 0 ] && [ $MISSING_COUNT -eq 0 ]; then
    echo -e "${GREEN}🎉 All screenshots match!${NC}"
    exit 0
else
    echo -e "${YELLOW}⚠️  Visual differences detected${NC}"
    exit 1
fi
