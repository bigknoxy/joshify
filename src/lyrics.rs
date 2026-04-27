//! Lyrics fetching and display using LRCLIB
//!
//! Provides synced lyrics for currently playing tracks.
//! Uses lrclib.net API (free, no authentication required).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

/// LRCLIB API endpoint
const LRCLIB_API: &str = "https://lrclib.net/api";

/// Lyrics entry with timing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LyricsEntry {
    /// Timestamp in milliseconds
    pub timestamp_ms: u32,
    /// Lyrics line text
    pub text: String,
}

impl LyricsEntry {
    pub fn new(timestamp_ms: u32, text: String) -> Self {
        Self { timestamp_ms, text }
    }

    /// Format timestamp as [MM:SS.ms]
    pub fn formatted_timestamp(&self) -> String {
        let total_seconds = self.timestamp_ms / 1000;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        let ms = self.timestamp_ms % 1000;
        format!("[{:02}:{:02}.{:03}]", minutes, seconds, ms)
    }
}

/// Lyrics data for a track
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrackLyrics {
    /// Track title
    pub track_name: String,
    /// Artist name
    pub artist_name: String,
    /// Album name
    pub album_name: String,
    /// Duration in milliseconds
    pub duration_ms: u32,
    /// Synced lyrics lines
    pub synced_lyrics: Vec<LyricsEntry>,
    /// Plain lyrics (unsynced)
    pub plain_lyrics: Option<String>,
}

impl TrackLyrics {
    /// Create empty lyrics
    pub fn empty() -> Self {
        Self::default()
    }

    /// Check if lyrics are available
    pub fn has_lyrics(&self) -> bool {
        !self.synced_lyrics.is_empty() || self.plain_lyrics.is_some()
    }

    /// Get current lyrics line for playback position
    pub fn get_current_line(&self, position_ms: u32) -> Option<&LyricsEntry> {
        if self.synced_lyrics.is_empty() {
            return None;
        }

        // Find the line that should be displayed at this position
        let mut current = None;
        for line in &self.synced_lyrics {
            if line.timestamp_ms <= position_ms {
                current = Some(line);
            } else {
                break;
            }
        }
        current
    }

    /// Get line index for playback position
    pub fn get_line_index(&self, position_ms: u32) -> Option<usize> {
        self.synced_lyrics
            .iter()
            .enumerate()
            .filter(|(_, line)| line.timestamp_ms <= position_ms)
            .map(|(idx, _)| idx)
            .last()
    }

    /// Get lines around current position (for display)
    pub fn get_context_lines(&self, position_ms: u32, context: usize) -> Vec<LyricsEntry> {
        let current_idx = self.get_line_index(position_ms).unwrap_or(0);
        let start = current_idx.saturating_sub(context);
        let end = (current_idx + context + 1).min(self.synced_lyrics.len());
        self.synced_lyrics[start..end].to_vec()
    }
}

/// LRCLIB API client
pub struct LyricsClient {
    http_client: reqwest::Client,
}

