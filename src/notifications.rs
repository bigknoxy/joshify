//! Desktop notifications for track changes
//!
//! Shows native OS notifications when tracks change, including
//! album art thumbnails when available.

use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::info;

/// Notification service for desktop notifications
pub struct NotificationService {
    /// Configuration
    config: NotificationConfig,
    /// Last notification time (for rate limiting)
    last_notification: Option<Instant>,
    /// Track ID of last notification (prevent duplicates)
    last_track_id: Option<String>,
    /// Whether the service is running
    running: bool,
    /// Platform-specific notifier
    platform_notifier: Option<Box<dyn PlatformNotifier>>,
}

/// Notification configuration
#[derive(Debug, Clone)]
pub struct NotificationConfig {
    /// Enable notifications
    pub enabled: bool,
    /// Show album art in notifications
    pub show_album_art: bool,
    /// Minimum seconds between notifications
    pub cooldown_secs: u64,
    /// Only show when window is not focused
    pub only_when_unfocused: bool,
    /// Notification timeout in seconds (0 for system default)
    pub timeout_secs: u64,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_album_art: true,
            cooldown_secs: 5,
            only_when_unfocused: false,
            timeout_secs: 5,
        }
    }
}

/// Track information for notifications
#[derive(Debug, Clone, Default)]
pub struct TrackInfo {
    /// Track ID
    pub track_id: Option<String>,
    /// Track name
    pub name: Option<String>,
    /// Artist names
    pub artists: Vec<String>,
    /// Album name
    pub album: Option<String>,
    /// Album art path or URL
    pub album_art: Option<String>,
    /// Duration in milliseconds
    pub duration_ms: Option<u32>,
}

impl TrackInfo {
    /// Create track info from track summary and context
    pub fn new(name: &str, artists: Vec<String>, album: Option<&str>) -> Self {
        Self {
            name: Some(name.to_string()),
            artists,
            album: album.map(|s| s.to_string()),
            ..Default::default()
        }
    }

    /// Get display title for notification
    pub fn title(&self) -> String {
        self.name.clone().unwrap_or_else(|| "Unknown Track".to_string())
    }

    /// Get display body for notification
    pub fn body(&self) -> String {
        let artists = if self.artists.is_empty() {
            "Unknown Artist".to_string()
        } else {
            self.artists.join(", ")
        };

        if let Some(ref album) = self.album {
            format!("{} — {}", artists, album)
        } else {
            artists
        }
    }

    /// Check if this is the same track as another
    pub fn is_same_track(&self, other: &TrackInfo) -> bool {
        match (&self.track_id, &other.track_id) {
            (Some(a), Some(b)) => a == b,
            _ => self.name == other.name && self.artists == other.artists,
        }
    }
}

/// Platform-specific notification trait
pub trait PlatformNotifier: Send + Sync {
    /// Show a notification
    fn notify(&self, title: &str, body: &str, icon: Option<&str>) -> Result<()>;
}

/// Stub platform notifier for when notifications are unavailable
pub struct StubNotifier;

impl PlatformNotifier for StubNotifier {
    fn notify(&self, _title: &str, _body: &str, _icon: Option<&str>) -> Result<()> {
        // Do nothing
        Ok(())
    }
}

impl NotificationService {
    /// Create a new notification service
    pub fn new(config: NotificationConfig) -> Self {
        let platform_notifier = Self::create_platform_notifier(&config);

        Self {
            config,
            last_notification: None,
            last_track_id: None,
            running: true,
            platform_notifier,
        }
    }

    /// Create platform-specific notifier
    fn create_platform_notifier(config: &NotificationConfig) -> Option<Box<dyn PlatformNotifier>> {
        if !config.enabled {
            return None;
        }

        #[cfg(target_os = "linux")]
        {
            // Try to create a Linux notifier using notify-rust
            info!("Creating Linux desktop notifier");
            // Would use notify-rust here
            // For now, return stub
            Some(Box::new(StubNotifier))
        }

        #[cfg(target_os = "macos")]
        {
            info!("Creating macOS desktop notifier");
            // Would use mac-notification-sys here
            Some(Box::new(StubNotifier))
        }

        #[cfg(target_os = "windows")]
        {
            info!("Creating Windows desktop notifier");
            // Would use winrt-notification here
            Some(Box::new(StubNotifier))
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            warn!("No platform notifier available for this OS");
            None
        }
    }

    /// Show a track change notification
    pub fn notify_track_change(&mut self, track: &TrackInfo) -> Result<()> {
        if !self.should_notify(track) {
            return Ok(());
        }

        // Check if we have a notifier
        let notifier = match &self.platform_notifier {
            Some(n) => n,
            None => {
                // Log but don't fail
                info!("Would show notification: {} — {}", track.title(), track.body());
                return Ok(());
            }
        };

        // Show the notification
        notifier.notify(
            &track.title(),
            &track.body(),
            track.album_art.as_deref(),
        )?;

        // Update rate limiting state
        self.last_notification = Some(Instant::now());
        self.last_track_id = track.track_id.clone();

        info!("Sent notification for track: {}", track.title());
        Ok(())
    }

