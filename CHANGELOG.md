## [0.5.0](https://github.com/bigknoxy/joshify/compare/v0.4.0...v0.5.0) (2026-05-04)

### Bug Fixes

- **Playback Queue Auto-Advance**: Fixed issue where selecting a track in a playlist would cause it to play twice before continuing. Now correctly advances to the next track after playback ends.
- **Remote Mode Context Playback**: Fixed duplicate `playback_next()` call that could cause skipped tracks in Remote mode. Spotify handles auto-advance within context.
- **Mouse Click Handler**: Fixed missing `set_context_position()` call when double-clicking tracks, ensuring queue advancement works correctly for mouse interactions.

### Technical Details

- Added explicit `advance()` call after starting playback to keep queue position in sync
- Improved debug logging throughout playback flow for easier troubleshooting
- Fixed position tracking semantics: `context_position` now correctly represents "next track to be returned by advance()"

## [0.4.0](https://github.com/bigknoxy/joshify/compare/v0.3.0...v0.4.0) (2026-04-27)

### Features

- **Daemon Mode**: Background service with Unix socket IPC (`joshify daemon`, `joshify daemon-send`). JSON protocol for commands. 14 tests.
- **CLI Commands**: Full command-line interface for scripting. Commands: play, pause, next, previous, stop, status, volume, seek, search, queue-add. Output formats: text, json, minimal. 24 tests.
- **Lyrics Display**: Synced lyrics via LRCLIB API. Real-time lyric display with timestamp parsing. 10 tests.
- **Theme System**: 7 built-in themes (Catppuccin Mocha/Latte, Gruvbox Dark/Light, Nord, Tokyo Night, Dracula). Dynamic theme switching. Theme trait for extensibility. 12 tests.
- **Structured Logging**: Tracing-based logging with file rotation (10MB max, 5 files). Log level filtering. 12 tests.
- **Documentation**: Updated README with all new features, CLI examples, configuration guide.

### Dependencies

- Added `tracing-appender = "0.2"` for log rotation
- Added `toml = "0.8"` for configuration files
- Added `dirs-next = "2"` for config directory detection
- Added `realfft = "3"` for FFT audio visualization
- Added `notify-rust = "4"` for Linux notifications (optional)

---

## [0.3.0](https://github.com/bigknoxy/joshify/compare/v0.2.0...v0.3.0) (2026-04-27)

### Features

- **Configuration System**: TOML-based configuration at `~/.config/joshify/config.toml`. Settings for audio, notifications, media control, UI, keybindings. Auto-created with defaults. 5 tests.
- **Audio Visualization**: Real-time FFT spectrum visualization. Configurable bands (32, 64, 128). Smoothing factor control. Works with local playback. 7 tests.
- **Media Control**: MPRIS integration for OS media key support. Platform abstraction for Linux/macOS/Windows. Commands: play, pause, next, previous, stop. 10 tests.
- **Desktop Notifications**: Native OS notifications on track change. Rate limiting (5s cooldown). Duplicate detection. Album art thumbnails when available. 17 tests.
- **Fuzzy Search**: Typo-tolerant search with relevance scoring. Custom implementation with consecutive match bonuses and gap penalties. 17 tests.
- **Test Suite Growth**: 280+ tests covering all new functionality.

### Dependencies

- Added `toml = "0.8"` for configuration parsing
- Added `dirs-next = "2"` for cross-platform directories
- Added `realfft = "3"` for FFT processing
- Added `num-complex = "0.4"` for complex numbers (FFT)

---

## [0.2.0](https://github.com/bigknoxy/joshify/compare/v0.1.0...v0.2.0) (2026-04-08)

### Breaking Changes

- Player bar height increased from 5 to 6 rows to accommodate the new layout
- Search Enter key now attempts device discovery before playback (previously silently failed)
- Debounce timing changed from "time since last search dispatch" to "time since last keystroke"

### Features

- **Search overhaul**: Fixed search returning no results by adding `Market::FromToken` parameter and capping API limit to 10 (Spotify's new maximum). Search now works correctly.
- **Search UX**: Tab key adds selected track to queue. Enter plays the track. Results display in overlay with proper truncation.
- **Cursor alignment**: Search cursor and text now align correctly using unicode display width instead of character count. Works with emoji and wide characters.
- **Now Playing redesign**: 4 interior rows — scrolling title (bold Mauve, marquee animation for long names), artist line (dim + badges + key hints), progress bar (Green Gauge widget + time labels), volume bar (visual indicator with percentage).
- **Scrolling title**: Long track names now scroll horizontally like a car radio (8 cols/sec, 2-second pause at start/end, resets on track change).
- **All key handlers non-blocking**: Play/pause, next/prev, volume, seek, shuffle, repeat, device transfer — all use `tokio::spawn` instead of blocking `.lock().await`.
- **Optimistic volume updates**: Volume keys update local state immediately, no 2-second delay before visual feedback.
- **Album art repositions on resize**: Art is re-processed with correct coordinates when the terminal is resized. Kitty images are explicitly deleted before redrawing.
- **Gorilla-with-headphones ASCII art**: Replaced the mauve face logo with a green gorilla wearing mauve headphones.

### Bug Fixes

- **Search cursor misalignment** (#1): Used `unicode-width` for display width calculation instead of `chars().count()`. Replaced emoji `🔍` with ASCII `/` in the search prompt prefix.
- **Search infinite re-search loop** (#2): Added `last_searched_query` field to prevent re-firing a search for a query that already returned results. Debounce now measures from last keystroke.
- **Search Enter not playing** (#3): Added device discovery (`available_devices()` + `transfer_playback()`) before `start_playback()`. Previously passed `device_id=None` which silently failed with 403.
- **Stale search results on fast typing** (#4): `insert_char`/`delete_char` now clear `pending_query` so stale results are discarded.
- **Channel only processing one message per loop** (#5): Changed `if let Ok` to `while let Ok` for `rx.try_recv()` to drain all pending messages.
- **TUI freezing on key presses** (#6): All 15 key handlers that called `client.lock().await` directly now use `tokio::spawn` for async execution.
- **Text overflow in overlays** (#7): Queue and Help overlays now use `bg.inner()` for content rendering and `truncate_from_start()` for text truncation. Matches the Search overlay pattern.
- **Album art ghost on resize** (#8): Kitty images now use protocol-level delete command (`\x1b_Ga=d`) and space-filling (not `\x1b[K` which erases to end of line, wiping adjacent content).
- **Progress bar spacing** (#9): Time labels now use separate layout columns with proper gaps instead of cramming everything on one line.
- **Volume bar misalignment** (#10): Standardized volume bar patterns to consistent 6-character widths.
- **`truncate()` panic on multi-byte characters** (#11): Replaced byte-slicing with `unicode_truncate()` in `main_view.rs`.
- **Spotify API limit** (#12): Spotify reduced max search results from 50 to 10. Updated all search calls.

### Dependencies

- Added `unicode-width = "0.2"` for display width calculations
- Added `unicode-truncate = "2"` for safe width-based truncation