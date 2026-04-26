//! Mouse event handling for interactive TUI elements
//!
//! Main entry point for mouse events that routes to component-specific handlers.
//! Uses LayoutCache for hit detection and returns actions for the caller to apply.

use crate::ui::layout_cache::{ClickableArea, LayoutCache};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

/// Mouse event result for the application to handle
#[derive(Debug, Clone, PartialEq)]
pub enum MouseAction {
    /// Select and activate a nav item
    SelectNavItem(crate::state::app_state::NavItem),
    /// Select a playlist at index
    SelectPlaylist(usize),
    /// Open playlist at index (double-click)
    OpenPlaylist(usize),
    /// Select a track at index
    SelectTrack(usize),
    /// Play track at index with context (double-click)
    PlayTrack(usize),
    /// Play selected track
    PlaySelected,
    /// Toggle play/pause
    TogglePlayPause,
    /// Skip to next track
    SkipNext,
    /// Skip to previous track
    SkipPrevious,
    /// Toggle shuffle
    ToggleShuffle,
    /// Cycle repeat mode
    CycleRepeat,
    /// Seek to position (percentage 0-100)
    Seek(u8),
    /// Set volume (percentage 0-100)
    SetVolume(u8),
    /// Adjust volume by delta
    AdjustVolume(i32),
    /// Toggle queue overlay
    ToggleQueue,
    /// Scroll list up
    ScrollUp,
    /// Scroll list down
    ScrollDown,
    /// Close overlay
    CloseOverlay,
    /// Change focus area
    SetFocus(crate::state::app_state::FocusTarget),
    /// No action
    None,
}

/// State for tracking double-clicks
#[derive(Debug, Default)]
pub struct MouseState {
    /// Last click position (x, y)
    pub last_click_pos: Option<(u16, u16)>,
    /// Last click time
    pub last_click_time: Option<std::time::Instant>,
    /// Double-click threshold in milliseconds
    pub double_click_threshold: u64,
}

impl MouseState {
    /// Create new mouse state
    pub fn new() -> Self {
        Self {
            last_click_pos: None,
            last_click_time: None,
            double_click_threshold: 300, // 300ms
        }
    }

    /// Check if current click is a double-click
    pub fn is_double_click(&mut self, x: u16, y: u16) -> bool {
        let now = std::time::Instant::now();
        let is_double = if let Some((last_x, last_y)) = self.last_click_pos {
            if let Some(last_time) = self.last_click_time {
                let elapsed = now.duration_since(last_time).as_millis() as u64;
                let is_same_pos = last_x.abs_diff(x) <= 2 && last_y.abs_diff(y) <= 2;
                elapsed < self.double_click_threshold && is_same_pos
            } else {
                false
            }
        } else {
            false
        };

        self.last_click_pos = Some((x, y));
        self.last_click_time = Some(now);

        is_double
    }
}

/// Main entry point for mouse events.
/// Returns the action to perform. The caller owns all state and must apply the action.
pub fn handle_mouse_event(
    mouse: MouseEvent,
    layout_cache: &LayoutCache,
    mouse_state: &mut MouseState,
) -> MouseAction {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            handle_left_click(mouse.column, mouse.row, layout_cache, mouse_state)
        }
        MouseEventKind::Down(MouseButton::Right) => MouseAction::None,
        MouseEventKind::ScrollUp => handle_scroll_up(mouse.column, mouse.row, layout_cache),
        MouseEventKind::ScrollDown => handle_scroll_down(mouse.column, mouse.row, layout_cache),
        _ => MouseAction::None,
    }
}

