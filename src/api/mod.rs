//! Spotify API client wrapper
//!
//! Modular design:
//! - `client` - Core client setup and authentication
//! - `playback` - Playback control methods
//! - `library` - Library, playlists, and search

mod client;
#[cfg(test)]
pub(crate) mod fake_spotify;
mod library;
pub mod playback;

pub use client::SpotifyClient;
pub use playback::select_play_device;
