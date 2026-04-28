//! Layout cache for mouse hit testing
//!
//! Stores the rectangles of rendered UI components to enable accurate
//! mouse click detection. Updated each frame during rendering.

use crate::state::app_state::NavItem;
use ratatui::layout::Rect;

/// Cache of rendered layout areas for mouse hit testing
#[derive(Debug, Default, Clone)]
pub struct LayoutCache {
    /// Sidebar area (navigation)
    pub sidebar: Option<Rect>,
    /// Main content area (track lists, playlists)
    pub main_view: Option<Rect>,
    /// Player bar area (controls)
    pub player_bar: Option<Rect>,
    /// Search input overlay area
    pub search_input: Option<Rect>,
    /// Help overlay area
    pub help_overlay: Option<Rect>,
    /// Queue overlay area
    pub queue_overlay: Option<Rect>,
    /// Individual navigation item areas within sidebar
    pub nav_items: Vec<Rect>,
    /// Individual playlist item areas within main view
    pub playlist_items: Vec<Rect>,
    /// Individual track item areas within main view
    pub track_items: Vec<Rect>,
    /// Player control areas
    pub prev_button: Option<Rect>,
    pub play_pause_button: Option<Rect>,
    pub next_button: Option<Rect>,
    pub progress_bar: Option<Rect>,
    pub volume_bar: Option<Rect>,
    pub shuffle_button: Option<Rect>,
    pub repeat_button: Option<Rect>,
    pub queue_button: Option<Rect>,
}

impl LayoutCache {
    /// Create a new empty layout cache
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all cached areas (call at start of each frame)
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Check if a point is within a rectangle
    pub fn rect_contains(rect: Rect, x: u16, y: u16) -> bool {
        x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
    }

    /// Find which component area contains the given point
    pub fn area_at(&self, x: u16, y: u16) -> Option<ClickableArea> {
        // Check overlays first (they're on top)
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
        if let Some(queue) = self.queue_overlay {
            if Self::rect_contains(queue, x, y) {
                return Some(ClickableArea::QueueOverlay);
            }
        }

        // Check player controls (specific buttons)
        if let Some(prev) = self.prev_button {
            if Self::rect_contains(prev, x, y) {
                return Some(ClickableArea::PrevButton);
            }
        }
        if let Some(play) = self.play_pause_button {
            if Self::rect_contains(play, x, y) {
                return Some(ClickableArea::PlayPauseButton);
            }
        }
        if let Some(next) = self.next_button {
            if Self::rect_contains(next, x, y) {
                return Some(ClickableArea::NextButton);
            }
        }
        if let Some(progress) = self.progress_bar {
            if Self::rect_contains(progress, x, y) {
                return Some(ClickableArea::ProgressBar);
            }
        }
        if let Some(volume) = self.volume_bar {
            if Self::rect_contains(volume, x, y) {
                return Some(ClickableArea::VolumeBar);
            }
        }
        if let Some(shuffle) = self.shuffle_button {
            if Self::rect_contains(shuffle, x, y) {
                return Some(ClickableArea::ShuffleButton);
            }
        }
        if let Some(repeat) = self.repeat_button {
            if Self::rect_contains(repeat, x, y) {
                return Some(ClickableArea::RepeatButton);
            }
        }
        if let Some(queue) = self.queue_button {
            if Self::rect_contains(queue, x, y) {
                return Some(ClickableArea::QueueButton);
            }
        }

        // Check playlist items
        for (i, rect) in self.playlist_items.iter().enumerate() {
            if Self::rect_contains(*rect, x, y) {
                return Some(ClickableArea::PlaylistItem(i));
            }
        }

        // Check track items
        for (i, rect) in self.track_items.iter().enumerate() {
            if Self::rect_contains(*rect, x, y) {
                return Some(ClickableArea::TrackItem(i));
            }
        }

        // Check nav items
        for (i, rect) in self.nav_items.iter().enumerate() {
            if Self::rect_contains(*rect, x, y) {
                let all_items = NavItem::all();
                if i < all_items.len() {
                    return Some(ClickableArea::NavItem(all_items[i]));
                }
            }
        }

        // Check general areas (lowest priority)
        if let Some(sidebar) = self.sidebar {
            if Self::rect_contains(sidebar, x, y) {
                return Some(ClickableArea::Sidebar);
            }
        }
        if let Some(main) = self.main_view {
            if Self::rect_contains(main, x, y) {
                return Some(ClickableArea::MainView);
            }
        }
        if let Some(player) = self.player_bar {
            if Self::rect_contains(player, x, y) {
                return Some(ClickableArea::PlayerBar);
            }
        }

        None
    }
}

