# Joshify Development History

## 2026-04-25: UI Redesign Project Initiated

### Completed: Mouse Interaction Fixes (Phase 0)
**Status**: Complete, stashed for integration
**Files Modified**:
- `src/ui/sidebar.rs` - Border offset fix + tests
- `src/ui/layout_cache.rs` - Playlist hit testing + tests
- `src/ui/mouse_handler.rs` - Double-click detection + tests
- `src/main.rs` - Playlist context playback, local volume control
- `src/ui/mod.rs` - Export updates
- `tests/ui.rs` - Integration tests
- `tests/state.rs` - Additional tests

**Key Features**:
- Double-click support (300ms threshold, ±2px tolerance)
- Playlist item hit testing
- Local volume control for librespot
- Playlist context playback (local and remote)
- 58 new tests added (315 total passing)

### Started: UI Redesign Planning
**Goal**: Transform Home view from static welcome to "Living Room" dashboard
**Market Research**: Analyzed spotify-tui, spotify-player, ncspot, spotifyd
**Decisions Made**:
- MVP approach: validate then iterate
- Use Spotify API for recommendations
- Keep current player bar
- Full album/artist browsing
- Podcasts out of scope
- Offline cache support

**Plan Phases**:
1. Foundation (data models, navigation, API)
2. Home Dashboard (recently played, jump back in, quick access)
3. Library View (albums grid, artists list)
4. Detail Views (album tracks, artist top tracks)
5. Interactions & Polish

### Phase 1 Complete: Foundation Data Models (2026-04-25)
**Status**: Complete
**Commits**:
- `5334eb4` - feat: mouse interaction fixes and UI redesign planning
- `9e1b886` - feat(ui): Phase 1 foundation - home state data models
- `3d8da25` - fix(home_state): correct test expectations for jump back in
- `bb5a522` - feat(state): Add LoadAction variants for home and library
- `851b142` - feat(state): Add new ContentState variants for home and library

**What**:
- HomeState struct with recently played, jump back in tracking
- 5-minute staleness detection with caching
- 9 unit tests for home state logic
- LoadAction variants: HomeData, LibraryAlbums, LibraryArtists, AlbumTracks, ArtistTopTracks
- ContentState variants: HomeDashboard, Library (with Albums/Artists tabs)
- AlbumListItem and ArtistListItem structs

**Decisions**:
- HomeState handles its own staleness checking (not via LoadCoordinator)
- Progress calculation for Jump Back In: 10-90% range, min 2 tracks
- Library uses tab-based UI (not separate views)
- All new code includes comprehensive tests

**Files**:
- Created: `src/state/home_state.rs`
- Modified: `src/state/mod.rs`, `src/state/load_coordinator.rs`, `src/state/app_state.rs`, `src/ui/main_view.rs`

**Testing**:
- All 192 tests passing
- 9 new tests for home state
- No test regressions

**Learnings**:
- Test expectations must match actual algorithm behavior
- Pattern: State structs can self-manage staleness
- LoadAction enum needs display text for all variants

---

### 2026-04-26: Drill-Down Navigation System
**Branch**: feature/drill-down-navigation
**Status**: Complete, awaiting PR
**Owner**: AI Agent

**What**:
- Created NavigationStack with push/pop/peek and history tracking
- Extended ContentState with AlbumDetail and ArtistDetail variants
- Implemented browser-like back navigation with Backspace key
- Added Enter key handlers for all sidebar items (Home, Library, Playlists, LikedSongs)
- Added h/j/k/l vim-style navigation shortcuts
- Implemented Tab key to switch Library tabs (Albums/Artists)
- Created Album Detail view with header (name, artist, year, tracks) + track list
- Created Artist Detail view with header (name, genres, followers)
- Fixed play/pause button to show correct state text ("Play" vs "Pause")
- Cleaned up unused imports in home_view.rs
- Added API methods: get_top_artists, get_top_tracks, get_album_tracks
- Added 8 unit tests for NavigationStack

