# Joshify Mouse Implementation Plan
## Goal: Match/Exceed spotatui's Mouse Interaction Quality

This plan provides a step-by-step guide for implementing professional-grade mouse support in joshify, based on extensive research of spotatui (the gold standard for TUI Spotify clients) and lazydocker's interaction patterns.

---

## Research Summary

### What Makes spotatui's Mouse Support Excellent

1. **Unified Event Handling**: Mouse events flow through the same channel as keyboard events
2. **Layout-Based Hit Testing**: Uses ratatui's Rect system for precise click detection
3. **Smart Click-to-Index Mapping**: Accounts for borders, padding, and scroll offset
4. **Focus Management**: Clicking automatically shifts focus to the clicked component
5. **Double-Click Actions**: Clicking an already-selected item activates it
6. **Playbar Hitboxes**: Custom hitbox system for player controls (play/pause, next, prev, volume)
7. **Wheel Scrolling**: Scroll wheel navigates lists and content areas
8. **Input Mode Awareness**: Mouse is disabled when typing in search fields

### Key Architectural Patterns from Research

**spotatui's approach (src/tui/handlers/mouse.rs - 900+ lines):**
- Single `handler(mouse: MouseEvent, app: &mut App)` entry point
- Route-based dispatch (different handlers for different views)
- `main_layout_areas()` calculates all interactive Rects once per frame
- `rect_contains()` helper for hit testing
- Component-specific handlers delegate to keyboard handlers (reuse logic!)
- `list_item_index_from_click()` handles scroll offset math

**lazydocker's approach (Go/gocui):**
- Mouse as "keys" (MouseWheelUp/Down treated as key events)
- View hit testing via `VisibleViewByPosition()`
- Automatic focus switching on click
- Click-to-select-line with cursor + origin calculation

---

## Phase 1: Core Mouse Infrastructure (Foundation)

### 1.1 Enable Mouse Capture in Main
**File**: `src/main.rs`

Add mouse capture enable/disable:

```rust
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};

// In initialization (around line 150-200):
execute!(stdout, EnableMouseCapture)?;

// In cleanup/shutdown:
execute!(stdout, DisableMouseCapture)?;
```

**Test**: Run app, click around - should see mouse events captured (no terminal selection)

### 1.2 Extend Event System for Mouse
**File**: `src/main.rs` (event handling section)

Current code polls keyboard events. Extend to capture mouse:

```rust
// Current (around line where events are polled):
if event::poll(poll_duration)? {
    if let Event::Key(key) = event::read()? {
        // handle key
    }
}

// NEW: Handle both key and mouse
if event::poll(poll_duration)? {
    match event::read()? {
        Event::Key(key) => {
            // existing key handling
        }
        Event::Mouse(mouse_event) => {
            // NEW: Route to mouse handler
            handle_mouse_event(mouse_event, &mut app);
        }
        _ => {}
    }
}
```

**Test**: Add debug print, verify mouse clicks are captured

### 1.3 Create Mouse Handler Module
**NEW File**: `src/ui/mouse_handler.rs`

Based on spotatui's pattern, create the main mouse handler:

```rust
use crossterm::event::{MouseEvent, MouseEventKind, MouseButton};
use ratatui::layout::{Rect, Position};
use crate::state::AppState;

/// Main entry point for mouse events
pub fn handle_mouse_event(mouse: MouseEvent, app: &mut AppState) {
    // Get current focus area
    let current_focus = app.ui.focus;
    
    // Calculate all layout areas for hit testing
    let areas = calculate_layout_areas(app);
    
    // Route to appropriate handler based on click location
    match mouse.kind {
        MouseEventKind::ScrollDown => handle_scroll_down(mouse, &areas, app),
        MouseEventKind::ScrollUp => handle_scroll_up(mouse, &areas, app),
        MouseEventKind::Down(MouseButton::Left) => {
            handle_left_click(mouse, &areas, app)
        }
        MouseEventKind::Down(MouseButton::Right) => {
            handle_right_click(mouse, &areas, app)
        }
        _ => {} // Ignore other events
    }
}

/// Calculate all interactive areas on screen
fn calculate_layout_areas(app: &AppState) -> LayoutAreas {
    // TODO: Calculate based on current terminal size and layout
    // This mirrors spotatui's main_layout_areas() function
    LayoutAreas {
        sidebar: calculate_sidebar_rect(app),
        main_view: calculate_main_view_rect(app),
        player_bar: calculate_player_bar_rect(app),
        search_input: if app.ui.search_active { Some(calculate_search_rect(app)) } else { None },
    }
}

/// Check if point is inside rect
fn rect_contains(rect: Rect, x: u16, y: u16) -> bool {
    let right = rect.x.saturating_add(rect.width);
    let bottom = rect.y.saturating_add(rect.height);
    x >= rect.x && x < right && y >= rect.y && y < bottom
}

// Helper structs
pub struct LayoutAreas {
    pub sidebar: Rect,
    pub main_view: Rect,
    pub player_bar: Rect,
    pub search_input: Option<Rect>,
}
```

