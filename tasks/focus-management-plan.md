# Keyboard Focus Management Plan

## Problem Statement

Current issues:
1. **Tab from navigation doesn't focus main content**: When pressing Tab from sidebar, focus cycles through Sidebar → MainContent → PlayerBar, but when MainContent is focused, keyboard navigation (j/k/Enter) doesn't work
2. **Enter on sidebar doesn't transfer focus**: Pressing Enter on a sidebar nav item should immediately shift focus to the main content area so users can navigate items
3. **Double-click on tracks not working**: Mouse double-click to play tracks in main content isn't functioning

## Research Findings

### TUI Focus Management Patterns

From analyzing Helix, Zellij, Tmux, and other TUI tools:

**Helix Editor Pattern** (`helix-term/src/compositor.rs`):
- Uses `Component` trait with `handle_event()` method
- Events bubble through layers from front to back
- Each component declares `required_size()` and `cursor()` position
- Focus is implicit — the topmost layer that consumes the event has focus

**Tmux Pattern** (`screen-redraw.c`):
- Explicit pane focus with active/inactive border highlighting
- Focus determines which pane receives keyboard input
- Mouse clicks transfer focus to clicked pane

**Standard TUI Conventions**:
1. **Tab/Shift+Tab**: Cycle focus between major regions (sidebar → main → player)
2. **Enter on nav**: Select nav item AND transfer focus to associated content
3. **Arrow keys**: Navigate within the focused region
4. **hjkl**: Vim-style navigation (optional but expected in terminal apps)
5. **Double-click**: Mouse action to open/play item

## Implementation Plan

### Phase 1: Fix Enter Key Focus Transfer

When Enter is pressed on sidebar:
- Currently: Loads content but keeps focus in sidebar
- Fix: After loading content, transfer focus to MainContent

**Code location**: `main.rs`, Enter key handler around line 1783

### Phase 2: Fix Main Content Keyboard Navigation

When MainContent is focused:
- j/k or Down/Up should navigate items
- Enter should play/select item
- Currently: Only works when sidebar is focused (j/k navigate sidebar)

**Root cause**: The j/k handlers check `if app.focus == FocusTarget::Sidebar` first, then `else if app.focus == FocusTarget::MainContent`. The logic is correct but we need to ensure:
1. Content is loaded before navigation works
2. selected_index and scroll_offset are reset when switching views

### Phase 3: Add Vim-Style Navigation (h/j/k/l)

Add shortcuts:
- `h`: Navigate left / back
- `j`: Down (already implemented for sidebar)
- `k`: Up (already implemented for sidebar)
- `l`: Navigate right / into item

This matches patterns in lazygit, tig, and other terminal tools.

### Phase 4: Fix Double-Click Playback

Double-click mouse events should trigger playback. Currently:
- Single click: selects item
- Double click: should play item

Check `mouse_handler.rs` and main.rs mouse handling around line 2337.

## Detailed Implementation

### 1. Fix Enter on Sidebar → Transfer Focus

```rust
// In main.rs, Enter key handler (around line 1783)
FocusTarget::Sidebar => {
    // Load content based on selected_nav
    match app.selected_nav {
        NavItem::LikedSongs => {
            app.content_state = ContentState::Loading(LoadAction::LikedSongs);
            app.selected_index = 0;
            app.scroll_offset = 0;
            // FIX: Transfer focus to main content
            app.focus = FocusTarget::MainContent;
        }
        // ... same for other nav items
    }
}
```

### 2. Ensure Main Content Navigation Works

The j/k handlers already check `FocusTarget::MainContent`, but we need to verify:
- Library view content navigation is handled
- Album/Artist tabs can be switched with Tab key

Add Library content state handling:
```rust
// In j/k handlers, add Library to the match
ContentState::Library { albums, artists, selected_tab } => {
    match selected_tab {
        LibraryTab::Albums => albums.len(),
        LibraryTab::Artists => artists.len(),
    }
}
```

### 3. Add Tab Navigation Within Main Content

When MainContent is focused, Tab should:
- In Library view: switch between Albums/Artists tabs
- In other views: no-op or cycle through interactive elements

### 4. Fix Double-Click

The mouse handler should generate `MouseAction::PlayTrack` on double-click. Check:
- `mouse_handler.rs`: Detects double-click timing
- `layout_cache.rs`: Has track item areas
- Main event loop: Handles `MouseAction::PlayTrack`

## Testing Checklist

- [ ] Enter on Library → loads albums → focus transfers to main content
- [ ] Enter on Playlists → loads playlists → focus transfers to main content
- [ ] Enter on Liked Songs → loads tracks → focus transfers to main content
- [ ] Tab cycles: Sidebar → MainContent → PlayerBar → Sidebar
- [ ] When MainContent focused:
  - [ ] j/Down navigates down list
  - [ ] k/Up navigates up list
  - [ ] Enter plays selected item
- [ ] When Library view shown:
  - [ ] Tab switches between Albums/Artists tabs
  - [ ] j/k navigates items in current tab
- [ ] Double-click on track plays it
- [ ] Double-click on playlist opens it

## Future Enhancements (Out of Scope)

- Visual focus indicator (highlight border of focused region)
- More granular focus within main content (e.g., focus on tab bar vs list)
- Search-as-you-type within lists
- Number prefix for jumping to item N
