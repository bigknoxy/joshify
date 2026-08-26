//! Playback domain model
//!
//! Core abstractions for Spotify-style context playback and queue management:
//! - `PlaybackQueue` — Two-tier queue (user queue + context tracks)
//! - `PlaybackContext` — Playlist/Album/Artist/None context types
//! - `QueueEntry` — Track metadata for queue items
//! - `QueueView` — Point-in-time snapshot for UI rendering

pub mod domain;

pub use domain::{CurrentSource, PlaybackContext, PlaybackQueue, QueueEntry, QueueView};

/// Whether playback is local (librespot) or remote (Spotify Connect).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackMode {
    #[default]
    Local,
    Remote,
}