**Test**: Compile and run, verify no crashes on mouse events

---

## Phase 2: Component-Specific Mouse Handlers

### 2.1 Sidebar Mouse Handler
**File**: `src/ui/mouse_handler.rs` (add function)

Handle clicks and scrolls on sidebar navigation:

```rust
fn handle_sidebar_mouse(mouse: MouseEvent, sidebar_area: Rect, app: &mut AppState) {
    match mouse.kind {
        MouseEventKind::ScrollDown => {
            // Scroll down in sidebar
            app.ui.sidebar.selected = (app.ui.sidebar.selected + 1)
                .min(app.ui.sidebar.items.len().saturating_sub(1));
            app.ui.focus = FocusArea::Sidebar;
        }
        MouseEventKind::ScrollUp => {
            // Scroll up in sidebar
            app.ui.sidebar.selected = app.ui.sidebar.selected.saturating_sub(1);
            app.ui.focus = FocusArea::Sidebar;
        }
        MouseEventKind::Down(MouseButton::Left) => {
            // Click to select nav item
            if let Some(index) = sidebar_index_from_click(sidebar_area, mouse.row, app) {
                let was_selected = app.ui.sidebar.selected == index;
                app.ui.sidebar.selected = index;
                app.ui.focus = FocusArea::Sidebar;
                
                // Double-click behavior: activate if already selected
                if was_selected {
                    activate_sidebar_item(app);
                }
            }
        }
        _ => {}
    }
}

/// Convert mouse Y position to sidebar item index
fn sidebar_index_from_click(
    sidebar_area: Rect,
    mouse_row: u16,
    app: &AppState
) -> Option<usize> {
    // Account for borders and padding
    let inner_top = sidebar_area.y.saturating_add(1); // Top border
    let inner_bottom = sidebar_area.y + sidebar_area.height - 1; // Bottom border
    
    if mouse_row < inner_top || mouse_row >= inner_bottom {
        return None; // Clicked in border area
    }
    
    let row_index = (mouse_row - inner_top) as usize;
    let item_count = app.ui.sidebar.items.len();
    
    // Calculate scroll offset (sidebar may have more items than visible)
    let visible_height = sidebar_area.height.saturating_sub(2) as usize;
    let selected = app.ui.sidebar.selected;
    let offset = selected.saturating_add(1).saturating_sub(visible_height);
    
    let clicked_index = offset + row_index;
    
    if clicked_index < item_count {
        Some(clicked_index)
    } else {
        None
    }
}

fn activate_sidebar_item(app: &mut AppState) {
    // Navigate to selected section
    // This should call existing navigation logic
    match app.ui.sidebar.selected {
        0 => navigate_to_home(app),
        1 => navigate_to_search(app),
        2 => navigate_to_library(app),
        // ... etc
        _ => {}
    }
}
```

**Test**: Click sidebar items, verify selection changes. Double-click to navigate.

### 2.2 Track List (Main View) Mouse Handler
**File**: `src/ui/mouse_handler.rs` (add function)

Handle clicks and scrolls on track lists (most complex handler):

