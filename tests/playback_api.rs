//! Tests for playback API error handling

use joshify::api::SpotifyClient;
use joshify::auth::OAuthConfig;

#[tokio::test]
#[ignore = "requires Spotify credentials"]
async fn test_current_playback_no_device() {
    // Test that NO_ACTIVE_DEVICE returns Ok(None) not error
    let config = OAuthConfig {
        client_id: std::env::var("SPOTIFY_CLIENT_ID").unwrap_or_default(),
        client_secret: std::env::var("SPOTIFY_CLIENT_SECRET").unwrap_or_default(),
        redirect_uri: std::env::var("SPOTIFY_REDIRECT_URI")
            .unwrap_or_else(|_| "http://127.0.0.1:8888/callback".to_string()),
    };

    let client = SpotifyClient::new(&config).await.unwrap();
    let result = client.current_playback().await;

    // Should not error, should return None when no device active
    assert!(
        result.is_ok(),
        "current_playback should not error: {:?}",
        result
    );
}

#[tokio::test]
#[ignore = "requires Spotify credentials"]
async fn test_available_devices() {
    // Test that we can get available devices
    let config = OAuthConfig {
        client_id: std::env::var("SPOTIFY_CLIENT_ID").unwrap_or_default(),
        client_secret: std::env::var("SPOTIFY_CLIENT_SECRET").unwrap_or_default(),
        redirect_uri: std::env::var("SPOTIFY_REDIRECT_URI")
            .unwrap_or_else(|_| "http://127.0.0.1:8888/callback".to_string()),
    };

    let client = SpotifyClient::new(&config).await.unwrap();
    let result = client.available_devices().await;

    // Should not error
    assert!(
        result.is_ok(),
        "available_devices should not error: {:?}",
        result
    );

    // If successful, should return a Vec (possibly empty)
    if let Ok(devices) = result {
        println!("Found {} devices", devices.len());
    }
}

#[tokio::test]
#[ignore = "requires Spotify credentials"]
async fn test_transfer_playback() {
    // Test that we can transfer playback
    let config = OAuthConfig {
        client_id: std::env::var("SPOTIFY_CLIENT_ID").unwrap_or_default(),
        client_secret: std::env::var("SPOTIFY_CLIENT_SECRET").unwrap_or_default(),
        redirect_uri: std::env::var("SPOTIFY_REDIRECT_URI")
            .unwrap_or_else(|_| "http://127.0.0.1:8888/callback".to_string()),
    };

    let client = SpotifyClient::new(&config).await.unwrap();

    // First get devices
    let devices = client.available_devices().await.unwrap();

    if let Some(device) = devices.first() {
        if let Some(ref device_id) = device.id {
            let result = client.transfer_playback(device_id).await;
            assert!(
                result.is_ok(),
                "transfer_playback should not error: {:?}",
                result
            );
        }
    }
}

#[test]
fn test_error_string_matching() {
    // Test that our error string matching works for various error formats

    let test_cases = vec![
        ("No active device found", true),
        ("NO_ACTIVE_DEVICE", true),
        ("no active device", true),
        ("No_Active_Device", true),
        ("Playback failed", false),
        ("Token expired", false),
        ("", false),
    ];

    for (error_msg, should_match) in test_cases {
        let matches = error_msg.contains("NO_ACTIVE_DEVICE")
            || error_msg.contains("no active device")
            || error_msg.to_lowercase().contains("no active device")
            || error_msg.to_lowercase().contains("no_active_device");

        assert_eq!(
            matches, should_match,
            "Error matching failed for: {}",
            error_msg
        );
    }
}
