//! librespot session management
//!
//! Handles creating a librespot session from an OAuth access token,
//! which allows local audio playback through the user's speakers.

use anyhow::{Context, Result};
use librespot::core::{
    authentication::Credentials, cache::Cache, config::SessionConfig, session::Session,
};
use std::path::PathBuf;
use std::sync::Arc;

/// Cache directory for librespot (stores credentials, audio cache)
fn cache_dir() -> Result<PathBuf> {
    let base = std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .context("Failed to determine cache directory")?;
    Ok(base.join("joshify"))
}

/// Create a librespot cache for credentials and audio data
fn create_cache() -> Result<Cache> {
    let dir = cache_dir()?;
    std::fs::create_dir_all(&dir).ok();
    let cache = Cache::new(Some(&dir), Some(&dir), Some(&dir.join("files")), None)
        .context("Failed to create librespot cache")?;
    Ok(cache)
}

/// Session manager for local Spotify playback
pub struct LocalSession {
    pub session: Session,
    pub cache: Cache,
}

impl LocalSession {
    /// Create a new librespot session from an OAuth access token
    pub async fn from_access_token(token: &str) -> Result<Self> {
        let session_config = SessionConfig::default();
        let cache = create_cache()?;
        let credentials = Credentials::with_access_token(token);

        let session = Session::new(session_config, Some(cache.clone()));
        session
            .connect(credentials, false)
            .await
            .context("Failed to connect to Spotify")?;

        tracing::info!(
            "librespot session connected for user: {}",
            session.username()
        );

        Ok(Self { session, cache })
    }

    /// Try to create a session from cached credentials
    pub async fn from_cache() -> Result<Self> {
        let session_config = SessionConfig::default();
        let cache = create_cache()?;

        let credentials = cache.credentials().context("No cached credentials found")?;

        let session = Session::new(session_config, Some(cache.clone()));
        session
            .connect(credentials, false)
            .await
            .context("Failed to connect with cached credentials")?;

        tracing::info!(
            "librespot session restored from cache for user: {}",
            session.username()
        );

        Ok(Self { session, cache })
    }

    /// Get the username of the connected session
    pub fn username(&self) -> String {
        self.session.username()
    }
}

/// Shared session type for use across the app
pub type SharedSession = Arc<LocalSession>;
