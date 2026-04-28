//! Media control support (MPRIS on Linux, media keys on macOS/Windows)
//!
//! Allows controlling playback via system media keys and MPRIS-compatible
//! media players on Linux.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

/// Media control commands that can be sent from the OS
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaCommand {
    Play,
    Pause,
    PlayPause,
    Stop,
    Next,
    Previous,
    SeekForward,
    SeekBackward,
}

/// Callback type for media control events
pub type MediaControlCallback = Arc<dyn Fn(MediaCommand) + Send + Sync>;

/// Media control service that handles system media keys
pub struct MediaControlService {
    /// Channel for receiving media commands
    command_rx: mpsc::UnboundedReceiver<MediaCommand>,
    /// Whether the service is running
    running: bool,
}

impl MediaControlService {
    /// Create a new media control service
    pub fn new() -> (Self, mpsc::UnboundedSender<MediaCommand>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Self {
                command_rx: rx,
                running: false,
            },
            tx,
        )
    }

    /// Start the media control service
    ///
    /// Platform-specific implementations will register with the OS
    pub fn start(&mut self) -> Result<()> {
        if self.running {
            return Ok(());
        }

        info!("Starting media control service");

        #[cfg(target_os = "linux")]
        {
            // MPRIS support would be initialized here
            // For now, this is a stub that logs the intent
            info!("Media control: Linux MPRIS support initialized (stub)");
        }

        #[cfg(target_os = "macos")]
        {
            info!("Media control: macOS media keys support initialized (stub)");
        }

        #[cfg(target_os = "windows")]
        {
            info!("Media control: Windows media transport support initialized (stub)");
        }

        self.running = true;
        Ok(())
    }

    /// Stop the media control service
    pub fn stop(&mut self) {
        if !self.running {
            return;
        }

        info!("Stopping media control service");
        self.running = false;
    }

    /// Check if a command is available
    pub async fn recv(&mut self) -> Option<MediaCommand> {
        if !self.running {
            return None;
        }

        self.command_rx.recv().await
    }

    /// Try to receive a command without blocking
    pub fn try_recv(&mut self) -> Option<MediaCommand> {
        if !self.running {
            return None;
        }

        match self.command_rx.try_recv() {
            Ok(cmd) => Some(cmd),
            Err(_) => None,
        }
    }

    /// Check if the service is running
    pub fn is_running(&self) -> bool {
        self.running
    }
}

impl Default for MediaControlService {
    fn default() -> Self {
        let (service, _) = Self::new();
        service
    }
}

/// Media control configuration
#[derive(Debug, Clone)]
pub struct MediaControlConfig {
    /// Enable media control
    pub enabled: bool,
    /// Media player identity (for MPRIS)
    pub identity: String,
    /// Desktop entry name (for MPRIS)
    pub desktop_entry: Option<String>,
}

impl Default for MediaControlConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            identity: "Joshify".to_string(),
            desktop_entry: Some("joshify".to_string()),
        }
    }
}

/// Media control state for MPRIS
#[derive(Debug, Clone, Default)]
pub struct MediaControlState {
    /// Current playback status
    pub playback_status: PlaybackStatus,
    /// Current track metadata
    pub metadata: TrackMetadata,
    /// Current position in milliseconds
    pub position_ms: u64,
    /// Track duration in milliseconds
    pub duration_ms: u64,
    /// Volume (0.0 - 1.0)
    pub volume: f64,
    /// Can go next
    pub can_go_next: bool,
    /// Can go previous
    pub can_go_previous: bool,
    /// Can play
    pub can_play: bool,
    /// Can pause
    pub can_pause: bool,
    /// Can seek
    pub can_seek: bool,
}

impl MediaControlState {
    pub fn new() -> Self {
        Self {
            can_go_next: true,
            can_go_previous: true,
            can_play: true,
            can_pause: true,
            can_seek: true,
            ..Default::default()
        }
    }
}

/// Playback status for MPRIS
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackStatus {
    #[default]
    Stopped,
    Playing,
    Paused,
}