    /// Check if we should send a notification
    fn should_notify(&self, track: &TrackInfo) -> bool {
        if !self.config.enabled {
            return false;
        }

        if !self.running {
            return false;
        }

        // Check for duplicate (same track)
        if let Some(ref last_id) = self.last_track_id {
            if let Some(ref track_id) = track.track_id {
                if last_id == track_id {
                    return false;
                }
            }
        }

        // Check rate limiting
        if let Some(last) = self.last_notification {
            let elapsed = last.elapsed();
            let cooldown = Duration::from_secs(self.config.cooldown_secs);
            if elapsed < cooldown {
                return false;
            }
        }

        true
    }

    /// Stop the notification service
    pub fn stop(&mut self) {
        info!("Stopping notification service");
        self.running = false;
    }

    /// Start the notification service
    pub fn start(&mut self) {
        info!("Starting notification service");
        self.running = true;
    }

    /// Check if the service is running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Update configuration
    pub fn update_config(&mut self, config: NotificationConfig) {
        // Recreate notifier if enabled status changed
        if config.enabled != self.config.enabled {
            self.platform_notifier = Self::create_platform_notifier(&config);
        }
        self.config = config;
    }

    /// Get current configuration
    pub fn config(&self) -> &NotificationConfig {
        &self.config
    }

    /// Reset rate limiting (for testing)
    pub fn reset_rate_limit(&mut self) {
        self.last_notification = None;
        self.last_track_id = None;
    }
}

impl Default for NotificationService {
    fn default() -> Self {
        Self::new(NotificationConfig::default())
    }
}

/// Thread-safe shared notification service
pub type SharedNotificationService = Arc<Mutex<NotificationService>>;

/// Create a shared notification service
pub fn create_service(config: NotificationConfig) -> SharedNotificationService {
    Arc::new(Mutex::new(NotificationService::new(config)))
}

/// Linux notification implementation (would use notify-rust)
#[cfg(target_os = "linux")]
pub mod linux {
    use super::*;

    /// Linux desktop notifier using notify-rust
    pub struct LinuxNotifier;

    impl PlatformNotifier for LinuxNotifier {
        fn notify(&self, _title: &str, _body: &str, _icon: Option<&str>) -> Result<()> {
            // Would use notify_rust::Notification here
            // Example:
            // notify_rust::Notification::new()
            //     .summary(_title)
            //     .body(_body)
            //     .icon(_icon.unwrap_or("music"))
            //     .timeout(Duration::from_secs(5))
            //     .show()?;
            info!("Linux notification: {} — {}", _title, _body);
            Ok(())
        }
    }
}

/// macOS notification implementation (would use mac-notification-sys)
#[cfg(target_os = "macos")]
pub mod macos {
    use super::*;

    /// macOS desktop notifier
    pub struct MacOSNotifier;

    impl PlatformNotifier for MacOSNotifier {
        fn notify(&self, title: &str, body: &str, icon: Option<&str>) -> Result<()> {
            // Would use mac_notification_sys here
            info!("macOS notification: {} — {}", title, body);
            Ok(())
        }
    }
}

/// Windows notification implementation (would use winrt-notification)
#[cfg(target_os = "windows")]
pub mod windows {
    use super::*;

    /// Windows desktop notifier
    pub struct WindowsNotifier;

