//! Core playback service trait, commands, state, and implementations.

use anyhow::Result;
use async_trait::async_trait;
use rspotify::clients::OAuthClient;
use rspotify::prelude::Id;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::api::SpotifyClient;
use crate::player::LocalPlayer;
use crate::state::player_state::RepeatMode;

use super::domain::{PlaybackContext, PlaybackQueue, QueueEntry};

// ──────────────────────────────────────────────
// Playback Mode
// ──────────────────────────────────────────────

/// Whether playback is local (librespot) or remote (Spotify Connect).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackMode {
    #[default]
    Local,
    Remote,
}

// ──────────────────────────────────────────────
// Playback Command
// ──────────────────────────────────────────────

/// All playback operations expressed as a single command enum.
///
/// This enables uniform handling, logging, and potential command queues/undo.
#[derive(Debug, Clone)]
pub enum PlaybackCommand {
    /// Play a specific track, optionally within a context.
    ///
    /// When `context` is `Some`, the service should use Spotify's context playback
    /// API with `Offset::Uri(track_uri)` so that next/previous navigate within
    /// the context rather than the global queue.
    ///
    /// `context_tracks` provides the full track list for local playback fallback.
    PlayTrack {
        track_uri: String,
        context: Option<PlaybackContext>,
        /// Full track list for the context (used by local playback to determine next/prev).
        context_tracks: Option<Vec<QueueEntry>>,
        /// Start position in milliseconds.
        position_ms: u32,
    },

    /// Play the next track from the local queue, or fall back to context.
    ///
    /// If the local queue has entries, dequeue and play the front entry.
    /// Otherwise, delegate to the backend's native next-track behavior.
    PlayFromQueue,

    /// Skip to the next track.
    NextTrack,

    /// Skip to the previous track.
    PreviousTrack,

    /// Add a track to the local up-next queue.
    AddToQueue(QueueEntry),

    /// Toggle shuffle on/off.
    ToggleShuffle,

    /// Cycle through repeat modes: Off → Context → Track → Off.
    CycleRepeat,

    /// Set the repeat mode explicitly.
    SetRepeat(RepeatMode),

    /// Set the shuffle state explicitly.
    SetShuffle(bool),

    /// Seek to a position in the current track.
    Seek(u32),

    /// Set volume (0-100).
    SetVolume(u32),

    /// Toggle play/pause.
    TogglePlayPause,

    /// Pause playback.
    Pause,

    /// Resume playback.
    Resume,

    /// Stop playback entirely.
    Stop,

    /// Transfer playback to a specific device (remote mode only).
    TransferToDevice(String),
}

// ──────────────────────────────────────────────
// Playback State
// ──────────────────────────────────────────────

/// A snapshot of the current playback state, independent of UI rendering.
///
/// This is the service-layer view; the UI's `PlayerState` may derive from this.
#[derive(Debug, Clone, Default)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub current_track_uri: Option<String>,
    pub current_track_name: Option<String>,
    pub current_artist_name: Option<String>,
    pub progress_ms: u32,
    pub duration_ms: u32,
    pub volume: u32,
    pub shuffle: bool,
    pub repeat_mode: RepeatMode,
    pub available_devices: Vec<DeviceInfo>,
    pub context: Option<PlaybackContext>,
}

/// Minimal device info for the service layer.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub id: Option<String>,
    pub name: String,
    pub is_active: bool,
    pub is_restricted: bool,
    pub device_type: String,
}

// ──────────────────────────────────────────────
// Playback Error
// ──────────────────────────────────────────────

/// Service-level errors with categorization for UI display.
#[derive(Debug, thiserror::Error)]
pub enum PlaybackError {
    #[error("No active device available")]
    NoActiveDevice,

    #[error("Track not found: {0}")]
    TrackNotFound(String),

    #[error("Context not available for playback: {0}")]
    ContextUnavailable(String),

    #[error("Invalid context ID: {0}")]
    InvalidContext(String),

    #[error("Local player not initialized")]
    LocalPlayerNotReady,

    #[error("Remote client not connected")]
    RemoteClientNotReady,

    #[error("Playback operation failed: {0}")]
    OperationFailed(String),

    #[error("Rate limited by Spotify API")]
    RateLimited,

    #[error("Authentication expired")]
    AuthExpired,
}

impl PlaybackError {
    /// Whether this error is likely transient and worth retrying.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::NoActiveDevice | Self::RateLimited | Self::OperationFailed(_)
        )
    }

    /// User-facing message suitable for status bar display.
    pub fn status_message(&self) -> String {
        match self {
            Self::NoActiveDevice => "No active Spotify device. Open Spotify first.".into(),
            Self::TrackNotFound(uri) => format!("Track not found: {uri}"),
            Self::ContextUnavailable(ctx) => format!("Cannot play context: {ctx}"),
            Self::InvalidContext(msg) => format!("Invalid context: {msg}"),
            Self::LocalPlayerNotReady => "Local player not ready".into(),
            Self::RemoteClientNotReady => "Not connected to Spotify".into(),
            Self::OperationFailed(msg) => format!("Playback error: {msg}"),
            Self::RateLimited => "Rate limited — please wait".into(),
            Self::AuthExpired => "Re-authenticate to continue".into(),
        }
    }
}

// ──────────────────────────────────────────────
// Playback Service Trait
// ──────────────────────────────────────────────

/// The core playback abstraction.
///
/// All playback operations flow through this trait, enabling:
/// - Dependency inversion (main.rs depends on the trait, not concrete types)
/// - Easy mocking for tests
/// - Swapping between local and remote playback at runtime
#[async_trait]
pub trait PlaybackService: Send + Sync {
    /// Execute a playback command.
    async fn execute(&self, cmd: PlaybackCommand) -> std::result::Result<(), PlaybackError>;

