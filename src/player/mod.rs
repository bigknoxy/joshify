//! Audio player wrapper for librespot
//!
//! Provides a high-level interface for local Spotify playback
//! with event-driven updates for the TUI.

pub mod visualization;

use anyhow::{Context, Result};
use librespot::{
    core::{SpotifyId, SpotifyUri},
    metadata::audio::UniqueFields,
    playback::{
        audio_backend,
        config::{AudioFormat, PlayerConfig},
        mixer::{self, Mixer, MixerConfig},
        player::{Player, PlayerEvent},
    },
};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;

/// Playback state for the TUI
#[derive(Debug, Clone, Default)]
pub struct PlaybackState {
    pub is_playing: bool,
    pub current_track_uri: Option<String>,
    pub current_track_name: Option<String>,
    pub current_artist_name: Option<String>,
    pub progress_ms: u32,
    pub duration_ms: u32,
    pub volume: u16, // 0-65535
}

/// Convert a 0-100 volume percentage to the 0-65535 range used by librespot.
///
/// Arithmetic is done in `u32` to avoid overflow before the cast to `u16`.
/// Values above 100 are clamped.
pub fn percent_to_volume(percent: u32) -> u16 {
    (percent.min(100) * 65535 / 100) as u16
}

/// Extract a display artist from a librespot audio item.
///
/// The artist is present in every `TrackChanged` event and was simply never
/// read, so local playback showed a stale or empty artist (issue #58).
pub fn artist_from_unique_fields(fields: &UniqueFields) -> Option<String> {
    match fields {
        UniqueFields::Track { artists, .. } => artists.0.first().map(|artist| artist.name.clone()),
        // Local files carry a single free-form string rather than a list.
        UniqueFields::Local { artists, .. } => artists.clone(),
        // Podcasts have a show, not an artist; showing it beats showing nothing.
        UniqueFields::Episode { show_name, .. } => Some(show_name.clone()),
    }
}

/// Where local audio goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioOutput {
    /// librespot's default backend for this platform: CoreAudio on macOS,
    /// ALSA (through rodio) on Linux.
    Default,
    /// Raw PCM piped into a helper process. Used when ALSA has no default
    /// device but a PulseAudio server is reachable - the normal state of a WSL
    /// distribution, where WSLg provides Pulse and nobody installs the
    /// ALSA-to-Pulse plugin. Carries the shell command.
    Subprocess(String),
}

impl AudioOutput {
    /// A short label for the status bar.
    pub fn describe(&self) -> &'static str {
        match self {
            AudioOutput::Default => "local audio",
            AudioOutput::Subprocess(_) => "local audio via PulseAudio",
        }
    }
}

/// Result of checking whether audio can actually be played on this machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioProbe {
    /// A device was opened and closed successfully through this output.
    Available(AudioOutput),
    /// No device could be opened. Carries a human-readable reason.
    Unavailable(String),
}

/// The `pacat` invocation that matches what librespot emits: 44.1 kHz stereo
/// in the default [`AudioFormat`] (signed 16-bit little-endian).
pub const PACAT_COMMAND: &str =
    "pacat --playback --raw --format=s16le --rate=44100 --channels=2 --client-name=joshify --stream-name=Spotify";

/// If this machine has a reachable PulseAudio server and the `pacat` client,
/// the command to pipe audio into it; otherwise `None`.
///
/// Reachability is checked with `pactl info`, because `pacat` itself starts
/// happily and only dies once the first buffer is written - too late for a
/// probe to notice.
pub fn pulse_fallback_command() -> Option<String> {
    let hinted = std::env::var_os("PULSE_SERVER").is_some_and(|v| !v.is_empty())
        || std::env::var_os("XDG_RUNTIME_DIR")
            .map(|dir| std::path::Path::new(&dir).join("pulse/native").exists())
            .unwrap_or(false);
    if !hinted {
        return None;
    }
    let reachable = std::process::Command::new("pactl")
        .arg("info")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if !reachable {
        return None;
    }
    Some(PACAT_COMMAND.to_string())
}

