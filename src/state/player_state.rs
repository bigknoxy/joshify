//! Player state and playback tracking
//!
//! Moved from player.rs to the state module for better organization.

use rspotify::model::{CurrentPlaybackContext, RepeatState};
use rspotify::prelude::Id;

/// Repeat mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepeatMode {
    #[default]
    Off,
    Track,
    Context,
}

impl RepeatMode {
    pub fn from_spotify(state: RepeatState) -> Self {
        match state {
            RepeatState::Off => Self::Off,
            RepeatState::Track => Self::Track,
            RepeatState::Context => Self::Context,
        }
    }

    pub fn cycle(&self) -> Self {
        match self {
            Self::Off => Self::Context,
            Self::Context => Self::Track,
            Self::Track => Self::Off,
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Off => "R",
            Self::Context => "R",
            Self::Track => "R¹",
        }
    }
}

/// Scrolling title state for marquee animation
#[derive(Debug, Clone, Default)]
pub enum TitleScrollState {
    #[default]
    Static,
    PausedAtStart {
        frames_left: usize,
    },
    Scrolling {
        fractional_offset: f32,
    },
    PausedAtEnd {
        frames_left: usize,
    },
}

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
    pub current_album_art_data: Option<Vec<u8>>,
    pub current_album_art_kitty: Option<Vec<u8>>,
    pub current_album_art_ascii: Option<Vec<ratatui::text::Line<'static>>>,
    pub current_track_uri: Option<String>,
    pub shuffle: bool,
    pub repeat_mode: RepeatMode,
    pub art_rendered_for_area: Option<ratatui::prelude::Rect>,
    /// Area where the last Kitty image was rendered (for clearing on resize/track change)
    pub last_kitty_render_area: Option<ratatui::prelude::Rect>,
    pub title_scroll_state: TitleScrollState,
}

