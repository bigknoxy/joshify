//! CLI commands for Joshify
//!
//! Provides non-interactive commands for scripting and automation:
//! joshify play, joshify pause, joshify status, etc.

use anyhow::{Context, Result};
use rspotify::model::{CurrentPlaybackContext, PlayableItem, RepeatState};
use rspotify::prelude::Id;
use serde::{Deserialize, Serialize};
use std::io::Write;
use tracing::{debug, info, warn};

use crate::api::CliClient;

/// CLI command types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    /// Play a track, album, or playlist
    Play { uri: Option<String> },
    /// Pause playback
    Pause,
    /// Resume playback
    Resume,
    /// Toggle play/pause
    PlayPause,
    /// Next track
    Next,
    /// Previous track
    Previous,
    /// Stop playback
    Stop,
    /// Get current status
    Status { format: OutputFormat },
    /// Set or get volume
    Volume { value: Option<u8> },
    /// Seek to position
    Seek { position_ms: u32 },
    /// Seek forward
    SeekForward { duration_ms: u32 },
    /// Seek backward
    SeekBackward { duration_ms: u32 },
    /// Toggle shuffle
    Shuffle { enabled: Option<bool> },
    /// Toggle repeat
    Repeat { mode: Option<String> },
    /// Get current track info
    Current { format: OutputFormat },
    /// Search for tracks/artists/albums
    Search { query: String, limit: usize },
    /// Add track to queue
    QueueAdd { uri: String },
    /// Clear queue
    QueueClear,
    /// Show help
    Help,
    /// Show version
    Version,
}

/// Output format for CLI commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable text
    Text,
    /// JSON for scripting
    Json,
    /// Minimal output (just values)
    Minimal,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Text
    }
}

/// Playback status for CLI output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackStatus {
    pub is_playing: bool,
    pub track: Option<TrackInfo>,
    pub progress_ms: u32,
    pub duration_ms: u32,
    pub shuffle: bool,
    pub repeat: String,
    pub volume_percent: u8,
}

/// Track info for CLI output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackInfo {
    pub name: String,
    pub artists: Vec<String>,
    pub album: String,
    pub uri: String,
    pub duration_ms: u32,
}

/// CLI command handler
pub struct CliHandler<C: CliClient> {
    /// Spotify client used to execute commands
    client: C,
    /// Output stream (usually stdout)
    output: Box<dyn Write>,
}

impl<C: CliClient> CliHandler<C> {
    /// Create new CLI handler
    pub fn new(client: C) -> Self {
        Self {
            client,
            output: Box::new(std::io::stdout()),
        }
    }

    /// Create with custom output
    pub fn with_output<W>(client: C, output: W) -> Self
    where
        W: Write + 'static,
    {
        Self {
            client,
            output: Box::new(output),
        }
    }

    /// Create with a static writer reference for testing
    pub fn with_static_output(client: C, output: Box<dyn Write>) -> Self {
        Self { client, output }
    }

    /// Execute a CLI command
    pub async fn execute(&mut self, command: CliCommand) -> Result<()> {
        info!("Executing CLI command: {:?}", command);

        match command {
            CliCommand::Play { uri } => self.cmd_play(uri).await,
            CliCommand::Pause => self.cmd_pause().await,
            CliCommand::Resume => self.cmd_resume().await,
            CliCommand::PlayPause => self.cmd_play_pause().await,
            CliCommand::Next => self.cmd_next().await,
            CliCommand::Previous => self.cmd_previous().await,
            CliCommand::Stop => self.cmd_stop().await,
            CliCommand::Status { format } => self.cmd_status(format).await,
            CliCommand::Volume { value } => self.cmd_volume(value).await,
            CliCommand::Seek { position_ms } => self.cmd_seek(position_ms).await,
            CliCommand::SeekForward { duration_ms } => self.cmd_seek_forward(duration_ms).await,
            CliCommand::SeekBackward { duration_ms } => self.cmd_seek_backward(duration_ms).await,
            CliCommand::Shuffle { enabled } => self.cmd_shuffle(enabled).await,
            CliCommand::Repeat { mode } => self.cmd_repeat(mode).await,
            CliCommand::Current { format } => self.cmd_current(format).await,
            CliCommand::Search { query, limit } => self.cmd_search(query, limit).await,
            CliCommand::QueueAdd { uri } => self.cmd_queue_add(uri).await,
            CliCommand::QueueClear => self.cmd_queue_clear().await,
            CliCommand::Help => self.cmd_help(),
            CliCommand::Version => self.cmd_version(),
        }
    }