```rust
fn handle_track_list_mouse(mouse: MouseEvent, main_area: Rect, app: &mut AppState) {
    if app.main_view.tracks.is_empty() {
        return;
    }
    
    match mouse.kind {
        MouseEventKind::ScrollDown => {
            // Scroll down, load next page if at bottom
            let current = app.main_view.selected_index;
            let max = app.main_view.tracks.len().saturating_sub(1);
            
            if current < max {
                app.main_view.selected_index += 1;
            } else {
                // At bottom, load next page
                load_next_page(app);
            }
            app.ui.focus = FocusArea::MainView;
        }
        MouseEventKind::ScrollUp => {
            // Scroll up, load prev page if at top
            let current = app.main_view.selected_index;
            
            if current > 0 {
                app.main_view.selected_index -= 1;
            } else {
                // At top, load previous page
                load_previous_page(app);
            }
            app.ui.focus = FocusArea::MainView;
        }
        MouseEventKind::Down(MouseButton::Left) => {
            // Click to select track
            if let Some(index) = track_index_from_click(main_area, mouse.row, app) {
                app.main_view.selected_index = index;
                app.ui.focus = FocusArea::MainView;
                
                // Single click selects, double-click plays
                // (Need to track last click time for double-click detection)
                // For now: click plays immediately (simpler UX)
                play_selected_track(app);
            }
        }
        MouseEventKind::Down(MouseButton::Right) => {
            // Right-click: context menu (optional future feature)
            // For now: add to queue
            if let Some(index) = track_index_from_click(main_area, mouse.row, app) {
                add_track_to_queue(app, index);
            }
        }
        _ => {}
    }
}

/// Convert mouse position to track index (handles pagination)
fn track_index_from_click(
    main_area: Rect,
    mouse_row: u16,
    app: &AppState
) -> Option<usize> {
    // Account for table header (usually 2-3 rows)
    let header_rows = 2u16;
    let first_data_row = main_area.y.saturating_add(header_rows);
    let last_data_row = main_area.y + main_area.height - 1;
    
    if mouse_row < first_data_row || mouse_row >= last_data_row {
        return None;
    }
    
    let visible_rows = main_area.height.saturating_sub(3) as usize; // Header + borders
    let row_index = (mouse_row - first_data_row) as usize;
    
    // Calculate page-based offset (like spotatui's approach)
    let selected = app.main_view.selected_index;
    let page_size = visible_rows;
    let current_page = selected / page_size;
    let offset = current_page * page_size;
    
    let clicked_index = offset + row_index;
    let item_count = app.main_view.tracks.len();
    
    if clicked_index < item_count {
        Some(clicked_index)
    } else {
        None
    }
}
```

**Test**: Scroll track list, click tracks to play. Verify pagination works.

### 2.3 Player Bar Mouse Handler
**File**: `src/ui/mouse_handler.rs` (add function)

Handle clicks on player controls (play/pause, next, prev, volume, progress):

```rust
/// Player control hitboxes (like spotatui's PlaybarControl enum)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerControl {
    Prev,
    PlayPause,
    Next,
    Shuffle,
    Repeat,
    Like,
    VolumeDown,
    VolumeUp,
    ProgressBar { position: f32 }, // Click position as percentage
}

fn handle_player_bar_mouse(mouse: MouseEvent, player_area: Rect, app: &mut AppState) {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(control) = player_control_at(mouse.column, mouse.row, player_area, app) {
                match control {
                    PlayerControl::Prev => skip_to_previous(app),
                    PlayerControl::PlayPause => toggle_playback(app),
                    PlayerControl::Next => skip_to_next(app),
                    PlayerControl::Shuffle => toggle_shuffle(app),
                    PlayerControl::Repeat => toggle_repeat(app),
                    PlayerControl::Like => toggle_like_current_track(app),
                    PlayerControl::VolumeDown => adjust_volume(app, -5),
                    PlayerControl::VolumeUp => adjust_volume(app, 5),
                    PlayerControl::ProgressBar { position } => {
                        seek_to_position(app, position);
                    }
                }
            }
        }
        MouseEventKind::ScrollDown => {
            // Scroll down = volume down
            adjust_volume(app, -5);
        }
        MouseEventKind::ScrollUp => {
            // Scroll up = volume up
            adjust_volume(app, 5);
        }
        _ => {}
    }
}

/// Calculate which control was clicked
fn player_control_at(
    x: u16,
    y: u16,
    player_area: Rect,
    app: &AppState
) -> Option<PlayerControl> {
    // Define hitboxes for each control
    // Layout: [Prev] [Play/Pause] [Next] | Progress | [Shuffle] [Repeat] Vol: [---]
    
    let controls_y = player_area.y + 1; // Center vertically in player bar
    
    // Check if click is on control row
    if y != controls_y && y != controls_y + 1 {
        // Check progress bar area (typically 1-2 rows)
        if y >= player_area.y + 1 && y <= player_area.y + player_area.height - 2 {
            // Click on progress bar - calculate position
            let progress_start_x = player_area.x + 20; // After controls
            let progress_end_x = player_area.x + player_area.width - 25; // Before volume
            
            if x >= progress_start_x && x <= progress_end_x {
                let position = (x - progress_start_x) as f32 / 
                              (progress_end_x - progress_start_x) as f32;
                return Some(PlayerControl::ProgressBar { position });
            }
        }
        return None;
    }
    
    // Calculate control positions
    let mut current_x = player_area.x + 2;
    
    // Prev button
    if x >= current_x && x < current_x + 6 {
        return Some(PlayerControl::Prev);
    }
    current_x += 7;
    
    // Play/Pause button
    if x >= current_x && x < current_x + 10 {
        return Some(PlayerControl::PlayPause);
    }
    current_x += 11;
    
    // Next button
    if x >= current_x && x < current_x + 6 {
        return Some(PlayerControl::Next);
    }
    current_x += 15;
    
    // Shuffle
    if x >= current_x && x < current_x + 8 {
        return Some(PlayerControl::Shuffle);
    }
    current_x += 9;
    
    // Repeat
    if x >= current_x && x < current_x + 8 {
        return Some(PlayerControl::Repeat);
    }
    
    // Volume controls (right side)
    let volume_x = player_area.x + player_area.width - 15;
    if x >= volume_x && x < volume_x + 3 {
        return Some(PlayerControl::VolumeDown);
    }
    if x >= volume_x + 8 && x < volume_x + 11 {
        return Some(PlayerControl::VolumeUp);
    }
    
    None
}
```