    /// Get the current playback state snapshot.
    async fn get_state(&self) -> Result<PlaybackState>;

    /// Get the current playback mode.
    fn mode(&self) -> PlaybackMode;

    /// Check if the service is ready to accept commands.
    fn is_ready(&self) -> bool;
}

// ──────────────────────────────────────────────
// Spotify Playback Service (Remote)
// ──────────────────────────────────────────────

/// Remote playback via the Spotify Web API (rspotify).
///
/// Uses Spotify Connect for all operations. Supports context playback with
/// `Offset::Uri` for proper next/previous navigation within playlists/albums.
pub struct SpotifyPlaybackService {
    client: Arc<Mutex<SpotifyClient>>,
    /// Cached local queue for up-next tracks.
    queue: Arc<Mutex<PlaybackQueue>>,
    /// The current playback context (playlist/album/artist).
    current_context: Arc<Mutex<Option<PlaybackContext>>>,
}

impl SpotifyPlaybackService {
    pub fn new(client: Arc<Mutex<SpotifyClient>>) -> Self {
        Self {
            client,
            queue: Arc::new(Mutex::new(PlaybackQueue::new())),
            current_context: Arc::new(Mutex::new(None)),
        }
    }

    /// Create with an external queue reference for shared state.
    pub fn with_queue(client: Arc<Mutex<SpotifyClient>>, queue: Arc<Mutex<PlaybackQueue>>) -> Self {
        Self {
            client,
            queue,
            current_context: Arc::new(Mutex::new(None)),
        }
    }

    /// Try to play using context playback (playlist/album/artist).
    ///
    /// Falls back to simple track playback if the context ID cannot be parsed.
    /// Uses `Offset::Uri` for better compatibility with Spotify Connect devices.
    ///
    /// NOTE: We use `Offset::Uri(track_uri)` instead of `Offset::Position(index)` because:
    /// 1. The Spotify API `offset.position` expects a track index (0-based integer)
    /// 2. rspotify's `Offset::Position(Duration)` incorrectly converts to milliseconds
    /// 3. `Offset::Uri` is unambiguous and works reliably across all devices
    async fn play_with_context(
        &self,
        track_uri: &str,
        context: &PlaybackContext,
        position_ms: u32,
    ) -> std::result::Result<(), PlaybackError> {
        let guard = self.client.lock().await;

        // Ensure we have an active device
        Self::ensure_active_device(&guard).await?;

        let position = chrono::TimeDelta::milliseconds(position_ms as i64);
        // Use URI-based offset for unambiguous track selection within context
        // This is more reliable than index-based offsets which can drift
        let offset = rspotify::model::Offset::Uri(track_uri.to_string());

        match context {
            PlaybackContext::Playlist { uri, .. } => {
                let playlist_id_str = uri.strip_prefix("spotify:playlist:").unwrap_or(uri);
                match rspotify::model::PlaylistId::from_id(playlist_id_str) {
                    Ok(playlist_id) => {
                        let play_context = rspotify::model::PlayContextId::from(playlist_id);
                        tracing::info!(
                            "Starting context playback for playlist {} with offset {}",
                            playlist_id_str,
                            track_uri
                        );
                        match guard
                            .oauth
                            .start_context_playback(
                                play_context,
                                None,
                                Some(offset),
                                Some(position),
                            )
                            .await
                        {
                            Ok(()) => {
                                tracing::info!("Context playback started successfully");
                                return Ok(());
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Context playback API failed for playlist {}: {}. Falling back to simple playback.",
                                    playlist_id_str,
                                    e
                                );
                                // Only fallback on API failure, not parse failure
                                drop(guard);
                                return self.play_track_simple(track_uri, position_ms).await;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to parse playlist ID '{}': {}. Cannot start context playback.",
                            playlist_id_str,
                            e
                        );
                        return Err(PlaybackError::InvalidContext(format!(
                            "Invalid playlist ID: {}",
                            e
                        )));
                    }
                }
            }
            PlaybackContext::Album { uri, .. } => {
                let album_id_str = uri.strip_prefix("spotify:album:").unwrap_or(uri);
                match rspotify::model::AlbumId::from_id(album_id_str) {
                    Ok(album_id) => {
                        let play_context = rspotify::model::PlayContextId::from(album_id);
                        tracing::info!(
                            "Starting context playback for album {} with offset {}",
                            album_id_str,
                            track_uri
                        );
                        match guard
                            .oauth
                            .start_context_playback(
                                play_context,
                                None,
                                Some(offset),
                                Some(position),
                            )
                            .await
                        {
                            Ok(()) => {
                                tracing::info!("Album context playback started successfully");
                                return Ok(());
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Album context playback API failed for {}: {}. Falling back to simple playback.",
                                    album_id_str,
                                    e
                                );
                                drop(guard);
                                return self.play_track_simple(track_uri, position_ms).await;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to parse album ID '{}': {}. Cannot start context playback.",
                            album_id_str,
                            e
                        );
                        return Err(PlaybackError::InvalidContext(format!(
                            "Invalid album ID: {}",
                            e
                        )));
                    }
                }
            }
            PlaybackContext::Artist { uri, .. } => {
                let artist_id_str = uri.strip_prefix("spotify:artist:").unwrap_or(uri);
                match rspotify::model::ArtistId::from_id(artist_id_str) {
                    Ok(artist_id) => {
                        let play_context = rspotify::model::PlayContextId::from(artist_id);
                        tracing::info!(
                            "Starting context playback for artist {} with offset {}",
                            artist_id_str,
                            track_uri
                        );
                        match guard
                            .oauth
                            .start_context_playback(
                                play_context,
                                None,
                                Some(offset),
                                Some(position),
                            )
                            .await
                        {
                            Ok(()) => {
                                tracing::info!("Artist context playback started successfully");
                                return Ok(());
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Artist context playback API failed for {}: {}. Falling back to simple playback.",
                                    artist_id_str,
                                    e
                                );
                                drop(guard);
                                return self.play_track_simple(track_uri, position_ms).await;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to parse artist ID '{}': {}. Cannot start context playback.",
                            artist_id_str,
                            e
                        );
                        return Err(PlaybackError::InvalidContext(format!(
                            "Invalid artist ID: {}",
                            e
                        )));
                    }
                }
            }
            PlaybackContext::None => {
                // No context — use simple track playback
                tracing::info!(
                    "No context available, using simple track playback for {}",
                    track_uri
                );
                drop(guard);
                return self.play_track_simple(track_uri, position_ms).await;
            }
        }
    }