/// Handle left mouse button click
pub fn handle_left_click(
    x: u16,
    y: u16,
    layout_cache: &LayoutCache,
    mouse_state: &mut MouseState,
) -> MouseAction {
    let is_double_click = mouse_state.is_double_click(x, y);

    match layout_cache.area_at(x, y) {
        Some(ClickableArea::NavItem(nav)) => MouseAction::SelectNavItem(nav),
        Some(ClickableArea::PlaylistItem(index)) => {
            if is_double_click {
                MouseAction::OpenPlaylist(index)
            } else {
                MouseAction::SelectPlaylist(index)
            }
        }
        Some(ClickableArea::TrackItem(index)) => {
            if is_double_click {
                MouseAction::PlayTrack(index)
            } else {
                MouseAction::SelectTrack(index)
            }
        }
        Some(ClickableArea::Sidebar) => {
            MouseAction::SetFocus(crate::state::app_state::FocusTarget::Sidebar)
        }
        Some(ClickableArea::MainView) => {
            MouseAction::SetFocus(crate::state::app_state::FocusTarget::MainContent)
        }
        Some(ClickableArea::PlayPauseButton) => MouseAction::TogglePlayPause,
        Some(ClickableArea::NextButton) => MouseAction::SkipNext,
        Some(ClickableArea::PrevButton) => MouseAction::SkipPrevious,
        Some(ClickableArea::ShuffleButton) => MouseAction::ToggleShuffle,
        Some(ClickableArea::RepeatButton) => MouseAction::CycleRepeat,
        Some(ClickableArea::QueueButton) => MouseAction::ToggleQueue,
        Some(ClickableArea::ProgressBar) => layout_cache
            .progress_bar
            .and_then(|rect| calculate_percentage_from_x(x, rect))
            .map(MouseAction::Seek)
            .unwrap_or(MouseAction::None),
        Some(ClickableArea::VolumeBar) => layout_cache
            .volume_bar
            .and_then(|rect| calculate_percentage_from_x(x, rect))
            .map(MouseAction::SetVolume)
            .unwrap_or(MouseAction::None),
        Some(ClickableArea::PlayerBar) => {
            MouseAction::SetFocus(crate::state::app_state::FocusTarget::PlayerBar)
        }
        Some(ClickableArea::SearchInput) => MouseAction::None,
        Some(ClickableArea::HelpOverlay) | Some(ClickableArea::QueueOverlay) => {
            MouseAction::CloseOverlay
        }
        None => MouseAction::None,
    }
}

/// Handle scroll up event
pub fn handle_scroll_up(
    x: u16,
    y: u16,
    layout_cache: &LayoutCache,
) -> MouseAction {
    match layout_cache.area_at(x, y) {
        Some(ClickableArea::Sidebar)
        | Some(ClickableArea::NavItem(_))
        | Some(ClickableArea::MainView)
        | Some(ClickableArea::TrackItem(_))
        | Some(ClickableArea::QueueOverlay) => MouseAction::ScrollUp,
        Some(ClickableArea::PlayerBar) | Some(ClickableArea::VolumeBar) => {
            MouseAction::AdjustVolume(5)
        }
        _ => MouseAction::None,
    }
}

/// Handle scroll down event
pub fn handle_scroll_down(
    x: u16,
    y: u16,
    layout_cache: &LayoutCache,
) -> MouseAction {
    match layout_cache.area_at(x, y) {
        Some(ClickableArea::Sidebar)
        | Some(ClickableArea::NavItem(_))
        | Some(ClickableArea::MainView)
        | Some(ClickableArea::TrackItem(_))
        | Some(ClickableArea::QueueOverlay) => MouseAction::ScrollDown,
        Some(ClickableArea::PlayerBar) | Some(ClickableArea::VolumeBar) => {
            MouseAction::AdjustVolume(-5)
        }
        _ => MouseAction::None,
    }
}