    impl PlatformNotifier for WindowsNotifier {
        fn notify(&self, title: &str, body: &str, icon: Option<&str>) -> Result<()> {
            // Would use winrt_notification::Toast here
            info!("Windows notification: {} — {}", title, body);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_config_defaults() {
        let config = NotificationConfig::default();
        assert!(config.enabled);
        assert!(config.show_album_art);
        assert_eq!(config.cooldown_secs, 5);
        assert!(!config.only_when_unfocused);
        assert_eq!(config.timeout_secs, 5);
    }

    #[test]
    fn test_track_info_creation() {
        let track = TrackInfo::new(
            "Test Track",
            vec!["Artist 1".to_string(), "Artist 2".to_string()],
            Some("Test Album"),
        );

        assert_eq!(track.name, Some("Test Track".to_string()));
        assert_eq!(track.artists.len(), 2);
        assert_eq!(track.album, Some("Test Album".to_string()));
        assert_eq!(track.title(), "Test Track");
        assert_eq!(track.body(), "Artist 1, Artist 2 — Test Album");
    }

    #[test]
    fn test_track_info_no_album() {
        let track = TrackInfo::new(
            "Test Track",
            vec!["Artist 1".to_string()],
            None,
        );

        assert_eq!(track.body(), "Artist 1");
    }

    #[test]
    fn test_track_info_no_artists() {
        let track = TrackInfo::new(
            "Test Track",
            vec![],
            Some("Test Album"),
        );

        assert_eq!(track.body(), "Unknown Artist — Test Album");
    }

    #[test]
    fn test_track_info_default() {
        let track = TrackInfo::default();
        assert_eq!(track.title(), "Unknown Track");
        assert_eq!(track.body(), "Unknown Artist");
    }

    #[test]
    fn test_service_creation() {
        let service = NotificationService::default();
        assert!(service.is_running());
        assert!(service.config().enabled);
    }

    #[test]
    fn test_should_notify_enabled() {
        let mut service = NotificationService::default();
        service.config.enabled = false;

        let track = TrackInfo::new("Test", vec![], None);
        assert!(!service.should_notify(&track));
    }

    #[test]
    fn test_should_notify_running() {
        let mut service = NotificationService::default();
        service.stop();

        let track = TrackInfo::new("Test", vec![], None);
        assert!(!service.should_notify(&track));
    }

    #[test]
    fn test_should_notify_duplicate() {
        let mut service = NotificationService::default();
        service.reset_rate_limit();

        let track = TrackInfo {
            track_id: Some("track-123".to_string()),
            name: Some("Test Track".to_string()),
            ..Default::default()
        };

        // First notification should succeed
        assert!(service.should_notify(&track));

        // Simulate notification sent
        service.last_track_id = track.track_id.clone();

        // Duplicate should be blocked
        assert!(!service.should_notify(&track));
    }

    #[test]
    fn test_should_notify_rate_limit() {
        let mut config = NotificationConfig::default();
        config.cooldown_secs = 1; // Short cooldown for testing

        let mut service = NotificationService::new(config);
        service.reset_rate_limit();

        let track1 = TrackInfo {
            track_id: Some("track-1".to_string()),
            name: Some("Track 1".to_string()),
            ..Default::default()
        };

        let track2 = TrackInfo {
            track_id: Some("track-2".to_string()),
            name: Some("Track 2".to_string()),
            ..Default::default()
        };

        // First notification should succeed
        assert!(service.should_notify(&track1));
        service.last_notification = Some(Instant::now());
        service.last_track_id = track1.track_id.clone();

        // Second notification immediately should fail (rate limited)
        assert!(!service.should_notify(&track2));
    }

    #[test]
    fn test_service_start_stop() {
        let mut service = NotificationService::default();
        assert!(service.is_running());

        service.stop();
        assert!(!service.is_running());

        service.start();
        assert!(service.is_running());
    }

    #[test]
    fn test_service_config_update() {
        let mut service = NotificationService::default();
        assert!(service.config().enabled);

        let new_config = NotificationConfig {
            enabled: false,
            ..Default::default()
        };

        service.update_config(new_config);
        assert!(!service.config().enabled);
    }

    #[test]
    fn test_stub_notifier() {
        let notifier = StubNotifier;
        // Should not fail
        assert!(notifier.notify("Title", "Body", None).is_ok());
        assert!(notifier.notify("Title", "Body", Some("icon.png")).is_ok());
    }

    #[test]
    fn test_create_shared_service() {
        let service = create_service(NotificationConfig::default());
        let guard = service.lock().unwrap();
        assert!(guard.is_running());
    }

    #[test]
    fn test_track_is_same_track() {
        let track1 = TrackInfo {
            track_id: Some("track-123".to_string()),
            name: Some("Test".to_string()),
            artists: vec!["Artist".to_string()],
            ..Default::default()
        };

        let track2 = TrackInfo {
            track_id: Some("track-123".to_string()),
            name: Some("Different Name".to_string()),
            artists: vec!["Different Artist".to_string()],
            ..Default::default()
        };

        let track3 = TrackInfo {
            track_id: Some("track-456".to_string()),
            name: Some("Test".to_string()),
            artists: vec!["Artist".to_string()],
            ..Default::default()
        };

        // Same track ID means same track
        assert!(track1.is_same_track(&track2));

        // Different track ID means different track
        assert!(!track1.is_same_track(&track3));

        // No track ID - compare by name and artists
        let track_no_id1 = TrackInfo::new("Test", vec!["Artist".to_string()], None);
        let track_no_id2 = TrackInfo::new("Test", vec!["Artist".to_string()], None);
        let track_no_id3 = TrackInfo::new("Different", vec!["Artist".to_string()], None);

        assert!(track_no_id1.is_same_track(&track_no_id2));
        assert!(!track_no_id1.is_same_track(&track_no_id3));
    }

    #[test]
    fn test_notify_with_disabled_service() {
        let config = NotificationConfig {
            enabled: false,
            ..Default::default()
        };

        let mut service = NotificationService::new(config);
        service.reset_rate_limit();

        let track = TrackInfo::new("Test", vec![], None);

        // Should not fail, just do nothing
        assert!(service.notify_track_change(&track).is_ok());
    }

    #[tokio::test]
    async fn test_notification_rate_limit_cooldown() {
        let mut config = NotificationConfig::default();
        config.cooldown_secs = 0; // No cooldown

        let mut service = NotificationService::new(config);
        service.reset_rate_limit();

        let track = TrackInfo::new("Test", vec![], None);

        // Should be able to notify immediately
        assert!(service.should_notify(&track));
    }
}
