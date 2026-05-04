//! Domain model for Spotify-style context playback and queue management.
//!
//! This module provides the core abstractions for managing playback state,
//! including context-aware queue management (playlists, albums, artists),
//! user-queued tracks, and shuffle support.
//!
//! # Key Concepts
//!
//! ## Two-Tier Queue System
//!
//! The queue has two layers that work together:
//!
//! 1. **User Queue (`up_next`)**: Tracks explicitly queued by the user.
//!    These always play first, in the order they were added. Shuffle does
//!    not affect this layer.
//!
//! 2. **Context Tracks (`context_tracks`)**: The full track list of the
//!    current playback context (playlist, album, or artist). The queue
//!    position tracks progress through this list. Shuffle reorders this layer.
//!
//! ## Playback Flow
//!
//! ```text
//! advance() called
//!     │
//!     ├── up_next not empty? → pop from front (user queue priority)
//!     │
//!     └── context_position < context_tracks.len()?
//!         │
//!         ├── yes → return context_tracks[context_position], increment
//!         │
//!         └── no → None (end of queue)
//! ```
//!
//! # Examples
//!
//! ```
//! use joshify::playback::domain::{PlaybackQueue, PlaybackContext, QueueEntry};
//!
//! // Create a queue with a playlist context
//! let mut queue = PlaybackQueue::new();
//! queue.set_context(
//!     PlaybackContext::Playlist {
//!         uri: "spotify:playlist:abc123".to_string(),
//!         name: "My Playlist".to_string(),
//!         start_index: 0,
//!     },
//!     vec![
//!         "spotify:track:1".to_string(),
//!         "spotify:track:2".to_string(),
//!         "spotify:track:3".to_string(),
//!     ],
//! );
//!
//! // User queues a track to play next
//! queue.add_to_up_front(QueueEntry {
//!     uri: "spotify:track:99".to_string(),
//!     name: "Skip Ahead".to_string(),
//!     artist: "Artist".to_string(),
//!     ..Default::default()
//! });
//!
//! // advance() returns the user-queued track first
//! assert_eq!(queue.advance(), Some("spotify:track:99".to_string()));
//!
//! // Then falls back to context
//! assert_eq!(queue.advance(), Some("spotify:track:1".to_string()));
//! ```

use std::fmt;

// ---------------------------------------------------------------------------
// PlaybackContext
// ---------------------------------------------------------------------------

/// The type of playback context the user is currently listening to.
///
/// A context represents a collection of tracks that defines the "session" —
/// what the user was browsing when they started playback. This is used for
/// context-aware features like "Go to Album", queue history, and smart
/// recommendations.
///
/// # Variants
///
/// - `Playlist` — A user or Spotify playlist, includes the track index for
///   context-aware navigation.
/// - `Album` — A full album release.
/// - `Artist` — An artist's top tracks or discography.
/// - `None` — No context (e.g., playing a single track, radio mode, or
///   the user explicitly cleared context).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PlaybackContext {
    /// A playlist context with its URI, display name, and the track index
    /// within the playlist where playback started.
    Playlist {
        /// Spotify playlist URI (e.g., `spotify:playlist:37i9dQZF1DXcBWIGoYBM5M`)
        uri: String,
        /// Display name for the playlist
        name: String,
        /// Index in the playlist where playback began (for "go to playlist" navigation)
        start_index: usize,
    },
    /// An album context with its URI and display name.
    Album {
        /// Spotify album URI (e.g., `spotify:album:4aawyAB9vmqN3uQ7FjRGTy`)
        uri: String,
        /// Display name for the album
        name: String,
    },
    /// An artist context with its URI and display name.
    Artist {
        /// Spotify artist URI (e.g., `spotify:artist:4q3ewBCX7sLwd24euuV69X`)
        uri: String,
        /// Display name for the artist
        name: String,
    },
    /// No active context — playback is not tied to a specific collection.
    #[default]
    None,
}

impl PlaybackContext {
    /// Returns the Spotify URI for this context, if available.
    ///
    /// Returns `None` for `PlaybackContext::None`.
    pub fn uri(&self) -> Option<&str> {
        match self {
            Self::Playlist { uri, .. } => Some(uri),
            Self::Album { uri, .. } => Some(uri),
            Self::Artist { uri, .. } => Some(uri),
            Self::None => None,
        }
    }

    /// Returns the display name for this context.
    ///
    /// For `PlaybackContext::None`, returns an empty string.
    pub fn name(&self) -> &str {
        match self {
            Self::Playlist { name, .. } => name,
            Self::Album { name, .. } => name,
            Self::Artist { name, .. } => name,
            Self::None => "",
        }
    }

    /// Returns a human-readable label for the context type.
    ///
    /// # Examples
    ///
    /// ```
    /// use joshify::playback::domain::PlaybackContext;
    ///
    /// let ctx = PlaybackContext::Album {
    ///     uri: "spotify:album:abc".to_string(),
    ///     name: "Test Album".to_string(),
    /// };
    /// assert_eq!(ctx.type_label(), "Album");
    /// ```
    pub fn type_label(&self) -> &'static str {
        match self {
            Self::Playlist { .. } => "Playlist",
            Self::Album { .. } => "Album",
            Self::Artist { .. } => "Artist",
            Self::None => "None",
        }
    }

    /// Returns `true` if this is `PlaybackContext::None`.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

impl fmt::Display for PlaybackContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Playlist { name, .. } => write!(f, "Playlist: {name}"),
            Self::Album { name, .. } => write!(f, "Album: {name}"),
            Self::Artist { name, .. } => write!(f, "Artist: {name}"),
            Self::None => write!(f, "No context"),
        }
    }
}

// ---------------------------------------------------------------------------
// QueueEntry
// ---------------------------------------------------------------------------

/// A single track entry in the playback queue.
///
/// Contains all metadata needed for UI display and playback without requiring
/// additional API calls. This is a denormalized snapshot — if track metadata
/// changes on Spotify's side, this entry is not automatically updated.
///
/// # Examples
///
/// ```
/// use joshify::playback::domain::QueueEntry;
///
/// let entry = QueueEntry {
///     uri: "spotify:track:abc123".to_string(),
///     name: "Song Title".to_string(),
///     artist: "Artist Name".to_string(),
///     album: Some("Album Name".to_string()),
///     duration_ms: Some(210_000),
///     added_by_user: true,
///     is_recommendation: false,
/// };
/// assert_eq!(entry.display_name(), "Song Title");
/// assert_eq!(entry.display_artist(), "Artist Name");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QueueEntry {
    /// Spotify track URI (e.g., `spotify:track:4iV5W9uYEdYUVa79Axb7Rh`)
    pub uri: String,
    /// Track title
    pub name: String,
    /// Primary artist name
    pub artist: String,
    /// Album name, if known
    pub album: Option<String>,
    /// Track duration in milliseconds, if known
    pub duration_ms: Option<u32>,
    /// `true` if the user explicitly queued this track (vs. auto-added)
    pub added_by_user: bool,
    /// `true` if this was added by a recommendation engine (radio, discover)
    pub is_recommendation: bool,
}

impl QueueEntry {
    /// Creates a new `QueueEntry` with minimal required fields.
    ///
    /// # Arguments
    ///
    /// * `uri` - Spotify track URI
    /// * `name` - Track title
    /// * `artist` - Primary artist name
    pub fn new(uri: impl Into<String>, name: impl Into<String>, artist: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: name.into(),
            artist: artist.into(),
            album: None,
            duration_ms: None,
            added_by_user: true,
            is_recommendation: false,
        }
    }

    /// Returns the track name for display.
    ///
    /// Falls back to the URI if the name is empty.
    pub fn display_name(&self) -> &str {
        if self.name.is_empty() {
            &self.uri
        } else {
            &self.name
        }
    }

    /// Returns the artist name for display.
    ///
    /// Falls back to "Unknown Artist" if the artist is empty.
    pub fn display_artist(&self) -> &str {
        if self.artist.is_empty() {
            "Unknown Artist"
        } else {
            &self.artist
        }
    }

    /// Returns the formatted duration string (MM:SS), if available.
    pub fn formatted_duration(&self) -> Option<String> {
        self.duration_ms.map(|ms| {
            let total_secs = ms / 1000;
            let mins = total_secs / 60;
            let secs = total_secs % 60;
            format!("{mins:02}:{secs:02}")
        })
    }
}

