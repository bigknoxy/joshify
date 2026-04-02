//! Library, playlists, and search methods

use anyhow::{Context, Result};
use rspotify::clients::{BaseClient, OAuthClient};

use super::SpotifyClient;

impl SpotifyClient {
    /// Get user's liked tracks
    pub async fn current_user_saved_tracks(
        &self,
        limit: u32,
    ) -> Result<Vec<rspotify::model::SavedTrack>> {
        let result = self
            .oauth
            .current_user_saved_tracks_manual(None, None, None)
            .await
            .context("Failed to get liked tracks")?;
        Ok(result.items.into_iter().take(limit as usize).collect())
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
        eprintln!("DEBUG: Loading playlist {}", playlist_id);
        let pid =
            rspotify::model::PlaylistId::from_id(playlist_id).context("Invalid playlist ID")?;
        let result = self
            .oauth
            .playlist_items_manual(pid, None, None, None, None)
            .await;
        
        match result {
            Ok(r) => {
                eprintln!("DEBUG: Got {} playlist items", r.items.len());
                Ok(r.items)
            }
            Err(e) => {
                eprintln!("DEBUG: Playlist items error: {:?}", e);
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
        let result = self
            .oauth
            .search(
                query,
                rspotify::model::SearchType::Track,
                None,
                None,
                None,
                None,
            )
            .await
            .context("Search failed")?;

        match result {
            rspotify::model::SearchResult::Tracks(page) => {
                Ok(page.items.into_iter().take(track_limit as usize).collect())
            }
            _ => Ok(vec![]),
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
