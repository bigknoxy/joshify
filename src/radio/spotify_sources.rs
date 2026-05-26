//! Spotify-specific implementations of TrackSource
//!
//! Provides concrete implementations that fetch tracks from Spotify's API
//! following SOLID principles (Dependency Inversion, Single Responsibility)

use super::{RadioTrack, TrackSource};
use anyhow::{Context, Result};
use async_trait::async_trait;
use rspotify::prelude::Id;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Spotify API client wrapper for radio functionality
pub struct SpotifyRadioClient {
    client: Arc<Mutex<crate::api::SpotifyClient>>,
}

impl SpotifyRadioClient {
    /// Create a new Spotify radio client
    pub fn new(client: Arc<Mutex<crate::api::SpotifyClient>>) -> Self {
        Self { client }
    }
    
    /// Get the inner client (for use by source implementations)
    pub fn client(&self) -> Arc<Mutex<crate::api::SpotifyClient>> {
        self.client.clone()
    }
}

/// Track source that fetches top tracks from the seed artist
/// (Primary source for radio - 70% of tracks)
pub struct ArtistTopTracksSource {
    spotify: SpotifyRadioClient,
}

impl ArtistTopTracksSource {
    /// Create a new source for fetching artist top tracks
    pub fn new(spotify: SpotifyRadioClient) -> Self {
        Self { spotify }
    }
}

#[async_trait]
impl TrackSource for ArtistTopTracksSource {
    async fn get_tracks(&self, seed_artist_id: &str, _seed_track_id: &str, limit: u32) -> Result<Vec<RadioTrack>> {
        let guard = self.spotify.client().lock().await;
        
        // Fetch artist's top tracks
        let market = rspotify::model::Market::Country(rspotify::model::Country::UnitedStates);
        let top_tracks = guard
            .oauth
            .artist_top_tracks(
                rspotify::model::ArtistId::from_id(seed_artist_id)
                    .context("Invalid artist ID")?,
                Some(market),
            )
            .await
            .context("Failed to fetch artist top tracks")?;
        
        // Convert to RadioTrack
        let tracks: Vec<RadioTrack> = top_tracks
            .into_iter()
            .take(limit as usize)
            .map(|track| RadioTrack {
                uri: track.uri.to_string(),
                name: track.name,
                artist: track.artists.first()
                    .map(|a| a.name.clone())
                    .unwrap_or_default(),
                album: Some(track.album.name),
                duration_ms: Some(track.duration.num_milliseconds() as u32),
            })
            .collect();
        
        Ok(tracks)
    }
    
    fn source_name(&self) -> &'static str {
        "artist_top_tracks"
    }
}

/// Track source that fetches tracks from related artists
/// (Secondary source for radio - 30% of tracks)
pub struct RelatedArtistsSource {
    spotify: SpotifyRadioClient,
}

impl RelatedArtistsSource {
    /// Create a new source for fetching related artists' tracks
    pub fn new(spotify: SpotifyRadioClient) -> Self {
        Self { spotify }
    }
}

#[async_trait]
impl TrackSource for RelatedArtistsSource {
    async fn get_tracks(&self, seed_artist_id: &str, _seed_track_id: &str, limit: u32) -> Result<Vec<RadioTrack>> {
        let guard = self.spotify.client().lock().await;
        
        // Fetch related artists
        let related_artists = guard
            .oauth
            .artist_related_artists(
                rspotify::model::ArtistId::from_id(seed_artist_id)
                    .context("Invalid artist ID")?,
            )
            .await
            .context("Failed to fetch related artists")?;
        
        // Get top tracks from each related artist (limit to avoid too many API calls)
        let mut all_tracks = Vec::new();
        let market = rspotify::model::Market::Country(rspotify::model::Country::UnitedStates);
        
        for artist in related_artists.iter().take(5) { // Limit to 5 related artists
            if let Ok(top_tracks) = guard
                .oauth
                .artist_top_tracks(
                    artist.id.clone(),
                    Some(market),
                )
                .await
            {
                let tracks: Vec<RadioTrack> = top_tracks
                    .into_iter()
                    .take(2) // Take top 2 tracks from each related artist
                    .map(|track| RadioTrack {
                        uri: track.uri.to_string(),
                        name: track.name,
                        artist: track.artists.first()
                            .map(|a| a.name.clone())
                            .unwrap_or_default(),
                        album: Some(track.album.name),
                        duration_ms: Some(track.duration.num_milliseconds() as u32),
                    })
                    .collect();
                
                all_tracks.extend(tracks);
                
                if all_tracks.len() >= limit as usize {
                    break;
                }
            }
        }
        
        // Trim to limit
        all_tracks.truncate(limit as usize);
        
        Ok(all_tracks)
    }
    
    fn source_name(&self) -> &'static str {
        "related_artists"
    }
}

/// Alternative source using search API (fallback if artist endpoints fail)
pub struct SearchBasedSource {
    spotify: SpotifyRadioClient,
}

impl SearchBasedSource {
    /// Create a new source using search API
    pub fn new(spotify: SpotifyRadioClient) -> Self {
        Self { spotify }
    }
}

#[async_trait]
impl TrackSource for SearchBasedSource {
    async fn get_tracks(&self, _seed_artist_id: &str, seed_track_id: &str, limit: u32) -> Result<Vec<RadioTrack>> {
        // First get the seed track to find the artist
        let guard = self.spotify.client().lock().await;
        
        let seed_track = guard
            .oauth
            .track(
                rspotify::model::TrackId::from_id(seed_track_id)
                    .context("Invalid track ID")?,
                None,
            )
            .await
            .context("Failed to fetch seed track")?;
        
        // Get the primary artist name
        let artist_name = seed_track
            .artists
            .first()
            .map(|a| a.name.clone())
            .unwrap_or_default();
        
        // Search for tracks by this artist (max 10 per API limit as of Feb 2026)
        let query = format!("artist:{}", artist_name);
        let request_limit = (limit * 2).min(10);
        let search_result = guard
            .oauth
            .search(
                &query,
                rspotify::model::SearchType::Track,
                Some(rspotify::model::Market::FromToken),
                None,
                Some(request_limit),
                None,
            )
            .await
            .context("Search failed")?;
        
        let tracks = match search_result {
            rspotify::model::SearchResult::Tracks(page) => {
                page.items
                    .into_iter()
                    .filter(|track| {
                        // Exclude the seed track — id.to_string() returns URI,
                        // use id.id() for raw base62 ID comparison
                        track.id.as_ref()
                            .map(|id| id.id() != seed_track_id)
                            .unwrap_or(true)
                    })
                    .take(limit as usize)
                    .map(|track| RadioTrack {
                        uri: track.uri.to_string(),
                        name: track.name,
                        artist: track.artists.first()
                            .map(|a| a.name.clone())
                            .unwrap_or_default(),
                        album: track.album.map(|a| a.name),
                        duration_ms: Some(track.duration_ms),
                    })
                    .collect()
            }
            _ => Vec::new(),
        };
        
        Ok(tracks)
    }
    
    fn source_name(&self) -> &'static str {
        "search_based"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_artist_top_tracks_source_name() {
        // Mock client would be needed for full test
        // For now just verify the structure compiles
        let client = SpotifyRadioClient {
            client: Arc::new(Mutex::new(crate::api::SpotifyClient::new(&Default::default()).await.unwrap())),
        };
        let source = ArtistTopTracksSource::new(client);
        assert_eq!(source.source_name(), "artist_top_tracks");
    }
}