// ---------------------------------------------------------------------------
// CurrentSource
// ---------------------------------------------------------------------------

/// Indicates where the currently playing track originated from.
///
/// This is set by `advance()` and tells the UI and playback engine whether
/// the current track came from the user queue or the context track list.
///
/// # Usage
///
/// After calling `advance()`, check `current_source()` to determine:
/// - Whether to show a "back to context" hint in the UI
/// - Whether the track can be "removed" from the queue (user-queued only)
/// - Whether repeat-context logic should apply
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CurrentSource {
    /// The current track came from the user queue (`up_next`).
    UpNext,
    /// The current track came from the context track list.
    Context,
    /// No track is currently loaded (initial state or queue exhausted).
    #[default]
    None,
}

impl CurrentSource {
    /// Returns `true` if the source is `UpNext`.
    pub fn is_up_next(&self) -> bool {
        matches!(self, Self::UpNext)
    }

    /// Returns `true` if the source is `Context`.
    pub fn is_context(&self) -> bool {
        matches!(self, CurrentSource::Context)
    }

    /// Returns `true` if no source is set.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

// ---------------------------------------------------------------------------
// PlaybackQueue
// ---------------------------------------------------------------------------

/// Manages the full playback queue with context awareness.
///
/// `PlaybackQueue` is the central abstraction for track ordering. It maintains
/// two layers:
///
/// 1. **User Queue** (`up_next`): Explicitly queued tracks that play first.
/// 2. **Context** (`context_tracks`): The full track list of the current
///    playback context, with a position pointer.
///
/// # Thread Safety
///
/// This struct is `Send + Sync` and can be wrapped in `Arc<Mutex<Self>>` for
/// shared access across the event loop and UI threads.
///
/// # Examples
///
/// ```
/// use joshify::playback::domain::{PlaybackQueue, PlaybackContext, QueueEntry};
///
/// let mut queue = PlaybackQueue::new();
///
/// // Set up a playlist context
/// queue.set_context(
///     PlaybackContext::Playlist {
///         uri: "spotify:playlist:abc".to_string(),
///         name: "My Playlist".to_string(),
///         start_index: 0,
///     },
///     vec![
///         "spotify:track:1".to_string(),
///         "spotify:track:2".to_string(),
///     ],
/// );
///
/// // Queue a track to play next
/// queue.add_to_up_front(QueueEntry::new(
///     "spotify:track:99",
///     "Skip Ahead",
///     "Artist",
/// ));
///
/// // advance() returns user-queued track first, then context
/// assert_eq!(queue.advance(), Some("spotify:track:99".to_string()));
/// assert_eq!(queue.advance(), Some("spotify:track:1".to_string()));
/// assert_eq!(queue.advance(), Some("spotify:track:2".to_string()));
/// assert_eq!(queue.advance(), None); // exhausted
/// ```
#[derive(Debug, Clone)]
pub struct PlaybackQueue {
    /// User-queued tracks that play before context tracks.
    /// Ordered front-to-back: index 0 plays next.
    up_next: Vec<QueueEntry>,

    /// The current playback context (playlist, album, artist, or none).
    context: PlaybackContext,

    /// All track URIs in the current context, in playback order.
    /// When shuffle is enabled, this list is reordered but `context_position`
    /// still advances linearly.
    context_tracks: Vec<String>,

    /// Current position within `context_tracks`.
    /// Tracks at indices < `context_position` have already been played.
    context_position: usize,

    /// Where the most recently advanced track came from.
    current_source: CurrentSource,

    /// Whether shuffle mode is active.
    /// When `true`, `context_tracks` is shuffled but `up_next` is preserved.
    shuffle: bool,
}

impl Default for PlaybackQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaybackQueue {
    /// Creates a new empty `PlaybackQueue`.
    ///
    /// The queue starts with no context, no tracks, and `CurrentSource::None`.
    pub fn new() -> Self {
        Self {
            up_next: Vec::new(),
            context: PlaybackContext::None,
            context_tracks: Vec::new(),
            context_position: 0,
            current_source: CurrentSource::None,
            shuffle: false,
        }
    }

    // -----------------------------------------------------------------------
    // Query methods
    // -----------------------------------------------------------------------

    /// Returns the URI of the next track without advancing the queue.
    ///
    /// This is a pure query — calling it multiple times returns the same
    /// result and does not mutate any internal state.
    ///
    /// # Priority
    ///
    /// 1. First item in `up_next` (user queue)
    /// 2. Current `context_tracks[context_position]` (context)
    /// 3. `None` if both are exhausted
    ///
    /// # Examples
    ///
    /// ```
    /// use joshify::playback::domain::{PlaybackQueue, PlaybackContext, QueueEntry};
    ///
    /// let mut queue = PlaybackQueue::new();
    /// queue.set_context(
    ///     PlaybackContext::Album {
    ///         uri: "spotify:album:abc".to_string(),
    ///         name: "Album".to_string(),
    ///     },
    ///     vec!["spotify:track:1".to_string()],
    /// );
    ///
    /// // peek_next returns context track
    /// assert_eq!(queue.peek_next(), Some("spotify:track:1".to_string()));
    ///
    /// // peek again — same result, no advancement
    /// assert_eq!(queue.peek_next(), Some("spotify:track:1".to_string()));
    ///
    /// // Add to user queue — peek now returns that instead
    /// queue.add_to_up_front(QueueEntry::new("spotify:track:99", "X", "Y"));
    /// assert_eq!(queue.peek_next(), Some("spotify:track:99".to_string()));
    /// ```
    pub fn peek_next(&self) -> Option<String> {
        self.up_next
            .first()
            .map(|entry| entry.uri.clone())
            .or_else(|| self.context_tracks.get(self.context_position).cloned())
    }

    /// Returns the full `QueueEntry` for the next track without advancing.
    ///
    /// For context tracks, only the URI is available (the entry metadata
    /// is not stored). Returns a minimal `QueueEntry` with just the URI set.
    pub fn peek_next_entry(&self) -> Option<QueueEntry> {
        self.up_next.first().cloned().or_else(|| {
            self.context_tracks
                .get(self.context_position)
                .map(|uri| QueueEntry {
                    uri: uri.clone(),
                    ..Default::default()
                })
        })
    }

    /// Returns the URI of the currently playing track, if known.
    ///
    /// This is the track most recently returned by `advance()`.
    /// Returns `None` if no track has been advanced to yet, or if the
    /// queue is exhausted.
    pub fn current_track_uri(&self) -> Option<String> {
        match self.current_source {
            CurrentSource::UpNext => {
                // The current track was just removed from up_next,
                // so we can't recover it from up_next.
                // This is a design limitation — callers should track
                // the current track separately via the advance() return value.
                None
            }
            CurrentSource::Context => {
                // context_position was incremented after returning the track,
                // so the current track is at position - 1.
                if self.context_position > 0 {
                    self.context_tracks.get(self.context_position - 1).cloned()
                } else {
                    None
                }
            }
            CurrentSource::None => None,
        }
    }

    // -----------------------------------------------------------------------
    // Mutation methods
    // -----------------------------------------------------------------------

