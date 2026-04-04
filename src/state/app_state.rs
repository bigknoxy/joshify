//! Application state - navigation, focus, UI overlays, and content state
//!
//! This module consolidates the UI-related state that was previously in main.rs

use super::load_coordinator::{LoadAction, LoadCoordinator, LoadResult};
use super::player_state::PlayerState;
use crate::album_art::AlbumArtCache;
use ratatui::layout::Rect;

/// Navigation items for the sidebar
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavItem {
    Home,
    Search,
    Library,
    Playlists,
    LikedSongs,
}

impl NavItem {
    pub fn all() -> &'static [NavItem] {
        &[
            NavItem::Home,
            NavItem::Search,
            NavItem::Library,
            NavItem::Playlists,
            NavItem::LikedSongs,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            NavItem::Home => "Home",
            NavItem::Search => "Search",
            NavItem::Library => "Library",
            NavItem::Playlists => "Playlists",
            NavItem::LikedSongs => "Liked Songs",
        }
    }
}

/// Track list item for display
#[derive(Debug, Clone, PartialEq)]
pub struct TrackListItem {
    pub name: String,
    pub artist: String,
    pub uri: String,
}

/// Playlist list item for display
#[derive(Debug, Clone, PartialEq)]
pub struct PlaylistListItem {
    pub name: String,
    pub id: String,
    pub track_count: u32,
}

/// Represents a selectable playback device in the UI
#[derive(Clone)]
pub enum DeviceEntry {
    /// Local playback on this machine
    ThisDevice { active: bool },
    /// Remote Spotify Connect device
    Remote(rspotify::model::Device),
}

/// Content state for main view
#[derive(Clone)]
pub enum ContentState {
    Home,
    Loading(LoadAction),
    LoadingInProgress(LoadAction),
    LikedSongs(Vec<TrackListItem>),
    Playlists(Vec<PlaylistListItem>),
    PlaylistTracks(String, Vec<TrackListItem>),
    SearchResults(String, Vec<TrackListItem>),
    Error(String),
    DeviceSelector(Vec<DeviceEntry>),
}

impl Default for ContentState {
    fn default() -> Self {
        Self::Home
    }
}

/// Focus target for Tab navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTarget {
    Sidebar,
    MainContent,
    PlayerBar,
}

impl Default for FocusTarget {
    fn default() -> Self {
        Self::Sidebar
    }
}