/// Calculate percentage (0-100) from X position within a rectangle
fn calculate_percentage_from_x(x: u16, rect: Rect) -> Option<u8> {
    if x < rect.x || x >= rect.x + rect.width {
        return None;
    }

    let inner_width = rect.width.saturating_sub(2) as f32; // Account for borders
    if inner_width <= 0.0 {
        return Some(0);
    }

    let relative_x = x.saturating_sub(rect.x + 1) as f32;
    let percentage = (relative_x / inner_width * 100.0) as u8;

    Some(percentage.clamp(0, 100))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_percentage_from_x() {
        let rect = Rect::new(10, 10, 20, 3); // Inner width = 18

        // Left edge (x=11 is position 0 in inner area)
        assert_eq!(calculate_percentage_from_x(11, rect), Some(0));

        // Right edge (x=28 is position 17 in inner area, 17/18 = 94%)
        let result = calculate_percentage_from_x(28, rect);
        assert!(result.is_some());
        assert!(result.unwrap() >= 90);

        // Center (x=19 is position 8 in inner area, 8/18 = 44%)
        let result = calculate_percentage_from_x(19, rect);
        assert!(result.is_some());
        assert!(result.unwrap() > 40 && result.unwrap() < 50);

        // Outside area
        assert_eq!(calculate_percentage_from_x(5, rect), None);
        assert_eq!(calculate_percentage_from_x(35, rect), None);
    }

    #[test]
    fn test_calculate_percentage_edge_cases() {
        // Very narrow rect
        let narrow = Rect::new(10, 10, 2, 3);
        assert_eq!(calculate_percentage_from_x(11, narrow), Some(0));

        // Single column rect
        let single = Rect::new(10, 10, 1, 3);
        assert_eq!(calculate_percentage_from_x(10, single), Some(0));
    }

    #[test]
    fn test_mouse_state_double_click() {
        let mut state = MouseState::new();
        state.double_click_threshold = 500; // 500ms for testing

        // First click
        assert!(!state.is_double_click(10, 10));

        // Click nearby quickly - should be double click
        assert!(state.is_double_click(11, 11));

        // Reset by creating new state (simulating time passing)
        let mut state2 = MouseState::new();
        state2.double_click_threshold = 500;
        assert!(!state2.is_double_click(10, 10));

        // Click far away - not a double click
        assert!(!state2.is_double_click(100, 100));
    }

    #[test]
    fn test_handle_left_click_nav_item() {
        let cache = LayoutCache {
            sidebar: Some(Rect::new(0, 0, 20, 40)),
            ..Default::default()
        };
        let mut mouse_state = MouseState::new();

        let action = handle_left_click(5, 5, &cache, &mut mouse_state);
        assert!(
            matches!(action, MouseAction::SetFocus(crate::state::app_state::FocusTarget::Sidebar)),
            "Expected SetFocus(Sidebar), got {:?}",
            action
        );
    }

    #[test]
    fn test_handle_left_click_overlay_closes() {
        let cache = LayoutCache {
            help_overlay: Some(Rect::new(10, 10, 60, 20)),
            ..Default::default()
        };
        let mut mouse_state = MouseState::new();

        let action = handle_left_click(15, 15, &cache, &mut mouse_state);
        assert_eq!(action, MouseAction::CloseOverlay);
    }

    #[test]
    fn test_handle_scroll_up_volume() {
        let cache = LayoutCache {
            player_bar: Some(Rect::new(20, 34, 60, 6)),
            ..Default::default()
        };

        let action = handle_scroll_up(25, 36, &cache);
        assert_eq!(action, MouseAction::AdjustVolume(5));
    }

    #[test]
    fn test_handle_scroll_down_main() {
        let cache = LayoutCache {
            main_view: Some(Rect::new(20, 0, 60, 34)),
            ..Default::default()
        };

        let action = handle_scroll_down(25, 10, &cache);
        assert_eq!(action, MouseAction::ScrollDown);
    }

    #[test]
    fn test_handle_mouse_event_ignores_other_buttons() {
        let cache = LayoutCache::default();
        let mut mouse_state = MouseState::new();

        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        let middle_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Middle),
            column: 10,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };

        let action = handle_mouse_event(middle_click, &cache, &mut mouse_state);
        assert_eq!(action, MouseAction::None);
    }

    #[test]
    fn test_double_click_on_track_plays() {
        let cache = LayoutCache {
            track_items: vec![Rect::new(20, 5, 60, 1)],
            main_view: Some(Rect::new(20, 0, 60, 34)),
            ..Default::default()
        };
        let mut mouse_state = MouseState::new();
        mouse_state.double_click_threshold = 500;

        // First click - select
        let first_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 25,
            row: 5,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        let action1 = handle_mouse_event(first_click, &cache, &mut mouse_state);
        assert!(matches!(action1, MouseAction::SelectTrack(0)));

        // Second click quickly - play
        let second_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 25,
            row: 5,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        let action2 = handle_mouse_event(second_click, &cache, &mut mouse_state);
        assert!(matches!(action2, MouseAction::PlayTrack(0)));
    }

    #[test]
    fn test_double_click_on_playlist_opens() {
        let cache = LayoutCache {
            playlist_items: vec![Rect::new(0, 5, 20, 1)],
            sidebar: Some(Rect::new(0, 0, 20, 40)),
            ..Default::default()
        };
        let mut mouse_state = MouseState::new();
        mouse_state.double_click_threshold = 500;

        // First click - select
        let first_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: 5,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        let action1 = handle_mouse_event(first_click, &cache, &mut mouse_state);
        assert!(matches!(action1, MouseAction::SelectPlaylist(0)));

        // Second click quickly - open
        let second_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: 5,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        let action2 = handle_mouse_event(second_click, &cache, &mut mouse_state);
        assert!(matches!(action2, MouseAction::OpenPlaylist(0)));
    }

    #[test]
    fn test_single_click_after_delay_is_not_double() {
        let cache = LayoutCache {
            track_items: vec![Rect::new(20, 5, 60, 1)],
            main_view: Some(Rect::new(20, 0, 60, 34)),
            ..Default::default()
        };
        let mut mouse_state = MouseState::new();
        mouse_state.double_click_threshold = 100; // Very short threshold

        // First click
        let first_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 25,
            row: 5,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        let action1 = handle_mouse_event(first_click, &cache, &mut mouse_state);
        assert!(matches!(action1, MouseAction::SelectTrack(0)));

        // Wait longer than threshold (simulated by creating new state)
        let mut mouse_state2 = MouseState::new();
        mouse_state2.double_click_threshold = 100;

        // Second click after "delay" - should be select, not play
        let second_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 25,
            row: 5,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };
        let action2 = handle_mouse_event(second_click, &cache, &mut mouse_state2);
        assert!(matches!(action2, MouseAction::SelectTrack(0)));
    }

    #[test]
    fn test_double_click_detection() {
        let mut state = MouseState::new();
        state.double_click_threshold = 300; // 300ms threshold

        // First click at position (10, 10)
        let is_double_1 = state.is_double_click(10, 10);
        assert!(!is_double_1, "First click should not be a double-click");

        // Second click at same position within threshold - should be double-click
        let is_double_2 = state.is_double_click(10, 10);
        assert!(is_double_2, "Second click at same position should be double-click");

        // Third click immediately - should also be double-click
        let is_double_3 = state.is_double_click(10, 10);
        assert!(is_double_3, "Third click should also be double-click");
    }

    #[test]
    fn test_double_click_position_tolerance() {
        let mut state = MouseState::new();
        state.double_click_threshold = 300;

        // First click
        state.is_double_click(10, 10);

        // Click within tolerance (±2 pixels)
        assert!(state.is_double_click(11, 11), "Should be double-click within tolerance");

        // Reset state
        let mut state2 = MouseState::new();
        state2.double_click_threshold = 300;
        state2.is_double_click(10, 10);

        // Click outside tolerance (>2 pixels away)
        assert!(!state2.is_double_click(13, 13), "Should not be double-click outside tolerance");
        assert!(!state2.is_double_click(10, 13), "Should not be double-click with y outside tolerance");
        assert!(!state2.is_double_click(13, 10), "Should not be double-click with x outside tolerance");
    }

    #[test]
    fn test_single_click_vs_double() {
        let mut state = MouseState::new();
        state.double_click_threshold = 300;

        // First click is always single
        assert!(!state.is_double_click(50, 50));

        // Quick second click at same position is double
        assert!(state.is_double_click(50, 50));

        // After double-click, next click at different position is single
        assert!(!state.is_double_click(60, 60));

        // But quick click at same new position is double
        assert!(state.is_double_click(60, 60));
    }

    #[test]
    fn test_double_click_threshold_timing() {
        use std::time::{Duration, Instant};

        let mut state = MouseState::new();
        state.double_click_threshold = 100; // 100ms for precise testing

        // First click
        state.is_double_click(10, 10);

        // Manually set last click time to simulate time passing
        state.last_click_time = Some(Instant::now() - Duration::from_millis(150));

        // Click after threshold - should not be double-click
        assert!(!state.is_double_click(10, 10), "Click after threshold should not be double");
    }

    #[test]
    fn test_scroll_actions_volume_bar() {
        let cache = LayoutCache {
            volume_bar: Some(Rect::new(60, 36, 15, 2)),
            player_bar: Some(Rect::new(20, 34, 60, 6)),
            ..Default::default()
        };

        // Scroll up on volume bar
        let action_up = handle_scroll_up(65, 37, &cache);
        assert_eq!(action_up, MouseAction::AdjustVolume(5));

        // Scroll down on volume bar
        let action_down = handle_scroll_down(65, 37, &cache);
        assert_eq!(action_down, MouseAction::AdjustVolume(-5));
    }

    #[test]
    fn test_scroll_actions_sidebar() {
        let cache = LayoutCache {
            sidebar: Some(Rect::new(0, 0, 20, 40)),
            nav_items: vec![Rect::new(0, 16, 20, 1)],
            ..Default::default()
        };

        // Scroll up in sidebar
        let action_up = handle_scroll_up(10, 20, &cache);
        assert_eq!(action_up, MouseAction::ScrollUp);

        // Scroll down in sidebar
        let action_down = handle_scroll_down(10, 20, &cache);
        assert_eq!(action_down, MouseAction::ScrollDown);

        // Scroll on nav item
        let action_nav_up = handle_scroll_up(10, 16, &cache);
        assert_eq!(action_nav_up, MouseAction::ScrollUp);

        let action_nav_down = handle_scroll_down(10, 16, &cache);
        assert_eq!(action_nav_down, MouseAction::ScrollDown);
    }

    #[test]
    fn test_scroll_actions_main_view() {
        let cache = LayoutCache {
            main_view: Some(Rect::new(20, 0, 60, 34)),
            track_items: vec![Rect::new(25, 5, 50, 1)],
            ..Default::default()
        };

        // Scroll up in main view
        let action_up = handle_scroll_up(30, 10, &cache);
        assert_eq!(action_up, MouseAction::ScrollUp);

        // Scroll down in main view
        let action_down = handle_scroll_down(30, 10, &cache);
        assert_eq!(action_down, MouseAction::ScrollDown);

        // Scroll on track item
        let action_track_up = handle_scroll_up(30, 5, &cache);
        assert_eq!(action_track_up, MouseAction::ScrollUp);

        let action_track_down = handle_scroll_down(30, 5, &cache);
        assert_eq!(action_track_down, MouseAction::ScrollDown);
    }

    #[test]
    fn test_scroll_actions_queue_overlay() {
        let cache = LayoutCache {
            queue_overlay: Some(Rect::new(20, 20, 40, 15)),
            ..Default::default()
        };

        // Scroll up in queue overlay
        let action_up = handle_scroll_up(30, 25, &cache);
        assert_eq!(action_up, MouseAction::ScrollUp);

        // Scroll down in queue overlay
        let action_down = handle_scroll_down(30, 25, &cache);
        assert_eq!(action_down, MouseAction::ScrollDown);
    }

    #[test]
    fn test_scroll_actions_outside_scrollable_areas() {
        let cache = LayoutCache {
            player_bar: Some(Rect::new(20, 34, 60, 6)),
            progress_bar: Some(Rect::new(25, 34, 50, 1)),
            ..Default::default()
        };

        // Scroll on progress bar (not a scrollable area)
        let action_up = handle_scroll_up(50, 34, &cache);
        assert_eq!(action_up, MouseAction::None);

        let action_down = handle_scroll_down(50, 34, &cache);
        assert_eq!(action_down, MouseAction::None);

        // Scroll outside any defined area
        let action_outside_up = handle_scroll_up(100, 100, &cache);
        assert_eq!(action_outside_up, MouseAction::None);

        let action_outside_down = handle_scroll_down(100, 100, &cache);
        assert_eq!(action_outside_down, MouseAction::None);
    }

    #[test]
    fn test_handle_left_click_playlist_item() {
        let cache = LayoutCache {
            playlist_items: vec![Rect::new(25, 5, 50, 1)],
            ..Default::default()
        };
        let mut mouse_state = MouseState::new();

        let action = handle_left_click(30, 5, &cache, &mut mouse_state);
        assert_eq!(action, MouseAction::SelectPlaylist(0));
    }

    #[test]
    fn test_handle_left_click_track_item() {
        let cache = LayoutCache {
            track_items: vec![Rect::new(25, 5, 50, 1)],
            ..Default::default()
        };
        let mut mouse_state = MouseState::new();

        let action = handle_left_click(30, 5, &cache, &mut mouse_state);
        assert_eq!(action, MouseAction::SelectTrack(0));
    }

    #[test]
    fn test_handle_left_click_all_control_buttons() {
        let cache = LayoutCache {
            player_bar: Some(Rect::new(20, 34, 60, 6)),
            prev_button: Some(Rect::new(25, 36, 3, 2)),
            play_pause_button: Some(Rect::new(30, 36, 3, 2)),
            next_button: Some(Rect::new(35, 36, 3, 2)),
            shuffle_button: Some(Rect::new(40, 36, 3, 2)),
            repeat_button: Some(Rect::new(45, 36, 3, 2)),
            queue_button: Some(Rect::new(50, 36, 3, 2)),
            ..Default::default()
        };
        let mut mouse_state = MouseState::new();

        assert_eq!(
            handle_left_click(26, 37, &cache, &mut mouse_state),
            MouseAction::SkipPrevious
        );
        assert_eq!(
            handle_left_click(31, 37, &cache, &mut mouse_state),
            MouseAction::TogglePlayPause
        );
        assert_eq!(
            handle_left_click(36, 37, &cache, &mut mouse_state),
            MouseAction::SkipNext
        );
        assert_eq!(
            handle_left_click(41, 37, &cache, &mut mouse_state),
            MouseAction::ToggleShuffle
        );
        assert_eq!(
            handle_left_click(46, 37, &cache, &mut mouse_state),
            MouseAction::CycleRepeat
        );
        assert_eq!(
            handle_left_click(51, 37, &cache, &mut mouse_state),
            MouseAction::ToggleQueue
        );
    }

    #[test]
    fn test_handle_left_click_progress_bar_seeks() {
        let cache = LayoutCache {
            player_bar: Some(Rect::new(20, 34, 60, 6)),
            progress_bar: Some(Rect::new(25, 34, 50, 1)),
            ..Default::default()
        };
        let mut mouse_state = MouseState::new();

        // Click at start of progress bar (should seek to ~0%)
        let action_start = handle_left_click(26, 34, &cache, &mut mouse_state);
        assert!(matches!(action_start, MouseAction::Seek(p) if p < 10));

        // Click at end of progress bar (should seek to ~100%)
        let action_end = handle_left_click(73, 34, &cache, &mut mouse_state);
        assert!(matches!(action_end, MouseAction::Seek(p) if p > 90));

        // Click at middle of progress bar (should seek to ~50%)
        let action_middle = handle_left_click(50, 34, &cache, &mut mouse_state);
        assert!(matches!(action_middle, MouseAction::Seek(p) if p > 40 && p < 60));
    }

    #[test]
    fn test_handle_left_click_volume_bar_sets_volume() {
        let cache = LayoutCache {
            player_bar: Some(Rect::new(20, 34, 60, 6)),
            volume_bar: Some(Rect::new(60, 36, 15, 2)),
            ..Default::default()
        };
        let mut mouse_state = MouseState::new();

        // Click at start of volume bar (should set to ~0%)
        let action_start = handle_left_click(61, 37, &cache, &mut mouse_state);
        assert!(matches!(action_start, MouseAction::SetVolume(p) if p < 10));

        // Click at end of volume bar (should set to ~100%)
        let action_end = handle_left_click(73, 37, &cache, &mut mouse_state);
        assert!(matches!(action_end, MouseAction::SetVolume(p) if p > 90));
    }

    #[test]
    fn test_handle_left_click_focus_targets() {
        let cache = LayoutCache {
            sidebar: Some(Rect::new(0, 0, 20, 40)),
            main_view: Some(Rect::new(20, 0, 60, 34)),
            player_bar: Some(Rect::new(20, 34, 60, 6)),
            ..Default::default()
        };
        let mut mouse_state = MouseState::new();

        use crate::state::app_state::FocusTarget;

        // Click in sidebar
        assert_eq!(
            handle_left_click(10, 10, &cache, &mut mouse_state),
            MouseAction::SetFocus(FocusTarget::Sidebar)
        );

        // Click in main view
        assert_eq!(
            handle_left_click(30, 10, &cache, &mut mouse_state),
            MouseAction::SetFocus(FocusTarget::MainContent)
        );

        // Click in player bar (not on specific control)
        // Player bar x range: 20 to 79 (20+60-1)
        assert_eq!(
            handle_left_click(79, 35, &cache, &mut mouse_state),
            MouseAction::SetFocus(FocusTarget::PlayerBar)
        );
    }

    #[test]
    fn test_handle_mouse_event_right_click_ignored() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

        let cache = LayoutCache::default();
        let mut mouse_state = MouseState::new();

        let right_click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Right),
            column: 10,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };

        let action = handle_mouse_event(right_click, &cache, &mut mouse_state);
        assert_eq!(action, MouseAction::None);
    }

    #[test]
    fn test_handle_mouse_event_mouse_release_ignored() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

        let cache = LayoutCache::default();
        let mut mouse_state = MouseState::new();

        let release = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 10,
            row: 10,
            modifiers: crossterm::event::KeyModifiers::empty(),
        };

        let action = handle_mouse_event(release, &cache, &mut mouse_state);
        assert_eq!(action, MouseAction::None);
    }

    #[test]
    fn test_mouse_state_default() {
        let state = MouseState::default();
        assert!(state.last_click_pos.is_none());
        assert!(state.last_click_time.is_none());
        // Default derive gives 0 for u64; use new() for 300ms threshold
        assert_eq!(state.double_click_threshold, 0);
    }

    #[test]
    fn test_mouse_state_new() {
        let state = MouseState::new();
        assert!(state.last_click_pos.is_none());
        assert!(state.last_click_time.is_none());
        assert_eq!(state.double_click_threshold, 300);
    }
}