impl PlaybackStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlaybackStatus::Playing => "Playing",
            PlaybackStatus::Paused => "Paused",
            PlaybackStatus::Stopped => "Stopped",
        }
    }
}

/// Track metadata for MPRIS
#[derive(Debug, Clone, Default)]
pub struct TrackMetadata {
    /// Track ID (Spotify URI)
    pub track_id: Option<String>,
    /// Track name
    pub title: Option<String>,
    /// Artist names
    pub artists: Vec<String>,
    /// Album name
    pub album: Option<String>,
    /// Album art URL
    pub art_url: Option<String>,
    /// Track length in microseconds
    pub length_us: Option<u64>,
    /// Track number
    pub track_number: Option<u32>,
}

impl TrackMetadata {
    /// Convert to MPRIS metadata dictionary
    pub fn to_mpris_dict(&self) -> std::collections::HashMap<String, String> {
        let mut dict = std::collections::HashMap::new();

        if let Some(ref id) = self.track_id {
            dict.insert("mpris:trackid".to_string(), id.clone());
        }

        if let Some(ref title) = self.title {
            dict.insert("xesam:title".to_string(), title.clone());
        }

        if !self.artists.is_empty() {
            dict.insert("xesam:artist".to_string(), self.artists.join(", "));
        }

        if let Some(ref album) = self.album {
            dict.insert("xesam:album".to_string(), album.clone());
        }

        if let Some(length) = self.length_us {
            dict.insert("mpris:length".to_string(), length.to_string());
        }

        if let Some(ref art_url) = self.art_url {
            dict.insert("mpris:artUrl".to_string(), art_url.clone());
        }

        dict
    }
}

/// Stub for MPRIS implementation (Linux)
///
/// In a full implementation, this would use the dbus crate to register
/// an MPRIS2 compatible media player interface
#[cfg(target_os = "linux")]
pub mod linux {
    use super::*;

    /// Initialize MPRIS DBus interface
    pub fn init_mpris(
        _config: &MediaControlConfig,
        _command_tx: mpsc::UnboundedSender<MediaCommand>,
    ) -> Result<()> {
        // This would initialize the DBus connection and register
        // the org.mpris.MediaPlayer2.joshify interface
        info!("MPRIS: Would initialize DBus interface here");
        Ok(())
    }

    /// Update MPRIS state
    pub fn update_state(_state: &MediaControlState) {
        // This would update the MPRIS PropertiesChanged signal
    }
}

/// Stub for macOS media key support
#[cfg(target_os = "macos")]
pub mod macos {
    use super::*;

    /// Initialize media key monitoring
    pub fn init_media_keys(
        _command_tx: mpsc::UnboundedSender<MediaCommand>,
    ) -> Result<()> {
        info!("macOS: Would initialize media key monitoring here");
        Ok(())
    }
}

/// Stub for Windows media transport support
#[cfg(target_os = "windows")]
pub mod windows {
    use super::*;

    /// Initialize Windows media transport controls
    pub fn init_media_transport(
        _command_tx: mpsc::UnboundedSender<MediaCommand>,
    ) -> Result<()> {
        info!("Windows: Would initialize media transport controls here");
        Ok(())
    }
}

use std::sync::OnceLock;

/// Global media control instance
static MEDIA_CONTROL: OnceLock<std::sync::Mutex<MediaControlService>> = OnceLock::new();
static COMMAND_TX: OnceLock<mpsc::UnboundedSender<MediaCommand>> = OnceLock::new();

/// Initialize global media control
pub fn init() -> Result<mpsc::UnboundedSender<MediaCommand>> {
    let (mut service, tx) = MediaControlService::new();
    service.start()?;
    
    MEDIA_CONTROL.set(std::sync::Mutex::new(service))
        .map_err(|_| anyhow::anyhow!("Media control already initialized"))?;
    COMMAND_TX.set(tx.clone())
        .map_err(|_| anyhow::anyhow!("Media control already initialized"))?;

    Ok(tx)
}