    /// Advances the queue and returns the next track URI to play.
    ///
    /// This is the primary method for progressing through the queue. It:
    ///
    /// 1. Pops the first item from `up_next` if available (user queue priority)
    /// 2. Otherwise returns `context_tracks[context_position]` and increments
    ///    the position
    /// 3. Returns `None` if both layers are exhausted
    ///
    /// # Side Effects
    ///
    /// - Sets `current_source` to reflect where the track came from
    /// - Removes the track from `up_next` if played from there
    /// - Increments `context_position` if played from context
    ///
    /// # Examples
    ///
    /// ```
    /// use joshify::playback::domain::{PlaybackQueue, PlaybackContext, QueueEntry};
    ///
    /// let mut queue = PlaybackQueue::new();
    /// queue.set_context(
    ///     PlaybackContext::Album {
    ///         uri: "spotify:album:abc".to_string(),
    ///         name: "Album".to_string(),
    ///     },
    ///     vec![
    ///         "spotify:track:1".to_string(),
    ///         "spotify:track:2".to_string(),
    ///     ],
    /// );
    ///
    /// assert_eq!(queue.advance(), Some("spotify:track:1".to_string()));
    /// assert_eq!(queue.advance(), Some("spotify:track:2".to_string()));
    /// assert_eq!(queue.advance(), None); // exhausted
    /// ```
    pub fn advance(&mut self) -> Option<String> {
        // Priority 1: User queue
        if !self.up_next.is_empty() {
            let entry = self.up_next.remove(0);
            self.current_source = CurrentSource::UpNext;
            return Some(entry.uri);
        }

        // Priority 2: Context tracks
        if let Some(uri) = self.context_tracks.get(self.context_position).cloned() {
            self.context_position += 1;
            self.current_source = CurrentSource::Context;
            return Some(uri);
        }

        // Exhausted
        self.current_source = CurrentSource::None;
        None
    }

    /// Adds a track to the front of the user queue (`up_next`).
    ///
    /// This track will be the next one returned by `advance()` or `peek_next()`,
    /// regardless of context position.
    ///
    /// # Examples
    ///
    /// ```
    /// use joshify::playback::domain::{PlaybackQueue, QueueEntry};
    ///
    /// let mut queue = PlaybackQueue::new();
    /// queue.add_to_up_front(QueueEntry::new("spotify:track:1", "First", "A"));
    /// queue.add_to_up_front(QueueEntry::new("spotify:track:2", "Second", "B"));
    ///
    /// // Most recently added plays first (LIFO for front-inserts)
    /// assert_eq!(queue.advance(), Some("spotify:track:2".to_string()));
    /// assert_eq!(queue.advance(), Some("spotify:track:1".to_string()));
    /// ```
    pub fn add_to_up_front(&mut self, entry: QueueEntry) {
        self.up_next.insert(0, entry);
    }

    /// Adds a track to the end of the user queue (`up_next`).
    ///
    /// This track will play after all other user-queued tracks but before
    /// context tracks resume.
    ///
    /// # Examples
    ///
    /// ```
    /// use joshify::playback::domain::{PlaybackQueue, QueueEntry};
    ///
    /// let mut queue = PlaybackQueue::new();
    /// queue.add_to_up_next(QueueEntry::new("spotify:track:1", "First", "A"));
    /// queue.add_to_up_next(QueueEntry::new("spotify:track:2", "Second", "B"));
    ///
    /// // FIFO order preserved
    /// assert_eq!(queue.advance(), Some("spotify:track:1".to_string()));
    /// assert_eq!(queue.advance(), Some("spotify:track:2".to_string()));
    /// ```
    pub fn add_to_up_next(&mut self, entry: QueueEntry) {
        self.up_next.push(entry);
    }

    /// Adds multiple tracks to the end of the user queue.
    ///
    /// Tracks are added in the order they appear in the iterator, so the
    /// first item in the iterator plays first.
    pub fn add_all_to_up_next(&mut self, entries: impl IntoIterator<Item = QueueEntry>) {
        self.up_next.extend(entries);
    }

    // -----------------------------------------------------------------------
    // Context management
    // -----------------------------------------------------------------------

    /// Sets the playback context and its full track list.
    ///
    /// This replaces any existing context and resets `context_position` to 0.
    /// The `up_next` user queue is preserved.
    ///
    /// # Arguments
    ///
    /// * `context` - The playback context (playlist, album, artist)
    /// * `tracks` - All track URIs in the context, in their natural order
    ///
    /// # Shuffle Interaction
    ///
    /// If `shuffle` is currently enabled, the `tracks` list will be shuffled
    /// immediately after being set.
    ///
    /// # Examples
    ///
    /// ```
    /// use joshify::playback::domain::{PlaybackQueue, PlaybackContext};
    ///
    /// let mut queue = PlaybackQueue::new();
    /// queue.set_context(
    ///     PlaybackContext::Album {
    ///         uri: "spotify:album:abc".to_string(),
    ///         name: "Album".to_string(),
    ///     },
    ///     vec![
    ///         "spotify:track:1".to_string(),
    ///         "spotify:track:2".to_string(),
    ///     ],
    /// );
    ///
    /// assert_eq!(queue.context().name(), "Album");
    /// assert_eq!(queue.context_track_count(), 2);
    /// ```
    pub fn set_context(&mut self, context: PlaybackContext, tracks: Vec<String>) {
        self.context = context;
        self.context_tracks = tracks;
        self.context_position = 0;

        if self.shuffle {
            self.shuffle_context();
        }
    }

    /// Clears the current context and all tracks.
    ///
    /// The user queue (`up_next`) is preserved. `current_source` is reset
    /// to `None` if the context was the active source.
    pub fn clear_context(&mut self) {
        self.context = PlaybackContext::None;
        self.context_tracks.clear();
        self.context_position = 0;

        if self.current_source == CurrentSource::Context {
            self.current_source = CurrentSource::None;
        }
    }

    /// Returns a reference to the current playback context.
    pub fn context(&self) -> &PlaybackContext {
        &self.context
    }

    /// Returns the number of tracks in the context.
    pub fn context_track_count(&self) -> usize {
        self.context_tracks.len()
    }

    /// Returns the current position within the context track list.
    pub fn context_position(&self) -> usize {
        self.context_position
    }

    /// Sets the current position within the context track list.
    ///
    /// This is useful when starting playback at a specific track within a context,
    /// without consuming the tracks that come before it.
    ///
    /// # Arguments
    ///
    /// * `position` - The index within context_tracks to start from (0-based)
    ///
    /// # Examples
    ///
    /// ```
    /// use joshify::playback::domain::{PlaybackQueue, PlaybackContext};
    ///
    /// let mut queue = PlaybackQueue::new();
    /// queue.set_context(
    ///     PlaybackContext::Playlist {
    ///         uri: "spotify:playlist:test".to_string(),
    ///         name: "Test".to_string(),
    ///     },
    ///     vec![
    ///         "spotify:track:1".to_string(),
    ///         "spotify:track:2".to_string(),
    ///         "spotify:track:3".to_string(),
    ///     ],
    /// );
    ///
    /// // Start at track 3 (index 2)
    /// queue.set_context_position(2);
    /// assert_eq!(queue.context_position(), 2);
    /// assert_eq!(queue.remaining_context_tracks(), 1);
    ///
    /// // Advance should return track 3
    /// assert_eq!(queue.advance(), Some("spotify:track:3".to_string()));
    /// ```
    pub fn set_context_position(&mut self, position: usize) {
        self.context_position = position.min(self.context_tracks.len());
    }

    /// Returns the number of remaining context tracks (not yet played).
    pub fn remaining_context_tracks(&self) -> usize {
        self.context_tracks
            .len()
            .saturating_sub(self.context_position)
    }

    // -----------------------------------------------------------------------
    // User queue management
    // -----------------------------------------------------------------------

    /// Returns the number of tracks in the user queue.
    pub fn up_next_count(&self) -> usize {
        self.up_next.len()
    }

    /// Returns `true` if the user queue is empty.
    pub fn is_up_next_empty(&self) -> bool {
        self.up_next.is_empty()
    }

    /// Returns a reference to the user queue entries.
    pub fn up_next_entries(&self) -> &[QueueEntry] {
        &self.up_next
    }

    /// Removes and returns the track at the given index from the user queue.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn remove_from_up_next(&mut self, index: usize) -> Option<QueueEntry> {
        if index < self.up_next.len() {
            Some(self.up_next.remove(index))
        } else {
            None
        }
    }

    /// Clears all user-queued tracks.
    ///
    /// The context is preserved.
    pub fn clear_up_next(&mut self) {
        self.up_next.clear();
    }

    // -----------------------------------------------------------------------
    // Shuffle support
    // -----------------------------------------------------------------------

    /// Returns whether shuffle mode is active.
    pub fn shuffle(&self) -> bool {
        self.shuffle
    }

