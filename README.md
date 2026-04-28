# Joshify ⚡

A beautiful terminal Spotify client built with Rust and ratatui.

[![Build Status](https://img.shields.io/github/actions/workflow/status/bigknoxy/joshify/ci.yml?branch=main&style=flat-square)](https://github.com/bigknoxy/joshify/actions/workflows/ci.yml)
[![Tests](https://img.shields.io/badge/tests-335%20passing-brightgreen?style=flat-square)](https://github.com/bigknoxy/joshify)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange?style=flat-square&logo=rust)](https://www.rust-lang.org)

```
      ╭═══════╮
    ╔═╝ ◉   ◉ ╚═╗
    ║ ╭───────╮ ║
    ║ │ ██████ │ ║
    ║ │ ▀▀▀▀▀ │ ║
    ╚═╧ ▼▼▼▼▼ ╧═╝
        ╲▄▄▄▄╄╱
       ╱▓▓▌ ▐▓▓╲
      │▓▓▓▌ ▐▓▓▓│
      │ ║║   ║║ │
      ╰─╯    ╰─╯
     ♪ JOSHIFY ♪
```
     ⚡ JOSHIFY ⚡
    ╱▔▔▔▔▔▔▔▔▔╲
   ╱  ▀▄   ▄▀  ╲
  │   ▄▀▀▀▀▄   │
  │  │ ▀▀ │  │
   ╲  ╲__╱  ╱
    ╲_____╱
```

## Features

- **Full Spotify Integration** - Play any track, browse playlists, access liked songs
- **Local Playback** - Play directly through your computer (no Spotify app needed)
- **Search** - Find any song, artist, or album with fuzzy matching
- **Fuzzy Search** - Typo-tolerant search with relevance scoring
- **Album Art** - Displayed with terminal graphics protocols (kitty, sixel, iTerm2) or ASCII fallback
- **Audio Visualization** - Real-time FFT spectrum visualization (32/64/128 bands)
- **Lyrics Display** - Synced lyrics via LRCLIB API
- **Queue Management** - View and add tracks to queue
- **Themes** - 7 built-in color themes (Catppuccin, Gruvbox, Nord, Tokyo Night, Dracula)
- **Media Controls** - MPRIS integration for OS media key support
- **Desktop Notifications** - Native OS notifications on track change
- **CLI Mode** - Full command-line interface for scripting
- **Daemon Mode** - Background service with IPC control
- **Configuration File** - TOML-based user preferences
- **Keyboard First** - All actions accessible via keyboard shortcuts
- **Minimal Resource Usage** - Runs entirely in your terminal
- **Comprehensive Test Suite** - 335 tests covering all functionality

## Installation

### One-Line Installer (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/bigknoxy/joshify/main/install.sh | bash
```

### Via Cargo

```bash
cargo install joshify
```

### From Source (requires Rust)

```bash
git clone https://github.com/bigknoxy/joshify.git
cd joshify
cargo install --path .
```

### Via npm/bun

```bash
npm install -g joshify  # Coming soon
bun add -g joshify      # Coming soon
```

### Uninstall

```bash
# If installed via shell script
curl -fsSL https://raw.githubusercontent.com/bigknoxy/joshify/main/uninstall.sh | bash

# If installed via cargo
cargo uninstall joshify
```

## Quick Start

### Interactive Mode (default)

1. **Run Joshify**
   ```bash
   joshify
   ```

2. **Authenticate** - The app will open your browser for Spotify OAuth

3. **Navigate**
   - `Tab` / `Shift+Tab` - Switch between sections
   - `↑` / `↓` or `j` / `k` - Navigate lists
   - `h` / `l` - Focus sidebar / main content (vim-style)
   - `Enter` - Play selected track
   - `Backspace` - Go back (browser-like navigation)
   - `/` - Search for music
   - `Tab` (in search) - Add track to queue

4. **Playback Controls**
   - `Space` - Play/Pause
   - `n` - Next track
   - `p` - Previous track
   - `←` / `→` - Seek ±10 seconds
   - `+` / `-` - Volume up/down
   - `s` - Toggle shuffle
   - `r` - Cycle repeat mode

5. **Other Actions**
   - `Q` - Toggle queue view
   - `a` - Add current track to queue
   - `v` - Toggle audio visualization
   - `L` - Show lyrics for current track
   - `T` - Cycle themes
   - `?` - Show help
   - `q` - Quit

### CLI Mode (Non-Interactive)

Joshify provides a full CLI for scripting and automation:

```bash
# Playback control
joshify play                          # Resume playback
joshify play <spotify:track:xxx>      # Play specific track
joshify pause
joshify next
joshify previous
joshify stop

# Status and info
joshify status                        # Human-readable status
joshify status --format json          # JSON output for scripting
joshify current                       # Show current track
joshify current --format minimal      # Just track name

# Volume and seeking
joshify volume                        # Show current volume
joshify volume 75                     # Set volume to 75%
joshify seek 120000                   # Seek to 2 minutes
joshify forward 10000                 # Skip forward 10 seconds
joshify backward 10000                # Skip backward 10 seconds

# Search and queue
joshify search "artist name"          # Search tracks
joshify search "query" --limit 10     # Limit results
joshify queue-add <uri>               # Add track to queue
joshify queue-clear                   # Clear queue

# Shuffle and repeat
joshify shuffle                       # Toggle shuffle
joshify shuffle on                    # Enable shuffle
joshify repeat                        # Cycle repeat mode
joshify repeat track                  # Set repeat mode
```

### Daemon Mode

Run Joshify as a background daemon for headless operation:

```bash
# Start daemon
joshify daemon

# Send commands to daemon
joshify daemon-send play
joshify daemon-send pause
joshify daemon-send next
joshify daemon-send status

# Stop daemon
joshify daemon-send shutdown
```

The daemon communicates via Unix socket at `~/.cache/joshify/daemon.sock` using JSON protocol.

### Non-Interactive Authentication (headless/automated)

For scripted deployments or headless environments, provide credentials via environment variables or CLI flags:

**Environment variables:**
```bash
export SPOTIFY_CLIENT_ID=your_client_id
export SPOTIFY_CLIENT_SECRET=your_client_secret
export SPOTIFY_ACCESS_TOKEN=your_access_token
export SPOTIFY_REFRESH_TOKEN=your_refresh_token  # optional
joshify
```

**CLI flags:**
```bash
joshify --client-id xxx --client-secret yyy --access-token zzz
```

**Mixed (CLI overrides env vars):**
```bash
export SPOTIFY_CLIENT_ID=xxx
export SPOTIFY_CLIENT_SECRET=yyy
joshify --access-token zzz  # use fresh token
```

**Token expiration:**
```bash
export SPOTIFY_TOKEN_EXPIRES_AT=1743552000  # Unix timestamp
```

If no expiration is provided, tokens are assumed valid for 1 hour.

## Configuration

Joshify loads configuration from `~/.config/joshify/config.toml`. The file is automatically created with defaults on first run.

### Example Configuration

```toml
[audio]
visualization = true           # Enable audio visualization
visualization_bands = 64       # Number of bands (32, 64, or 128)
visualization_smoothing = 0.3  # Smoothing factor (0.0 - 1.0)
default_volume = 50            # Default volume (0-100)

[notifications]
enabled = true                 # Show desktop notifications
cooldown_seconds = 5           # Minimum seconds between notifications
show_album_art = true          # Include album art in notifications

[media_control]
enabled = true                 # Enable OS media key support

[ui]
theme = "catppuccin_mocha"     # Color theme
time_format = "elapsed/total"  # Time display format
show_breadcrumbs = true        # Show navigation breadcrumbs
compact_layout = false         # Use compact layout

[keybindings]
# Optional custom keybindings (defaults shown)
# quit = "q"
# search = "/"
# help = "?"
```

### Available Themes

- `catppuccin_mocha` (default) - Dark pastel theme
- `catppuccin_latte` - Light variant
- `gruvbox_dark` - Retro dark theme
- `gruvbox_light` - Retro light theme
- `nord` - Arctic North blue theme
- `tokyo_night` - Dark Tokyo Night theme
- `dracula` - Classic Dracula theme

Press `T` in the app to cycle through themes, or set in your config file.

Credentials are stored in `~/.config/joshify/credentials.json` and automatically saved to your OS keyring when available (GNOME Keyring, macOS Keychain, Windows Credential Manager).

To re-authenticate or change accounts, press `c` in the app.

## System Requirements

- macOS, Linux, or Windows with a terminal emulator
- Spotify Premium account
- Terminal with UTF-8 support
- For best album art: kitty, iTerm2, or sixel-capable terminal
- For audio visualization: Local playback mode

## Development

```bash
# Run in development
cargo run

# Build release
cargo build --release

# Run all tests
cargo test

# Run specific test category
cargo test --test player
cargo test --test auth
cargo test --test api

# Generate coverage report (requires cargo-tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --out Lcov
```

## Architecture

Joshify uses a modular architecture with separated concerns:

```
src/
├── main.rs              # Application entry point, event loop
├── lib.rs               # Library exports
├── auth.rs              # OAuth authentication
├── album_art.rs         # LRU-cached album art fetching
├── keyring_store.rs     # OS keyring integration
├── config.rs            # TOML configuration management
├── cli.rs               # Command-line interface
├── daemon.rs            # Background daemon with IPC
├── logging.rs           # Structured logging with rotation
├── lyrics.rs            # LRCLIB lyrics fetching
├── media_control.rs     # MPRIS media key integration
├── notifications.rs     # Desktop notifications
├── search.rs            # Fuzzy search engine
├── themes.rs            # Theme system (7 themes)
├── api/                 # Spotify API client
│   ├── client.rs        # Client creation & auth
│   ├── playback.rs      # Playback controls
│   ├── library.rs       # Library & search
│   └── rate_limit.rs    # Rate limit handling
├── state/               # Application state
│   ├── app_state.rs     # Main state coordinator
│   ├── player_state.rs  # Playback state
│   ├── load_coordinator.rs  # Async task coordination
│   ├── library_state.rs     # Library cache
│   ├── queue_state.rs       # Queue management
│   ├── home_state.rs        # Home dashboard state
│   └── navigation_stack.rs  # Drill-down navigation
├── player/              # Local playback
│   ├── mod.rs           # Player interface
│   ├── librespot.rs     # Spotify Connect integration
│   └── visualization.rs # FFT spectrum visualization
└── ui/                  # Terminal rendering
    ├── sidebar.rs       # Navigation sidebar
    ├── main_view.rs     # Main content area
    ├── player_bar.rs    # Now playing bar
    ├── overlays.rs      # Search, help, queue
    ├── home_view.rs     # Home dashboard
    ├── help.rs          # Help overlay
    ├── theme.rs         # Theme colors
    └── image_renderer.rs # Terminal image protocols
```

## Tech Stack

- **Rust** - Systems programming language
- **ratatui** - Terminal UI framework
- **rspotify** - Spotify API client
- **tokio** - Async runtime
- **crossterm** - Terminal manipulation
- **librespot** - Spotify Connect local playback
- **realfft** - FFT for audio visualization
- **tracing** - Structured logging
- **serde** - Serialization for config/credentials
- **toml** - Configuration file format
- **lru** - LRU cache for album art (50 entry limit)
- **keyring** - OS keyring integration

## Testing

Joshify has a comprehensive test suite with 335 tests across 12 categories:

| Category | Tests | Description |
|----------|-------|-------------|
| Player | 15 | Duration formatting, PlayerState, track detection, visualization |
| Auth | 12 | Credentials loading, OAuth config, persistence, keyring |
| Album Art | 10 | LRU cache, eviction, disk persistence |
| API | 15 | Rate limiting, exponential backoff, library methods |
| State | 25 | Navigation, focus, search, scrolling, home state |
| UI | 20 | Component rendering, themes, overlays |
| Config | 5 | Config loading, defaults, save/restore |
| CLI | 24 | Command parsing, execution, output formatting |
| Daemon | 14 | IPC protocol, command handling, JSON messages |
| Lyrics | 10 | LRCLIB API, synced lyrics parsing |
| Media Control | 10 | MPRIS stubs, platform abstraction |
| Notifications | 17 | Rate limiting, duplicate detection |
| Search | 17 | Fuzzy matching, relevance scoring |
| Themes | 12 | Theme loading, color schemes |
| Logging | 12 | Log rotation, level filtering |

## Releases

- **v0.4.0** (Current) - Release Readiness: CLI, daemon mode, themes, lyrics, logging
- **v0.3.0** - Polish Core UX: Config, visualization, media control, notifications, fuzzy search
- **v0.2.0** - Drill-down navigation, album/artist detail views, vim-style shortcuts
- **v0.1.0** - Initial release: Basic playback, search, queue, album art

See [CHANGELOG.md](CHANGELOG.md) for detailed release notes.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) first.

### Reporting Issues

- Bug reports: Include steps to reproduce, expected vs actual behavior
- Feature requests: Describe the use case and desired behavior
- Security issues: Please email directly before opening a public issue

---

Built with ⚡ by Joshify
