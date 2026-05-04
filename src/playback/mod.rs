//! Playback domain model and service layer
//!
//! # Domain Model (`domain`)
//!
//! Core abstractions for Spotify-style context playback and queue management:
//! - `PlaybackQueue` — Two-tier queue (user queue + context tracks)
//! - `PlaybackContext` — Playlist/Album/Artist/None context types
//! - `QueueEntry` — Track metadata for queue items
//! - `QueueView` — Point-in-time snapshot for UI rendering
//!
//! # Service Layer (`service`)
//!
//! Trait-based abstraction over local (librespot) and remote (rspotify) playback.
//! Provides `PlaybackService` trait with `SpotifyPlaybackService` and
//! `LocalPlaybackService` implementations.

pub mod domain;
pub mod service;

pub use domain::{CurrentSource, PlaybackContext, PlaybackQueue, QueueEntry, QueueView};
pub use service::*;
