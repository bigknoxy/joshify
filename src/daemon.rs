//! Daemon Mode for Joshify
//!
//! Provides a background daemon service that can receive commands via IPC
//! and manage Spotify playback independently of the TUI.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Default socket path in ~/.cache/joshify/
const SOCKET_FILENAME: &str = "daemon.sock";
const PID_FILENAME: &str = "daemon.pid";

/// Daemon commands sent from CLI to daemon
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "command", content = "args")]
pub enum DaemonCommand {
    /// Play a specific URI or resume playback
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
    Status,
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
    Current,
    /// Add track to queue
    QueueAdd { uri: String },
    /// Clear queue
    QueueClear,
    /// Check if daemon is alive
    Ping,
    /// Stop the daemon
    Shutdown,
}

/// Daemon responses sent back to CLI
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status")]
pub enum DaemonResponse {
    /// Command executed successfully
    #[serde(rename = "ok")]
    Ok { message: String },
    /// Current playback status
    #[serde(rename = "status")]
    Status { data: PlaybackStatus },
    /// Current track info
    #[serde(rename = "track")]
    Track { data: TrackInfo },
    /// Error response
    #[serde(rename = "error")]
    Error { message: String },
    /// Pong response
    #[serde(rename = "pong")]
    Pong,
}

/// Playback status information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlaybackStatus {
    pub is_playing: bool,
    pub is_paused: bool,
    pub track: Option<TrackInfo>,
    pub progress_ms: u32,
    pub duration_ms: u32,
    pub shuffle: bool,
    pub repeat: String,
    pub volume_percent: u8,
    pub queue_length: usize,
}

/// Track information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TrackInfo {
    pub name: String,
    pub artists: Vec<String>,
    pub album: String,
    pub uri: String,
    pub duration_ms: u32,
}

/// Daemon configuration
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Path to the socket file
    pub socket_path: PathBuf,
    /// Path to the PID file
    pub pid_path: PathBuf,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        let cache_dir = get_cache_dir().unwrap_or_else(|_| PathBuf::from("/tmp/joshify"));
        Self {
            socket_path: cache_dir.join(SOCKET_FILENAME),
            pid_path: cache_dir.join(PID_FILENAME),
        }
    }
}

/// Get the cache directory for joshify
fn get_cache_dir() -> Result<PathBuf> {
    let cache_dir = dirs_next::cache_dir()
        .context("Failed to get cache directory")?
        .join("joshify");
    std::fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}

/// Daemon service that manages playback in the background
pub struct DaemonService {
    config: DaemonConfig,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl DaemonService {
    /// Create a new daemon service with default configuration
    pub fn new() -> Self {
        Self {
            config: DaemonConfig::default(),
            shutdown_tx: None,
        }
    }

    /// Create a new daemon service with custom configuration
    pub fn with_config(config: DaemonConfig) -> Self {
        Self {
            config,
            shutdown_tx: None,
        }
    }

    /// Check if the daemon is currently running
    pub fn is_running() -> bool {
        if let Ok(config) = Self::get_config() {
            // Check PID file
            if let Ok(pid_str) = std::fs::read_to_string(&config.pid_path) {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    // Check if process exists
                    #[cfg(unix)]
                    {
                        use std::process::Command;
                        let output = Command::new("kill")
                            .args(["-0", &pid.to_string()])
                            .output();
                        return output.map(|o| o.status.success()).unwrap_or(false);
                    }
                }
            }
        }
        false
    }

    /// Get the daemon configuration
    fn get_config() -> Result<DaemonConfig> {
        Ok(DaemonConfig::default())
    }

    /// Get the PID of the running daemon if any
    pub fn get_pid() -> Result<u32> {
        let config = DaemonConfig::default();
        let pid_str = std::fs::read_to_string(&config.pid_path)
            .context("No PID file found - daemon not running")?;
        let pid = pid_str.trim().parse::<u32>()
            .context("Invalid PID in PID file")?;
        Ok(pid)
    }