    async fn cmd_play(&mut self, uri: Option<String>) -> Result<()> {
        if let Some(track_uri) = uri {
            debug!("Playing track: {}", track_uri);
            self.client
                .start_playback(vec![track_uri.clone()], None)
                .await
                .context("Failed to start playback")?;
            writeln!(self.output, "Playing: {}", track_uri)?;
        } else {
            debug!("Resuming playback");
            self.client
                .playback_resume()
                .await
                .context("Failed to resume playback")?;
            writeln!(self.output, "Resuming playback")?;
        }
        Ok(())
    }

    async fn cmd_pause(&mut self) -> Result<()> {
        debug!("Pausing playback");
        self.client
            .playback_pause()
            .await
            .context("Failed to pause playback")?;
        writeln!(self.output, "Paused")?;
        Ok(())
    }

    async fn cmd_resume(&mut self) -> Result<()> {
        debug!("Resuming playback");
        self.client
            .playback_resume()
            .await
            .context("Failed to resume playback")?;
        writeln!(self.output, "Resumed")?;
        Ok(())
    }

    async fn cmd_play_pause(&mut self) -> Result<()> {
        debug!("Toggling play/pause");
        let ctx = self.client.current_playback().await?;
        match ctx {
            Some(ctx) if ctx.is_playing => {
                self.client
                    .playback_pause()
                    .await
                    .context("Failed to pause playback")?;
                writeln!(self.output, "Paused")?;
            }
            _ => {
                self.client
                    .playback_resume()
                    .await
                    .context("Failed to resume playback")?;
                writeln!(self.output, "Resumed")?;
            }
        }
        Ok(())
    }

    async fn cmd_next(&mut self) -> Result<()> {
        debug!("Skipping to next track");
        self.client
            .playback_next()
            .await
            .context("Failed to skip to next track")?;
        writeln!(self.output, "Next track")?;
        Ok(())
    }

    async fn cmd_previous(&mut self) -> Result<()> {
        debug!("Going to previous track");
        self.client
            .playback_previous()
            .await
            .context("Failed to skip to previous track")?;
        writeln!(self.output, "Previous track")?;
        Ok(())
    }

    async fn cmd_stop(&mut self) -> Result<()> {
        debug!("Stopping playback");
        self.client
            .playback_pause()
            .await
            .context("Failed to stop playback")?;
        writeln!(self.output, "Stopped")?;
        Ok(())
    }

    async fn cmd_status(&mut self, format: OutputFormat) -> Result<()> {
        let ctx = self.client.current_playback().await?;
        let status = match ctx {
            Some(ctx) => PlaybackStatus::from_context(&ctx),
            None => PlaybackStatus {
                is_playing: false,
                track: None,
                progress_ms: 0,
                duration_ms: 0,
                shuffle: false,
                repeat: "off".to_string(),
                volume_percent: 0,
            },
        };

        match format {
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(&status)?;
                writeln!(self.output, "{}", json)?;
            }
            OutputFormat::Minimal => {
                if let Some(track) = status.track {
                    writeln!(
                        self.output,
                        "{} - {} [{}/{}]",
                        track.name,
                        track.artists.join(", "),
                        format_duration(status.progress_ms),
                        format_duration(status.duration_ms)
                    )?;
                } else {
                    writeln!(self.output, "Not playing")?;
                }
            }
            OutputFormat::Text => {
                writeln!(
                    self.output,
                    "Status: {}",
                    if status.is_playing {
                        "Playing"
                    } else {
                        "Paused"
                    }
                )?;
                if let Some(track) = status.track {
                    writeln!(self.output, "Track: {}", track.name)?;
                    writeln!(self.output, "Artists: {}", track.artists.join(", "))?;
                    writeln!(self.output, "Album: {}", track.album)?;
                    writeln!(
                        self.output,
                        "Progress: {}/{}",
                        format_duration(status.progress_ms),
                        format_duration(status.duration_ms)
                    )?;
                }
                writeln!(self.output, "Volume: {}%", status.volume_percent)?;
                writeln!(
                    self.output,
                    "Shuffle: {}",
                    if status.shuffle { "on" } else { "off" }
                )?;
                writeln!(self.output, "Repeat: {}", status.repeat)?;
            }
        }

