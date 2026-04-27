//! CLI commands for Joshify
//!
//! Provides non-interactive commands for scripting and automation:
//! joshify play, joshify pause, joshify status, etc.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;
use tracing::{debug, info, warn};

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
pub struct CliHandler {
    /// Output stream (usually stdout)
    output: Box<dyn Write>,
}

impl CliHandler {
    /// Create new CLI handler
    pub fn new() -> Self {
        Self {
            output: Box::new(std::io::stdout()),
        }
    }

    /// Create with custom output
    pub fn with_output<W: Write + 'static>(output: W) -> Self {
        Self {
            output: Box::new(output),
        }
    }

    /// Execute a CLI command
    pub fn execute(&mut self, command: CliCommand) -> Result<()> {
        info!("Executing CLI command: {:?}", command);

        match command {
            CliCommand::Play { uri } => self.cmd_play(uri),
            CliCommand::Pause => self.cmd_pause(),
            CliCommand::Resume => self.cmd_resume(),
            CliCommand::PlayPause => self.cmd_play_pause(),
            CliCommand::Next => self.cmd_next(),
            CliCommand::Previous => self.cmd_previous(),
            CliCommand::Stop => self.cmd_stop(),
            CliCommand::Status { format } => self.cmd_status(format),
            CliCommand::Volume { value } => self.cmd_volume(value),
            CliCommand::Seek { position_ms } => self.cmd_seek(position_ms),
            CliCommand::SeekForward { duration_ms } => self.cmd_seek_forward(duration_ms),
            CliCommand::SeekBackward { duration_ms } => self.cmd_seek_backward(duration_ms),
            CliCommand::Shuffle { enabled } => self.cmd_shuffle(enabled),
            CliCommand::Repeat { mode } => self.cmd_repeat(mode),
            CliCommand::Current { format } => self.cmd_current(format),
            CliCommand::Search { query, limit } => self.cmd_search(query, limit),
            CliCommand::QueueAdd { uri } => self.cmd_queue_add(uri),
            CliCommand::QueueClear => self.cmd_queue_clear(),
            CliCommand::Help => self.cmd_help(),
            CliCommand::Version => self.cmd_version(),
        }
    }

    fn cmd_play(&mut self, uri: Option<String>) -> Result<()> {
        if let Some(track_uri) = uri {
            debug!("Playing track: {}", track_uri);
            writeln!(self.output, "Playing: {}", track_uri)?;
        } else {
            debug!("Resuming playback");
            writeln!(self.output, "Resuming playback")?;
        }
        Ok(())
    }

    fn cmd_pause(&mut self) -> Result<()> {
        debug!("Pausing playback");
        writeln!(self.output, "Paused")?;
        Ok(())
    }

    fn cmd_resume(&mut self) -> Result<()> {
        debug!("Resuming playback");
        writeln!(self.output, "Resumed")?;
        Ok(())
    }

    fn cmd_play_pause(&mut self) -> Result<()> {
        debug!("Toggling play/pause");
        writeln!(self.output, "Toggled play/pause")?;
        Ok(())
    }

    fn cmd_next(&mut self) -> Result<()> {
        debug!("Skipping to next track");
        writeln!(self.output, "Next track")?;
        Ok(())
    }

    fn cmd_previous(&mut self) -> Result<()> {
        debug!("Going to previous track");
        writeln!(self.output, "Previous track")?;
        Ok(())
    }

    fn cmd_stop(&mut self) -> Result<()> {
        debug!("Stopping playback");
        writeln!(self.output, "Stopped")?;
        Ok(())
    }

    fn cmd_status(&mut self, format: OutputFormat) -> Result<()> {
        // Mock status for now
        let status = PlaybackStatus {
            is_playing: true,
            track: Some(TrackInfo {
                name: "Test Track".to_string(),
                artists: vec!["Test Artist".to_string()],
                album: "Test Album".to_string(),
                uri: "spotify:track:test".to_string(),
                duration_ms: 180000,
            }),
            progress_ms: 60000,
            duration_ms: 180000,
            shuffle: false,
            repeat: "off".to_string(),
            volume_percent: 70,
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
                writeln!(self.output, "Status: {}", if status.is_playing { "Playing" } else { "Paused" })?;
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
                writeln!(self.output, "Shuffle: {}", if status.shuffle { "on" } else { "off" })?;
                writeln!(self.output, "Repeat: {}", status.repeat)?;
            }
        }

        Ok(())
    }

    fn cmd_volume(&mut self, value: Option<u8>) -> Result<()> {
        match value {
            Some(v) => {
                let clamped = v.min(100);
                debug!("Setting volume to {}%", clamped);
                writeln!(self.output, "Volume set to {}%", clamped)?;
            }
            None => {
                writeln!(self.output, "Current volume: 70%")?;
            }
        }
        Ok(())
    }

    fn cmd_seek(&mut self, position_ms: u32) -> Result<()> {
        debug!("Seeking to {}ms", position_ms);
        writeln!(self.output, "Seeked to {}", format_duration(position_ms))?;
        Ok(())
    }

    fn cmd_seek_forward(&mut self, duration_ms: u32) -> Result<()> {
        debug!("Seeking forward {}ms", duration_ms);
        writeln!(self.output, "Seeked forward {}s", duration_ms / 1000)?;
        Ok(())
    }

    fn cmd_seek_backward(&mut self, duration_ms: u32) -> Result<()> {
        debug!("Seeking backward {}ms", duration_ms);
        writeln!(self.output, "Seeked backward {}s", duration_ms / 1000)?;
        Ok(())
    }

    fn cmd_shuffle(&mut self, enabled: Option<bool>) -> Result<()> {
        match enabled {
            Some(true) => {
                debug!("Enabling shuffle");
                writeln!(self.output, "Shuffle: on")?;
            }
            Some(false) => {
                debug!("Disabling shuffle");
                writeln!(self.output, "Shuffle: off")?;
            }
            None => {
                writeln!(self.output, "Shuffle: off")?;
            }
        }
        Ok(())
    }

    fn cmd_repeat(&mut self, mode: Option<String>) -> Result<()> {
        match mode {
            Some(m) => {
                debug!("Setting repeat mode to: {}", m);
                writeln!(self.output, "Repeat: {}", m)?;
            }
            None => {
                writeln!(self.output, "Repeat: off")?;
            }
        }
        Ok(())
    }

    fn cmd_current(&mut self, format: OutputFormat) -> Result<()> {
        // Same as status but only shows track info
        self.cmd_status(format)?;
        Ok(())
    }

    fn cmd_search(&mut self, query: String, limit: usize) -> Result<()> {
        debug!("Searching for: {} (limit: {})", query, limit);
        writeln!(self.output, "Searching for: {}", query)?;
        writeln!(self.output, "Results: (mock)")?;
        writeln!(self.output, "  1. Test Track - Test Artist")?;
        Ok(())
    }

    fn cmd_queue_add(&mut self, uri: String) -> Result<()> {
        debug!("Adding to queue: {}", uri);
        writeln!(self.output, "Added to queue: {}", uri)?;
        Ok(())
    }

    fn cmd_queue_clear(&mut self) -> Result<()> {
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
        writeln!(self.output, "A beautiful terminal Spotify client built with Rust.")?;
        Ok(())
    }
}