    /// Start the daemon in the background
    pub async fn start_daemon() -> Result<u32> {
        if Self::is_running() {
            let pid = Self::get_pid()?;
            info!("Daemon is already running with PID {}", pid);
            return Ok(pid);
        }

        // Remove old socket if it exists
        let config = DaemonConfig::default();
        if config.socket_path.exists() {
            std::fs::remove_file(&config.socket_path)?;
        }

        // Spawn daemon process
        let current_exe = std::env::current_exe()
            .context("Failed to get current executable path")?;
        
        let child = std::process::Command::new(current_exe)
            .arg("--daemon")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .context("Failed to spawn daemon process")?;

        let pid = child.id();
        
        // Wait a moment for daemon to start
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // Verify daemon started
        if !Self::is_running() {
            anyhow::bail!("Daemon failed to start");
        }

        info!("Daemon started with PID {}", pid);
        Ok(pid)
    }

    /// Stop the daemon gracefully
    pub async fn stop_daemon() -> Result<()> {
        if !Self::is_running() {
            warn!("Daemon is not running");
            return Ok(());
        }

        // Send shutdown command
        match Self::send_command(DaemonCommand::Shutdown).await {
            Ok(_) => {
                // Wait for daemon to stop
                for _ in 0..50 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    if !Self::is_running() {
                        info!("Daemon stopped successfully");
                        return Ok(());
                    }
                }
                anyhow::bail!("Daemon did not stop in time");
            }
            Err(e) => {
                // Force kill if graceful shutdown failed
                let config = DaemonConfig::default();
                if let Ok(pid) = Self::get_pid() {
                    #[cfg(unix)]
                    {
                        use std::process::Command;
                        let _ = Command::new("kill")
                            .args(["-9", &pid.to_string()])
                            .output();
                    }
                }
                // Clean up socket and PID file
                let _ = std::fs::remove_file(&config.socket_path);
                let _ = std::fs::remove_file(&config.pid_path);
                Err(e)
            }
        }
    }

    /// Send a command to the daemon and wait for response
    pub async fn send_command(command: DaemonCommand) -> Result<DaemonResponse> {
        let config = DaemonConfig::default();
        
        if !config.socket_path.exists() {
            anyhow::bail!("Daemon socket not found - is the daemon running?");
        }

        // Connect to daemon
        let mut stream = UnixStream::connect(&config.socket_path)
            .await
            .context("Failed to connect to daemon socket")?;

        // Serialize command
        let command_json = serde_json::to_string(&command)
            .context("Failed to serialize command")?;
        let command_bytes = command_json.as_bytes();
        let length = command_bytes.len() as u32;

        // Send length prefix + command
        stream.write_all(&length.to_be_bytes()).await?;
        stream.write_all(command_bytes).await?;
        stream.flush().await?;

        // Read response
        let mut length_buf = [0u8; 4];
        stream.read_exact(&mut length_buf).await
            .context("Failed to read response length")?;
        let response_length = u32::from_be_bytes(length_buf) as usize;

        if response_length > 1024 * 1024 {
            anyhow::bail!("Response too large");
        }

        let mut response_buf = vec![0u8; response_length];
        stream.read_exact(&mut response_buf).await
            .context("Failed to read response")?;

        let response: DaemonResponse = serde_json::from_slice(&response_buf)
            .context("Failed to deserialize response")?;

        Ok(response)
    }

    /// Run the daemon service (this blocks and runs the event loop)
    pub async fn run(&mut self) -> Result<()> {
        // Write PID file
        let pid = std::process::id();
        tokio::fs::write(&self.config.pid_path, pid.to_string()).await
            .context("Failed to write PID file")?;

        // Clean up old socket
        if self.config.socket_path.exists() {
            tokio::fs::remove_file(&self.config.socket_path).await?;
        }

        // Create socket
        let listener = UnixListener::bind(&self.config.socket_path)
            .context("Failed to bind to socket")?;

        info!("Daemon listening on {:?}", self.config.socket_path);

        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        // Daemon state
        let mut daemon_state = DaemonState::default();

        loop {
            tokio::select! {
                // Handle incoming connections
                result = listener.accept() => {
                    match result {
                        Ok((stream, _addr)) => {
                            debug!("Accepted new connection");
                            if let Err(e) = self.handle_connection(stream, &mut daemon_state).await {
                                warn!("Connection handler error: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Accept error: {}", e);
                        }
                    }
                }
                // Handle shutdown signal
                _ = shutdown_rx.recv() => {
                    info!("Shutdown signal received");
                    break;
                }
            }
        }

        // Clean up
        info!("Daemon shutting down");
        let _ = tokio::fs::remove_file(&self.config.pid_path).await;
        let _ = tokio::fs::remove_file(&self.config.socket_path).await;

        Ok(())
    }

    /// Handle a single client connection
    async fn handle_connection(
        &self,
        mut stream: UnixStream,
        daemon_state: &mut DaemonState,
    ) -> Result<()> {
        // Read command length
        let mut length_buf = [0u8; 4];
        stream.read_exact(&mut length_buf).await
            .context("Failed to read command length")?;
        let command_length = u32::from_be_bytes(length_buf) as usize;

        if command_length > 1024 * 1024 {
            anyhow::bail!("Command too large");
        }

        // Read command
        let mut command_buf = vec![0u8; command_length];
        stream.read_exact(&mut command_buf).await
            .context("Failed to read command")?;

        let command: DaemonCommand = serde_json::from_slice(&command_buf)
            .context("Failed to deserialize command")?;

        debug!("Received command: {:?}", command);

        // Execute command
        let response = self.execute_command(command, daemon_state).await;

        // Send response
        let response_json = serde_json::to_string(&response)?;
        let response_bytes = response_json.as_bytes();
        let response_length = response_bytes.len() as u32;

        stream.write_all(&response_length.to_be_bytes()).await?;
        stream.write_all(response_bytes).await?;
        stream.flush().await?;

        Ok(())
    }

    /// Execute a daemon command and return the response
    async fn execute_command(
        &self,
        command: DaemonCommand,
        daemon_state: &mut DaemonState,
    ) -> DaemonResponse {
        match command {
            DaemonCommand::Play { uri } => {
                if let Some(track_uri) = uri {
                    daemon_state.is_playing = true;
                    daemon_state.is_paused = false;
                    daemon_state.current_track = Some(TrackInfo {
                        name: format!("Track from {}", track_uri),
                        artists: vec!["Unknown Artist".to_string()],
                        album: "Unknown Album".to_string(),
                        uri: track_uri,
                        duration_ms: 180000,
                    });
                    DaemonResponse::Ok {
                        message: format!("Playing: {}", daemon_state.current_track.as_ref().unwrap().name),
                    }
                } else {
                    if daemon_state.current_track.is_some() {
                        daemon_state.is_playing = true;
                        daemon_state.is_paused = false;
                        DaemonResponse::Ok {
                            message: "Resumed playback".to_string(),
                        }
                    } else {
                        DaemonResponse::Error {
                            message: "No track to play".to_string(),
                        }
                    }
                }
            }
            DaemonCommand::Pause => {
                if daemon_state.is_playing {
                    daemon_state.is_paused = true;
                    daemon_state.is_playing = false;
                    DaemonResponse::Ok {
                        message: "Paused".to_string(),
                    }
                } else {
                    DaemonResponse::Error {
                        message: "Not playing".to_string(),
                    }
                }
            }
            DaemonCommand::Resume => {
                if daemon_state.is_paused {
                    daemon_state.is_paused = false;
                    daemon_state.is_playing = true;
                    DaemonResponse::Ok {
                        message: "Resumed".to_string(),
                    }
                } else if daemon_state.current_track.is_some() {
                    daemon_state.is_playing = true;
                    DaemonResponse::Ok {
                        message: "Resumed".to_string(),
                    }
                } else {
                    DaemonResponse::Error {
                        message: "No track to resume".to_string(),
                    }
                }
            }
            DaemonCommand::PlayPause => {
                if daemon_state.is_playing {
                    daemon_state.is_paused = true;
                    daemon_state.is_playing = false;
                    DaemonResponse::Ok {
                        message: "Paused".to_string(),
                    }
                } else if daemon_state.current_track.is_some() {
                    daemon_state.is_playing = true;
                    daemon_state.is_paused = false;
                    DaemonResponse::Ok {
                        message: "Resumed".to_string(),
                    }
                } else {
                    DaemonResponse::Error {
                        message: "No track loaded".to_string(),
                    }
                }
            }
            DaemonCommand::Next => {
                daemon_state.advance_track();
                DaemonResponse::Ok {
                    message: "Next track".to_string(),
                }
            }
            DaemonCommand::Previous => {
                daemon_state.rewind_track();
                DaemonResponse::Ok {
                    message: "Previous track".to_string(),
                }
            }
            DaemonCommand::Stop => {
                daemon_state.is_playing = false;
                daemon_state.is_paused = false;
                daemon_state.progress_ms = 0;
                DaemonResponse::Ok {
                    message: "Stopped".to_string(),
                }
            }
            DaemonCommand::Status => {
                let status = PlaybackStatus {
                    is_playing: daemon_state.is_playing,
                    is_paused: daemon_state.is_paused,
                    track: daemon_state.current_track.clone(),
                    progress_ms: daemon_state.progress_ms,
                    duration_ms: daemon_state.current_track.as_ref().map(|t| t.duration_ms).unwrap_or(0),
                    shuffle: daemon_state.shuffle,
                    repeat: daemon_state.repeat.clone(),
                    volume_percent: daemon_state.volume,
                    queue_length: daemon_state.queue.len(),
                };
                DaemonResponse::Status { data: status }
            }
            DaemonCommand::Volume { value } => {
                match value {
                    Some(v) => {
                        daemon_state.volume = v.min(100);
                        DaemonResponse::Ok {
                            message: format!("Volume set to {}%", daemon_state.volume),
                        }
                    }
                    None => {
                        DaemonResponse::Ok {
                            message: format!("Volume: {}%", daemon_state.volume),
                        }
                    }
                }
            }
            DaemonCommand::Seek { position_ms } => {
                daemon_state.progress_ms = position_ms.min(daemon_state.current_track.as_ref().map(|t| t.duration_ms).unwrap_or(position_ms));
                DaemonResponse::Ok {
                    message: format!("Seeked to {}ms", daemon_state.progress_ms),
                }
            }
            DaemonCommand::SeekForward { duration_ms } => {
                let max_ms = daemon_state.current_track.as_ref().map(|t| t.duration_ms).unwrap_or(0);
                daemon_state.progress_ms = (daemon_state.progress_ms + duration_ms).min(max_ms);
                DaemonResponse::Ok {
                    message: format!("Seeked forward {}ms", duration_ms),
                }
            }
            DaemonCommand::SeekBackward { duration_ms } => {
                daemon_state.progress_ms = daemon_state.progress_ms.saturating_sub(duration_ms);
                DaemonResponse::Ok {
                    message: format!("Seeked backward {}ms", duration_ms),
                }
            }
            DaemonCommand::Shuffle { enabled } => {
                match enabled {
                    Some(e) => {
                        daemon_state.shuffle = e;
                        DaemonResponse::Ok {
                            message: format!("Shuffle: {}", if e { "on" } else { "off" }),
                        }
                    }
                    None => {
                        DaemonResponse::Ok {
                            message: format!("Shuffle: {}", if daemon_state.shuffle { "on" } else { "off" }),
                        }
                    }
                }
            }
            DaemonCommand::Repeat { mode } => {
                match mode {
                    Some(m) => {
                        daemon_state.repeat = m;
                        DaemonResponse::Ok {
                            message: format!("Repeat: {}", daemon_state.repeat),
                        }
                    }
                    None => {
                        DaemonResponse::Ok {
                            message: format!("Repeat: {}", daemon_state.repeat),
                        }
                    }
                }
            }
            DaemonCommand::Current => {
                match daemon_state.current_track.clone() {
                    Some(track) => DaemonResponse::Track { data: track },
                    None => DaemonResponse::Error {
                        message: "No track playing".to_string(),
                    },
                }
            }
            DaemonCommand::QueueAdd { uri } => {
                daemon_state.queue.push(uri.clone());
                DaemonResponse::Ok {
                    message: format!("Added to queue: {}", uri),
                }
            }
            DaemonCommand::QueueClear => {
                daemon_state.queue.clear();
                DaemonResponse::Ok {
                    message: "Queue cleared".to_string(),
                }
            }
            DaemonCommand::Ping => {
                DaemonResponse::Pong
            }
            DaemonCommand::Shutdown => {
                // Signal shutdown
                if let Some(tx) = &self.shutdown_tx {
                    let _ = tx.send(()).await;
                }
                DaemonResponse::Ok {
                    message: "Shutting down".to_string(),
                }
            }
        }
    }
}

