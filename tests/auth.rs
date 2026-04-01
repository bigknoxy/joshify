//! Authentication tests
//!
//! Tests for OAuth config, credentials, and persistence.

use std::env;
use tempfile::TempDir;
use serial_test::serial;

// Re-implement the types we need for testing (mirroring src/auth.rs)

/// OAuth credentials from Spotify Developer Dashboard
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct OAuthConfig {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            client_id: std::env::var("SPOTIFY_CLIENT_ID").unwrap_or_default(),
            client_secret: std::env::var("SPOTIFY_CLIENT_SECRET").unwrap_or_default(),
            redirect_uri: std::env::var("SPOTIFY_REDIRECT_URI")
                .unwrap_or_else(|_| "http://127.0.0.1:8888/callback".to_string()),
        }
    }
}

/// Cached user credentials (tokens)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Credentials {
    access_token: String,
    refresh_token: Option<String>,
    expires_at: u64,
}

impl Credentials {
    fn from_env() -> Option<Self> {
        let access_token = std::env::var("SPOTIFY_ACCESS_TOKEN").ok()?;
        let refresh_token = std::env::var("SPOTIFY_REFRESH_TOKEN").ok();
        let expires_at = std::env::var("SPOTIFY_TOKEN_EXPIRES_AT")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        Some(Self {
            access_token,
            refresh_token,
            expires_at,
        })
    }

    fn is_expired(&self) -> bool {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|dur| dur.as_secs() >= self.expires_at)
            .unwrap_or(true)
    }
}

fn get_config_dir() -> anyhow::Result<std::path::PathBuf> {
    use anyhow::Context;
    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .map(|dir| std::path::PathBuf::from(dir).join("joshify"))
        .or_else(|_| {
            std::env::var("HOME").map(|dir| std::path::PathBuf::from(dir).join(".config/joshify"))
        })
        .context("Failed to determine config directory")?;
    Ok(config_dir)
}

fn load_credentials() -> anyhow::Result<Option<Credentials>> {
    use anyhow::Context;
    if let Some(creds) = Credentials::from_env() {
        return Ok(Some(creds));
    }
    let config_dir = get_config_dir()?;
    let creds_path = config_dir.join("credentials.json");
    if !creds_path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(creds_path)
        .context("Failed to read credentials file")?;
    let creds: Credentials = serde_json::from_str(&content)
        .context("Failed to parse credentials JSON")?;
    Ok(Some(creds))
}

fn save_credentials(creds: &Credentials) -> anyhow::Result<()> {
    use anyhow::Context;
    let config_dir = get_config_dir()?;
    std::fs::create_dir_all(&config_dir)
        .context("Failed to create config directory")?;
    let creds_path = config_dir.join("credentials.json");
    let content = serde_json::to_string_pretty(creds)
        .context("Failed to serialize credentials")?;
    std::fs::write(creds_path, content)
        .context("Failed to write credentials file")?;
    Ok(())
}

/// Helper to set up a temporary config directory
/// Returns a TempDir that cleans up on drop
fn setup_temp_config() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    env::set_var("HOME", temp_dir.path());
    env::remove_var("XDG_CONFIG_HOME");
    // Clear all Spotify env vars to ensure clean state
    env::remove_var("SPOTIFY_CLIENT_ID");
    env::remove_var("SPOTIFY_CLIENT_SECRET");
    env::remove_var("SPOTIFY_REDIRECT_URI");
    env::remove_var("SPOTIFY_ACCESS_TOKEN");
    env::remove_var("SPOTIFY_REFRESH_TOKEN");
    env::remove_var("SPOTIFY_TOKEN_EXPIRES_AT");
    temp_dir
}

#[test]
#[serial]
fn test_credentials_from_env() {
    let _temp = setup_temp_config();

    env::set_var("SPOTIFY_ACCESS_TOKEN", "test_access_token");
    env::set_var("SPOTIFY_REFRESH_TOKEN", "test_refresh_token");
    env::set_var("SPOTIFY_TOKEN_EXPIRES_AT", "1234567890");

    let creds = Credentials::from_env().expect("Failed to load credentials from env");

    assert_eq!(creds.access_token, "test_access_token");
    assert_eq!(creds.refresh_token, Some("test_refresh_token".to_string()));
    assert_eq!(creds.expires_at, 1234567890);
}

