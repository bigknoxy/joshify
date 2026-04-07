//! Spotify client creation and authentication

use anyhow::Result;
use rspotify::{AuthCodeSpotify, Config, Credentials, OAuth};
use std::collections::HashSet;

use crate::auth::{load_credentials, OAuthConfig};

/// Spotify API client
pub struct SpotifyClient {
    pub oauth: AuthCodeSpotify,
}

impl SpotifyClient {
    /// Create a new Spotify client with auto token refresh enabled
    pub async fn new(config: &OAuthConfig) -> Result<Self> {
        let creds = Credentials::new(&config.client_id, &config.client_secret);

        let oauth_config = OAuth {
            redirect_uri: config.redirect_uri.clone(),
            scopes: HashSet::from([
                "user-read-playback-state".to_string(),
                "user-modify-playback-state".to_string(),
                "user-read-currently-playing".to_string(),
                "streaming".to_string(),
                "playlist-read-private".to_string(),
                "playlist-modify-private".to_string(),
                "playlist-modify-public".to_string(),
                "user-follow-modify".to_string(),
                "user-follow-read".to_string(),
                "user-library-modify".to_string(),
                "user-library-read".to_string(),
                "user-read-email".to_string(),
                "user-read-private".to_string(),
                "user-top-read".to_string(),
                "user-read-recently-played".to_string(),
            ]),
            ..Default::default()
        };

        // Enable automatic token refreshing
        let rspotify_config = Config {
            token_refreshing: true,
            ..Default::default()
        };

        let oauth = AuthCodeSpotify::with_config(creds, oauth_config, rspotify_config);

        let client = Self { oauth };

        // Try to load cached credentials and apply them to the OAuth client
        if let Some(creds) = load_credentials()? {
            // Check expiration after acquiring lock to avoid race condition
            if let Ok(mut token_guard) = client.oauth.token.lock().await {
                if !creds.is_expired() {
                    let token = rspotify::Token {
                        access_token: creds.access_token,
                        refresh_token: creds.refresh_token,
                        expires_at: Some(
                            chrono::DateTime::from_timestamp(creds.expires_at as i64, 0)
                                .unwrap_or(chrono::DateTime::UNIX_EPOCH),
                        ),
                        expires_in: chrono::TimeDelta::seconds(3600),
                        scopes: HashSet::new(),
                    };
                    *token_guard = Some(token);
                    tracing::debug!("Loaded cached credentials");
                } else {
                    tracing::warn!("Cached token expired - will attempt refresh on next API call");
                    // Even if expired, set the token so rspotify can attempt refresh
                    let token = rspotify::Token {
                        access_token: creds.access_token,
                        refresh_token: creds.refresh_token,
                        expires_at: Some(
                            chrono::DateTime::from_timestamp(creds.expires_at as i64, 0)
                                .unwrap_or(chrono::DateTime::UNIX_EPOCH),
                        ),
                        expires_in: chrono::TimeDelta::seconds(0),
                        scopes: HashSet::new(),
                    };
                    *token_guard = Some(token);
                }
            }
        }

        Ok(client)
    }
}
