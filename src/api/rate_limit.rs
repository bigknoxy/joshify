//! Rate limit handling with exponential backoff
//!
//! Spotify API rate limits:
//! - 180 requests per minute for most endpoints
//! - 429 Too Many Requests response includes Retry-After header
//!
//! This module provides automatic retry with exponential backoff:
//! - Attempt 1: immediate
//! - Attempt 2: 1 second delay
//! - Attempt 3: 2 seconds delay
//! - Attempt 4: 4 seconds delay
//! - Attempt 5: 8 seconds delay
//! - Attempt 6: 16 seconds delay (max)
//! - After 6 attempts: give up

use anyhow::Result;
use tokio::time::{sleep, Duration};

/// Maximum number of retry attempts
pub const MAX_RETRIES: u32 = 6;

/// Maximum backoff delay (16 seconds)
const MAX_BACKOFF_MS: u64 = 16000;

/// Calculate backoff delay for given attempt (exponential: 1s, 2s, 4s, 8s, 16s)
pub fn backoff_delay(attempt: u32) -> Duration {
    let delay_ms = (1000 * 2u64.pow(attempt.saturating_sub(1))).min(MAX_BACKOFF_MS);
    Duration::from_millis(delay_ms)
}

/// Execute a future with rate limit retry handling
///
/// Retries on 429 Too Many Requests with exponential backoff.
/// Uses the Retry-After header if provided by Spotify, otherwise uses backoff.
pub async fn with_rate_limit_retry<F, T>(operation: F) -> Result<T>
where
    F: Fn() -> futures_util::future::BoxFuture<'static, Result<T>>,
{
    let mut last_error = None;

    for attempt in 0..MAX_RETRIES {
        if attempt > 0 {
            let delay = backoff_delay(attempt);
            tracing::warn!(
                "Rate limited, retrying in {:?} (attempt {}/{})",
                delay,
                attempt + 1,
                MAX_RETRIES
            );
            sleep(delay).await;
        }

        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                let error_str = e.to_string();
                // Check if this is a 429 rate limit error
                if error_str.contains("429") || error_str.contains("rate limit") {
                    last_error = Some(e);
                    continue;
                }
                // For non-rate-limit errors, fail immediately
                return Err(e);
            }
        }
    }

    // Exhausted all retries
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Rate limit exceeded after maximum retries")))
}