impl LyricsClient {
    /// Create new lyrics client
    pub fn new() -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { http_client })
    }

    /// Fetch lyrics for a track
    pub async fn fetch_lyrics(
        &self,
        track_name: &str,
        artist_name: &str,
        album_name: &str,
        duration_ms: Option<u32>,
    ) -> Result<TrackLyrics> {
        let url = format!("{}/get", LRCLIB_API);
        
        debug!("Fetching lyrics from: {} for {} - {}", url, artist_name, track_name);

        let response = self
            .http_client
            .get(&url)
            .query(&[
                ("track_name", track_name),
                ("artist_name", artist_name),
                ("album_name", album_name),
            ])
            .send()
            .await
            .context("Failed to fetch lyrics from LRCLIB")?;

        if response.status().is_success() {
            let lrclib_response: LrclibResponse = response
                .json()
                .await
                .context("Failed to parse LRCLIB response")?;

            let lyrics = self.parse_lrclib_response(lrclib_response)?;
            info!("Successfully fetched lyrics for {} - {}", artist_name, track_name);
            Ok(lyrics)
        } else if response.status() == reqwest::StatusCode::NOT_FOUND {
            warn!("Lyrics not found for {} - {}", artist_name, track_name);
            Ok(TrackLyrics::empty())
        } else {
            anyhow::bail!("LRCLIB API error: {}", response.status())
        }
    }

    /// Search for lyrics
    pub async fn search_lyrics(&self, query: &str) -> Result<Vec<TrackLyrics>> {
        let url = format!("{}/search", LRCLIB_API);
        
        debug!("Searching lyrics with query: {}", query);

        let response = self
            .http_client
            .get(&url)
            .query(&[("q", query)])
            .send()
            .await
            .context("Failed to search lyrics")?;

        if response.status().is_success() {
            let search_results: Vec<LrclibResponse> = response
                .json()
                .await
                .context("Failed to parse search results")?;

            let lyrics_list: Vec<TrackLyrics> = search_results
                .into_iter()
                .filter_map(|resp| self.parse_lrclib_response(resp).ok())
                .collect();

            info!("Found {} lyric results for query: {}", lyrics_list.len(), query);
            Ok(lyrics_list)
        } else {
            anyhow::bail!("LRCLIB search error: {}", response.status())
        }
    }

    /// Parse LRCLIB API response
    fn parse_lrclib_response(&self, response: LrclibResponse) -> Result<TrackLyrics> {
        let synced_lyrics = if let Some(synced) = response.syncedLyrics {
            self.parse_synced_lyrics(&synced)?
        } else {
            vec![]
        };

        Ok(TrackLyrics {
            track_name: response.trackName,
            artist_name: response.artistName,
            album_name: response.albumName,
            duration_ms: (response.duration * 1000.0) as u32,
            synced_lyrics,
            plain_lyrics: response.plainLyrics,
        })
    }

    /// Parse synced lyrics format (LRC format)
    /// Format: [MM:SS.ms] Lyrics text
    fn parse_synced_lyrics(&self, lyrics: &str) -> Result<Vec<LyricsEntry>> {
        let mut entries = Vec::new();

        for line in lyrics.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Parse [MM:SS.ms] or [MM:SS.mmm] format
            if let Some(end_bracket) = line.find(']') {
                let timestamp_str = &line[1..end_bracket];
                let text = line[end_bracket + 1..].trim().to_string();

                if let Some(timestamp_ms) = self.parse_timestamp(timestamp_str) {
                    entries.push(LyricsEntry::new(timestamp_ms, text));
                }
            }
        }

        // Sort by timestamp
        entries.sort_by_key(|e| e.timestamp_ms);

        Ok(entries)
    }

    /// Parse timestamp string to milliseconds
    /// Supports: MM:SS.ms, MM:SS.mmm, SS.ms
    fn parse_timestamp(&self, ts: &str) -> Option<u32> {
        // Try MM:SS.ms format
        if let Some(colon_pos) = ts.find(':') {
            let minutes: u32 = ts[..colon_pos].parse().ok()?;
            let rest = &ts[colon_pos + 1..];

            if let Some(dot_pos) = rest.find('.') {
                let seconds: u32 = rest[..dot_pos].parse().ok()?;
                let ms_str = &rest[dot_pos + 1..];
                let ms: u32 = ms_str.parse().ok()?;
                // Normalize ms to 3 digits
                let ms_normalized = if ms_str.len() == 2 { ms * 10 } else { ms };
                Some(minutes * 60 * 1000 + seconds * 1000 + ms_normalized)
            } else {
                let seconds: u32 = rest.parse().ok()?;
                Some(minutes * 60 * 1000 + seconds * 1000)
            }
        } else {
            // Try SS.ms format
            if let Some(dot_pos) = ts.find('.') {
                let seconds: u32 = ts[..dot_pos].parse().ok()?;
                let ms: u32 = ts[dot_pos + 1..].parse().ok()?;
                Some(seconds * 1000 + ms)
            } else {
                ts.parse::<u32>().ok().map(|s| s * 1000)
            }
        }
    }
}

impl Default for LyricsClient {
    fn default() -> Self {
        Self::new().expect("Failed to create LyricsClient")
    }
}

/// LRCLIB API response structure
#[derive(Debug, Clone, Deserialize)]
struct LrclibResponse {
    #[serde(rename = "id")]
    _id: i64,
    #[serde(rename = "trackName")]
    trackName: String,
    #[serde(rename = "artistName")]
    artistName: String,
    #[serde(rename = "albumName")]
    albumName: String,
    #[serde(rename = "duration")]
    duration: f32,
    #[serde(rename = "instrumental")]
    _instrumental: bool,
    #[serde(rename = "plainLyrics")]
    plainLyrics: Option<String>,
    #[serde(rename = "syncedLyrics")]
    syncedLyrics: Option<String>,
}

/// Lyrics display configuration
#[derive(Debug, Clone)]
pub struct LyricsDisplayConfig {
    /// Number of context lines to show
    pub context_lines: usize,
    /// Highlight current line
    pub highlight_current: bool,
    /// Show timestamps
    pub show_timestamps: bool,
}

impl Default for LyricsDisplayConfig {
    fn default() -> Self {
        Self {
            context_lines: 3,
            highlight_current: true,
            show_timestamps: false,
        }
    }
}

