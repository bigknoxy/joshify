//! Async task coordination to prevent race conditions and duplicate spawns
//!
//! This module replaces the fragile string-based state machine with a proper
//! enum-based approach that tracks active tasks and rejects stale results.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Actions that can be loaded asynchronously
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LoadAction {
    LikedSongs,
    LikedSongsPage { offset: u32 },
    Playlists,
    PlaylistTracks { name: String, id: String },
    Search { query: String },
    Devices,
    /// Home dashboard data - recently played and jump back in
    HomeData,
    /// User's saved albums
    LibraryAlbums,
    /// User's followed artists
    LibraryArtists,
    /// Album tracks (for detail view)
    AlbumTracks { album_id: String, name: String },
    /// Artist top tracks (for detail view)
    ArtistTopTracks { artist_id: String, name: String },
}

/// Result from an async load operation
#[derive(Debug, Clone)]
pub struct LoadResult<T> {
    pub action: LoadAction,
    pub data: T,
    pub sequence: u64,
}

/// Coordinator for async tasks
///
/// Tracks active tasks, prevents duplicates, and provides sequence numbers
/// to reject stale results.
pub struct LoadCoordinator {
    /// Currently running tasks by action type
    active_tasks: HashMap<LoadAction, JoinHandle<()>>,
    /// Monotonically increasing sequence number
    sequence: AtomicU64,
    /// Last completed sequence for each action type
    completed_sequences: HashMap<LoadAction, u64>,
}

impl LoadCoordinator {
    pub fn new() -> Self {
        Self {
            active_tasks: HashMap::new(),
            sequence: AtomicU64::new(0),
            completed_sequences: HashMap::new(),
        }
    }

    /// Get the next sequence number
    pub fn next_sequence(&self) -> u64 {
        self.sequence.fetch_add(1, Ordering::SeqCst)
    }

    /// Check if an action is currently being loaded
    pub fn is_loading(&self, action: &LoadAction) -> bool {
        self.active_tasks.contains_key(action)
    }

    /// Register a task as active
    pub fn register_task(&mut self, action: LoadAction, handle: JoinHandle<()>) {
        // Cancel any existing task for this action
        if let Some(existing) = self.active_tasks.remove(&action) {
            existing.abort();
        }
        self.active_tasks.insert(action, handle);
    }

    /// Mark a task as completed
    pub fn mark_completed(&mut self, action: &LoadAction, sequence: u64) {
        self.active_tasks.remove(action);
        self.completed_sequences.insert(action.clone(), sequence);
    }

    /// Check if a result is stale (older than the last completed sequence)
    pub fn is_stale(&self, action: &LoadAction, sequence: u64) -> bool {
        self.completed_sequences
            .get(action)
            .is_some_and(|&last| sequence < last)
    }

    /// Cancel all pending tasks
    pub fn cancel_all(&mut self) {
        for (_, handle) in self.active_tasks.drain() {
            handle.abort();
        }
    }

    /// Cancel tasks for a specific action
    pub fn cancel(&mut self, action: &LoadAction) {
        if let Some(handle) = self.active_tasks.remove(action) {
            handle.abort();
        }
    }
}

impl Default for LoadCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Wrapper for sending load results back to the main loop
pub struct LoadSender<T> {
    tx: mpsc::Sender<LoadResult<T>>,
    action: LoadAction,
    sequence: u64,
}

impl<T: Send + 'static> LoadSender<T> {
    pub fn new(tx: mpsc::Sender<LoadResult<T>>, action: LoadAction, sequence: u64) -> Self {
        Self {
            tx,
            action,
            sequence,
        }
    }

    /// Send a result, ignoring if channel is closed
    pub async fn send(&self, data: T) {
        let _ = self
            .tx
            .send(LoadResult {
                action: self.action.clone(),
                data,
                sequence: self.sequence,
            })
            .await;
    }
}
