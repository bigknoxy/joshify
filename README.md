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

## Installation

### One-Line Installer (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/bigknoxy/joshify/main/install.sh | bash
```

### Via npm

```bash
npm install -g joshify
```
*Coming soon - not yet published*

### Via bun

```bash
bun add -g joshify
```
*Coming soon - not yet published*

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

## Uninstall

```bash
# If installed via shell script
curl -fsSL https://raw.githubusercontent.com/bigknoxy/joshify/main/uninstall.sh | bash

# If installed via npm
npm uninstall -g joshify

# If installed via bun
bun remove -g joshify

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