**Test**: Click all player controls, verify they work. Click progress bar to seek.

---

## Phase 3: Integration with Existing Code

### 3.1 Wire Up Mouse Handler in Main Loop
**File**: `src/main.rs`

Integrate mouse handler into the event loop:

```rust
// Around line 400-500 where events are processed
loop {
    // ... existing setup ...
    
    if event::poll(poll_duration)? {
        match event::read()? {
            Event::Key(key) => {
                // Existing key handling
                if handle_key_event(key, &mut app).await? {
                    break; // Exit requested
                }
            }
            Event::Mouse(mouse) => {
                // NEW: Mouse handling
                use crate::ui::mouse_handler::handle_mouse_event;
                handle_mouse_event(mouse, &mut app);
            }
            Event::Resize(width, height) => {
                // Existing resize handling
                app.ui.size = (width, height);
            }
            _ => {}
        }
    }
    
    // ... render ...
}
```

### 3.2 Update State to Track Mouse-Relevant Data
**File**: `src/state/app_state.rs` (add fields)

```rust
#[derive(Debug, Clone)]
pub struct AppState {
    // ... existing fields ...
    
    /// Current terminal size for layout calculations
    pub terminal_size: (u16, u16),
    
    /// Last mouse click position (for double-click detection)
    pub last_click: Option<(u16, u16, std::time::Instant)>,
    
    /// Current focus area
    pub focus: FocusArea,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusArea {
    Sidebar,
    MainView,
    PlayerBar,
    SearchInput,
    HelpOverlay,
}
```

### 3.3 Modify Renderers to Report Their Areas
**Files**: `src/ui/sidebar.rs`, `src/ui/main_view.rs`, `src/ui/player_bar.rs`

To enable accurate hit testing, renderers need to report where they drew:

```rust
// In sidebar.rs render function
pub fn render_sidebar(
    frame: &mut Frame,
    app: &AppState,
    area: Rect,
    layout_cache: &mut LayoutCache, // NEW: Report position
) {
    // ... existing render code ...
    
    // NEW: Store area for mouse hit testing
    layout_cache.sidebar = area;
}

// Similar for main_view.rs and player_bar.rs
```

### 3.4 Create LayoutCache for Hit Testing
**NEW File**: `src/ui/layout_cache.rs`

```rust
use ratatui::layout::Rect;

/// Cache of rendered layout areas for mouse hit testing
#[derive(Debug, Default, Clone)]
pub struct LayoutCache {
    pub sidebar: Rect,
    pub main_view: Rect,
    pub player_bar: Rect,
    pub search_input: Option<Rect>,
    pub help_overlay: Option<Rect>,
}

impl LayoutCache {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Find which area contains the given point
    pub fn area_at(&self, x: u16, y: u16) -> Option<ClickableArea> {
        if Self::rect_contains(self.sidebar, x, y) {
            return Some(ClickableArea::Sidebar);
        }
        if Self::rect_contains(self.main_view, x, y) {
            return Some(ClickableArea::MainView);
        }
        if Self::rect_contains(self.player_bar, x, y) {
            return Some(ClickableArea::PlayerBar);
        }
        if let Some(search) = self.search_input {
            if Self::rect_contains(search, x, y) {
                return Some(ClickableArea::SearchInput);
            }
        }
        if let Some(help) = self.help_overlay {
            if Self::rect_contains(help, x, y) {
                return Some(ClickableArea::HelpOverlay);
            }
        }
        None
    }
    
    fn rect_contains(rect: Rect, x: u16, y: u16) -> bool {
        x >= rect.x && x < rect.x + rect.width &&
        y >= rect.y && y < rect.y + rect.height
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClickableArea {
    Sidebar,
    MainView,
    PlayerBar,
    SearchInput,
    HelpOverlay,
}
```