impl Default for CliHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Format milliseconds as MM:SS
fn format_duration(ms: u32) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
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
            let query = rest.join(" ");
            let limit = parse_limit_flag(rest).unwrap_or(20);
            Ok(CliCommand::Search { query, limit })
        }
        "queue-add" => {
            let uri = rest
                .first()
                .context("URI required")?
                .to_string();
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

    #[test]
    fn test_cli_command_variants() {
        assert_eq!(CliCommand::Play { uri: None }, CliCommand::Play { uri: None });
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
        assert_eq!(cmd, CliCommand::Play { uri: Some("spotify:track:abc".to_string()) });
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
        assert_eq!(cmd, CliCommand::Status { format: OutputFormat::Text });
    }

    #[test]
    fn test_parse_args_status_json() {
        let args = vec!["status".to_string(), "--format".to_string(), "json".to_string()];
        let cmd = parse_args(&args).unwrap();
        assert_eq!(cmd, CliCommand::Status { format: OutputFormat::Json });
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
        assert_eq!(cmd, CliCommand::Shuffle { enabled: Some(true) });
    }

    #[test]
    fn test_parse_args_shuffle_off() {
        let args = vec!["shuffle".to_string(), "off".to_string()];
        let cmd = parse_args(&args).unwrap();
        assert_eq!(cmd, CliCommand::Shuffle { enabled: Some(false) });
    }

    #[test]
    fn test_parse_args_search() {
        let args = vec!["search".to_string(), "taylor".to_string(), "swift".to_string()];
        let cmd = parse_args(&args).unwrap();
        assert_eq!(cmd, CliCommand::Search { query: "taylor swift".to_string(), limit: 20 });
    }

    #[test]
    fn test_parse_args_search_with_limit() {
        let args = vec!["search".to_string(), "test".to_string(), "--limit".to_string(), "10".to_string()];
        let cmd = parse_args(&args).unwrap();
        assert_eq!(cmd, CliCommand::Search { query: "test".to_string(), limit: 10 });
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

    #[test]
    fn test_cli_handler_execute() {
        let mut handler = CliHandler::new();
        let cmd = CliCommand::Version;
        let result = handler.execute(cmd);
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

    #[test]
    fn test_cli_handler_with_output() {
        let mut buf: Vec<u8> = Vec::new();
        let mut handler = CliHandler::with_output(&mut buf);
        let cmd = CliCommand::Pause;
        handler.execute(cmd).unwrap();
        assert!(!buf.is_empty());
    }
}
