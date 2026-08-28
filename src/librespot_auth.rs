//! OAuth credentials for local playback (librespot)
//!
//! Spotify's Web API (browsing playlists, search, library) accepts an access
//! token minted under any registered client ID. Actually decrypting and
//! streaming audio locally is different: librespot fetches a playback
//! license through Spotify's login5 service, which only recognizes a small
//! set of Spotify-approved client IDs. A token minted under the user's own
//! Developer Dashboard app — the one `joshify --setup` configures for the
//! Web API — is rejected there with `INVALID_CREDENTIALS`, even though the
//! exact same token works fine for the Web API itself.
//!
//! librespot's own examples authenticate against the community client ID
//! below for local playback, and Spotify's login5 grants it the scope a
//! self-registered app cannot get. This module does the same, caching only
//! the long-lived refresh token so only the first run (or one after the
//! refresh token is revoked) needs a browser.

use anyhow::{Context, Result};
use librespot_oauth::OAuthClientBuilder;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Client ID librespot's own examples and the wider open-source Spotify
/// client ecosystem use for local playback. Not a joshify secret — it is
/// public in librespot's source and shared by every client that uses it.
const CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";
const REDIRECT_URI: &str = "http://127.0.0.1:8898/login";
const SCOPES: &[&str] = &["streaming"];

#[derive(Serialize, Deserialize)]
struct CachedToken {
    refresh_token: String,
}

fn cache_path() -> Result<PathBuf> {
    Ok(crate::auth::get_config_dir()?.join("librespot_oauth.json"))
}

fn load_cached_refresh_token_at(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let cached: CachedToken = serde_json::from_str(&content).ok()?;
    if cached.refresh_token.is_empty() {
        return None;
    }
    Some(cached.refresh_token)
}

fn save_refresh_token_at(path: &Path, refresh_token: &str) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).context("Failed to create config directory")?;
    }
    let content = serde_json::to_string_pretty(&CachedToken {
        refresh_token: refresh_token.to_string(),
    })
    .context("Failed to serialize local playback OAuth cache")?;
    std::fs::write(path, content).context("Failed to write local playback OAuth cache")?;
    Ok(())
}

fn build_client() -> Result<librespot_oauth::OAuthClient> {
    OAuthClientBuilder::new(CLIENT_ID, REDIRECT_URI, SCOPES.to_vec())
        .open_in_browser()
        .build()
        .map_err(|e| anyhow::anyhow!("Failed to build local playback OAuth client: {e}"))
}

/// Try the cached refresh token, if any. On success, re-caches the
/// (possibly rotated) refresh token and returns the fresh access token.
async fn try_cached_refresh(client: &librespot_oauth::OAuthClient, path: &Path) -> Option<String> {
    let refresh_token = load_cached_refresh_token_at(path)?;
    match client.refresh_token_async(&refresh_token).await {
        Ok(token) => {
            if !token.refresh_token.is_empty() {
                let _ = save_refresh_token_at(path, &token.refresh_token);
            }
            Some(token.access_token)
        }
        Err(e) => {
            tracing::warn!("Cached local playback token refresh failed ({e})");
            None
        }
    }
}

/// Get an access token usable with `librespot::core::authentication::Credentials::with_access_token`.
///
/// Reuses a cached refresh token silently when possible; only the very
/// first call, or one after the refresh token is revoked, opens a browser.
pub async fn get_local_playback_token() -> Result<String> {
    let path = cache_path()?;
    let client = build_client()?;

    if let Some(access_token) = try_cached_refresh(&client, &path).await {
        return Ok(access_token);
    }

    println!(
        "Local playback needs a one-time authorization, separate from your Spotify app credentials."
    );
    let token = client
        .get_access_token_async()
        .await
        .map_err(|e| anyhow::anyhow!("Local playback authorization failed: {e}"))?;
    if !token.refresh_token.is_empty() {
        save_refresh_token_at(&path, &token.refresh_token)?;
    }
    Ok(token.access_token)
}

/// Get an access token for local playback without ever opening a browser.
///
/// For non-interactive/unattended runs (e.g. `SPOTIFY_ACCESS_TOKEN` set):
/// a first-run browser prompt would hang forever with nobody to click
/// "Allow", but reusing an already-cached token from a prior interactive
/// run is safe and still fixes local playback for those runs too.
pub async fn get_cached_local_playback_token() -> Option<String> {
    let path = cache_path().ok()?;
    let client = build_client().ok()?;
    try_cached_refresh(&client, &path).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_cache_file_yields_no_cached_token() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("librespot_oauth.json");
        assert!(load_cached_refresh_token_at(&path).is_none());
    }

    #[test]
    fn corrupt_cache_file_yields_no_cached_token() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("librespot_oauth.json");
        std::fs::write(&path, "not json").expect("write");
        assert!(load_cached_refresh_token_at(&path).is_none());
    }

    #[test]
    fn empty_refresh_token_in_cache_is_treated_as_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("librespot_oauth.json");
        std::fs::write(&path, r#"{"refresh_token":""}"#).expect("write");
        assert!(load_cached_refresh_token_at(&path).is_none());
    }

    #[test]
    fn save_then_load_round_trips_the_refresh_token() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nested").join("librespot_oauth.json");
        save_refresh_token_at(&path, "a-refresh-token").expect("save");
        assert_eq!(
            load_cached_refresh_token_at(&path),
            Some("a-refresh-token".to_string())
        );
    }

    #[test]
    fn save_overwrites_a_previous_token() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("librespot_oauth.json");
        save_refresh_token_at(&path, "old-token").expect("save");
        save_refresh_token_at(&path, "new-token").expect("save");
        assert_eq!(
            load_cached_refresh_token_at(&path),
            Some("new-token".to_string())
        );
    }

    /// Regressing either constant back to joshify's own app / broader scopes
    /// silently reintroduces INVALID_CREDENTIALS for local playback.
    #[test]
    fn authenticates_as_the_login5_approved_community_client_with_streaming_scope_only() {
        assert_eq!(CLIENT_ID, "65b708073fc0480ea92a077233ca87bd");
        assert_eq!(SCOPES, &["streaming"]);
    }
}
