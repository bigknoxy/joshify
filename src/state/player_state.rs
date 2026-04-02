//! Player state and playback tracking
//!
//! Moved from player.rs to the state module for better organization.

use rspotify::model::CurrentPlaybackContext;
use rspotify::prelude::Id;

/// Playback state
#[derive(Debug, Clone, Default)]
pub struct PlayerState {
    pub is_playing: bool,
    pub progress_ms: u32,
    pub duration_ms: u32,
    pub volume: u32,
    pub current_track_name: Option<String>,
    pub current_artist_name: Option<String>,
    pub current_album_art_url: Option<String>,
    pub current_album_art_data: Option<Vec<u8>>, // Downloaded image bytes
    pub current_track_uri: Option<String>,
}

impl PlayerState {
    /// Create player state from Spotify playback context
    pub fn from_context(ctx: &CurrentPlaybackContext) -> Self {
        let (track_name, artist_name, album_art_url, duration_ms, track_uri) = match &ctx.item {
            Some(rspotify::model::PlayableItem::Track(track)) => {
                let track_name = track.name.clone();
                let artist_name = track.artists.first().map(|a| a.name.clone());
                let album_art_url = track.album.images.first().map(|img| img.url.clone());
                let duration_ms = track.duration.num_milliseconds() as u32;
                let track_uri = track
                    .id
                    .as_ref()
                    .map(|id| format!("spotify:track:{}", id.id()))
                    .unwrap_or_default();
                (
                    Some(track_name),
                    artist_name,
                    album_art_url,
                    duration_ms,
                    Some(track_uri),
                )
            }
            Some(rspotify::model::PlayableItem::Episode(episode)) => {
                let name = episode.name.clone();
                let artist_name = Some(episode.show.publisher.clone());
                let album_art_url = episode.show.images.first().map(|img| img.url.clone());
                let duration_ms = episode.duration.num_milliseconds() as u32;
                let track_uri = format!("spotify:episode:{}", episode.id.id());
                (
                    Some(name),
                    artist_name,
                    album_art_url,
                    duration_ms,
                    Some(track_uri),
                )
            }
            None => (None, None, None, 0, None),
        };

        Self {
            is_playing: ctx.is_playing,
            progress_ms: ctx
                .progress
                .map(|d| d.num_milliseconds() as u32)
                .unwrap_or(0),
            duration_ms,
            volume: ctx.device.volume_percent.unwrap_or(50),
            current_track_name: track_name,
            current_artist_name: artist_name,
            current_album_art_url: album_art_url,
            current_album_art_data: None,
            current_track_uri: track_uri,
        }
    }

    /// Check if the current track has changed
    pub fn track_changed(&self, new_uri: Option<&str>) -> bool {
        match (&self.current_track_uri, new_uri) {
            (Some(current), Some(new)) => current != new,
            (None, Some(_)) => true,
            (Some(_), None) => true,
            (None, None) => false,
        }
    }
}

/// Format milliseconds as MM:SS
pub fn format_duration(ms: u32) -> String {
    let total_secs = ms / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{:02}:{:02}", mins, secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0), "00:00");
        assert_eq!(format_duration(1000), "00:01");
        assert_eq!(format_duration(60000), "01:00");
        assert_eq!(format_duration(61000), "01:01");
        assert_eq!(format_duration(125000), "02:05");
    }

    #[test]
    fn test_track_changed() {
        let mut state = PlayerState::default();
        state.current_track_uri = Some("spotify:track:abc".to_string());

        assert!(!state.track_changed(Some("spotify:track:abc")));
        assert!(state.track_changed(Some("spotify:track:def")));
        assert!(state.track_changed(None));

        state.current_track_uri = None;
        assert!(state.track_changed(Some("spotify:track:abc")));
        assert!(!state.track_changed(None));
    }
}