impl Default for DaemonService {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal daemon state
#[derive(Debug, Default)]
struct DaemonState {
    is_playing: bool,
    is_paused: bool,
    current_track: Option<TrackInfo>,
    progress_ms: u32,
    shuffle: bool,
    repeat: String,
    volume: u8,
    queue: Vec<String>,
    track_history: Vec<String>,
}

impl DaemonState {
    fn advance_track(&mut self) {
        if let Some(ref track) = self.current_track {
            self.track_history.push(track.uri.clone());
        }
        
        if !self.queue.is_empty() {
            let next_uri = self.queue.remove(0);
            self.current_track = Some(TrackInfo {
                name: "Queued Track".to_string(),
                artists: vec!["Unknown".to_string()],
                album: "Unknown".to_string(),
                uri: next_uri,
                duration_ms: 180000,
            });
            self.is_playing = true;
            self.is_paused = false;
            self.progress_ms = 0;
        } else {
            self.current_track = None;
            self.is_playing = false;
            self.progress_ms = 0;
        }
    }

    fn rewind_track(&mut self) {
        if self.progress_ms > 5000 && self.current_track.is_some() {
            // If we're more than 5 seconds in, go to start of current track
            self.progress_ms = 0;
        } else if !self.track_history.is_empty() {
            // Go to previous track
            let prev_uri = self.track_history.pop().unwrap();
            self.current_track = Some(TrackInfo {
                name: "Previous Track".to_string(),
                artists: vec!["Unknown".to_string()],
                album: "Unknown".to_string(),
                uri: prev_uri,
                duration_ms: 180000,
            });
            self.progress_ms = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_config_default() {
        let config = DaemonConfig::default();
        assert!(config.socket_path.to_string_lossy().contains("daemon.sock"));
        assert!(config.pid_path.to_string_lossy().contains("daemon.pid"));
    }

    #[test]
    fn test_daemon_command_serialization() {
        let cmd = DaemonCommand::Play { uri: Some("spotify:track:abc".to_string()) };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("Play"));
        assert!(json.contains("spotify:track:abc"));

        let deserialized: DaemonCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, deserialized);
    }