/// Try to actually open the audio output device.
///
/// [`audio_backend::find`] only resolves a backend *by name* — it succeeds on a
/// machine with no working audio at all, because nothing touches a device until
/// the sink is started. That happens later, on the player's own audio thread,
/// where a failure never reaches the user and playback is simply silent.
///
/// Opening and immediately closing a sink here converts that silent failure
/// into something reportable. See issue #49. When the default backend has no
/// device but PulseAudio is reachable, the `pacat` subprocess output is tried
/// next, so WSL gets local playback instead of a "remote only" banner.
pub fn probe_audio_output() -> AudioProbe {
    probe_with_fallback(pulse_fallback_command().as_deref())
}

/// [`probe_audio_output`] with the fallback command supplied by the caller.
pub fn probe_with_fallback(fallback: Option<&str>) -> AudioProbe {
    let default_error = match try_open_sink(&AudioOutput::Default) {
        Ok(()) => return AudioProbe::Available(AudioOutput::Default),
        Err(reason) => reason,
    };
    let Some(command) = fallback else {
        return AudioProbe::Unavailable(default_error);
    };
    let output = AudioOutput::Subprocess(command.to_string());
    match try_open_sink(&output) {
        Ok(()) => AudioProbe::Available(output),
        Err(fallback_error) => AudioProbe::Unavailable(format!(
            "{default_error}; PulseAudio fallback failed: {fallback_error}"
        )),
    }
}

/// Resolve the backend and device string for an output.
fn backend_for(output: &AudioOutput) -> Option<(audio_backend::SinkBuilder, Option<String>)> {
    match output {
        AudioOutput::Default => audio_backend::find(None).map(|b| (b, None)),
        AudioOutput::Subprocess(command) => {
            audio_backend::find(Some("subprocess".to_string())).map(|b| (b, Some(command.clone())))
        }
    }
}

/// Open and immediately close a sink for `output`, reporting why it could not.
fn try_open_sink(output: &AudioOutput) -> Result<(), String> {
    use std::panic::{self, AssertUnwindSafe};

    let Some((backend, device)) = backend_for(output) else {
        return Err("no audio backend is compiled in for this platform".to_string());
    };

    // librespot's rodio backend panics rather than returning an error when it
    // is built on a machine with no output device (`unwrap()` on
    // `NoDeviceAvailable`). In normal operation that panic happens on the
    // player's audio thread, which is exactly why the failure is invisible.
    // Guard the probe so it reports instead of taking the process down.
    //
    // The default hook prints to stderr, which would appear through the TUI, so
    // it is muted for the duration. The panic hook is global and other threads
    // are already running by now, so mute only panics raised on *this* thread
    // and delegate everything else to the previous hook.
    let probing_thread = std::thread::current().id();
    let previous_hook = Arc::new(panic::take_hook());
    let delegate = Arc::clone(&previous_hook);
    panic::set_hook(Box::new(move |info| {
        if std::thread::current().id() != probing_thread {
            delegate(info);
        }
    }));

    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let mut sink = backend(device, AudioFormat::default());
        sink.start().map(|()| {
            // Best effort: we only care that the device opened.
            let _ = sink.stop();
        })
    }));

    // Put the original hook back. Dropping ours first releases the Arc clone it
    // holds, so the original can be unwrapped and reinstalled - reverting to
    // the default here instead would quietly discard whatever hook the caller
    // had installed (ratatui installs one to restore the terminal on panic).
    drop(panic::take_hook());
    match Arc::try_unwrap(previous_hook) {
        Ok(hook) => panic::set_hook(hook),
        Err(_) => debug_assert!(false, "probe panic hook outlived the probe"),
    }

    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err("no audio output device available".to_string()),
    }
}

/// Local audio player backed by librespot
pub struct LocalPlayer {
    player: Arc<Player>,
    mixer: Arc<dyn Mixer>,
    event_rx: Option<UnboundedReceiver<PlayerEvent>>,
    pub state: PlaybackState,
}

