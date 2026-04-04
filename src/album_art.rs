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

    /// Load image from cache or download
    pub async fn get_or_fetch(&self, url: &str) -> Option<Vec<u8>> {
        // Check memory cache first
        {
            let mut cache_guard = self.cache.lock().await;
            if let Some(data) = cache_guard.get(url) {
                return Some(data.clone());
            }
        }

        // Check disk cache
        if let Some(ref cache_dir) = self.cache_dir {
            let cache_key = url_to_filename(url);
            let cache_path = cache_dir.join(&cache_key);

            if cache_path.exists() {
                if let Ok(data) = std::fs::read(&cache_path) {
                    let mut cache_guard = self.cache.lock().await;
                    cache_guard.put(url.to_string(), data.clone());
                    return Some(data);
                }
            }
        }

        // Download from URL with timeout
        use tokio::time::{timeout, Duration};
        match timeout(Duration::from_secs(10), reqwest::get(url)).await {
            Ok(Ok(response)) => {
                match response.bytes().await {
                    Ok(bytes) => {
                        let data = bytes.to_vec();

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

/// Convert URL to safe filename
fn url_to_filename(url: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    format!("art_{:x}", hasher.finish())
}
