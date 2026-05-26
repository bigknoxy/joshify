//! Radio mode service - provides "radio" functionality using non-deprecated Spotify APIs
//!
//! This module implements an artist-based radio that:
//! 1. Fetches top tracks from the seed track's artist
//! 2. Fetches related artists and their top tracks
//! 3. Mixes both sources for variety
//!
//! Design follows SOLID principles:
//! - Single Responsibility: Each component does one thing
//! - Open/Closed: New track sources can be added without changing existing code
//! - Liskov Substitution: Different track sources are interchangeable
//! - Interface Segregation: Clean trait boundaries
//! - Dependency Inversion: Depends on abstractions (traits)

use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashSet;

/// Represents a track that can be queued
#[derive(Debug, Clone, PartialEq)]
pub struct RadioTrack {
    pub uri: String,
    pub name: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration_ms: Option<u32>,
}

/// Trait for track sources (follows Interface Segregation + Dependency Inversion)
#[async_trait]
pub trait TrackSource: Send + Sync {
    /// Get tracks from this source
    async fn get_tracks(&self, seed_artist_id: &str, seed_track_id: &str, limit: u32) -> Result<Vec<RadioTrack>>;
    
    /// Get the source name for debugging
    fn source_name(&self) -> &'static str;
}

/// Configuration for radio mixing
#[derive(Debug, Clone)]
pub struct RadioMixConfig {
    /// Percentage of tracks from primary source (artist top tracks) - 0-100
    pub primary_source_percentage: u8,
    /// Total number of tracks to generate
    pub total_tracks: u32,
    /// Exclude the seed track from results
    pub exclude_seed_track: bool,
}

impl Default for RadioMixConfig {
    fn default() -> Self {
        Self {
            // 70% from seed artist, 30% from related artists
            primary_source_percentage: 70,
            total_tracks: 20,
            exclude_seed_track: true,
        }
    }
}

/// Radio service that mixes tracks from multiple sources
/// (follows Single Responsibility + Open/Closed)
pub struct RadioService {
    primary_source: Box<dyn TrackSource>,
    secondary_source: Box<dyn TrackSource>,
    config: RadioMixConfig,
}

impl RadioService {
    /// Create a new radio service with the given sources
    /// 
    /// # Type Parameters
    /// - `P`: Primary track source (must implement TrackSource)
    /// - `S`: Secondary track source (must implement TrackSource)
    pub fn new<P: TrackSource + 'static, S: TrackSource + 'static>(
        primary_source: P,
        secondary_source: S,
        config: RadioMixConfig,
    ) -> Self {
        Self {
            primary_source: Box::new(primary_source),
            secondary_source: Box::new(secondary_source),
            config,
        }
    }
    
    /// Generate a radio playlist
    /// 
    /// # Arguments
    /// - `seed_artist_id`: Spotify ID of the seed artist
    /// - `seed_track_id`: Spotify ID of the seed track (to exclude)
    /// - `seed_artist_name`: Name of the seed artist (for context)
    ///
    /// # Returns
    /// Mixed list of tracks for the radio playlist
    pub async fn generate_radio(
        &self,
        seed_artist_id: &str,
        seed_track_id: &str,
        _seed_artist_name: &str,
    ) -> Result<Vec<RadioTrack>> {
        let primary_limit = (self.config.total_tracks * self.config.primary_source_percentage as u32) / 100;
        let secondary_limit = self.config.total_tracks - primary_limit;
        
        // Fetch from both sources concurrently
        let (primary_tracks, secondary_tracks) = tokio::join!(
            self.primary_source.get_tracks(seed_artist_id, seed_track_id, primary_limit * 2), // Fetch extra for filtering
            self.secondary_source.get_tracks(seed_artist_id, seed_track_id, secondary_limit * 2),
        );
        
        let primary_tracks = primary_tracks?;
        let secondary_tracks = secondary_tracks?;
        
        // Mix the tracks (follows DRY - reusable mixing logic)
        let mixed = Self::mix_tracks(
            primary_tracks,
            secondary_tracks,
            self.config.primary_source_percentage,
            self.config.total_tracks,
            seed_track_id,
            self.config.exclude_seed_track,
        );
        
        Ok(mixed)
    }
    
