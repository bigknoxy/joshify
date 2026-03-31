# Joshify ⚡

A beautiful terminal Spotify client built with Rust and ratatui.

```
     ⚡ JOSHIFY ⚡
    ╱▔▔▔▔▔▔▔▔▔╲
   ╱  ▀▄   ▄▀  ╲
  │   ▄▀▀▀▀▄   │
  │  │ ▀▀ │  │
   ╲  ╲__╱  ╱
    ╲_____╱
```

## One-Line Installer

```bash
curl -fsSL https://raw.githubusercontent.com/joshify/joshify/main/install.sh | bash
```

## Manual Installation

### From Source (requires Rust)

```bash
git clone https://github.com/joshify/joshify.git
cd joshify
cargo install --path .
```

### Via Cargo

```bash
cargo install joshify
```

### Via Homebrew (macOS)

```bash
brew install joshify
```

## Quick Start

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

4. **Playback Controls**
   - `Space` - Play/Pause
   - `n` - Next track
   - `p` - Previous track
   - `←` / `→` - Seek ±10 seconds
   - `+` / `-` - Volume up/down

5. **Other Actions**
   - `Q` - Toggle queue view
   - `a` - Add current track to queue
   - `?` - Show help
   - `q` - Quit

## Features

- **Full Spotify Integration** - Play any track, browse playlists, access liked songs
- **Search** - Find any song, artist, or album
- **Album Art** - Displayed with terminal graphics protocols (kitty, sixel, iTerm2) or ASCII fallback
- **Queue Management** - View and add tracks to queue
- **Keyboard First** - All actions accessible via keyboard shortcuts
- **Minimal Resource Usage** - Runs entirely in your terminal

## System Requirements

- macOS, Linux, or Windows with a terminal emulator
- Spotify Premium account
- Terminal with UTF-8 support
- For best album art: kitty, iTerm2, or sixel-capable terminal

## Configuration

Joshify stores credentials in `~/.config/joshify/credentials.json`.

To re-authenticate or change accounts, press `c` in the app.

## Development

```bash
# Run in development
cargo run

# Build release
cargo build --release

# Run tests
cargo test
```

## Tech Stack

- **Rust** - Systems programming language
- **ratatui** - Terminal UI framework
- **rspotify** - Spotify API client
- **tokio** - Async runtime
- **crossterm** - Terminal manipulation

## License

MIT License - see [LICENSE](LICENSE) for details.

## Contributing

Contributions welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) first.

---

Built with ⚡ by Joshify
