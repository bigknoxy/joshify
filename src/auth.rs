//! OAuth authentication with Spotify
//!
//! Handles the OAuth flow:
//! 1. Open browser to Spotify OAuth URL
//! 2. User authorizes app
//! 3. Exchange code for tokens
//! 4. Cache tokens in ~/.config/spotify-tui/credentials.json

use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// OAuth credentials from Spotify Developer Dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
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

impl OAuthConfig {
    /// Create config from CLI args (args take precedence over env vars and config file)
    pub fn from_args(args: &crate::CliArgs) -> Self {
        Self {
            client_id: args.client_id.clone()
                .or_else(|| std::env::var("SPOTIFY_CLIENT_ID").ok())
                .unwrap_or_default(),
            client_secret: args.client_secret.clone()
                .or_else(|| std::env::var("SPOTIFY_CLIENT_SECRET").ok())
                .unwrap_or_default(),
            redirect_uri: args.redirect_uri.clone()
                .or_else(|| std::env::var("SPOTIFY_REDIRECT_URI").ok())
                .unwrap_or_else(|| "http://127.0.0.1:8888/callback".to_string()),
        }
    }
}

/// Cached user credentials (tokens)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credentials {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64,
}

impl Credentials {
    /// Load credentials from environment variables
    pub fn from_env() -> Option<Self> {
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
}

impl Credentials {
    pub fn is_expired(&self) -> bool {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|dur| dur.as_secs() >= self.expires_at)
            .unwrap_or(true)
    }
}

pub fn get_config_dir() -> Result<PathBuf> {
    let config_dir = std::env::var("XDG_CONFIG_HOME")
        .map(|dir| PathBuf::from(dir).join("joshify"))
        .or_else(|_| {
            std::env::var("HOME").map(|dir| PathBuf::from(dir).join(".config/joshify"))
        })
        .context("Failed to determine config directory")?;

    Ok(config_dir)
}

/// Load cached credentials from disk or environment
pub fn load_credentials() -> Result<Option<Credentials>> {
    // First check environment variables (highest priority)
    if let Some(creds) = Credentials::from_env() {
        return Ok(Some(creds));
    }

    // Then check disk cache
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

/// Save credentials to disk
pub fn save_credentials(creds: &Credentials) -> Result<()> {
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

/// Load OAuth config from disk or environment
pub fn load_oauth_config() -> Result<OAuthConfig> {
    let config_dir = get_config_dir()?;
    let config_path = config_dir.join("config.json");

    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        let config: OAuthConfig = serde_json::from_str(&content)?;
        Ok(config)
    } else {
        // Fall back to environment variables
        Ok(OAuthConfig::default())
    }
}

/// Save OAuth config to disk
pub fn save_oauth_config(config: &OAuthConfig) -> Result<()> {
    let config_dir = get_config_dir()?;
    std::fs::create_dir_all(&config_dir)?;

    let config_path = config_dir.join("config.json");
    let content = serde_json::to_string_pretty(config)?;
    std::fs::write(config_path, content)?;

    Ok(())
}

/// Build Spotify OAuth URL
pub fn get_oauth_url(config: &OAuthConfig) -> Result<String> {
    let scopes = [
        "user-read-playback-state",
        "user-modify-playback-state",
        "user-read-currently-playing",
        "streaming",
        "playlist-read-private",
        "playlist-modify-private",
        "playlist-modify-public",
        "user-follow-modify",
        "user-follow-read",
        "user-library-modify",
        "user-library-read",
        "user-read-email",
        "user-read-private",
        "user-top-read",
        "user-read-recently-played",
    ].join(" ");

    let mut url = url::Url::parse("https://accounts.spotify.com/authorize")?;
    url.query_pairs_mut()
        .append_pair("client_id", &config.client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", &config.redirect_uri)
        .append_pair("scope", &scopes);

    Ok(url.to_string())
}

/// Open the OAuth URL in the default browser
pub fn open_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .context("Failed to open browser. Please install xdg-utils.")?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .context("Failed to open browser")?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .arg("/c")
            .arg("start")
            .arg(url)
            .spawn()
            .context("Failed to open browser")?;
    }

    println!("Opening browser for authentication...");
    println!("If the browser didn't open, visit: {}", url);

    Ok(())
}

