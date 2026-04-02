//! Spotify client creation and authentication

use anyhow::Result;
use rspotify::{
    AuthCodeSpotify,
    Credentials, OAuth,
};
use std::collections::HashSet;

use crate::auth::{OAuthConfig, load_credentials};

/// Spotify API client
pub struct SpotifyClient {
    pub(crate) oauth: AuthCodeSpotify,
}

impl SpotifyClient {
    /// Create a new Spotify client
    pub async fn new(config: &OAuthConfig) -> Result<Self> {
        let creds = Credentials::new(&config.client_id, &config.client_secret);

        let mut oauth_config = OAuth::default();
        oauth_config.redirect_uri = config.redirect_uri.clone();
        oauth_config.scopes = HashSet::from([
            "user-read-playback-state".to_string(),
            "user-modify-playback-state".to_string(),
            "user-read-currently-playing".to_string(),
            "streaming".to_string(),
            "playlist-read-private".to_string(),
            "playlist-modify-private".to_string(),
            "user-library-read".to_string(),
            "user-read-recently-played".to_string(),
        ]);

        let oauth = AuthCodeSpotify::new(creds, oauth_config);

        let client = Self { oauth };

        // Try to load cached credentials and apply them to the OAuth client
        if let Some(creds) = load_credentials()? {
            // Check expiration after acquiring lock to avoid race condition
            if let Ok(mut token_guard) = client.oauth.token.lock().await {
                if !creds.is_expired() {
                    let token = rspotify::Token {
                        access_token: creds.access_token,
                        refresh_token: creds.refresh_token,
                        expires_at: Some(chrono::DateTime::from_timestamp(creds.expires_at as i64, 0)
                            .unwrap_or(chrono::DateTime::UNIX_EPOCH)),
                        expires_in: chrono::TimeDelta::seconds(3600),
                        scopes: HashSet::new(),
                    };
                    *token_guard = Some(token);
                    println!("Loaded cached credentials");
                } else {
                    println!("Cached token expired - re-authentication needed");
                }
            }
        }

        Ok(client)
    }
}
