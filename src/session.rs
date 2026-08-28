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
use url::Url;

/// Environment variables checked, in order, for a forward proxy to reach
/// Spotify's access point through. Same names curl/npm/pip already respect,
/// so a corporate proxy set up for those tools covers librespot too.
const PROXY_ENV_VARS: &[&str] = &[
    "SPOTIFY_PROXY",
    "HTTPS_PROXY",
    "https_proxy",
    "ALL_PROXY",
    "all_proxy",
    "HTTP_PROXY",
    "http_proxy",
];

/// Build librespot's session config, honoring escape hatches for networks
/// that block Spotify's normal access-point port (4070/TCP).
///
/// Some corporate firewalls / SSL-inspecting proxies (Zscaler and similar)
/// allow only 80/443 outbound and silently drop everything else, so the
/// access-point handshake just hangs. librespot already supports both
/// workarounds it needs:
/// - `ap_port`: ask Spotify's resolver for an access point on that port
///   instead of 4070 (Spotify offers 443 and 80 as fallbacks).
/// - `proxy`: tunnel the access-point connection through an HTTP CONNECT
///   proxy, same as browsers do for HTTPS.
///
/// `SPOTIFY_AP_PORT` forces the port explicitly. Otherwise, if a proxy is
/// configured (checking the same env vars curl/npm/pip use), the port
/// defaults to 443 too, since most forward proxies only permit CONNECT to
/// that port.
fn session_config() -> SessionConfig {
    let mut config = SessionConfig::default();

    let forced_port = std::env::var("SPOTIFY_AP_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok());

    // A scheme-less value like "proxy.corp.example:8080" (a common way to
    // write these, and how some other tools document the same env vars)
    // still parses under `Url::parse` — but as scheme "proxy.corp.example"
    // with no host, not a host:port pair. Require a host so a value like
    // that fails loudly here instead of silently producing a useless tunnel
    // target later.
    let proxy = PROXY_ENV_VARS.iter().find_map(|name| {
        std::env::var(name)
            .ok()
            .filter(|v| !v.is_empty())
            .and_then(|v| Url::parse(&v).ok())
            .filter(|url| url.host().is_some())
    });

    if let Some(url) = &proxy {
        tracing::info!("Routing Spotify access-point connection through proxy {url}");
        config.proxy = Some(url.clone());
    }

    config.ap_port = match forced_port {
        Some(port) => Some(port),
        None if proxy.is_some() => Some(443),
        None => None,
    };
    if let Some(port) = config.ap_port {
        tracing::info!("Requesting a Spotify access point on port {port}");
    }

    config
}

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
        let session_config = session_config();
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
        let session_config = session_config();
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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn clear_env() {
        std::env::remove_var("SPOTIFY_AP_PORT");
        for name in PROXY_ENV_VARS {
            std::env::remove_var(name);
        }
    }

    #[test]
    #[serial]
    fn defaults_to_no_proxy_and_no_forced_port() {
        clear_env();
        let config = session_config();
        assert!(config.proxy.is_none());
        assert!(config.ap_port.is_none());
        clear_env();
    }

    #[test]
    #[serial]
    fn forced_port_applies_with_no_proxy() {
        clear_env();
        std::env::set_var("SPOTIFY_AP_PORT", "443");
        let config = session_config();
        assert!(config.proxy.is_none());
        assert_eq!(config.ap_port, Some(443));
        clear_env();
    }

    #[test]
    #[serial]
    fn proxy_env_var_sets_proxy_and_defaults_port_to_443() {
        clear_env();
        std::env::set_var("HTTPS_PROXY", "http://proxy.corp.example:8080");
        let config = session_config();
        assert_eq!(
            config.proxy.as_ref().map(|u| u.as_str()),
            Some("http://proxy.corp.example:8080/")
        );
        assert_eq!(config.ap_port, Some(443));
        clear_env();
    }

    #[test]
    #[serial]
    fn forced_port_overrides_proxy_default() {
        clear_env();
        std::env::set_var("HTTPS_PROXY", "http://proxy.corp.example:8080");
        std::env::set_var("SPOTIFY_AP_PORT", "80");
        let config = session_config();
        assert!(config.proxy.is_some());
        assert_eq!(config.ap_port, Some(80));
        clear_env();
    }

    #[test]
    #[serial]
    fn invalid_proxy_url_is_ignored() {
        clear_env();
        std::env::set_var("HTTPS_PROXY", "not a url");
        let config = session_config();
        assert!(config.proxy.is_none());
        assert!(config.ap_port.is_none());
        clear_env();
    }

    #[test]
    #[serial]
    fn scheme_less_proxy_value_is_ignored() {
        clear_env();
        std::env::set_var("HTTPS_PROXY", "proxy.corp.example:8080");
        let config = session_config();
        assert!(config.proxy.is_none());
        assert!(config.ap_port.is_none());
        clear_env();
    }
}
