# Joshify Learnings Log

## Format
Each entry should include:
- Date
- Category (bug, pattern, decision, gotcha)
- Description
- Prevention strategy

---

## 2026-04-25

### Category: Pattern
**Learned**: Mouse event handling requires careful coordinate math with terminal borders
**Context**: Sidebar nav items were off by 1 due to not accounting for `Borders::ALL` in hit testing
**Prevention**: Always account for border offset when calculating hit test regions. Content starts at `area.y + 1` when borders are present.
**File**: `src/ui/sidebar.rs`

### Category: Bug
**Learned**: LayoutCache `area_at()` must check all variants, not just track_items
**Context**: Playlist items weren't clickable because `ClickableArea` enum had `PlaylistItem` but `area_at()` didn't check it
**Prevention**: When adding new clickable areas, update BOTH the enum AND the hit test function
**File**: `src/ui/layout_cache.rs`

### Category: Gotcha
**Learned**: Volume normalization differs between local and remote playback
**Context**: Spotify API uses 0-100, librespot uses 0-65535
**Prevention**: Always normalize volume based on playback mode. Use `(new_volume as u32 * 65535 / 100) as u16` for local, direct for remote.
**File**: `src/main.rs`

### Category: Pattern
**Learned**: u16 overflow can happen in seemingly safe calculations
**Context**: `new_volume as u16 * 65535 / 100` overflows at volume > 99
**Prevention**: Cast to u32 BEFORE multiplication, then back to u16
**File**: `src/main.rs`

---

## 2026-04-26

### Category: Pattern
**Learned**: Navigation stack pattern enables browser-like back/forward navigation in TUI
**Context**: Implemented NavigationStack with push/pop/peek methods, storing (ContentState, selected_index) tuples
**Prevention**: When drilling down (Enter on item), push current state BEFORE loading new view. On back, restore saved state.
**File**: `src/state/navigation_stack.rs`

### Category: Pattern
**Learned**: ContentState enum extension requires careful handling of all match arms
**Context**: Added AlbumDetail and ArtistDetail variants; had to update render_content() to handle all cases
**Prevention**: When adding enum variants, use exhaustive match to find all locations needing updates
**File**: `src/state/app_state.rs`, `src/ui/main_view.rs`

### Category: Gotcha
**Learned**: Focus transfer from sidebar to main content requires explicit state change
**Context**: Enter key on sidebar items was loading content but not changing focus target
**Prevention**: Always set `app.focus = FocusTarget::MainContent` when user navigates to content from sidebar
**File**: `src/main.rs`

### Category: Decision
**Learned**: Spotify deprecated artist_top_tracks endpoint; using simplified approach
**Context**: API call was returning 404, decided to stub rather than implement workaround
**Prevention**: Check API changelog before implementing features; stub first, verify endpoint availability
**File**: `src/api/library.rs`

### Category: Pattern
**Learned**: Vim-style navigation (h/j/k/l) integrates well with existing arrow key handlers
**Context**: Added 'h' for sidebar focus, 'l' for main content, 'j'/'k' for up/down
**Prevention**: Keep navigation intuitive - h=left (sidebar is left), l=right (content is right)
**File**: `src/main.rs`

---

## 2026-04-26

### Category: Pattern
**Learned**: OnceLock is the modern Rust pattern for lazy static initialization
**Context**: Replaced unsafe `static mut` with `std::sync::OnceLock` for config, media_control, and daemon globals
**Prevention**: Always use OnceLock for thread-safe global initialization in Rust 1.70+. It provides safe, one-time initialization without unsafe blocks.
**File**: `src/config.rs`, `src/media_control.rs`

### Category: Decision
**Learned**: Custom fuzzy search can be better than external crates with API issues
**Context**: Nucleo crate had API issues, so implemented custom fuzzy search with scoring
**Prevention**: When external crates have API problems, consider if a custom implementation is feasible. Sometimes simpler is better.
**File**: `src/search.rs`

### Category: Gotcha
**Learned**: Tracing file logging can be complex with trait bounds
**Context**: Attempted complex Box<dyn Write> setup for tracing-appender, simplified to avoid trait bound issues
**Prevention**: For file logging with tracing, use simple working patterns. Don't over-abstract the writer type.
**File**: `src/logging.rs`

### Category: Pattern
**Learned**: Expert subagents enable effective parallel development
**Context**: Daemon mode (14 tests) was implemented by subagent while main agent worked on other features
**Prevention**: For independent features, use subagents to parallelize work. Provide complete context and clear deliverables.
**File**: `src/daemon.rs`

### Category: Decision
**Learned**: LRCLIB provides free synced lyrics without authentication
**Context**: Other lyrics APIs require auth or paid tiers. LRCLIB is free and works well.
**Prevention**: Research free APIs before committing to paid providers. Open-source alternatives often exist.
**File**: `src/lyrics.rs`

### Category: Gotcha
**Learned**: CLI argument parsing with flags requires careful filtering
**Context**: Search query was including `--limit` and its value in the query string
**Prevention**: When parsing args with flags, filter out flag pairs (flag + value) before joining the query.
**File**: `src/cli.rs`

### Category: Pattern
**Learned**: Theme system with trait allows extensibility beyond hardcoded themes
**Context**: Created Theme trait that users could implement for custom themes
**Prevention**: Use traits for theming systems - provides both built-in options and extensibility.
**File**: `src/themes.rs`

### Category: Decision
**Learned**: Unix sockets are simpler than TCP for local IPC
**Context**: Daemon mode uses Unix sockets at ~/.cache/joshify/daemon.sock instead of TCP
**Prevention**: For local-only IPC, prefer Unix sockets (no port conflicts, file-based permissions, simpler cleanup).
**File**: `src/daemon.rs`

---

## Future Learning Sources
- Test failures
- Code review feedback
- Performance bottlenecks
- User experience issues
- API behavior surprises
- Documentation gaps
