//! Library state management
//!
//! Placeholder for Phase 3 library features.
//! Caches liked songs and playlists for quick access.

use super::app_state::{PlaylistListItem, TrackListItem};

/// Library state
#[derive(Debug, Clone, Default)]
pub struct LibraryState {
    /// Cached liked songs
    pub liked_songs: Vec<TrackListItem>,
    /// Cached playlists
    pub playlists: Vec<PlaylistListItem>,
    /// Last fetched timestamp for liked songs
    pub liked_songs_fetched_at: Option<u64>,
    /// Last fetched timestamp for playlists
    pub playlists_fetched_at: Option<u64>,
    /// Liked songs total count from Spotify
    pub liked_songs_total: Option<u32>,
}

impl LibraryState {
    pub fn new() -> Self {
        Self {
            liked_songs: Vec::new(),
            playlists: Vec::new(),
            liked_songs_fetched_at: None,
            playlists_fetched_at: None,
            liked_songs_total: None,
        }
    }

    /// Check if liked songs cache is stale (older than 5 minutes)
    pub fn liked_songs_stale(&self) -> bool {
        self.liked_songs_fetched_at.is_none_or(|ts| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|now| now.as_secs() - ts > 300)
                .unwrap_or(true)
        })
    }

    /// Check if playlists cache is stale (older than 5 minutes)
    pub fn playlists_stale(&self) -> bool {
        self.playlists_fetched_at.is_none_or(|ts| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|now| now.as_secs() - ts > 300)
                .unwrap_or(true)
        })
    }

    /// Update liked songs cache
    pub fn update_liked_songs(&mut self, songs: Vec<TrackListItem>, total: Option<u32>) {
        self.liked_songs = songs;
        self.liked_songs_total = total;
        self.liked_songs_fetched_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs());
    }

    /// Update playlists cache
    pub fn update_playlists(&mut self, playlists: Vec<PlaylistListItem>) {
        self.playlists = playlists;
        self.playlists_fetched_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs());
    }
}