    /// Play a track without context (simple URI playback).
    async fn play_track_simple(
        &self,
        track_uri: &str,
        position_ms: u32,
    ) -> std::result::Result<(), PlaybackError> {
        let guard = self.client.lock().await;
        Self::ensure_active_device(&guard).await?;

        let playable_uris = Self::uris_to_playable(&[track_uri.to_string()])
            .ok_or_else(|| PlaybackError::TrackNotFound(track_uri.to_string()))?;

        let position = chrono::TimeDelta::milliseconds(position_ms as i64);
        guard
            .oauth
            .start_uris_playback(playable_uris, None, None, Some(position))
            .await
            .map_err(|e| PlaybackError::OperationFailed(e.to_string()))?;
        Ok(())
    }

    /// Convert string URIs to rspotify PlayableId types.
    fn uris_to_playable(uris: &[String]) -> Option<Vec<rspotify::model::PlayableId<'static>>> {
        let result: Vec<rspotify::model::PlayableId<'static>> = uris
            .iter()
            .filter_map(|uri| {
                if let Some(id) = uri.strip_prefix("spotify:track:") {
                    rspotify::model::TrackId::from_id(id.to_string())
                        .ok()
                        .map(rspotify::model::PlayableId::Track)
                } else if let Some(id) = uri.strip_prefix("spotify:episode:") {
                    rspotify::model::EpisodeId::from_id(id.to_string())
                        .ok()
                        .map(rspotify::model::PlayableId::Episode)
                } else {
                    None
                }
            })
            .collect();
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    /// Ensure there's an active Spotify device, transferring if needed.
    async fn ensure_active_device(
        client: &SpotifyClient,
    ) -> std::result::Result<(), PlaybackError> {
        let devices = client
            .available_devices()
            .await
            .map_err(|e| PlaybackError::OperationFailed(e.to_string()))?;

        if let Some(device) = devices.iter().find(|d| d.is_active) {
            if device.id.is_some() {
                return Ok(());
            }
        }

        // No active device — transfer to the first available one
        if let Some(device) = devices.first() {
            if let Some(ref device_id) = device.id {
                client
                    .transfer_playback(device_id)
                    .await
                    .map_err(|e| PlaybackError::OperationFailed(e.to_string()))?;
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                return Ok(());
            }
        }

        Err(PlaybackError::NoActiveDevice)
    }

    /// Build a PlaybackState from the current Spotify context.
    async fn build_state(&self) -> Result<PlaybackState> {
        let guard = self.client.lock().await;
        let playback = guard.current_playback().await?;

        let (
            is_playing,
            progress_ms,
            duration_ms,
            volume,
            track_name,
            artist_name,
            track_uri,
            shuffle,
            repeat_mode,
        ) = if let Some(ctx) = &playback {
            let (name, artist, uri, dur) = match &ctx.item {
                Some(rspotify::model::PlayableItem::Track(t)) => (
                    Some(t.name.clone()),
                    t.artists.first().map(|a| a.name.clone()),
                    t.id.as_ref().map(|id| format!("spotify:track:{}", id.id())),
                    t.duration.num_milliseconds().max(0) as u32,
                ),
                Some(rspotify::model::PlayableItem::Episode(e)) => (
                    Some(e.name.clone()),
                    #[allow(deprecated)]
                    Some(e.show.publisher.clone()),
                    Some(format!("spotify:episode:{}", e.id.id())),
                    e.duration.num_milliseconds().max(0) as u32,
                ),
                _ => (None, None, None, 0),
            };
            (
                ctx.is_playing,
                ctx.progress
                    .and_then(|d| d.num_milliseconds().try_into().ok())
                    .unwrap_or(0),
                dur,
                ctx.device.volume_percent.unwrap_or(50),
                name,
                artist,
                uri,
                ctx.shuffle_state,
                RepeatMode::from_spotify(ctx.repeat_state),
            )
        } else {
            (false, 0, 0, 50, None, None, None, false, RepeatMode::Off)
        };

        let devices = guard
            .available_devices()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|d| DeviceInfo {
                id: d.id,
                name: d.name,
                is_active: d.is_active,
                is_restricted: d.is_restricted,
                device_type: format!("{:?}", d._type),
            })
            .collect();

        let context = self.current_context.lock().await.clone();

        Ok(PlaybackState {
            is_playing,
            current_track_uri: track_uri,
            current_track_name: track_name,
            current_artist_name: artist_name,
            progress_ms,
            duration_ms,
            volume,
            shuffle,
            repeat_mode,
            available_devices: devices,
            context,
        })
    }
}

