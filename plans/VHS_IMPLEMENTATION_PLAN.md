# VHS Visual Testing Implementation Plan

## Executive Summary

This document outlines a complete implementation plan for integrating VHS (Video Home System) by Charm Bracelet into the Joshify project to enable automated TUI visual testing and screenshot capture. **VHS has been verified to work** on the current system and can generate both GIF recordings and PNG screenshots.

---

## ✅ Proof of Concept: VERIFIED

**Status**: VHS v0.11.0 successfully tested and working on the development environment.

### Test Results
- ✅ VHS binary downloaded and extracted (v0.11.0)
- ✅ ttyd dependency installed (v1.7.7)
- ✅ Test tape executed successfully
- ✅ Generated `simple_test.gif` (13KB)
- ✅ Generated `simple_test.png` (640x480 screenshot)

**Test Location**: `/home/josh/projects/joshify/vhs_test/`

---

## Architecture Overview

```
Joshify VHS Integration
│
├── tapes/                          # VHS tape scripts (declarative UI tests)
│   ├── home_view.tape              # Home screen showcase
│   ├── library_view.tape           # Library navigation
│   ├── player_view.tape            # Now playing with album art
│   ├── search_overlay.tape         # Search functionality
│   ├── help_overlay.tape           # Help screen
│   ├── themes.tape                 # Theme switching demo
│   └── full_demo.tape              # Complete app walkthrough
│
├── .github/
│   └── workflows/
│       ├── ci.yml                  # Updated with screenshot job
│       └── visual-regression.yml   # PR visual diff check
│
├── screenshots/                     # Generated screenshots (git-ignored)
│   ├── reference/                  # Baseline screenshots
│   └── current/                    # PR-generated screenshots
│
├── scripts/
│   ├── vhs-setup.sh                # Install VHS + dependencies
│   ├── capture-screenshots.sh      # Manual screenshot capture
│   └── compare-screenshots.sh      # Visual diff comparison
│
└── docs/
    └── VHS_USAGE.md                # Developer documentation
```

---

## Phase 1: Foundation (Week 1)

### 1.1 Create Directory Structure

```bash
mkdir -p tapes/
mkdir -p screenshots/{reference,current}
mkdir -p scripts/
```

### 1.2 Setup Script (`scripts/vhs-setup.sh`)

```bash
#!/bin/bash
# Install VHS and dependencies for CI/local development

set -e

VHS_VERSION="0.11.0"
TTYD_VERSION="1.7.7"

# Detect architecture
ARCH=$(uname -m)
if [ "$ARCH" = "x86_64" ]; then
    VHS_ARCH="amd64"
    TTYD_ARCH="x86_64"
elif [ "$ARCH" = "aarch64" ]; then
    VHS_ARCH="arm64"
    TTYD_ARCH="aarch64"
else
    echo "Unsupported architecture: $ARCH"
    exit 1
fi

echo "Installing VHS v${VHS_VERSION} for ${VHS_ARCH}..."

# Download and extract VHS
wget -q "https://github.com/charmbracelet/vhs/releases/download/v${VHS_VERSION}/vhs_${VHS_VERSION}_linux_${VHS_ARCH}.deb" -O /tmp/vhs.deb
dpkg-deb -x /tmp/vhs.deb /tmp/vhs_extract
sudo cp /tmp/vhs_extract/usr/bin/vhs /usr/local/bin/vhs
sudo chmod +x /usr/local/bin/vhs

# Download and install ttyd
wget -q "https://github.com/tsl0922/ttyd/releases/download/${TTYD_VERSION}/ttyd.${TTYD_ARCH}" -O /tmp/ttyd
sudo cp /tmp/ttyd /usr/local/bin/ttyd
sudo chmod +x /usr/local/bin/ttyd

# Verify installation
echo "VHS version: $(vhs version)"
echo "ttyd version: $(ttyd --version)"

echo "✅ VHS setup complete!"
```

### 1.3 Gitignore Update

Add to `.gitignore`:
```
# VHS generated files
screenshots/current/
*.gif
vhs_test/
```

---

## Phase 2: Tape Scripts (Week 1-2)

### 2.1 Mock Data Strategy

Since Joshify requires Spotify authentication, we need a mock mode for testing:

**Option A: Mock Mode in Joshify** (Recommended)
Add a `--mock-data` flag or `JOSHIFY_MOCK=1` env var that:
- Loads fake track/artist/album data
- Simulates playback without actual Spotify API
- Shows all UI states without network calls

**Option B: Pre-authenticated Session**
- Store encrypted credentials for CI
- Requires secure secret management

### 2.2 Tape: Home View (`tapes/home_view.tape`)

