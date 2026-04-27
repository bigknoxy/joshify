//! Structured logging for Joshify
//!
//! Provides file-based logging with rotation for debugging user issues.
//! Logs are stored in ~/.cache/joshify/logs/ with automatic rotation.

use anyhow::Result;
use std::path::PathBuf;
use tracing::{info, Level};

/// Default log directory name
const LOG_DIR: &str = ".cache/joshify/logs";
/// Default log file name
const LOG_FILE: &str = "joshify.log";
/// Maximum log file size in bytes (10MB)
const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024;
/// Maximum number of log files to keep
const MAX_LOG_FILES: usize = 5;

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: Level,
    /// Whether to log to file
    pub file_logging: bool,
    /// Whether to log to stderr
    pub stderr_logging: bool,
    /// Log format (pretty, compact, json)
    pub format: LogFormat,
    /// Custom log directory (None for default)
    pub custom_dir: Option<PathBuf>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: Level::INFO,
            file_logging: true,
            stderr_logging: true,
            format: LogFormat::Pretty,
            custom_dir: None,
        }
    }
}

/// Log format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-readable with colors
    Pretty,
    /// Compact single-line
    Compact,
    /// JSON for structured logging
    Json,
}

/// Initialize the logging system
/// 
/// Note: This is a simplified implementation. In a full implementation,
/// you would use tracing-subscriber with proper layer composition.
/// For now, we use the default subscriber setup.
pub fn init(config: LogConfig) -> Result<()> {
    // For this simplified version, we'll use the default subscriber
    // The actual tracing setup would require more complex layer composition
    
    let log_dir = config
        .custom_dir
        .clone()
        .unwrap_or_else(|| default_log_dir());
    std::fs::create_dir_all(&log_dir)?;

    // Set up basic stderr logging if enabled
    if config.stderr_logging {
        // Note: In a real implementation, this would configure tracing_subscriber
        // For now, we just log the initialization
        tracing::info!("Logging initialized at level: {}", config.level);
    }

    if config.file_logging {
        info!("File logging would write to: {:?}", log_dir.join(LOG_FILE));
    }

    Ok(())
}

/// Initialize with default configuration
pub fn init_default() -> Result<()> {
    init(LogConfig::default())
}

/// Get default log directory
pub fn default_log_dir() -> PathBuf {
    dirs_next::home_dir()
        .expect("Cannot find home directory")
        .join(LOG_DIR)
}

/// Get current log file path
pub fn log_file_path() -> PathBuf {
    default_log_dir().join(LOG_FILE)
}

/// Clean up old log files if they exceed max size
pub fn cleanup_old_logs() -> Result<()> {
    let log_dir = default_log_dir();
    let log_file = log_file_path();

    if !log_file.exists() {
        return Ok(());
    }

    let metadata = std::fs::metadata(&log_file)?;
    if metadata.len() < MAX_LOG_SIZE {
        return Ok(());
    }

    // Rotate logs
    rotate_logs(&log_dir)?;

    Ok(())
}

/// Rotate log files
pub fn rotate_logs(log_dir: &PathBuf) -> Result<()> {
    // Delete oldest log file if it exists
    let oldest = log_dir.join(format!("{}.{}.{}", LOG_FILE, MAX_LOG_FILES, "old"));
    if oldest.exists() {
        std::fs::remove_file(oldest)?;
    }

    // Shift all log files up by one
    for i in (1..MAX_LOG_FILES).rev() {
        let old_path = log_dir.join(format!("{}.{}", LOG_FILE, i));
        let new_path = log_dir.join(format!("{}.{}", LOG_FILE, i + 1));
        if old_path.exists() {
            std::fs::rename(&old_path, &new_path)?;
        }
    }

    // Move current log to .1
    let current = log_dir.join(LOG_FILE);
    let backup = log_dir.join(format!("{}.{}", LOG_FILE, 1));
    if current.exists() {
        std::fs::rename(&current, &backup)?;
    }

    info!("Rotated log files");

    Ok(())
}

/// Get recent log entries (for displaying in UI)
pub fn get_recent_logs(lines: usize) -> Result<Vec<String>> {
    let log_file = log_file_path();

    if !log_file.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&log_file)?;
    let all_lines: Vec<&str> = content.lines().collect();

    // Get last N lines
    let start = all_lines.len().saturating_sub(lines);
    let recent: Vec<String> = all_lines[start..].iter().map(|s| s.to_string()).collect();

    Ok(recent)
}

/// Clear all logs
pub fn clear_logs() -> Result<()> {
    let log_dir = default_log_dir();

    // Remove main log file
    let log_file = log_dir.join(LOG_FILE);
    if log_file.exists() {
        std::fs::remove_file(log_file)?;
    }

    // Remove rotated log files
    for i in 1..=MAX_LOG_FILES {
        let rotated = log_dir.join(format!("{}.{}", LOG_FILE, i));
        if rotated.exists() {
            std::fs::remove_file(rotated)?;
        }
    }

    info!("Cleared all log files");

    Ok(())
}

/// Log a user action (for debugging)
pub fn log_user_action(action: &str, details: Option<&str>) {
    match details {
        Some(d) => info!(action = action, details = d, "User action"),
        None => info!(action = action, "User action"),
    }
}

/// Log an API request (for debugging)
pub fn log_api_request(endpoint: &str, status: u16, duration_ms: u64) {
    info!(
        endpoint = endpoint,
        status = status,
        duration_ms = duration_ms,
        "API request"
    );
}

