# Search Cursor Alignment Fix — Summary

## Problem
When pressing `/` in the TUI to search:
1. **Cursor misalignment**: The cursor appeared 1-2 cells to the left of where typed letters appeared
2. **Unicode issues**: The code used `chars().count()` instead of display width, breaking for emoji and wide characters
3. **Panic risk**: The `truncate()` function used byte slicing which could panic on multi-byte characters

## Root Cause
Two interacting bugs:

### Bug 1: Emoji Display Width Miscalculation
The search input prefix `"  🔍 "` was rendered with the 🔍 emoji, which displays as **2 terminal columns** wide in most terminals. However, the cursor offset calculation assumed it was 1 column wide:

```rust
// WRONG: assumes emoji is 1 column
inner.x + 3 + (cursor_pos as u16)  // offset of 3 for "  🔍"
```

The actual display width is 4 columns (2 spaces + 2-wide emoji + 1 space), causing the cursor to be off by 1-2 cells.

### Bug 2: Character Count vs Display Width
All width calculations used `chars().count()` which treats every character as 1 column wide. This fails for:
- Emoji (🔍, 🦀, etc.) — display width 2
- CJK characters — display width 2
- Fullwidth characters — display width 2

The `truncate()` function in `main_view.rs` used byte slicing (`&text[..max_width]`) which can **panic** when slicing within multi-byte UTF-8 characters.

## Solution

### 1. Added Dependencies
```toml
unicode-width = "0.2"      # Correct display width calculation
unicode-truncate = "2"     # Safe width-based truncation
```

### 2. Replaced Emoji with ASCII in Search Input
Changed the search input prefix from `"  🔍 "` (5 display columns with variable-width emoji) to `"  / "` (4 display columns, all ASCII width-1):

```rust
// src/ui/theme.rs
pub const SEARCH_PROMPT: &str = "/";  // ASCII for search input prefix
```

This eliminates variable-width emoji in the one place where pixel-perfect cursor alignment matters.

### 3. Fixed All Width Calculations
Replaced `chars().count()` with `UnicodeWidthStr::width()` throughout:

**src/ui/overlays.rs:**
```rust
fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

fn truncate_from_start(text: &str, max_width: usize) -> String {
    if display_width(text) <= max_width {
        text.to_string()
    } else {
        let (truncated, _) = text.unicode_truncate_start(max_width.saturating_sub(1));
        format!("…{}", truncated)
    }
}
```

**Cursor positioning:**
```rust
let cursor_display_offset = search_state.cursor_display_offset();
let query_width = search_state.query_display_width();

if query_width > input_max_width {
    // Truncated case: account for skipped display width
    let visible_start_width = query_width.saturating_sub(input_max_width.saturating_sub(1));
    let visible_cursor_offset = cursor_display_offset.saturating_sub(visible_start_width);
    inner.x + prefix_width as u16 + 1 + visible_cursor_offset as u16
} else {
    inner.x + prefix_width as u16 + cursor_display_offset as u16
}
```

### 4. Fixed truncate() in main_view.rs
```rust
fn truncate(text: &str, max_width: usize) -> String {
    use unicode_truncate::UnicodeTruncateStr;
    use unicode_width::UnicodeWidthStr;
    
    if UnicodeWidthStr::width(text) <= max_width {
        text.to_string()
    } else {
        let (truncated, _) = text.unicode_truncate(max_width.saturating_sub(1));
        format!("{truncated}…")
    }
}
```

### 5. Added Helper Methods to SearchState
```rust
pub fn cursor_display_offset(&self) -> usize {
    let byte_pos = self.byte_index();
    if byte_pos == 0 {
        0
    } else if byte_pos >= self.query.len() {
        UnicodeWidthStr::width(self.query.as_str())
    } else {
        UnicodeWidthStr::width(&self.query[..byte_pos])
    }
}

pub fn query_display_width(&self) -> usize {
    UnicodeWidthStr::width(self.query.as_str())
}
```

### 6. Fixed player_bar.rs Truncation
Same pattern — replaced `.len()` and `.chars().take()` with display-width-aware truncation.

### 7. Removed Misleading Footer Text
Removed `a: Add to queue` from the search footer since pressing 'a' would insert the character into the search query (can't have both behaviors).

## Files Changed
- `Cargo.toml` — Added unicode-width and unicode-truncate deps
- `src/ui/theme.rs` — Added `SEARCH_PROMPT` ASCII symbol
- `src/ui/overlays.rs` — Fixed cursor positioning, truncation, added tests
- `src/ui/main_view.rs` — Fixed `truncate()` function
- `src/ui/player_bar.rs` — Fixed name truncation
- `src/state/search_state.rs` — Added display width helper methods + tests

## Verification

### Automated Tests
All 212 tests pass:
```bash
cargo test
# 107 lib tests + 6 album_art + 13 album_art_rendering + 11 api + 8 auth +
# 5 concurrency + 8 error_injection + 18 performance + 4 playback_api +
# 3 playback_error + 25 player + 7 state + 8 ui = 212 total
```

### Manual Testing
Run the app and test:
```bash
cargo run
```

Then press `/` and test:
1. **Basic typing**: Type "hello" — cursor should align with each letter
2. **Emoji typing**: Type "🦀" — cursor should move 2 cells right
3. **Mixed content**: Type "a🦀b" — cursor should be at positions 0, 1, 3, 4
4. **Long queries**: Type a long query that triggers truncation — cursor should stay aligned
5. **Search functionality**: Type a search query — results should appear
6. **Navigation**: Use ↑↓ to navigate results, Enter to play

### Linting
```bash
cargo clippy --message-format=short
# Only 3 pre-existing warnings about function argument counts (not related to this fix)
```

### Release Build
```bash
cargo build --release
# Compiles successfully
```

## Related Issues
This is a well-known class of bug in ratatui TUI apps. The ratatui maintainers fixed similar issues in:
- PR #1089 — Fixed unicode truncation panics
- PR #2188 — Acknowledged unicode-width vs terminal display discrepancies

The fix pattern used here (using `unicode-width` and `unicode-truncate`) is the same approach ratatui uses internally.
