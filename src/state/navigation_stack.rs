//! Navigation stack for drill-down browsing (browser-like back/forward navigation)
//!
//! This enables users to:
//! - Navigate from Library -> Albums -> Album Tracks (drill down)
//! - Press Backspace to go back (like browser back button)
//! - See breadcrumb trail of their navigation path

use super::app_state::{AlbumListItem, ArtistListItem, PlaylistListItem, TrackListItem};

/// Represents a location in the navigation hierarchy that can be returned to
#[derive(Debug, Clone, PartialEq)]
pub enum NavigationEntry {
    /// Home dashboard
    Home,
    /// Library view with albums/artists
    Library { albums: Vec<AlbumListItem>, artists: Vec<ArtistListItem> },
    /// Album detail with tracks
    AlbumDetail { album: AlbumListItem, tracks: Vec<TrackListItem> },
    /// Artist detail with top tracks/albums
    ArtistDetail { artist: ArtistListItem },
    /// Playlists list
    Playlists(Vec<PlaylistListItem>),
    /// Playlist tracks
    PlaylistTracks { playlist: PlaylistListItem, tracks: Vec<TrackListItem> },
    /// Liked songs
    LikedSongs(Vec<TrackListItem>),
    /// Search results
    SearchResults { query: String, tracks: Vec<TrackListItem> },
}

/// Navigation stack for drill-down browsing
#[derive(Debug, Default)]
pub struct NavigationStack {
    /// History of visited locations
    history: Vec<NavigationEntry>,
    /// Current position in history
    current_index: usize,
    /// Maximum history size
    max_size: usize,
}

impl NavigationStack {
    /// Create a new navigation stack
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            current_index: 0,
            max_size: 50, // Keep last 50 locations
        }
    }

    /// Push a new entry onto the stack
    pub fn push(&mut self, entry: NavigationEntry) {
        // Remove any forward history if we're in the middle of the stack
        if self.current_index < self.history.len() {
            self.history.truncate(self.current_index);
        }

        // Add new entry
        self.history.push(entry);
        self.current_index = self.history.len();

        // Keep stack within limits
        if self.history.len() > self.max_size {
            self.history.remove(0);
            self.current_index = self.current_index.saturating_sub(1);
        }
    }

    /// Navigate back one level
    pub fn back(&mut self) -> Option<&NavigationEntry> {
        if self.can_go_back() {
            self.current_index = self.current_index.saturating_sub(1);
            self.history.get(self.current_index.saturating_sub(1))
        } else {
            None
        }
    }

    /// Navigate forward one level (if user went back previously)
    pub fn forward(&mut self) -> Option<&NavigationEntry> {
        if self.can_go_forward() {
            self.current_index += 1;
            self.history.get(self.current_index.saturating_sub(1))
        } else {
            None
        }
    }

    /// Check if we can go back
    pub fn can_go_back(&self) -> bool {
        self.current_index > 1 // Must have at least one entry behind current
    }

    /// Check if we can go forward
    pub fn can_go_forward(&self) -> bool {
        self.current_index < self.history.len()
    }

    /// Get current location
    pub fn current(&self) -> Option<&NavigationEntry> {
        if self.current_index > 0 && self.current_index <= self.history.len() {
            self.history.get(self.current_index.saturating_sub(1))
        } else {
            None
        }
    }

    /// Get breadcrumb trail for display
    pub fn breadcrumb(&self) -> Vec<String> {
        let mut trail = Vec::new();
        for (i, entry) in self.history.iter().enumerate() {
            if i >= self.current_index {
                break;
            }
            trail.push(entry_name(entry));
        }
        trail
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.history.clear();
        self.current_index = 0;
    }

    /// Get current depth (number of entries in history)
    pub fn depth(&self) -> usize {
        self.current_index
    }

    /// Reset to initial state
    pub fn reset(&mut self) {
        self.history.clear();
        self.current_index = 0;
    }
}

