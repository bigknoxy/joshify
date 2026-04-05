//! Spotify Connect receiver
//!
//! Makes joshify appear as a Spotify Connect device that can be
//! controlled from other Spotify apps (phone, desktop, web).

use anyhow::{Context, Result};
use librespot::{
    connect::{ConnectConfig, Spirc},
    core::{authentication::Credentials, session::Session},
    playback::{
        mixer::{self, Mixer, MixerConfig},
        player::Player,
    },
};
use std::sync::Arc;
use tokio::task::JoinHandle;

/// Spotify Connect manager - makes joshify discoverable as a playback device
pub struct ConnectManager {
    spirc: Option<Arc<Spirc>>,
    task_handle: Option<JoinHandle<()>>,
    device_name: String,
}

impl ConnectManager {
    /// Create a new Connect manager
    pub fn new(device_name: String) -> Self {
        Self {
            spirc: None,
            task_handle: None,
            device_name,
        }
    }

    /// Start the Spotify Connect receiver
    /// This makes joshify appear in the device list of other Spotify clients
    pub async fn start(
        &mut self,
        session: &Session,
        credentials: Credentials,
        player: Arc<Player>,
        mixer: Arc<dyn Mixer>,
    ) -> Result<()> {
        let connect_config = ConnectConfig {
            name: self.device_name.clone(),
            ..ConnectConfig::default()
        };

        let (spirc, spirc_task) =
            Spirc::new(connect_config, session.clone(), credentials, player, mixer)
                .await
                .context("Failed to create Spotify Connect receiver")?;

        let spirc = Arc::new(spirc);

        // Spawn the Spirc task to handle incoming Connect commands
        let handle = tokio::spawn(async move {
            tracing::info!("Spotify Connect receiver started");
            spirc_task.await;
            tracing::info!("Spotify Connect receiver stopped");
        });

        self.spirc = Some(spirc);
        self.task_handle = Some(handle);

        tracing::info!("Spotify Connect active as '{}'", self.device_name);

        Ok(())
    }

    /// Stop the Spotify Connect receiver
    pub fn stop(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
            tracing::info!("Spotify Connect receiver stopped");
        }
        self.spirc = None;
    }

    /// Check if Connect is active
    pub fn is_active(&self) -> bool {
        self.spirc.is_some()
    }

    /// Get the device name
    pub fn device_name(&self) -> &str {
        &self.device_name
    }
}

/// Create a mixer for the player
pub fn create_mixer() -> Result<Arc<dyn Mixer>> {
    let mixer_builder = mixer::find(None).context("No mixer available")?;
    let mixer_config = MixerConfig::default();
    mixer_builder(mixer_config).context("Failed to create mixer")
}

/// Get the default device name for Spotify Connect
pub fn default_device_name() -> String {
    let hostname = hostname::get()
        .ok()
        .and_then(|h: std::ffi::OsString| h.into_string().ok())
        .unwrap_or_else(|| "Unknown".to_string());
    format!("Joshify on {}", hostname)
}
