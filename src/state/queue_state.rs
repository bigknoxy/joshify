//! Queue state management
//!
//! Placeholder for Phase 3 smart queue features.
//! Currently just wraps the Spotify queue data.

use rspotify::model::CurrentUserQueue;

/// Queue state
#[derive(Debug, Clone, Default)]
pub struct QueueState {
    /// Current queue from Spotify
    pub spotify_queue: Option<CurrentUserQueue>,
    /// Locally queued tracks (for Phase 3 smart queue)
    pub local_queue: Vec<QueueEntry>,
    /// Radio mode enabled (Phase 3)
    pub radio_mode: bool,
    /// Queue persistence path (Phase 3)
    pub persistence_path: Option<String>,
}

/// Queue entry with metadata
#[derive(Debug, Clone)]
pub struct QueueEntry {
    pub uri: String,
    pub name: String,
    pub artist: String,
    pub added_by_user: bool,
    pub is_recommendation: bool,
}

impl QueueState {
    pub fn new() -> Self {
        Self {
            spotify_queue: None,
            local_queue: Vec::new(),
            radio_mode: false,
            persistence_path: None,
        }
    }

    /// Clear the local queue
    pub fn clear(&mut self) {
        self.local_queue.clear();
    }

    /// Add a track to the local queue
    pub fn add(&mut self, entry: QueueEntry) {
        self.local_queue.push(entry);
    }

    /// Get the next track from local queue
    pub fn next(&mut self) -> Option<QueueEntry> {
        if !self.local_queue.is_empty() {
            Some(self.local_queue.remove(0))
        } else {
            None
        }
    }
}

impl Default for QueueEntry {
    fn default() -> Self {
        Self {
            uri: String::new(),
            name: String::new(),
            artist: String::new(),
            added_by_user: false,
            is_recommendation: false,
        }
    }
}
