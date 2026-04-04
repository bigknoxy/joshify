//! Audio player wrapper for librespot
//!
//! Provides a high-level interface for local Spotify playback
//! with event-driven updates for the TUI.

use anyhow::{Context, Result};
use librespot::{
    core::{SpotifyId, SpotifyUri},
    playback::{
        audio_backend,
        config::{AudioFormat, PlayerConfig},
        mixer::{self, MixerConfig},
        player::{Player, PlayerEvent},
    },
};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

/// Playback state for the TUI
#[derive(Debug, Clone, Default)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub current_track_uri: Option<String>,
    pub current_track_name: Option<String>,
    pub current_artist_name: Option<String>,
    pub progress_ms: u32,
    pub duration_ms: u32,
    pub volume: u16, // 0-65535
}

/// Local audio player backed by librespot
pub struct LocalPlayer {
    player: Arc<Player>,
    event_rx: Option<UnboundedReceiver<PlayerEvent>>,
    pub state: PlaybackState,
}

impl LocalPlayer {
    /// Create a new player with the default audio backend
    pub fn new(session: &librespot::core::session::Session) -> Result<Self> {
        let backend = audio_backend::find(None).context(
            "No audio backend available. Install ALSA (Linux) or ensure audio drivers are present.",
        )?;
        let mixer_builder = mixer::find(None).context("No mixer available")?;
        let mixer_config = MixerConfig::default();
        let mixer = mixer_builder(mixer_config).context("Failed to create mixer")?;

        let player_config = PlayerConfig::default();
        let audio_format = AudioFormat::default();

        let player = Player::new(
            player_config,
            session.clone(),
            mixer.get_soft_volume(),
            move || backend(None, audio_format),
        );

        let event_rx = player.get_player_event_channel();

        Ok(Self {
            player,
            event_rx: Some(event_rx),
            state: PlaybackState::default(),
        })
    }

    /// Load and optionally play a track by Spotify URI string
    pub fn load_uri(&self, uri: &str, start_playing: bool, position_ms: u32) -> Result<()> {
        let spotify_uri = Self::parse_uri(uri).context("Failed to parse Spotify URI")?;
        self.player.load(spotify_uri, start_playing, position_ms);
        Ok(())
    }

    /// Play the current track
    pub fn play(&self) {
        self.player.play();
    }

    /// Pause the current track
    pub fn pause(&self) {
        self.player.pause();
    }

    /// Stop playback
    pub fn stop(&self) {
        self.player.stop();
    }

    /// Seek to position in milliseconds
    pub fn seek(&self, position_ms: u32) {
        self.player.seek(position_ms);
    }

    /// Set volume (0-65535)
    pub fn set_volume(&self, volume: u16) {
        self.player.emit_volume_changed_event(volume);
    }

    /// Get the event channel for TUI updates
    pub fn take_event_channel(&mut self) -> Option<UnboundedReceiver<PlayerEvent>> {
        self.event_rx.take()
    }

    /// Update state from a player event
    pub fn handle_event(&mut self, event: PlayerEvent) {
        use PlayerEvent::*;
        match event {
            Playing {
                track_id,
                position_ms,
                ..
            } => {
                self.state.is_playing = true;
                self.state.current_track_uri = Some(track_id.to_uri());
                self.state.progress_ms = position_ms;
            }
            Paused {
                track_id,
                position_ms,
                ..
            } => {
                self.state.is_playing = false;
                self.state.current_track_uri = Some(track_id.to_uri());
                self.state.progress_ms = position_ms;
            }
            Stopped { .. } => {
                self.state.is_playing = false;
            }
            EndOfTrack { .. } => {
                self.state.is_playing = false;
            }
            TrackChanged { audio_item } => {
                self.state.current_track_name = Some(audio_item.name.clone());
                self.state.duration_ms = audio_item.duration_ms;
                self.state.current_track_uri = Some(audio_item.uri.clone());
            }
            VolumeChanged { volume } => {
                self.state.volume = volume;
            }
            Seeked { position_ms, .. } => {
                self.state.progress_ms = position_ms;
            }
            PositionChanged { position_ms, .. } | PositionCorrection { position_ms, .. } => {
                self.state.progress_ms = position_ms;
            }
            Loading {
                track_id,
                position_ms,
                ..
            } => {
                self.state.current_track_uri = Some(track_id.to_uri());
                self.state.progress_ms = position_ms;
            }
            _ => {}
        }
    }

    /// Parse a Spotify URI string into a SpotifyUri
    fn parse_uri(uri: &str) -> Result<SpotifyUri> {
        // Handle "spotify:track:BASE62ID" format
        if let Some(id) = uri.strip_prefix("spotify:track:") {
            let track_id = SpotifyId::from_base62(id).context("Invalid track ID format")?;
            return Ok(SpotifyUri::Track { id: track_id });
        }

        // Handle "spotify:episode:BASE62ID" format
        if let Some(id) = uri.strip_prefix("spotify:episode:") {
            let episode_id = SpotifyId::from_base62(id).context("Invalid episode ID format")?;
            return Ok(SpotifyUri::Episode { id: episode_id });
        }

        // Handle full URI format
        if uri.starts_with("spotify:") {
            return SpotifyUri::from_uri(uri).map_err(|e| anyhow::anyhow!(e));
        }

        // Assume it's a base62 track ID
        let track_id = SpotifyId::from_base62(uri).context("Invalid URI format")?;
        Ok(SpotifyUri::Track { id: track_id })
    }
}

/// Shared player type for use across the app
pub type SharedPlayer = Arc<LocalPlayer>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_state_defaults() {
        let state = PlaybackState::default();
        assert!(!state.is_playing);
        assert!(state.current_track_uri.is_none());
        assert!(state.current_track_name.is_none());
        assert!(state.current_artist_name.is_none());
        assert_eq!(state.progress_ms, 0);
        assert_eq!(state.duration_ms, 0);
        assert_eq!(state.volume, 0);
    }

    #[test]
    fn parse_uri_track_uri_format() {
        let uri = LocalPlayer::parse_uri("spotify:track:4uLU6hMCjMI75M1A2tKUQC").unwrap();
        match uri {
            SpotifyUri::Track { id } => {
                assert_eq!(id.to_base62(), "4uLU6hMCjMI75M1A2tKUQC");
            }
            _ => panic!("Expected Track variant"),
        }
    }

    #[test]
    fn parse_uri_episode_uri_format() {
        let uri = LocalPlayer::parse_uri("spotify:episode:5Xt5DXGzch68nYYamXrNxZ").unwrap();
        match uri {
            SpotifyUri::Episode { id } => {
                assert_eq!(id.to_base62(), "5Xt5DXGzch68nYYamXrNxZ");
            }
            _ => panic!("Expected Episode variant"),
        }
    }

    #[test]
    fn parse_uri_base62_only() {
        let uri = LocalPlayer::parse_uri("4uLU6hMCjMI75M1A2tKUQC").unwrap();
        match uri {
            SpotifyUri::Track { id } => {
                assert_eq!(id.to_base62(), "4uLU6hMCjMI75M1A2tKUQC");
            }
            _ => panic!("Expected Track variant"),
        }
    }

    #[test]
    fn parse_uri_invalid_format() {
        let result = LocalPlayer::parse_uri("not-a-spotify-uri");
        assert!(result.is_err());
    }

    #[test]
    fn parse_uri_empty_string() {
        let result = LocalPlayer::parse_uri("");
        assert!(result.is_err());
    }
}
