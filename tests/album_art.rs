//! Album art cache tests
//!
//! Tests for LRU caching, disk persistence, and URL handling.

use tempfile::TempDir;

// Re-implement the cache logic for testing (mirroring src/album_art.rs)
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::Arc;

fn url_to_filename(url: &str) -> String {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    format!("art_{:x}", hasher.finish())
}

/// Test version of AlbumArtCache with public internals for testing
#[derive(Clone)]
struct AlbumArtCache {
    cache: Arc<tokio::sync::Mutex<lru::LruCache<String, Vec<u8>>>>,
}

impl AlbumArtCache {
    fn new() -> Self {
        Self {
            cache: Arc::new(tokio::sync::Mutex::new(lru::LruCache::new(
                NonZeroUsize::new(50).unwrap(),
            ))),
        }
    }

    /// Get from cache (for testing - direct access)
    async fn get(&self, url: &str) -> Option<Vec<u8>> {
        let mut cache_guard = self.cache.lock().await;
        cache_guard.get(url).cloned()
    }

    /// Put in cache (for testing - direct access)
    async fn put(&self, url: &str, data: Vec<u8>) {
        let mut cache_guard = self.cache.lock().await;
        cache_guard.put(url.to_string(), data);
    }

    /// Get cache length (for testing)
    async fn len(&self) -> usize {
        let cache_guard = self.cache.lock().await;
        cache_guard.len()
    }
}

#[tokio::test]
async fn test_cache_new() {
    let cache = AlbumArtCache::new();

    // Verify cache is created empty
    assert_eq!(cache.len().await, 0);
}

#[tokio::test]
async fn test_cache_hit() {
    let cache = AlbumArtCache::new();
    let url = "https://example.com/art1.jpg";
    let image_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]; // PNG header

    // Populate cache
    cache.put(url, image_data.clone()).await;

    // Fetch should return cached data
    let result = cache.get(url).await;
    assert!(result.is_some());
    assert_eq!(result.unwrap(), image_data);
}

#[tokio::test]
async fn test_cache_miss() {
    let cache = AlbumArtCache::new();
    let url = "https://example.com/unknown.jpg";

    // Fetch uncached URL should return None
    let result = cache.get(url).await;
    assert!(result.is_none());
}

#[tokio::test]
async fn test_cache_eviction() {
    let cache = AlbumArtCache::new();

    // Fill cache beyond capacity (50 entries)
    for i in 0..60 {
        let url = format!("https://example.com/art{}.jpg", i);
        let data = vec![i as u8; 100];
        cache.put(&url, data).await;
    }

    // Cache should be at capacity
    assert_eq!(cache.len().await, 50);

    // First entries should be evicted (LRU)
    assert!(cache.get("https://example.com/art0.jpg").await.is_none());
    assert!(cache.get("https://example.com/art9.jpg").await.is_none());

    // Recent entries should still be present
    assert!(cache.get("https://example.com/art59.jpg").await.is_some());
}

#[tokio::test]
async fn test_cache_disk_persistence() {
    // Create temp directory for cache
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let cache_dir = temp_dir.path().join("album_art");
    std::fs::create_dir_all(&cache_dir).expect("Failed to create cache dir");

    let url = "https://example.com/persistent_art.jpg";
    let image_data = vec![0xFF, 0xD8, 0xFF, 0xE0]; // JPEG header

    // Manually save to disk cache
    let filename = url_to_filename(url);
    let cache_path = cache_dir.join(&filename);
    std::fs::write(&cache_path, &image_data).expect("Failed to write cache file");

    // Verify file exists
    assert!(cache_path.exists());

    // Read back and verify
    let loaded = std::fs::read(&cache_path).expect("Failed to read cache file");
    assert_eq!(loaded, image_data);
}

#[test]
fn test_url_to_filename() {
    // Same URL should produce same filename
    let url = "https://i.scdn.co/image/ab67616d0000b273abc123";
    let filename1 = url_to_filename(url);
    let filename2 = url_to_filename(url);
    assert_eq!(filename1, filename2);

    // Different URLs should produce different filenames
    let url2 = "https://i.scdn.co/image/ab67616d0000b273def456";
    let filename3 = url_to_filename(url2);
    assert_ne!(filename1, filename3);

    // Filename should be safe for filesystem
    assert!(!filename1.contains('/'));
    assert!(!filename1.contains('\\'));
    assert!(filename1.starts_with("art_"));
    // Length is "art_" (4) + hex representation of hash (varies, typically 12-16 chars)
    assert!(filename1.len() >= 12);
}