/// Render lyrics for display
pub fn render_lyrics(
    lyrics: &TrackLyrics,
    position_ms: u32,
    config: &LyricsDisplayConfig,
) -> Vec<String> {
    let mut output = Vec::new();

    if !lyrics.has_lyrics() {
        output.push("No lyrics available".to_string());
        return output;
    }

    let context = lyrics.get_context_lines(position_ms, config.context_lines);
    let current_idx = lyrics.get_line_index(position_ms);

    for (i, entry) in context.iter().enumerate() {
        let is_current = current_idx.map(|idx| {
            let start = idx.saturating_sub(config.context_lines);
            i == idx - start
        }).unwrap_or(false);

        let line = if config.show_timestamps {
            format!("{} {}", entry.formatted_timestamp(), entry.text)
        } else {
            entry.text.clone()
        };

        if is_current && config.highlight_current {
            output.push(format!("> {}", line));
        } else {
            output.push(format!("  {}", line));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lyrics_entry() {
        let entry = LyricsEntry::new(60000, "Test lyric".to_string());
        assert_eq!(entry.timestamp_ms, 60000);
        assert_eq!(entry.text, "Test lyric");
        assert_eq!(entry.formatted_timestamp(), "[01:00.000]");
    }

    #[test]
    fn test_track_lyrics_empty() {
        let lyrics = TrackLyrics::empty();
        assert!(!lyrics.has_lyrics());
        assert!(lyrics.get_current_line(0).is_none());
    }

    #[test]
    fn test_track_lyrics_with_synced() {
        let lyrics = TrackLyrics {
            track_name: "Test".to_string(),
            artist_name: "Artist".to_string(),
            album_name: "Album".to_string(),
            duration_ms: 180000,
            synced_lyrics: vec![
                LyricsEntry::new(0, "Line 1".to_string()),
                LyricsEntry::new(5000, "Line 2".to_string()),
                LyricsEntry::new(10000, "Line 3".to_string()),
            ],
            plain_lyrics: None,
        };

        assert!(lyrics.has_lyrics());
        assert_eq!(lyrics.get_current_line(0).map(|l| l.text.clone()), Some("Line 1".to_string()));
        assert_eq!(lyrics.get_current_line(6000).map(|l| l.text.clone()), Some("Line 2".to_string()));
        assert_eq!(lyrics.get_line_index(6000), Some(1));
    }

    #[test]
    fn test_get_context_lines() {
        let lyrics = TrackLyrics {
            track_name: "Test".to_string(),
            artist_name: "Artist".to_string(),
            album_name: "Album".to_string(),
            duration_ms: 60000,
            synced_lyrics: (0..10)
                .map(|i| LyricsEntry::new(i * 5000, format!("Line {}", i)))
                .collect(),
            plain_lyrics: None,
        };

        let context = lyrics.get_context_lines(25000, 2);
        assert_eq!(context.len(), 5); // 2 before, current, 2 after
        assert_eq!(context[2].text, "Line 5"); // Current line
    }

    #[test]
    fn test_lyrics_client_parse_timestamp() {
        let client = LyricsClient::new().unwrap();
        
        assert_eq!(client.parse_timestamp("01:30.500"), Some(90500));
        assert_eq!(client.parse_timestamp("00:45.250"), Some(45250));
        assert_eq!(client.parse_timestamp("02:00"), Some(120000));
        assert_eq!(client.parse_timestamp("30.500"), Some(30500));
    }

    #[test]
    fn test_lyrics_client_parse_synced_lyrics() {
        let client = LyricsClient::new().unwrap();
        let lyrics_text = r#"[00:00.00] First line
[00:05.50] Second line
[00:10.00] Third line"#;

        let entries = client.parse_synced_lyrics(lyrics_text).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].timestamp_ms, 0);
        assert_eq!(entries[0].text, "First line");
        assert_eq!(entries[1].timestamp_ms, 5500);
        assert_eq!(entries[2].timestamp_ms, 10000);
    }

    #[test]
    fn test_render_lyrics() {
        let lyrics = TrackLyrics {
            track_name: "Test".to_string(),
            artist_name: "Artist".to_string(),
            album_name: "Album".to_string(),
            duration_ms: 30000,
            synced_lyrics: vec![
                LyricsEntry::new(0, "Line 1".to_string()),
                LyricsEntry::new(5000, "Line 2".to_string()),
                LyricsEntry::new(10000, "Line 3".to_string()),
            ],
            plain_lyrics: None,
        };

        let config = LyricsDisplayConfig {
            context_lines: 1,
            highlight_current: true,
            show_timestamps: false,
        };

        let output = render_lyrics(&lyrics, 5000, &config);
        assert_eq!(output.len(), 3);
        assert!(output[1].starts_with(">")); // Current line
    }

    #[test]
    fn test_render_lyrics_empty() {
        let lyrics = TrackLyrics::empty();
        let config = LyricsDisplayConfig::default();
        let output = render_lyrics(&lyrics, 0, &config);
        assert_eq!(output.len(), 1);
        assert_eq!(output[0], "No lyrics available");
    }

    #[test]
    fn test_lyrics_display_config_default() {
        let config = LyricsDisplayConfig::default();
        assert_eq!(config.context_lines, 3);
        assert!(config.highlight_current);
        assert!(!config.show_timestamps);
    }

    #[test]
    fn test_track_lyrics_with_plain_only() {
        let lyrics = TrackLyrics {
            track_name: "Test".to_string(),
            artist_name: "Artist".to_string(),
            album_name: "Album".to_string(),
            duration_ms: 180000,
            synced_lyrics: vec![],
            plain_lyrics: Some("Plain lyrics text".to_string()),
        };

        assert!(lyrics.has_lyrics());
    }
}
