//! Spotify API client wrapper

use anyhow::{Result, Context};
use rspotify::{
    AuthCodeSpotify,
    clients::{BaseClient, OAuthClient},
    model::{CurrentPlaybackContext, PlaylistId, PlayableItem, TrackId, AdditionalType},
    Credentials, OAuth,
};
use std::collections::HashSet;

use crate::auth::{OAuthConfig, load_credentials};

/// Spotify API client
pub struct SpotifyClient {
    pub(crate) oauth: AuthCodeSpotify,
}

impl SpotifyClient {
    /// Create a new Spotify client
    pub async fn new(config: &OAuthConfig) -> Result<Self> {
        let creds = Credentials::new(&config.client_id, &config.client_secret);

        let mut oauth_config = OAuth::default();
        oauth_config.redirect_uri = config.redirect_uri.clone();
        oauth_config.scopes = HashSet::from([
            "user-read-playback-state".to_string(),
            "user-modify-playback-state".to_string(),
            "user-read-currently-playing".to_string(),
            "streaming".to_string(),
            "playlist-read-private".to_string(),
            "playlist-modify-private".to_string(),
            "user-library-read".to_string(),
            "user-read-recently-played".to_string(),
        ]);

        let oauth = AuthCodeSpotify::new(creds, oauth_config);

        let client = Self { oauth };

        // Try to load cached credentials and apply them to the OAuth client
        if let Some(creds) = load_credentials()? {
            // Check expiration after acquiring lock to avoid race condition
            if let Ok(mut token_guard) = client.oauth.token.lock().await {
                if !creds.is_expired() {
                    let token = rspotify::Token {
                        access_token: creds.access_token,
                        refresh_token: creds.refresh_token,
                        expires_at: Some(chrono::DateTime::from_timestamp(creds.expires_at as i64, 0)
                            .unwrap_or(chrono::DateTime::UNIX_EPOCH)),
                        expires_in: chrono::TimeDelta::seconds(3600),
                        scopes: HashSet::new(),
                    };
                    *token_guard = Some(token);
                    println!("Loaded cached credentials");
                } else {
                    println!("Cached token expired - re-authentication needed");
                }
            }
        }

        Ok(client)
    }

    /// Get auth URL
    pub fn get_authorize_url(&self) -> String {
        self.oauth.auth_url("")
    }

    /// Get current playback state
    pub async fn current_playback(&self) -> Result<Option<CurrentPlaybackContext>> {
        match self.oauth.current_playback(None, None::<Vec<&AdditionalType>>).await {
            Ok(ctx) => Ok(ctx),
            Err(e) => {
                // Check if it's a "no active device" error - this is normal, not an error
                let err_str = e.to_string();
                if err_str.contains("NO_ACTIVE_DEVICE") || err_str.contains("no active device") {
                    Ok(None)
                } else {
                    Err(e).context("Failed to get current playback state")
                }
            }
        }
    }


    /// Get user's playlists - simplified, returns first page
    pub async fn current_user_playlists(&self, limit: u32) -> Result<Vec<rspotify::model::SimplifiedPlaylist>> {
        let result = self.oauth.current_user_playlists_manual(Some(limit), None).await
            .context("Failed to get playlists")?;
        Ok(result.items)
    }

    /// Get playlist tracks
    pub async fn playlist_tracks(&self, playlist_id: &str) -> Result<Vec<PlayableItem>> {
        let id = PlaylistId::from_id(playlist_id)
            .context("Invalid playlist ID")?;

        let tracks = self.oauth.playlist_items_manual(id, None, None, None, None).await
            .context("Failed to get playlist tracks")?;

        Ok(tracks.items.into_iter().filter_map(|pt| pt.track).collect())
    }

    // Playback controls

    /// Start or resume playback
    pub async fn playback_resume(&self) -> Result<()> {
        self.oauth.resume_playback(None, None).await
            .context("Failed to resume playback")?;
        Ok(())
    }

    /// Pause playback
    pub async fn playback_pause(&self) -> Result<()> {
        self.oauth.pause_playback(None).await
            .context("Failed to pause playback")?;
        Ok(())
    }

    /// Skip to next track
    pub async fn playback_next(&self) -> Result<()> {
        self.oauth.next_track(None).await
            .context("Failed to skip to next track")?;
        Ok(())
    }

    /// Skip to previous track
    pub async fn playback_previous(&self) -> Result<()> {
        self.oauth.previous_track(None).await
            .context("Failed to skip to previous track")?;
        Ok(())
    }

    /// Set volume (0-100)
    pub async fn set_volume(&self, volume_percent: u32) -> Result<()> {
        let vol = volume_percent.min(100) as u8;
        self.oauth.volume(vol, None).await
            .context("Failed to set volume")?;
        Ok(())
    }

    /// Toggle shuffle
    pub async fn set_shuffle(&self, shuffle: bool) -> Result<()> {
        self.oauth.shuffle(shuffle, None).await
            .context("Failed to toggle shuffle")?;
        Ok(())
    }

    /// Set repeat mode
    pub async fn set_repeat(&self, state: rspotify::model::RepeatState) -> Result<()> {
        self.oauth.repeat(state, None).await
            .context("Failed to set repeat mode")?;
        Ok(())
    }

