//! Queue state management
//!
//! Manages both Spotify server-side queue and local queue for track ordering.
//! Wraps the new `PlaybackQueue` domain model for backward compatibility.

use rspotify::model::CurrentUserQueue;

use crate::playback::domain::{PlaybackQueue, QueueEntry as DomainQueueEntry};

/// Queue state
#[derive(Debug, Clone)]
pub struct QueueState {
    /// Current queue from Spotify
    pub spotify_queue: Option<CurrentUserQueue>,
    /// Locally queued tracks — kept in sync with playback_queue for backward compat.
    pub local_queue: Vec<QueueEntry>,
    /// New domain-based playback queue for context-aware operations
    playback_queue: PlaybackQueue,
    /// Radio mode enabled (Phase 3)
    pub radio_mode: bool,
    /// Queue persistence path (Phase 3)
    pub persistence_path: Option<String>,
}

impl Default for QueueState {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueState {
    pub fn new() -> Self {
        Self {
            spotify_queue: None,
            local_queue: Vec::new(),
            playback_queue: PlaybackQueue::new(),
            radio_mode: true, // Enabled by default for continuous playback
            persistence_path: None,
        }
    }

    /// Clear the local queue
    pub fn clear(&mut self) {
        self.local_queue.clear();
        self.playback_queue.reset();
    }

    /// Add a track to the local queue (end)
    pub fn add(&mut self, entry: QueueEntry) {
        self.local_queue.push(entry.clone());
        self.playback_queue.add_to_up_next(entry.into());
    }

    /// Get the next track from local queue (FIFO)
    pub fn next_track(&mut self) -> Option<QueueEntry> {
        if !self.local_queue.is_empty() {
            let entry = self.local_queue.remove(0);
            // Also advance the playback queue
            let _ = self.playback_queue.advance();
            Some(entry)
        } else {
            None
        }
    }

    /// Check if queue is empty (both local and Spotify)
    pub fn is_empty(&self) -> bool {
        self.local_queue.is_empty()
            && self
                .spotify_queue
                .as_ref()
                .is_none_or(|q| q.queue.is_empty())
    }

    /// Total number of items in queue (local + Spotify)
    pub fn total_count(&self) -> usize {
        let spotify_count = self.spotify_queue.as_ref().map_or(0, |q| q.queue.len());
        self.local_queue.len() + spotify_count
    }

    /// Update the Spotify queue data
    pub fn update_spotify_queue(&mut self, queue: CurrentUserQueue) {
        self.spotify_queue = Some(queue);
    }

    /// Access the underlying PlaybackQueue for advanced operations
    pub fn playback_queue(&self) -> &PlaybackQueue {
        &self.playback_queue
    }

    /// Mutable access to the underlying PlaybackQueue
    pub fn playback_queue_mut(&mut self) -> &mut PlaybackQueue {
        &mut self.playback_queue
    }