#[async_trait]
impl PlaybackService for SpotifyPlaybackService {
    async fn execute(&self, cmd: PlaybackCommand) -> std::result::Result<(), PlaybackError> {
        match cmd {
            PlaybackCommand::PlayTrack {
                track_uri,
                context,
                context_tracks: _,
                position_ms,
            } => {
                // Update tracked context
                if let Some(ref ctx) = context {
                    *self.current_context.lock().await = Some(ctx.clone());
                }

                if let Some(ref ctx) = context {
                    self.play_with_context(&track_uri, ctx, position_ms).await
                } else {
                    self.play_track_simple(&track_uri, position_ms).await
                }
            }

            PlaybackCommand::PlayFromQueue => {
                let mut queue = self.queue.lock().await;
                // Peek first to get track info before advancing
                let entry = queue.peek_next_entry();
                if let Some(uri) = queue.advance() {
                    drop(queue);
                    let name = entry.as_ref().map_or("unknown", |e| &e.name);
                    tracing::info!("Playing from queue: {}", name);
                    self.play_track_simple(&uri, 0).await
                } else {
                    // Queue empty — fall back to Spotify's next
                    drop(queue);
                    self.execute(PlaybackCommand::NextTrack).await
                }
            }

            PlaybackCommand::NextTrack => {
                let guard = self.client.lock().await;
                guard
                    .playback_next()
                    .await
                    .map_err(|e| PlaybackError::OperationFailed(e.to_string()))
            }

            PlaybackCommand::PreviousTrack => {
                let guard = self.client.lock().await;
                guard
                    .playback_previous()
                    .await
                    .map_err(|e| PlaybackError::OperationFailed(e.to_string()))
            }

            PlaybackCommand::AddToQueue(entry) => {
                let mut queue = self.queue.lock().await;
                queue.add_to_up_next(entry);
                Ok(())
            }

            PlaybackCommand::ToggleShuffle => {
                let state = self
                    .get_state()
                    .await
                    .map_err(|e| PlaybackError::OperationFailed(e.to_string()))?;
                let guard = self.client.lock().await;
                guard
                    .toggle_shuffle(!state.shuffle)
                    .await
                    .map_err(|e| PlaybackError::OperationFailed(e.to_string()))
            }

            PlaybackCommand::CycleRepeat => {
                let state = self
                    .get_state()
                    .await
                    .map_err(|e| PlaybackError::OperationFailed(e.to_string()))?;
                let next = state.repeat_mode.cycle();
                let rspotify_repeat = match next {
                    RepeatMode::Off => rspotify::model::RepeatState::Off,
                    RepeatMode::Track => rspotify::model::RepeatState::Track,
                    RepeatMode::Context => rspotify::model::RepeatState::Context,
                };
                let guard = self.client.lock().await;
                guard
                    .set_repeat(rspotify_repeat)
                    .await
                    .map_err(|e| PlaybackError::OperationFailed(e.to_string()))
            }

            PlaybackCommand::SetRepeat(mode) => {
                let rspotify_repeat = match mode {
                    RepeatMode::Off => rspotify::model::RepeatState::Off,
                    RepeatMode::Track => rspotify::model::RepeatState::Track,
                    RepeatMode::Context => rspotify::model::RepeatState::Context,
                };
                let guard = self.client.lock().await;
                guard
                    .set_repeat(rspotify_repeat)
                    .await
                    .map_err(|e| PlaybackError::OperationFailed(e.to_string()))
            }

            PlaybackCommand::SetShuffle(enabled) => {
                let guard = self.client.lock().await;
                guard
                    .toggle_shuffle(enabled)
                    .await
                    .map_err(|e| PlaybackError::OperationFailed(e.to_string()))
            }

            PlaybackCommand::Seek(position_ms) => {
                let guard = self.client.lock().await;
                guard
                    .seek(position_ms, None)
                    .await
                    .map_err(|e| PlaybackError::OperationFailed(e.to_string()))
            }

            PlaybackCommand::SetVolume(vol) => {
                let guard = self.client.lock().await;
                guard
                    .set_volume(vol)
                    .await
                    .map_err(|e| PlaybackError::OperationFailed(e.to_string()))
            }

            PlaybackCommand::TogglePlayPause => {
                let state = self
                    .get_state()
                    .await
                    .map_err(|e| PlaybackError::OperationFailed(e.to_string()))?;
                let guard = self.client.lock().await;
                if state.is_playing {
                    guard
                        .playback_pause()
                        .await
                        .map_err(|e| PlaybackError::OperationFailed(e.to_string()))
                } else {
                    guard
                        .playback_resume()
                        .await
                        .map_err(|e| PlaybackError::OperationFailed(e.to_string()))
                }
            }

            PlaybackCommand::Pause => {
                let guard = self.client.lock().await;
                guard
                    .playback_pause()
                    .await
                    .map_err(|e| PlaybackError::OperationFailed(e.to_string()))
            }

            PlaybackCommand::Resume => {
                let guard = self.client.lock().await;
                guard
                    .playback_resume()
                    .await
                    .map_err(|e| PlaybackError::OperationFailed(e.to_string()))
            }

            PlaybackCommand::Stop => {
                // Spotify API doesn't have a direct "stop" — pause is the closest
                let guard = self.client.lock().await;
                guard
                    .playback_pause()
                    .await
                    .map_err(|e| PlaybackError::OperationFailed(e.to_string()))
            }

            PlaybackCommand::TransferToDevice(device_id) => {
                let guard = self.client.lock().await;
                guard
                    .transfer_playback(&device_id)
                    .await
                    .map_err(|e| PlaybackError::OperationFailed(e.to_string()))
            }
        }
    }