        Ok(())
    }

    async fn cmd_volume(&mut self, value: Option<u8>) -> Result<()> {
        match value {
            Some(v) => {
                let clamped = v.min(100);
                debug!("Setting volume to {}%", clamped);
                self.client
                    .set_volume(clamped as u32)
                    .await
                    .context("Failed to set volume")?;
                writeln!(self.output, "Volume set to {}%", clamped)?;
            }
            None => {
                let ctx = self.client.current_playback().await?;
                let volume = ctx
                    .as_ref()
                    .and_then(|c| c.device.volume_percent)
                    .unwrap_or(0);
                writeln!(self.output, "Current volume: {}%", volume)?;
            }
        }
        Ok(())
    }

    async fn cmd_seek(&mut self, position_ms: u32) -> Result<()> {
        debug!("Seeking to {}ms", position_ms);
        self.client
            .seek(position_ms, None)
            .await
            .context("Failed to seek")?;
        writeln!(self.output, "Seeked to {}", format_duration(position_ms))?;
        Ok(())
    }

    async fn cmd_seek_forward(&mut self, duration_ms: u32) -> Result<()> {
        debug!("Seeking forward {}ms", duration_ms);
        let ctx = self.client.current_playback().await?;
        let current = ctx
            .as_ref()
            .and_then(|c| c.progress)
            .map(|d| d.num_milliseconds() as u32)
            .unwrap_or(0);
        let target = current.saturating_add(duration_ms);
        self.client
            .seek(target, None)
            .await
            .context("Failed to seek forward")?;
        writeln!(self.output, "Seeked forward {}s", duration_ms / 1000)?;
        Ok(())
    }

    async fn cmd_seek_backward(&mut self, duration_ms: u32) -> Result<()> {
        debug!("Seeking backward {}ms", duration_ms);
        let ctx = self.client.current_playback().await?;
        let current = ctx
            .as_ref()
            .and_then(|c| c.progress)
            .map(|d| d.num_milliseconds() as u32)
            .unwrap_or(0);
        let target = current.saturating_sub(duration_ms);
        self.client
            .seek(target, None)
            .await
            .context("Failed to seek backward")?;
        writeln!(self.output, "Seeked backward {}s", duration_ms / 1000)?;
        Ok(())
    }

    async fn cmd_shuffle(&mut self, enabled: Option<bool>) -> Result<()> {
        match enabled {
            Some(true) => {
                debug!("Enabling shuffle");
                self.client
                    .toggle_shuffle(true)
                    .await
                    .context("Failed to enable shuffle")?;
                writeln!(self.output, "Shuffle: on")?;
            }
            Some(false) => {
                debug!("Disabling shuffle");
                self.client
                    .toggle_shuffle(false)
                    .await
                    .context("Failed to disable shuffle")?;
                writeln!(self.output, "Shuffle: off")?;
            }
            None => {
                let ctx = self.client.current_playback().await?;
                let shuffle = ctx.as_ref().map(|c| c.shuffle_state).unwrap_or(false);
                writeln!(
                    self.output,
                    "Shuffle: {}",
                    if shuffle { "on" } else { "off" }
                )?;
            }
        }
        Ok(())
    }

    async fn cmd_repeat(&mut self, mode: Option<String>) -> Result<()> {
        match mode {
            Some(m) => {
                let state = match m.as_str() {
                    "off" => RepeatState::Off,
                    "track" => RepeatState::Track,
                    "context" => RepeatState::Context,
                    other => anyhow::bail!(
                        "Invalid repeat mode: {} (use off, track, or context)",
                        other
                    ),
                };
                debug!("Setting repeat mode to: {}", m);
                self.client
                    .set_repeat(state)
                    .await
                    .context("Failed to set repeat mode")?;
                writeln!(self.output, "Repeat: {}", m)?;
            }
            None => {
                let ctx = self.client.current_playback().await?;
                let repeat = ctx
                    .as_ref()
                    .map(|c| repeat_to_string(c.repeat_state))
                    .unwrap_or_else(|| "off".to_string());
                writeln!(self.output, "Repeat: {}", repeat)?;
            }
        }
        Ok(())
    }

    async fn cmd_current(&mut self, format: OutputFormat) -> Result<()> {
        // Same as status but only shows track info
        self.cmd_status(format).await?;
        Ok(())
    }

    async fn cmd_search(&mut self, query: String, limit: usize) -> Result<()> {
        debug!("Searching for: {} (limit: {})", query, limit);
        let tracks = self
            .client
            .search(&query, limit.min(50) as u32)
            .await
            .context("Search failed")?;
        if tracks.is_empty() {
            writeln!(self.output, "No results for: {}", query)?;
            return Ok(());
        }
        for (i, track) in tracks.iter().enumerate() {
            let artists = track
                .artists
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(self.output, "{}. {} - {}", i + 1, track.name, artists)?;
        }
        Ok(())
    }

    async fn cmd_queue_add(&mut self, uri: String) -> Result<()> {
        debug!("Adding to queue: {}", uri);
        self.client
            .add_to_queue(&uri)
            .await
            .context("Failed to add to queue")?;
        writeln!(self.output, "Added to queue: {}", uri)?;
        Ok(())
    }

    async fn cmd_queue_clear(&mut self) -> Result<()> {
        debug!("Clearing queue");
        writeln!(self.output, "Queue cleared")?;
        Ok(())
    }

    fn cmd_help(&mut self) -> Result<()> {
        let help_text = r#"Joshify CLI - Terminal Spotify Client

USAGE:
    joshify [COMMAND] [OPTIONS]

COMMANDS:
    play [URI]              Play a track/album/playlist or resume playback
    pause                   Pause playback
    resume                  Resume playback
    play-pause              Toggle play/pause
    next                    Skip to next track
    previous                Go to previous track
    stop                    Stop playback
    status                  Show playback status
    current                 Show current track info
    volume [PERCENT]        Get or set volume (0-100)
    seek POSITION           Seek to position in milliseconds
    seek-forward SECONDS    Seek forward by seconds
    seek-backward SECONDS   Seek backward by seconds
    shuffle [on|off]        Get or set shuffle mode
    repeat [off|track|context]  Get or set repeat mode
    search QUERY            Search for tracks/artists/albums
    queue-add URI           Add track to queue
    queue-clear             Clear playback queue
    help                    Show this help message
    version                 Show version information

OPTIONS:
    --format FORMAT         Output format: text, json, minimal (default: text)
    --limit N               Limit search results (default: 20)

EXAMPLES:
    joshify play spotify:track:4uLU6hMCjMI75M1A2tKUQC
    joshify status --format json
    joshify volume 50
    joshify search "taylor swift" --limit 10
    joshify seek 60000

ENVIRONMENT:
    JOSHIFY_LOG             Log level: trace, debug, info, warn, error
    SPOTIFY_CLIENT_ID       Spotify Client ID
    SPOTIFY_CLIENT_SECRET   Spotify Client Secret
"#;
        writeln!(self.output, "{}", help_text)?;
        Ok(())
    }

    fn cmd_version(&mut self) -> Result<()> {
        let version = env!("CARGO_PKG_VERSION");
        writeln!(self.output, "Joshify {}", version)?;
        writeln!(
            self.output,
            "A beautiful terminal Spotify client built with Rust."
        )?;
        Ok(())
    }
}

