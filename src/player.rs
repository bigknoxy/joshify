//! Player state and playback controls

use rspotify::model::CurrentPlaybackContext;
use rspotify::prelude::Id;

#[derive(Debug, Clone)]
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

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            is_playing: false,
            progress_ms: 0,
            duration_ms: 0,
            volume: 50,
            current_track_name: None,
            current_artist_name: None,
            current_album_art_url: None,
            current_album_art_data: None,
            current_track_uri: None,
        }
    }
}

impl PlayerState {
    pub fn from_context(ctx: &CurrentPlaybackContext) -> Self {
        let (track_name, artist_name, album_art_url, duration_ms, track_uri) = match &ctx.item {
            Some(rspotify::model::PlayableItem::Track(track)) => {
                let track_name = track.name.clone();
                let artist_name = track.artists.first().map(|a| a.name.clone());
                let album_art_url = track.album.images.first().map(|img| img.url.clone());
                let duration_ms = track.duration.num_milliseconds() as u32;
                // Construct URI from track ID
                let track_uri = track.id.as_ref().map(|id| format!("spotify:track:{}", id.id())).unwrap_or_default();
                (Some(track_name), artist_name, album_art_url, duration_ms, Some(track_uri))
            }
            Some(rspotify::model::PlayableItem::Episode(episode)) => {
                let name = episode.name.clone();
                let artist_name = Some(episode.show.publisher.clone());
                let album_art_url = episode.show.images.first().map(|img| img.url.clone());
                let duration_ms = episode.duration.num_milliseconds() as u32;
                // Construct URI from episode ID
                let track_uri = format!("spotify:episode:{}", episode.id.id());
                (Some(name), artist_name, album_art_url, duration_ms, Some(track_uri))
            }
            None => (None, None, None, 0, None),
        };

        Self {
            is_playing: ctx.is_playing,
            progress_ms: ctx.progress.map(|d| d.num_milliseconds() as u32).unwrap_or(0),
            duration_ms,
            volume: ctx.device.volume_percent.unwrap_or(50),
            current_track_name: track_name,
            current_artist_name: artist_name,
            current_album_art_url: album_art_url,
            current_album_art_data: None, // Will be fetched asynchronously
            current_track_uri: track_uri,
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
