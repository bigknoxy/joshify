//! Player state tests
//!
//! Tests for format_duration, PlayerState, and track change detection.

// Tests for player state - run with: cargo test --test player
// Note: These tests use the binary crate, so we reference via crate::

#[path = "../src/state/player_state.rs"]
mod player_state;

use player_state::{format_duration, PlayerState, RepeatMode};

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
    // Zero
    assert_eq!(format_duration(0), "00:00");

    // Very large values (u32::MAX ms = 4294967295 / 1000 = 4294967 secs = 71582 mins 47 secs)
    assert_eq!(format_duration(u32::MAX), "71582:47");

    // Large but reasonable values
    assert_eq!(format_duration(3600000), "60:00"); // 1 hour
    assert_eq!(format_duration(7200000), "120:00"); // 2 hours
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
fn test_player_state_default() {
    // Test that default player state has sensible defaults
    let state = PlayerState::default();

    assert!(!state.is_playing);
    assert_eq!(state.progress_ms, 0);
    assert_eq!(state.duration_ms, 0);
    assert_eq!(state.volume, 0);
    assert_eq!(state.current_track_name, None);
    assert_eq!(state.current_artist_name, None);
    assert_eq!(state.current_album_art_url, None);
    assert_eq!(state.current_album_art_data, None);
    assert_eq!(state.current_track_uri, None);
}

#[test]
fn test_player_state_with_track() {
    // Test player state populated with track data
    let mut state = PlayerState {
        is_playing: true,
        progress_ms: 60000,
        duration_ms: 180000,
        volume: 75,
        current_track_name: Some("Test Track".to_string()),
        current_artist_name: Some("Test Artist".to_string()),
        current_album_art_url: Some("https://example.com/art.jpg".to_string()),
        current_album_art_data: Some(vec![0x89, 0x50, 0x4E, 0x47]), // PNG header
        current_track_uri: Some("spotify:track:abc123".to_string()),
        current_album_art_ascii: None,
        current_album_art_kitty: None,
        shuffle: false,
        repeat_mode: RepeatMode::Off,
    };

    assert!(state.is_playing);
    assert_eq!(state.progress_ms, 60000);
    assert_eq!(state.duration_ms, 180000);
    assert_eq!(state.volume, 75);
    assert_eq!(state.current_track_name, Some("Test Track".to_string()));
    assert_eq!(state.current_artist_name, Some("Test Artist".to_string()));
    assert_eq!(
        state.current_album_art_url,
        Some("https://example.com/art.jpg".to_string())
    );
    assert!(state.current_album_art_data.is_some());
    assert_eq!(
        state.current_track_uri,
        Some("spotify:track:abc123".to_string())
    );

    // Modify state
    state.is_playing = false;
    state.progress_ms = 90000;
    state.volume = 50;

    assert!(!state.is_playing);
    assert_eq!(state.progress_ms, 90000);
    assert_eq!(state.volume, 50);
}

#[test]
fn test_player_state_no_track() {
    // Test player state with no track (no active playback)
    let state = PlayerState {
        is_playing: false,
        progress_ms: 0,
        duration_ms: 0,
        volume: 50,
        current_track_name: None,
        current_artist_name: None,
        current_album_art_url: None,
        current_album_art_data: None,
        current_track_uri: None,
        current_album_art_ascii: None,
        current_album_art_kitty: None,
        shuffle: false,
        repeat_mode: RepeatMode::Off,
    };

    assert!(!state.is_playing);
    assert_eq!(state.progress_ms, 0);
    assert_eq!(state.duration_ms, 0);
    assert_eq!(state.volume, 50);
    assert_eq!(state.current_track_name, None);
    assert_eq!(state.current_artist_name, None);
    assert_eq!(state.current_album_art_url, None);
    assert_eq!(state.current_track_uri, None);
}

#[test]
fn test_player_state_episode_context() {
    // Test player state with episode/podcast data
    let state = PlayerState {
        is_playing: true,
        progress_ms: 1800000, // 30 minutes
        duration_ms: 3600000, // 1 hour
        volume: 50,
        current_track_name: Some("Test Episode".to_string()),
        current_artist_name: Some("Test Publisher".to_string()),
        current_album_art_url: Some("https://example.com/episode.jpg".to_string()),
        current_album_art_data: None,
        current_track_uri: Some("spotify:episode:xyz789".to_string()),
        current_album_art_ascii: None,
        current_album_art_kitty: None,
        shuffle: false,
        repeat_mode: RepeatMode::Off,
    };

    assert!(state.is_playing);
    assert_eq!(state.progress_ms, 1800000);
    assert_eq!(state.duration_ms, 3600000);
    assert_eq!(state.current_track_name, Some("Test Episode".to_string()));
    assert_eq!(
        state.current_artist_name,
        Some("Test Publisher".to_string())
    );
    assert_eq!(
        state.current_track_uri,
        Some("spotify:episode:xyz789".to_string())
    );
}