    /// Sync the local_queue Vec from the PlaybackQueue's up_next entries.
    /// Call this after direct PlaybackQueue mutations to keep local_queue in sync.
    pub fn sync_from_playback_queue(&mut self) {
        self.local_queue = self
            .playback_queue
            .up_next_entries()
            .iter()
            .cloned()
            .map(DomainQueueEntry::into)
            .collect();
    }
}

/// Queue entry with metadata — backward-compatible wrapper.
/// Converts to/from domain::QueueEntry.
#[derive(Debug, Clone, Default)]
pub struct QueueEntry {
    pub uri: String,
    pub name: String,
    pub artist: String,
    pub added_by_user: bool,
    pub is_recommendation: bool,
}

impl From<DomainQueueEntry> for QueueEntry {
    fn from(entry: DomainQueueEntry) -> Self {
        Self {
            uri: entry.uri,
            name: entry.name,
            artist: entry.artist,
            added_by_user: entry.added_by_user,
            is_recommendation: entry.is_recommendation,
        }
    }
}

impl From<QueueEntry> for DomainQueueEntry {
    fn from(entry: QueueEntry) -> Self {
        Self {
            uri: entry.uri,
            name: entry.name,
            artist: entry.artist,
            album: None,
            duration_ms: None,
            added_by_user: entry.added_by_user,
            is_recommendation: entry.is_recommendation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(name: &str, artist: &str, uri: &str) -> QueueEntry {
        QueueEntry {
            name: name.to_string(),
            artist: artist.to_string(),
            uri: uri.to_string(),
            added_by_user: true,
            is_recommendation: false,
        }
    }

    #[test]
    fn test_queue_defaults() {
        let queue = QueueState::new();
        assert!(queue.local_queue.is_empty());
        assert!(queue.spotify_queue.is_none());
        assert!(queue.radio_mode); // Radio mode enabled by default
        assert!(queue.persistence_path.is_none());
    }

    #[test]
    fn test_queue_entry_defaults() {
        let entry = QueueEntry::default();
        assert!(entry.uri.is_empty());
        assert!(entry.name.is_empty());
        assert!(entry.artist.is_empty());
        assert!(!entry.added_by_user);
        assert!(!entry.is_recommendation);
    }

    #[test]
    fn test_add_single_track() {
        let mut queue = QueueState::new();
        let entry = make_entry("Song A", "Artist A", "spotify:track:123");
        queue.add(entry);

        assert_eq!(queue.local_queue.len(), 1);
        assert_eq!(queue.local_queue[0].name, "Song A");
        assert_eq!(queue.local_queue[0].artist, "Artist A");
        assert!(queue.local_queue[0].added_by_user);
    }

    #[test]
    fn test_add_multiple_tracks() {
        let mut queue = QueueState::new();
        queue.add(make_entry("Song A", "Artist A", "spotify:track:1"));
        queue.add(make_entry("Song B", "Artist B", "spotify:track:2"));
        queue.add(make_entry("Song C", "Artist C", "spotify:track:3"));

        assert_eq!(queue.local_queue.len(), 3);
        assert_eq!(queue.local_queue[0].name, "Song A");
        assert_eq!(queue.local_queue[1].name, "Song B");
        assert_eq!(queue.local_queue[2].name, "Song C");
    }

    #[test]
    fn test_next_returns_first_track() {
        let mut queue = QueueState::new();
        queue.add(make_entry("Song A", "Artist A", "spotify:track:1"));
        queue.add(make_entry("Song B", "Artist B", "spotify:track:2"));

        let next = queue.next_track();
        assert!(next.is_some());
        let entry = next.unwrap();
        assert_eq!(entry.name, "Song A");
        assert_eq!(entry.uri, "spotify:track:1");

        // Queue should now have 1 item
        assert_eq!(queue.local_queue.len(), 1);
        assert_eq!(queue.local_queue[0].name, "Song B");
    }

    #[test]
    fn test_next_on_empty_queue() {
        let mut queue = QueueState::new();
        assert!(queue.next_track().is_none());
    }

    #[test]
    fn test_next_removes_track_from_queue() {
        let mut queue = QueueState::new();
        queue.add(make_entry("Song A", "Artist A", "spotify:track:1"));

        let first = queue.next_track();
        assert!(first.is_some());
        assert_eq!(first.unwrap().name, "Song A");

        let second = queue.next_track();
        assert!(second.is_none());

        assert!(queue.local_queue.is_empty());
    }

    #[test]
    fn test_clear_removes_all_tracks() {
        let mut queue = QueueState::new();
        queue.add(make_entry("Song A", "Artist A", "spotify:track:1"));
        queue.add(make_entry("Song B", "Artist B", "spotify:track:2"));
        queue.add(make_entry("Song C", "Artist C", "spotify:track:3"));

        assert_eq!(queue.local_queue.len(), 3);
        queue.clear();
        assert!(queue.local_queue.is_empty());
    }

    #[test]
    fn test_clear_on_empty_queue() {
        let mut queue = QueueState::new();
        queue.clear();
        assert!(queue.local_queue.is_empty());
    }

    #[test]
    fn test_is_empty_with_no_tracks() {
        let queue = QueueState::new();
        assert!(queue.is_empty());
    }

    #[test]
    fn test_is_empty_with_local_tracks() {
        let mut queue = QueueState::new();
        queue.add(make_entry("Song A", "Artist A", "spotify:track:1"));
        assert!(!queue.is_empty());
    }

    #[test]
    fn test_is_empty_with_spotify_queue() {
        let mut queue = QueueState::new();
        queue.spotify_queue = Some(CurrentUserQueue {
            currently_playing: None,
            queue: vec![],
        });
        assert!(queue.is_empty());
    }

    #[test]
    fn test_total_count_empty_queue() {
        let queue = QueueState::new();
        assert_eq!(queue.total_count(), 0);
    }

    #[test]
    fn test_total_count_local_only() {
        let mut queue = QueueState::new();
        queue.add(make_entry("Song A", "Artist A", "spotify:track:1"));
        queue.add(make_entry("Song B", "Artist B", "spotify:track:2"));
        assert_eq!(queue.total_count(), 2);
    }

    #[test]
    fn test_total_count_with_spotify_queue() {
        let mut queue = QueueState::new();
        queue.add(make_entry("Local A", "Artist A", "spotify:track:1"));
        queue.add(make_entry("Local B", "Artist B", "spotify:track:2"));

        queue.spotify_queue = Some(CurrentUserQueue {
            currently_playing: None,
            queue: vec![],
        });

        assert_eq!(queue.local_queue.len(), 2);
        assert_eq!(queue.total_count(), 2);
    }

    #[test]
    fn test_update_spotify_queue() {
        let mut queue = QueueState::new();
        assert!(queue.spotify_queue.is_none());

        let spotify_queue = CurrentUserQueue {
            currently_playing: None,
            queue: vec![],
        };
        queue.update_spotify_queue(spotify_queue);

        assert!(queue.spotify_queue.is_some());
        assert_eq!(queue.spotify_queue.as_ref().unwrap().queue.len(), 0);
    }

    #[test]
    fn test_queue_fifo_order() {
        let mut queue = QueueState::new();
        queue.add(make_entry("First", "A", "spotify:track:1"));
        queue.add(make_entry("Second", "B", "spotify:track:2"));
        queue.add(make_entry("Third", "C", "spotify:track:3"));

        // Verify FIFO: first added = first out
        assert_eq!(queue.next_track().unwrap().name, "First");
        assert_eq!(queue.next_track().unwrap().name, "Second");
        assert_eq!(queue.next_track().unwrap().name, "Third");
        assert!(queue.next_track().is_none());
    }

    #[test]
    fn test_add_after_clear() {
        let mut queue = QueueState::new();
        queue.add(make_entry("Old Song", "Old Artist", "spotify:track:1"));
        queue.clear();
        queue.add(make_entry("New Song", "New Artist", "spotify:track:2"));

        assert_eq!(queue.local_queue.len(), 1);
        assert_eq!(queue.local_queue[0].name, "New Song");
    }

    #[test]
    fn test_recommendation_flag_preserved() {
        let mut queue = QueueState::new();
        let mut entry = make_entry("Recommended", "Artist", "spotify:track:1");
        entry.is_recommendation = true;
        entry.added_by_user = false;
        queue.add(entry);

        assert!(queue.local_queue[0].is_recommendation);
        assert!(!queue.local_queue[0].added_by_user);
    }

    #[test]
    fn test_entry_conversion_to_domain() {
        let entry = QueueEntry {
            uri: "spotify:track:abc".into(),
            name: "Test".into(),
            artist: "Artist".into(),
            added_by_user: true,
            is_recommendation: false,
        };
        let domain: DomainQueueEntry = entry.into();
        assert_eq!(domain.uri, "spotify:track:abc");
        assert_eq!(domain.name, "Test");
        assert_eq!(domain.artist, "Artist");
        assert!(domain.added_by_user);
        assert!(!domain.is_recommendation);
        assert!(domain.album.is_none());
        assert!(domain.duration_ms.is_none());
    }

    #[test]
    fn test_entry_conversion_from_domain() {
        let domain = DomainQueueEntry {
            uri: "spotify:track:xyz".into(),
            name: "Domain Song".into(),
            artist: "Domain Artist".into(),
            album: Some("Album".into()),
            duration_ms: Some(180_000),
            added_by_user: false,
            is_recommendation: true,
        };
        let entry: QueueEntry = domain.into();
        assert_eq!(entry.uri, "spotify:track:xyz");
        assert_eq!(entry.name, "Domain Song");
        assert_eq!(entry.artist, "Domain Artist");
        assert!(!entry.added_by_user);
        assert!(entry.is_recommendation);
    }

    // -----------------------------------------------------------------------
    // Integration tests: QueueState wrapper + PlaybackQueue domain
    // -----------------------------------------------------------------------

    #[test]
    fn test_queue_state_syncs_with_playback_queue() {
        let mut queue = QueueState::new();

        // Add tracks via the wrapper
        queue.add(make_entry("Track 1", "Artist 1", "spotify:track:1"));
        queue.add(make_entry("Track 2", "Artist 2", "spotify:track:2"));

        // Both local_queue and playback_queue should have 2 items
        assert_eq!(queue.local_queue.len(), 2);
        assert_eq!(queue.playback_queue().up_next_count(), 2);
    }

    #[test]
    fn test_queue_state_next_track_advances_both() {
        let mut queue = QueueState::new();
        queue.add(make_entry("First", "A", "spotify:track:1"));
        queue.add(make_entry("Second", "B", "spotify:track:2"));
        queue.add(make_entry("Third", "C", "spotify:track:3"));

        // Advance via wrapper
        let first = queue.next_track();
        assert!(first.is_some());
        assert_eq!(first.unwrap().name, "First");

        // Both should be advanced
        assert_eq!(queue.local_queue.len(), 2);
        assert_eq!(queue.playback_queue().up_next_count(), 2);
    }

    #[test]
    fn test_queue_state_clear_resets_both() {
        let mut queue = QueueState::new();
        queue.add(make_entry("Track", "Artist", "spotify:track:1"));

        queue.clear();

        assert!(queue.local_queue.is_empty());
        assert!(queue.playback_queue().is_exhausted());
    }

    #[test]
    fn test_queue_state_total_count_includes_both() {
        let mut queue = QueueState::new();
        queue.add(make_entry("Local 1", "A", "spotify:track:1"));
        queue.add(make_entry("Local 2", "B", "spotify:track:2"));

        // Add a mock spotify queue
        queue.spotify_queue = Some(CurrentUserQueue {
            currently_playing: None,
            queue: vec![], // Can't easily construct FullTrack, so empty
        });

        assert_eq!(queue.total_count(), 2); // 2 local + 0 spotify
    }

    #[test]
    fn test_queue_state_is_empty_consistent() {
        let mut queue = QueueState::new();

        // Empty
        assert!(queue.is_empty());

        // Add track
        queue.add(make_entry("Track", "Artist", "spotify:track:1"));
        assert!(!queue.is_empty());

        // Remove track
        queue.next_track();
        assert!(queue.is_empty());
    }

    #[test]
    fn test_playback_queue_accessible_via_wrapper() {
        use crate::playback::domain::PlaybackContext;

        let mut queue = QueueState::new();

        // Set context via the underlying PlaybackQueue
        queue.playback_queue_mut().set_context(
            PlaybackContext::Playlist {
                uri: "spotify:playlist:abc".to_string(),
                name: "My Playlist".to_string(),
                start_index: 0,
            },
            vec!["spotify:track:1".to_string(), "spotify:track:2".to_string()],
        );

        // Verify context is set
        assert!(queue.playback_queue().has_context());
        assert_eq!(queue.playback_queue().context().name(), "My Playlist");
        assert_eq!(queue.playback_queue().context_track_count(), 2);

        // Advance via playback queue
        let next = queue.playback_queue_mut().advance();
        assert_eq!(next, Some("spotify:track:1".to_string()));
    }
}