/// Application state coordinator
pub struct AppState {
    /// Current navigation selection
    pub selected_nav: NavItem,
    /// Whether we're authenticated
    pub is_authenticated: bool,
    /// Current playback state
    pub player_state: PlayerState,
    /// Status message (shown at top)
    pub status_message: Option<String>,
    /// Current focus target for Tab navigation
    pub focus: FocusTarget,
    /// Show queue overlay
    pub show_queue: bool,
    /// Help message lines
    pub help_lines: Option<Vec<String>>,
    /// Last frame area (for mouse handling)
    pub area: Option<Rect>,
    /// Main content state
    pub content_state: ContentState,
    /// Current selection index in content list
    pub selected_index: usize,
    /// Scroll offset for long lists
    pub scroll_offset: usize,
    /// Search input buffer
    pub search_query: String,
    /// Whether we're in search input mode
    pub is_searching: bool,
    /// Album art cache
    pub album_art_cache: AlbumArtCache,
    /// Async task coordinator
    pub load_coordinator: LoadCoordinator,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            selected_nav: NavItem::Home,
            is_authenticated: false,
            player_state: PlayerState::default(),
            status_message: None,
            focus: FocusTarget::default(),
            show_queue: false,
            help_lines: None,
            area: None,
            content_state: ContentState::Home,
            selected_index: 0,
            scroll_offset: 0,
            search_query: String::new(),
            is_searching: false,
            album_art_cache: AlbumArtCache::new(),
            load_coordinator: LoadCoordinator::new(),
        }
    }

    /// Cycle focus to next target
    pub fn focus_next(&mut self) {
        self.focus = match self.focus {
            FocusTarget::Sidebar => FocusTarget::MainContent,
            FocusTarget::MainContent => FocusTarget::PlayerBar,
            FocusTarget::PlayerBar => FocusTarget::Sidebar,
        };
    }

    /// Cycle focus to previous target
    pub fn focus_previous(&mut self) {
        self.focus = match self.focus {
            FocusTarget::Sidebar => FocusTarget::PlayerBar,
            FocusTarget::MainContent => FocusTarget::Sidebar,
            FocusTarget::PlayerBar => FocusTarget::MainContent,
        };
    }

    /// Handle search input character
    pub fn search_input(&mut self, c: char) {
        self.search_query.push(c);
        self.content_state = ContentState::Loading(LoadAction::Search {
            query: format!("Search: {}", self.search_query),
        });
    }

    /// Handle search backspace
    pub fn search_backspace(&mut self) {
        self.search_query.pop();
        self.content_state = ContentState::Loading(if self.search_query.is_empty() {
            LoadAction::Search {
                query: "Type search query...".to_string(),
            }
        } else {
            LoadAction::Search {
                query: format!("Search: {}", self.search_query),
            }
        });
    }

    /// Start search
    pub fn start_search(&mut self) {
        if !self.search_query.is_empty() {
            let query = self.search_query.clone();
            self.content_state = ContentState::Loading(LoadAction::Search { query });
            self.selected_index = 0;
            self.scroll_offset = 0;
        }
        self.is_searching = false;
    }

    /// Cancel search
    pub fn cancel_search(&mut self) {
        self.is_searching = false;
        self.content_state = ContentState::Home;
    }

    /// Select nav item
    pub fn select_nav(&mut self, nav: NavItem) {
        self.selected_nav = nav;
        match nav {
            NavItem::LikedSongs => {
                self.content_state = ContentState::Loading(LoadAction::LikedSongs);
                self.selected_index = 0;
                self.scroll_offset = 0;
            }
            NavItem::Playlists => {
                self.content_state = ContentState::Loading(LoadAction::Playlists);
                self.selected_index = 0;
                self.scroll_offset = 0;
            }
            NavItem::Home => {
                self.content_state = ContentState::Home;
            }
            NavItem::Search => {
                self.content_state = ContentState::Loading(LoadAction::Search {
                    query: "Type to search...".to_string(),
                });
            }
            NavItem::Library => {
                self.content_state = ContentState::Loading(LoadAction::Search {
                    query: "Loading library...".to_string(),
                });
            }
        }
    }

    /// Scroll list down
    pub fn scroll_down(&mut self, len: usize) {
        if len > 0 {
            self.selected_index = (self.selected_index + 1).min(len - 1);
            if self.selected_index >= self.scroll_offset + 10 {
                self.scroll_offset = self.selected_index - 9;
            }
        }
    }

    /// Scroll list up
    pub fn scroll_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            if self.selected_index < self.scroll_offset {
                self.scroll_offset = self.selected_index;
            }
        }
    }

    /// Check if a load action should be spawned
    pub fn should_spawn_load(&self, action: &LoadAction) -> bool {
        matches!(self.content_state, ContentState::Loading(ref a) if a == action)
            && !self.load_coordinator.is_loading(action)
    }

    /// Mark content as loading in progress
    pub fn set_loading_in_progress(&mut self, action: LoadAction) {
        self.content_state = ContentState::LoadingInProgress(action);
    }

    /// Apply load result if not stale
    pub fn apply_load_result<T, F>(&mut self, result: LoadResult<T>, apply: F)
    where
        F: FnOnce(&mut Self, T),
    {
        if !self
            .load_coordinator
            .is_stale(&result.action, result.sequence)
        {
            apply(self, result.data);
            self.load_coordinator
                .mark_completed(&result.action, result.sequence);
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