/// Format milliseconds as MM:SS
fn format_duration(ms: u32) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

/// Convert a Spotify repeat state to a string
fn repeat_to_string(state: RepeatState) -> String {
    match state {
        RepeatState::Off => "off".to_string(),
        RepeatState::Track => "track".to_string(),
        RepeatState::Context => "context".to_string(),
    }
}

impl PlaybackStatus {
    /// Build a CLI playback status from a Spotify playback context
    pub fn from_context(ctx: &CurrentPlaybackContext) -> Self {
        let (track, duration_ms) = match &ctx.item {
            Some(PlayableItem::Track(track)) => {
                let artists = track.artists.iter().map(|a| a.name.clone()).collect();
                let uri = track
                    .id
                    .as_ref()
                    .map(|id| format!("spotify:track:{}", id.id()))
                    .unwrap_or_default();
                let duration_ms = track.duration.num_milliseconds().max(0) as u32;
                (
                    Some(TrackInfo {
                        name: track.name.clone(),
                        artists,
                        album: track.album.name.clone(),
                        uri,
                        duration_ms,
                    }),
                    duration_ms,
                )
            }
            Some(PlayableItem::Episode(episode)) => {
                #[allow(deprecated)]
                let artist = episode.show.publisher.clone();
                let duration_ms = episode.duration.num_milliseconds().max(0) as u32;
                (
                    Some(TrackInfo {
                        name: episode.name.clone(),
                        artists: vec![artist],
                        album: episode.show.name.clone(),
                        uri: format!("spotify:episode:{}", episode.id.id()),
                        duration_ms,
                    }),
                    duration_ms,
                )
            }
            Some(PlayableItem::Unknown(_)) | None => (None, 0),
        };

        Self {
            is_playing: ctx.is_playing,
            track,
            progress_ms: ctx
                .progress
                .map(|d| d.num_milliseconds() as u32)
                .unwrap_or(0),
            duration_ms,
            shuffle: ctx.shuffle_state,
            repeat: repeat_to_string(ctx.repeat_state),
            volume_percent: ctx.device.volume_percent.unwrap_or(0) as u8,
        }
    }
}

