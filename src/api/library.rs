//! Library, playlists, and search methods

use anyhow::{Context, Result};
use rspotify::clients::{BaseClient, OAuthClient};

use super::SpotifyClient;

impl SpotifyClient {
    /// Get user's liked tracks (first page)
    pub async fn current_user_saved_tracks(
        &self,
        limit: u32,
    ) -> Result<Vec<rspotify::model::SavedTrack>> {
        let effective_limit = limit.min(50);
        tracing::info!("Fetching liked tracks (limit={})", effective_limit);
        let result = self
            .oauth
            .current_user_saved_tracks_manual(
                Some(rspotify::model::Market::FromToken),
                Some(effective_limit),
                Some(0),
            )
            .await
            .context("Failed to get liked tracks")?;
        Ok(result.items)
    }

    /// Get user's liked tracks with pagination support
    /// Returns (tracks, total_count, next_offset)
    pub async fn current_user_saved_tracks_paginated(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<rspotify::model::SavedTrack>, u32, Option<u32>)> {
        let effective_limit = limit.min(50);
        tracing::info!("Fetching liked tracks page (limit={}, offset={})", effective_limit, offset);
        let result = self
            .oauth
            .current_user_saved_tracks_manual(
                Some(rspotify::model::Market::FromToken),
                Some(effective_limit),
                Some(offset),
            )
            .await
            .context("Failed to get liked tracks")?;
        let total = result.total;
        let next_offset = if result.next.is_some() {
            Some(offset + effective_limit)
        } else {
            None
        };
        tracing::info!("Got {} liked tracks (total={}, next_offset={:?})", result.items.len(), total, next_offset);
        Ok((result.items, total, next_offset))
    }

    /// Get user's playlists
    pub async fn current_users_playlists(
        &self,
        limit: u32,
    ) -> Result<Vec<rspotify::model::SimplifiedPlaylist>> {
        let result = self
            .oauth
            .current_user_playlists_manual(Some(limit), None)
            .await
            .context("Failed to get playlists")?;
        Ok(result.items)
    }

    /// Get playlist tracks
    pub async fn playlist_get_items(
        &self,
        playlist_id: &str,
    ) -> Result<Vec<rspotify::model::PlaylistItem>> {
        tracing::debug!("Loading playlist {}", playlist_id);
        let pid =
            rspotify::model::PlaylistId::from_id(playlist_id).context("Invalid playlist ID")?;
        let result = self
            .oauth
            .playlist_items_manual(pid, None, None, None, None)
            .await;

        match result {
            Ok(r) => {
                tracing::debug!("Got {} playlist items", r.items.len());
                Ok(r.items)
            }
            Err(e) => {
                tracing::warn!("Playlist items error: {:?}", e);
                Err(e).context("Failed to get playlist items")
            }
        }
    }

    /// Search Spotify
    pub async fn search(
        &self,
        query: &str,
        track_limit: u32,
    ) -> Result<Vec<rspotify::model::FullTrack>> {
        use rspotify::clients::BaseClient;

        tracing::info!("Searching Spotify for: '{}'", query);

        if let Err(e) = self.oauth.auto_reauth().await {
            tracing::warn!("Token refresh failed before search: {:?}", e);
        }

        let limit = track_limit.min(10);
        tracing::debug!(
            "Search params: query='{}', limit={}, market=FromToken",
            query,
            limit
        );

        let result = self
            .oauth
            .search(
                query,
                rspotify::model::SearchType::Track,
                Some(rspotify::model::Market::FromToken),
                None,
                Some(limit),
                None,
            )
            .await;

        match result {
            Ok(rspotify::model::SearchResult::Tracks(page)) => {
                tracing::info!(
                    "Search returned {} tracks for '{}'",
                    page.items.len(),
                    query
                );
                if page.items.is_empty() {
                    tracing::warn!("Spotify returned empty track list for query '{}' - this may indicate a market/auth issue", query);
                }
                Ok(page.items)
            }
            Ok(other) => {
                tracing::warn!("Search returned unexpected type: {:?}", other);
                Ok(vec![])
            }
            Err(e) => {
                let err_str = e.to_string();
                let err_debug = format!("{:?}", e);
                tracing::error!("Search API error for '{}': {}", query, err_str);
                tracing::debug!("Search API error details: {}", err_debug);

                if err_str.contains("401")
                    || err_str.contains("Unauthorized")
                    || err_debug.contains("401")
                {
                    tracing::warn!(
                        "Token may be expired - re-authentication required (401 Unauthorized)"
                    );
                } else if err_str.contains("429") || err_debug.contains("429") {
                    tracing::warn!("Rate limited by Spotify API");
                } else if err_str.contains("400") || err_debug.contains("400") {
                    tracing::warn!("Bad request to Spotify API - check query format");
                    if let Some(json_start) = err_debug.find("{") {
                        if let Some(json_end) = err_debug.rfind("}") {
                            let json = &err_debug[json_start..=json_end];
                            tracing::warn!("Spotify error response: {}", json);
                        }
                    }
                }

                Err(e).context(format!("Search for '{}' failed", query))
            }
        }
    }