impl LocalPlayer {
    /// Create a new player that plays through `output` - the one the probe
    /// found working, so what the player opens is what was tested.
    pub fn new(session: &librespot::core::session::Session, output: &AudioOutput) -> Result<Self> {
        let (backend, device) = backend_for(output).context(
            "No audio backend available. Install ALSA (Linux) or ensure audio drivers are present.",
        )?;
        let mixer_builder = mixer::find(None).context("No mixer available")?;
        let mixer_config = MixerConfig::default();
        let mixer = mixer_builder(mixer_config).context("Failed to create mixer")?;

        // Emit PositionChanged once a second so the UI can show the REAL
        // playback position instead of guessing from wall-clock time (which
        // kept ticking after silent failures).
        let player_config = PlayerConfig {
            position_update_interval: Some(std::time::Duration::from_secs(1)),
            ..PlayerConfig::default()
        };
        let audio_format = AudioFormat::default();

        let player = Player::new(
            player_config,
            session.clone(),
            mixer.get_soft_volume(),
            move || backend(device.clone(), audio_format),
        );

        let event_rx = player.get_player_event_channel();

        Ok(Self {
            player,
            mixer,
            event_rx: Some(event_rx),
            state: PlaybackState::default(),
        })
    }

    /// Get the underlying librespot player for Spotify Connect
    pub fn player(&self) -> Arc<Player> {
        self.player.clone()
    }

    /// Get the mixer for Spotify Connect
    pub fn mixer(&self) -> Arc<dyn Mixer> {
        self.mixer.clone()
    }

    /// Load and optionally play a track by Spotify URI string
    pub fn load_uri(&self, uri: &str, start_playing: bool, position_ms: u32) -> Result<()> {
        let spotify_uri = Self::parse_uri(uri).context("Failed to parse Spotify URI")?;
        self.player.load(spotify_uri, start_playing, position_ms);
        Ok(())
    }

    /// Play the current track
    pub fn play(&self) {
        self.player.play();
    }

    /// Pause the current track
    pub fn pause(&self) {
        self.player.pause();
    }

    /// Stop playback
    pub fn stop(&self) {
        self.player.stop();
    }

    /// Seek to position in milliseconds
    pub fn seek(&self, position_ms: u32) {
        self.player.seek(position_ms);
    }

    /// Set volume (0-65535)
    pub fn set_volume(&self, volume: u16) {
        // `emit_volume_changed_event` only broadcasts a notification to
        // listeners; it does not move the mixer. On its own it made the
        // on-screen volume number change while the audio stayed exactly as loud
        // as before. Set the mixer first, then announce it.
        self.mixer.set_volume(volume);
        self.player.emit_volume_changed_event(volume);
    }

    /// Get the event channel for TUI updates
    pub fn take_event_channel(&mut self) -> Option<UnboundedReceiver<PlayerEvent>> {
        self.event_rx.take()
    }

    /// Update state from a player event
    pub fn handle_event(&mut self, event: PlayerEvent) {
        use PlayerEvent::*;
        match event {
            Playing {
                track_id,
                position_ms,
                ..
            } => {
                self.state.is_playing = true;
                self.state.current_track_uri = Some(track_id.to_uri());
                self.state.progress_ms = position_ms;
            }
            Paused {
                track_id,
                position_ms,
                ..
            } => {
                self.state.is_playing = false;
                self.state.current_track_uri = Some(track_id.to_uri());
                self.state.progress_ms = position_ms;
            }
            Stopped { .. } => {
                self.state.is_playing = false;
            }
            EndOfTrack { .. } => {
                self.state.is_playing = false;
            }
            TrackChanged { audio_item } => {
                self.state.current_track_name = Some(audio_item.name.clone());
                self.state.duration_ms = audio_item.duration_ms;
                self.state.current_track_uri = Some(audio_item.uri.clone());
                if let Some(artist) = artist_from_unique_fields(&audio_item.unique_fields) {
                    self.state.current_artist_name = Some(artist);
                }
            }
            VolumeChanged { volume } => {
                self.state.volume = volume;
            }
            Seeked { position_ms, .. } => {
                self.state.progress_ms = position_ms;
            }
            PositionChanged { position_ms, .. } | PositionCorrection { position_ms, .. } => {
                self.state.progress_ms = position_ms;
            }
            Loading {
                track_id,
                position_ms,
                ..
            } => {
                self.state.current_track_uri = Some(track_id.to_uri());
                self.state.progress_ms = position_ms;
            }
            _ => {}
        }
    }

