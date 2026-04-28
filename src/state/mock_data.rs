//! Mock data for visual testing
//!
//! Provides fake data for VHS visual testing without requiring
//! Spotify authentication. Enable with JOSHIFY_MOCK=1 environment variable.
//!
//! # Example
//! ```bash
//! JOSHIFY_MOCK=1 cargo run
//! ```

use super::app_state::{
    AlbumListItem, ArtistListItem, ContentState, NavItem, PlaylistListItem, TrackListItem,
};
use super::player_state::{PlayerState, RepeatMode};

/// Check if mock mode is enabled via environment variable
pub fn is_mock_mode() -> bool {
    std::env::var("JOSHIFY_MOCK").is_ok()
}

/// Get mock tracks for display
pub fn get_mock_tracks() -> Vec<TrackListItem> {
    vec![
        TrackListItem {
            name: "Never Gonna Give You Up".to_string(),
            artist: "Rick Astley".to_string(),
            uri: "spotify:track:mock1".to_string(),
        },
        TrackListItem {
            name: "Bohemian Rhapsody".to_string(),
            artist: "Queen".to_string(),
            uri: "spotify:track:mock2".to_string(),
        },
        TrackListItem {
            name: "Hotel California".to_string(),
            artist: "Eagles".to_string(),
            uri: "spotify:track:mock3".to_string(),
        },
        TrackListItem {
            name: "Sweet Child O' Mine".to_string(),
            artist: "Guns N' Roses".to_string(),
            uri: "spotify:track:mock4".to_string(),
        },
        TrackListItem {
            name: "Billie Jean".to_string(),
            artist: "Michael Jackson".to_string(),
            uri: "spotify:track:mock5".to_string(),
        },
        TrackListItem {
            name: "Livin' on a Prayer".to_string(),
            artist: "Bon Jovi".to_string(),
            uri: "spotify:track:mock6".to_string(),
        },
        TrackListItem {
            name: "Imagine".to_string(),
            artist: "John Lennon".to_string(),
            uri: "spotify:track:mock7".to_string(),
        },
        TrackListItem {
            name: "Smells Like Teen Spirit".to_string(),
            artist: "Nirvana".to_string(),
            uri: "spotify:track:mock8".to_string(),
        },
    ]
}

/// Get mock playlists for display
pub fn get_mock_playlists() -> Vec<PlaylistListItem> {
    vec![
        PlaylistListItem {
            name: "Discover Weekly".to_string(),
            id: "mock_pl_1".to_string(),
            track_count: 30,
        },
        PlaylistListItem {
            name: "Release Radar".to_string(),
            id: "mock_pl_2".to_string(),
            track_count: 30,
        },
        PlaylistListItem {
            name: "Liked Songs".to_string(),
            id: "mock_pl_3".to_string(),
            track_count: 142,
        },
        PlaylistListItem {
            name: "Chill Vibes".to_string(),
            id: "mock_pl_4".to_string(),
            track_count: 45,
        },
        PlaylistListItem {
            name: "Workout Mix".to_string(),
            id: "mock_pl_5".to_string(),
            track_count: 28,
        },
    ]
}

/// Get mock albums for display
pub fn get_mock_albums() -> Vec<AlbumListItem> {
    vec![
        AlbumListItem {
            name: "Whenever You Need Somebody".to_string(),
            artist: "Rick Astley".to_string(),
            id: "mock_album_1".to_string(),
            image_url: None,
            total_tracks: 10,
            release_year: Some(1987),
        },
        AlbumListItem {
            name: "A Night at the Opera".to_string(),
            artist: "Queen".to_string(),
            id: "mock_album_2".to_string(),
            image_url: None,
            total_tracks: 12,
            release_year: Some(1975),
        },
        AlbumListItem {
            name: "Hotel California".to_string(),
            artist: "Eagles".to_string(),
            id: "mock_album_3".to_string(),
            image_url: None,
            total_tracks: 9,
            release_year: Some(1976),
        },
        AlbumListItem {
            name: "Thriller".to_string(),
            artist: "Michael Jackson".to_string(),
            id: "mock_album_4".to_string(),
            image_url: None,
            total_tracks: 9,
            release_year: Some(1982),
        },
    ]
}

/// Get mock artists for display
pub fn get_mock_artists() -> Vec<ArtistListItem> {
    vec![
        ArtistListItem {
            name: "Rick Astley".to_string(),
            id: "mock_artist_1".to_string(),
            image_url: None,
            genres: vec!["pop".to_string(), "synth-pop".to_string()],
            follower_count: Some(2500000),
        },
        ArtistListItem {
            name: "Queen".to_string(),
            id: "mock_artist_2".to_string(),
            image_url: None,
            genres: vec!["rock".to_string(), "classic rock".to_string()],
            follower_count: Some(35000000),
        },
        ArtistListItem {
            name: "Michael Jackson".to_string(),
            id: "mock_artist_3".to_string(),
            image_url: None,
            genres: vec!["pop".to_string(), "r&b".to_string()],
            follower_count: Some(42000000),
        },
    ]
}

/// Get mock content state for a specific navigation item
pub fn get_mock_content_state(nav_item: &NavItem) -> ContentState {
    match nav_item {
        NavItem::Home => ContentState::Home,
        NavItem::Library => ContentState::Library {
            albums: get_mock_albums(),
            artists: get_mock_artists(),
            selected_tab: super::app_state::LibraryTab::Albums,
        },
        NavItem::Playlists => ContentState::Playlists(get_mock_playlists()),
        NavItem::LikedSongs => ContentState::LikedSongs(get_mock_tracks()),
    }
}

/// Get mock player state (simulating active playback)
pub fn get_mock_player_state() -> PlayerState {
    PlayerState {
        current_track_name: Some("Never Gonna Give You Up".to_string()),
        current_artist_name: Some("Rick Astley".to_string()),
        is_playing: true,
        progress_ms: 45000,  // 45 seconds in
        duration_ms: 213000, // 3:33 total
        volume: 75,
        repeat_mode: RepeatMode::Off,
        shuffle: false,
        ..Default::default()
    }
}

/// Initialize mock authentication and state
/// Call this at app startup when JOSHIFY_MOCK is set
pub fn init_mock_state(
    is_authenticated: &mut bool,
    content_state: &mut ContentState,
    player_state: &mut PlayerState,
) {
    if !is_mock_mode() {
        return;
    }

    // Mark as authenticated
    *is_authenticated = true;

    // Set initial content state (Home view)
    *content_state = ContentState::Home;

    // Set mock player state (now playing)
    *player_state = get_mock_player_state();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_tracks() {
        let tracks = get_mock_tracks();
        assert!(!tracks.is_empty());
        assert_eq!(tracks[0].name, "Never Gonna Give You Up");
    }

    #[test]
    fn test_mock_playlists() {
        let playlists = get_mock_playlists();
        assert!(!playlists.is_empty());
        assert_eq!(playlists[0].name, "Discover Weekly");
    }

    #[test]
    fn test_mock_player_state() {
        let state = get_mock_player_state();
        assert!(state.is_playing);
        assert_eq!(
            state.current_track_name,
            Some("Never Gonna Give You Up".to_string())
        );
    }

    #[test]
    fn test_mock_content_state() {
        let playlists_state = get_mock_content_state(&NavItem::Playlists);
        match playlists_state {
            ContentState::Playlists(list) => assert!(!list.is_empty()),
            _ => panic!("Expected Playlists content state"),
        }
    }
}