    async fn get_state(&self) -> Result<PlaybackState> {
        self.build_state().await
    }

    fn mode(&self) -> PlaybackMode {
        PlaybackMode::Remote
    }

    fn is_ready(&self) -> bool {
        // For remote mode, we're always "ready" — device availability is checked per-command
        true
    }
}

// ──────────────────────────────────────────────
// Local Playback Service (librespot)
// ──────────────────────────────────────────────

/// Local playback via librespot.
///
/// Does not support Spotify Connect context playback natively.
/// For context-aware next/previous, the service maintains a local queue
/// and context track list to determine what to play next.
pub struct LocalPlaybackService {
    player: Arc<LocalPlayer>,
    /// Local up-next queue.
    queue: Arc<Mutex<PlaybackQueue>>,
    /// Tracks from the current context, used for next/prev navigation.
    context_tracks: Arc<Mutex<Option<Vec<QueueEntry>>>>,
    /// The current playback context.
    current_context: Arc<Mutex<Option<PlaybackContext>>>,
}

impl LocalPlaybackService {
    pub fn new(player: Arc<LocalPlayer>) -> Self {
        Self {
            player,
            queue: Arc::new(Mutex::new(PlaybackQueue::new())),
            context_tracks: Arc::new(Mutex::new(None)),
            current_context: Arc::new(Mutex::new(None)),
        }
    }

    /// Create with an external queue reference for shared state.
    pub fn with_queue(player: Arc<LocalPlayer>, queue: Arc<Mutex<PlaybackQueue>>) -> Self {
        Self {
            player,
            queue,
            context_tracks: Arc::new(Mutex::new(None)),
            current_context: Arc::new(Mutex::new(None)),
        }
    }

    /// Load and play a track URI locally.
    async fn load_and_play(
        &self,
        track_uri: &str,
        position_ms: u32,
    ) -> std::result::Result<(), PlaybackError> {
        self.player
            .load_uri(track_uri, true, position_ms)
            .map_err(|e| PlaybackError::OperationFailed(e.to_string()))?;
        Ok(())
    }

    /// Determine the next track to play from context or queue.
    async fn next_track_from_context(&self) -> Option<String> {
        let tracks = self.context_tracks.lock().await;
        let tracks = tracks.as_ref()?;

        let current_uri = self.player.state.current_track_uri.as_ref()?;
        let ctx = self.current_context.lock().await;

        // Find current track index in context
        let current_idx = tracks
            .iter()
            .position(|t| &t.uri == current_uri)
            .or_else(|| {
                ctx.as_ref().and_then(|c| match c {
                    PlaybackContext::Playlist { start_index, .. } => Some(*start_index),
                    _ => None,
                })
            })?;

        // Return next track URI
        tracks.get(current_idx + 1).map(|t| t.uri.clone())
    }

    /// Determine the previous track to play from context.
    async fn prev_track_from_context(&self) -> Option<String> {
        let tracks = self.context_tracks.lock().await;
        let tracks = tracks.as_ref()?;

        let current_uri = self.player.state.current_track_uri.as_ref()?;
        let current_idx = tracks.iter().position(|t| &t.uri == current_uri)?;

        if current_idx > 0 {
            tracks.get(current_idx - 1).map(|t| t.uri.clone())
        } else {
            // At start of context — restart current track
            Some(current_uri.clone())
        }
    }

    /// Build PlaybackState from the local player's state.
    fn build_state(&self) -> PlaybackState {
        let ps = &self.player.state;
        let context = self.current_context.try_lock().ok().and_then(|c| c.clone());
        PlaybackState {
            is_playing: ps.is_playing,
            current_track_uri: ps.current_track_uri.clone(),
            current_track_name: ps.current_track_name.clone(),
            current_artist_name: ps.current_artist_name.clone(),
            progress_ms: ps.progress_ms,
            duration_ms: ps.duration_ms,
            volume: (ps.volume as u32) * 100 / 65535,
            shuffle: false, // librespot doesn't expose shuffle state
            repeat_mode: RepeatMode::Off,
            available_devices: vec![],
            context,
        }
    }
}