/// Get global media control command sender
pub fn get_command_sender() -> Option<&'static mpsc::UnboundedSender<MediaCommand>> {
    COMMAND_TX.get()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_control_service_creation() {
        let (mut service, tx) = MediaControlService::new();
        assert!(!service.is_running());

        // Should be able to start
        service.start().unwrap();
        assert!(service.is_running());

        // Send a command
        tx.send(MediaCommand::Play).unwrap();

        // Should receive it
        let cmd = service.try_recv();
        assert_eq!(cmd, Some(MediaCommand::Play));

        // Can stop
        service.stop();
        assert!(!service.is_running());
    }

    #[test]
    fn test_media_command_variants() {
        let commands = vec![
            MediaCommand::Play,
            MediaCommand::Pause,
            MediaCommand::PlayPause,
            MediaCommand::Stop,
            MediaCommand::Next,
            MediaCommand::Previous,
            MediaCommand::SeekForward,
            MediaCommand::SeekBackward,
        ];

        for cmd in commands {
            // Each command should be cloneable and comparable
            let cloned = cmd;
            assert_eq!(cmd, cloned);
        }
    }

    #[test]
    fn test_media_control_state_defaults() {
        let state = MediaControlState::new();
        assert!(state.can_go_next);
        assert!(state.can_go_previous);
        assert!(state.can_play);
        assert!(state.can_pause);
        assert!(state.can_seek);
        assert_eq!(state.playback_status, PlaybackStatus::Stopped);
    }

    #[test]
    fn test_playback_status_as_str() {
        assert_eq!(PlaybackStatus::Playing.as_str(), "Playing");
        assert_eq!(PlaybackStatus::Paused.as_str(), "Paused");
        assert_eq!(PlaybackStatus::Stopped.as_str(), "Stopped");
    }

    #[test]
    fn test_track_metadata_to_mpris() {
        let metadata = TrackMetadata {
            track_id: Some("spotify:track:abc123".to_string()),
            title: Some("Test Track".to_string()),
            artists: vec!["Test Artist".to_string()],
            album: Some("Test Album".to_string()),
            art_url: Some("https://example.com/art.jpg".to_string()),
            length_us: Some(180000000),
            track_number: Some(1),
        };

        let dict = metadata.to_mpris_dict();
        assert_eq!(dict.get("mpris:trackid"), Some(&"spotify:track:abc123".to_string()));
        assert_eq!(dict.get("xesam:title"), Some(&"Test Track".to_string()));
        assert_eq!(dict.get("xesam:artist"), Some(&"Test Artist".to_string()));
        assert_eq!(dict.get("xesam:album"), Some(&"Test Album".to_string()));
        assert_eq!(dict.get("mpris:artUrl"), Some(&"https://example.com/art.jpg".to_string()));
        assert_eq!(dict.get("mpris:length"), Some(&"180000000".to_string()));
    }

    #[test]
    fn test_track_metadata_empty() {
        let metadata = TrackMetadata::default();
        let dict = metadata.to_mpris_dict();
        assert!(dict.is_empty());
    }

    #[test]
    fn test_media_control_config_defaults() {
        let config = MediaControlConfig::default();
        assert!(config.enabled);
        assert_eq!(config.identity, "Joshify");
        assert_eq!(config.desktop_entry, Some("joshify".to_string()));
    }

    #[tokio::test]
    async fn test_async_command_receive() {
        let (mut service, tx) = MediaControlService::new();
        service.start().unwrap();

        // Send command asynchronously
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            tx_clone.send(MediaCommand::Next).unwrap();
        });

        // Receive command
        let cmd = service.recv().await;
        assert_eq!(cmd, Some(MediaCommand::Next));
    }

    #[test]
    fn test_service_double_start() {
        let (mut service, _) = MediaControlService::new();

        // First start should succeed
        service.start().unwrap();
        assert!(service.is_running());

        // Second start should be a no-op (not fail)
        service.start().unwrap();
        assert!(service.is_running());
    }

    #[test]
    fn test_service_recv_when_stopped() {
        let (mut service, _) = MediaControlService::new();

        // Should return None when not running
        let cmd = service.try_recv();
        assert_eq!(cmd, None);
    }
}