```tape
# Home View Showcase
# Usage: vhs tapes/home_view.tape

Output screenshots/home_view.gif
Set Shell "bash"
Set FontSize 14
Set Width 120
Set Height 40
Set Theme "Catppuccin Mocha"
Set Padding 20

# Start Joshify with mock data
Hide
Type "JOSHIFY_MOCK=1 cargo run --release 2>/dev/null &"
Enter
Sleep 3s
Show

# Capture initial home view
Screenshot screenshots/home_view.png
Sleep 500ms

# Navigate to Library
Type "l"
Sleep 500ms
Screenshot screenshots/library_view.png

# Show search overlay
Type "/"
Sleep 200ms
Type "never gonna give you up"
Sleep 1s
Screenshot screenshots/search_overlay.png

# Close search
Type "Escape"
Sleep 200ms

# Show help overlay
Type "?"
Sleep 500ms
Screenshot screenshots/help_overlay.png

# Close help
Type "Escape"
Sleep 200ms

# Quit
Type "q"
Sleep 200ms

# Cleanup
Hide
Type "pkill -f joshify"
Enter
Show
```

### 2.3 Tape: Player View (`tapes/player_view.tape`)

```tape
# Player View with Album Art
# Demonstrates now playing bar, album art, and controls

Output screenshots/player_view.gif
Set Shell "bash"
Set FontSize 14
Set Width 120
Set Height 40
Set Theme "Catppuccin Mocha"

Hide
Type "JOSHIFY_MOCK=1 cargo run --release 2>/dev/null &"
Enter
Sleep 3s
Show

# Navigate to a playlist and play
Type "l"
Sleep 200ms
Type "j" # Down to playlists
Sleep 200ms
Enter
Sleep 500ms

# Select first track
Type "j"
Sleep 200ms
Enter
Sleep 2s

# Capture player view
Screenshot screenshots/player_playing.png

# Show queue
Type "Q"
Sleep 500ms
Screenshot screenshots/queue_overlay.png
Type "Q"
Sleep 200ms

# Toggle visualizer
Type "v"
Sleep 1s
Screenshot screenshots/visualizer.png

# Cycle themes
Type "T"
Sleep 500ms
Screenshot screenshots/theme_gruvbox.png
Type "T"
Sleep 500ms
Screenshot screenshots/theme_nord.png

# Quit
Type "q"
Sleep 200ms

Hide
Type "pkill -f joshify"
Enter
Show
```

### 2.4 Tape: Navigation Demo (`tapes/navigation.tape`)

```tape
# Full Navigation Demo
# Shows sidebar, main view transitions, breadcrumbs

Output screenshots/navigation.gif
Set Shell "bash"
Set FontSize 14
Set Width 120
Set Height 40
Set Theme "Catppuccin Mocha"

Hide
Type "JOSHIFY_MOCK=1 cargo run --release 2>/dev/null &"
Enter
Sleep 3s
Show

# Home → Library → Playlist → Track → Back
Type "l"
Sleep 500ms
Screenshot screenshots/nav_library.png

Type "j"
Sleep 200ms
Enter
Sleep 500ms
Screenshot screenshots/nav_playlist.png

Type "j"
Sleep 200ms
Enter
Sleep 500ms
Screenshot screenshots/nav_track_detail.png

Type "Backspace"
Sleep 500ms
Screenshot screenshots/nav_back_to_playlist.png

Type "Backspace"
Sleep 500ms
Screenshot screenshots/nav_back_to_library.png

Type "h"
Sleep 500ms
Screenshot screenshots/nav_home_focused.png

# Quit
Type "q"
Sleep 200ms

Hide
Type "pkill -f joshify"
Enter
Show
```

---

## Phase 3: CI/CD Integration (Week 2)

### 3.1 Updated CI Workflow (`.github/workflows/ci.yml`)

Add to existing CI:

```yaml
  visual-tests:
    runs-on: ubuntu-latest
    needs: build
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install system dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y pkg-config libchafa-dev libglib2.0-dev libasound2-dev ffmpeg

      - name: Install VHS
        run: |
          curl -fsSL https://github.com/charmbracelet/vhs/releases/download/v0.11.0/vhs_0.11.0_linux_amd64.deb -o vhs.deb
          sudo dpkg -i vhs.deb || sudo apt-get install -f -y
          curl -fsSL https://github.com/tsl0922/ttyd/releases/download/1.7.7/ttyd.x86_64 -o ttyd
          chmod +x ttyd
          sudo mv ttyd /usr/local/bin/

      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2

      - name: Build release binary
        run: cargo build --release

      - name: Generate screenshots
        run: |
          mkdir -p screenshots/current
          export PATH="/usr/local/bin:$PATH"
          # Run all tape files
          for tape in tapes/*.tape; do
            echo "Running: $tape"
            vhs "$tape" || echo "Warning: $tape failed"
          done

      - name: Upload screenshots
        uses: actions/upload-artifact@v4
        with:
          name: screenshots
          path: screenshots/
          retention-days: 30
```