#[async_trait]
impl PlaybackService for LocalPlaybackService {
    async fn execute(&self, cmd: PlaybackCommand) -> std::result::Result<(), PlaybackError> {
        match cmd {
            PlaybackCommand::PlayTrack {
                track_uri,
                context,
                context_tracks,
                position_ms,
            } => {
                // Store context for next/prev navigation
                if let Some(ref ctx) = context {
                    *self.current_context.lock().await = Some(ctx.clone());
                }
                if let Some(tracks) = context_tracks {
                    *self.context_tracks.lock().await = Some(tracks);
                }

                self.load_and_play(&track_uri, position_ms).await
            }

            PlaybackCommand::PlayFromQueue => {
                let mut queue = self.queue.lock().await;
                if let Some(uri) = queue.advance() {
                    let entry = queue.peek_next_entry();
                    drop(queue);
                    let name = entry.as_ref().map_or("unknown", |e| &e.name);
                    tracing::info!("Playing from queue: {}", name);
                    self.load_and_play(&uri, 0).await
                } else {
                    drop(queue);
                    // Fall back to context-based next
                    if let Some(uri) = self.next_track_from_context().await {
                        self.load_and_play(&uri, 0).await
                    } else {
                        Err(PlaybackError::OperationFailed("Queue empty".into()))
                    }
                }
            }

            PlaybackCommand::NextTrack => {
                // Try context-based next first
                if let Some(uri) = self.next_track_from_context().await {
                    return self.load_and_play(&uri, 0).await;
                }
                // Fall back to queue
                self.execute(PlaybackCommand::PlayFromQueue).await
            }

            PlaybackCommand::PreviousTrack => {
                if let Some(uri) = self.prev_track_from_context().await {
                    return self.load_and_play(&uri, 0).await;
                }
                Err(PlaybackError::OperationFailed(
                    "No context for previous track".into(),
                ))
            }

            PlaybackCommand::AddToQueue(entry) => {
                let mut queue = self.queue.lock().await;
                queue.add_to_up_next(entry);
                Ok(())
            }

            PlaybackCommand::ToggleShuffle => {
                // librespot doesn't support shuffle via API — no-op with warning
                tracing::warn!("Shuffle not supported in local playback mode");
                Ok(())
            }

            PlaybackCommand::CycleRepeat => {
                // librespot doesn't support repeat via API — no-op with warning
                tracing::warn!("Repeat not supported in local playback mode");
                Ok(())
            }

            PlaybackCommand::SetRepeat(_) => {
                tracing::warn!("Repeat not supported in local playback mode");
                Ok(())
            }

            PlaybackCommand::SetShuffle(_) => {
                tracing::warn!("Shuffle not supported in local playback mode");
                Ok(())
            }

            PlaybackCommand::Seek(position_ms) => {
                self.player.seek(position_ms);
                Ok(())
            }

            PlaybackCommand::SetVolume(vol) => {
                let vol_16bit = (vol.min(100) as u64 * 65535 / 100) as u16;
                self.player.set_volume(vol_16bit);
                Ok(())
            }

            PlaybackCommand::TogglePlayPause => {
                if self.player.state.is_playing {
                    self.player.pause();
                } else {
                    self.player.play();
                }
                Ok(())
            }

            PlaybackCommand::Pause => {
                self.player.pause();
                Ok(())
            }

            PlaybackCommand::Resume => {
                self.player.play();
                Ok(())
            }

            PlaybackCommand::Stop => {
                self.player.stop();
                Ok(())
            }

            PlaybackCommand::TransferToDevice(_) => {
                // Local mode doesn't support device transfer
                Err(PlaybackError::OperationFailed(
                    "Device transfer not available in local mode".into(),
                ))
            }
        }
    }

    async fn get_state(&self) -> Result<PlaybackState> {
        Ok(self.build_state())
    }

    fn mode(&self) -> PlaybackMode {
        PlaybackMode::Local
    }

    fn is_ready(&self) -> bool {
        // Local player is ready if it was constructed successfully
        true
    }
}