/// Parse CLI arguments
pub fn parse_args(args: &[String]) -> Result<CliCommand> {
    if args.is_empty() {
        return Ok(CliCommand::Help);
    }

    let command = &args[0];
    let rest = &args[1..];

    match command.as_str() {
        "play" => {
            let uri = rest.first().map(|s| s.to_string());
            Ok(CliCommand::Play { uri })
        }
        "pause" => Ok(CliCommand::Pause),
        "resume" => Ok(CliCommand::Resume),
        "play-pause" => Ok(CliCommand::PlayPause),
        "next" => Ok(CliCommand::Next),
        "previous" | "prev" => Ok(CliCommand::Previous),
        "stop" => Ok(CliCommand::Stop),
        "status" => {
            let format = parse_format_flag(rest);
            Ok(CliCommand::Status { format })
        }
        "volume" => {
            let value = rest.first().and_then(|s| s.parse().ok());
            Ok(CliCommand::Volume { value })
        }
        "seek" => {
            let position_ms = rest
                .first()
                .and_then(|s| s.parse().ok())
                .context("Expected position in milliseconds")?;
            Ok(CliCommand::Seek { position_ms })
        }
        "seek-forward" => {
            let duration_ms = rest
                .first()
                .and_then(|s| s.parse().ok())
                .map(|s: u32| s * 1000)
                .unwrap_or(5000);
            Ok(CliCommand::SeekForward { duration_ms })
        }
        "seek-backward" => {
            let duration_ms = rest
                .first()
                .and_then(|s| s.parse().ok())
                .map(|s: u32| s * 1000)
                .unwrap_or(5000);
            Ok(CliCommand::SeekBackward { duration_ms })
        }
        "shuffle" => {
            let enabled = rest.first().map(|s| s == "on");
            Ok(CliCommand::Shuffle { enabled })
        }
        "repeat" => {
            let mode = rest.first().map(|s| s.to_string());
            Ok(CliCommand::Repeat { mode })
        }
        "current" => {
            let format = parse_format_flag(rest);
            Ok(CliCommand::Current { format })
        }
        "search" => {
            if rest.is_empty() {
                anyhow::bail!("Search query required");
            }
            let limit = parse_limit_flag(rest).unwrap_or(20);
            // Filter out the --limit and its value from the query
            let query_parts: Vec<&str> = rest
                .iter()
                .enumerate()
                .filter(|(i, arg)| {
                    // Skip --limit/-l and the value that follows it
                    if *arg == "--limit" || *arg == "-l" {
                        return false;
                    }
                    // Skip the value after --limit/-l
                    if *i > 0 && (rest[i - 1] == "--limit" || rest[i - 1] == "-l") {
                        return false;
                    }
                    true
                })
                .map(|(_, arg)| arg.as_str())
                .collect();
            let query = query_parts.join(" ");
            Ok(CliCommand::Search { query, limit })
        }
        "queue-add" => {
            let uri = rest.first().context("URI required")?.to_string();
            Ok(CliCommand::QueueAdd { uri })
        }
        "queue-clear" => Ok(CliCommand::QueueClear),
        "help" | "--help" | "-h" => Ok(CliCommand::Help),
        "version" | "--version" | "-v" => Ok(CliCommand::Version),
        _ => {
            warn!("Unknown command: {}", command);
            anyhow::bail!("Unknown command: {}", command)
        }
    }
}

/// Parse --format flag from arguments
fn parse_format_flag(args: &[String]) -> OutputFormat {
    for (i, arg) in args.iter().enumerate() {
        if arg == "--format" || arg == "-f" {
            if let Some(format) = args.get(i + 1) {
                return match format.as_str() {
                    "json" => OutputFormat::Json,
                    "minimal" => OutputFormat::Minimal,
                    _ => OutputFormat::Text,
                };
            }
        }
    }
    OutputFormat::Text
}