    /// Enables or disables shuffle mode.
    ///
    /// When enabling shuffle, the context tracks are immediately shuffled
    /// using Fisher-Yates. The user queue (`up_next`) is never shuffled.
    ///
    /// When disabling shuffle, context tracks are restored to their original
    /// order. **Note**: This requires the original order to be stored elsewhere
    /// — this implementation does not preserve the original order. If you need
    /// to restore the original order, call `set_context()` again with the
    /// unshuffled track list.
    ///
    /// # Examples
    ///
    /// ```
    /// use joshify::playback::domain::{PlaybackQueue, PlaybackContext};
    ///
    /// let mut queue = PlaybackQueue::new();
    /// queue.set_context(
    ///     PlaybackContext::Album {
    ///         uri: "spotify:album:abc".to_string(),
    ///         name: "Album".to_string(),
    ///     },
    ///     vec![
    ///         "spotify:track:1".to_string(),
    ///         "spotify:track:2".to_string(),
    ///         "spotify:track:3".to_string(),
    ///     ],
    /// );
    ///
    /// queue.set_shuffle(true);
    /// assert!(queue.shuffle());
    ///
    /// // Context tracks are shuffled, but user queue is not
    /// queue.add_to_up_front(
    ///     joshify::playback::domain::QueueEntry::new("spotify:track:99", "X", "Y")
    /// );
    /// assert_eq!(queue.advance(), Some("spotify:track:99".to_string())); // user queue first
    /// ```
    pub fn set_shuffle(&mut self, enabled: bool) {
        if enabled != self.shuffle {
            self.shuffle = enabled;
            if enabled {
                self.shuffle_context();
            }
            // Note: Disabling shuffle does NOT restore original order.
            // Callers must re-set context if they need the original order.
        }
    }

    /// Shuffles the context tracks in place using unbiased Fisher-Yates algorithm.
    ///
    /// The user queue is never affected. The context position is reset to 0
    /// so all tracks are available again in their new order.
    /// Uses `rand::thread_rng()` for high-quality randomness.
    fn shuffle_context(&mut self) {
        if self.context_tracks.len() <= 1 {
            return;
        }

        use rand::seq::SliceRandom;
        use rand::thread_rng;

        let mut rng = thread_rng();
        self.context_tracks.shuffle(&mut rng);
        self.context_position = 0;
    }

    // -----------------------------------------------------------------------
    // Source tracking
    // -----------------------------------------------------------------------

    /// Returns where the most recently advanced track came from.
    pub fn current_source(&self) -> CurrentSource {
        self.current_source
    }

    /// Sets the current source explicitly.
    ///
    /// Use this when the playback engine loads a track from an external source
    /// (e.g., Spotify Connect started playback independently).
    pub fn set_current_source(&mut self, source: CurrentSource) {
        self.current_source = source;
    }

    // -----------------------------------------------------------------------
    // Queue view (UI snapshot)
    // -----------------------------------------------------------------------

    /// Generates a `QueueView` snapshot for UI display.
    ///
    /// This is a point-in-time snapshot that captures:
    /// - The currently playing track (if known)
    /// - All user-queued tracks
    /// - The next few context tracks (up to `max_context_preview`)
    /// - The context name
    ///
    /// # Arguments
    ///
    /// * `now_playing` - The currently playing track entry, if known.
    ///   Pass `None` if the current track is not available.
    /// * `max_context_preview` - Maximum number of context tracks to include
    ///   in the view. Use this to limit UI rendering (e.g., 10 tracks).
    ///
    /// # Examples
    ///
    /// ```
    /// use joshify::playback::domain::{PlaybackQueue, PlaybackContext, QueueEntry, CurrentSource};
    ///
    /// let mut queue = PlaybackQueue::new();
    /// queue.set_context(
    ///     PlaybackContext::Playlist {
    ///         uri: "spotify:playlist:abc".to_string(),
    ///         name: "My Playlist".to_string(),
    ///         start_index: 0,
    ///     },
    ///     vec![
    ///         "spotify:track:1".to_string(),
    ///         "spotify:track:2".to_string(),
    ///         "spotify:track:3".to_string(),
    ///     ],
    /// );
    /// queue.set_current_source(CurrentSource::Context);
    ///
    /// // Advance to "play" the first track
    /// queue.advance();
    ///
    /// let view = queue.get_queue_view(None, 10);
    /// assert_eq!(view.context_name, "Playlist: My Playlist");
    /// assert_eq!(view.next_from_context.len(), 2); // tracks 2 and 3 remaining
    /// ```
    pub fn get_queue_view(
        &self,
        now_playing: Option<QueueEntry>,
        max_context_preview: usize,
    ) -> QueueView {
        let context_name = match &self.context {
            PlaybackContext::None => String::new(),
            ctx => format!("{}: {}", ctx.type_label(), ctx.name()),
        };

        let next_from_context = self
            .context_tracks
            .get(self.context_position..)
            .unwrap_or(&[])
            .iter()
            .take(max_context_preview)
            .cloned()
            .collect();

        QueueView {
            now_playing,
            up_next: self.up_next.clone(),
            next_from_context,
            context_name,
            context_position: self.context_position,
            context_total: self.context_tracks.len(),
        }
    }

    // -----------------------------------------------------------------------
    // State queries
    // -----------------------------------------------------------------------

    /// Returns `true` if the queue has no more tracks to play.
    ///
    /// This is `true` when both the user queue and context are exhausted.
    pub fn is_exhausted(&self) -> bool {
        self.up_next.is_empty() && self.context_position >= self.context_tracks.len()
    }

    /// Returns the total number of tracks remaining in the queue.
    ///
    /// This includes both user-queued tracks and remaining context tracks.
    pub fn remaining_count(&self) -> usize {
        self.up_next.len() + self.remaining_context_tracks()
    }

    /// Returns `true` if there is an active playback context.
    pub fn has_context(&self) -> bool {
        !self.context.is_none()
    }

    // -----------------------------------------------------------------------
    // Reset
    // -----------------------------------------------------------------------

    /// Resets the queue to its initial empty state.
    ///
    /// Clears all tracks, context, and resets the source to `None`.
    pub fn reset(&mut self) {
        self.up_next.clear();
        self.context = PlaybackContext::None;
        self.context_tracks.clear();
        self.context_position = 0;
        self.current_source = CurrentSource::None;
        self.shuffle = false;
    }
}

// ---------------------------------------------------------------------------
// QueueView
// ---------------------------------------------------------------------------

/// A point-in-time snapshot of the queue for UI display.
///
/// `QueueView` is designed to be cheap to create and render. It captures
/// the queue state at a moment in time so the UI can render without holding
/// a lock on the `PlaybackQueue`.
///
/// # Rendering Guide
///
/// ```text
/// ┌─ Now Playing ───────────────────┐
/// │ {now_playing.name} - {artist}   │
/// └─────────────────────────────────┘
/// ┌─ Up Next ({up_next.len()}) ─────┐
/// │ {up_next[0].name} - {artist}    │
/// │ {up_next[1].name} - {artist}    │
/// │ ...                             │
/// └─────────────────────────────────┘
/// ┌─ {context_name} ────────────────┐
/// │ {next_from_context[0]}          │
/// │ {next_from_context[1]}          │
/// │ ... ({context_total -           │
/// │  context_position} remaining)   │
/// └─────────────────────────────────┘
/// ```
#[derive(Debug, Clone, Default)]
pub struct QueueView {
    /// The currently playing track, if known.
    pub now_playing: Option<QueueEntry>,
    /// User-queued tracks waiting to play.
    pub up_next: Vec<QueueEntry>,
    /// Upcoming context tracks (limited by `max_context_preview`).
    pub next_from_context: Vec<String>,
    /// Human-readable context label (e.g., "Playlist: My Playlist").
    pub context_name: String,
    /// Current position within the full context track list.
    pub context_position: usize,
    /// Total number of tracks in the context.
    pub context_total: usize,
}

impl QueueView {
    /// Returns `true` if there is nothing to display.
    pub fn is_empty(&self) -> bool {
        self.now_playing.is_none() && self.up_next.is_empty() && self.next_from_context.is_empty()
    }

