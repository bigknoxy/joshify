# VHS Visual Testing Guide

This guide explains how to use VHS (Video Home System) for automated visual testing of Joshify's TUI.

## Overview

VHS allows us to:
- **Capture screenshots** of Joshify UI states
- **Record GIF demos** for documentation
- **Detect visual regressions** in PRs
- **Test without Spotify auth** using mock data

## Quick Start

### 1. Install VHS

```bash
./scripts/vhs-setup.sh
```

This installs:
- VHS v0.11.0 - Terminal recorder
- ttyd v1.7.7 - Terminal daemon (required by VHS)

### 2. Run Tests

```bash
# Generate all screenshots
./scripts/capture-screenshots.sh

# Check what was generated
ls screenshots/
```

### 3. Compare Screenshots (Visual Regression)

```bash
# First, establish reference screenshots
cp screenshots/current/*.png screenshots/reference/

# Later, after making changes
./scripts/capture-screenshots.sh
./scripts/compare-screenshots.sh
```

## Tape Scripts

Tape scripts are declarative recordings of terminal sessions.

### Available Tapes

| Tape | Description |
|------|-------------|
| `home_view.tape` | Home screen, navigation, overlays |
| `player_view.tape` | Now playing bar, themes, visualizer |
| `navigation.tape` | Drill-down navigation, breadcrumbs |

### Running Individual Tapes

```bash
# Run a specific tape
vhs tapes/home_view.tape

# Validate a tape without running
vhs validate tapes/home_view.tape
```

### Creating New Tapes

```bash
# Use VHS to record interactions
vhs record > my-test.tape

# Or create manually
vhs new my-test.tape
```

## Tape Syntax Reference

```tape
# Output file
Output demo.gif

# Terminal settings
Set FontSize 14
Set Width 120
Set Height 40
Set Theme "Catppuccin Mocha"

# Commands
Type "echo Hello"
Enter
Sleep 1s

# Screenshots
Screenshot my-screenshot.png

# Hide commands from output
Hide
Type "secret-command"
Enter
Show
```

## Mock Data Mode

VHS tests use mock data instead of real Spotify authentication.

### Enable Mock Mode

```bash
# Via environment variable
JOSHIFY_MOCK=1 cargo run

# Or in tape scripts
Hide
Type "JOSHIFY_MOCK=1 cargo run --release 2>&1 &"
Enter
Sleep 3s
Show
```

### Mock Data Features

- Fake tracks, playlists, albums, and artists
- Simulated playback state ("Now Playing")
- No network calls to Spotify
- Consistent data for reliable screenshots

### Customizing Mock Data

Edit `src/state/mock_data.rs`:

```rust
pub fn get_mock_tracks() -> Vec<TrackListItem> {
    vec![
        TrackListItem {
            name: "Your Track".to_string(),
            artist: "Your Artist".to_string(),
            uri: "spotify:track:custom".to_string(),
        },
        // ... more tracks
    ]
}
```

## CI/CD Integration

### Visual Tests Workflow

Runs on every push/PR to `main`:

```yaml
# .github/workflows/visual-tests.yml
- Generates screenshots from tape files
- Uploads artifacts
- Updates reference screenshots on main
```

### Visual Regression Workflow

Runs on PRs that modify UI code:

```yaml
# .github/workflows/visual-regression.yml
- Generates current screenshots
- Compares with reference screenshots
- Comments on PR if differences detected
```

### Viewing Results

1. Go to the Actions tab in GitHub
2. Click on the workflow run
3. Download the `screenshots` artifact
4. Check `screenshots/diffs/` for any changes

## Directory Structure

```
joshify/
├── tapes/                    # VHS tape scripts
├── screenshots/
│   ├── reference/           # Baseline screenshots
│   ├── current/             # Latest screenshots
│   └── diffs/               # Comparison diffs
├── scripts/
│   ├── vhs-setup.sh         # Install VHS
│   ├── capture-screenshots.sh
│   └── compare-screenshots.sh
└── docs/
    └── VHS_USAGE.md         # This file
```

## Troubleshooting

### VHS not found

```bash
# Ensure VHS is in PATH
which vhs

# If not found, run setup
./scripts/vhs-setup.sh

# Or manually add to PATH
export PATH="$HOME/.local/bin:$PATH"
```

### ttyd not found

```bash
# Check if ttyd is installed
which ttyd

# Install manually
wget https://github.com/tsl0922/ttyd/releases/download/1.7.7/ttyd.x86_64
chmod +x ttyd
sudo mv ttyd /usr/local/bin/
```

### Screenshots not generating

Check:
1. Is the binary built? `cargo build --release`
2. Is mock mode enabled? `JOSHIFY_MOCK=1`
3. Check tape syntax: `vhs validate my.tape`
4. Check VHS output for errors

### Permission denied

```bash
# Make scripts executable
chmod +x scripts/*.sh
```

## Best Practices

1. **Use Mock Mode**: Always use `JOSHIFY_MOCK=1` in tape scripts
2. **Consistent Timing**: Use `Sleep 500ms` or `Sleep 1s` for stability
3. **Screenshot Naming**: Use descriptive names like `home_view.png`
4. **Terminal Size**: Keep `Width 120` and `Height 40` for consistency
5. **Theme**: Use `Catppuccin Mocha` to match Joshify's default theme
6. **Cleanup**: Always kill the process at the end of tapes

## Advanced Usage

### Custom Themes in Tapes

```tape
# Available themes: Catppuccin Mocha, Gruvbox, Nord, Tokyo Night, Dracula
Set Theme "Tokyo Night"
```

### Wait for Conditions

```tape
# Wait for text to appear
Wait+Screen /Hello/

# Wait with timeout
Wait+Screen@10s /Ready/
```

### Multiple Outputs

```tape
# Generate both GIF and PNG
Output demo.gif
Screenshot demo.png
```

## Resources

- [VHS Documentation](https://github.com/charmbracelet/vhs)
- [VHS Tape Syntax](https://github.com/charmbracelet/vhs#tape-file-syntax)
- [ttyd Documentation](https://github.com/tsl0922/ttyd)
- [Joshify UI Components](../src/ui/)

## Contributing

When adding new UI features:

1. Add mock data if needed (`src/state/mock_data.rs`)
2. Create a tape script (`tapes/my-feature.tape`)
3. Test locally: `./scripts/capture-screenshots.sh`
4. Commit tape file and reference screenshots
5. Update this documentation
