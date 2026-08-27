//! Playback control methods

use anyhow::{Context, Result};
use rspotify::{
    clients::{BaseClient, OAuthClient},
    model::CurrentPlaybackContext,
};

use super::SpotifyClient;

/// Choose which device a playback command should target, given the devices
/// Spotify currently reports and the id the user picked in the device selector.
///
/// Spotify rejects commands with `NO_ACTIVE_DEVICE` unless a device id is
/// supplied or something is already active, so every command path must resolve
/// a concrete device up front. Preference order:
///
/// 1. The user's explicit choice, while that device is still online.
/// 2. Whatever Spotify already has active.
/// 3. Any remaining controllable device.
///
/// Devices Spotify marks `is_restricted` (they reject the Web API's control
/// endpoints) and devices with no id (nothing to address) are never returned.
pub fn select_play_device<'a>(
    devices: &'a [rspotify::model::Device],
    preferred: Option<&str>,
) -> Option<&'a rspotify::model::Device> {
    let controllable = |d: &&rspotify::model::Device| !d.is_restricted && d.id.is_some();

    if let Some(preferred) = preferred {
        if let Some(device) = devices
            .iter()
            .find(|d| d.id.as_deref() == Some(preferred) && controllable(d))
        {
            return Some(device);
        }
        tracing::warn!(
            "Selected device {} is offline or restricted; falling back",
            preferred
        );
    }

    devices
        .iter()
        .find(|d| d.is_active && controllable(d))
        .or_else(|| devices.iter().find(controllable))
}

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

    /// Resolve which device a command should target, consulting Spotify for
    /// what is currently online.
    pub async fn device_to_play_on(
        &self,
        preferred: Option<&str>,
    ) -> Result<Option<rspotify::model::Device>> {
        let devices = self.available_devices().await?;
        Ok(select_play_device(&devices, preferred).cloned())
    }

    /// Transfer playback to a device
    /// Note: play=false to avoid race with subsequent play commands
    pub async fn transfer_playback(&self, device_id: &str) -> Result<()> {
        self.oauth
            .transfer_playback(device_id, Some(false))
            .await
            .context("Failed to transfer playback")?;
        Ok(())
    }

    /// Start or resume playback
    pub async fn playback_resume(&self, device_id: Option<&str>) -> Result<()> {
        self.oauth
            .resume_playback(device_id, None)
            .await
            .context("Failed to resume playback")?;
        Ok(())
    }

    /// Pause playback
    pub async fn playback_pause(&self, device_id: Option<&str>) -> Result<()> {
        self.oauth
            .pause_playback(device_id)
            .await
            .context("Failed to pause playback")?;
        Ok(())
    }

    /// Skip to next track
    pub async fn playback_next(&self, device_id: Option<&str>) -> Result<()> {
        self.oauth
            .next_track(device_id)
            .await
            .context("Failed to skip to next track")?;
        Ok(())
    }

    /// Skip to previous track
    pub async fn playback_previous(&self, device_id: Option<&str>) -> Result<()> {
        self.oauth
            .previous_track(device_id)
            .await
            .context("Failed to skip to previous track")?;
        Ok(())
    }

    /// Set volume (0-100). Discovers an active device if needed.
    pub async fn set_volume(&self, volume_percent: u32, device_id: Option<&str>) -> Result<()> {
        let vol = volume_percent.min(100) as u8;
        tracing::info!("Setting volume to {}%", vol);
        match self.oauth.volume(vol, device_id).await {
            Ok(()) => {
                tracing::info!("Volume set to {}% successfully", vol);
                Ok(())
            }
            Err(e) => {
                tracing::warn!(
                    "Direct volume set failed ({}), trying with device transfer",
                    e
                );
                let devices =
                    self.oauth.device().await.map_err(|de| {
                        anyhow::anyhow!("Failed to get devices for volume: {}", de)
                    })?;
                if let Some(device) = select_play_device(&devices, device_id) {
                    if let Some(ref device_id) = device.id {
                        tracing::info!("Transferring playback to {} for volume", device.name);
                        self.oauth
                            .transfer_playback(device_id, Some(true))
                            .await
                            .map_err(|te| {
                                anyhow::anyhow!("Failed to transfer for volume: {}", te)
                            })?;
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        self.oauth.volume(vol, Some(device_id)).await.map_err(|ve| {
                            anyhow::anyhow!("Failed to set volume after transfer: {}", ve)
                        })
                    } else {
                        Err(anyhow::anyhow!("No device ID available for volume"))
                    }
                } else {
                    Err(anyhow::anyhow!(
                        "No active device found. Open Spotify on a device first."
                    ))
                }
            }
        }
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
        self.start_playback_on(uris, offset, None).await
    }

    /// Play a specific track by URI, targeting an explicit device
    pub async fn start_playback_on(
        &self,
        uris: Vec<String>,
        offset: Option<u32>,
        device_id: Option<&str>,
    ) -> Result<()> {
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
                device_id,
                None,
                offset.map(|o| chrono::TimeDelta::milliseconds(o as i64)),
            )
            .await
            .context("Failed to start playback")?;
        Ok(())
    }

    /// Start playback of a context (playlist/album), targeting an explicit device
    pub async fn start_context_playback_on(
        &self,
        context: rspotify::model::PlayContextId<'_>,
        offset: Option<rspotify::model::Offset>,
        device_id: Option<&str>,
    ) -> Result<()> {
        self.oauth
            .start_context_playback(context, device_id, offset, None)
            .await
            .context("Failed to start context playback")?;
        Ok(())
    }

    /// Toggle shuffle state
    pub async fn toggle_shuffle(&self, shuffle: bool, device_id: Option<&str>) -> Result<()> {
        self.oauth
            .shuffle(shuffle, device_id)
            .await
            .context("Failed to toggle shuffle")?;
        Ok(())
    }

    /// Set repeat mode
    pub async fn set_repeat(
        &self,
        state: rspotify::model::RepeatState,
        device_id: Option<&str>,
    ) -> Result<()> {
        self.oauth
            .repeat(state, device_id)
            .await
            .context("Failed to set repeat mode")?;
        Ok(())
    }
}

