//! Interactive setup wizard for Spotify credentials

use anyhow::Result;
use dialoguer::{Input, theme::ColorfulTheme};

use crate::auth::{OAuthConfig, save_oauth_config, get_oauth_url, open_browser, exchange_code_for_token, run_oauth_callback_server};

/// Run the interactive setup wizard
pub fn run_setup() -> Result<OAuthConfig> {
    let theme = ColorfulTheme::default();

    println!("\n╔══════════════════════════════════════╗");
    println!("║       Joshify - Spotify Setup        ║");
    println!("╚══════════════════════════════════════╝\n");

    println!("Step 1: Get credentials from Spotify");
    println!("  • Go to: https://developer.spotify.com/dashboard");
    println!("  • Create an app (or select existing)");
    println!("  • Copy Client ID and Secret\n");

    // Get Client ID
    let client_id: String = Input::with_theme(&theme)
        .with_prompt("Client ID")
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.is_empty() { Err("Required") }
            else if input.len() < 10 { Err("Too short") }
            else { Ok(()) }
        })
        .interact_text()?;

    // Get Client Secret
    let client_secret: String = Input::with_theme(&theme)
        .with_prompt("Client Secret")
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.is_empty() { Err("Required") }
            else if input.len() < 10 { Err("Too short") }
            else { Ok(()) }
        })
        .interact_text()?;

    // Redirect URI
    let default_redirect = "http://127.0.0.1:8888/callback";
    let redirect_uri: String = Input::with_theme(&theme)
        .with_prompt("Redirect URI")
        .default(default_redirect.to_string())
        .interact_text()?;

    let redirect_uri = if redirect_uri.is_empty() ||
        (!redirect_uri.starts_with("http://") && !redirect_uri.starts_with("https://")) {
        default_redirect.to_string()
    } else {
        redirect_uri
    };

    // Save config
    let config = OAuthConfig {
        client_id,
        client_secret,
        redirect_uri,
    };
    save_oauth_config(&config)?;

    println!("\n✓ Credentials saved\n");
    Ok(config)
}

/// Run the OAuth browser flow
pub async fn run_oauth_flow(config: &OAuthConfig) -> Result<bool> {
    use crate::auth::load_credentials;

    // Check for existing valid credentials
    if let Some(creds) = load_credentials()? {
        if !creds.is_expired() {
            return Ok(true); // Already authenticated
        }
    }

    println!("\nStep 2: Authorize Joshify with Spotify");
    println!("  Opening browser...");

    let auth_url = get_oauth_url(config)?;
    open_browser(&auth_url)?;

    println!("  Callback listener: {}", config.redirect_uri);
    println!("  Waiting for authorization...\n");

    let code = run_oauth_callback_server(config).await?;

    println!("✓ Authorization received");
    println!("✓ Tokens saved\n");

    exchange_code_for_token(config, &code).await?;
    Ok(false) // Fresh auth completed
}

/// Check if credentials are configured, run setup if not
pub fn ensure_configured() -> Result<OAuthConfig> {
    use crate::auth::load_oauth_config;

    match load_oauth_config() {
        Ok(config) if !config.client_id.is_empty() => Ok(config),
        _ => {
            println!("\nNo credentials found. Running setup...\n");
            run_setup()
        }
    }
}
