//! API client tests
//!
//! Tests for rate limiting, backoff calculations, and client behavior.

// Test the rate_limit module
#[path = "../src/api/rate_limit.rs"]
mod rate_limit;

use rate_limit::{backoff_delay, with_rate_limit_retry, MAX_RETRIES};
use tokio::time::Duration;

#[test]
fn test_backoff_delay_initial() {
    // Attempt 0: 1 second delay (first retry)
    assert_eq!(backoff_delay(0), Duration::from_millis(1000));
}

#[test]
fn test_backoff_delay_exponential() {
    // Exponential backoff: 1s, 1s, 2s, 4s, 8s, 16s (capped at 16s)
    // Formula: 1000 * 2^(attempt-1), so attempt 0 and 1 both give 1s
    assert_eq!(backoff_delay(0), Duration::from_millis(1000));
    assert_eq!(backoff_delay(1), Duration::from_millis(1000));
    assert_eq!(backoff_delay(2), Duration::from_millis(2000));
    assert_eq!(backoff_delay(3), Duration::from_millis(4000));
    assert_eq!(backoff_delay(4), Duration::from_millis(8000));
    assert_eq!(backoff_delay(5), Duration::from_millis(16000));
}

#[test]
fn test_backoff_delay_capped() {
    // Beyond attempt 5, should stay at max (16s)
    assert_eq!(backoff_delay(6), Duration::from_millis(16000));
    assert_eq!(backoff_delay(10), Duration::from_millis(16000));
    // Test with safe value that won't overflow
    assert_eq!(backoff_delay(31), Duration::from_millis(16000));
}

#[test]
fn test_max_retries_constant() {
    // Verify MAX_RETRIES is 6 as documented
    assert_eq!(MAX_RETRIES, 6);
}

#[tokio::test]
async fn test_with_rate_limit_retry_success() {
    // Operation that succeeds immediately
    let result = with_rate_limit_retry(|| {
        std::boxed::Box::pin(async { Ok::<_, anyhow::Error>("success".to_string()) })
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");
}

#[tokio::test]
async fn test_with_rate_limit_retry_non_rate_limit_error() {
    // Operation that fails with non-rate-limit error should fail immediately
    let result = with_rate_limit_retry(|| {
        std::boxed::Box::pin(async { Err::<String, _>(anyhow::anyhow!("Network error")) })
    })
    .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Network error"));
}

#[tokio::test]
async fn test_with_rate_limit_retry_recovers() {
    use std::sync::{Arc, Mutex};

    // Operation that fails twice with rate limit, then succeeds
    let attempts = Arc::new(Mutex::new(0));
    let attempts_clone = attempts.clone();

    let result = with_rate_limit_retry(move || {
        let attempts = attempts_clone.clone();
        std::boxed::Box::pin(async move {
            let mut count = attempts.lock().unwrap();
            *count += 1;
            if *count < 3 {
                Err(anyhow::anyhow!("429 Too Many Requests"))
            } else {
                Ok::<_, anyhow::Error>("recovered".to_string())
            }
        })
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "recovered");
    assert_eq!(*attempts.lock().unwrap(), 3);
}

#[tokio::test]
async fn test_with_rate_limit_retry_exhausted() {
    // Operation that always fails with rate limit should exhaust retries
    let result = with_rate_limit_retry(|| {
        std::boxed::Box::pin(async { Err::<String, _>(anyhow::anyhow!("429 Too Many Requests")) })
    })
    .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("429"));
}

/// Mock OAuthConfig for testing
#[derive(Debug, Clone)]
struct MockOAuthConfig {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

impl Default for MockOAuthConfig {
    fn default() -> Self {
        Self {
            client_id: std::env::var("SPOTIFY_CLIENT_ID").unwrap_or_default(),
            client_secret: std::env::var("SPOTIFY_CLIENT_SECRET").unwrap_or_default(),
            redirect_uri: std::env::var("SPOTIFY_REDIRECT_URI")
                .unwrap_or_else(|_| "http://127.0.0.1:8888/callback".to_string()),
        }
    }
}

#[test]
fn test_oauth_config_default() {
    // Clear env vars for clean test
    std::env::remove_var("SPOTIFY_CLIENT_ID");
    std::env::remove_var("SPOTIFY_CLIENT_SECRET");
    std::env::remove_var("SPOTIFY_REDIRECT_URI");

    let config = MockOAuthConfig::default();

    assert_eq!(config.client_id, "");
    assert_eq!(config.client_secret, "");
    assert_eq!(config.redirect_uri, "http://127.0.0.1:8888/callback");
}

#[test]
fn test_oauth_config_from_env() {
    std::env::set_var("SPOTIFY_CLIENT_ID", "test_client");
    std::env::set_var("SPOTIFY_CLIENT_SECRET", "test_secret");
    std::env::set_var("SPOTIFY_REDIRECT_URI", "http://localhost:9999/callback");

    let config = MockOAuthConfig::default();

    assert_eq!(config.client_id, "test_client");
    assert_eq!(config.client_secret, "test_secret");
    assert_eq!(config.redirect_uri, "http://localhost:9999/callback");
}

#[tokio::test]
async fn test_invalid_operation_error() {
    // Test that invalid operations return proper errors
    let result = with_rate_limit_retry(|| {
        std::boxed::Box::pin(async {
            Err::<(), _>(anyhow::anyhow!("Invalid track URI: spotify:track:"))
        })
    })
    .await;

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Invalid track URI"));
}