impl PlayerState {
    /// Create player state from Spotify playback context
    pub fn from_context(ctx: &CurrentPlaybackContext) -> Self {
        let (track_name, artist_name, album_art_url, duration_ms, track_uri) = match &ctx.item {
            Some(rspotify::model::PlayableItem::Track(track)) => {
                let track_name = track.name.clone();
                let artist_name = track.artists.first().map(|a| a.name.clone());
                let album_art_url = track.album.images.first().map(|img| img.url.clone());
                let duration_ms = track.duration.num_milliseconds();
                tracing::debug!("Track '{}' duration: {} ms", track_name, duration_ms);
                let duration_ms = duration_ms.max(0) as u32;
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
                #[allow(deprecated)]
                let artist_name = Some(episode.show.publisher.clone());
                let album_art_url = episode.show.images.first().map(|img| img.url.clone());
                let duration_ms = episode.duration.num_milliseconds().max(0) as u32;
                let track_uri = format!("spotify:episode:{}", episode.id.id());
                (
                    Some(name),
                    artist_name,
                    album_art_url,
                    duration_ms,
                    Some(track_uri),
                )
            }
            Some(rspotify::model::PlayableItem::Unknown(_)) | None => (None, None, None, 0, None),
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
            current_album_art_kitty: None,
            current_album_art_ascii: None,
            current_track_uri: track_uri,
            shuffle: ctx.shuffle_state,
            repeat_mode: RepeatMode::from_spotify(ctx.repeat_state),
            art_rendered_for_area: None,
            last_kitty_render_area: None,
            title_scroll_state: TitleScrollState::Static,
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

    /// Advance the scrolling title animation by one frame
    /// Returns the current integer scroll offset
    pub fn tick_scroll(&mut self, title_display_width: usize, available_width: usize) -> usize {
        if title_display_width <= available_width {
            self.title_scroll_state = TitleScrollState::Static;
            return 0;
        }

        let max_offset = title_display_width - available_width;
        let pause_frames = 60; // ~2s at 30fps
        let scroll_speed: f32 = 0.27; // ~8 cols/sec at 30fps

        match &mut self.title_scroll_state {
            TitleScrollState::Static => {
                self.title_scroll_state = TitleScrollState::PausedAtStart {
                    frames_left: pause_frames,
                };
                0
            }
            TitleScrollState::PausedAtStart { frames_left } => {
                if *frames_left == 0 {
                    self.title_scroll_state = TitleScrollState::Scrolling {
                        fractional_offset: 0.0,
                    };
                } else {
                    *frames_left -= 1;
                }
                0
            }
            TitleScrollState::Scrolling { fractional_offset } => {
                *fractional_offset += scroll_speed;
                let int_offset = *fractional_offset as usize;
                if int_offset >= max_offset {
                    self.title_scroll_state = TitleScrollState::PausedAtEnd {
                        frames_left: pause_frames,
                    };
                    max_offset
                } else {
                    int_offset
                }
            }
            TitleScrollState::PausedAtEnd { frames_left } => {
                if *frames_left == 0 {
                    self.title_scroll_state = TitleScrollState::PausedAtStart {
                        frames_left: pause_frames,
                    };
                    0
                } else {
                    *frames_left -= 1;
                    max_offset
                }
            }
        }
    }

    /// Reset scroll state (call when track changes)
    pub fn reset_scroll(&mut self) {
        self.title_scroll_state = TitleScrollState::Static;
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
    fn test_format_duration_edge_cases() {
        assert_eq!(format_duration(0), "00:00");
        assert_eq!(format_duration(59999), "00:59");
        assert_eq!(format_duration(3600000), "60:00");
        assert_eq!(format_duration(7200000), "120:00");
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

    #[test]
    fn test_player_state_defaults() {
        let state = PlayerState::default();
        assert!(!state.is_playing);
        assert_eq!(state.progress_ms, 0);
        assert_eq!(state.duration_ms, 0);
        assert_eq!(state.volume, 0);
        assert!(state.current_track_name.is_none());
        assert!(state.current_artist_name.is_none());
        assert!(state.current_album_art_url.is_none());
        assert!(state.current_album_art_data.is_none());
        assert!(state.current_track_uri.is_none());
    }

    #[test]
    fn test_progress_increments_when_playing() {
        // Simulates the main loop progress increment logic
        let mut state = PlayerState {
            is_playing: true,
            progress_ms: 0,
            duration_ms: 180000,
            ..Default::default()
        };

        // Simulate 3 poll intervals (each ~1 second)
        let poll_interval = 1000u32;
        for _ in 0..3 {
            state.progress_ms = state
                .progress_ms
                .saturating_add(poll_interval)
                .min(state.duration_ms);
        }

        assert_eq!(state.progress_ms, 3000);
        assert!(state.is_playing);
    }

    #[test]
    fn test_progress_pauses_when_not_playing() {
        let mut state = PlayerState {
            is_playing: false,
            progress_ms: 45000,
            duration_ms: 180000,
            ..Default::default()
        };

        // Simulate poll intervals - progress should NOT increment
        let poll_interval = 1000u32;
        let initial_progress = state.progress_ms;
        for _ in 0..5 {
            // Only increment if is_playing (this is the guard in main loop)
            if state.is_playing {
                state.progress_ms = state
                    .progress_ms
                    .saturating_add(poll_interval)
                    .min(state.duration_ms);
            }
        }

        assert_eq!(state.progress_ms, initial_progress);
    }

    #[test]
    fn test_progress_clamps_to_duration() {
        let mut state = PlayerState {
            is_playing: true,
            progress_ms: 179000,
            duration_ms: 180000,
            ..Default::default()
        };

        // Simulate many poll intervals - should clamp at duration
        let poll_interval = 1000u32;
        for _ in 0..100 {
            state.progress_ms = state
                .progress_ms
                .saturating_add(poll_interval)
                .min(state.duration_ms);
        }

        assert_eq!(state.progress_ms, 180000);
        assert!(state.progress_ms <= state.duration_ms);
    }

    #[test]
    fn test_progress_saturating_add_no_overflow() {
        let mut state = PlayerState {
            is_playing: true,
            progress_ms: u32::MAX - 500,
            duration_ms: u32::MAX,
            ..Default::default()
        };

        // Should not panic or overflow
        state.progress_ms = state
            .progress_ms
            .saturating_add(1000)
            .min(state.duration_ms);

        assert_eq!(state.progress_ms, u32::MAX);
    }

    #[test]
    fn test_player_state_with_track() {
        let state = PlayerState {
            is_playing: true,
            progress_ms: 60000,
            duration_ms: 180000,
            volume: 75,
            current_track_name: Some("Test Track".to_string()),
            current_artist_name: Some("Test Artist".to_string()),
            current_album_art_url: Some("https://example.com/art.jpg".to_string()),
            current_album_art_data: Some(vec![0x89, 0x50, 0x4E, 0x47]),
            current_album_art_kitty: None,
            current_album_art_ascii: None,
            current_track_uri: Some("spotify:track:abc123".to_string()),
            shuffle: false,
            repeat_mode: RepeatMode::Off,
            ..Default::default()
        };

        assert!(state.is_playing);
        assert_eq!(state.progress_ms, 60000);
        assert_eq!(state.duration_ms, 180000);
        assert_eq!(state.volume, 75);
        assert_eq!(state.current_track_name, Some("Test Track".to_string()));
        assert_eq!(state.current_artist_name, Some("Test Artist".to_string()));
        assert!(state.current_album_art_data.is_some());
        assert_eq!(
            state.current_track_uri,
            Some("spotify:track:abc123".to_string())
        );
    }

    #[test]
    fn test_player_state_no_track() {
        let state = PlayerState {
            is_playing: false,
            progress_ms: 0,
            duration_ms: 0,
            volume: 50,
            current_track_name: None,
            current_artist_name: None,
            current_album_art_url: None,
            current_album_art_data: None,
            current_album_art_kitty: None,
            current_album_art_ascii: None,
            current_track_uri: None,
            shuffle: false,
            repeat_mode: RepeatMode::Off,
            ..Default::default()
        };

        assert!(!state.is_playing);
        assert_eq!(state.progress_ms, 0);
        assert_eq!(state.duration_ms, 0);
        assert_eq!(state.volume, 50);
        assert!(state.current_track_name.is_none());
        assert!(state.current_artist_name.is_none());
        assert!(state.current_track_uri.is_none());
    }

    #[test]
    fn test_player_state_episode_context() {
        let state = PlayerState {
            is_playing: true,
            progress_ms: 1800000,
            duration_ms: 3600000,
            volume: 50,
            current_track_name: Some("Test Episode".to_string()),
            current_artist_name: Some("Test Publisher".to_string()),
            current_album_art_url: Some("https://example.com/episode.jpg".to_string()),
            current_album_art_data: None,
            current_album_art_kitty: None,
            current_album_art_ascii: None,
            current_track_uri: Some("spotify:episode:xyz789".to_string()),
            shuffle: false,
            repeat_mode: RepeatMode::Off,
            ..Default::default()
        };

        assert!(state.is_playing);
        assert_eq!(state.progress_ms, 1800000);
        assert_eq!(state.duration_ms, 3600000);
        assert_eq!(
            state.current_track_uri,
            Some("spotify:episode:xyz789".to_string())
        );
    }

    #[test]
    fn test_progress_increments_with_real_elapsed_time() {
        // Simulates the main loop progress increment logic using real elapsed time
        let mut state = PlayerState {
            is_playing: true,
            progress_ms: 0,
            duration_ms: 180000,
            ..Default::default()
        };
        let mut last_tick_ms = 0u64;
        let mut current_ms = 0u64;

        // Simulate 3 seconds passing (each iteration = 1000ms)
        for _ in 0..3 {
            current_ms += 1000;
            let elapsed = current_ms.saturating_sub(last_tick_ms);
            if elapsed >= 1000 {
                state.progress_ms = state
                    .progress_ms
                    .saturating_add(elapsed as u32)
                    .min(state.duration_ms);
                last_tick_ms = current_ms;
            }
        }

        assert_eq!(state.progress_ms, 3000);
        assert_eq!(last_tick_ms, 3000);
    }

    #[test]
    fn test_progress_does_not_increment_when_paused() {
        let mut state = PlayerState {
            is_playing: false,
            progress_ms: 45000,
            duration_ms: 180000,
            ..Default::default()
        };
        let mut last_tick_ms = 0u64;
        let mut current_ms = 0u64;

        // Simulate 10 seconds passing
        for _ in 0..10 {
            current_ms += 1000;
            // Only increment if is_playing
            if state.is_playing {
                let elapsed = current_ms.saturating_sub(last_tick_ms);
                if elapsed >= 1000 {
                    state.progress_ms = state
                        .progress_ms
                        .saturating_add(elapsed as u32)
                        .min(state.duration_ms);
                    last_tick_ms = current_ms;
                }
            }
        }

        assert_eq!(state.progress_ms, 45000);
        assert_eq!(last_tick_ms, 0);
    }

    #[test]
    fn test_progress_clamps_at_duration() {
        let mut state = PlayerState {
            is_playing: true,
            progress_ms: 179000,
            duration_ms: 180000,
            ..Default::default()
        };
        let mut last_tick_ms = 0u64;
        let mut current_ms = 0u64;

        // Simulate 10 seconds passing
        for _ in 0..10 {
            current_ms += 1000;
            let elapsed = current_ms.saturating_sub(last_tick_ms);
            if elapsed >= 1000 {
                state.progress_ms = state
                    .progress_ms
                    .saturating_add(elapsed as u32)
                    .min(state.duration_ms);
                last_tick_ms = current_ms;
            }
        }

        assert_eq!(state.progress_ms, 180000);
        assert!(state.progress_ms <= state.duration_ms);
    }

    #[test]
    fn test_progress_handles_large_elapsed_time() {
        // If the loop is delayed (e.g., 5 seconds between polls), progress should catch up
        let mut state = PlayerState {
            is_playing: true,
            progress_ms: 10000,
            duration_ms: 180000,
            ..Default::default()
        };
        let mut last_tick_ms = 0u64;
        let current_ms = 5000u64; // 5 seconds elapsed

        let elapsed = current_ms.saturating_sub(last_tick_ms);
        if elapsed >= 1000 {
            state.progress_ms = state
                .progress_ms
                .saturating_add(elapsed as u32)
                .min(state.duration_ms);
            last_tick_ms = current_ms;
        }

        assert_eq!(state.progress_ms, 15000);
        assert_eq!(last_tick_ms, 5000);
    }

    #[test]
    fn test_progress_resets_on_track_change() {
        let mut state = PlayerState {
            is_playing: true,
            progress_ms: 120000,
            duration_ms: 180000,
            current_track_uri: Some("spotify:track:old".to_string()),
            ..Default::default()
        };

        // Simulate track change
        state.current_track_uri = Some("spotify:track:new".to_string());
        state.progress_ms = 0;
        state.duration_ms = 200000;

        assert_eq!(state.progress_ms, 0);
        assert_eq!(state.duration_ms, 200000);
        assert_eq!(
            state.current_track_uri,
            Some("spotify:track:new".to_string())
        );
    }

    #[test]
    fn test_progress_does_not_increment_before_threshold() {
        // If less than 1000ms has elapsed, progress should NOT increment
        let mut state = PlayerState {
            is_playing: true,
            progress_ms: 0,
            duration_ms: 180000,
            ..Default::default()
        };
        let mut last_tick_ms = 0u64;
        let mut current_ms = 0u64;

        // Simulate small increments (50ms each - like the event poll interval)
        for _ in 0..20 {
            current_ms += 50;
            let elapsed = current_ms.saturating_sub(last_tick_ms);
            if elapsed >= 1000 {
                state.progress_ms = state
                    .progress_ms
                    .saturating_add(elapsed as u32)
                    .min(state.duration_ms);
                last_tick_ms = current_ms;
            }
        }

        // After 20 * 50ms = 1000ms, should have incremented once
        assert_eq!(state.progress_ms, 1000);
        assert_eq!(last_tick_ms, 1000);
    }

    #[test]
    fn test_progress_saturating_add_no_overflow_with_elapsed_time() {
        let mut state = PlayerState {
            is_playing: true,
            progress_ms: u32::MAX - 500,
            duration_ms: u32::MAX,
            ..Default::default()
        };
        let mut last_tick_ms = 0u64;
        let current_ms = 1000u64;

        let elapsed = current_ms.saturating_sub(last_tick_ms);
        if elapsed >= 1000 {
            state.progress_ms = state
                .progress_ms
                .saturating_add(elapsed as u32)
                .min(state.duration_ms);
            last_tick_ms = current_ms;
        }

        assert_eq!(state.progress_ms, u32::MAX);
    }
}
