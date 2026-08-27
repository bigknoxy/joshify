//! Library, playlists, and search methods

use anyhow::{Context, Result};
use rspotify::clients::{BaseClient, OAuthClient};

use super::SpotifyClient;

/// Largest `limit` Spotify accepts for `GET /playlists/{id}/items`.
///
/// The Web API documents "Default: 20. Minimum: 1. Maximum: 50." for this
/// endpoint. Asking for more is not clamped - the whole request is rejected
/// with a 400, which is what turned every playlist into "Failed to get
/// playlist items" in 0.8.3.
pub const SPOTIFY_PLAYLIST_ITEMS_MAX_LIMIT: u32 = 50;

/// Largest `limit` Spotify accepts for `GET /albums/{id}/tracks`.
pub const SPOTIFY_ALBUM_TRACKS_MAX_LIMIT: u32 = 50;

/// Page size used when walking a playlist.
pub const PLAYLIST_ITEMS_PAGE: u32 = SPOTIFY_PLAYLIST_ITEMS_MAX_LIMIT;

/// Page size used when walking an album.
pub const ALBUM_TRACKS_PAGE: u32 = SPOTIFY_ALBUM_TRACKS_MAX_LIMIT;

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
        tracing::info!(
            "Fetching liked tracks page (limit={}, offset={})",
            effective_limit,
            offset
        );
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
        tracing::info!(
            "Got {} liked tracks (total={}, next_offset={:?})",
            result.items.len(),
            total,
            next_offset
        );
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
        // Page through the whole playlist. A single call returns Spotify's
        // default page of 20, so anything longer was silently truncated while
        // the header went on showing the playlist's real track count.
        const PAGE: u32 = PLAYLIST_ITEMS_PAGE;
        const MAX_ITEMS: usize = 5_000;
        let mut items = Vec::new();
        let mut offset = 0u32;

        loop {
            let page = self
                .oauth
                .playlist_items_manual(pid.clone(), None, None, Some(PAGE), Some(offset))
                .await;
            let page = match page {
                Ok(page) => page,
                Err(e) => {
                    tracing::warn!("Playlist items error at offset {}: {:?}", offset, e);
                    // Keep what we already have rather than losing the whole
                    // playlist to one bad page.
                    if items.is_empty() {
                        return Err(e).context("Failed to get playlist items");
                    }
                    break;
                }
            };

            let returned = page.items.len();
            items.extend(page.items);

            // Stop on a short page, on a missing `next`, or at the safety cap -
            // never rely on only one of these to terminate.
            if returned < PAGE as usize || page.next.is_none() || items.len() >= MAX_ITEMS {
                break;
            }
            offset += PAGE;
        }

        tracing::debug!("Got {} playlist items", items.len());
        Ok(items)
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

    /// Get recently played tracks
    pub async fn get_recently_played(
        &self,
        limit: u32,
    ) -> Result<Vec<rspotify::model::PlayHistory>> {
        let effective_limit = limit.min(50);
        tracing::info!(
            "Fetching recently played tracks (limit={})",
            effective_limit
        );
        let result = self
            .oauth
            .current_user_recently_played(
                Some(effective_limit),
                None, // after
            )
            .await
            .context("Failed to get recently played tracks")?;
        Ok(result.items)
    }

    /// Get user's saved albums
    pub async fn get_user_albums(&self, limit: u32) -> Result<Vec<rspotify::model::SavedAlbum>> {
        let effective_limit = limit.min(50);
        tracing::info!("Fetching user albums (limit={})", effective_limit);
        let result = self
            .oauth
            .current_user_saved_albums_manual(
                Some(rspotify::model::Market::FromToken),
                Some(effective_limit),
                Some(0),
            )
            .await
            .context("Failed to get user albums")?;
        Ok(result.items)
    }

    /// Get user's followed artists
    pub async fn get_user_artists(&self, limit: u32) -> Result<Vec<rspotify::model::FullArtist>> {
        let effective_limit = limit.min(50);
        tracing::info!("Fetching followed artists (limit={})", effective_limit);
        let result = self
            .oauth
            .current_user_followed_artists(
                None, // after
                Some(effective_limit),
            )
            .await
            .context("Failed to get followed artists")?;
        Ok(result.items)
    }

    /// Get user's top artists
    pub async fn get_top_artists(
        &self,
        limit: u32,
        time_range: &str,
    ) -> Result<Vec<rspotify::model::FullArtist>> {
        let effective_limit = limit.min(50);
        tracing::info!(
            "Fetching top artists (limit={}, time_range={})",
            effective_limit,
            time_range
        );

        // Map string time_range to rspotify TimeRange
        let range = match time_range {
            "short" => rspotify::model::TimeRange::ShortTerm,
            "medium" => rspotify::model::TimeRange::MediumTerm,
            "long" => rspotify::model::TimeRange::LongTerm,
            _ => rspotify::model::TimeRange::MediumTerm,
        };

        let result = self
            .oauth
            .current_user_top_artists_manual(Some(range), Some(effective_limit), Some(0))
            .await
            .context("Failed to get top artists")?;
        Ok(result.items)
    }

    /// Get user's top tracks
    pub async fn get_top_tracks(
        &self,
        limit: u32,
        time_range: &str,
    ) -> Result<Vec<rspotify::model::FullTrack>> {
        let effective_limit = limit.min(50);
        tracing::info!(
            "Fetching top tracks (limit={}, time_range={})",
            effective_limit,
            time_range
        );

        // Map string time_range to rspotify TimeRange
        let range = match time_range {
            "short" => rspotify::model::TimeRange::ShortTerm,
            "medium" => rspotify::model::TimeRange::MediumTerm,
            "long" => rspotify::model::TimeRange::LongTerm,
            _ => rspotify::model::TimeRange::MediumTerm,
        };

        let result = self
            .oauth
            .current_user_top_tracks_manual(Some(range), Some(effective_limit), Some(0))
            .await
            .context("Failed to get top tracks")?;
        Ok(result.items)
    }

    /// Get album tracks
    pub async fn get_album_tracks(
        &self,
        album_id: &str,
    ) -> Result<Vec<rspotify::model::SimplifiedTrack>> {
        tracing::debug!("Loading album tracks for {}", album_id);
        let aid = rspotify::model::AlbumId::from_id(album_id).context("Invalid album ID")?;
        // One page of 50 silently cut off longer albums and compilations, and
        // the album header then rewrote its track count to match.
        const PAGE: u32 = ALBUM_TRACKS_PAGE;
        const MAX_ITEMS: usize = 1_000;
        let mut items = Vec::new();
        let mut offset = 0u32;

        loop {
            let page = self
                .oauth
                .album_track_manual(
                    aid.clone(),
                    Some(rspotify::model::Market::FromToken),
                    Some(PAGE),
                    Some(offset),
                )
                .await
                .context("Failed to get album tracks")?;

            let returned = page.items.len();
            items.extend(page.items);

            if returned < PAGE as usize || page.next.is_none() || items.len() >= MAX_ITEMS {
                break;
            }
            offset += PAGE;
        }

        Ok(items)
    }

    /// Get artist top tracks
    /// Get artist's top tracks
    ///
    /// Note: The `artist_top_tracks` endpoint was deprecated by Spotify.
    /// This method now returns an empty vec as a placeholder.
    /// Use `get_related_artists` and `get_artist_albums` for discovery instead.
    #[allow(deprecated)]
    pub async fn get_artist_top_tracks(
        &self,
        _artist_id: &str,
    ) -> Result<Vec<rspotify::model::FullTrack>> {
        tracing::warn!("artist_top_tracks endpoint is deprecated by Spotify - returning empty");
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spotify rejects the whole request when `limit` exceeds the endpoint's
    /// documented maximum; it does not clamp. 0.8.3 paged playlists at 100 and
    /// every playlist came back "Failed to get playlist items". Pin the pages
    /// to the numbers in the Web API reference, not to each other.
    #[test]
    // The point is that these are constants: the assertion pins them to the
    // documented API limit so the next "page bigger" change fails here.
    #[allow(clippy::assertions_on_constants)]
    fn test_playlist_page_size_within_spotify_limit() {
        assert!(
            PLAYLIST_ITEMS_PAGE >= 1 && PLAYLIST_ITEMS_PAGE <= 50,
            "GET /playlists/{{id}}/items accepts limit 1..=50, got {}",
            PLAYLIST_ITEMS_PAGE
        );
    }

    #[test]
    // The point is that these are constants: the assertion pins them to the
    // documented API limit so the next "page bigger" change fails here.
    #[allow(clippy::assertions_on_constants)]
    fn test_album_page_size_within_spotify_limit() {
        assert!(
            ALBUM_TRACKS_PAGE >= 1 && ALBUM_TRACKS_PAGE <= 50,
            "GET /albums/{{id}}/tracks accepts limit 1..=50, got {}",
            ALBUM_TRACKS_PAGE
        );
    }

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

/// The real pagination code against a fake Spotify that enforces the real
/// limits. These are the tests that would have caught 0.8.3: with the page
/// size at 100 the fake answers 400 on page 0 and `playlist_get_items`
/// returns "Failed to get playlist items".
#[cfg(test)]
mod paging_against_fake_spotify {
    use super::super::fake_spotify::{track_id, Catalog, FakeSpotify};
    use super::*;

    const PLAYLIST: &str = "37i9dQZF1DXcBWIGoYBM5M";
    const ALBUM: &str = "4aawyAB9vmqN3uQ7FjRGTy";

    fn catalog(playlist_total: u32, album_total: u32) -> Catalog {
        Catalog {
            playlist_total,
            album_total,
            fail_from_offset: None,
        }
    }

    #[tokio::test]
    async fn playlist_pager_stays_within_the_limit_and_reads_every_page() {
        let fake = FakeSpotify::start(catalog(120, 0)).await;
        let client = SpotifyClient::for_tests(&fake.base_url);

        let items = client
            .playlist_get_items(PLAYLIST)
            .await
            .expect("a 120-track playlist must load");

        assert_eq!(items.len(), 120, "every page must be read, in order");
        let last_id = match items[119].item.as_ref() {
            Some(rspotify::model::PlayableItem::Track(t)) => t.id.as_ref().map(|i| i.to_string()),
            other => panic!("expected a track, got {other:?}"),
        };
        assert_eq!(
            last_id.as_deref(),
            Some(format!("spotify:track:{}", track_id(119)).as_str())
        );

        let hits = fake.hits();
        assert_eq!(hits.len(), 3, "120 items at 50 a page is three requests");
        for hit in &hits {
            let limit = hit
                .query
                .get("limit")
                .expect("limit must be sent explicitly");
            let limit: u32 = limit.parse().expect("numeric limit");
            assert!(
                (1..=SPOTIFY_PLAYLIST_ITEMS_MAX_LIMIT).contains(&limit),
                "Spotify rejects limit={limit} on {}",
                hit.path
            );
        }
        let offsets: Vec<u32> = hits
            .iter()
            .map(|h| h.query["offset"].parse().unwrap())
            .collect();
        assert_eq!(offsets, vec![0, 50, 100]);
    }

    #[tokio::test]
    async fn album_pager_stays_within_the_limit_and_reads_every_page() {
        let fake = FakeSpotify::start(catalog(0, 75)).await;
        let client = SpotifyClient::for_tests(&fake.base_url);

        let tracks = client
            .get_album_tracks(ALBUM)
            .await
            .expect("a 75-track album must load");

        assert_eq!(tracks.len(), 75);
        let hits = fake.hits();
        assert_eq!(hits.len(), 2);
        for hit in &hits {
            let limit: u32 = hit.query["limit"].parse().unwrap();
            assert!((1..=SPOTIFY_ALBUM_TRACKS_MAX_LIMIT).contains(&limit));
        }
    }

    #[tokio::test]
    async fn a_short_playlist_is_a_single_request() {
        let fake = FakeSpotify::start(catalog(7, 0)).await;
        let client = SpotifyClient::for_tests(&fake.base_url);
        let items = client.playlist_get_items(PLAYLIST).await.unwrap();
        assert_eq!(items.len(), 7);
        assert_eq!(fake.hits().len(), 1, "a short page must end the walk");
    }

    #[tokio::test]
    async fn a_failing_first_page_is_reported_not_swallowed() {
        let fake = FakeSpotify::start(Catalog {
            playlist_total: 120,
            album_total: 0,
            fail_from_offset: Some(0),
        })
        .await;
        let client = SpotifyClient::for_tests(&fake.base_url);
        let err = client
            .playlist_get_items(PLAYLIST)
            .await
            .expect_err("nothing was read, so the caller must hear about it");
        assert!(
            err.to_string().contains("Failed to get playlist items"),
            "{err:#}"
        );
    }

    #[tokio::test]
    async fn a_failing_later_page_keeps_what_was_already_read() {
        let fake = FakeSpotify::start(Catalog {
            playlist_total: 120,
            album_total: 0,
            fail_from_offset: Some(50),
        })
        .await;
        let client = SpotifyClient::for_tests(&fake.base_url);
        let items = client
            .playlist_get_items(PLAYLIST)
            .await
            .expect("the first page was good; keep it");
        assert_eq!(items.len(), 50);
    }
}
