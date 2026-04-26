//! Home dashboard state management
//!
//! Tracks recently played tracks and unfinished contexts for "Jump Back In"

use chrono::{DateTime, Utc};
use std::time::Instant;

/// Home dashboard state
#[derive(Debug, Clone)]
pub struct HomeState {
    /// Recently played tracks (last 20)
    pub recently_played: Vec<RecentlyPlayedItem>,
    /// Items to "jump back in" to (unfinished contexts)
    pub jump_back_in: Vec<ContinueContext>,
    /// Whether data is loading
    pub is_loading: bool,
    /// Last successful fetch timestamp
    pub last_updated: Option<Instant>,
}

impl Default for HomeState {
    fn default() -> Self {
        Self {
            recently_played: Vec::new(),
            jump_back_in: Vec::new(),
            is_loading: false,
            last_updated: None,
        }
    }
}

impl HomeState {
    /// Create new empty home state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if data is stale (older than 5 minutes)
    pub fn is_stale(&self) -> bool {
        match self.last_updated {
            None => true,
            Some(last) => last.elapsed().as_secs() > 300, // 5 minutes
        }
    }

    /// Mark as loading
    pub fn set_loading(&mut self) {
        self.is_loading = true;
    }

    /// Update with new data
    pub fn update(&mut self, recently_played: Vec<RecentlyPlayedItem>, jump_back_in: Vec<ContinueContext>) {
        self.recently_played = recently_played;
        self.jump_back_in = jump_back_in;
        self.is_loading = false;
        self.last_updated = Some(Instant::now());
    }
}

/// A recently played track
#[derive(Debug, Clone, PartialEq)]
pub struct RecentlyPlayedItem {
    /// Track information
    pub track: TrackSummary,
    /// When it was played
    pub played_at: DateTime<Utc>,
    /// Context it was played from (album, playlist, etc.)
    pub context: Option<PlayContext>,
}

/// Minimal track info for recently played
#[derive(Debug, Clone, PartialEq)]
pub struct TrackSummary {
    pub name: String,
    pub artist: String,
    pub uri: String,
    pub duration_ms: u32,
}

/// Context where a track was played from
#[derive(Debug, Clone, PartialEq)]
pub struct PlayContext {
    pub context_type: ContextType,
    pub id: String,
    pub name: String,
}

/// Type of playback context
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContextType {
    Album,
    Playlist,
    Artist, // Radio
}

/// An item that can be "jumped back in" to
#[derive(Debug, Clone, PartialEq)]
pub struct ContinueContext {
    /// What type of context
    pub context_type: ContextType,
    /// Spotify ID
    pub id: String,
    /// Display name
    pub name: String,
    /// Progress percentage (0-100)
    pub progress_percent: u32,
    /// When it was last played
    pub last_played: DateTime<Utc>,
    /// Total number of tracks
    pub total_tracks: u32,
    /// Number of tracks completed (approximately)
    pub completed_tracks: u32,
}

impl ContinueContext {
    /// Calculate progress percentage from track counts
    pub fn calculate_progress(completed: u32, total: u32) -> u32 {
        if total == 0 {
            return 0;
        }
        (completed * 100 / total).min(100)
    }

    /// Format progress for display (e.g., "67%" or "12 of 20")
    pub fn format_progress(&self) -> String {
        if self.progress_percent > 0 {
            format!("{}%", self.progress_percent)
        } else {
            format!("{} of {}", self.completed_tracks, self.total_tracks)
        }
    }
}