---

## Phase 4: Testing & Polish

### 4.1 Unit Tests for Hit Testing
**NEW File**: `src/ui/mouse_handler/tests.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;
    
    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(10, 5, 20, 10);
        
        assert!(rect_contains(rect, 10, 5));     // Top-left corner
        assert!(rect_contains(rect, 29, 14));    // Bottom-right corner (exclusive)
        assert!(!rect_contains(rect, 30, 14));   // Outside right
        assert!(!rect_contains(rect, 10, 15));   // Outside bottom
        assert!(rect_contains(rect, 20, 10));    // Center
    }
    
    #[test]
    fn test_sidebar_index_from_click() {
        // Create mock app state
        let mut app = create_test_app();
        app.ui.sidebar.items = vec!["Home", "Search", "Library", "Queue"];
        app.ui.sidebar.selected = 0;
        
        let sidebar_area = Rect::new(0, 0, 20, 6); // 4 items + 2 borders
        
        // Click first item (row 1, inside border)
        assert_eq!(sidebar_index_from_click(sidebar_area, 1, &app), Some(0));
        
        // Click second item
        assert_eq!(sidebar_index_from_click(sidebar_area, 2, &app), Some(1));
        
        // Click border (should be None)
        assert_eq!(sidebar_index_from_click(sidebar_area, 0, &app), None);
        assert_eq!(sidebar_index_from_click(sidebar_area, 5, &app), None);
    }
    
    #[test]
    fn test_track_index_from_click() {
        let mut app = create_test_app();
        app.main_view.tracks = vec![Track::default(); 10];
        app.main_view.selected_index = 0;
        
        let main_area = Rect::new(20, 3, 60, 12); // Header at rows 3-4, data at 5+
        
        // Click first data row
        assert_eq!(track_index_from_click(main_area, 5, &app), Some(0));
        
        // Click third data row
        assert_eq!(track_index_from_click(main_area, 7, &app), Some(2));
        
        // Test pagination: selected item 10, should show page 2
        app.main_view.selected_index = 10;
        // With page_size ~9, page 2 starts at index 9
        assert_eq!(track_index_from_click(main_area, 5, &app), Some(9));
    }
}
```

### 4.2 Integration Test for Mouse Flow
**File**: `tests/ui.rs` (add test)

```rust
#[test]
fn test_mouse_click_selects_sidebar_item() {
    let mut app = create_test_app();
    
    // Simulate click on second sidebar item
    let mouse_event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 5,
        row: 2,
        modifiers: KeyModifiers::empty(),
    };
    
    handle_mouse_event(mouse_event, &mut app);
    
    assert_eq!(app.ui.sidebar.selected, 1);
    assert_eq!(app.ui.focus, FocusArea::Sidebar);
}

#[test]
fn test_mouse_scroll_moves_selection() {
    let mut app = create_test_app();
    app.ui.sidebar.selected = 0;
    
    // Simulate scroll down
    let scroll_event = MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 5,
        row: 2,
        modifiers: KeyModifiers::empty(),
    };
    
    handle_mouse_event(scroll_event, &mut app);
    
    assert_eq!(app.ui.sidebar.selected, 1);
}
```

### 4.3 Visual Feedback (Optional Enhancement)

Add hover state tracking for better UX:

```rust
// In AppState
pub hover_area: Option<ClickableArea>,

// In mouse handler, on MouseEventKind::Moved (if enabled)
fn handle_mouse_move(mouse: MouseEvent, areas: &LayoutAreas, app: &mut AppState) {
    // Requires enabling mouse move events in crossterm
    app.hover_area = areas.area_at(mouse.column, mouse.row);
}

// In renderers, use hover state to highlight
if app.hover_area == Some(ClickableArea::Sidebar) {
    // Render with hover styling
}
```

---

## Phase 5: Advanced Features (Future)