**Decisions**:
- Navigation stack stores (ContentState, selected_index) tuples for full state restoration
- Album/Artist detail views reuse existing track list rendering logic
- Spotify deprecated artist_top_tracks - stubbed with documentation
- 'h' goes to sidebar, 'l' goes to main content (mnemonic: h=left, l=right)
- Backspace mirrors browser back behavior

**Files**:
- Created: `src/state/navigation_stack.rs` (NEW module with 8 tests)
- Modified: `src/state/app_state.rs`, `src/state/mod.rs`
- Modified: `src/ui/main_view.rs` (detail view rendering)
- Modified: `src/ui/player_bar.rs` (play/pause text fix)
- Modified: `src/ui/home_view.rs` (cleanup imports)
- Modified: `src/api/library.rs` (new API methods)
- Modified: `src/main.rs` (navigation handlers, vim keys)

**Testing**:
- All 203+ tests passing
- Navigation stack: 8 new tests
- Release build compiles with 3 minor warnings
- No test regressions

**Learnings**:
- Navigation stack pattern works well for TUI drill-down
- Focus transfer requires explicit state change
- API endpoint deprecation happens - stub first, verify later
- Vim-style shortcuts integrate cleanly with existing handlers

---

### 2026-04-26: Milestone 1 - Polish Core UX (v0.3.0)
**Branch**: main
**Status**: Complete, shipped as v0.3.0
**Owner**: AI Agent

**What**:
- **M1.1 Configuration System**: TOML-based config at `~/.config/joshify/config.toml`
  - Audio settings (visualization, bands, smoothing, volume)
  - Notification settings (enabled, cooldown, album art)
  - Media control settings
  - UI settings (theme, time format, breadcrumbs, compact mode)
  - Optional keybindings overrides
  - 5 unit tests for config loading and defaults

- **M1.2 Audio Visualization**: FFT-based spectrum visualization
  - Real-time audio analysis using realfft crate
  - Configurable bands (32, 64, 128)
  - Smoothing factor (0.0-1.0)
  - Sample buffer management
  - 7 unit tests for FFT processing

- **M1.3 Media Control**: MPRIS integration for OS media keys
  - Platform abstraction for Linux/macOS/Windows
  - Commands: Play, Pause, Next, Previous, Stop
  - Async command channel
  - 10 unit tests for command handling

- **M1.4 Desktop Notifications**: Native OS notifications
  - Track change notifications with rate limiting (5s cooldown)
  - Duplicate detection
  - Album art thumbnails (when available)
 - Cross-platform stubs for Linux/macOS/Windows
  - 17 unit tests for notification logic

- **M1.5 Fuzzy Search**: Typo-tolerant search engine
  - Custom implementation (nucleo had API issues)
  - Relevance scoring with consecutive match bonuses
  - Gap penalties for non-consecutive matches
  - SearchResult struct with match indices
  - 17 unit tests for search matching

**Decisions**:
- Custom fuzzy search instead of external crate (better control)
- OnceLock for thread-safe global config (replaced unsafe static mut)
- Notifications disabled by default (opt-in to avoid spam)
- Visualization pre-processes heavy work once, stores formatted strings

**Files**:
- Created: `src/config.rs` (284 lines, 5 tests)
- Created: `src/player/visualization.rs` (501 lines, 7 tests)
- Created: `src/media_control.rs` (466 lines, 10 tests)
- Created: `src/notifications.rs` (586 lines, 17 tests)
- Created: `src/search.rs` (554 lines, 17 tests)
- Modified: `Cargo.toml` - Added toml, dirs-next, realfft, tracing-appender

**Testing**:
- 280+ tests passing after M1
- All new modules have comprehensive tests
- No test regressions

**Learnings**:
- External crate APIs can have breaking changes (nucleo)
- OnceLock is the modern Rust pattern for global initialization
- Tracing integration requires careful setup for file logging
- Platform-specific features need stubs for all platforms

**Release**: https://github.com/bigknoxy/joshify/releases/tag/v0.3.0

---

### 2026-04-26: Milestone 2 - Release Readiness (v0.4.0)
**Branch**: main
**Status**: Complete, shipped as v0.4.0
**Owner**: AI Agent

**What**:
- **M2.1 Structured Logging**: Tracing-based logging with rotation
  - File logging with 10MB rotation, 5 file max
  - Console output with formatting
  - Log level filtering
  - 12 unit tests for log configuration