#[cfg(test)]
mod select_play_device_tests {
    use super::select_play_device;
    use rspotify::model::{Device, DeviceType};

    fn device(id: Option<&str>, name: &str, is_active: bool, is_restricted: bool) -> Device {
        Device {
            id: id.map(str::to_string),
            is_active,
            is_private_session: false,
            is_restricted,
            name: name.to_string(),
            _type: DeviceType::Computer,
            volume_percent: Some(50),
        }
    }

    fn id_of(d: Option<&Device>) -> Option<&str> {
        d.and_then(|d| d.id.as_deref())
    }

    #[test]
    fn no_devices_yields_nothing() {
        assert!(select_play_device(&[], None).is_none());
        assert!(select_play_device(&[], Some("phone")).is_none());
    }

    #[test]
    fn prefers_the_active_device_over_list_order() {
        // The original bug: `devices.first()` was used, which picks whatever
        // Spotify happens to list first rather than the one actually playing.
        let devices = vec![
            device(Some("laptop"), "Laptop", false, false),
            device(Some("phone"), "Phone", true, false),
        ];
        assert_eq!(id_of(select_play_device(&devices, None)), Some("phone"));
    }

    #[test]
    fn falls_back_to_any_controllable_device_when_none_is_active() {
        let devices = vec![
            device(Some("laptop"), "Laptop", false, false),
            device(Some("phone"), "Phone", false, false),
        ];
        assert_eq!(id_of(select_play_device(&devices, None)), Some("laptop"));
    }

    #[test]
    fn the_users_choice_wins_over_the_active_device() {
        let devices = vec![
            device(Some("phone"), "Phone", true, false),
            device(Some("speaker"), "Speaker", false, false),
        ];
        assert_eq!(
            id_of(select_play_device(&devices, Some("speaker"))),
            Some("speaker")
        );
    }

    #[test]
    fn an_offline_choice_falls_back_instead_of_failing() {
        // The chosen speaker went to sleep. Playing on the active device beats
        // reporting "no device" at the user.
        let devices = vec![device(Some("phone"), "Phone", true, false)];
        assert_eq!(
            id_of(select_play_device(&devices, Some("speaker"))),
            Some("phone")
        );
    }

    #[test]
    fn restricted_devices_are_never_chosen() {
        // Restricted devices reject the Web API control endpoints, so playing
        // to one is the silent no-op this whole fix exists to prevent.
        let devices = vec![
            device(Some("cast"), "Chromecast", true, true),
            device(Some("laptop"), "Laptop", false, false),
        ];
        assert_eq!(id_of(select_play_device(&devices, None)), Some("laptop"));
        assert_eq!(
            id_of(select_play_device(&devices, Some("cast"))),
            Some("laptop"),
            "explicitly choosing a restricted device must still not target it"
        );
    }

    #[test]
    fn devices_without_an_id_are_never_chosen() {
        let devices = vec![
            device(None, "Anonymous", true, false),
            device(Some("laptop"), "Laptop", false, false),
        ];
        assert_eq!(id_of(select_play_device(&devices, None)), Some("laptop"));
    }

    #[test]
    fn all_devices_unusable_yields_nothing() {
        let devices = vec![
            device(Some("cast"), "Chromecast", true, true),
            device(None, "Anonymous", false, false),
        ];
        assert!(select_play_device(&devices, None).is_none());
        assert!(select_play_device(&devices, Some("cast")).is_none());
    }
}
