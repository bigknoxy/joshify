//! Playback control methods

use anyhow::{Result, Context};
use rspotify::{
    clients::OAuthClient,
    model::{CurrentPlaybackContext, AdditionalType},
};

use super::SpotifyClient;

impl SpotifyClient {
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

    /// Seek to position
    pub async fn seek(&self, position_ms: u32, device_id: Option<&str>) -> Result<()> {
        let position = chrono::TimeDelta::milliseconds(position_ms as i64);
        self.oauth.seek_track(position, device_id).await
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

        self.oauth.start_uris_playback(
            playable_uris,
            None,
            None,
            offset.map(|o| chrono::TimeDelta::milliseconds(o as i64))
        ).await
        .context("Failed to start playback")?;
        Ok(())
    }
}
