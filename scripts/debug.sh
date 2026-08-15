#!/bin/bash
# Debug script to test Spotify API directly

echo "=== Joshify Debug Script ==="
echo ""

# Check for curl
if ! command -v curl &> /dev/null; then
    echo "❌ curl not found. Install with: sudo apt install curl"
    exit 1
fi

# Get token from credentials file (try multiple locations)
CREDS_FILES=(
    "$HOME/.config/joshify/credentials.json"
    "$HOME/.cache/spotify-player/credentials.json"
    "$HOME/.config/spotify-player/credentials.json"
)

CREDS_FILE=""
for file in "${CREDS_FILES[@]}"; do
    if [ -f "$file" ]; then
        CREDS_FILE="$file"
        break
    fi
done

if [ -z "$CREDS_FILE" ]; then
    echo "❌ Credentials not found in any of:"
    for file in "${CREDS_FILES[@]}"; do
        echo "   $file"
    done
    echo ""
    echo "💡 SOLUTION: Run 'cargo run' to authenticate first"
    echo "   This will save credentials to ~/.config/joshify/credentials.json"
    exit 1
fi

echo "✓ Found credentials: $CREDS_FILE"

# Try to extract access token (handle different formats)
if grep -q "access_token" "$CREDS_FILE"; then
    # Joshify format
    ACCESS_TOKEN=$(cat "$CREDS_FILE" | grep -o '"access_token"[[:space:]]*:[[:space:]]*"[^"]*"' | cut -d'"' -f4)
elif grep -q "auth_data" "$CREDS_FILE"; then
    # spotify-player format (base64 encoded)
    echo "⚠ Found spotify-player credentials (different format)"
    echo "   Run 'cargo run' with joshify to create proper credentials"
    exit 1
else
    echo "❌ Could not extract access token from $CREDS_FILE"
    echo "   File contents:"
    cat "$CREDS_FILE"
    exit 1
fi

if [ -z "$ACCESS_TOKEN" ]; then
    echo "❌ Could not extract access token"
    exit 1
fi

echo "✓ Loaded access token (first 10 chars): ${ACCESS_TOKEN:0:10}..."
echo ""

# Test 1: Get devices
echo "=== Test 1: Available Devices ==="
DEVICES_RESPONSE=$(curl -s -X GET "https://api.spotify.com/v1/me/player/devices" \
    -H "Authorization: Bearer $ACCESS_TOKEN")

echo "Raw response:"
echo "$DEVICES_RESPONSE" | python3 -m json.tool 2>/dev/null || echo "$DEVICES_RESPONSE"
echo ""

DEVICE_COUNT=$(echo "$DEVICES_RESPONSE" | grep -o '"devices"' | wc -l)
if [ "$DEVICE_COUNT" -gt 0 ]; then
    echo "✓ Got devices response"
    ACTIVE=$(echo "$DEVICES_RESPONSE" | grep -o '"is_active"[[:space:]]*:[[:space:]]*true' | wc -l)
    if [ "$ACTIVE" -gt 0 ]; then
        echo "✓ Found active device"
    else
        echo "⚠ No active device (nothing is playing)"
    fi
else
    echo "❌ Failed to get devices"
    echo "   Response: $DEVICES_RESPONSE"
fi
echo ""

# Test 2: Get playback state
echo "=== Test 2: Playback State (/me/player) ==="
PLAYBACK_RESPONSE=$(curl -s -X GET "https://api.spotify.com/v1/me/player" \
    -H "Authorization: Bearer $ACCESS_TOKEN")

echo "Raw response (first 500 chars):"
echo "$PLAYBACK_RESPONSE" | head -c 500
if [ ${#PLAYBACK_RESPONSE} -gt 500 ]; then
    echo ""
    echo "... ($(echo "$PLAYBACK_RESPONSE" | wc -c) total bytes)"
fi
echo ""
echo ""

# Analyze response
echo "=== ANALYSIS ==="
if echo "$PLAYBACK_RESPONSE" | grep -q '"item"'; then
    echo "✓ Contains 'item' field (playback context)"
    if echo "$PLAYBACK_RESPONSE" | grep -q '"item"[[:space:]]*:[[:space:]]*null'; then
        echo "  → item is null (nothing playing)"
    else
        echo "  → item is present (something is playing)"
    fi
elif echo "$PLAYBACK_RESPONSE" | grep -q '"is_active"'; then
    echo "⚠ Contains 'is_active' but NO 'item' field"
    echo "  → This is a DEVICE OBJECT, not playback context"
    echo "  → NORMAL when devices exist but nothing is playing"
    if echo "$PLAYBACK_RESPONSE" | grep -q '"is_active"[[:space:]]*:[[:space:]]*false'; then
        echo "  → is_active: false (confirms nothing playing)"
    fi
    echo ""
    echo "💡 The TUI should show 'Nothing playing' - NOT an error!"
    echo "   This is the root cause of the deserialization error."
else
    echo "? Unknown response format"
    echo "   Full response: $PLAYBACK_RESPONSE"
fi
echo ""

echo "=== Debug Complete ==="
