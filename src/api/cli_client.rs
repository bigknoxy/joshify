//! Mockable client trait for CLI commands
//!
//! The CLI handler needs to call Spotify API methods, but we want to unit
//! test it without hitting the network. This trait abstracts the playback
//! and library operations the CLI needs, and is implemented by
//! [`SpotifyClient`]. Tests use a mockall-generated mock.

use anyhow::Result;
use async_trait::async_trait;
use rspotify::model::{CurrentPlaybackContext, FullTrack, RepeatState};

use super::SpotifyClient;

/// Operations the CLI needs from the Spotify client.
///
/// Kept intentionally small — only the methods used by `CliHandler`.
#[async_trait]
pub trait CliClient: Send + Sync {
    /// Get current playback state (None if nothing is playing).
    async fn current_playback(&self) -> Result<Option<CurrentPlaybackContext>>;

    /// Pause playback.
    async fn playback_pause(&self) -> Result<()>;

    /// Resume playback.
    async fn playback_resume(&self) -> Result<()>;

    /// Skip to next track.
    async fn playback_next(&self) -> Result<()>;

    /// Skip to previous track.
    async fn playback_previous(&self) -> Result<()>;

    /// Set volume (0-100).
    async fn set_volume(&self, volume_percent: u32) -> Result<()>;

    /// Seek to a position in milliseconds.
    async fn seek(&self, position_ms: u32, device_id: Option<String>) -> Result<()>;

    /// Toggle shuffle state.
    async fn toggle_shuffle(&self, shuffle: bool) -> Result<()>;

    /// Set repeat mode.
    async fn set_repeat(&self, state: RepeatState) -> Result<()>;

    /// Start playback of the given URIs.
    async fn start_playback(&self, uris: Vec<String>, offset: Option<u32>) -> Result<()>;

    /// Search for tracks.
    async fn search(&self, query: &str, limit: u32) -> Result<Vec<FullTrack>>;

    /// Add a track to the queue.
    async fn add_to_queue(&self, track_uri: &str) -> Result<()>;
}

#[async_trait]
impl CliClient for SpotifyClient {
    async fn current_playback(&self) -> Result<Option<CurrentPlaybackContext>> {
        SpotifyClient::current_playback(self).await
    }

    async fn playback_pause(&self) -> Result<()> {
        SpotifyClient::playback_pause(self).await
    }

    async fn playback_resume(&self) -> Result<()> {
        SpotifyClient::playback_resume(self).await
    }

    async fn playback_next(&self) -> Result<()> {
        SpotifyClient::playback_next(self).await
    }

    async fn playback_previous(&self) -> Result<()> {
        SpotifyClient::playback_previous(self).await
    }

    async fn set_volume(&self, volume_percent: u32) -> Result<()> {
        SpotifyClient::set_volume(self, volume_percent).await
    }

    async fn seek(&self, position_ms: u32, device_id: Option<String>) -> Result<()> {
        SpotifyClient::seek(self, position_ms, device_id.as_deref()).await
    }

    async fn toggle_shuffle(&self, shuffle: bool) -> Result<()> {
        SpotifyClient::toggle_shuffle(self, shuffle).await
    }

    async fn set_repeat(&self, state: RepeatState) -> Result<()> {
        SpotifyClient::set_repeat(self, state).await
    }

    async fn start_playback(&self, uris: Vec<String>, offset: Option<u32>) -> Result<()> {
        SpotifyClient::start_playback(self, uris, offset).await
    }

    async fn search(&self, query: &str, limit: u32) -> Result<Vec<FullTrack>> {
        SpotifyClient::search(self, query, limit).await
    }

    async fn add_to_queue(&self, track_uri: &str) -> Result<()> {
        SpotifyClient::add_to_queue(self, track_uri).await
    }
}