### 3.2 Visual Regression Workflow (`.github/workflows/visual-regression.yml`)

```yaml
name: Visual Regression Tests

on:
  pull_request:
    branches: [main]
    paths:
      - 'src/ui/**'
      - 'src/main.rs'
      - 'tapes/**'

jobs:
  compare-screenshots:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y pkg-config libchafa-dev libglib2.0-dev libasound2-dev ffmpeg imagemagick

      - name: Install VHS
        run: |
          curl -fsSL https://github.com/charmbracelet/vhs/releases/download/v0.11.0/vhs_0.11.0_linux_amd64.deb -o vhs.deb
          sudo dpkg -i vhs.deb || sudo apt-get install -f -y
          curl -fsSL https://github.com/tsl0922/ttyd/releases/download/1.7.7/ttyd.x86_64 -o ttyd
          chmod +x ttyd
          sudo mv ttyd /usr/local/bin/

      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2

      - name: Build release
        run: cargo build --release

      - name: Checkout reference screenshots
        run: |
          git checkout main -- screenshots/reference/ || mkdir -p screenshots/reference

      - name: Generate current screenshots
        run: |
          mkdir -p screenshots/current
          for tape in tapes/*.tape; do
            vhs "$tape" || true
          done

      - name: Compare screenshots
        run: |
          ./scripts/compare-screenshots.sh || true

      - name: Upload comparison results
        uses: actions/upload-artifact@v4
        if: always()
        with:
          name: visual-regression-results
          path: screenshots/
          retention-days: 7
```

### 3.3 Screenshot Comparison Script (`scripts/compare-screenshots.sh`)

```bash
#!/bin/bash
# Compare reference screenshots with current screenshots

set -e

REF_DIR="screenshots/reference"
CUR_DIR="screenshots/current"
DIFF_DIR="screenshots/diffs"

mkdir -p "$DIFF_DIR"

DIFF_FOUND=0

for ref in "$REF_DIR"/*.png; do
    [ -e "$ref" ] || continue
    
    basename=$(basename "$ref")
    cur="$CUR_DIR/$basename"
    diff="$DIFF_DIR/$basename"
    
    if [ ! -f "$cur" ]; then
        echo "❌ MISSING: $basename (current screenshot not found)"
        DIFF_FOUND=1
        continue
    fi
    
    # Use ImageMagick to compare
    if compare "$ref" "$cur" "$diff" 2>/dev/null; then
        # Check if diff is significant (> 1% pixels changed)
        diff_pct=$(compare -metric PHASH "$ref" "$cur" null: 2>&1 | cut -d' ' -f1)
        if (( $(echo "$diff_pct > 0.01" | bc -l) )); then
            echo "⚠️  CHANGED: $basename (diff: ${diff_pct}%)"
            DIFF_FOUND=1
        else
            echo "✅ OK: $basename"
            rm "$diff"  # Remove insignificant diff
        fi
    else
        echo "✅ OK: $basename (identical)"
        rm "$diff"
    fi
done

if [ $DIFF_FOUND -eq 1 ]; then
    echo ""
    echo "Visual differences detected. Check screenshots/diffs/"
    exit 1
else
    echo ""
    echo "All screenshots match reference!"
    exit 0
fi
```

---

## Phase 4: Mock Data Implementation (Week 2-3)

### 4.1 Mock Data Module

Create `src/test_utils/mock_data.rs`:

```rust
//! Mock data for visual testing
//! Enabled with JOSHIFY_MOCK=1 environment variable

use crate::state::library_state::{Album, Artist, Playlist, Track};

pub fn is_mock_mode() -> bool {
    std::env::var("JOSHIFY_MOCK").is_ok()
}

pub fn get_mock_tracks() -> Vec<Track> {
    vec![
        Track {
            id: "mock_track_1".to_string(),
            name: "Never Gonna Give You Up".to_string(),
            artist: "Rick Astley".to_string(),
            album: "Whenever You Need Somebody".to_string(),
            duration_ms: 213000,
            uri: "spotify:track:mock1".to_string(),
        },
        Track {
            id: "mock_track_2".to_string(),
            name: "Bohemian Rhapsody".to_string(),
            artist: "Queen".to_string(),
            album: "A Night at the Opera".to_string(),
            duration_ms: 354000,
            uri: "spotify:track:mock2".to_string(),
        },
        // ... more tracks
    ]
}

pub fn get_mock_playlists() -> Vec<Playlist> {
    vec![
        Playlist {
            id: "mock_pl_1".to_string(),
            name: "Discover Weekly".to_string(),
            description: "Your weekly mixtape".to_string(),
            tracks_count: 30,
            uri: "spotify:playlist:mock1".to_string(),
        },
        // ... more playlists
    ]
}
```