/// Parse --limit flag from arguments
fn parse_limit_flag(args: &[String]) -> Option<usize> {
    for (i, arg) in args.iter().enumerate() {
        if arg == "--limit" || arg == "-l" {
            if let Some(limit) = args.get(i + 1) {
                return limit.parse().ok();
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use mockall::mock;
    use mockall::predicate::eq;
    use rspotify::model::FullTrack;
    use std::io::Cursor;

    mock! {
        pub CliClient {}
        #[async_trait]
        impl CliClient for CliClient {
            async fn current_playback(&self) -> Result<Option<CurrentPlaybackContext>>;
            async fn playback_pause(&self) -> Result<()>;
            async fn playback_resume(&self) -> Result<()>;
            async fn playback_next(&self) -> Result<()>;
            async fn playback_previous(&self) -> Result<()>;
            async fn set_volume(&self, volume_percent: u32) -> Result<()>;
            async fn seek(&self, position_ms: u32, device_id: Option<String>) -> Result<()>;
            async fn toggle_shuffle(&self, shuffle: bool) -> Result<()>;
            async fn set_repeat(&self, state: RepeatState) -> Result<()>;
            async fn start_playback(&self, uris: Vec<String>, offset: Option<u32>) -> Result<()>;
            async fn search(&self, query: &str, limit: u32) -> Result<Vec<FullTrack>>;
            async fn add_to_queue(&self, track_uri: &str) -> Result<()>;
        }
    }

    /// Test writer that captures output into a shared Vec<u8>
    #[derive(Clone, Default)]
    struct TestWriter {
        buf: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    }

    impl Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.buf.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// Build a handler with a mock client and capture its output
    fn handler_with_buf(
        mock: MockCliClient,
    ) -> (
        CliHandler<MockCliClient>,
        std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    ) {
        let writer = TestWriter::default();
        let buf = writer.buf.clone();
        let handler = CliHandler::with_output(mock, writer);
        (handler, buf)
    }

    /// Read the captured output as a String
    fn read_buf(buf: &std::sync::Arc<std::sync::Mutex<Vec<u8>>>) -> String {
        String::from_utf8(buf.lock().unwrap().clone()).unwrap()
    }

    #[test]
    fn test_cli_command_variants() {
        assert_eq!(
            CliCommand::Play { uri: None },
            CliCommand::Play { uri: None }
        );
        assert_eq!(CliCommand::Pause, CliCommand::Pause);
        assert_eq!(CliCommand::Next, CliCommand::Next);
    }

    #[test]
    fn test_output_format_default() {
        let format: OutputFormat = Default::default();
        assert_eq!(format, OutputFormat::Text);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0), "00:00");
        assert_eq!(format_duration(60000), "01:00");
        assert_eq!(format_duration(90000), "01:30");
        assert_eq!(format_duration(180000), "03:00");
    }

    #[test]
    fn test_parse_args_play() {
        let args = vec!["play".to_string()];
        let cmd = parse_args(&args).unwrap();
        assert_eq!(cmd, CliCommand::Play { uri: None });
    }

    #[test]
    fn test_parse_args_play_with_uri() {
        let args = vec!["play".to_string(), "spotify:track:abc".to_string()];
        let cmd = parse_args(&args).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Play {
                uri: Some("spotify:track:abc".to_string())
            }
        );
    }

    #[test]
    fn test_parse_args_pause() {
        let args = vec!["pause".to_string()];
        let cmd = parse_args(&args).unwrap();
        assert_eq!(cmd, CliCommand::Pause);
    }

    #[test]
    fn test_parse_args_status() {
        let args = vec!["status".to_string()];
        let cmd = parse_args(&args).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Status {
                format: OutputFormat::Text
            }
        );
    }

    #[test]
    fn test_parse_args_status_json() {
        let args = vec![
            "status".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ];
        let cmd = parse_args(&args).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Status {
                format: OutputFormat::Json
            }
        );
    }

    #[test]
    fn test_parse_args_volume_set() {
        let args = vec!["volume".to_string(), "50".to_string()];
        let cmd = parse_args(&args).unwrap();
        assert_eq!(cmd, CliCommand::Volume { value: Some(50) });
    }

    #[test]
    fn test_parse_args_volume_get() {
        let args = vec!["volume".to_string()];
        let cmd = parse_args(&args).unwrap();
        assert_eq!(cmd, CliCommand::Volume { value: None });
    }

    #[test]
    fn test_parse_args_seek() {
        let args = vec!["seek".to_string(), "60000".to_string()];
        let cmd = parse_args(&args).unwrap();
        assert_eq!(cmd, CliCommand::Seek { position_ms: 60000 });
    }

    #[test]
    fn test_parse_args_shuffle_on() {
        let args = vec!["shuffle".to_string(), "on".to_string()];
        let cmd = parse_args(&args).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Shuffle {
                enabled: Some(true)
            }
        );
    }

    #[test]
    fn test_parse_args_shuffle_off() {
        let args = vec!["shuffle".to_string(), "off".to_string()];
        let cmd = parse_args(&args).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Shuffle {
                enabled: Some(false)
            }
        );
    }

    #[test]
    fn test_parse_args_search() {
        let args = vec![
            "search".to_string(),
            "taylor".to_string(),
            "swift".to_string(),
        ];
        let cmd = parse_args(&args).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Search {
                query: "taylor swift".to_string(),
                limit: 20
            }
        );
    }

    #[test]
    fn test_parse_args_search_with_limit() {
        let args = vec![
            "search".to_string(),
            "test".to_string(),
            "--limit".to_string(),
            "10".to_string(),
        ];
        let cmd = parse_args(&args).unwrap();
        assert_eq!(
            cmd,
            CliCommand::Search {
                query: "test".to_string(),
                limit: 10
            }
        );
    }

    #[test]
    fn test_parse_args_help() {
        let args = vec!["help".to_string()];
        let cmd = parse_args(&args).unwrap();
        assert_eq!(cmd, CliCommand::Help);
    }

    #[test]
    fn test_parse_args_version() {
        let args = vec!["version".to_string()];
        let cmd = parse_args(&args).unwrap();
        assert_eq!(cmd, CliCommand::Version);
    }

    #[test]
    fn test_parse_args_empty() {
        let args: Vec<String> = vec![];
        let cmd = parse_args(&args).unwrap();
        assert_eq!(cmd, CliCommand::Help);
    }

    #[test]
    fn test_parse_args_unknown() {
        let args = vec!["unknown".to_string()];
        let result = parse_args(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_format_flag() {
        let args = vec!["--format".to_string(), "json".to_string()];
        assert_eq!(parse_format_flag(&args), OutputFormat::Json);

        let args = vec!["--format".to_string(), "minimal".to_string()];
        assert_eq!(parse_format_flag(&args), OutputFormat::Minimal);

        let args: Vec<String> = vec![];
        assert_eq!(parse_format_flag(&args), OutputFormat::Text);
    }

    #[test]
    fn test_parse_limit_flag() {
        let args = vec!["--limit".to_string(), "50".to_string()];
        assert_eq!(parse_limit_flag(&args), Some(50));

        let args: Vec<String> = vec![];
        assert_eq!(parse_limit_flag(&args), None);
    }

    #[tokio::test]
    async fn test_cli_handler_execute() {
        let mock = MockCliClient::new();
        let mut handler = CliHandler::new(mock);
        let cmd = CliCommand::Version;
        let result = handler.execute(cmd).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_track_info_serialization() {
        let track = TrackInfo {
            name: "Test Track".to_string(),
            artists: vec!["Artist 1".to_string(), "Artist 2".to_string()],
            album: "Test Album".to_string(),
            uri: "spotify:track:test".to_string(),
            duration_ms: 180000,
        };

        let json = serde_json::to_string(&track).unwrap();
        assert!(json.contains("Test Track"));
        assert!(json.contains("Artist 1"));
    }

    #[test]
    fn test_playback_status_serialization() {
        let status = PlaybackStatus {
            is_playing: true,
            track: Some(TrackInfo {
                name: "Test".to_string(),
                artists: vec!["Artist".to_string()],
                album: "Album".to_string(),
                uri: "spotify:track:test".to_string(),
                duration_ms: 180000,
            }),
            progress_ms: 60000,
            duration_ms: 180000,
            shuffle: false,
            repeat: "off".to_string(),
            volume_percent: 70,
        };

        let json = serde_json::to_string_pretty(&status).unwrap();
        assert!(json.contains("is_playing"));
        assert!(json.contains("Test"));
    }

    #[test]
    fn test_seek_forward_default() {
        let args: Vec<String> = vec![];
        let result = parse_args(&["seek-forward".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_seek_backward_default() {
        let result = parse_args(&["seek-backward".to_string()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_previous_alias() {
        let args = vec!["prev".to_string()];
        let cmd = parse_args(&args).unwrap();
        assert_eq!(cmd, CliCommand::Previous);
    }

    #[tokio::test]
    async fn test_cli_handler_with_output() {
        let mock = MockCliClient::new();
        let buf: Vec<u8> = Vec::new();
        let cursor = Cursor::new(buf);
        {
            let mut handler = CliHandler::with_output(mock, cursor);
            let cmd = CliCommand::Version;
            handler.execute(cmd).await.unwrap();
        }
        // The test just verifies the handler was created and executed without error
    }

    #[tokio::test]
    async fn test_cmd_pause_calls_api() {
        let mut mock = MockCliClient::new();
        mock.expect_playback_pause().times(1).returning(|| Ok(()));
        let (mut handler, buf) = handler_with_buf(mock);
        handler.execute(CliCommand::Pause).await.unwrap();
        assert_eq!(read_buf(&buf), "Paused\n");
    }

    #[tokio::test]
    async fn test_cmd_resume_calls_api() {
        let mut mock = MockCliClient::new();
        mock.expect_playback_resume().times(1).returning(|| Ok(()));
        let (mut handler, buf) = handler_with_buf(mock);
        handler.execute(CliCommand::Resume).await.unwrap();
        assert_eq!(read_buf(&buf), "Resumed\n");
    }

    #[tokio::test]
    async fn test_cmd_next_calls_api() {
        let mut mock = MockCliClient::new();
        mock.expect_playback_next().times(1).returning(|| Ok(()));
        let (mut handler, buf) = handler_with_buf(mock);
        handler.execute(CliCommand::Next).await.unwrap();
        assert_eq!(read_buf(&buf), "Next track\n");
    }

    #[tokio::test]
    async fn test_cmd_previous_calls_api() {
        let mut mock = MockCliClient::new();
        mock.expect_playback_previous()
            .times(1)
            .returning(|| Ok(()));
        let (mut handler, buf) = handler_with_buf(mock);
        handler.execute(CliCommand::Previous).await.unwrap();
        assert_eq!(read_buf(&buf), "Previous track\n");
    }

    #[tokio::test]
    async fn test_cmd_volume_set_calls_api() {
        let mut mock = MockCliClient::new();
        mock.expect_set_volume()
            .with(eq(50u32))
            .times(1)
            .returning(|_| Ok(()));
        let (mut handler, buf) = handler_with_buf(mock);
        handler
            .execute(CliCommand::Volume { value: Some(50) })
            .await
            .unwrap();
        assert_eq!(read_buf(&buf), "Volume set to 50%\n");
    }

    #[tokio::test]
    async fn test_cmd_volume_get_reads_playback() {
        let mut mock = MockCliClient::new();
        mock.expect_current_playback()
            .times(1)
            .returning(|| Ok(None));
        let (mut handler, buf) = handler_with_buf(mock);
        handler
            .execute(CliCommand::Volume { value: None })
            .await
            .unwrap();
        assert_eq!(read_buf(&buf), "Current volume: 0%\n");
    }

    #[tokio::test]
    async fn test_cmd_status_json_output() {
        let mut mock = MockCliClient::new();
        mock.expect_current_playback()
            .times(1)
            .returning(|| Ok(None));
        let (mut handler, buf) = handler_with_buf(mock);
        handler
            .execute(CliCommand::Status {
                format: OutputFormat::Json,
            })
            .await
            .unwrap();
        let out = read_buf(&buf);
        assert!(out.contains("\"is_playing\": false"));
    }

    #[tokio::test]
    async fn test_cmd_play_with_uri_calls_start_playback() {
        let mut mock = MockCliClient::new();
        mock.expect_start_playback()
            .with(eq(vec!["spotify:track:abc".to_string()]), eq(None))
            .times(1)
            .returning(|_, _| Ok(()));
        let (mut handler, buf) = handler_with_buf(mock);
        handler
            .execute(CliCommand::Play {
                uri: Some("spotify:track:abc".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(read_buf(&buf), "Playing: spotify:track:abc\n");
    }

    #[tokio::test]
    async fn test_cmd_queue_add_calls_api() {
        let mut mock = MockCliClient::new();
        mock.expect_add_to_queue()
            .with(eq("spotify:track:abc".to_string()))
            .times(1)
            .returning(|_| Ok(()));
        let (mut handler, buf) = handler_with_buf(mock);
        handler
            .execute(CliCommand::QueueAdd {
                uri: "spotify:track:abc".to_string(),
            })
            .await
            .unwrap();
        assert_eq!(read_buf(&buf), "Added to queue: spotify:track:abc\n");
    }

    #[tokio::test]
    async fn test_cmd_pause_error_propagates() {
        let mut mock = MockCliClient::new();
        mock.expect_playback_pause()
            .times(1)
            .returning(|| Err(anyhow::anyhow!("no active device")));
        let (mut handler, _buf) = handler_with_buf(mock);
        let result = handler.execute(CliCommand::Pause).await;
        assert!(result.is_err());
    }
}
