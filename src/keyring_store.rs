//! OS keyring integration for secure credential storage
//!
//! Uses the native OS keyring service:
//! - Linux: Secret Service API (GNOME Keyring, KWallet)
//! - macOS: Keychain
//! - Windows: Windows Credential Manager
//!
//! Falls back to file-based storage if keyring is unavailable.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Service name for keyring entries
const KEYRING_SERVICE: &str = "joshify";
/// Username for keyring entries (fixed since we have one user)
const KEYRING_USER: &str = "spotify_credentials";

/// Cached credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureCredentials {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: u64,
}

impl SecureCredentials {
    pub fn is_expired(&self) -> bool {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|dur| dur.as_secs() >= self.expires_at)
            .unwrap_or(true)
    }
}

/// Get credentials from OS keyring
pub fn get_credentials_keyring() -> Result<Option<SecureCredentials>> {
    let keyring = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .context("Failed to create keyring entry")?;

    match keyring.get_password() {
        Ok(password) => {
            let creds: SecureCredentials = serde_json::from_str(&password)
                .context("Failed to parse credentials from keyring")?;
            Ok(Some(creds))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => {
            tracing::warn!("Keyring error: {}", e);
            Ok(None)
        }
    }
}

/// Save credentials to OS keyring
pub fn set_credentials_keyring(creds: &SecureCredentials) -> Result<()> {
    let keyring = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .context("Failed to create keyring entry")?;

    let json = serde_json::to_string(creds).context("Failed to serialize credentials")?;

    keyring
        .set_password(&json)
        .context("Failed to save credentials to keyring")?;

    Ok(())
}

/// Delete credentials from OS keyring
pub fn delete_credentials_keyring() -> Result<()> {
    let keyring = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .context("Failed to create keyring entry")?;

    match keyring.delete_password() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e).context("Failed to delete credentials from keyring"),
    }
}

/// Check if keyring is available
pub fn is_keyring_available() -> bool {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).is_ok()
}

/// Load credentials - tries keyring first, falls back to file
pub fn load_credentials_secure() -> Result<Option<SecureCredentials>> {
    // Try keyring first
    if let Some(creds) = get_credentials_keyring()? {
        return Ok(Some(creds));
    }

    // Fall back to file-based storage
    crate::auth::load_credentials().map(|opt| {
        opt.map(|c| SecureCredentials {
            access_token: c.access_token,
            refresh_token: c.refresh_token,
            expires_at: c.expires_at,
        })
    })
}

/// Save credentials - tries keyring first, falls back to file
pub fn save_credentials_secure(creds: &SecureCredentials) -> Result<()> {
    // Try keyring first
    if is_keyring_available() {
        match set_credentials_keyring(creds) {
            Ok(()) => {
                tracing::info!("Credentials saved to OS keyring");
                return Ok(());
            }
            Err(e) => {
                tracing::warn!("Keyring save failed: {}, falling back to file", e);
            }
        }
    }

    // Fall back to file-based storage
    crate::auth::save_credentials(&crate::auth::Credentials {
        access_token: creds.access_token.clone(),
        refresh_token: creds.refresh_token.clone(),
        expires_at: creds.expires_at,
    })
}