    /// Mix tracks from multiple sources with proper distribution
    /// (DRY: reusable mixing algorithm)
    fn mix_tracks(
        primary: Vec<RadioTrack>,
        secondary: Vec<RadioTrack>,
        primary_percentage: u8,
        total: u32,
        seed_track_id: &str,
        exclude_seed: bool,
    ) -> Vec<RadioTrack> {
        let mut result = Vec::with_capacity(total as usize);
        let mut seen_uris: HashSet<String> = HashSet::new();
        
        // Calculate how many from each source
        let primary_count = ((total as u32 * primary_percentage as u32) / 100).max(1);
        
        // Add tracks from primary source
        for track in primary {
            if result.len() >= primary_count as usize {
                break;
            }
            let is_seed = exclude_seed && track.uri.contains(seed_track_id);
            if !is_seed && !seen_uris.contains(&track.uri) {
                seen_uris.insert(track.uri.clone());
                result.push(track);
            }
        }
        
        // Add tracks from secondary source
        for track in secondary {
            if result.len() >= total as usize {
                break;
            }
            let is_seed = exclude_seed && track.uri.contains(seed_track_id);
            if !is_seed && !seen_uris.contains(&track.uri) {
                seen_uris.insert(track.uri.clone());
                result.push(track);
            }
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    /// Mock track source for testing (follows Dependency Inversion)
    struct MockTrackSource {
        name: &'static str,
        tracks: Vec<RadioTrack>,
    }
    
    #[async_trait]
    impl TrackSource for MockTrackSource {
        async fn get_tracks(&self, _seed_artist_id: &str, _seed_track_id: &str, limit: u32) -> Result<Vec<RadioTrack>> {
            Ok(self.tracks.iter().take(limit as usize).cloned().collect())
        }
        
        fn source_name(&self) -> &'static str {
            self.name
        }
    }
    
    fn create_test_track(id: &str, name: &str, artist: &str) -> RadioTrack {
        RadioTrack {
            uri: format!("spotify:track:{}", id),
            name: name.to_string(),
            artist: artist.to_string(),
            album: Some("Test Album".to_string()),
            duration_ms: Some(180000),
        }
    }
    
    #[tokio::test]
    async fn test_radio_service_mixes_sources() {
        // Arrange
        let primary_tracks = vec![
            create_test_track("1", "Primary Track 1", "Artist A"),
            create_test_track("2", "Primary Track 2", "Artist A"),
            create_test_track("3", "Primary Track 3", "Artist A"),
        ];
        
        let secondary_tracks = vec![
            create_test_track("4", "Secondary Track 1", "Artist B"),
            create_test_track("5", "Secondary Track 2", "Artist B"),
            create_test_track("6", "Secondary Track 3", "Artist B"),
        ];
        
        let primary_source = MockTrackSource {
            name: "primary",
            tracks: primary_tracks,
        };
        
        let secondary_source = MockTrackSource {
            name: "secondary",
            tracks: secondary_tracks,
        };
        
        let config = RadioMixConfig {
            primary_source_percentage: 50,
            total_tracks: 4,
            exclude_seed_track: true,
        };
        
        let service = RadioService::new(primary_source, secondary_source, config);
        
        // Act
        let result = service.generate_radio("artist123", "seed456", "Test Artist").await;
        
        // Assert
        assert!(result.is_ok());
        let tracks = result.unwrap();
        assert_eq!(tracks.len(), 4);
        
        // Check that we have both primary and secondary tracks
        let primary_count = tracks.iter().filter(|t| t.artist == "Artist A").count();
        let secondary_count = tracks.iter().filter(|t| t.artist == "Artist B").count();
        
        assert!(primary_count >= 1, "Should have at least 1 primary track");
        assert!(secondary_count >= 1, "Should have at least 1 secondary track");
    }
    
    #[tokio::test]
    async fn test_excludes_seed_track() {
        // Arrange
        let seed_track_id = "seed123";
        
        let primary_tracks = vec![
            create_test_track(seed_track_id, "Seed Track", "Artist A"),
            create_test_track("2", "Other Track", "Artist A"),
        ];
        
        let secondary_tracks = vec![];
        
        let service = RadioService::new(
            MockTrackSource { name: "primary", tracks: primary_tracks },
            MockTrackSource { name: "secondary", tracks: secondary_tracks },
            RadioMixConfig::default(),
        );
        
        // Act
        let result = service.generate_radio("artist123", seed_track_id, "Test Artist").await.unwrap();
        
        // Assert
        assert!(!result.iter().any(|t| t.uri.contains(seed_track_id)),
            "Should not include the seed track");
    }
    
    #[tokio::test]
    async fn test_no_duplicate_tracks() {
        // Arrange
        let duplicate_track = create_test_track("1", "Duplicate", "Artist A");
        
        let primary_tracks = vec![
            duplicate_track.clone(),
            create_test_track("2", "Track 2", "Artist A"),
        ];
        
        let secondary_tracks = vec![
            duplicate_track.clone(), // Same track in secondary
            create_test_track("3", "Track 3", "Artist B"),
        ];
        
        let service = RadioService::new(
            MockTrackSource { name: "primary", tracks: primary_tracks },
            MockTrackSource { name: "secondary", tracks: secondary_tracks },
            RadioMixConfig {
                primary_source_percentage: 50,
                total_tracks: 10,
                exclude_seed_track: false,
            },
        );
        
        // Act
        let result = service.generate_radio("artist123", "seed456", "Test Artist").await.unwrap();
        
        // Assert
        let mut uris = HashSet::new();
        for track in &result {
            assert!(uris.insert(track.uri.clone()), "Should not have duplicate URIs");
        }
    }
    
    #[test]
    fn test_mix_tracks_distribution() {
        // Arrange
        let primary = vec![
            create_test_track("1", "P1", "A"),
            create_test_track("2", "P2", "A"),
            create_test_track("3", "P3", "A"),
            create_test_track("4", "P4", "A"),
        ];
        
        let secondary = vec![
            create_test_track("5", "S1", "B"),
            create_test_track("6", "S2", "B"),
            create_test_track("7", "S3", "B"),
            create_test_track("8", "S4", "B"),
        ];
        
        // Act - 75% primary target, but limited by available tracks
        let result = RadioService::mix_tracks(
            primary,
            secondary,
            75,
            8,
            "seed",
            false,
        );
        
        // Assert
        assert_eq!(result.len(), 8);
        let primary_count = result.iter().filter(|t| t.artist == "A").count();
        let secondary_count = result.iter().filter(|t| t.artist == "B").count();
        
        // With 4 primary tracks and 75% target (6 tracks), we get all 4 primary + 4 from secondary
        assert_eq!(primary_count, 4, "Should have all 4 primary tracks");
        assert_eq!(secondary_count, 4, "Should have 4 secondary tracks to fill to 8");
    }
}
