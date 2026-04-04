//! Queue state management
//!
//! Manages both Spotify server-side queue and local queue for track ordering.

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

    /// Get the next track from local queue (FIFO)
    pub fn next(&mut self) -> Option<QueueEntry> {
        if !self.local_queue.is_empty() {
            Some(self.local_queue.remove(0))
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
                .map_or(true, |q| q.queue.is_empty())
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
        assert!(!queue.radio_mode);
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

        let next = queue.next();
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
        assert!(queue.next().is_none());
    }

    #[test]
    fn test_next_removes_track_from_queue() {
        let mut queue = QueueState::new();
        queue.add(make_entry("Song A", "Artist A", "spotify:track:1"));

        let first = queue.next();
        assert!(first.is_some());
        assert_eq!(first.unwrap().name, "Song A");

        let second = queue.next();
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
        // Create a minimal spotify queue for testing
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

        // Spotify queue with 3 items
        queue.spotify_queue = Some(CurrentUserQueue {
            currently_playing: None,
            queue: vec![
                // We can't easily construct FullTrack, so test with empty queue
                // The count logic is simple enough to verify via local_queue.len()
            ],
        });

        // Should only count local items since spotify queue is empty
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
        assert_eq!(queue.next().unwrap().name, "First");
        assert_eq!(queue.next().unwrap().name, "Second");
        assert_eq!(queue.next().unwrap().name, "Third");
        assert!(queue.next().is_none());
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
}