// ──────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── PlaybackContext ──

    #[test]
    fn test_context_uri_playlist() {
        let ctx = PlaybackContext::Playlist {
            uri: "spotify:playlist:abc123".into(),
            name: "My Playlist".into(),
            start_index: 5,
        };
        assert_eq!(ctx.uri(), Some("spotify:playlist:abc123"));
        assert_eq!(ctx.name(), "My Playlist");
    }

    #[test]
    fn test_context_uri_album() {
        let ctx = PlaybackContext::Album {
            uri: "spotify:album:def456".into(),
            name: "Great Album".into(),
        };
        assert_eq!(ctx.uri(), Some("spotify:album:def456"));
        assert_eq!(ctx.name(), "Great Album");
    }

    #[test]
    fn test_context_uri_artist() {
        let ctx = PlaybackContext::Artist {
            uri: "spotify:artist:ghi789".into(),
            name: "Cool Artist".into(),
        };
        assert_eq!(ctx.uri(), Some("spotify:artist:ghi789"));
        assert_eq!(ctx.name(), "Cool Artist");
    }

    // ── PlaybackCommand ──

    #[test]
    fn test_playback_command_debug() {
        let cmd = PlaybackCommand::NextTrack;
        let debug = format!("{cmd:?}");
        assert!(debug.contains("NextTrack"));
    }

    #[test]
    fn test_playback_command_clone() {
        let cmd = PlaybackCommand::PlayTrack {
            track_uri: "spotify:track:abc".into(),
            context: Some(PlaybackContext::Album {
                uri: "spotify:album:def".into(),
                name: "Album".into(),
            }),
            context_tracks: None,
            position_ms: 5000,
        };
        let cloned = cmd.clone();
        match cloned {
            PlaybackCommand::PlayTrack {
                track_uri,
                context,
                position_ms,
                ..
            } => {
                assert_eq!(track_uri, "spotify:track:abc");
                assert!(context.is_some());
                assert_eq!(position_ms, 5000);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_add_to_queue_command() {
        let entry = QueueEntry {
            uri: "spotify:track:xyz".into(),
            name: "Song".into(),
            artist: "Artist".into(),
            added_by_user: true,
            is_recommendation: false,
            ..Default::default()
        };
        let cmd = PlaybackCommand::AddToQueue(entry);
        match cmd {
            PlaybackCommand::AddToQueue(e) => {
                assert_eq!(e.uri, "spotify:track:xyz");
                assert!(e.added_by_user);
            }
            _ => panic!("Wrong variant"),
        }
    }

    // ── PlaybackState ──

    #[test]
    fn test_playback_state_defaults() {
        let state = PlaybackState::default();
        assert!(!state.is_playing);
        assert!(state.current_track_uri.is_none());
        assert_eq!(state.progress_ms, 0);
        assert_eq!(state.duration_ms, 0);
        assert_eq!(state.volume, 0);
        assert!(!state.shuffle);
        assert_eq!(state.repeat_mode, RepeatMode::Off);
        assert!(state.available_devices.is_empty());
        assert!(state.context.is_none());
    }

    #[test]
    fn test_playback_state_clone() {
        let state = PlaybackState {
            is_playing: true,
            current_track_uri: Some("spotify:track:abc".into()),
            current_track_name: Some("Test".into()),
            current_artist_name: Some("Artist".into()),
            progress_ms: 30000,
            duration_ms: 180000,
            volume: 75,
            shuffle: true,
            repeat_mode: RepeatMode::Context,
            available_devices: vec![DeviceInfo {
                id: Some("dev1".into()),
                name: "Laptop".into(),
                is_active: true,
                is_restricted: false,
                device_type: "Computer".into(),
            }],
            context: Some(PlaybackContext::Album {
                uri: "spotify:album:xyz".into(),
                name: "Album".into(),
            }),
        };

        let cloned = state.clone();
        assert_eq!(cloned.is_playing, state.is_playing);
        assert_eq!(cloned.current_track_uri, state.current_track_uri);
        assert_eq!(cloned.progress_ms, state.progress_ms);
        assert_eq!(cloned.volume, state.volume);
        assert_eq!(cloned.repeat_mode, state.repeat_mode);
        assert_eq!(cloned.available_devices.len(), 1);
    }

    // ── PlaybackError ──

    #[test]
    fn test_error_status_messages() {
        assert_eq!(
            PlaybackError::NoActiveDevice.status_message(),
            "No active Spotify device. Open Spotify first."
        );
        assert_eq!(
            PlaybackError::TrackNotFound("spotify:track:abc".into()).status_message(),
            "Track not found: spotify:track:abc"
        );
        assert_eq!(
            PlaybackError::LocalPlayerNotReady.status_message(),
            "Local player not ready"
        );
        assert_eq!(
            PlaybackError::RateLimited.status_message(),
            "Rate limited — please wait"
        );
    }

    #[test]
    fn test_error_retryable() {
        assert!(PlaybackError::NoActiveDevice.is_retryable());
        assert!(PlaybackError::RateLimited.is_retryable());
        assert!(PlaybackError::OperationFailed("timeout".into()).is_retryable());
        assert!(!PlaybackError::TrackNotFound("x".into()).is_retryable());
        assert!(!PlaybackError::LocalPlayerNotReady.is_retryable());
        assert!(!PlaybackError::AuthExpired.is_retryable());
    }

    #[test]
    fn test_error_display() {
        let err = PlaybackError::TrackNotFound("spotify:track:abc".into());
        let display = format!("{err}");
        assert!(display.contains("spotify:track:abc"));
    }

    // ── PlaybackMode ──

    #[test]
    fn test_playback_mode_default() {
        let mode = PlaybackMode::default();
        assert_eq!(mode, PlaybackMode::Local);
    }

    #[test]
    fn test_playback_mode_equality() {
        assert_eq!(PlaybackMode::Local, PlaybackMode::Local);
        assert_eq!(PlaybackMode::Remote, PlaybackMode::Remote);
        assert_ne!(PlaybackMode::Local, PlaybackMode::Remote);
    }

    // ── Queue integration ──

    #[test]
    fn test_queue_entry_in_command() {
        let entry = QueueEntry {
            uri: "spotify:track:1".into(),
            name: "First".into(),
            artist: "A".into(),
            added_by_user: true,
            is_recommendation: false,
            ..Default::default()
        };
        let cmd = PlaybackCommand::AddToQueue(entry);
        match cmd {
            PlaybackCommand::AddToQueue(e) => {
                assert_eq!(e.name, "First");
                assert_eq!(e.artist, "A");
                assert!(e.added_by_user);
                assert!(!e.is_recommendation);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_cycle_repeat_command_variants() {
        let cmd1 = PlaybackCommand::CycleRepeat;
        let cmd2 = PlaybackCommand::SetRepeat(RepeatMode::Track);
        let cmd3 = PlaybackCommand::SetShuffle(true);

        // Just verify they're constructible and debuggable
        let _ = format!("{cmd1:?}");
        let _ = format!("{cmd2:?}");
        let _ = format!("{cmd3:?}");
    }

    // ── DeviceInfo ──

    #[test]
    fn test_device_info_construction() {
        let device = DeviceInfo {
            id: Some("dev-123".into()),
            name: "Kitchen Speaker".into(),
            is_active: true,
            is_restricted: false,
            device_type: "Speaker".into(),
        };
        assert_eq!(device.id, Some("dev-123".into()));
        assert!(device.is_active);
        assert!(!device.is_restricted);
    }

    #[test]
    fn test_device_info_no_id() {
        let device = DeviceInfo {
            id: None,
            name: "Restricted Device".into(),
            is_active: false,
            is_restricted: true,
            device_type: "Unknown".into(),
        };
        assert!(device.id.is_none());
        assert!(device.is_restricted);
    }

    // ── RepeatMode integration ──

    #[test]
    fn test_repeat_mode_in_playback_state() {
        let state = PlaybackState {
            repeat_mode: RepeatMode::Track,
            ..Default::default()
        };
        assert_eq!(state.repeat_mode, RepeatMode::Track);
    }

    #[test]
    fn test_repeat_mode_cycle_in_command() {
        // Verify that CycleRepeat is a valid command
        let cmd = PlaybackCommand::CycleRepeat;
        match cmd {
            PlaybackCommand::CycleRepeat => {}
            _ => panic!("Expected CycleRepeat"),
        }
    }

    // ── Volume conversion ──

    #[test]
    fn test_volume_conversion_bounds() {
        // 0% → 0
        assert_eq!((0u32.min(100) as u64 * 65535 / 100) as u16, 0);
        // 50% → ~32767
        let mid = (50u32.min(100) as u64 * 65535 / 100) as u16;
        assert!(mid >= 32700 && mid <= 32800);
        // 100% → 65535
        assert_eq!((100u32.min(100) as u64 * 65535 / 100) as u16, 65535);
        // >100 clamped
        assert_eq!((150u32.min(100) as u64 * 65535 / 100) as u16, 65535);
    }

    // ── Context track navigation helpers ──

    #[test]
    fn test_context_tracks_next_navigation() {
        let tracks = vec![
            QueueEntry {
                uri: "spotify:track:1".into(),
                name: "A".into(),
                artist: "X".into(),
                added_by_user: false,
                is_recommendation: false,
                ..Default::default()
            },
            QueueEntry {
                uri: "spotify:track:2".into(),
                name: "B".into(),
                artist: "X".into(),
                added_by_user: false,
                is_recommendation: false,
                ..Default::default()
            },
            QueueEntry {
                uri: "spotify:track:3".into(),
                name: "C".into(),
                artist: "X".into(),
                added_by_user: false,
                is_recommendation: false,
                ..Default::default()
            },
        ];

        // Simulate finding next track from index 0
        let current_idx = 0;
        let next = tracks.get(current_idx + 1);
        assert!(next.is_some());
        assert_eq!(next.unwrap().uri, "spotify:track:2");

        // Last track — no next
        let next = tracks.get(2 + 1);
        assert!(next.is_none());
    }

    #[test]
    fn test_context_tracks_prev_navigation() {
        let tracks = vec![
            QueueEntry {
                uri: "spotify:track:1".into(),
                name: "A".into(),
                artist: "X".into(),
                added_by_user: false,
                is_recommendation: false,
                ..Default::default()
            },
            QueueEntry {
                uri: "spotify:track:2".into(),
                name: "B".into(),
                artist: "X".into(),
                added_by_user: false,
                is_recommendation: false,
                ..Default::default()
            },
            QueueEntry {
                uri: "spotify:track:3".into(),
                name: "C".into(),
                artist: "X".into(),
                added_by_user: false,
                is_recommendation: false,
                ..Default::default()
            },
        ];

        // From index 2, prev is index 1
        let current_idx = 2;
        if current_idx > 0 {
            let prev = tracks.get(current_idx - 1);
            assert!(prev.is_some());
            assert_eq!(prev.unwrap().uri, "spotify:track:2");
        }

        // From index 0, restart current
        let current_idx = 0;
        if current_idx > 0 {
            panic!("Should not enter");
        } else {
            // Restart current track
            let current = tracks.get(current_idx);
            assert!(current.is_some());
            assert_eq!(current.unwrap().uri, "spotify:track:1");
        }
    }

    // ── PlaybackContext URI extraction ──

    #[test]
    fn test_context_uri_extraction_playlist() {
        let ctx = PlaybackContext::Playlist {
            uri: "spotify:playlist:37i9dQZF1DXcBWIGoYBM5M".into(),
            name: "Today's Top Hits".into(),
            start_index: 10,
        };
        assert_eq!(ctx.uri(), Some("spotify:playlist:37i9dQZF1DXcBWIGoYBM5M"));
        assert_eq!(ctx.name(), "Today's Top Hits");
    }

    #[test]
    fn test_context_uri_extraction_album() {
        let ctx = PlaybackContext::Album {
            uri: "spotify:album:6DEjYFkNZh67HP7R9PSZvv".into(),
            name: "Abbey Road".into(),
        };
        assert_eq!(ctx.uri(), Some("spotify:album:6DEjYFkNZh67HP7R9PSZvv"));
        assert_eq!(ctx.name(), "Abbey Road");
    }

    // ── Command completeness ──

    #[test]
    fn test_all_command_variants_constructible() {
        // Verify every variant can be constructed — catches missing variants on enum changes
        let _ = PlaybackCommand::PlayTrack {
            track_uri: "uri".into(),
            context: None,
            context_tracks: None,
            position_ms: 0,
        };
        let _ = PlaybackCommand::PlayFromQueue;
        let _ = PlaybackCommand::NextTrack;
        let _ = PlaybackCommand::PreviousTrack;
        let _ = PlaybackCommand::AddToQueue(QueueEntry::default());
        let _ = PlaybackCommand::ToggleShuffle;
        let _ = PlaybackCommand::CycleRepeat;
        let _ = PlaybackCommand::SetRepeat(RepeatMode::Off);
        let _ = PlaybackCommand::SetShuffle(true);
        let _ = PlaybackCommand::Seek(5000);
        let _ = PlaybackCommand::SetVolume(50);
        let _ = PlaybackCommand::TogglePlayPause;
        let _ = PlaybackCommand::Pause;
        let _ = PlaybackCommand::Resume;
        let _ = PlaybackCommand::Stop;
        let _ = PlaybackCommand::TransferToDevice("dev-id".into());
    }

    // ── Error categorization ──

    #[test]
    fn test_error_categorization_comprehensive() {
        let errors = [
            (PlaybackError::NoActiveDevice, true),
            (PlaybackError::TrackNotFound("x".into()), false),
            (PlaybackError::ContextUnavailable("x".into()), false),
            (PlaybackError::LocalPlayerNotReady, false),
            (PlaybackError::RemoteClientNotReady, false),
            (PlaybackError::OperationFailed("x".into()), true),
            (PlaybackError::RateLimited, true),
            (PlaybackError::AuthExpired, false),
        ];

        for (err, expected_retryable) in errors {
            assert_eq!(
                err.is_retryable(),
                expected_retryable,
                "Wrong retryable for {err:?}"
            );
        }
    }
}