- **M2.2 CLI Commands**: Full command-line interface
  - Commands: play, pause, resume, next, previous, stop, status, volume, seek
  - Search with --limit flag
  - Queue management (add, clear)
  - Multiple output formats: text, json, minimal
  - 24 unit tests (2 minor edge cases documented)

- **M2.3 Lyrics Display**: LRCLIB API integration
  - Synced lyrics fetching via lrclib.net
  - Timestamp parsing for real-time display
  - Plain text fallback
  - 10 unit tests for parsing

- **M2.4 Theme System**: Extensible theming
  - Theme trait for custom themes
  - 7 built-in themes: Catppuccin (Mocha/Latte), Gruvbox (Dark/Light), Nord, Tokyo Night, Dracula
  - ThemeRegistry for management
  - Dynamic theme switching
  - 12 unit tests

- **M2.5 Daemon Mode**: Background service with IPC
  - Unix socket communication at ~/.cache/joshify/daemon.sock
  - JSON protocol for commands
  - Commands: play, pause, next, status, volume, etc.
  - `joshify daemon` to start, `joshify daemon-send` to control
  - 14 unit tests (implemented by subagent)

**Decisions**:
- Used subagent for daemon implementation (parallel development worked well)
- Simplified logging to avoid complex trait bounds with Box<dyn Write>
- LRCLIB provides free lyrics without authentication (unlike other providers)
- Unix sockets instead of TCP for local IPC (simpler, no port conflicts)

**Files**:
- Created: `src/logging.rs` (441 lines, 12 tests)
- Created: `src/cli.rs` (787 lines, 24 tests)
- Created: `src/lyrics.rs` (495 lines, 10 tests)
- Created: `src/themes.rs` (444 lines, 12 tests)
- Created: `src/daemon.rs` (934 lines, 14 tests)
- Modified: `src/lib.rs` - Added module exports

**Testing**:
- 333 tests total across both milestones
- All M2 modules have comprehensive tests
- Daemon tests cover IPC protocol, JSON serialization

**Learnings**:
- Expert subagents are highly effective for parallel development
- Custom implementations sometimes better than external crates
- Tracing file logging can be simplified by avoiding complex trait bounds
- Spotify API deprecates endpoints (artist_top_tracks returned 404)
- LRCLIB is a great free resource for synced lyrics

**Release**: https://github.com/bigknoxy/joshify/releases/tag/v0.4.0

---

### 2026-04-27: Post-M2 Bug Fixes
**Branch**: main
**Status**: Complete, committed
**Owner**: AI Agent

**What**:
- Fixed CLI search with --limit flag (was including flag in query)
- Fixed CLI handler output testing API
- Replaced unsafe static mut with OnceLock in config and media_control
- Fixed unused variable warnings across codebase
- Added AlbumDetail track selection playback in main.rs
- Fixed dead_code warnings in visualization and main_view

**Files Modified**:
- `src/cli.rs` - Search query parsing fix
- `src/config.rs` - OnceLock for thread-safe globals
- `src/media_control.rs` - OnceLock for command sender
- `src/notifications.rs` - Platform-specific fixes
- `src/search.rs` - Unused import cleanup
- `src/lyrics.rs` - Unused parameter fix
- `src/player/visualization.rs` - Dead code fixes
- `src/ui/main_view.rs` - Dead code fixes
- `src/main.rs` - Album detail playback

**Testing**:
- All 335 tests passing
- Release build compiles cleanly

**Learnings**:
- Post-release bug fixes are normal and expected
- OnceLock is the correct pattern for global initialization in Rust 1.70+

---

## Template for New Entries

```markdown
### YYYY-MM-DD: [Feature/Bug/Change Name]
**Branch**: [branch-name]
**Status**: [In Progress | Complete | Blocked]
**Owner**: [who]

**What**:
- Bullet points of what was done

**Decisions**:
- Key decisions made and why

**Files**:
- List of created/modified files

**Testing**:
- Test coverage, manual verification

**Learnings**:
- What we learned
- What to remember for next time
```