    /// Parse a Spotify URI string into a SpotifyUri
    fn parse_uri(uri: &str) -> Result<SpotifyUri> {
        // Handle "spotify:track:BASE62ID" format
        if let Some(id) = uri.strip_prefix("spotify:track:") {
            let track_id = SpotifyId::from_base62(id).context("Invalid track ID format")?;
            return Ok(SpotifyUri::Track { id: track_id });
        }

        // Handle "spotify:episode:BASE62ID" format
        if let Some(id) = uri.strip_prefix("spotify:episode:") {
            let episode_id = SpotifyId::from_base62(id).context("Invalid episode ID format")?;
            return Ok(SpotifyUri::Episode { id: episode_id });
        }

        // Handle full URI format
        if uri.starts_with("spotify:") {
            return SpotifyUri::from_uri(uri).map_err(|e| anyhow::anyhow!(e));
        }

        // Assume it's a base62 track ID
        let track_id = SpotifyId::from_base62(uri).context("Invalid URI format")?;
        Ok(SpotifyUri::Track { id: track_id })
    }
}

/// Shared player type for use across the app
pub type SharedPlayer = Arc<LocalPlayer>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_state_defaults() {
        let state = PlaybackState::default();
        assert!(!state.is_playing);
        assert!(state.current_track_uri.is_none());
        assert!(state.current_track_name.is_none());
        assert!(state.current_artist_name.is_none());
        assert_eq!(state.progress_ms, 0);
        assert_eq!(state.duration_ms, 0);
        assert_eq!(state.volume, 0);
    }

    #[test]
    fn percent_to_volume_full_range() {
        assert_eq!(percent_to_volume(0), 0);
        assert_eq!(percent_to_volume(1), 655);
        assert_eq!(percent_to_volume(50), 32767);
        assert_eq!(percent_to_volume(100), 65535);
    }

    #[test]
    fn percent_to_volume_clamps_above_100() {
        assert_eq!(percent_to_volume(150), 65535);
        assert_eq!(percent_to_volume(200), 65535);
    }

    /// Regression for issue #10: scaling used `(percent as u16) * 65535`,
    /// overflowing for any percent >= 2. Sweep every input so a return to
    /// narrow integer math panics here under debug assertions.
    #[test]
    fn percent_to_volume_sweep_is_monotonic() {
        let mut prev = 0;
        for percent in 0..=500u32 {
            let vol = percent_to_volume(percent);
            assert!(vol >= prev, "volume must not decrease as percent rises");
            prev = vol;
        }
    }

    /// Regression for #58: the artist is present in every TrackChanged event
    /// and was never read, so local playback showed a stale or empty artist.
    #[test]
    fn artist_is_extracted_from_a_track() {
        use librespot::core::{SpotifyId, SpotifyUri};
        use librespot::metadata::artist::{ArtistRole, ArtistWithRole, ArtistsWithRole};

        let any_id = SpotifyUri::Artist {
            id: SpotifyId::from_raw(&[0u8; 16]).expect("16 zero bytes is a valid id"),
        };

        let fields = UniqueFields::Track {
            artists: ArtistsWithRole(vec![
                ArtistWithRole {
                    id: any_id.clone(),
                    name: "Kendrick Lamar".to_string(),
                    role: ArtistRole::ARTIST_ROLE_MAIN_ARTIST,
                },
                ArtistWithRole {
                    id: any_id,
                    name: "Someone Else".to_string(),
                    role: ArtistRole::ARTIST_ROLE_MAIN_ARTIST,
                },
            ]),
            album: "Not Like Us".to_string(),
            album_artists: vec![],
            popularity: 0,
            number: 1,
            disc_number: 1,
        };

        assert_eq!(
            artist_from_unique_fields(&fields),
            Some("Kendrick Lamar".to_string()),
            "should take the first credited artist"
        );
    }

    #[test]
    fn artist_is_none_when_a_track_credits_nobody() {
        use librespot::metadata::artist::ArtistsWithRole;

        let fields = UniqueFields::Track {
            artists: ArtistsWithRole(vec![]),
            album: "x".to_string(),
            album_artists: vec![],
            popularity: 0,
            number: 1,
            disc_number: 1,
        };
        assert_eq!(artist_from_unique_fields(&fields), None);
    }

    #[test]
    fn artist_is_extracted_from_a_local_file() {
        let fields = UniqueFields::Local {
            artists: Some("Some Artist".to_string()),
            album: None,
            album_artists: None,
            number: None,
            disc_number: None,
            path: std::path::PathBuf::from("/tmp/x.mp3"),
        };
        assert_eq!(
            artist_from_unique_fields(&fields),
            Some("Some Artist".to_string())
        );
    }

    #[test]
    fn parse_uri_track_uri_format() {
        let uri = LocalPlayer::parse_uri("spotify:track:4uLU6hMCjMI75M1A2tKUQC").unwrap();
        match uri {
            SpotifyUri::Track { id } => {
                assert_eq!(id.to_base62(), "4uLU6hMCjMI75M1A2tKUQC");
            }
            _ => panic!("Expected Track variant"),
        }
    }

    #[test]
    fn parse_uri_episode_uri_format() {
        let uri = LocalPlayer::parse_uri("spotify:episode:5Xt5DXGzch68nYYamXrNxZ").unwrap();
        match uri {
            SpotifyUri::Episode { id } => {
                assert_eq!(id.to_base62(), "5Xt5DXGzch68nYYamXrNxZ");
            }
            _ => panic!("Expected Episode variant"),
        }
    }

    #[test]
    fn parse_uri_base62_only() {
        let uri = LocalPlayer::parse_uri("4uLU6hMCjMI75M1A2tKUQC").unwrap();
        match uri {
            SpotifyUri::Track { id } => {
                assert_eq!(id.to_base62(), "4uLU6hMCjMI75M1A2tKUQC");
            }
            _ => panic!("Expected Track variant"),
        }
    }

    #[test]
    fn parse_uri_invalid_format() {
        let result = LocalPlayer::parse_uri("not-a-spotify-uri");
        assert!(result.is_err());
    }

    #[test]
    fn parse_uri_empty_string() {
        let result = LocalPlayer::parse_uri("");
        assert!(result.is_err());
    }

    #[test]
    fn parse_uri_show_uri_format() {
        let uri = LocalPlayer::parse_uri("spotify:show:4rOoJ6Egrf8K2IrywzwOMk").unwrap();
        match uri {
            SpotifyUri::Show { id } => {
                assert_eq!(id.to_base62(), "4rOoJ6Egrf8K2IrywzwOMk");
            }
            _ => panic!("Expected Show variant"),
        }
    }

    #[test]
    fn parse_uri_artist_uri_format() {
        let uri = LocalPlayer::parse_uri("spotify:artist:0TnOYISbd1XYRBk9myaseg").unwrap();
        match uri {
            SpotifyUri::Artist { id } => {
                assert_eq!(id.to_base62(), "0TnOYISbd1XYRBk9myaseg");
            }
            _ => panic!("Expected Artist variant"),
        }
    }

    #[test]
    fn parse_uri_album_uri_format() {
        let uri = LocalPlayer::parse_uri("spotify:album:6DEjYFkNZh67HP7R9PSZvv").unwrap();
        match uri {
            SpotifyUri::Album { id } => {
                assert_eq!(id.to_base62(), "6DEjYFkNZh67HP7R9PSZvv");
            }
            _ => panic!("Expected Album variant"),
        }
    }

    #[test]
    fn playback_state_is_playing_flag() {
        let mut state = PlaybackState::default();
        assert!(!state.is_playing);
        state.is_playing = true;
        assert!(state.is_playing);
        state.is_playing = false;
        assert!(!state.is_playing);
    }

    #[test]
    fn playback_state_volume_range() {
        let state = PlaybackState {
            volume: 0,
            ..Default::default()
        };
        assert_eq!(state.volume, 0);

        let state = PlaybackState {
            volume: 65535,
            ..Default::default()
        };
        assert_eq!(state.volume, 65535);

        let state = PlaybackState {
            volume: 32767,
            ..Default::default()
        };
        assert_eq!(state.volume, 32767);
    }

    #[test]
    fn playback_state_progress_bounds() {
        let state = PlaybackState {
            progress_ms: 0,
            duration_ms: 180000,
            ..Default::default()
        };
        assert_eq!(state.progress_ms, 0);
        assert!(state.progress_ms <= state.duration_ms);

        let state = PlaybackState {
            progress_ms: 180000,
            duration_ms: 180000,
            ..Default::default()
        };
        assert_eq!(state.progress_ms, state.duration_ms);
    }

    #[test]
    fn playback_state_clone_is_safe() {
        let state = PlaybackState {
            is_playing: true,
            current_track_uri: Some("spotify:track:abc".to_string()),
            current_track_name: Some("Test".to_string()),
            progress_ms: 50000,
            duration_ms: 200000,
            volume: 75,
            ..Default::default()
        };

        let cloned = state.clone();
        assert_eq!(state.is_playing, cloned.is_playing);
        assert_eq!(state.current_track_uri, cloned.current_track_uri);
        assert_eq!(state.progress_ms, cloned.progress_ms);
    }
}