/// Clear cached credentials (for logout)
pub fn clear_credentials() -> Result<()> {
    let config_dir = get_config_dir()?;
    let creds_path = config_dir.join("credentials.json");

    if creds_path.exists() {
        std::fs::remove_file(creds_path)?;
    }

    Ok(())
}

/// Exchange authorization code for access token
pub async fn exchange_code_for_token(config: &OAuthConfig, code: &str) -> Result<()> {
    use rspotify::{Credentials as RspotifyCredentials, OAuth, AuthCodeSpotify, clients::OAuthClient};
    use std::collections::HashSet;

    let creds = RspotifyCredentials::new(&config.client_id, &config.client_secret);

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

    // Exchange the code for a token
    oauth.request_token(code).await
        .context("Failed to exchange authorization code for token")?;

    // Get the token from the oauth client
    if let Ok(token_guard) = oauth.token.lock().await {
        if let Some(ref token) = *token_guard {
            let credentials = Credentials {
                access_token: token.access_token.clone(),
                refresh_token: token.refresh_token.clone(),
                expires_at: token.expires_at.map(|dt: chrono::DateTime<_>| dt.timestamp() as u64).unwrap_or(0),
            };
            save_credentials(&credentials)?;
        }
    }

    Ok(())
}

/// Run a local HTTP server to receive the OAuth callback
pub async fn run_oauth_callback_server(config: &OAuthConfig) -> Result<String> {
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper::{Request, Response, StatusCode};
    use http_body_util::Full;
    use bytes::Bytes;
    use std::net::SocketAddr;
    use tokio::sync::mpsc;

    // Parse the redirect URI to get the port
    let redirect_uri = &config.redirect_uri;
    let port = redirect_uri
        .split(':')
        .nth(2)
        .unwrap_or("8888")
        .split('/')
        .next()
        .unwrap_or("8888")
        .parse::<u16>()
        .unwrap_or(8888);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await
        .context(format!("Failed to bind to port {}. Make sure the redirect URI port is available.", port))?;

    println!("Listening on http://{}", addr);
    println!("Waiting for Spotify callback...");

    let (tx, mut rx) = mpsc::channel::<String>(1);

    // Spawn the server
    let handle = tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else { continue };
            let stream = hyper_util::rt::TokioIo::new(stream);
            let tx = tx.clone();

            let service = service_fn(move |req: Request<hyper::body::Incoming>| {
                let uri = req.uri().clone();
                let tx = tx.clone();
                async move {
                    let path = uri.path();
                    let query = uri.query().unwrap_or("");

                    if path == "/callback" || path.ends_with("/callback") {
                        // Parse the code from query string
                        if let Some(code) = query.split('&')
                            .find(|p| p.starts_with("code="))
                            .map(|p| &p[5..])
                        {
                            // Send success response
                            let body = Full::new(Bytes::from(
                                "<html><body><h1>Success! You can close this window and return to the terminal.</h1></body></html>"
                            ));
                            let resp = Response::builder()
                                .status(StatusCode::OK)
                                .header("Content-Type", "text/html")
                                .body(body)
                                .unwrap();

                            // Send the code to the main task
                            let _ = tx.send(code.to_string()).await;
                            return Ok::<_, hyper::Error>(resp);
                        }
                    }

                    // Default 404
                    let body = Full::new(Bytes::from("Not found"));
                    let resp = Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(body)
                        .unwrap();
                    Ok::<_, hyper::Error>(resp)
                }
            });

            if let Err(e) = http1::Builder::new().serve_connection(stream, service).await {
                eprintln!("Error serving connection: {:?}", e);
            }
        }
    });

    // Wait for the code
    let code = rx.recv().await.context("Callback server failed")?;

    // Give the server a moment to finish sending the response
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Abort the server task
    handle.abort();

    Ok(code)
}
