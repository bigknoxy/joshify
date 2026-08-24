# Joshify ⚡

<p align="center">
  <img src="assets/logo.svg" alt="Joshify Logo" width="200">
</p>

<p align="center">
  <b>A beautiful terminal Spotify client built with Rust</b>
</p>

<p align="center">
  <a href="https://github.com/bigknoxy/joshify/actions/workflows/ci.yml">
    <img src="https://img.shields.io/github/actions/workflow/status/bigknoxy/joshify/ci.yml?branch=main&style=for-the-badge&logo=github&label=CI" alt="Build Status">
  </a>
  <a href="https://github.com/bigknoxy/joshify/actions/workflows/visual-tests.yml">
    <img src="https://img.shields.io/github/actions/workflow/status/bigknoxy/joshify/visual-tests.yml?branch=main&style=for-the-badge&logo=github&label=Visual%20Tests" alt="Visual Tests">
  </a>
  <a href="https://github.com/bigknoxy/joshify/releases/latest">
    <img src="https://img.shields.io/github/v/release/bigknoxy/joshify?style=for-the-badge&logo=rust&color=orange" alt="Latest Release">
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/license-MIT-blue?style=for-the-badge" alt="License">
  </a>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#installation">Installation</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#screenshots">Screenshots</a> •
  <a href="#documentation">Documentation</a>
</p>

---

## ✨ Features

- 🎵 **Full Spotify Integration** - Play any track, browse playlists, access liked songs
- 🏠 **Local Playback** - Play directly through your computer (no Spotify app needed)
- 🔍 **Fuzzy Search** - Typo-tolerant search with relevance scoring
- 🎨 **Album Art** - Terminal graphics protocols (kitty, sixel, iTerm2) or ASCII fallback
- 📊 **Audio Visualization** - Real-time FFT spectrum visualization (32/64/128 bands)
- 🎤 **Lyrics Display** - Synced lyrics via LRCLIB API
- 📋 **Queue Management** - View and manage playback queue
- 🎭 **7 Beautiful Themes** - Catppuccin, Gruvbox, Nord, Tokyo Night, Dracula, and more
- ⌨️ **Keyboard First** - Vim-style navigation, all actions via keyboard
- 🖱️ **Mouse Support** - Click to play, scroll to navigate
- 🔔 **Desktop Notifications** - Native OS notifications on track change
- 💻 **CLI Mode** - Full command-line interface for scripting
- 🔄 **Daemon Mode** - Background service with IPC control
- 📸 **Visual Testing** - Automated screenshot testing with VHS
- ⚡ **Lightning Fast** - Built with Rust for minimal resource usage

## 📸 Demo

<p align="center">
  <img src="assets/demo.gif" alt="Joshify Demo" width="800">
</p>

<p align="center">
  <i>Auto-generated with <a href="https://github.com/charmbracelet/vhs">VHS</a> on every push</i>
</p>

### Screenshots

<p align="center">
  <img src="screenshots/reference/demo.gif" alt="Joshify demo" width="800">
</p>

| Home View | Library View | Search |
|-----------|--------------|--------|
| <img src="screenshots/reference/home_view.png" alt="Home" width="300"> | <img src="screenshots/reference/library_view.png" alt="Library" width="300"> | <img src="screenshots/reference/search_overlay.png" alt="Search" width="300"> |

## 🚀 Installation

### One-Line Installer (Recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/bigknoxy/joshify/main/install.sh | bash
```

#### Installer environment variables

| Variable | Effect |
| --- | --- |
| `TMPDIR` | Preferred scratch directory. Respected if binaries can be executed from it; otherwise the installer falls back to `/tmp`, then `~/.cache/joshify/tmp`. |
| `JOSHIFY_SKIP_DEPS=1` | Skip the system dependency step and just print the packages to install yourself. |

#### Running without a terminal

The system libraries need `sudo`. When a terminal is attached, the installer
authenticates once up front (via `/dev/tty`, so it works under `curl | bash`)
and reuses the cached credential. When there is no TTY and `sudo` is not
passwordless — CI, background jobs, some SSH/orchestration setups — it does
**not** fail: it prints the packages to install and continues to build Joshify.
Install those packages first, or pre-authorize `sudo`, for a fully unattended run.

Hosts with `/tmp` mounted `noexec` (common on WSL2 and hardened Linux) are
handled automatically — the installer probes the temp directory and relocates
if `rustup-init` could not be executed from it.

### Pre-built Binaries

Download the latest release for Linux x86_64 or macOS (Apple Silicon) from the
[releases page](https://github.com/bigknoxy/joshify/releases/latest).

### From Source

```bash
git clone https://github.com/bigknoxy/joshify.git
cd joshify
cargo install --path .
```

### Prerequisites

- Spotify Premium account
- Terminal with UTF-8 support
- For album art: kitty, iTerm2, or sixel-capable terminal

### Supported Platforms

Pre-built release binaries are provided for **Linux x86_64** and **macOS (Apple Silicon)**.
Other platforms (Intel macOS, ARM Linux, musl) can build from source; broader
pre-built coverage is tracked in [#33](https://github.com/bigknoxy/joshify/issues/33).

System libraries needed to build from source (installed automatically by
`install.sh` when it can elevate — see
[Running without a terminal](#running-without-a-terminal)):
- Linux: `libasound2-dev pkg-config libssl-dev build-essential libchafa-dev libglib2.0-dev`
- macOS: `brew install pkgconf chafa`

## 🎮 Quick Start

### Interactive Mode

```bash
# Start Joshify
joshify