### 5.1 Double-Click Detection
```rust
const DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(300);

fn is_double_click(app: &AppState, x: u16, y: u16) -> bool {
    if let Some((last_x, last_y, last_time)) = app.last_click {
        if last_x == x && last_y == y && last_time.elapsed() < DOUBLE_CLICK_THRESHOLD {
            return true;
        }
    }
    false
}
```

### 5.2 Drag Scrolling
```rust
// Track mouse drag for scrollable areas
fn handle_mouse_drag(mouse: MouseEvent, app: &mut AppState) {
    if let Some((start_x, start_y)) = app.drag_start {
        let delta_y = mouse.row as i16 - start_y as i16;
        // Scroll by delta
    }
}
```

### 5.3 Context Menus on Right-Click
```rust
fn show_context_menu(x: u16, y: u16, context: ContextMenuType, app: &mut AppState) {
    app.context_menu = Some(ContextMenu {
        x, y, items: context.items(),
    });
}
```

---

## Implementation Checklist

### Phase 1: Foundation
- [ ] Add EnableMouseCapture/DisableMouseCapture in main.rs
- [ ] Extend event loop to handle Event::Mouse
- [ ] Create src/ui/mouse_handler.rs with basic structure
- [ ] Test: Mouse events are captured and routed

### Phase 2: Component Handlers
- [ ] Implement sidebar mouse handler (click + scroll)
- [ ] Implement track list mouse handler (click + scroll + pagination)
- [ ] Implement player bar mouse handler (buttons + progress bar + volume)
- [ ] Test: All components respond to mouse

### Phase 3: Integration
- [ ] Add LayoutCache for storing rendered areas
- [ ] Modify renderers to report their areas
- [ ] Wire mouse handler into main event loop
- [ ] Update AppState with mouse-relevant fields
- [ ] Test: Mouse works end-to-end

### Phase 4: Testing
- [ ] Write unit tests for hit testing functions
- [ ] Write integration tests for mouse flows
- [ ] Manual testing: Click everything, scroll everything
- [ ] Test edge cases: Empty lists, small terminals, etc.

### Phase 5: Polish (Optional)
- [ ] Add double-click detection
- [ ] Add hover state tracking (requires mouse move events)
- [ ] Add right-click context menus
- [ ] Performance: Profile mouse handling

---

## Key Design Decisions

1. **Layout-based hit testing** (spotatui pattern): Calculate Rect areas and check containment
2. **Delegate to keyboard handlers** (spotatui pattern): Mouse actions reuse existing logic
3. **Focus tracking**: Clicking automatically shifts focus for keyboard continuity
4. **Scroll offset math**: Account for borders and pagination when mapping clicks to indices
5. **Custom hitboxes for controls**: Player controls need precise hitbox calculation

## Files to Create/Modify

**New Files:**
- `src/ui/mouse_handler.rs` - Main mouse handler (600+ lines expected)
- `src/ui/layout_cache.rs` - Layout area tracking

**Modified Files:**
- `src/main.rs` - Event loop, enable mouse capture
- `src/state/app_state.rs` - Add mouse-relevant fields
- `src/ui/mod.rs` - Export mouse handler
- `src/ui/sidebar.rs` - Report render area
- `src/ui/main_view.rs` - Report render area
- `src/ui/player_bar.rs` - Report render area, add control hitboxes

## Success Criteria

- [ ] All sidebar navigation items clickable
- [ ] Track list scrollable with mouse wheel
- [ ] Track selection and playback via click
- [ ] All player controls clickable (play, next, prev, volume, progress)
- [ ] Clicking changes focus appropriately
- [ ] Mouse doesn't interfere with keyboard usage
- [ ] All 21+ tests pass
- [ ] No regression in keyboard-only usage

---

## Reference: spotatui Code Locations

For detailed reference:
- `src/tui/handlers/mouse.rs` - Complete mouse handler (900+ lines)
- `src/tui/ui/player.rs` - Player control hitboxes
- `src/app.rs` - ActiveBlock enum, focus management
- `src/main.rs` - Event loop integration

## Reference: lazydocker Code Locations

- `vendor/github.com/jesseduffield/gocui/gui.go` - Mouse event handling
- `pkg/gui/keybindings.go` - Mouse keybindings
- `pkg/gui/view_helpers.go` - Click handling

---

**Plan Author**: Research-based implementation guide
**Based On**: spotatui v0.35+, lazydocker master, ratatui v0.30+
**Last Updated**: April 2026