/// Calculate "Jump Back In" items from recently played tracks
/// 
/// Groups recently played tracks by context and identifies
/// contexts that are unfinished (not all tracks played)
pub fn calculate_jump_back_in(
    recent_tracks: &[RecentlyPlayedItem],
    _saved_albums: Option<&[crate::state::app_state::AlbumListItem]>,
    _saved_playlists: Option<&[crate::state::app_state::PlaylistListItem]>,
) -> Vec<ContinueContext> {
    use std::collections::HashMap;

    let mut contexts: HashMap<String, (PlayContext, Vec<&RecentlyPlayedItem>)> = HashMap::new();

    // Group tracks by context
    for item in recent_tracks {
        if let Some(ref ctx) = item.context {
            let key = format!("{:?}:{}", ctx.context_type, ctx.id);
            contexts
                .entry(key)
                .or_insert_with(|| (ctx.clone(), Vec::new()))
                .1
                .push(item);
        }
    }

    let mut result: Vec<ContinueContext> = contexts
        .into_iter()
        .filter_map(|(_, (ctx, tracks))| {
            // Skip if only 1-2 tracks played (likely just browsing)
            if tracks.len() < 2 {
                return None;
            }

            // Calculate progress
            // For MVP, we estimate based on track count vs total
            // In future, we could get actual album/playlist track count
            let completed = tracks.len() as u32;
            let total = completed + 10; // Estimate - will be refined
            let progress = ContinueContext::calculate_progress(completed, total);

            // Only show if between 10% and 90% complete
            if progress < 10 || progress > 90 {
                return None;
            }

            let last_played = tracks
                .iter()
                .map(|t| t.played_at)
                .max()
                .unwrap_or_else(Utc::now);

            Some(ContinueContext {
                context_type: ctx.context_type,
                id: ctx.id.clone(),
                name: ctx.name.clone(),
                progress_percent: progress,
                last_played,
                total_tracks: total,
                completed_tracks: completed,
            })
        })
        .collect();

    // Sort by most recently played
    result.sort_by(|a, b| b.last_played.cmp(&a.last_played));

    // Limit to 6 items
    result.truncate(6);

    result
}

