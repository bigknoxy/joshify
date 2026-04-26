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

## Future Learning Sources
- Test failures
- Code review feedback
- Performance bottlenecks
- User experience issues
- API behavior surprises
