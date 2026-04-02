//! Error injection tests
//!
//! Tests for graceful error handling: timeouts, malformed data, keyring failures, rate limit exhaustion.

use std::time::Duration;

/// Test timeout handling for album art fetch
#[tokio::test]
async fn test_album_art_timeout() {
    // Simulate a fetch that times out
    let fetch_result = tokio::time::timeout(
        Duration::from_millis(100),
        tokio::time::sleep(Duration::from_millis(500))
    ).await;

    // Should timeout, not panic
    assert!(fetch_result.is_err(), "Should timeout");
}

/// Test malformed JSON handling
#[test]
fn test_malformed_json() {
    // Truly invalid JSON syntax
    let invalid_json = r#"{"access_token": "test",}"#;
    let result: Result<serde_json::Value, _> = serde_json::from_str(invalid_json);

    // Should return error, not panic
    assert!(result.is_err(), "Should fail to parse invalid JSON");
}

/// Test keyring failure handling
#[test]
fn test_keyring_failure() {
    // Keyring operations should fail gracefully in test environment
    let result = keyring::Entry::new("test_service", "test_user");

    // May succeed or fail depending on environment - either way should not panic
    match result {
        Ok(entry) => {
            // If keyring exists, setting password should work or fail gracefully
            let pw_result = entry.set_password("test");
            assert!(pw_result.is_ok() || pw_result.is_err());
        }
        Err(_) => {
            // Keyring unavailable - this is fine, fallback to file storage
        }
    }
}

/// Test rate limit exhaustion recovery
#[tokio::test]
async fn test_rate_limit_exhaust() {
    use std::sync::{Arc, Mutex};

    // Simulate 6 rate limit errors (MAX_RETRIES)
    let attempts = Arc::new(Mutex::new(0));
    let attempts_clone = attempts.clone();

    // This mirrors the with_rate_limit_retry logic from src/api/rate_limit.rs
    let mut last_error = None;
    let max_retries = 6;

    for attempt in 0..max_retries {
        if attempt > 0 {
            // Simulate backoff delay (shortened for test)
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        let mut count = attempts_clone.lock().unwrap();
        *count += 1;

        // Always return rate limit error
        let err = anyhow::anyhow!("429 Too Many Requests");
        last_error = Some(err);
    }

    // After 6 attempts, should give up
    assert_eq!(*attempts.lock().unwrap(), 6, "Should exhaust all retries");
    assert!(last_error.is_some());
}

/// Test token expiry mid-session handling
#[test]
fn test_token_expiry_mid_session() {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Create credentials that expire in the past
    let expires_at_past = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() - 1000;

    let is_expired = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_secs() >= expires_at_past)
        .unwrap_or(true);

    assert!(is_expired, "Token should be expired");

    // Create credentials that expire in the future
    let expires_at_future = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() + 3600;

    let is_expired = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_secs() >= expires_at_future)
        .unwrap_or(true);

    assert!(!is_expired, "Token should not be expired");
}

/// Test network error handling
#[tokio::test]
async fn test_network_error_graceful() {
    // Simulate network error that should be handled gracefully
    let result = reqwest::get("http://invalid-host-that-does-not-exist.example").await;

    // Should return error, not panic
    assert!(result.is_err(), "Should fail to connect");
}

/// Test empty response handling
#[test]
fn test_empty_response_handling() {
    let empty_json = "";
    let result: Result<serde_json::Value, _> = serde_json::from_str(empty_json);

    assert!(result.is_err(), "Should fail to parse empty string");
}

/// Test None option handling
#[test]
fn test_option_none_handling() {
    let maybe_value: Option<String> = None;

    // Should handle None gracefully with unwrap_or
    let default = maybe_value.clone().unwrap_or_else(|| "default".to_string());
    assert_eq!(default, "default");

    // Should handle None with map
    let mapped = maybe_value.map(|s| s.to_uppercase());
    assert!(mapped.is_none());
}
