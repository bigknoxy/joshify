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