/// Get a display name for a navigation entry
fn entry_name(entry: &NavigationEntry) -> String {
    match entry {
        NavigationEntry::Home => "Home".to_string(),
        NavigationEntry::Library { .. } => "Library".to_string(),
        NavigationEntry::AlbumDetail { album, .. } => format!("{}", album.name),
        NavigationEntry::ArtistDetail { artist, .. } => format!("{}", artist.name),
        NavigationEntry::Playlists(_) => "Playlists".to_string(),
        NavigationEntry::PlaylistTracks { playlist, .. } => playlist.name.clone(),
        NavigationEntry::LikedSongs(_) => "Liked Songs".to_string(),
        NavigationEntry::SearchResults { query, .. } => format!("Search: {}", query),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_album(name: &str) -> AlbumListItem {
        AlbumListItem {
            name: name.to_string(),
            artist: "Artist".to_string(),
            id: "123".to_string(),
            image_url: None,
            total_tracks: 10,
            release_year: Some(2023),
        }
    }

    fn make_playlist(name: &str) -> PlaylistListItem {
        PlaylistListItem {
            name: name.to_string(),
            id: "abc".to_string(),
            track_count: 20,
        }
    }

    #[test]
    fn test_empty_stack() {
        let stack = NavigationStack::new();
        assert!(!stack.can_go_back());
        assert!(!stack.can_go_forward());
        assert_eq!(stack.current(), None);
    }

    #[test]
    fn test_push_and_current() {
        let mut stack = NavigationStack::new();
        stack.push(NavigationEntry::Home);
        
        assert_eq!(stack.current_index, 1);
        assert!(matches!(stack.current(), Some(NavigationEntry::Home)));
    }

    #[test]
    fn test_back_navigation() {
        let mut stack = NavigationStack::new();
        
        // Navigate: Home -> Library -> Album
        stack.push(NavigationEntry::Home);
        stack.push(NavigationEntry::Library { 
            albums: vec![], 
            artists: vec![] 
        });
        stack.push(NavigationEntry::AlbumDetail { 
            album: make_album("Test Album"), 
            tracks: vec![] 
        });

        assert_eq!(stack.current_index, 3);
        assert!(stack.can_go_back());
        
        // Go back
        let prev = stack.back();
        assert!(prev.is_some());
        assert_eq!(stack.current_index, 2);
        
        // Go back again
        let prev2 = stack.back();
        assert!(prev2.is_some());
        assert_eq!(stack.current_index, 1);
        
        // Can't go back further
        assert!(!stack.can_go_back());
    }

    #[test]
    fn test_forward_navigation() {
        let mut stack = NavigationStack::new();
        
        stack.push(NavigationEntry::Home);
        stack.push(NavigationEntry::Library { albums: vec![], artists: vec![] });
        
        // Go back
        stack.back();
        assert_eq!(stack.current_index, 1);
        
        // Go forward
        assert!(stack.can_go_forward());
        let next = stack.forward();
        assert!(next.is_some());
        assert_eq!(stack.current_index, 2);
    }

    #[test]
    fn test_breadcrumb() {
        let mut stack = NavigationStack::new();
        
        stack.push(NavigationEntry::Home);
        stack.push(NavigationEntry::Library { albums: vec![], artists: vec![] });
        stack.push(NavigationEntry::AlbumDetail { 
            album: make_album("My Album"), 
            tracks: vec![] 
        });

        let breadcrumb = stack.breadcrumb();
        assert_eq!(breadcrumb, vec!["Home", "Library", "My Album"]);
    }

    #[test]
    fn test_push_clears_forward_history() {
        let mut stack = NavigationStack::new();
        
        // Navigate: Home -> Library -> Album
        stack.push(NavigationEntry::Home);
        stack.push(NavigationEntry::Library { albums: vec![], artists: vec![] });
        stack.push(NavigationEntry::AlbumDetail { album: make_album("Old Album"), tracks: vec![] });
        
        // Go back to Library
        stack.back();
        assert_eq!(stack.current_index, 2);
        
        // Push new entry - should clear forward history
        stack.push(NavigationEntry::AlbumDetail { album: make_album("New Album"), tracks: vec![] });
        
        // Can't go forward anymore
        assert!(!stack.can_go_forward());
        assert_eq!(stack.depth(), 3);
    }

    #[test]
    fn test_max_size_limit() {
        let mut stack = NavigationStack::new();
        stack.max_size = 3;
        
        stack.push(NavigationEntry::Home);
        stack.push(NavigationEntry::Library { albums: vec![], artists: vec![] });
        stack.push(NavigationEntry::LikedSongs(vec![]));
        stack.push(NavigationEntry::Playlists(vec![]));
        
        // Should have removed Home
        assert_eq!(stack.depth(), 3);
        let breadcrumb = stack.breadcrumb();
        assert_eq!(breadcrumb.len(), 3);
    }

    #[test]
    fn test_entry_names() {
        assert_eq!(entry_name(&NavigationEntry::Home), "Home");
        
        let album = make_album("Test Album");
        assert_eq!(entry_name(&NavigationEntry::AlbumDetail { album, tracks: vec![] }), "Test Album");
        
        let playlist = make_playlist("My Playlist");
        assert_eq!(entry_name(&NavigationEntry::PlaylistTracks { playlist, tracks: vec![] }), "My Playlist");
        
        assert_eq!(entry_name(&NavigationEntry::SearchResults { query: "foo".to_string(), tracks: vec![] }), "Search: foo");
    }
}