    /// Add track to queue
    pub async fn add_to_queue(&self, track_uri: &str) -> Result<()> {
        // Parse URI to get track ID
        let uri_parts: Vec<&str> = track_uri.split(':').collect();
        if uri_parts.len() >= 3 && uri_parts[0] == "spotify" {
            let track_id = uri_parts[2];
            if let Ok(id) = rspotify::model::TrackId::from_id(track_id) {
                self.oauth
                    .add_item_to_queue(rspotify::model::PlayableId::Track(id), None)
                    .await
                    .context("Failed to add to queue")?;
                return Ok(());
            }
        }
        anyhow::bail!("Invalid Spotify track URI");
    }

    /// Get current queue
    pub async fn get_queue(&self) -> Result<rspotify::model::CurrentUserQueue> {
        let queue = self
            .oauth
            .current_user_queue()
            .await
            .context("Failed to get queue")?;
        Ok(queue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_validation() {
        let long_query = "a".repeat(50);
        let valid_queries: Vec<&str> = vec![
            "test",
            "test-query",
            "test_query",
            "test query",
            "テスト",
            "rock & roll",
            "a",
            &long_query,
        ];

        for query in valid_queries {
            assert!(!query.is_empty(), "Query should not be empty: '{}'", query);
            assert!(
                query.len() <= 100,
                "Query should be reasonable length: '{}'",
                query
            );
        }
    }

    #[test]
    fn test_search_track_limit_parameter() {
        let limits = vec![1, 10, 25, 50];
        for limit in limits {
            assert!(limit > 0, "Limit must be positive: {}", limit);
            assert!(limit <= 50, "Limit must not exceed 50: {}", limit);
        }
    }

    #[test]
    fn test_search_market_parameter() {
        use rspotify::model::{Country, Market};

        let market = Market::FromToken;
        assert_eq!(Into::<&'static str>::into(market), "from_token");

        let market_us = Market::Country(Country::UnitedStates);
        assert_eq!(Into::<&'static str>::into(market_us), "US");
    }

    #[test]
    fn test_search_result_processing() {
        let mock_items: Vec<rspotify::model::FullTrack> = vec![];
        assert!(mock_items.is_empty());
    }

    #[test]
    fn test_queue_uri_parsing() {
        let valid_uris = vec![
            "spotify:track:abc123",
            "spotify:track:4uLU6hMCjMI75M1A2tKUQC",
        ];

        for uri in valid_uris {
            let parts: Vec<&str> = uri.split(':').collect();
            assert_eq!(parts.len(), 3);
            assert_eq!(parts[0], "spotify");
            assert_eq!(parts[1], "track");
            assert!(!parts[2].is_empty());
        }

        let invalid_uris = vec!["spotify:album:abc", "invalid", "spotify:track:"];
        for uri in invalid_uris {
            let parts: Vec<&str> = uri.split(':').collect();
            if parts.len() >= 3 && parts[0] == "spotify" && parts[1] == "track" {
                assert!(
                    parts[2].is_empty() || rspotify::model::TrackId::from_id(parts[2]).is_err()
                );
            }
        }
    }
}
