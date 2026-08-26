//! Spotify API client wrapper
//!
//! Modular design:
//! - `client` - Core client setup and authentication
//! - `playback` - Playback control methods
//! - `library` - Library, playlists, and search

mod client;
mod library;
mod playback;

pub use client::SpotifyClient;