    /// Returns the total number of upcoming tracks (user queue + context preview).
    pub fn upcoming_count(&self) -> usize {
        self.up_next.len() + self.next_from_context.len()
    }

    /// Returns the number of remaining context tracks not shown in the preview.
    pub fn hidden_context_tracks(&self) -> usize {
        self.context_total
            .saturating_sub(self.context_position)
            .saturating_sub(self.next_from_context.len())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn make_entry(uri: &str, name: &str, artist: &str) -> QueueEntry {
        QueueEntry {
            uri: uri.to_string(),
            name: name.to_string(),
            artist: artist.to_string(),
            album: None,
            duration_ms: None,
            added_by_user: true,
            is_recommendation: false,
        }
    }

    fn make_album_context(uri: &str, name: &str) -> PlaybackContext {
        PlaybackContext::Album {
            uri: uri.to_string(),
            name: name.to_string(),
        }
    }

    fn make_playlist_context(uri: &str, name: &str) -> PlaybackContext {
        PlaybackContext::Playlist {
            uri: uri.to_string(),
            name: name.to_string(),
            start_index: 0,
        }
    }

    // -----------------------------------------------------------------------
    // PlaybackContext tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_context_uri_returns_some_for_playlists() {
        let ctx = make_playlist_context("spotify:playlist:abc", "My Playlist");
        assert_eq!(ctx.uri(), Some("spotify:playlist:abc"));
    }

    #[test]
    fn test_context_uri_returns_some_for_albums() {
        let ctx = make_album_context("spotify:album:xyz", "Album");
        assert_eq!(ctx.uri(), Some("spotify:album:xyz"));
    }

    #[test]
    fn test_context_uri_returns_none_for_none_variant() {
        let ctx = PlaybackContext::None;
        assert_eq!(ctx.uri(), None);
    }

    #[test]
    fn test_context_name_returns_display_name() {
        let ctx = make_playlist_context("spotify:playlist:abc", "Chill Vibes");
        assert_eq!(ctx.name(), "Chill Vibes");
    }

    #[test]
    fn test_context_name_empty_for_none() {
        let ctx = PlaybackContext::None;
        assert_eq!(ctx.name(), "");
    }

    #[test]
    fn test_context_type_label() {
        assert_eq!(make_playlist_context("", "").type_label(), "Playlist");
        assert_eq!(make_album_context("", "").type_label(), "Album");
        assert_eq!(PlaybackContext::None.type_label(), "None");
    }

    #[test]
    fn test_context_is_none() {
        assert!(PlaybackContext::None.is_none());
        assert!(!make_album_context("", "").is_none());
    }

    #[test]
    fn test_context_display() {
        let ctx = make_album_context("", "Dark Side");
        assert_eq!(ctx.to_string(), "Album: Dark Side");

        let ctx = PlaybackContext::None;
        assert_eq!(ctx.to_string(), "No context");
    }