    #[test]
    fn test_daemon_response_serialization() {
        let response = DaemonResponse::Ok { message: "Success".to_string() };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("ok"));
        assert!(json.contains("Success"));

        let deserialized: DaemonResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(response, deserialized);
    }

    #[test]
    fn test_playback_status_serialization() {
        let status = PlaybackStatus {
            is_playing: true,
            is_paused: false,
            track: Some(TrackInfo {
                name: "Test Track".to_string(),
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
            queue_length: 5,
        };

        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("is_playing"));
        assert!(json.contains("Test Track"));

        let deserialized: PlaybackStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(status, deserialized);
    }

    #[test]
    fn test_track_info_default() {
        let track = TrackInfo::default();
        assert_eq!(track.name, "");
        assert_eq!(track.artists, Vec::<String>::new());
        assert_eq!(track.duration_ms, 0);
    }

    #[test]
    fn test_daemon_command_variants() {
        // Test all command variants
        assert_eq!(DaemonCommand::Pause, DaemonCommand::Pause);
        assert_eq!(DaemonCommand::Next, DaemonCommand::Next);
        assert_eq!(DaemonCommand::Previous, DaemonCommand::Previous);
        assert_eq!(DaemonCommand::Ping, DaemonCommand::Ping);
        assert_eq!(DaemonCommand::Shutdown, DaemonCommand::Shutdown);
        
        let cmd1 = DaemonCommand::Volume { value: Some(50) };
        let cmd2 = DaemonCommand::Volume { value: Some(50) };
        assert_eq!(cmd1, cmd2);
    }

    #[test]
    fn test_daemon_response_variants() {
        let resp1 = DaemonResponse::Pong;
        let resp2 = DaemonResponse::Pong;
        assert_eq!(resp1, resp2);

        let resp3 = DaemonResponse::Error { message: "test".to_string() };
        let resp4 = DaemonResponse::Error { message: "test".to_string() };
        assert_eq!(resp3, resp4);
    }

    #[test]
    fn test_daemon_state_default() {
        let state = DaemonState::default();
        assert!(!state.is_playing);
        assert!(!state.is_paused);
        assert!(state.current_track.is_none());
        assert_eq!(state.progress_ms, 0);
        assert_eq!(state.volume, 0);
        assert!(state.queue.is_empty());
        assert!(state.track_history.is_empty());
    }

    #[test]
    fn test_daemon_state_advance_track() {
        let mut state = DaemonState::default();
        state.current_track = Some(TrackInfo {
            name: "Current".to_string(),
            artists: vec!["Artist".to_string()],
            album: "Album".to_string(),
            uri: "spotify:track:current".to_string(),
            duration_ms: 180000,
        });
        state.queue.push("spotify:track:next".to_string());
        
        state.advance_track();
        
        assert!(state.is_playing);
        assert!(!state.is_paused);
        assert_eq!(state.current_track.as_ref().unwrap().uri, "spotify:track:next");
        assert_eq!(state.progress_ms, 0);
        assert!(state.queue.is_empty());
    }

    #[test]
    fn test_daemon_state_rewind_track() {
        let mut state = DaemonState::default();
        state.current_track = Some(TrackInfo {
            name: "Current".to_string(),
            artists: vec!["Artist".to_string()],
            album: "Album".to_string(),
            uri: "spotify:track:current".to_string(),
            duration_ms: 180000,
        });
        state.progress_ms = 10000; // 10 seconds in
        state.track_history.push("spotify:track:prev".to_string());
        
        // Should go to start of current track since > 5s in
        state.rewind_track();
        assert_eq!(state.progress_ms, 0);
        assert_eq!(state.current_track.as_ref().unwrap().name, "Current");
        
        // Should go to previous track if at start
        state.progress_ms = 1000;
        state.rewind_track();
        assert_eq!(state.current_track.as_ref().unwrap().uri, "spotify:track:prev");
    }

    #[tokio::test]
    async fn test_daemon_service_creation() {
        let service = DaemonService::new();
        assert!(service.shutdown_tx.is_none());
    }

    #[tokio::test]
    async fn test_daemon_service_with_config() {
        let config = DaemonConfig::default();
        let service = DaemonService::with_config(config);
        assert!(service.shutdown_tx.is_none());
    }

    #[test]
    fn test_command_serialization_roundtrip() {
        let commands = vec![
            DaemonCommand::Play { uri: None },
            DaemonCommand::Play { uri: Some("test".to_string()) },
            DaemonCommand::Pause,
            DaemonCommand::Resume,
            DaemonCommand::PlayPause,
            DaemonCommand::Next,
            DaemonCommand::Previous,
            DaemonCommand::Stop,
            DaemonCommand::Status,
            DaemonCommand::Volume { value: None },
            DaemonCommand::Volume { value: Some(50) },
            DaemonCommand::Seek { position_ms: 1000 },
            DaemonCommand::SeekForward { duration_ms: 5000 },
            DaemonCommand::SeekBackward { duration_ms: 5000 },
            DaemonCommand::Shuffle { enabled: None },
            DaemonCommand::Shuffle { enabled: Some(true) },
            DaemonCommand::Repeat { mode: None },
            DaemonCommand::Repeat { mode: Some("track".to_string()) },
            DaemonCommand::Current,
            DaemonCommand::QueueAdd { uri: "test".to_string() },
            DaemonCommand::QueueClear,
            DaemonCommand::Ping,
            DaemonCommand::Shutdown,
        ];

        for cmd in commands {
            let json = serde_json::to_string(&cmd).unwrap();
            let deserialized: DaemonCommand = serde_json::from_str(&json).unwrap();
            assert_eq!(cmd, deserialized);
        }
    }

    #[test]
    fn test_response_serialization_roundtrip() {
        let responses = vec![
            DaemonResponse::Ok { message: "ok".to_string() },
            DaemonResponse::Error { message: "error".to_string() },
            DaemonResponse::Pong,
            DaemonResponse::Status {
                data: PlaybackStatus {
                    is_playing: true,
                    is_paused: false,
                    track: None,
                    progress_ms: 0,
                    duration_ms: 0,
                    shuffle: false,
                    repeat: "off".to_string(),
                    volume_percent: 50,
                    queue_length: 0,
                }
            },
            DaemonResponse::Track {
                data: TrackInfo {
                    name: "Test".to_string(),
                    artists: vec!["Artist".to_string()],
                    album: "Album".to_string(),
                    uri: "uri".to_string(),
                    duration_ms: 1000,
                }
            },
        ];

        for resp in responses {
            let json = serde_json::to_string(&resp).unwrap();
            let deserialized: DaemonResponse = serde_json::from_str(&json).unwrap();
            assert_eq!(resp, deserialized);
        }
    }
}
