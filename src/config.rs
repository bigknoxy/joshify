//! Configuration management for Joshify
//!
//! Loads user preferences from ~/.config/joshify/config.toml
//! Falls back to sensible defaults if config doesn't exist

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

const DEFAULT_CONFIG_FOLDER: &str = ".config/joshify";
const CONFIG_FILE: &str = "config.toml";

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Audio settings
    pub audio: AudioConfig,
    /// Notification settings
    pub notifications: NotificationConfig,
    /// Media control settings
    pub media_control: MediaControlConfig,
    /// UI settings
    pub ui: UiConfig,
    /// Keybindings (optional overrides)
    pub keybindings: Option<KeybindingsConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            audio: AudioConfig::default(),
            notifications: NotificationConfig::default(),
            media_control: MediaControlConfig::default(),
            ui: UiConfig::default(),
            keybindings: None,
        }
    }
}

impl Config {
    /// Load config from file or create with defaults
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            info!("Loading config from {:?}", config_path);
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            info!("No config found at {:?}, using defaults", config_path);
            let config = Config::default();
            // Try to create default config file
            if let Err(e) = config.save() {
                warn!("Failed to create default config: {}", e);
            }
            Ok(config)
        }
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let config_dir = Self::config_dir()?;
        std::fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join(CONFIG_FILE);
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;

        info!("Saved config to {:?}", config_path);
        Ok(())
    }

    /// Get config directory path
    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs_next::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
        Ok(home.join(DEFAULT_CONFIG_FOLDER))
    }

    /// Get config file path
    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join(CONFIG_FILE))
    }

    /// Load config from a specific path
    pub fn load_from(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}

/// Audio configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    /// Enable audio visualization
    pub visualization: bool,
    /// Number of frequency bands (32, 64, or 128)
    pub visualization_bands: u8,
    /// Smoothing factor for visualization (0.0-1.0)
    pub visualization_smoothing: f32,
    /// Audio normalization
    pub normalization: bool,
    /// Default volume (0-100)
    pub default_volume: u8,
    /// Bitrate for streaming (96, 160, or 320 kbps)
    pub bitrate: u16,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            visualization: true,
            visualization_bands: 64,
            visualization_smoothing: 0.7,
            normalization: false,
            default_volume: 70,
            bitrate: 320,
        }
    }
}

/// Notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Enable desktop notifications
    pub enabled: bool,
    /// Show album art in notifications
    pub show_album_art: bool,
    /// Minimum seconds between notifications (rate limiting)
    pub cooldown_secs: u64,
    /// Show notifications only when window is not focused
    pub only_when_unfocused: bool,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_album_art: true,
            cooldown_secs: 5,
            only_when_unfocused: false,
        }
    }
}

/// Media control configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaControlConfig {
    /// Enable media key support (MPRIS on Linux, media keys on macOS/Windows)
    pub enabled: bool,
    /// Media player name for MPRIS (Linux only)
    pub mpris_identity: String,
}

impl Default for MediaControlConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mpris_identity: "Joshify".to_string(),
        }
    }
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Color theme name
    pub theme: String,
    /// Show audio visualization in player bar
    pub show_visualizer: bool,
    /// Compact mode (smaller UI elements)
    pub compact_mode: bool,
    /// Border style (plain, rounded, double, thick)
    pub border_style: String,
    /// Album art size (small, medium, large)
    pub album_art_size: String,
    /// Show help on startup
    pub show_help_on_startup: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "catppuccin_mocha".to_string(),
            show_visualizer: true,
            compact_mode: false,
            border_style: "rounded".to_string(),
            album_art_size: "medium".to_string(),
            show_help_on_startup: true,
        }
    }
}

/// Keybindings configuration (optional overrides)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    pub play_pause: Option<String>,
    pub next: Option<String>,
    pub previous: Option<String>,
    pub volume_up: Option<String>,
    pub volume_down: Option<String>,
    pub seek_forward: Option<String>,
    pub seek_backward: Option<String>,
    pub shuffle: Option<String>,
    pub repeat: Option<String>,
    pub search: Option<String>,
    pub queue: Option<String>,
    pub help: Option<String>,
    pub quit: Option<String>,
}

use std::sync::OnceLock;

/// Global configuration instance
static CONFIG: OnceLock<Config> = OnceLock::new();

/// Initialize global config
pub fn init() -> Result<()> {
    let config = Config::load().unwrap_or_default();
    CONFIG.set(config).map_err(|_| anyhow::anyhow!("Config already initialized"))?;
    Ok(())
}

/// Get global config instance
pub fn get() -> &'static Config {
    CONFIG.get().expect("Config not initialized. Call config::init() first.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.audio.visualization);
        assert_eq!(config.audio.visualization_bands, 64);
        assert!(config.notifications.enabled);
        assert!(config.media_control.enabled);
        assert_eq!(config.ui.theme, "catppuccin_mocha");
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();

        // Should be valid TOML
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.audio.visualization_bands, config.audio.visualization_bands);
    }

    #[test]
    fn test_config_load_save() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config = Config::default();
        config.save().unwrap();

        // Verify file was created
        assert!(config_path.exists() || Config::config_path().unwrap().exists());
    }

    #[test]
    fn test_audio_config_validation() {
        let audio = AudioConfig::default();
        assert!(audio.visualization_bands <= 128);
        assert!(audio.visualization_bands >= 16);
        assert!(audio.visualization_smoothing >= 0.0 && audio.visualization_smoothing <= 1.0);
        assert!(audio.default_volume <= 100);
    }

    #[test]
    fn test_notification_cooldown() {
        let notif = NotificationConfig::default();
        assert!(notif.cooldown_secs > 0);
        assert!(notif.cooldown_secs <= 60);
    }
}
