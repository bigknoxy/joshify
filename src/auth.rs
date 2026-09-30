//! OAuth authentication with Spotify
//!
//! Handles the OAuth flow:
//! 1. Open browser to Spotify OAuth URL
//! 2. User authorizes app
//! 3. Exchange code for tokens
//! 4. Cache tokens in ~/.config/spotify-tui/credentials.json

use anyhow::{Context, Result};
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
            client_id: args
                .client_id
                .clone()
                .or_else(|| std::env::var("SPOTIFY_CLIENT_ID").ok())
                .unwrap_or_default(),
            client_secret: args
                .client_secret
                .clone()
                .or_else(|| std::env::var("SPOTIFY_CLIENT_SECRET").ok())
                .unwrap_or_default(),
            redirect_uri: args
                .redirect_uri
                .clone()
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
        .or_else(|_| std::env::var("HOME").map(|dir| PathBuf::from(dir).join(".config/joshify")))
        .context("Failed to determine config directory")?;

    Ok(config_dir)
}

/// Load cached credentials from disk, keyring, or environment
pub fn load_credentials() -> Result<Option<Credentials>> {
    // First check environment variables (highest priority)
    if let Some(creds) = Credentials::from_env() {
        return Ok(Some(creds));
    }

    // Try OS keyring first (more secure)
    if let Ok(Some(creds)) = crate::keyring_store::get_credentials_keyring() {
        return Ok(Some(Credentials {
            access_token: creds.access_token,
            refresh_token: creds.refresh_token,
            expires_at: creds.expires_at,
        }));
    }

    // Then check disk cache
    let config_dir = get_config_dir()?;
    let creds_path = config_dir.join("credentials.json");

    if !creds_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(creds_path).context("Failed to read credentials file")?;
    let creds: Credentials =
        serde_json::from_str(&content).context("Failed to parse credentials JSON")?;
    Ok(Some(creds))
}

/// Save credentials to disk or OS keyring
pub fn save_credentials(creds: &Credentials) -> Result<()> {
    // Try OS keyring first (more secure)
    if crate::keyring_store::is_keyring_available() {
        let secure_creds = crate::keyring_store::SecureCredentials {
            access_token: creds.access_token.clone(),
            refresh_token: creds.refresh_token.clone(),
            expires_at: creds.expires_at,
        };
        match crate::keyring_store::set_credentials_keyring(&secure_creds) {
            Ok(()) => {
                println!("Credentials saved to OS keyring");
                return Ok(());
            }
            Err(e) => {
                eprintln!("Keyring save failed: {}, falling back to file", e);
            }
        }
    }

    // Fall back to disk cache
    let config_dir = get_config_dir()?;
    std::fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

    let creds_path = config_dir.join("credentials.json");
    let content = serde_json::to_string_pretty(creds).context("Failed to serialize credentials")?;
    std::fs::write(creds_path, content).context("Failed to write credentials file")?;

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
    ]
    .join(" ");

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

/// Exchange authorization code for access token
pub async fn exchange_code_for_token(config: &OAuthConfig, code: &str) -> Result<()> {
    use rspotify::{
        clients::OAuthClient, AuthCodeSpotify, Credentials as RspotifyCredentials, OAuth,
    };
    use std::collections::HashSet;

    let creds = RspotifyCredentials::new(&config.client_id, &config.client_secret);

    let oauth_config = OAuth {
        redirect_uri: config.redirect_uri.clone(),
        scopes: HashSet::from([
            "user-read-playback-state".to_string(),
            "user-modify-playback-state".to_string(),
            "user-read-currently-playing".to_string(),
            "streaming".to_string(),
            "playlist-read-private".to_string(),
            "playlist-modify-private".to_string(),
            "user-library-read".to_string(),
            "user-read-recently-played".to_string(),
        ]),
        ..Default::default()
    };

    let oauth = AuthCodeSpotify::new(creds, oauth_config);

    // Exchange the code for a token
    oauth
        .request_token(code)
        .await
        .context("Failed to exchange authorization code for token")?;

    // Get the token from the oauth client
    if let Ok(token_guard) = oauth.token.lock().await {
        if let Some(ref token) = *token_guard {
            let credentials = Credentials {
                access_token: token.access_token.clone(),
                refresh_token: token.refresh_token.clone(),
                expires_at: token
                    .expires_at
                    .map(|dt: chrono::DateTime<_>| dt.timestamp() as u64)
                    .unwrap_or(0),
            };
            save_credentials(&credentials)?;
        }
    }

    Ok(())
}

