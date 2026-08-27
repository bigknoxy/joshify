//! Spotify API client wrapper
//!
//! Modular design:
//! - `client` - Core client setup and authentication
//! - `playback` - Playback control methods
//! - `library` - Library, playlists, and search

mod client;
mod library;
pub mod playback;

pub use client::SpotifyClient;
pub use playback::select_play_device;