/// Format a timestamp as relative time (e.g., "2m ago", "3h ago")
pub fn format_relative_time(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now - dt;

    let minutes = diff.num_minutes();
    let hours = diff.num_hours();
    let days = diff.num_days();

    if minutes < 1 {
        "just now".to_string()
    } else if minutes < 60 {
        format!("{}m ago", minutes)
    } else if hours < 24 {
        format!("{}h ago", hours)
    } else if days < 7 {
        format!("{}d ago", days)
    } else {
        dt.format("%b %d").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_home_state_stale() {
        let mut state = HomeState::new();
        assert!(state.is_stale()); // Never updated = stale

        state.last_updated = Some(Instant::now());
        assert!(!state.is_stale()); // Just updated = not stale

        // Manually set last_updated to 6 minutes ago
        // We can't easily manipulate Instant, so we'll test the update logic instead
        state.last_updated = None;
        assert!(state.is_stale());
    }

    #[test]
    fn test_home_state_update() {
        let mut state = HomeState::new();
        state.set_loading();
        assert!(state.is_loading);

        let recent = vec![RecentlyPlayedItem {
            track: TrackSummary {
                name: "Test Track".to_string(),
                artist: "Test Artist".to_string(),
                uri: "spotify:track:test".to_string(),
                duration_ms: 180000,
            },
            played_at: Utc::now(),
            context: None,
        }];

        state.update(recent, Vec::new());
        assert!(!state.is_loading);
        assert_eq!(state.recently_played.len(), 1);
        assert!(state.last_updated.is_some());
    }

    #[test]
    fn test_continue_context_calculate_progress() {
        assert_eq!(ContinueContext::calculate_progress(0, 10), 0);
        assert_eq!(ContinueContext::calculate_progress(5, 10), 50);
        assert_eq!(ContinueContext::calculate_progress(10, 10), 100);
        assert_eq!(ContinueContext::calculate_progress(0, 0), 0); // Edge case
        assert_eq!(ContinueContext::calculate_progress(100, 100), 100);
    }

    #[test]
    fn test_continue_context_format_progress() {
        let ctx = ContinueContext {
            context_type: ContextType::Album,
            id: "test".to_string(),
            name: "Test".to_string(),
            progress_percent: 67,
            last_played: Utc::now(),
            total_tracks: 10,
            completed_tracks: 7,
        };
        assert_eq!(ctx.format_progress(), "67%");

        let ctx2 = ContinueContext {
            context_type: ContextType::Playlist,
            id: "test2".to_string(),
            name: "Test 2".to_string(),
            progress_percent: 0,
            last_played: Utc::now(),
            total_tracks: 20,
            completed_tracks: 0,
        };
        assert_eq!(ctx2.format_progress(), "0 of 20");
    }

    #[test]
    fn test_format_relative_time() {
        let now = Utc::now();

        assert_eq!(format_relative_time(now), "just now");
        assert_eq!(format_relative_time(now - Duration::minutes(2)), "2m ago");
        assert_eq!(format_relative_time(now - Duration::hours(3)), "3h ago");
        assert_eq!(format_relative_time(now - Duration::days(2)), "2d ago");
        // Test week+ formatting (just verify it doesn't panic)
        let _ = format_relative_time(now - Duration::days(10));
    }

    #[test]
    fn test_calculate_jump_back_in_empty() {
        let result = calculate_jump_back_in(&[], None, None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_calculate_jump_back_in_single_track_no_context() {
        let tracks = vec![RecentlyPlayedItem {
            track: TrackSummary {
                name: "Track".to_string(),
                artist: "Artist".to_string(),
                uri: "spotify:track:1".to_string(),
                duration_ms: 180000,
            },
            played_at: Utc::now(),
            context: None,
        }];
        let result = calculate_jump_back_in(&tracks, None, None);
        assert!(result.is_empty()); // No context = no jump back in
    }

    #[test]
    fn test_calculate_jump_back_in_insufficient_tracks() {
        let now = Utc::now();
        let tracks = vec![
            RecentlyPlayedItem {
                track: TrackSummary {
                    name: "Track 1".to_string(),
                    artist: "Artist".to_string(),
                    uri: "spotify:track:1".to_string(),
                    duration_ms: 180000,
                },
                played_at: now,
                context: Some(PlayContext {
                    context_type: ContextType::Album,
                    id: "album1".to_string(),
                    name: "Album One".to_string(),
                }),
            },
        ];
        let result = calculate_jump_back_in(&tracks, None, None);
        assert!(result.is_empty()); // Only 1 track = not enough
    }

    #[test]
    fn test_calculate_jump_back_in_groups_by_context() {
        let now = Utc::now();
        let tracks = vec![
            RecentlyPlayedItem {
                track: TrackSummary {
                    name: "Track 1".to_string(),
                    artist: "Artist".to_string(),
                    uri: "spotify:track:1".to_string(),
                    duration_ms: 180000,
                },
                played_at: now,
                context: Some(PlayContext {
                    context_type: ContextType::Album,
                    id: "album1".to_string(),
                    name: "Album One".to_string(),
                }),
            },
            RecentlyPlayedItem {
                track: TrackSummary {
                    name: "Track 2".to_string(),
                    artist: "Artist".to_string(),
                    uri: "spotify:track:2".to_string(),
                    duration_ms: 180000,
                },
                played_at: now - Duration::minutes(5),
                context: Some(PlayContext {
                    context_type: ContextType::Album,
                    id: "album1".to_string(),
                    name: "Album One".to_string(),
                }),
            },
            RecentlyPlayedItem {
                track: TrackSummary {
                    name: "Track 3".to_string(),
                    artist: "Artist".to_string(),
                    uri: "spotify:track:3".to_string(),
                    duration_ms: 180000,
                },
                played_at: now - Duration::minutes(10),
                context: Some(PlayContext {
                    context_type: ContextType::Playlist,
                    id: "playlist1".to_string(),
                    name: "My Playlist".to_string(),
                }),
            },
            RecentlyPlayedItem {
                track: TrackSummary {
                    name: "Track 4".to_string(),
                    artist: "Artist".to_string(),
                    uri: "spotify:track:4".to_string(),
                    duration_ms: 180000,
                },
                played_at: now - Duration::minutes(15),
                context: Some(PlayContext {
                    context_type: ContextType::Playlist,
                    id: "playlist1".to_string(),
                    name: "My Playlist".to_string(),
                }),
            },
            RecentlyPlayedItem {
                track: TrackSummary {
                    name: "Track 5".to_string(),
                    artist: "Artist".to_string(),
                    uri: "spotify:track:5".to_string(),
                    duration_ms: 180000,
                },
                played_at: now - Duration::minutes(20),
                context: Some(PlayContext {
                    context_type: ContextType::Playlist,
                    id: "playlist1".to_string(),
                    name: "My Playlist".to_string(),
                }),
            },
        ];
        let result = calculate_jump_back_in(&tracks, None, None);
        
        // Should have 2 contexts: album1 and playlist1
        // Album: 2 tracks (would estimate 20% progress - filtered out for < 10%)
        // Playlist: 3 tracks (would estimate 23% progress - included)
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "playlist1");
    }
}