/// Represents a clickable area in the UI
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClickableArea {
    /// Navigation sidebar container
    Sidebar,
    /// Main content area container
    MainView,
    /// Player bar container
    PlayerBar,
    /// Search input overlay
    SearchInput,
    /// Help overlay
    HelpOverlay,
    /// Queue overlay
    QueueOverlay,
    /// Specific navigation item
    NavItem(NavItem),
    /// Specific playlist item at index
    PlaylistItem(usize),
    /// Specific track item at index
    TrackItem(usize),
    /// Previous track button
    PrevButton,
    /// Play/Pause button
    PlayPauseButton,
    /// Next track button
    NextButton,
    /// Progress bar (seek)
    ProgressBar,
    /// Volume bar
    VolumeBar,
    /// Shuffle toggle button
    ShuffleButton,
    /// Repeat mode button
    RepeatButton,
    /// Queue toggle button
    QueueButton,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(10, 10, 20, 10);

        assert!(LayoutCache::rect_contains(rect, 10, 10)); // Top-left corner
        assert!(LayoutCache::rect_contains(rect, 29, 19)); // Bottom-right corner
        assert!(LayoutCache::rect_contains(rect, 20, 15)); // Center

        assert!(!LayoutCache::rect_contains(rect, 5, 15)); // Outside left
        assert!(!LayoutCache::rect_contains(rect, 35, 15)); // Outside right
        assert!(!LayoutCache::rect_contains(rect, 15, 5)); // Outside top
        assert!(!LayoutCache::rect_contains(rect, 15, 25)); // Outside bottom
    }

    #[test]
    fn test_area_at_returns_none_outside_all_areas() {
        let cache = LayoutCache {
            sidebar: Some(Rect::new(0, 0, 20, 40)),
            main_view: Some(Rect::new(20, 0, 60, 34)),
            player_bar: Some(Rect::new(20, 34, 60, 6)),
            ..Default::default()
        };

        assert_eq!(cache.area_at(100, 100), None);
    }

    #[test]
    fn test_area_at_finds_sidebar() {
        let cache = LayoutCache {
            sidebar: Some(Rect::new(0, 0, 20, 40)),
            ..Default::default()
        };

        assert_eq!(cache.area_at(5, 5), Some(ClickableArea::Sidebar));
    }

    #[test]
    fn test_area_at_finds_specific_button() {
        let cache = LayoutCache {
            sidebar: Some(Rect::new(0, 0, 20, 40)),
            play_pause_button: Some(Rect::new(25, 36, 10, 2)),
            ..Default::default()
        };

        // Should find specific button, not general player bar
        assert_eq!(cache.area_at(30, 37), Some(ClickableArea::PlayPauseButton));
    }

    #[test]
    fn test_clear_resets_cache() {
        let mut cache = LayoutCache {
            sidebar: Some(Rect::new(0, 0, 20, 40)),
            main_view: Some(Rect::new(20, 0, 60, 34)),
            ..Default::default()
        };

        cache.clear();

        assert!(cache.sidebar.is_none());
        assert!(cache.main_view.is_none());
        assert!(cache.player_bar.is_none());
    }

    #[test]
    fn test_playlist_item_hit() {
        let cache = LayoutCache {
            playlist_items: vec![
                Rect::new(25, 5, 50, 1), // Item 0
                Rect::new(25, 6, 50, 1), // Item 1
                Rect::new(25, 7, 50, 1), // Item 2
            ],
            ..Default::default()
        };

        // Click on first playlist item
        assert_eq!(cache.area_at(30, 5), Some(ClickableArea::PlaylistItem(0)));

        // Click on second playlist item
        assert_eq!(cache.area_at(30, 6), Some(ClickableArea::PlaylistItem(1)));

        // Click on third playlist item
        assert_eq!(cache.area_at(30, 7), Some(ClickableArea::PlaylistItem(2)));

        // Click between items - should return None or fall through to main view
        assert_eq!(cache.area_at(30, 10), None);
    }

    #[test]
    fn test_track_item_hit() {
        let cache = LayoutCache {
            track_items: vec![
                Rect::new(25, 5, 50, 1), // Track 0
                Rect::new(25, 6, 50, 1), // Track 1
                Rect::new(25, 7, 50, 1), // Track 2
                Rect::new(25, 8, 50, 1), // Track 3
            ],
            ..Default::default()
        };

        // Click on first track
        assert_eq!(cache.area_at(30, 5), Some(ClickableArea::TrackItem(0)));

        // Click on middle track
        assert_eq!(cache.area_at(30, 7), Some(ClickableArea::TrackItem(2)));

        // Click on last track
        assert_eq!(cache.area_at(30, 8), Some(ClickableArea::TrackItem(3)));

        // Click outside track area
        assert_eq!(cache.area_at(100, 100), None);
    }

    #[test]
    fn test_control_buttons_hit() {
        let cache = LayoutCache {
            player_bar: Some(Rect::new(20, 34, 60, 6)),
            prev_button: Some(Rect::new(25, 36, 3, 2)),
            play_pause_button: Some(Rect::new(30, 36, 3, 2)),
            next_button: Some(Rect::new(35, 36, 3, 2)),
            shuffle_button: Some(Rect::new(40, 36, 3, 2)),
            repeat_button: Some(Rect::new(45, 36, 3, 2)),
            queue_button: Some(Rect::new(50, 36, 3, 2)),
            volume_bar: Some(Rect::new(60, 36, 15, 2)),
            progress_bar: Some(Rect::new(25, 34, 50, 1)),
            ..Default::default()
        };

        // Test prev button
        assert_eq!(cache.area_at(26, 37), Some(ClickableArea::PrevButton));

        // Test play/pause button
        assert_eq!(cache.area_at(31, 37), Some(ClickableArea::PlayPauseButton));

        // Test next button
        assert_eq!(cache.area_at(36, 37), Some(ClickableArea::NextButton));

        // Test shuffle button
        assert_eq!(cache.area_at(41, 37), Some(ClickableArea::ShuffleButton));

        // Test repeat button
        assert_eq!(cache.area_at(46, 37), Some(ClickableArea::RepeatButton));

        // Test queue button
        assert_eq!(cache.area_at(51, 37), Some(ClickableArea::QueueButton));

        // Test volume bar (not a button, but clickable area)
        assert_eq!(cache.area_at(65, 37), Some(ClickableArea::VolumeBar));

        // Test progress bar
        assert_eq!(cache.area_at(50, 34), Some(ClickableArea::ProgressBar));
    }

    #[test]
    fn test_overlay_priority() {
        // Overlays are checked in order: search → help → queue
        let cache = LayoutCache {
            sidebar: Some(Rect::new(0, 0, 20, 40)),
            main_view: Some(Rect::new(20, 0, 60, 34)),
            player_bar: Some(Rect::new(20, 34, 60, 6)),
            search_input: Some(Rect::new(10, 10, 60, 3)),
            help_overlay: Some(Rect::new(15, 15, 50, 20)),
            queue_overlay: Some(Rect::new(20, 20, 40, 15)),
            play_pause_button: Some(Rect::new(25, 36, 3, 2)),
            ..Default::default()
        };

        // Search overlay takes priority over sidebar/main view
        assert_eq!(cache.area_at(15, 11), Some(ClickableArea::SearchInput));

        // Help overlay takes priority (checked before queue)
        // Help: x=15-64, y=15-34
        assert_eq!(cache.area_at(20, 18), Some(ClickableArea::HelpOverlay));

        // Queue overlay (in area not covered by help)
        // Queue: x=20-59, y=20-34; Help: x=15-64, y=15-34
        // Queue is entirely within help's y range, so test outside help's x range
        // Actually queue is inside help, so help takes priority
        // Test point outside help but in queue's extended area - but queue is smaller
        // Let's test a point in queue that's also in help - help wins
        assert_eq!(cache.area_at(30, 25), Some(ClickableArea::HelpOverlay));

        // Outside overlays - should find player button
        assert_eq!(cache.area_at(26, 37), Some(ClickableArea::PlayPauseButton));

        // Outside everything
        assert_eq!(cache.area_at(100, 100), None);
    }

    #[test]
    fn test_overlay_priority_help_over_queue() {
        // Help overlay is checked before queue, so it takes priority when they overlap
        let cache = LayoutCache {
            help_overlay: Some(Rect::new(10, 10, 60, 20)), // x: 10-69, y: 10-29
            queue_overlay: Some(Rect::new(15, 15, 50, 15)), // x: 15-64, y: 15-29
            ..Default::default()
        };

        // In overlapping area - help takes priority (checked first)
        assert_eq!(cache.area_at(20, 20), Some(ClickableArea::HelpOverlay));

        // In help overlay but outside queue
        // Queue starts at x=15, so x=12 is in help but not queue
        assert_eq!(cache.area_at(12, 12), Some(ClickableArea::HelpOverlay));

        // Queue is entirely contained within help's bounds
        // help: x=10-69, y=10-29; queue: x=15-64, y=15-29
        // There's no point in queue that's outside help
        // So we can only test overlapping (help wins) or help-only areas
    }

    #[test]
    fn test_nav_item_hit_detection() {
        use crate::state::app_state::NavItem;

        let cache = LayoutCache {
            sidebar: Some(Rect::new(0, 0, 20, 40)),
            nav_items: vec![
                Rect::new(0, 16, 20, 1), // Home
                Rect::new(0, 17, 20, 1), // Library (Search removed - accessible via '/')
                Rect::new(0, 18, 20, 1), // Playlists
                Rect::new(0, 19, 20, 1), // Liked Songs
            ],
            ..Default::default()
        };

        assert_eq!(
            cache.area_at(5, 16),
            Some(ClickableArea::NavItem(NavItem::Home))
        );
        // Search removed from sidebar - accessible via '/' key
        assert_eq!(
            cache.area_at(5, 17),
            Some(ClickableArea::NavItem(NavItem::Library))
        );
        assert_eq!(
            cache.area_at(5, 18),
            Some(ClickableArea::NavItem(NavItem::Playlists))
        );
        assert_eq!(
            cache.area_at(5, 19),
            Some(ClickableArea::NavItem(NavItem::LikedSongs))
        );
    }

    #[test]
    fn test_playlist_item_edge_positions() {
        let cache = LayoutCache {
            playlist_items: vec![Rect::new(25, 5, 50, 1)],
            ..Default::default()
        };

        // Left edge of playlist item
        assert_eq!(cache.area_at(25, 5), Some(ClickableArea::PlaylistItem(0)));

        // Right edge of playlist item (x=25+50-1=74)
        assert_eq!(cache.area_at(74, 5), Some(ClickableArea::PlaylistItem(0)));

        // Just outside left edge
        assert_eq!(cache.area_at(24, 5), None);

        // Just outside right edge
        assert_eq!(cache.area_at(75, 5), None);
    }

    #[test]
    fn test_track_item_edge_positions() {
        let cache = LayoutCache {
            track_items: vec![Rect::new(25, 5, 50, 1)],
            ..Default::default()
        };

        // Left edge
        assert_eq!(cache.area_at(25, 5), Some(ClickableArea::TrackItem(0)));

        // Right edge
        assert_eq!(cache.area_at(74, 5), Some(ClickableArea::TrackItem(0)));

        // Outside
        assert_eq!(cache.area_at(24, 5), None);
        assert_eq!(cache.area_at(75, 5), None);
    }

    #[test]
    fn test_control_button_edge_positions() {
        let cache = LayoutCache {
            play_pause_button: Some(Rect::new(30, 36, 3, 2)),
            ..Default::default()
        };

        // Top-left corner
        assert_eq!(cache.area_at(30, 36), Some(ClickableArea::PlayPauseButton));

        // Bottom-right corner (x=30+3-1=32, y=36+2-1=37)
        assert_eq!(cache.area_at(32, 37), Some(ClickableArea::PlayPauseButton));

        // Just outside
        assert_eq!(cache.area_at(29, 36), None);
        assert_eq!(cache.area_at(33, 37), None);
        assert_eq!(cache.area_at(30, 35), None);
        assert_eq!(cache.area_at(32, 38), None);
    }

    #[test]
    fn test_empty_cache_returns_none() {
        let cache = LayoutCache::new();

        assert_eq!(cache.area_at(0, 0), None);
        assert_eq!(cache.area_at(50, 50), None);
        assert_eq!(cache.area_at(100, 100), None);
    }

    #[test]
    fn test_sidebar_fallback_when_no_nav_item_hit() {
        use crate::state::app_state::NavItem;

        // Sidebar with nav items, but click in sidebar area between nav items
        let cache = LayoutCache {
            sidebar: Some(Rect::new(0, 0, 20, 40)),
            nav_items: vec![Rect::new(0, 16, 20, 1), Rect::new(0, 17, 20, 1)],
            ..Default::default()
        };

        // Click in sidebar but not on a nav item - should return Sidebar
        assert_eq!(cache.area_at(5, 5), Some(ClickableArea::Sidebar));

        // Click on nav item - should return specific nav item
        assert_eq!(
            cache.area_at(5, 16),
            Some(ClickableArea::NavItem(NavItem::Home))
        );
    }

    #[test]
    fn test_main_view_fallback() {
        let cache = LayoutCache {
            main_view: Some(Rect::new(20, 0, 60, 34)),
            ..Default::default()
        };

        // Click in main view but not on any specific item
        assert_eq!(cache.area_at(30, 10), Some(ClickableArea::MainView));

        // Click outside main view
        assert_eq!(cache.area_at(10, 10), None);
    }

    #[test]
    fn test_player_bar_fallback() {
        let cache = LayoutCache {
            player_bar: Some(Rect::new(20, 34, 60, 6)),
            ..Default::default()
        };

        // Click in player bar but not on any specific control
        // Player bar x range: 20 to 79 (20+60-1)
        assert_eq!(cache.area_at(79, 35), Some(ClickableArea::PlayerBar));
    }
}
