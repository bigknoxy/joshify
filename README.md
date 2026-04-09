# Joshify ⚡

A beautiful terminal Spotify client built with Rust and ratatui.

[![Build Status](https://img.shields.io/github/actions/workflow/status/bigknoxy/joshify/ci.yml?branch=main&style=flat-square)](https://github.com/bigknoxy/joshify/actions/workflows/ci.yml)
[![Tests](https://img.shields.io/badge/tests-222%20passing-brightgreen?style=flat-square)](https://github.com/bigknoxy/joshify)
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
   - `Enter` - Play selected track
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
   - `?` - Show help
   - `q` - Quit

### Non-Interactive Mode (headless/automated)

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

## Features

- **Full Spotify Integration** - Play any track, browse playlists, access liked songs
- **Search** - Find any song, artist, or album
- **Album Art** - Displayed with terminal graphics protocols (kitty, sixel, iTerm2) or ASCII fallback
- **Queue Management** - View and add tracks to queue
- **Keyboard First** - All actions accessible via keyboard shortcuts
- **Minimal Resource Usage** - Runs entirely in your terminal
- **Comprehensive Test Suite** - 64 tests covering all core functionality

## System Requirements

- macOS, Linux, or Windows with a terminal emulator
- Spotify Premium account
- Terminal with UTF-8 support
- For best album art: kitty, iTerm2, or sixel-capable terminal

## Configuration

Joshify stores credentials in `~/.config/joshify/credentials.json`. Credentials are automatically saved to your OS keyring when available (GNOME Keyring, macOS Keychain, Windows Credential Manager).

To re-authenticate or change accounts, press `c` in the app.

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
├── main.rs          # Application entry point, event loop
├── auth.rs          # OAuth authentication
├── album_art.rs     # LRU-cached album art fetching
├── keyring_store.rs # OS keyring integration
├── api/             # Spotify API client
│   ├── client.rs    # Client creation & auth
│   ├── playback.rs  # Playback controls
│   ├── library.rs   # Library & search
│   └── rate_limit.rs# Rate limit handling
├── state/           # Application state
│   ├── app_state.rs # Main state coordinator
│   ├── player_state.rs # Playback state
│   ├── load_coordinator.rs # Async task coordination
│   ├── library_state.rs   # Library cache
│   └── queue_state.rs     # Queue management
└── ui/              # Terminal rendering
    ├── sidebar.rs   # Navigation sidebar
    ├── main_view.rs # Main content area
    ├── player_bar.rs# Now playing bar
    └── overlays.rs  # Search, help, queue
```

## Tech Stack

- **Rust** - Systems programming language
- **ratatui** - Terminal UI framework
- **rspotify** - Spotify API client
- **tokio** - Async runtime
- **crossterm** - Terminal manipulation
- **lru** - LRU cache for album art (50 entry limit)
- **keyring** - OS keyring integration
- **serde** - Serialization for credentials

## Testing

Joshify has a comprehensive test suite with 63 tests across 9 categories:

| Category | Tests | Description |
|----------|-------|-------------|
| Player | 9 | Duration formatting, PlayerState, track detection |
| Auth | 8 | Credentials loading, OAuth config, persistence |
| Album Art | 6 | LRU cache, eviction, disk persistence |
| API | 11 | Rate limiting, exponential backoff |
| State | 7 | Navigation, focus, search, scrolling |
| UI | 8 | Component rendering tests |
| Concurrency | 5 | Async coordination, stale result rejection |
| Error Injection | 8 | Timeout, malformed data, failure handling |

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