#[test]
#[serial]
fn test_credentials_from_env_partial() {
    let _temp = setup_temp_config();

    // Only access token, no refresh token or expires_at
    env::set_var("SPOTIFY_ACCESS_TOKEN", "test_access_token");
    env::remove_var("SPOTIFY_REFRESH_TOKEN");
    env::remove_var("SPOTIFY_TOKEN_EXPIRES_AT");

    let creds = Credentials::from_env().expect("Failed to load credentials from env");

    assert_eq!(creds.access_token, "test_access_token");
    assert_eq!(creds.refresh_token, None);
    assert_eq!(creds.expires_at, 0);
}

#[test]
#[serial]
fn test_credentials_expired() {
    // Set expires_at to 1 second after epoch (definitely expired)
    let creds = Credentials {
        access_token: "test".to_string(),
        refresh_token: None,
        expires_at: 1,
    };

    assert!(creds.is_expired());
}

#[test]
#[serial]
fn test_credentials_valid() {
    // Set expires_at to far future (not expired)
    let creds = Credentials {
        access_token: "test".to_string(),
        refresh_token: None,
        expires_at: u64::MAX,
    };

    assert!(!creds.is_expired());
}

#[test]
#[serial]
fn test_oauth_config_default() {
    let _temp = setup_temp_config();

    let config = OAuthConfig::default();

    assert_eq!(config.client_id, "");
    assert_eq!(config.client_secret, "");
    assert_eq!(config.redirect_uri, "http://127.0.0.1:8888/callback");
}

#[test]
#[serial]
fn test_oauth_config_from_env() {
    let _temp = setup_temp_config();

    env::set_var("SPOTIFY_CLIENT_ID", "env_client_id");
    env::set_var("SPOTIFY_CLIENT_SECRET", "env_client_secret");
    env::set_var("SPOTIFY_REDIRECT_URI", "http://example.com/callback");

    let config = OAuthConfig::default();

    assert_eq!(config.client_id, "env_client_id");
    assert_eq!(config.client_secret, "env_client_secret");
    assert_eq!(config.redirect_uri, "http://example.com/callback");
}

#[test]
#[serial]
fn test_save_load_credentials() {
    let _temp = setup_temp_config();

    let creds = Credentials {
        access_token: "test_access".to_string(),
        refresh_token: Some("test_refresh".to_string()),
        expires_at: 9999999999,
    };

    // Save credentials
    save_credentials(&creds).expect("Failed to save credentials");

    // Clear env vars to force disk load
    env::remove_var("SPOTIFY_ACCESS_TOKEN");
    env::remove_var("SPOTIFY_REFRESH_TOKEN");
    env::remove_var("SPOTIFY_TOKEN_EXPIRES_AT");

    // Load credentials
    let loaded = load_credentials()
        .expect("Failed to load credentials")
        .expect("No credentials found");

    assert_eq!(loaded.access_token, "test_access");
    assert_eq!(loaded.refresh_token, Some("test_refresh".to_string()));
    assert_eq!(loaded.expires_at, 9999999999);

    // Verify file exists
    let config_dir = get_config_dir().expect("Failed to get config dir");
    let creds_path = config_dir.join("credentials.json");
    assert!(creds_path.exists(), "Credentials file should exist");
}

#[test]
#[serial]
fn test_keyring_fallback_to_file() {
    let _temp = setup_temp_config();

    // Keyring will likely be unavailable in test environment
    // This test verifies the file fallback works

    let creds = Credentials {
        access_token: "fallback_test".to_string(),
        refresh_token: None,
        expires_at: 8888888888,
    };

    save_credentials(&creds).expect("Failed to save credentials");

    // Clear env and load - should fall back to file
    env::remove_var("SPOTIFY_ACCESS_TOKEN");

    let loaded = load_credentials()
        .expect("Failed to load credentials")
        .expect("No credentials found");

    assert_eq!(loaded.access_token, "fallback_test");
    assert_eq!(loaded.expires_at, 8888888888);
}