#[cfg(test)]
mod audio_output_tests {
    use super::*;

    /// The fallback is only consulted when the default backend has no device.
    /// On a machine with working audio it must never displace the default; on
    /// one without (CI, WSL without the ALSA plugin) it must be what makes
    /// local playback available. `cat` stands in for `pacat`: it accepts PCM
    /// on stdin and exists everywhere.
    #[test]
    #[serial_test::serial(panic_hook)]
    fn fallback_is_used_exactly_when_the_default_has_no_device() {
        let alone = probe_with_fallback(None);
        let with_cat = probe_with_fallback(Some("cat"));
        match alone {
            AudioProbe::Available(AudioOutput::Default) => {
                assert_eq!(with_cat, AudioProbe::Available(AudioOutput::Default));
            }
            AudioProbe::Unavailable(_) => {
                assert_eq!(
                    with_cat,
                    AudioProbe::Available(AudioOutput::Subprocess("cat".to_string()))
                );
            }
            other => panic!("the default probe never yields a subprocess: {other:?}"),
        }
    }

    /// A fallback command that cannot be spawned is a failure with both
    /// reasons in it, not a silent success - the subprocess sink only fails at
    /// spawn time, so this is the one thing the probe can catch.
    #[test]
    #[serial_test::serial(panic_hook)]
    fn a_broken_fallback_is_reported_with_both_reasons() {
        if let AudioProbe::Available(_) = probe_with_fallback(None) {
            return; // the default works here; the fallback is never consulted
        }
        match probe_with_fallback(Some("/nonexistent/joshify-fake-pacat")) {
            AudioProbe::Unavailable(reason) => {
                assert!(reason.contains("PulseAudio fallback failed"), "{reason}");
            }
            other => panic!("a missing binary cannot be an available output: {other:?}"),
        }
    }

    #[test]
    fn pacat_command_matches_what_librespot_emits() {
        // librespot decodes to 44.1 kHz stereo and AudioFormat::default() is S16.
        assert_eq!(AudioFormat::default(), AudioFormat::S16);
        for needle in [
            "pacat",
            "--raw",
            "--format=s16le",
            "--rate=44100",
            "--channels=2",
        ] {
            assert!(PACAT_COMMAND.contains(needle), "missing {needle}");
        }
    }

    #[test]
    fn subprocess_output_is_named_for_the_status_bar() {
        assert_eq!(AudioOutput::Default.describe(), "local audio");
        assert!(AudioOutput::Subprocess("pacat".into())
            .describe()
            .contains("PulseAudio"));
    }
}