    // -----------------------------------------------------------------------
    // QueueEntry tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_queue_entry_new() {
        let entry = QueueEntry::new("spotify:track:abc", "Song", "Artist");
        assert_eq!(entry.uri, "spotify:track:abc");
        assert_eq!(entry.name, "Song");
        assert_eq!(entry.artist, "Artist");
        assert!(entry.added_by_user);
        assert!(!entry.is_recommendation);
    }

    #[test]
    fn test_queue_entry_display_name_falls_back_to_uri() {
        let entry = QueueEntry {
            uri: "spotify:track:abc".to_string(),
            name: String::new(),
            artist: "Artist".to_string(),
            ..Default::default()
        };
        assert_eq!(entry.display_name(), "spotify:track:abc");
    }

    #[test]
    fn test_queue_entry_display_artist_falls_back() {
        let entry = QueueEntry {
            uri: "spotify:track:abc".to_string(),
            name: "Song".to_string(),
            artist: String::new(),
            ..Default::default()
        };
        assert_eq!(entry.display_artist(), "Unknown Artist");
    }

    #[test]
    fn test_queue_entry_formatted_duration() {
        let entry = QueueEntry {
            duration_ms: Some(210_000),
            ..Default::default()
        };
        assert_eq!(entry.formatted_duration(), Some("03:30".to_string()));
    }

    #[test]
    fn test_queue_entry_formatted_duration_none() {
        let entry = QueueEntry::default();
        assert_eq!(entry.formatted_duration(), None);
    }

    // -----------------------------------------------------------------------
    // CurrentSource tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_current_source_defaults_to_none() {
        let source = CurrentSource::default();
        assert!(source.is_none());
        assert!(!source.is_up_next());
        assert!(!source.is_context());
    }

    // -----------------------------------------------------------------------
    // PlaybackQueue: initialization
    // -----------------------------------------------------------------------

    #[test]
    fn test_queue_new_is_empty() {
        let queue = PlaybackQueue::new();
        assert!(queue.is_exhausted());
        assert_eq!(queue.remaining_count(), 0);
        assert!(!queue.has_context());
        assert_eq!(queue.current_source(), CurrentSource::None);
    }

    #[test]
    fn test_queue_default() {
        let queue = PlaybackQueue::default();
        assert!(queue.is_exhausted());
    }

    // -----------------------------------------------------------------------
    // PlaybackQueue: context management
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_context_resets_position() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("spotify:album:abc", "Album"),
            vec!["spotify:track:1".to_string(), "spotify:track:2".to_string()],
        );

        assert_eq!(queue.context().name(), "Album");
        assert_eq!(queue.context_track_count(), 2);
        assert_eq!(queue.context_position(), 0);
        assert!(queue.has_context());
    }

    #[test]
    fn test_set_context_replaces_existing() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("spotify:album:old", "Old Album"),
            vec!["spotify:track:1".to_string()],
        );
        queue.advance(); // position = 1

        // Replace with new context
        queue.set_context(
            make_playlist_context("spotify:playlist:new", "New Playlist"),
            vec![
                "spotify:track:10".to_string(),
                "spotify:track:20".to_string(),
            ],
        );

        assert_eq!(queue.context().name(), "New Playlist");
        assert_eq!(queue.context_position(), 0); // reset
        assert_eq!(queue.context_track_count(), 2);
    }

    #[test]
    fn test_clear_context() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("spotify:album:abc", "Album"),
            vec!["spotify:track:1".to_string()],
        );
        queue.clear_context();

        assert!(!queue.has_context());
        assert_eq!(queue.context_track_count(), 0);
        assert_eq!(queue.context_position(), 0);
    }

    #[test]
    fn test_clear_context_preserves_user_queue() {
        let mut queue = PlaybackQueue::new();
        queue.add_to_up_next(make_entry("spotify:track:user", "User Track", "Artist"));
        queue.set_context(
            make_album_context("spotify:album:abc", "Album"),
            vec!["spotify:track:1".to_string()],
        );
        queue.clear_context();

        assert_eq!(queue.up_next_count(), 1);
    }

    #[test]
    fn test_remaining_context_tracks() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec![
                "spotify:track:1".to_string(),
                "spotify:track:2".to_string(),
                "spotify:track:3".to_string(),
            ],
        );

        assert_eq!(queue.remaining_context_tracks(), 3);
        queue.advance();
        assert_eq!(queue.remaining_context_tracks(), 2);
        queue.advance();
        assert_eq!(queue.remaining_context_tracks(), 1);
        queue.advance();
        assert_eq!(queue.remaining_context_tracks(), 0);
    }

    // -----------------------------------------------------------------------
    // PlaybackQueue: advance
    // -----------------------------------------------------------------------

    #[test]
    fn test_advance_context_tracks_in_order() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec![
                "spotify:track:1".to_string(),
                "spotify:track:2".to_string(),
                "spotify:track:3".to_string(),
            ],
        );

        assert_eq!(queue.advance(), Some("spotify:track:1".to_string()));
        assert_eq!(queue.advance(), Some("spotify:track:2".to_string()));
        assert_eq!(queue.advance(), Some("spotify:track:3".to_string()));
        assert_eq!(queue.advance(), None);
    }

    #[test]
    fn test_advance_exhausted_returns_none() {
        let mut queue = PlaybackQueue::new();
        assert_eq!(queue.advance(), None);
        assert_eq!(queue.advance(), None); // still none
    }

    #[test]
    fn test_advance_sets_current_source_context() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec!["spotify:track:1".to_string()],
        );

        queue.advance();
        assert_eq!(queue.current_source(), CurrentSource::Context);
    }

    #[test]
    fn test_advance_exhausted_sets_source_none() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec!["spotify:track:1".to_string()],
        );

        queue.advance(); // context track
        assert_eq!(queue.current_source(), CurrentSource::Context);

        queue.advance(); // exhausted
        assert_eq!(queue.current_source(), CurrentSource::None);
    }

    // -----------------------------------------------------------------------
    // PlaybackQueue: user queue priority
    // -----------------------------------------------------------------------

    #[test]
    fn test_user_queue_plays_before_context() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec!["spotify:track:context".to_string()],
        );
        queue.add_to_up_next(make_entry("spotify:track:user", "User", "Artist"));

        // User queue plays first
        assert_eq!(queue.advance(), Some("spotify:track:user".to_string()));
        assert_eq!(queue.current_source(), CurrentSource::UpNext);

        // Then context
        assert_eq!(queue.advance(), Some("spotify:track:context".to_string()));
        assert_eq!(queue.current_source(), CurrentSource::Context);
    }

    #[test]
    fn test_add_to_up_front_plays_immediately() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec!["spotify:track:1".to_string()],
        );
        queue.add_to_up_front(make_entry("spotify:track:front", "Front", "Artist"));

        assert_eq!(queue.advance(), Some("spotify:track:front".to_string()));
        assert_eq!(queue.advance(), Some("spotify:track:1".to_string()));
    }

    #[test]
    fn test_add_to_up_next_fifo_order() {
        let mut queue = PlaybackQueue::new();
        queue.add_to_up_next(make_entry("spotify:track:1", "A", ""));
        queue.add_to_up_next(make_entry("spotify:track:2", "B", ""));
        queue.add_to_up_next(make_entry("spotify:track:3", "C", ""));

        assert_eq!(queue.advance(), Some("spotify:track:1".to_string()));
        assert_eq!(queue.advance(), Some("spotify:track:2".to_string()));
        assert_eq!(queue.advance(), Some("spotify:track:3".to_string()));
        assert_eq!(queue.advance(), None);
    }

    #[test]
    fn test_add_to_up_front_lifo_order() {
        let mut queue = PlaybackQueue::new();
        queue.add_to_up_front(make_entry("spotify:track:1", "A", ""));
        queue.add_to_up_front(make_entry("spotify:track:2", "B", ""));
        queue.add_to_up_front(make_entry("spotify:track:3", "C", ""));

        // Most recently added to front plays first
        assert_eq!(queue.advance(), Some("spotify:track:3".to_string()));
        assert_eq!(queue.advance(), Some("spotify:track:2".to_string()));
        assert_eq!(queue.advance(), Some("spotify:track:1".to_string()));
    }

    #[test]
    fn test_mixed_front_and_back_adds() {
        let mut queue = PlaybackQueue::new();
        queue.add_to_up_next(make_entry("spotify:track:back", "Back", ""));
        queue.add_to_up_front(make_entry("spotify:track:front", "Front", ""));

        // Front plays first, then back
        assert_eq!(queue.advance(), Some("spotify:track:front".to_string()));
        assert_eq!(queue.advance(), Some("spotify:track:back".to_string()));
    }

    #[test]
    fn test_user_queue_exhausted_falls_to_context() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec!["spotify:track:ctx".to_string()],
        );
        queue.add_to_up_next(make_entry("spotify:track:user", "User", ""));

        assert_eq!(queue.advance(), Some("spotify:track:user".to_string()));
        assert_eq!(queue.advance(), Some("spotify:track:ctx".to_string()));
        assert_eq!(queue.advance(), None);
    }

    // -----------------------------------------------------------------------
    // PlaybackQueue: peek_next
    // -----------------------------------------------------------------------

    #[test]
    fn test_peek_next_returns_context_without_advancing() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec!["spotify:track:1".to_string()],
        );

        assert_eq!(queue.peek_next(), Some("spotify:track:1".to_string()));
        assert_eq!(queue.peek_next(), Some("spotify:track:1".to_string())); // same
        assert_eq!(queue.context_position(), 0); // not advanced
    }

    #[test]
    fn test_peek_next_prefers_user_queue() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec!["spotify:track:ctx".to_string()],
        );
        queue.add_to_up_next(make_entry("spotify:track:user", "User", ""));

        assert_eq!(queue.peek_next(), Some("spotify:track:user".to_string()));
    }

    #[test]
    fn test_peek_next_returns_none_when_exhausted() {
        let queue = PlaybackQueue::new();
        assert_eq!(queue.peek_next(), None);
    }

    // -----------------------------------------------------------------------
    // PlaybackQueue: user queue management
    // -----------------------------------------------------------------------

    #[test]
    fn test_remove_from_up_next() {
        let mut queue = PlaybackQueue::new();
        queue.add_to_up_next(make_entry("spotify:track:1", "A", ""));
        queue.add_to_up_next(make_entry("spotify:track:2", "B", ""));
        queue.add_to_up_next(make_entry("spotify:track:3", "C", ""));

        // Remove middle item
        let removed = queue.remove_from_up_next(1);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().uri, "spotify:track:2");
        assert_eq!(queue.up_next_count(), 2);
    }

    #[test]
    fn test_remove_from_up_next_out_of_bounds() {
        let mut queue = PlaybackQueue::new();
        queue.add_to_up_next(make_entry("spotify:track:1", "A", ""));

        assert!(queue.remove_from_up_next(5).is_none());
        assert_eq!(queue.up_next_count(), 1);
    }

    #[test]
    fn test_clear_up_next() {
        let mut queue = PlaybackQueue::new();
        queue.add_to_up_next(make_entry("spotify:track:1", "A", ""));
        queue.add_to_up_next(make_entry("spotify:track:2", "B", ""));
        queue.clear_up_next();

        assert!(queue.is_up_next_empty());
        assert_eq!(queue.up_next_count(), 0);
    }

    #[test]
    fn test_add_all_to_up_next() {
        let mut queue = PlaybackQueue::new();
        queue.add_all_to_up_next(vec![
            make_entry("spotify:track:1", "A", ""),
            make_entry("spotify:track:2", "B", ""),
        ]);

        assert_eq!(queue.up_next_count(), 2);
        assert_eq!(queue.advance(), Some("spotify:track:1".to_string()));
        assert_eq!(queue.advance(), Some("spotify:track:2".to_string()));
    }

    // -----------------------------------------------------------------------
    // PlaybackQueue: shuffle
    // -----------------------------------------------------------------------

    #[test]
    fn test_shuffle_reorders_context() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec![
                "spotify:track:1".to_string(),
                "spotify:track:2".to_string(),
                "spotify:track:3".to_string(),
                "spotify:track:4".to_string(),
                "spotify:track:5".to_string(),
            ],
        );

        // Enable shuffle — this will reorder context_tracks
        queue.set_shuffle(true);
        assert!(queue.shuffle());

        // All original tracks should still be present (just reordered)
        let mut collected = Vec::new();
        while let Some(uri) = queue.advance() {
            collected.push(uri);
        }

        collected.sort();
        let mut expected = vec![
            "spotify:track:1".to_string(),
            "spotify:track:2".to_string(),
            "spotify:track:3".to_string(),
            "spotify:track:4".to_string(),
            "spotify:track:5".to_string(),
        ];
        expected.sort();
        assert_eq!(collected, expected);
    }

    #[test]
    fn test_shuffle_does_not_affect_user_queue() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec!["spotify:track:1".to_string(), "spotify:track:2".to_string()],
        );

        // Add user tracks
        queue.add_to_up_next(make_entry("spotify:track:user1", "User1", ""));
        queue.add_to_up_next(make_entry("spotify:track:user2", "User2", ""));

        queue.set_shuffle(true);

        // User tracks should still play first, in order
        assert_eq!(queue.advance(), Some("spotify:track:user1".to_string()));
        assert_eq!(queue.advance(), Some("spotify:track:user2".to_string()));

        // Then shuffled context tracks
        let ctx1 = queue.advance().unwrap();
        let ctx2 = queue.advance().unwrap();
        assert!(ctx1.starts_with("spotify:track:"));
        assert!(ctx2.starts_with("spotify:track:"));
    }

    #[test]
    fn test_shuffle_on_set_context() {
        let mut queue = PlaybackQueue::new();
        queue.set_shuffle(true);

        // Setting context while shuffle is on should shuffle immediately
        queue.set_context(
            make_album_context("", ""),
            vec![
                "spotify:track:1".to_string(),
                "spotify:track:2".to_string(),
                "spotify:track:3".to_string(),
            ],
        );

        assert!(queue.shuffle());
        // All tracks should be available
        assert_eq!(queue.remaining_context_tracks(), 3);
    }

    #[test]
    fn test_shuffle_single_track_noop() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec!["spotify:track:1".to_string()],
        );
        queue.set_shuffle(true); // should not panic

        assert_eq!(queue.advance(), Some("spotify:track:1".to_string()));
    }

    #[test]
    fn test_shuffle_empty_tracks_noop() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(make_album_context("", ""), vec![]);
        queue.set_shuffle(true); // should not panic

        assert_eq!(queue.advance(), None);
    }

    // -----------------------------------------------------------------------
    // PlaybackQueue: queue view
    // -----------------------------------------------------------------------

    #[test]
    fn test_get_queue_view_empty() {
        let queue = PlaybackQueue::new();
        let view = queue.get_queue_view(None, 10);

        assert!(view.is_empty());
        assert_eq!(view.upcoming_count(), 0);
        assert_eq!(view.context_name, "");
    }

    #[test]
    fn test_get_queue_view_with_context() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_playlist_context("spotify:playlist:abc", "Chill Mix"),
            vec![
                "spotify:track:1".to_string(),
                "spotify:track:2".to_string(),
                "spotify:track:3".to_string(),
            ],
        );
        queue.advance(); // play first track

        let view = queue.get_queue_view(None, 10);

        assert_eq!(view.context_name, "Playlist: Chill Mix");
        assert_eq!(view.context_position, 1);
        assert_eq!(view.context_total, 3);
        assert_eq!(view.next_from_context.len(), 2);
        assert_eq!(view.hidden_context_tracks(), 0);
    }

    #[test]
    fn test_get_queue_view_limits_context_preview() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec![
                "spotify:track:1".to_string(),
                "spotify:track:2".to_string(),
                "spotify:track:3".to_string(),
                "spotify:track:4".to_string(),
                "spotify:track:5".to_string(),
            ],
        );

        let view = queue.get_queue_view(None, 2);

        assert_eq!(view.next_from_context.len(), 2);
        assert_eq!(view.hidden_context_tracks(), 3); // 5 - 0 - 2 = 3
    }

    #[test]
    fn test_get_queue_view_includes_user_queue() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec!["spotify:track:ctx".to_string()],
        );
        queue.add_to_up_next(make_entry("spotify:track:user", "User", "Artist"));

        let view = queue.get_queue_view(None, 10);

        assert_eq!(view.up_next.len(), 1);
        assert_eq!(view.up_next[0].uri, "spotify:track:user");
    }

    #[test]
    fn test_get_queue_view_with_now_playing() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec!["spotify:track:1".to_string()],
        );

        let now_playing = make_entry("spotify:track:now", "Now Playing", "Artist");
        let view = queue.get_queue_view(Some(now_playing), 10);

        assert!(view.now_playing.is_some());
        assert_eq!(view.now_playing.unwrap().name, "Now Playing");
    }

    // -----------------------------------------------------------------------
    // PlaybackQueue: state queries
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_exhausted_empty_queue() {
        let queue = PlaybackQueue::new();
        assert!(queue.is_exhausted());
    }

    #[test]
    fn test_is_exhausted_with_context() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec!["spotify:track:1".to_string()],
        );
        assert!(!queue.is_exhausted());

        queue.advance();
        assert!(queue.is_exhausted());
    }

    #[test]
    fn test_is_exhausted_with_user_queue() {
        let mut queue = PlaybackQueue::new();
        queue.add_to_up_next(make_entry("spotify:track:user", "User", ""));
        assert!(!queue.is_exhausted());

        queue.advance();
        assert!(queue.is_exhausted());
    }

    #[test]
    fn test_remaining_count() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec!["spotify:track:1".to_string(), "spotify:track:2".to_string()],
        );
        queue.add_to_up_next(make_entry("spotify:track:user", "User", ""));

        assert_eq!(queue.remaining_count(), 3); // 1 user + 2 context

        queue.advance(); // user track
        assert_eq!(queue.remaining_count(), 2);

        queue.advance(); // context track 1
        assert_eq!(queue.remaining_count(), 1);

        queue.advance(); // context track 2
        assert_eq!(queue.remaining_count(), 0);
    }

    // -----------------------------------------------------------------------
    // PlaybackQueue: reset
    // -----------------------------------------------------------------------

    #[test]
    fn test_reset_clears_everything() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec!["spotify:track:1".to_string()],
        );
        queue.add_to_up_next(make_entry("spotify:track:user", "User", ""));
        queue.set_shuffle(true);
        queue.advance();

        queue.reset();

        assert!(queue.is_exhausted());
        assert!(!queue.has_context());
        assert!(queue.is_up_next_empty());
        assert_eq!(queue.current_source(), CurrentSource::None);
        assert!(!queue.shuffle());
    }

    // -----------------------------------------------------------------------
    // PlaybackQueue: current_track_uri
    // -----------------------------------------------------------------------

    #[test]
    fn test_current_track_uri_context_source() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec!["spotify:track:1".to_string(), "spotify:track:2".to_string()],
        );

        assert_eq!(queue.current_track_uri(), None); // not advanced yet

        queue.advance();
        assert_eq!(
            queue.current_track_uri(),
            Some("spotify:track:1".to_string())
        );

        queue.advance();
        assert_eq!(
            queue.current_track_uri(),
            Some("spotify:track:2".to_string())
        );
    }

    #[test]
    fn test_current_track_uri_up_next_source_returns_none() {
        let mut queue = PlaybackQueue::new();
        queue.add_to_up_next(make_entry("spotify:track:user", "User", ""));

        queue.advance();
        // up_next tracks are removed on advance, so we can't recover the URI
        assert_eq!(queue.current_track_uri(), None);
    }

    // -----------------------------------------------------------------------
    // PlaybackQueue: set_current_source
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_current_source() {
        let mut queue = PlaybackQueue::new();
        queue.set_current_source(CurrentSource::Context);
        assert_eq!(queue.current_source(), CurrentSource::Context);

        queue.set_current_source(CurrentSource::UpNext);
        assert_eq!(queue.current_source(), CurrentSource::UpNext);
    }

    // -----------------------------------------------------------------------
    // QueueView helpers
    // -----------------------------------------------------------------------

    #[test]
    fn test_queue_view_is_empty() {
        let view = QueueView::default();
        assert!(view.is_empty());
    }

    #[test]
    fn test_queue_view_upcoming_count() {
        let view = QueueView {
            up_next: vec![
                make_entry("spotify:track:1", "A", ""),
                make_entry("spotify:track:2", "B", ""),
            ],
            next_from_context: vec![
                "spotify:track:3".to_string(),
                "spotify:track:4".to_string(),
                "spotify:track:5".to_string(),
            ],
            ..Default::default()
        };

        assert_eq!(view.upcoming_count(), 5);
    }

    #[test]
    fn test_queue_view_hidden_context_tracks() {
        let view = QueueView {
            context_position: 2,
            context_total: 10,
            next_from_context: vec!["spotify:track:3".to_string(), "spotify:track:4".to_string()],
            ..Default::default()
        };

        // 10 total - 2 position = 8 remaining, 2 shown = 6 hidden
        assert_eq!(view.hidden_context_tracks(), 6);
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_advance_with_empty_context_tracks() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(make_album_context("", ""), vec![]);

        assert_eq!(queue.advance(), None);
        assert!(queue.is_exhausted());
    }

    #[test]
    fn test_context_position_does_not_overflow() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec!["spotify:track:1".to_string()],
        );

        // Advance past the end — should not panic
        queue.advance(); // position = 1, returns track
        queue.advance(); // position = 1, returns None (exhausted)
        queue.advance(); // position = 1, returns None (still exhausted)

        // Position stays at 1 (the position after the last successful advance)
        assert_eq!(queue.context_position(), 1);
        assert!(queue.is_exhausted());
    }

    #[test]
    fn test_add_to_queue_when_exhausted() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec!["spotify:track:1".to_string()],
        );
        queue.advance(); // exhaust
        assert!(queue.is_exhausted());

        // Add a new track
        queue.add_to_up_next(make_entry("spotify:track:new", "New", ""));
        assert!(!queue.is_exhausted());
        assert_eq!(queue.advance(), Some("spotify:track:new".to_string()));
    }

    #[test]
    fn test_multiple_context_switches() {
        let mut queue = PlaybackQueue::new();

        // First context
        queue.set_context(
            make_album_context("spotify:album:1", "Album 1"),
            vec!["spotify:track:a1".to_string()],
        );
        assert_eq!(queue.advance(), Some("spotify:track:a1".to_string()));

        // Switch to second context
        queue.set_context(
            make_playlist_context("spotify:playlist:2", "Playlist 2"),
            vec![
                "spotify:track:p1".to_string(),
                "spotify:track:p2".to_string(),
            ],
        );
        assert_eq!(queue.context().name(), "Playlist 2");
        assert_eq!(queue.context_position(), 0);
        assert_eq!(queue.advance(), Some("spotify:track:p1".to_string()));
        assert_eq!(queue.advance(), Some("spotify:track:p2".to_string()));
    }

    #[test]
    fn test_user_queue_persists_across_context_switches() {
        let mut queue = PlaybackQueue::new();
        queue.set_context(
            make_album_context("", ""),
            vec!["spotify:track:ctx1".to_string()],
        );
        queue.add_to_up_next(make_entry("spotify:track:user", "User", ""));

        // Switch context
        queue.set_context(
            make_album_context("", ""),
            vec!["spotify:track:ctx2".to_string()],
        );

        // User queue still plays first
        assert_eq!(queue.advance(), Some("spotify:track:user".to_string()));
        // Then new context
        assert_eq!(queue.advance(), Some("spotify:track:ctx2".to_string()));
    }

    // -----------------------------------------------------------------------
    // Auto-Advance and Queue Management Tests
    // -----------------------------------------------------------------------

    /// Test that queue correctly tracks remaining tracks for auto-advance scenarios
    #[test]
    fn test_queue_remaining_tracks_calculation() {
        let mut queue = PlaybackQueue::new();
        
        queue.set_context(
            make_playlist_context("spotify:playlist:test", "Test"),
            vec![
                "spotify:track:1".to_string(),
                "spotify:track:2".to_string(),
                "spotify:track:3".to_string(),
            ],
        );
        
        // Initially 3 remaining
        assert_eq!(queue.remaining_context_tracks(), 3);
        
        // After playing track 1
        queue.advance();
        assert_eq!(queue.remaining_context_tracks(), 2);
        
        // After playing track 2
        queue.advance();
        assert_eq!(queue.remaining_context_tracks(), 1);
        
        // After playing track 3
        queue.advance();
        assert_eq!(queue.remaining_context_tracks(), 0);
        
        // Exhausted
        assert_eq!(queue.advance(), None);
    }

    /// Test queue behavior when user adds tracks during playback
    #[test]
    fn test_queue_user_interruption_during_playback() {
        let mut queue = PlaybackQueue::new();
        
        // Set up context
        queue.set_context(
            make_playlist_context("spotify:playlist:test", "Test"),
            vec![
                "spotify:track:ctx1".to_string(),
                "spotify:track:ctx2".to_string(),
            ],
        );
        
        // Start playing context
        assert_eq!(queue.advance(), Some("spotify:track:ctx1".to_string()));
        
        // User adds track to queue during playback
        queue.add_to_up_next(make_entry("spotify:track:user1", "User Track", "Artist"));
        
        // User track plays next (priority over context)
        assert_eq!(queue.advance(), Some("spotify:track:user1".to_string()));
        
        // Then context continues
        assert_eq!(queue.advance(), Some("spotify:track:ctx2".to_string()));
    }

    /// Test queue exhaustion detection
    #[test]
    fn test_queue_exhaustion_detection() {
        let mut queue = PlaybackQueue::new();
        
        // Empty queue is exhausted
        assert!(queue.is_exhausted());
        
        // Add user track
        queue.add_to_up_next(make_entry("spotify:track:user", "User", "Artist"));
        assert!(!queue.is_exhausted());
        
        // Play user track
        queue.advance();
        assert!(queue.is_exhausted());
        
        // Add context
        queue.set_context(
            make_playlist_context("spotify:playlist:test", "Test"),
            vec!["spotify:track:ctx".to_string()],
        );
        assert!(!queue.is_exhausted());
        
        // Play context track
        queue.advance();
        assert!(queue.is_exhausted());
    }

    /// Test queue advance returns correct source type
    #[test]
    fn test_queue_advance_source_tracking() {
        let mut queue = PlaybackQueue::new();
        
        queue.add_to_up_next(make_entry("spotify:track:user", "User", "Artist"));
        queue.set_context(
            make_playlist_context("spotify:playlist:test", "Test"),
            vec!["spotify:track:ctx".to_string()],
        );
        
        // User track should be from UpNext
        queue.advance();
        assert_eq!(queue.current_source(), CurrentSource::UpNext);
        
        // Context track should be from Context
        queue.advance();
        assert_eq!(queue.current_source(), CurrentSource::Context);
        
        // Exhausted
        queue.advance();
        assert_eq!(queue.current_source(), CurrentSource::None);
    }

    /// Test queue with shuffle enabled
    #[test]
    fn test_queue_shuffle_preserves_up_next() {
        let mut queue = PlaybackQueue::new();
        
        queue.set_context(
            make_playlist_context("spotify:playlist:test", "Test"),
            vec![
                "spotify:track:1".to_string(),
                "spotify:track:2".to_string(),
                "spotify:track:3".to_string(),
            ],
        );
        
        // Add user track before shuffle
        queue.add_to_up_front(make_entry("spotify:track:user", "User", "Artist"));
        
        // Enable shuffle
        queue.set_shuffle(true);
        
        // User track should still play first (shuffle doesn't affect up_next)
        let next = queue.advance();
        assert_eq!(next, Some("spotify:track:user".to_string()));
        
        // Context tracks are shuffled (we can't predict order, but should get 3 tracks)
        let mut found_tracks = vec![];
        while let Some(uri) = queue.advance() {
            found_tracks.push(uri);
        }
        assert_eq!(found_tracks.len(), 3);
        assert!(found_tracks.contains(&"spotify:track:1".to_string()));
        assert!(found_tracks.contains(&"spotify:track:2".to_string()));
        assert!(found_tracks.contains(&"spotify:track:3".to_string()));
    }

    /// Test queue maintains correct total count
    #[test]
    fn test_queue_total_remaining_count() {
        let mut queue = PlaybackQueue::new();
        
        // Add user tracks
        queue.add_to_up_next(make_entry("spotify:track:user1", "User 1", "Artist"));
        queue.add_to_up_next(make_entry("spotify:track:user2", "User 2", "Artist"));
        
        // Set context
        queue.set_context(
            make_playlist_context("spotify:playlist:test", "Test"),
            vec![
                "spotify:track:ctx1".to_string(),
                "spotify:track:ctx2".to_string(),
            ],
        );
        
        // 2 user + 2 context = 4 total
        assert_eq!(queue.remaining_count(), 4);
        
        // After playing user tracks
        queue.advance();
        queue.advance();
        assert_eq!(queue.remaining_count(), 2);
        
        // After playing context tracks
        queue.advance();
        queue.advance();
        assert_eq!(queue.remaining_count(), 0);
    }
}
