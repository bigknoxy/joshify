//! Application state - navigation, focus, UI overlays, and content state
//!
//! This module consolidates the UI-related state that was previously in main.rs

use super::load_coordinator::{LoadAction, LoadCoordinator, LoadResult};
use super::player_state::PlayerState;
use crate::album_art::AlbumArtCache;
use ratatui::layout::Rect;

/// Navigation items for the sidebar
///
/// Note: Search is intentionally omitted from the sidebar navigation.
/// Users can access search globally by pressing '/' from any screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NavItem {
    Home,
    Library,
    Playlists,
    LikedSongs,
}

impl NavItem {
    pub fn all() -> &'static [NavItem] {
        &[
            NavItem::Home,
            NavItem::Library,
            NavItem::Playlists,
            NavItem::LikedSongs,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            NavItem::Home => "Home",
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
#[derive(Clone, Default)]
pub enum ContentState {
    #[default]
    Home,
    /// New Home dashboard with recently played and jump back in
    HomeDashboard(super::home_state::HomeState),
    Loading(LoadAction),
    LoadingInProgress(LoadAction),
    LikedSongs(Vec<TrackListItem>),
    LikedSongsPage {
        tracks: Vec<TrackListItem>,
        total: u32,
        next_offset: Option<u32>,
    },
    Playlists(Vec<PlaylistListItem>),
    PlaylistTracks(String, Vec<TrackListItem>),
    SearchResults(String, Vec<TrackListItem>),
    Error(String),
    DeviceSelector(Vec<DeviceEntry>),
    /// Live search results (from debounce-triggered search)
    SearchResultsLive(Vec<TrackListItem>),
    /// Live search error
    SearchErrorLive(String),
    /// Library view with tabs
    Library {
        albums: Vec<AlbumListItem>,
        artists: Vec<ArtistListItem>,
        selected_tab: LibraryTab,
    },
    /// Album detail view with tracks
    AlbumDetail {
        album: AlbumListItem,
        tracks: Vec<TrackListItem>,
    },
    /// Artist detail view with top tracks
    ArtistDetail {
        artist: ArtistListItem,
    },
    /// Radio recommendations (for radio mode)
    RadioRecommendations(Vec<crate::playback::domain::QueueEntry>),
}

/// Library tab selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibraryTab {
    #[default]
    Albums,
    Artists,
}

/// Album list item for display
#[derive(Debug, Clone, PartialEq)]
pub struct AlbumListItem {
    pub name: String,
    pub artist: String,
    pub id: String,
    pub image_url: Option<String>,
    pub total_tracks: u32,
    pub release_year: Option<u32>,
}

/// Artist list item for display
#[derive(Debug, Clone, PartialEq)]
pub struct ArtistListItem {
    pub name: String,
    pub id: String,
    pub image_url: Option<String>,
    pub genres: Vec<String>,
    pub follower_count: Option<u32>,
}

/// Focus target for Tab navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusTarget {
    #[default]
    Sidebar,
    MainContent,
    PlayerBar,
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
    /// Navigation stack for drill-down browsing
    pub nav_stack: super::navigation_stack::NavigationStack,
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
            nav_stack: super::navigation_stack::NavigationStack::new(),
        }
    }

    /// Navigate back in the navigation stack (browser back)
    pub fn navigate_back(&mut self) -> bool {
        if let Some(entry) = self.nav_stack.back().cloned() {
            self.restore_from_nav_entry(entry);
            true
        } else {
            false
        }
    }

    /// Restore state from a navigation entry
    fn restore_from_nav_entry(&mut self, entry: super::navigation_stack::NavigationEntry) {
        use super::navigation_stack::NavigationEntry;
        match entry {
            NavigationEntry::Home => {
                self.content_state = ContentState::Home;
                self.selected_nav = NavItem::Home;
            }
            NavigationEntry::Library { albums, artists } => {
                self.content_state = ContentState::Library {
                    albums,
                    artists,
                    selected_tab: LibraryTab::Albums,
                };
                self.selected_nav = NavItem::Library;
            }
            NavigationEntry::AlbumDetail { album, tracks } => {
                self.content_state = ContentState::AlbumDetail { album, tracks };
                self.selected_nav = NavItem::Library;
            }
            NavigationEntry::ArtistDetail { artist } => {
                self.content_state = ContentState::ArtistDetail { artist };
                self.selected_nav = NavItem::Library;
            }
            NavigationEntry::Playlists(playlists) => {
                self.content_state = ContentState::Playlists(playlists);
                self.selected_nav = NavItem::Playlists;
            }
            NavigationEntry::PlaylistTracks { playlist, tracks } => {
                self.content_state = ContentState::PlaylistTracks(playlist.name, tracks);
                self.selected_nav = NavItem::Playlists;
            }
            NavigationEntry::LikedSongs(tracks) => {
                self.content_state = ContentState::LikedSongs(tracks);
                self.selected_nav = NavItem::LikedSongs;
            }
            NavigationEntry::SearchResults { query, tracks } => {
                self.content_state = ContentState::SearchResults(query, tracks);
            }
        }
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    /// Push current state to navigation stack
    pub fn push_current_to_nav_stack(&mut self) {
        use super::navigation_stack::NavigationEntry;
        let entry = match &self.content_state {
            ContentState::Home => Some(NavigationEntry::Home),
            ContentState::Library {
                albums, artists, ..
            } => Some(NavigationEntry::Library {
                albums: albums.clone(),
                artists: artists.clone(),
            }),
            ContentState::AlbumDetail { album, tracks } => Some(NavigationEntry::AlbumDetail {
                album: album.clone(),
                tracks: tracks.clone(),
            }),
            ContentState::ArtistDetail { artist } => Some(NavigationEntry::ArtistDetail {
                artist: artist.clone(),
            }),
            ContentState::Playlists(playlists) => {
                Some(NavigationEntry::Playlists(playlists.clone()))
            }
            ContentState::PlaylistTracks(name, tracks) => {
                // Find the playlist in playlists list, or create a dummy one
                let playlist = PlaylistListItem {
                    name: name.clone(),
                    id: "unknown".to_string(),
                    track_count: tracks.len() as u32,
                };
                Some(NavigationEntry::PlaylistTracks {
                    playlist,
                    tracks: tracks.clone(),
                })
            }
            ContentState::LikedSongs(tracks) => Some(NavigationEntry::LikedSongs(tracks.clone())),
            ContentState::SearchResults(query, tracks) => Some(NavigationEntry::SearchResults {
                query: query.clone(),
                tracks: tracks.clone(),
            }),
            _ => None, // Loading states, etc. don't get pushed
        };

        if let Some(e) = entry {
            self.nav_stack.push(e);
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

    /// Switch library tab
    pub fn switch_library_tab(&mut self) {
        if let ContentState::Library {
            albums,
            artists,
            selected_tab,
        } = &self.content_state
        {
            let new_tab = match selected_tab {
                LibraryTab::Albums => LibraryTab::Artists,
                LibraryTab::Artists => LibraryTab::Albums,
            };
            self.content_state = ContentState::Library {
                albums: albums.clone(),
                artists: artists.clone(),
                selected_tab: new_tab,
            };
            // Reset selection when switching tabs
            self.selected_index = 0;
            self.scroll_offset = 0;
        }
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
            NavItem::Library => {
                self.content_state = ContentState::Loading(LoadAction::LibraryAlbums);
                self.selected_index = 0;
                self.scroll_offset = 0;
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
