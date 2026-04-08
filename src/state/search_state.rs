//! Search state management with debounce and caching

use crate::state::app_state::TrackListItem;

/// Search state with debounce and result caching
#[derive(Debug, Clone, Default)]
pub struct SearchState {
    /// Current search query
    pub query: String,
    /// Whether search overlay is active
    pub is_active: bool,
    /// Cursor position in characters (for proper Unicode handling)
    pub cursor_pos: usize,
    /// Selected result index
    pub selected_index: usize,
    /// Scroll offset for results
    pub scroll_offset: usize,
    /// Last time a search was triggered (ms)
    pub last_search_time_ms: u64,
    /// Debounce cooldown in ms (300ms recommended)
    pub debounce_ms: u64,
    /// Whether a search is currently in progress
    pub is_loading: bool,
    /// Cached search results
    pub results: Vec<TrackListItem>,
    /// Error message if search failed
    pub error: Option<String>,
    /// Pending query that needs to be searched (after debounce)
    pub pending_query: Option<String>,
    /// Auto-clear error after this timestamp (ms)
    pub error_display_until_ms: Option<u64>,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            is_active: false,
            cursor_pos: 0,
            selected_index: 0,
            scroll_offset: 0,
            last_search_time_ms: 0,
            debounce_ms: 300,
            is_loading: false,
            results: Vec::new(),
            error: None,
            pending_query: None,
            error_display_until_ms: None,
        }
    }

    /// Activate search overlay
    pub fn activate(&mut self) {
        self.is_active = true;
        self.query.clear();
        self.cursor_pos = 0;
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.results.clear();
        self.error = None;
        self.is_loading = false;
        self.pending_query = None;
        self.error_display_until_ms = None;
    }

    /// Deactivate search overlay
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }

    /// Insert a character at cursor position
    pub fn insert_char(&mut self, c: char) {
        let byte_idx = self.byte_index();
        self.query.insert(byte_idx, c);
        self.cursor_pos += 1;
        self.results.clear();
        self.error = None;
        self.is_loading = false;
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    /// Delete character before cursor
    pub fn delete_char(&mut self) {
        if self.cursor_pos > 0 {
            let byte_idx = self.byte_index();
            self.query.remove(byte_idx.saturating_sub(1));
            self.cursor_pos = self.cursor_pos.saturating_sub(1);
            self.results.clear();
            self.error = None;
            self.selected_index = 0;
            self.scroll_offset = 0;
        }
    }

    /// Move cursor left
    pub fn move_cursor_left(&mut self) {
        self.cursor_pos = self.cursor_pos.saturating_sub(1);
    }

    /// Move cursor right
    pub fn move_cursor_right(&mut self) {
        self.cursor_pos = (self.cursor_pos + 1).min(self.query.chars().count());
    }

    /// Get byte index from character position
    pub fn byte_index(&self) -> usize {
        self.query
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.cursor_pos)
            .unwrap_or(self.query.len())
    }

    /// Check if debounce cooldown has elapsed
    pub fn should_search(&self, current_time_ms: u64) -> bool {
        if self.query.is_empty() {
            return false;
        }
        if self.is_loading {
            return false;
        }
        current_time_ms.saturating_sub(self.last_search_time_ms) >= self.debounce_ms
    }

    /// Mark that a search has been initiated
    pub fn mark_search_started(&mut self, time_ms: u64) {
        self.last_search_time_ms = time_ms;
        self.is_loading = true;
        self.pending_query = Some(self.query.clone());
    }

    /// Set search results
    pub fn set_results(&mut self, results: Vec<TrackListItem>) {
        self.results = results;
        self.is_loading = false;
        self.pending_query = None;
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    /// Set search error
    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
        self.is_loading = false;
        self.pending_query = None;
        self.results.clear();
    }

    /// Move selection up
    pub fn select_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down
    pub fn select_down(&mut self, max_items: usize) {
        if self.selected_index < max_items.saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    /// Get the currently selected track
    pub fn selected_track(&self) -> Option<&TrackListItem> {
        self.results.get(self.selected_index)
    }

    /// Reset cursor to beginning
    pub fn reset_cursor(&mut self) {
        self.cursor_pos = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_state_defaults() {
        let state = SearchState::new();
        assert!(!state.is_active);
        assert!(state.query.is_empty());
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.selected_index, 0);
        assert!(!state.is_loading);
        assert!(state.results.is_empty());
        assert!(state.error.is_none());
    }

    #[test]
    fn test_activate_clears_state() {
        let mut state = SearchState::new();
        state.query = "test".to_string();
        state.is_active = true;
        state.is_loading = true;
        state.error = Some("error".to_string());

        state.activate();

        assert!(state.is_active);
        assert!(state.query.is_empty());
        assert!(!state.is_loading);
        assert!(state.error.is_none());
        assert!(state.results.is_empty());
    }

    #[test]
    fn test_insert_char() {
        let mut state = SearchState::new();
        state.insert_char('h');
        state.insert_char('i');

        assert_eq!(state.query, "hi");
        assert_eq!(state.cursor_pos, 2);
    }

    #[test]
    fn test_delete_char() {
        let mut state = SearchState::new();
        state.insert_char('h');
        state.insert_char('i');
        state.delete_char();

        assert_eq!(state.query, "h");
        assert_eq!(state.cursor_pos, 1);
    }

    #[test]
    fn test_delete_char_at_start_does_nothing() {
        let mut state = SearchState::new();
        state.delete_char();

        assert!(state.query.is_empty());
        assert_eq!(state.cursor_pos, 0);
    }

    #[test]
    fn test_cursor_movement() {
        let mut state = SearchState::new();
        state.insert_char('a');
        state.insert_char('b');
        state.insert_char('c');

        state.move_cursor_left();
        assert_eq!(state.cursor_pos, 2);

        state.move_cursor_left();
        assert_eq!(state.cursor_pos, 1);

        state.move_cursor_right();
        assert_eq!(state.cursor_pos, 2);

        state.move_cursor_right();
        assert_eq!(state.cursor_pos, 3);
    }

    #[test]
    fn test_byte_index() {
        let mut state = SearchState::new();
        state.query = "hello".to_string();
        state.cursor_pos = 2;

        assert_eq!(state.byte_index(), 2);
    }

    #[test]
    fn test_should_search_respects_debounce() {
        let mut state = SearchState::new();
        state.query = "test".to_string();
        state.last_search_time_ms = 0;
        state.debounce_ms = 300;

        // Not enough time has passed
        assert!(!state.should_search(200));

        // Enough time has passed
        assert!(state.should_search(300));
        assert!(state.should_search(400));
    }

    #[test]
    fn test_should_search_empty_query() {
        let state = SearchState::new();
        assert!(!state.should_search(1000));
    }

    #[test]
    fn test_should_search_while_loading() {
        let mut state = SearchState::new();
        state.query = "test".to_string();
        state.is_loading = true;
        state.last_search_time_ms = 0;

        assert!(!state.should_search(1000));
    }

    #[test]
    fn test_mark_search_started() {
        let mut state = SearchState::new();
        state.query = "test".to_string();
        state.mark_search_started(500);

        assert_eq!(state.last_search_time_ms, 500);
        assert!(state.is_loading);
        assert_eq!(state.pending_query, Some("test".to_string()));
    }

    #[test]
    fn test_set_results() {
        let mut state = SearchState::new();
        state.is_loading = true;
        state.set_results(vec![TrackListItem {
            name: "Test".to_string(),
            artist: "Artist".to_string(),
            uri: "spotify:track:123".to_string(),
        }]);

        assert!(!state.is_loading);
        assert_eq!(state.results.len(), 1);
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn test_selection_bounds() {
        let mut state = SearchState::new();

        // Add some results
        for i in 0..5 {
            state.results.push(TrackListItem {
                name: format!("Track {}", i),
                artist: "Artist".to_string(),
                uri: format!("spotify:track:{}", i),
            });
        }

        state.select_up(); // Should stay at 0
        assert_eq!(state.selected_index, 0);

        state.select_down(5);
        assert_eq!(state.selected_index, 1);

        state.select_down(5);
        state.select_down(5);
        state.select_down(5);
        state.select_down(5);
        assert_eq!(state.selected_index, 4);

        state.select_down(5); // Should stay at 4
        assert_eq!(state.selected_index, 4);
    }

    #[test]
    fn test_selected_track() {
        let mut state = SearchState::new();
        assert!(state.selected_track().is_none());

        state.results.push(TrackListItem {
            name: "Test".to_string(),
            artist: "Artist".to_string(),
            uri: "spotify:track:123".to_string(),
        });

        assert!(state.selected_track().is_some());
        assert_eq!(state.selected_track().unwrap().name, "Test");
    }

    #[test]
    fn test_deactivate() {
        let mut state = SearchState::new();
        state.is_active = true;
        state.deactivate();
        assert!(!state.is_active);
    }

    #[test]
    fn test_pending_query_cleared_on_results() {
        let mut state = SearchState::new();
        state.pending_query = Some("test".to_string());
        state.set_results(vec![TrackListItem {
            name: "Test".to_string(),
            artist: "Artist".to_string(),
            uri: "spotify:track:123".to_string(),
        }]);
        assert!(state.pending_query.is_none());
    }

    #[test]
    fn test_pending_query_cleared_on_error() {
        let mut state = SearchState::new();
        state.pending_query = Some("test".to_string());
        state.set_error("error".to_string());
        assert!(state.pending_query.is_none());
    }

    #[test]
    fn test_insert_char_resets_loading() {
        let mut state = SearchState::new();
        state.is_loading = true;
        state.insert_char('a');
        assert!(!state.is_loading);
    }

    #[test]
    fn test_insert_char_clears_results() {
        let mut state = SearchState::new();
        state.results.push(TrackListItem {
            name: "Test".to_string(),
            artist: "Artist".to_string(),
            uri: "spotify:track:123".to_string(),
        });
        state.insert_char('a');
        assert!(state.results.is_empty());
    }

    #[test]
    fn test_insert_in_middle() {
        let mut state = SearchState::new();
        state.query = "hllo".to_string();
        state.cursor_pos = 1;
        state.insert_char('e');

        assert_eq!(state.query, "hello");
        assert_eq!(state.cursor_pos, 2);
    }
}