    /// Like tracks
    pub async fn current_user_save_tracks(&self, track_ids: Vec<TrackId<'_>>) -> Result<()> {
        self.oauth.current_user_saved_tracks_add(track_ids).await
            .context("Failed to save tracks")?;
        Ok(())
    }

    /// Get user's liked tracks
    pub async fn current_user_saved_tracks(&self, limit: u32) -> Result<Vec<rspotify::model::SavedTrack>> {
        let result = self.oauth.current_user_saved_tracks_manual(None, None, None).await
            .context("Failed to get liked tracks")?;
        Ok(result.items.into_iter().take(limit as usize).collect())
    }

    /// Get user's playlists
    pub async fn current_users_playlists(&self, limit: u32) -> Result<Vec<rspotify::model::SimplifiedPlaylist>> {
        let result = self.oauth.current_user_playlists_manual(Some(limit), None).await
            .context("Failed to get playlists")?;
        Ok(result.items)
    }

    /// Get playlist tracks
    pub async fn playlist_get_items(&self, playlist_id: &str) -> Result<Vec<rspotify::model::PlaylistItem>> {
        let pid = rspotify::model::PlaylistId::from_id(playlist_id)
            .context("Invalid playlist ID")?;
        let result = self.oauth.playlist_items_manual(pid, None, None, None, None).await
            .context("Failed to get playlist items")?;
        Ok(result.items)
    }

    /// Search Spotify
    pub async fn search(&self, query: &str, track_limit: u32) -> Result<Vec<rspotify::model::FullTrack>> {
        let result = self.oauth.search(
            query,
            rspotify::model::SearchType::Track,
            None,
            None,
            None,
            None
        ).await
        .context("Search failed")?;

        match result {
            rspotify::model::SearchResult::Tracks(page) => Ok(page.items.into_iter().take(track_limit as usize).collect()),
            _ => Ok(vec![]),
        }
    }

    /// Play a specific track by URI
    pub async fn start_playback(&self, uris: Vec<String>, offset: Option<u32>) -> Result<()> {
        // Convert String URIs to PlayableId types
        let playable_uris: Vec<rspotify::model::PlayableId> = uris
            .iter()
            .filter_map(|uri| {
                if uri.starts_with("spotify:track:") {
                    let track_id = uri.strip_prefix("spotify:track:").unwrap();
                    rspotify::model::TrackId::from_id(track_id)
                        .ok()
                        .map(rspotify::model::PlayableId::Track)
                } else if uri.starts_with("spotify:episode:") {
                    let ep_id = uri.strip_prefix("spotify:episode:").unwrap();
                    rspotify::model::EpisodeId::from_id(ep_id)
                        .ok()
                        .map(rspotify::model::PlayableId::Episode)
                } else {
                    None
                }
            })
            .collect();

        self.oauth.start_uris_playback(
            playable_uris,
            None,
            None,
            offset.map(|o| chrono::TimeDelta::milliseconds(o as i64))
        ).await
        .context("Failed to start playback")?;
        Ok(())
    }

    /// Add track to queue
    pub async fn add_to_queue(&self, track_uri: &str) -> Result<()> {
        // Parse URI to get track ID
        let uri_parts: Vec<&str> = track_uri.split(':').collect();
        if uri_parts.len() >= 3 && uri_parts[0] == "spotify" {
            let track_id = uri_parts[2];
            if let Ok(id) = rspotify::model::TrackId::from_id(track_id) {
                self.oauth.add_item_to_queue(rspotify::model::PlayableId::Track(id), None).await
                    .context("Failed to add to queue")?;
                return Ok(());
            }
        }
        anyhow::bail!("Invalid Spotify track URI");
    }

    /// Get current queue
    pub async fn get_queue(&self) -> Result<rspotify::model::CurrentUserQueue> {
        let queue = self.oauth.current_user_queue().await
            .context("Failed to get queue")?;
        Ok(queue)
    }

    /// Seek to position
    pub async fn seek(&self, position_ms: u32, device_id: Option<&str>) -> Result<()> {
        let position = chrono::TimeDelta::milliseconds(position_ms as i64);
        self.oauth.seek_track(position, device_id).await
            .context("Failed to seek")?;
        Ok(())
    }

    /// Toggle shuffle
    pub async fn toggle_shuffle(&self) -> Result<bool> {
        // We need to get current state first, then toggle
        if let Ok(Some(ctx)) = self.current_playback().await {
            let new_state = !ctx.shuffle_state;
            self.set_shuffle(new_state).await?;
            Ok(new_state)
        } else {
            Ok(false)
        }
    }

    /// Cycle repeat mode
    pub async fn cycle_repeat(&self) -> Result<rspotify::model::RepeatState> {
        if let Ok(Some(ctx)) = self.current_playback().await {
            let new_state = match ctx.repeat_state {
                rspotify::model::RepeatState::Off => rspotify::model::RepeatState::Track,
                rspotify::model::RepeatState::Track => rspotify::model::RepeatState::Context,
                rspotify::model::RepeatState::Context => rspotify::model::RepeatState::Off,
            };
            self.set_repeat(new_state).await?;
            Ok(new_state)
        } else {
            Ok(rspotify::model::RepeatState::Off)
        }
    }
}
