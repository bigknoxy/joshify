//! Playback control methods

use anyhow::{Context, Result};
use rspotify::{
    clients::{BaseClient, OAuthClient},
    model::CurrentPlaybackContext,
};

use super::SpotifyClient;

impl SpotifyClient {
    /// Get current playback state
    pub async fn current_playback(&self) -> Result<Option<CurrentPlaybackContext>> {
        // Make raw API call to get JSON response
        use std::collections::HashMap;
        let params: HashMap<&str, &str> = HashMap::new();
        let result: Result<String, rspotify::ClientError> =
            self.oauth.api_get("me/player", &params).await;

        match result {
            Ok(json_str) => {
                // Check for empty response (no active playback)
                if json_str.is_empty() || json_str == "null" {
                    return Ok(None);
                }

                // Try to parse as CurrentPlaybackContext
                match serde_json::from_str::<CurrentPlaybackContext>(&json_str) {
                    Ok(ctx) => Ok(Some(ctx)),
                    Err(e) => {
                        // Deserialization failed - analyze what Spotify returned
                        let err_str = e.to_string();

                        // Check for device object with is_active: false
                        // This means "devices exist but nothing playing"
                        if json_str.contains("is_active") && json_str.contains("false") {
                            return Ok(None);
                        }

                        // Check for PlayableItem variant mismatch (ads, unknown types)
                        if err_str.contains("PlayableItem")
                            || err_str.contains("untagged")
                            || err_str.contains("variant")
                        {
                            return Ok(None);
                        }

                        // Check if it's an empty or null response
                        if json_str.is_empty() || json_str == "null" || json_str.contains("{}") {
                            return Ok(None);
                        }

                        // Check for "data does not match any variant" - generic serde error
                        if err_str.contains("data does not match")
                            || err_str.contains("does not match any variant")
                        {
                            return Ok(None);
                        }

                        // Fallback: ANY deserialization error = no playback
                        Ok(None)
                    }
                }
            }
            Err(e) => {
                // API call failed
                let err_str = e.to_string();
                let err_debug = format!("{:?}", e);

                let err_lower = err_str.to_lowercase();
                let err_debug_lower = err_debug.to_lowercase();

                // Match device-related errors
                let is_device_error = err_lower.contains("no active device")
                    || err_str.contains("NO_ACTIVE_DEVICE")
                    || err_lower.contains("no device")
                    || err_lower.contains("no player")
                    || err_lower.contains("player")
                    || err_lower.contains("device")
                    || err_lower.contains("inactive")
                    || err_str.contains("404")
                    || err_str.contains("400")
                    || err_debug_lower.contains("player")
                    || err_debug_lower.contains("device");

                if is_device_error {
                    Ok(None)
                } else {
                    Err(e).context("Failed to get current playback state")
                }
            }
        }
    }

    /// Get available devices
    pub async fn available_devices(&self) -> Result<Vec<rspotify::model::Device>> {
        tracing::debug!("Fetching available devices...");
        let devices = self.oauth.device().await?;
        tracing::debug!("Found {} devices", devices.len());
        for (i, device) in devices.iter().enumerate() {
            tracing::debug!(
                "  [{}] {} (type: {:?}, id: {}) - active: {}, restricted: {}",
                i,
                device.name,
                device._type,
                device.id.as_ref().unwrap_or(&"none".to_string()),
                device.is_active,
                device.is_restricted
            );
        }
        Ok(devices)
    }

    /// Transfer playback to a device
    pub async fn transfer_playback(&self, device_id: &str) -> Result<()> {
        self.oauth
            .transfer_playback(device_id, Some(true))
            .await
            .context("Failed to transfer playback")?;
        Ok(())
    }

    /// Start or resume playback
    pub async fn playback_resume(&self) -> Result<()> {
        self.oauth
            .resume_playback(None, None)
            .await
            .context("Failed to resume playback")?;
        Ok(())
    }

    /// Pause playback
    pub async fn playback_pause(&self) -> Result<()> {
        self.oauth
            .pause_playback(None)
            .await
            .context("Failed to pause playback")?;
        Ok(())
    }

    /// Skip to next track
    pub async fn playback_next(&self) -> Result<()> {
        self.oauth
            .next_track(None)
            .await
            .context("Failed to skip to next track")?;
        Ok(())
    }

    /// Skip to previous track
    pub async fn playback_previous(&self) -> Result<()> {
        self.oauth
            .previous_track(None)
            .await
            .context("Failed to skip to previous track")?;
        Ok(())
    }

    /// Set volume (0-100)
    pub async fn set_volume(&self, volume_percent: u32) -> Result<()> {
        let vol = volume_percent.min(100) as u8;
        self.oauth
            .volume(vol, None)
            .await
            .context("Failed to set volume")?;
        Ok(())
    }

    /// Seek to position
    pub async fn seek(&self, position_ms: u32, device_id: Option<&str>) -> Result<()> {
        let position = chrono::TimeDelta::milliseconds(position_ms as i64);
        self.oauth
            .seek_track(position, device_id)
            .await
            .context("Failed to seek")?;
        Ok(())
    }

    /// Play a specific track by URI
    pub async fn start_playback(&self, uris: Vec<String>, offset: Option<u32>) -> Result<()> {
        // Convert String URIs to PlayableId types
        let playable_uris: Vec<rspotify::model::PlayableId> = uris
            .iter()
            .filter_map(|uri| {
                if uri.starts_with("spotify:track:") {
                    let track_id = uri.strip_prefix("spotify:track:")?;
                    rspotify::model::TrackId::from_id(track_id)
                        .ok()
                        .map(rspotify::model::PlayableId::Track)
                } else if uri.starts_with("spotify:episode:") {
                    let ep_id = uri.strip_prefix("spotify:episode:")?;
                    rspotify::model::EpisodeId::from_id(ep_id)
                        .ok()
                        .map(rspotify::model::PlayableId::Episode)
                } else {
                    None
                }
            })
            .collect();

        if playable_uris.is_empty() {
            anyhow::bail!("No valid track/episode URIs provided");
        }

        self.oauth
            .start_uris_playback(
                playable_uris,
                None,
                None,
                offset.map(|o| chrono::TimeDelta::milliseconds(o as i64)),
            )
            .await
            .context("Failed to start playback")?;
        Ok(())
    }
}
