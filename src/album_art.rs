//! Album art fetching and rendering
//!
//! Downloads and caches album art images for display in the terminal.
//! Uses ratatui-image for rendering with support for:
//! - Kitty graphics protocol (best quality)
//! - Sixel graphics (good fallback)
//! - iTerm2 inline images
//! - ASCII/Unicode fallback (chafa-style)

use lru::LruCache;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Arc;

/// Album art cache (cloneable via Arc)
/// Uses LRU cache with 50 entry limit to bound memory usage
#[derive(Clone)]
pub struct AlbumArtCache {
    cache: Arc<tokio::sync::Mutex<LruCache<String, Vec<u8>>>>,
    cache_dir: Option<PathBuf>,
}

impl Default for AlbumArtCache {
    fn default() -> Self {
        Self::new()
    }
}

impl AlbumArtCache {
    pub fn new() -> Self {
        // Set up cache directory
        let cache_dir = std::env::var("HOME")
            .ok()
            .map(|home| PathBuf::from(home).join(".cache/joshify/album_art"));

        if let Some(ref dir) = cache_dir {
            let _ = std::fs::create_dir_all(dir);
        }

        Self {
            cache: Arc::new(tokio::sync::Mutex::new(LruCache::new(
                NonZeroUsize::new(50).unwrap(),
            ))),
            cache_dir,
        }
    }

    /// Test/preview constructor with an explicit cache directory.
    pub fn with_cache_dir(dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&dir);
        Self {
            cache: Arc::new(tokio::sync::Mutex::new(LruCache::new(
                NonZeroUsize::new(50).unwrap(),
            ))),
            cache_dir: Some(dir),
        }
    }

    /// Load image from cache or download
    pub async fn get_or_fetch(&self, url: &str) -> Option<Vec<u8>> {
        // Check memory cache first
        {
            let mut cache_guard = self.cache.lock().await;
            if let Some(data) = cache_guard.get(url) {
                return Some(data.clone());
            }
        }

        // Check disk cache. Entries are validated on read so images poisoned
        // into the cache by older versions (non-2xx bodies saved verbatim)
        // are evicted instead of served forever.
        if let Some(ref cache_dir) = self.cache_dir {
            let cache_key = url_to_filename(url);
            let cache_path = cache_dir.join(&cache_key);

            if cache_path.exists() {
                match std::fs::read(&cache_path) {
                    Ok(data) => {
                        if validate_art_bytes(200, &data).is_some() {
                            let mut cache_guard = self.cache.lock().await;
                            cache_guard.put(url.to_string(), data.clone());
                            return Some(data);
                        }
                        // Poisoned entry: remove it so a re-fetch can succeed.
                        tracing::warn!("Removed invalid cached album art for {}", url);
                        let _ = std::fs::remove_file(&cache_path);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to read cached album art: {}", e);
                    }
                }
            }
        }

        // Download from URL with timeout
        use tokio::time::{timeout, Duration};
        match timeout(Duration::from_secs(10), reqwest::get(url)).await {
            Ok(Ok(response)) => {
                let status = response.status().as_u16();
                match response.bytes().await {
                    Ok(bytes) => {
                        // Validate BEFORE caching: reqwest returns Ok for 404s
                        // and error pages, and a poisoned disk entry would
                        // blank that album permanently.
                        let data = match validate_art_bytes(status, &bytes) {
                            Some(data) => data,
                            None => return None,
                        };

                        // Save to disk cache
                        if let Some(ref cache_dir) = self.cache_dir {
                            let cache_key = url_to_filename(url);
                            let _ = std::fs::write(cache_dir.join(&cache_key), &data);
                        }

                        let mut cache_guard = self.cache.lock().await;
                        cache_guard.put(url.to_string(), data.clone());
                        Some(data)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to read album art response: {}", e);
                        None
                    }
                }
            }
            Ok(Err(e)) => {
                tracing::warn!("Failed to fetch album art: {}", e);
                None
            }
            Err(_) => {
                tracing::warn!("Album art fetch timed out after 10s: {}", url);
                None
            }
        }
    }
}

/// Validate an album-art HTTP response before it is trusted or cached.
///
/// Requires a 2xx status AND bytes that decode as a real image — Spotify CDN
/// failures, proxies and captive portals all happily return HTML with a
/// non-2xx (or even 2xx) status.
pub fn validate_art_bytes(status: u16, body: &[u8]) -> Option<Vec<u8>> {
    if !(200..300).contains(&status) {
        tracing::debug!("Album art request returned status {}", status);
        return None;
    }
    match image::load_from_memory(body) {
        Ok(_) => Some(body.to_vec()),
        Err(e) => {
            tracing::warn!("Album art response is not a valid image: {}", e);
            None
        }
    }
}

/// Convert URL to safe filename
fn url_to_filename(url: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    format!("art_{:x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal valid 1x1 red PNG.
    const PNG_1X1: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xCF, 0xC0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB0, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    /// Falsifier for cache poisoning: non-2xx responses and non-image bodies
    /// must be rejected BEFORE they reach the permanent disk cache.
    #[test]
    fn test_validate_art_bytes_rejects_poison() {
        assert!(validate_art_bytes(404, b"not found").is_none());
        assert!(validate_art_bytes(500, PNG_1X1).is_none());
        let html = b"<html><body>error page</body></html>";
        assert!(validate_art_bytes(200, html).is_none());
    }

    #[test]
    fn test_validate_art_bytes_accepts_real_image() {
        let validated = validate_art_bytes(200, PNG_1X1).expect("valid PNG must pass");
        assert_eq!(validated, PNG_1X1.to_vec());
    }

    fn test_cache_with_dir() -> AlbumArtCache {
        let dir = std::env::temp_dir().join(format!(
            "joshify_art_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        AlbumArtCache::with_cache_dir(dir)
    }

    /// Falsifier: an image poisoned into the disk cache by older versions
    /// must be detected on read, evicted, and NOT served as art.
    #[tokio::test]
    async fn test_disk_cache_self_heals_from_poisoned_entry() {
        use std::fs;

        let cache = test_cache_with_dir();
        let dir = cache.cache_dir.as_ref().unwrap().clone();

        // Simulate a legacy poisoned entry (an HTML error page on disk).
        let poisoned_path = dir.join(url_to_filename("https://example.com/poison.png"));
        fs::write(&poisoned_path, b"<html>503 backorigin</html>").unwrap();

        let result = cache.get_or_fetch("https://example.com/poison.png").await;
        assert!(result.is_none(), "poisoned disk entry must not be served");
        assert!(
            !poisoned_path.exists(),
            "poisoned disk entry must be deleted so it can be re-fetched"
        );
    }

    /// A valid image already on disk must be served without any network call
    /// (the URL is intentionally unreachable).
    #[tokio::test]
    async fn test_disk_cache_still_serves_valid_entries() {
        use std::fs;

        let cache = test_cache_with_dir();
        let dir = cache.cache_dir.as_ref().unwrap().clone();

        let valid_path = dir.join(url_to_filename("file://invalid-host/ok.png"));
        fs::write(&valid_path, PNG_1X1).unwrap();

        let result = cache.get_or_fetch("file://invalid-host/ok.png").await;
        assert_eq!(result, Some(PNG_1X1.to_vec()));
    }
}