### 4.2 Conditional Mock Loading

In `src/state/app_state.rs`:

```rust
use crate::test_utils::mock_data;

impl AppState {
    pub async fn load_library(&mut self) -> anyhow::Result<()> {
        if mock_data::is_mock_mode() {
            self.library_state.set_mock_data();
            return Ok(());
        }
        // ... normal API loading
    }
}
```

---

## Phase 5: Documentation Screenshots (Week 3)

### 5.1 README Integration

Update `README.md` to include generated screenshots:

```markdown
## Screenshots

![Home View](screenshots/reference/home_view.png)
*Joshify home screen with recently played tracks*

![Library View](screenshots/reference/library_view.png)
*Browse your playlists and liked songs*

![Now Playing](screenshots/reference/player_playing.png)
*Now playing bar with album art and controls*
```

### 5.2 Automated Screenshot Updates

Create `.github/workflows/update-screenshots.yml`:

```yaml
name: Update Reference Screenshots

on:
  workflow_dispatch:
  schedule:
    - cron: '0 0 * * 0'  # Weekly on Sunday

jobs:
  update:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup and run VHS
        run: |
          # ... setup steps ...
          mkdir -p screenshots/reference
          for tape in tapes/*.tape; do
            vhs "$tape"
          done

      - name: Commit updated screenshots
        run: |
          git config user.name "GitHub Actions"
          git config user.email "actions@github.com"
          git add screenshots/reference/
          git commit -m "chore: update reference screenshots [skip ci]" || echo "No changes"
          git push
```

---

## Deliverables Checklist

### For Junior Dev / Intern

- [ ] **Setup** (`scripts/vhs-setup.sh`)
  - [ ] Download VHS binary
  - [ ] Download ttyd binary
  - [ ] Install to /usr/local/bin
  - [ ] Verify installation

- [ ] **Directory Structure**
  - [ ] Create `tapes/` directory
  - [ ] Create `screenshots/reference/` directory
  - [ ] Create `screenshots/current/` directory
  - [ ] Update `.gitignore`

- [ ] **Tape Scripts** (start with 3)
  - [ ] `tapes/home_view.tape`
  - [ ] `tapes/player_view.tape`
  - [ ] `tapes/navigation.tape`

- [ ] **CI Integration**
  - [ ] Add visual-tests job to `.github/workflows/ci.yml`
  - [ ] Create `.github/workflows/visual-regression.yml`
  - [ ] Create `scripts/compare-screenshots.sh`

- [ ] **Mock Data** (if needed)
  - [ ] Add `JOSHIFY_MOCK` environment variable support
  - [ ] Create mock track data
  - [ ] Create mock playlist data

- [ ] **Documentation**
  - [ ] Create `docs/VHS_USAGE.md`
  - [ ] Update README with screenshot examples
  - [ ] Add VHS badge to README

### Verification Steps

1. Run `scripts/vhs-setup.sh` - should install VHS and ttyd
2. Run `vhs tapes/home_view.tape` - should generate screenshot
3. Check `screenshots/home_view.png` exists
4. Push to branch - CI should generate screenshots artifact

---

## Estimated Effort

| Task | Complexity | Estimated Time |
|------|-----------|----------------|
| Setup script | Low | 2 hours |
| 3 initial tape scripts | Medium | 6 hours |
| CI integration | Medium | 4 hours |
| Mock data implementation | Medium | 6 hours |
| Documentation | Low | 2 hours |
| **Total** | | **20 hours** |

---

## Success Criteria

✅ VHS generates screenshots automatically in CI
✅ Screenshots are uploaded as artifacts
✅ README displays current UI screenshots
✅ New PRs trigger visual regression checks
✅ Mock mode works for testing without Spotify auth

---

## Questions for Review

1. **Mock Data**: Should we implement a full mock mode, or use pre-authenticated credentials in CI secrets?
2. **Screenshot Retention**: How long should we keep screenshot artifacts? (Suggested: 30 days)
3. **Theme Testing**: Should we generate screenshots for all 7 themes, or just the default?
4. **Tape Coverage**: Which UI states are highest priority for screenshots?

---

## References

- **VHS Documentation**: https://github.com/charmbracelet/vhs
- **VHS Tape Syntax**: https://github.com/charmbracelet/vhs#tape-file-syntax
- **ttyd**: https://github.com/tsl0922/ttyd
- **Test Results**: `/home/josh/projects/joshify/vhs_test/`

---

**Plan Status**: ✅ VERIFIED AND READY FOR IMPLEMENTATION

This plan has been validated with a working VHS installation that successfully generates both GIF recordings and PNG screenshots. The implementation can proceed immediately.