# With mock data (no Spotify auth required)
JOSHIFY_MOCK=1 cargo run
```

**Navigation:**
- `Tab` / `Shift+Tab` - Switch between sections
- `↑` / `↓` or `j` / `k` - Navigate lists
- `h` / `l` - Focus sidebar / main content
- `Enter` - Play selected track
- `Backspace` - Go back
- `/` - Search
- `?` - Show help
- `q` - Quit

**Playback:**
- `Space` - Play/Pause
- `n` / `p` - Next/Previous track
- `←` / `→` - Seek ±10 seconds
- `+` / `-` - Volume up/down
- `s` / `r` - Toggle shuffle/repeat

### CLI Mode

```bash
# Control playback
joshify play
joshify pause
joshify next
joshify previous

# Get status
joshify status
joshify current

# Search and queue
joshify search "never gonna give you up"
joshify queue-add <track-uri>
```

### Daemon Mode

```bash
# Start daemon
joshify daemon

# Control via CLI
joshify daemon-send play
joshify daemon-send pause
```

## 🧪 Testing

Joshify has comprehensive test coverage (unit, integration, and doc tests — run `cargo test` for the current count):

```bash
# Run all tests
cargo test

# Run specific test categories
cargo test --lib          # Unit tests
cargo test --test ui      # UI tests
cargo test mock           # Mock data tests

# Visual testing with VHS
./scripts/vhs-setup.sh
./scripts/capture-screenshots.sh
```

## 📁 Project Structure

```
joshify/
├── src/
│   ├── main.rs           # Application entry point
│   ├── ui/               # Terminal UI components
│   │   ├── sidebar.rs    # Navigation sidebar
│   │   ├── player_bar.rs # Now playing bar
│   │   ├── home_view.rs  # Home dashboard
│   │   └── theme.rs      # Catppuccin Mocha theme
│   ├── state/            # Application state
│   │   ├── app_state.rs  # Main state coordinator
│   │   ├── player_state.rs
│   │   └── mock_data.rs  # Mock data for testing
│   ├── player/           # Local playback (librespot)
│   ├── api/              # Spotify REST client
│   └── auth/             # OAuth flow
├── tapes/                # VHS visual test scripts
├── scripts/              # Helper scripts
└── docs/                 # Documentation
```

## 🎨 Themes

Press `T` to cycle through themes:

| Theme | Description |
|-------|-------------|
| Catppuccin Mocha | Default - Dark pastel theme |
| Catppuccin Latte | Light variant |
| Gruvbox Dark | Retro dark theme |
| Gruvbox Light | Retro light theme |
| Nord | Arctic North blue theme |
| Tokyo Night | Dark blue-purple theme |
| Dracula | Classic dark theme |

## 📚 Documentation

- [VHS Usage Guide](docs/VHS_USAGE.md) - Visual testing documentation
- [Architecture Overview](#architecture) - Technical details
- [Contributing](CONTRIBUTING.md) - How to contribute
- [Changelog](CHANGELOG.md) - Version history

## 🏗️ Architecture

Joshify uses a modular architecture:

- **UI Layer** (`src/ui/`) - Ratatui-based terminal interface
- **State Layer** (`src/state/`) - Application state management
- **Player Layer** (`src/player/`) - librespot local playback
- **API Layer** (`src/api/`) - Spotify Web API client
- **Auth Layer** (`src/auth/`) - OAuth authentication

### Key Design Patterns

- **State Isolation** - Each domain has its own state module
- **Coordinator Pattern** - `LoadCoordinator` manages async data loading
- **Mock Data** - `JOSHIFY_MOCK=1` for testing without auth
- **Pre-processing** - Heavy work done once, not per-frame

## 🤝 Contributing

Contributions welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) first.

### Development Setup

```bash
# Clone the repo
git clone https://github.com/bigknoxy/joshify.git
cd joshify

# Install dependencies
cargo build

# Run tests
cargo test

# Run with mock data
JOSHIFY_MOCK=1 cargo run
```

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

## 🙏 Acknowledgments

- [ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI framework
- [librespot](https://github.com/librespot-org/librespot) - Spotify Connect library
- [rspotify](https://github.com/ramsayleung/rspotify) - Spotify Web API client
- [Catppuccin](https://catppuccin.com) - Beautiful color theme
- [VHS](https://github.com/charmbracelet/vhs) - Terminal recording

---

<p align="center">
  Built with ⚡ by <a href="https://github.com/bigknoxy">bigknoxy</a>
</p>

<p align="center">
  <a href="https://github.com/bigknoxy/joshify/stargazers">⭐ Star this repo</a> •
  <a href="https://github.com/bigknoxy/joshify/issues">🐛 Report issues</a> •
  <a href="https://github.com/bigknoxy/joshify/discussions">💬 Discussions</a>
</p>