/// Run a local HTTP server to receive the OAuth callback
pub async fn run_oauth_callback_server(config: &OAuthConfig) -> Result<String> {
    use bytes::Bytes;
    use http_body_util::Full;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper::{Request, Response, StatusCode};
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
    let listener = tokio::net::TcpListener::bind(addr).await.context(format!(
        "Failed to bind to port {}. Make sure the redirect URI port is available.",
        port
    ))?;

    println!("Listening on http://{}", addr);
    println!("Waiting for Spotify callback...");

    let (tx, mut rx) = mpsc::channel::<String>(1);

    // Shutdown handshake: the outer task signals once it has the code, which
    // tells the in-flight connection to finish writing its response rather than
    // being aborted mid-write (#25).
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

    // Spawn the server
    let handle = tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                continue;
            };
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
                        if let Some(code) = query
                            .split('&')
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
                                .expect("static response builder should never fail");

                            // Send the code to the main task
                            let _ = tx.send(code.to_string()).await;

                            // Test-only: stall after the code is delivered but
                            // before the response is written, reproducing a slow
                            // write. The old fixed 100 ms sleep then aborted the
                            // connection mid-response, which is the #25 race.
                            #[cfg(test)]
                            {
                                let delay = crate::auth::test_response_delay_ms();
                                if delay > 0 {
                                    tokio::time::sleep(std::time::Duration::from_millis(delay))
                                        .await;
                                }
                            }

                            return Ok::<_, hyper::Error>(resp);
                        }
                    }

                    // Default 404
                    let body = Full::new(Bytes::from("Not found"));
                    let resp = Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(body)
                        .expect("static response builder should never fail");
                    Ok::<_, hyper::Error>(resp)
                }
            });

            let conn = http1::Builder::new().serve_connection(stream, service);
            tokio::pin!(conn);

            // Poll the connection until it finishes on its own, or until the
            // outer task signals that it has the code. In the latter case ask
            // hyper to drain the in-flight response instead of dropping the
            // connection, then keep polling until the write completes (#25).
            let shutdown_requested = tokio::select! {
                res = conn.as_mut() => {
                    if let Err(e) = res {
                        eprintln!("Error serving connection: {:?}", e);
                    }
                    false
                }
                _ = shutdown_rx.recv() => true,
            };

            if shutdown_requested {
                conn.as_mut().graceful_shutdown();
                if let Err(e) = conn.await {
                    eprintln!("Error serving connection: {:?}", e);
                }
            }

            // One connection per exchange: once the response has been written
            // the server has nothing left to serve, so return rather than
            // looping back to accept. Dropping the listener also releases the
            // port for the next run.
            return;
        }
    });

    // Wait for the code
    let code = rx.recv().await.context("Callback server failed")?;

    // Ask the server to stop, then wait for the in-flight response to be
    // written before returning. A fixed sleep raced that write: on a slow
    // socket the abort landed mid-response and the browser saw a connection
    // error even though the code had been received (#25).
    let _ = shutdown_tx.send(()).await;
    if tokio::time::timeout(SHUTDOWN_GRACE, handle).await.is_err() {
        // The server did not wind down in time; drop it rather than hang.
        eprintln!("OAuth callback server did not shut down gracefully");
    }

    Ok(code)
}

/// How long to wait for the callback response to flush before giving up on a
/// graceful stop and letting the dropped task end the server.
const SHUTDOWN_GRACE: std::time::Duration = std::time::Duration::from_secs(5);

/// Test-only: artificial delay inserted between delivering the code and
/// writing the response, so the shutdown race is reproducible on loopback
/// instead of depending on real machine load (#25).
#[cfg(test)]
pub(crate) fn set_test_response_delay_ms(ms: u64) {
    use std::sync::atomic::Ordering;
    TEST_RESPONSE_DELAY_MS.store(ms, Ordering::SeqCst);
}

#[cfg(test)]
fn test_response_delay_ms() -> u64 {
    use std::sync::atomic::Ordering;
    TEST_RESPONSE_DELAY_MS.load(Ordering::SeqCst)
}

#[cfg(test)]
static TEST_RESPONSE_DELAY_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal OAuthConfig pointing at a loopback port.
    fn config_for(port: u16) -> OAuthConfig {
        OAuthConfig {
            client_id: "test-client".to_string(),
            // A placeholder, not a credential: this exchange never leaves loopback.
            client_secret: "not-a-real-secret".to_string(),
            redirect_uri: format!("http://127.0.0.1:{}/callback", port),
        }
    }

    /// An ephemeral port that was free a moment ago, so the server can bind it.
    async fn free_port() -> u16 {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let p = l.local_addr().unwrap().port();
        drop(l);
        p
    }

    /// #25 — the success response must reach the client before the server stops.
    ///
    /// The response is deliberately stalled past the old fixed 100 ms sleep, so
    /// the old abort tears the connection down mid-response and the client sees
    /// a dropped connection instead of the success page (#25).
    #[tokio::test]
    async fn callback_response_reaches_the_client() {
        set_test_response_delay_ms(400);
        let port = free_port().await;
        let cfg = config_for(port);

        let server = tokio::spawn({
            let cfg = cfg.clone();
            async move { run_oauth_callback_server(&cfg).await }
        });

        // Give the listener a moment to bind.
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let url = format!("http://127.0.0.1:{}/callback?code=abc123", port);
        let resp = reqwest::get(&url)
            .await
            .expect("client should get a response, not a dropped connection");

        assert_eq!(
            resp.status(),
            200,
            "callback must answer 200, not a dropped connection"
        );
        let body = resp.text().await.unwrap();
        assert!(
            body.contains("Success"),
            "client should receive the success HTML body, got: {:?}",
            body
        );

        let code = server.await.unwrap().unwrap();
        assert_eq!(
            code, "abc123",
            "server should return the code from the query"
        );
        set_test_response_delay_ms(0);
    }

    /// The server must also stop serving after a successful exchange, or it
    /// would hold the port and keep the process alive.
    #[tokio::test]
    async fn server_stops_after_the_exchange() {
        set_test_response_delay_ms(0);
        let port = free_port().await;
        let cfg = config_for(port);

        let server = tokio::spawn({
            let cfg = cfg.clone();
            async move { run_oauth_callback_server(&cfg).await }
        });

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let url = format!("http://127.0.0.1:{}/callback?code=xyz789", port);
        let resp = reqwest::get(&url)
            .await
            .expect("first request should succeed");
        assert_eq!(resp.status(), 200);
        let _ = resp.text().await;

        let code = server.await.unwrap().unwrap();
        assert_eq!(code, "xyz789");

        // The port must be free again, proving the listener is gone.
        let rebound = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            tokio::net::TcpListener::bind(("127.0.0.1", port)),
        )
        .await;
        assert!(
            rebound.is_ok(),
            "callback server should release port {} after the exchange",
            port
        );
    }
}
