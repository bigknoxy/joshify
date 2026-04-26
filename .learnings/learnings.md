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

## Future Learning Sources
- Test failures
- Code review feedback
- Performance bottlenecks
- User experience issues
- API behavior surprises
