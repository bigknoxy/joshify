//! Spotify API client wrapper
//!
//! Modular design:
//! - `client` - Core client setup and authentication
//! - `playback` - Playback control methods
//! - `library` - Library, playlists, and search
//! - `rate_limit` - Rate limit handling with exponential backoff

mod client;
mod library;
mod playback;
pub mod rate_limit;

pub use client::SpotifyClient;