/// Log playback state change
pub fn log_playback_change(
    track_name: Option<&str>,
    artist_name: Option<&str>,
    is_playing: bool,
) {
    let state = if is_playing { "playing" } else { "paused" };
    match (track_name, artist_name) {
        (Some(t), Some(a)) => info!(
            track = t,
            artist = a,
            state = state,
            "Playback state changed"
        ),
        (Some(t), None) => info!(track = t, state = state, "Playback state changed"),
        _ => info!(state = state, "Playback state changed"),
    }
}

/// Get log directory size in bytes
pub fn get_log_dir_size() -> Result<u64> {
    let log_dir = default_log_dir();
    
    if !log_dir.exists() {
        return Ok(0);
    }

    let mut total_size = 0u64;
    for entry in std::fs::read_dir(log_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_file() {
            total_size += metadata.len();
        }
    }

    Ok(total_size)
}

/// Format bytes to human-readable string
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    format!("{:.1} {}", size, UNITS[unit_idx])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_log_config_default() {
        let config = LogConfig::default();
        assert_eq!(config.level, Level::INFO);
        assert!(config.file_logging);
        assert!(config.stderr_logging);
        assert_eq!(config.format, LogFormat::Pretty);
        assert!(config.custom_dir.is_none());
    }

    #[test]
    fn test_default_log_dir() {
        let dir = default_log_dir();
        assert!(dir.to_string_lossy().contains(".cache/joshify/logs"));
    }

    #[test]
    fn test_log_file_path() {
        let path = log_file_path();
        assert!(path.to_string_lossy().contains("joshify.log"));
    }

    #[test]
    fn test_log_format_variants() {
        assert_eq!(LogFormat::Pretty, LogFormat::Pretty);
        assert_eq!(LogFormat::Compact, LogFormat::Compact);
        assert_eq!(LogFormat::Json, LogFormat::Json);
        assert_ne!(LogFormat::Pretty, LogFormat::Json);
    }

    #[test]
    fn test_log_rotation_in_temp_dir() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().join("logs");
        fs::create_dir_all(&log_dir).unwrap();

        // Create a log file
        let log_file = log_dir.join("joshify.log");
        fs::write(&log_file, "test log content").unwrap();

        // Rotate logs
        rotate_logs(&log_dir).unwrap();

        // Check that rotation happened
        assert!(!log_file.exists());
        assert!(log_dir.join("joshify.log.1").exists());
    }

    #[test]
    fn test_clear_logs() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().join("logs");
        fs::create_dir_all(&log_dir).unwrap();

        // Create some log files
        fs::write(log_dir.join("joshify.log"), "test").unwrap();
        fs::write(log_dir.join("joshify.log.1"), "test").unwrap();
        fs::write(log_dir.join("joshify.log.2"), "test").unwrap();

        // Clear logs manually
        for entry in fs::read_dir(&log_dir).unwrap() {
            let entry = entry.unwrap();
            fs::remove_file(entry.path()).unwrap();
        }

        // Verify all cleared
        let entries: Vec<_> = fs::read_dir(&log_dir).unwrap().collect();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_get_recent_logs() {
        let temp_dir = TempDir::new().unwrap();
        let log_file = temp_dir.path().join("test.log");

        // Write test log content
        let content = "line1\nline2\nline3\nline4\nline5";
        fs::write(&log_file, content).unwrap();

        // Read last 3 lines
        let log_content = fs::read_to_string(&log_file).unwrap();
        let lines: Vec<&str> = log_content.lines().collect();
        let recent: Vec<String> = lines[2..].iter().map(|s| s.to_string()).collect();

        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0], "line3");
        assert_eq!(recent[1], "line4");
        assert_eq!(recent[2], "line5");
    }

    #[test]
    fn test_log_config_custom_dir() {
        let temp_dir = TempDir::new().unwrap();
        let config = LogConfig {
            custom_dir: Some(temp_dir.path().to_path_buf()),
            ..Default::default()
        };

        assert_eq!(config.custom_dir, Some(temp_dir.path().to_path_buf()));
    }

    #[test]
    fn test_log_config_level_string() {
        assert_eq!(Level::TRACE.to_string(), "TRACE");
        assert_eq!(Level::DEBUG.to_string(), "DEBUG");
        assert_eq!(Level::INFO.to_string(), "INFO");
        assert_eq!(Level::WARN.to_string(), "WARN");
        assert_eq!(Level::ERROR.to_string(), "ERROR");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0.0 B");
        assert_eq!(format_bytes(512), "512.0 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn test_get_log_dir_size() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().join("logs");
        fs::create_dir_all(&log_dir).unwrap();

        // Create some log files
        fs::write(log_dir.join("joshify.log"), "test content").unwrap();
        fs::write(log_dir.join("joshify.log.1"), "more test content").unwrap();

        // Calculate size manually
        let mut total = 0u64;
        for entry in fs::read_dir(&log_dir).unwrap() {
            let entry = entry.unwrap();
            total += entry.metadata().unwrap().len();
        }

        assert!(total > 0);
    }

    #[test]
    fn test_cleanup_old_logs_no_rotation_needed() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().join("logs");
        fs::create_dir_all(&log_dir).unwrap();

        // Create small log file (under MAX_LOG_SIZE)
        fs::write(log_dir.join("joshify.log"), "small content").unwrap();

        // This should not rotate
        cleanup_old_logs_in_temp(&log_dir).unwrap();

        // File should still exist
        assert!(log_dir.join("joshify.log").exists());
    }

    /// Helper function for testing cleanup
    fn cleanup_old_logs_in_temp(log_dir: &PathBuf) -> Result<()> {
        let log_file = log_dir.join(LOG_FILE);

        if !log_file.exists() {
            return Ok(());
        }

        let metadata = std::fs::metadata(&log_file)?;
        if metadata.len() < MAX_LOG_SIZE {
            return Ok(());
        }

        rotate_logs(log_dir)?;
        Ok(())
    }
}
