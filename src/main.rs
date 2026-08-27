use anyhow::Result;
use joshify::auth::OAuthConfig;
use joshify::playback::domain::PlaybackContext;
use joshify::playback::PlaybackMode;
use joshify::player::LocalPlayer;
use joshify::session::LocalSession;
use joshify::state::app_state::{
    AlbumListItem, ArtistListItem, LibraryTab, PlaylistListItem, TrackListItem,
};
use joshify::state::player_state::PlayerState;
use joshify::state::search_state::SearchState;
use joshify::state::{ContentState, FocusTarget, LoadAction, NavItem};
use joshify::CliArgs;
use librespot::core::authentication::Credentials;
use std::sync::Arc;

/// Minimum interval between album-art fetch attempts (per app, not per URL).
const ART_FETCH_COOLDOWN_MS: u64 = 2000;

/// Advance local playback: user queue first, then context tracks, then stop.
///
/// Shared by the `EndOfTrack` handler and the explicit next-track key so both
/// paths behave identically.
fn advance_local_playback(app: &mut App) {
    app.push_local_history();
    if !app.queue_state.local_queue.is_empty() {
        tracing::info!(
            "EndOfTrack: Found {} items in user queue, advancing",
            app.queue_state.local_queue.len()
        );
        if let Some(next_entry) = app.queue_state.next_track() {
            if let Some(ref player) = app.local_player {
                match player.load_uri(&next_entry.uri, true, 0) {
                    Ok(_) => {
                        app.player_state.current_track_name = Some(next_entry.name.clone());
                        app.player_state.current_artist_name = Some(next_entry.artist.clone());
                        app.player_state.current_track_uri = Some(next_entry.uri.clone());
                        app.player_state.is_playing = true;
                        app.player_state.progress_ms = 0;
                        app.status_message =
                            Some(format!("Playing next from queue: {}", next_entry.name));
                        tracing::info!("Auto-advanced to user queue item: {}", next_entry.name);
                    }
                    Err(e) => {
                        app.status_message = Some(format!("Queue playback error: {}", e));
                        tracing::warn!("Queue playback failed: {}", e);
                    }
                }
            }
        }
    }
    // PHASE 2: Check context tracks if user queue is empty
    else if app.queue_state.playback_queue().remaining_context_tracks() > 0 {
        let remaining = app.queue_state.playback_queue().remaining_context_tracks();
        tracing::info!(
            "EndOfTrack: User queue empty, {} context tracks remaining. Advancing...",
            remaining
        );

        // Advance to next context track
        if let Some(next_uri) = app.queue_state.playback_queue_mut().advance() {
            tracing::info!("EndOfTrack: Advancing to next context track: {}", next_uri);

            if let Some(ref player) = app.local_player {
                match player.load_uri(&next_uri, true, 0) {
                    Ok(_) => {
                        // Try to get track info from the content state
                        // Look the next track up by URI. This used to copy the
                        // name and artist already in player_state - which still
                        // held the track that just ended - so the bar and the
                        // status line named the wrong song.
                        let (track_name, artist_name) = app
                            .context_track_meta
                            .get(&next_uri)
                            .cloned()
                            .unwrap_or_else(|| ("Unknown".to_string(), "Unknown".to_string()));
                        app.player_state.current_track_name = Some(track_name.clone());
                        app.player_state.current_artist_name = Some(artist_name.clone());
                        app.player_state.current_track_uri = Some(next_uri.clone());
                        app.player_state.is_playing = true;
                        app.player_state.progress_ms = 0;
                        app.status_message = Some(format!(
                            "Playing next from playlist: {} - {}",
                            track_name, artist_name
                        ));
                        tracing::info!(
                            "Auto-advanced to context track: {} ({} remaining)",
                            next_uri,
                            app.queue_state.playback_queue().remaining_context_tracks()
                        );
                    }
                    Err(e) => {
                        app.status_message = Some(format!("Context playback error: {}", e));
                        tracing::warn!("Failed to load next context track {}: {}", next_uri, e);
                    }
                }
            }
        } else {
            tracing::warn!(
                "EndOfTrack: advance() returned None despite {} remaining tracks",
                remaining
            );
        }
    }
    // PHASE 3: Nothing left to play
    else {
        tracing::info!("EndOfTrack: No more tracks to play (queue empty, context exhausted)");
        app.status_message = Some("Playback ended".to_string());
        // Actually stop audio — the UI says playback ended, so silence must
        // agree (previously the current track kept playing underneath).
        if let Some(ref player) = app.local_player {
            player.stop();
        }
        app.player_state.is_playing = false;
    }
}

/// Highlighted item in the current view (for queue operations)
#[derive(Debug, Clone)]
struct HighlightedItem {
    uri: String,
    name: String,
    artist: String,
    _context: Option<PlaybackContext>,
}

impl App {
    /// Record the currently playing track on the local previous-history stack.
    fn push_local_history(&mut self) {
        if let Some(uri) = self.player_state.current_track_uri.clone() {
            if self.local_history.last() != Some(&uri) {
                self.local_history.push(uri);
                if self.local_history.len() > 50 {
                    self.local_history.remove(0);
                }
            }
        }
    }
}

/// Turn Spotify tracks into radio queue entries, skipping anything already
/// queued or currently playing.
///
/// Marked `is_recommendation` so toggling radio back off can drop exactly these
/// and leave tracks the user queued by hand alone.
fn radio_entries_from(
    tracks: &[rspotify::model::FullTrack],
    exclude_uris: &std::collections::HashSet<String>,
) -> Vec<joshify::state::queue_state::QueueEntry> {
    tracks
        .iter()
        .filter_map(|t| {
            let id = t.id.as_ref()?;
            let uri = format!("spotify:track:{}", id.id());
            if exclude_uris.contains(&uri) {
                return None;
            }
            Some(joshify::state::queue_state::QueueEntry {
                uri,
                name: t.name.clone(),
                artist: t
                    .artists
                    .first()
                    .map(|a| a.name.clone())
                    .unwrap_or_else(|| "Unknown Artist".to_string()),
                added_by_user: false,
                is_recommendation: true,
            })
        })
        .collect()
}

/// Convert a click position on the progress bar (0-100) into a track offset.
///
/// Saturating throughout: a zero-length track (nothing loaded yet) must seek to
/// 0 rather than divide by zero, and a percentage above 100 is clamped instead
/// of running past the end of the track.
fn position_from_percent(percent: u8, duration_ms: u32) -> u32 {
    let percent = percent.min(100) as u64;
    ((duration_ms as u64 * percent) / 100).min(duration_ms as u64) as u32
}

/// Result of a background playback request, reported back to the UI loop.
///
/// Remote commands are fire-and-forget on a spawned task, so the UI cannot know
/// at keypress time whether Spotify accepted them. Without this the status bar
/// claimed success for requests that failed.
#[derive(Debug)]
enum PlaybackFeedback {
    Started {
        name: String,
        artist: String,
        uri: String,
    },
    Failed(String),
    /// A transport command failed. `revert` restores whatever the UI changed
    /// optimistically before issuing it, so the display cannot keep asserting
    /// a state Spotify rejected.
    CommandFailed {
        message: String,
        revert: Revert,
    },
    /// A device transfer settled. Until this arrived the UI announced
    /// "Switching to X..." and flipped to remote mode whether or not Spotify
    /// accepted the transfer.
    Transferred {
        device_name: String,
        error: Option<String>,
    },
}

/// UI state to put back when a transport command is refused.
#[derive(Debug, Clone, Copy)]
enum Revert {
    Nothing,
    Shuffle(bool),
    Repeat(joshify::state::player_state::RepeatMode),
    Volume(u32),
}

/// A transport command issued against the remote device.
#[derive(Debug, Clone, Copy)]
enum RemoteCommand {
    Pause,
    Resume,
    Next,
    Previous,
    Seek(u32),
    Volume(u32),
    Shuffle(bool),
    Repeat(rspotify::model::RepeatState),
}

impl RemoteCommand {
    /// How to name this command when telling the user it failed.
    fn describe(self) -> &'static str {
        match self {
            Self::Pause => "Pause",
            Self::Resume => "Resume",
            Self::Next => "Next track",
            Self::Previous => "Previous track",
            Self::Seek(_) => "Seek",
            Self::Volume(_) => "Volume",
            Self::Shuffle(_) => "Shuffle",
            Self::Repeat(_) => "Repeat",
        }
    }
}

/// Issue a transport command against the device the user picked, and report a
/// refusal instead of dropping it.
///
/// Every one of these used to be `let _ = guard.<cmd>().await` with no device
/// id, so with no active Spotify device the key was simply dead: no music, no
/// message, nothing in the UI to explain it.
fn spawn_remote_command(
    client: &Arc<Mutex<joshify::api::SpotifyClient>>,
    command: RemoteCommand,
    preferred_id: Option<String>,
    revert: Revert,
    tx_feedback: tokio::sync::mpsc::Sender<PlaybackFeedback>,
) {
    let c = client.clone();
    tokio::spawn(async move {
        let guard = c.lock().await;

        // Best effort: if the device lookup itself fails, fall through with
        // None and let Spotify apply the command to whatever is active. That
        // is still better than refusing to send it.
        let device = guard
            .device_to_play_on(preferred_id.as_deref())
            .await
            .ok()
            .flatten();
        let device_id = device.as_ref().and_then(|d| d.id.as_deref());

        let result = match command {
            RemoteCommand::Pause => guard.playback_pause(device_id).await,
            RemoteCommand::Resume => guard.playback_resume(device_id).await,
            RemoteCommand::Next => guard.playback_next(device_id).await,
            RemoteCommand::Previous => guard.playback_previous(device_id).await,
            RemoteCommand::Seek(pos) => guard.seek(pos, device_id).await,
            RemoteCommand::Volume(v) => guard.set_volume(v, device_id).await,
            RemoteCommand::Shuffle(on) => guard.toggle_shuffle(on, device_id).await,
            RemoteCommand::Repeat(state) => guard.set_repeat(state, device_id).await,
        };

        if let Err(e) = result {
            tracing::warn!("{} failed: {}", command.describe(), e);
            let _ = tx_feedback
                .send(PlaybackFeedback::CommandFailed {
                    message: format!("{} failed: {}", command.describe(), e),
                    revert,
                })
                .await;
        }
    });
}

/// Start remote playback of `track` (name, artist, uri) on a spawned task.
///
/// `context_uri` is an optional `spotify:playlist:...` to play the track
/// within. `preferred_id` is the device the user picked with 'd', if any - it
/// is used when it is still online, otherwise we fall back to whatever Spotify
/// reports as active. Success/failure is reported on `tx_feedback`.
fn spawn_remote_play(
    client: &Arc<Mutex<joshify::api::SpotifyClient>>,
    track: (String, String, String),
    context_uri: Option<String>,
    preferred_id: Option<String>,
    tx_feedback: tokio::sync::mpsc::Sender<PlaybackFeedback>,
) {
    let c = client.clone();
    let (name, artist, uri) = track;
    tokio::spawn(async move {
        let guard = c.lock().await;

        let device = match guard.device_to_play_on(preferred_id.as_deref()).await {
            Ok(Some(device)) => device,
            Ok(None) => {
                let _ = tx_feedback
                    .send(PlaybackFeedback::Failed(
                        "No Spotify device available - open Spotify somewhere, then press 'd' to pick a device".to_string(),
                    ))
                    .await;
                return;
            }
            Err(e) => {
                let _ = tx_feedback
                    .send(PlaybackFeedback::Failed(format!(
                        "Could not list Spotify devices: {}",
                        e
                    )))
                    .await;
                return;
            }
        };
        let device_id = match device.id.clone() {
            Some(id) => id,
            None => {
                let _ = tx_feedback
                    .send(PlaybackFeedback::Failed(format!(
                        "Device '{}' cannot be controlled remotely",
                        device.name
                    )))
                    .await;
                return;
            }
        };

        // Only transfer when the device is idle - transferring to the already
        // active device races with the play command that follows.
        if !device.is_active {
            if let Err(e) = guard.transfer_playback(&device_id).await {
                tracing::warn!("Transfer to {} failed: {}", device.name, e);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        let playlist_id = context_uri
            .as_deref()
            .and_then(|c| c.strip_prefix("spotify:playlist:"))
            .and_then(|id| rspotify::model::PlaylistId::from_id(id).ok())
            .map(|id| id.into_static());

        let result = match playlist_id {
            Some(playlist_id) => {
                tracing::info!(
                    "Remote playback: playlist={} track={} device={}",
                    playlist_id.id(),
                    uri,
                    device.name
                );
                guard
                    .start_context_playback_on(
                        rspotify::model::PlayContextId::from(playlist_id),
                        Some(rspotify::model::Offset::Uri(uri.clone())),
                        Some(&device_id),
                    )
                    .await
            }
            None => {
                tracing::info!("Remote playback: track={} device={}", uri, device.name);
                guard
                    .start_playback_on(vec![uri.clone()], None, Some(&device_id))
                    .await
            }
        };

        let feedback = match result {
            Ok(()) => PlaybackFeedback::Started { name, artist, uri },
            Err(e) => {
                PlaybackFeedback::Failed(format!("Playback failed on {}: {}", device.name, e))
            }
        };
        let _ = tx_feedback.send(feedback).await;
    });
}

/// What happened when the user asked for a track to play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayOutcome {
    /// The local player accepted the track; the player bar already reflects it.
    StartedLocally,
    /// The request went to Spotify on a spawned task; the result arrives as
    /// `PlaybackFeedback`.
    DispatchedRemotely,
    /// Nothing is playing and the status message says why.
    NotStarted,
}

/// The one way to start a track, wherever the user picked it.
///
/// Playback happens on this machine by default. Only when the user has chosen
/// another device with 'd' - which is what puts the app in remote mode - does
/// the track go to Spotify's API instead. 0.8.3 shipped one Enter handler
/// (search results) that skipped this decision and went remote unconditionally,
/// telling local users to go pick a device; `play_path_invariants` pins every
/// caller to this function so that cannot recur.
fn play_track(
    app: &mut App,
    client: Option<&Arc<Mutex<joshify::api::SpotifyClient>>>,
    track: (String, String, String),
    context_uri: Option<String>,
    tx_feedback: &tokio::sync::mpsc::Sender<PlaybackFeedback>,
) -> PlayOutcome {
    let (name, artist, uri) = track;
    if app.playback_mode == PlaybackMode::Local {
        if app.play_locally(&name, &artist, &uri) {
            PlayOutcome::StartedLocally
        } else {
            PlayOutcome::NotStarted
        }
    } else if let Some(client) = client {
        app.status_message = Some(format!("Starting: {}", name));
        spawn_remote_play(
            client,
            (name, artist, uri),
            context_uri,
            app.selected_device_id.clone(),
            tx_feedback.clone(),
        );
        PlayOutcome::DispatchedRemotely
    } else {
        app.status_message = Some("Not connected to Spotify".to_string());
        PlayOutcome::NotStarted
    }
}

/// Application state
struct App {
    selected_nav: NavItem,
    is_authenticated: bool,
    player_state: PlayerState,
    queue_state: joshify::state::queue_state::QueueState,
    /// Previously played track URIs for local `p` (previous) — newest last.
    local_history: Vec<String>,
    highlighted_item: Option<HighlightedItem>,
    current_context: Option<PlaybackContext>,
    status_message: Option<String>,
    last_poll_ms: u64,
    poll_interval_ms: u64,
    last_progress_tick_ms: u64,
    last_frame_time_ms: u64,
    last_art_fetch_ms: u64,
    event_batch: Vec<librespot::playback::player::PlayerEvent>,
    focus: FocusTarget,
    show_queue: bool,
    help_state: Option<joshify::ui::HelpOverlayState>,
    help_content: Option<joshify::ui::HelpContent>,
    area: Option<Rect>,
    content_state: ContentState,
    selected_index: usize,
    scroll_offset: usize,
    search_state: SearchState,
    album_art_cache: joshify::album_art::AlbumArtCache,
    last_fetched_art_uri: Option<String>,
    playback_mode: PlaybackMode,
    /// Remote device chosen with 'd'; commands target it explicitly.
    /// `None` means "whatever Spotify says is active".
    selected_device_id: Option<String>,
    /// Cursor in the queue overlay. The overlay advertised Enter/D/arrow keys
    /// while having no selection at all, so those keys acted on the main list's
    /// highlight (or did nothing).
    queue_selected_index: usize,
    /// uri -> (track name, artist) for the tracks in the current context.
    ///
    /// The playback queue holds URIs only, so auto-advance had no name to show
    /// and reused whatever was already on screen - announcing the track that
    /// just *ended* as the one now playing.
    context_track_meta: std::collections::HashMap<String, (String, String)>,
    local_session: Option<Arc<LocalSession>>,
    local_player: Option<Arc<LocalPlayer>>,
    player_event_rx:
        Option<tokio::sync::mpsc::UnboundedReceiver<librespot::playback::player::PlayerEvent>>,
    loading_more_liked_songs: bool,
    /// Layout cache for mouse hit testing
    layout_cache: joshify::ui::LayoutCache,
    /// Mouse state for tracking double-clicks
    mouse_state: joshify::ui::MouseState,
    /// Navigation stack for drill-down browsing
    nav_stack: joshify::state::navigation_stack::NavigationStack,
}

impl App {
    /// Start `uri` on this machine's player and mirror it into the player bar.
    ///
    /// This is what Enter means in local mode, wherever the track came from.
    /// Returns `false` when there is no local player; the status message says
    /// so either way, so the caller has nothing to add.
    fn play_locally(&mut self, name: &str, artist: &str, uri: &str) -> bool {
        let Some(player) = self.local_player.as_ref() else {
            self.status_message = Some("Local player not initialized".to_string());
            return false;
        };
        match player.load_uri(uri, true, 0) {
            Ok(()) => {
                // Remember what was playing so local `p` (previous) can return
                // to it, whichever list the new track came from.
                if let Some(previous) = self.player_state.current_track_uri.clone() {
                    if self.local_history.last() != Some(&previous) {
                        self.local_history.push(previous);
                        if self.local_history.len() > 50 {
                            self.local_history.remove(0);
                        }
                    }
                }
                self.player_state.current_track_name = Some(name.to_string());
                self.player_state.current_artist_name = Some(artist.to_string());
                self.player_state.current_track_uri = Some(uri.to_string());
                self.player_state.is_playing = true;
                self.player_state.progress_ms = 0;
                self.player_state.reset_scroll();
                self.status_message = Some(format!("Playing locally: {}", name));
                true
            }
            Err(e) => {
                self.status_message = Some(format!("Local playback error: {}", e));
                false
            }
        }
    }

    fn new() -> Self {
        Self {
            selected_nav: NavItem::Home,
            is_authenticated: false,
            player_state: PlayerState::default(),
            queue_state: joshify::state::queue_state::QueueState::new(),
            highlighted_item: None,
            current_context: None,
            status_message: None,
            last_poll_ms: 0,
            poll_interval_ms: 2000,
            last_progress_tick_ms: 0,
            last_frame_time_ms: 0,
            last_art_fetch_ms: 0,
            event_batch: Vec::with_capacity(32),
            focus: FocusTarget::Sidebar,
            show_queue: false,
            help_state: None,
            help_content: None,
            area: None,
            content_state: ContentState::Home,
            selected_index: 0,
            scroll_offset: 0,
            search_state: SearchState::new(),
            album_art_cache: joshify::album_art::AlbumArtCache::new(),
            last_fetched_art_uri: None,
            playback_mode: PlaybackMode::Local,
            selected_device_id: None,
            queue_selected_index: 0,
            context_track_meta: std::collections::HashMap::new(),
            local_session: None,
            local_player: None,
            local_history: Vec::new(),
            player_event_rx: None,
            loading_more_liked_songs: false,
            layout_cache: joshify::ui::LayoutCache::new(),
            mouse_state: joshify::ui::MouseState::new(),
            nav_stack: joshify::state::navigation_stack::NavigationStack::new(),
        }
    }

    fn focus_next(&mut self) {
        self.focus = match self.focus {
            FocusTarget::Sidebar => FocusTarget::MainContent,
            FocusTarget::MainContent => FocusTarget::PlayerBar,
            FocusTarget::PlayerBar => FocusTarget::Sidebar,
        };
    }

    fn focus_previous(&mut self) {
        self.focus = match self.focus {
            FocusTarget::Sidebar => FocusTarget::PlayerBar,
            FocusTarget::MainContent => FocusTarget::Sidebar,
            FocusTarget::PlayerBar => FocusTarget::MainContent,
        };
    }

    fn update_highlighted_item(&mut self) {
        let tracks = match &self.content_state {
            ContentState::LikedSongs(t) | ContentState::LikedSongsPage { tracks: t, .. } => {
                Some((t.as_slice(), None::<&str>))
            }
            ContentState::PlaylistTracks(name, t) => Some((t.as_slice(), Some(name.as_str()))),
            ContentState::SearchResults(_, t) => Some((t.as_slice(), None::<&str>)),
            _ => None,
        };

        if let Some((tracks, _context_name)) = tracks {
            if self.selected_index < tracks.len() {
                let track = &tracks[self.selected_index];
                self.highlighted_item = Some(HighlightedItem {
                    uri: track.uri.clone(),
                    name: track.name.clone(),
                    artist: track.artist.clone(),
                    _context: self.current_context.clone(),
                });

                // Update playlist context start_index when navigating
                if let Some(PlaybackContext::Playlist { uri, name, .. }) = &self.current_context {
                    self.current_context = Some(PlaybackContext::Playlist {
                        uri: uri.clone(),
                        name: name.clone(),
                        start_index: self.selected_index,
                    });
                }
            }
        }
    }

    async fn poll_playback(
        &mut self,
        client: &Arc<Mutex<joshify::api::SpotifyClient>>,
        tx_art: &tokio::sync::mpsc::Sender<(String, Vec<u8>)>,
    ) {
        // Store previous state for change detection
        let old_track_uri = self.player_state.current_track_uri.clone();
        let old_is_playing = self.player_state.is_playing;
        let old_progress_ms = self.player_state.progress_ms;
        let old_duration_ms = self.player_state.duration_ms;

        let client_guard = client.lock().await;
        match client_guard.current_playback().await {
            Ok(Some(ctx)) => {
                // Rebuild from the API response, then reconcile album art with
                // the previous state: same track keeps its fetched payloads,
                // a new track starts clean (stale cover never lingers).
                let mut new_state = PlayerState::from_context(&ctx);
                new_state.sync_art_with(&self.player_state);
                self.player_state = new_state;

                let new_track_uri = self.player_state.current_track_uri.clone();
                let new_is_playing = self.player_state.is_playing;

                // Track changed - could be auto-advance or manual skip
                if new_track_uri != old_track_uri {
                    self.player_state.reset_scroll();

                    // Log track change for debugging
                    if let (Some(ref old), Some(ref new)) = (&old_track_uri, &new_track_uri) {
                        tracing::info!(
                            "Track changed from {} to {} (is_playing: {})",
                            old,
                            new,
                            new_is_playing
                        );

                        // If we have a context and track changed while playing,
                        // update our queue position tracking
                        if new_is_playing && self.playback_mode == PlaybackMode::Remote {
                            self.handle_remote_track_advance().await;
                        }
                    }
                }

                // Detect when playback stopped (track ended or paused)
                if old_is_playing && !new_is_playing {
                    // Check if we were near the end of the track (within 2 seconds)
                    let was_near_end = old_duration_ms.saturating_sub(old_progress_ms) < 2000;

                    if was_near_end && self.playback_mode == PlaybackMode::Remote {
                        tracing::info!(
                            "Track ended naturally (progress: {}ms / {}ms) - triggering advance",
                            old_progress_ms,
                            old_duration_ms
                        );
                        self.trigger_remote_advance(client).await;
                    } else {
                        tracing::debug!(
                            "Playback stopped (progress: {}ms / {}ms, near_end: {})",
                            old_progress_ms,
                            old_duration_ms,
                            was_near_end
                        );
                    }
                }

                let new_album_art_url = self.player_state.current_album_art_url.clone();
                let art_missing = self.player_state.current_album_art_data.is_none();

                // Fetch when the track changed, OR when the current track has
                // a cover URL but no fetched payload (first poll after startup,
                // or retrying an earlier failure — throttled by the cooldown).
                let track_changed = new_track_uri != old_track_uri && new_track_uri.is_some();
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let cooled_down =
                    now_ms.saturating_sub(self.last_art_fetch_ms) >= ART_FETCH_COOLDOWN_MS;

                if new_track_uri.is_some()
                    && new_album_art_url.is_some()
                    && art_missing
                    && (track_changed || cooled_down)
                {
                    if let (Some(art_url), Some(art_uri)) = (new_album_art_url, new_track_uri) {
                        let cache = self.album_art_cache.clone();
                        let tx_art_clone = tx_art.clone();
                        let art_uri_for_closure = art_uri.clone();

                        tokio::spawn(async move {
                            match cache.get_or_fetch(&art_url).await {
                                Some(image_data) => {
                                    tracing::debug!(
                                        "Fetched album art for {}",
                                        art_uri_for_closure
                                    );
                                    let _ =
                                        tx_art_clone.send((art_uri_for_closure, image_data)).await;
                                }
                                None => {
                                    tracing::warn!("Failed to fetch album art for {}", art_url);
                                }
                            }
                        });

                        self.last_fetched_art_uri = Some(art_uri);
                        self.last_art_fetch_ms = now_ms;
                    }
                }
            }
            Ok(None) => {
                // Playback stopped completely
                if old_is_playing {
                    tracing::info!("Playback stopped (no active playback context)");

                    // If we were playing and now there's nothing, try to advance
                    if self.playback_mode == PlaybackMode::Remote {
                        self.trigger_remote_advance(client).await;
                    }
                }

                self.player_state.is_playing = false;
                self.player_state.current_track_name = Some("Nothing playing".to_string());
                self.player_state.current_artist_name = Some("".to_string());
                if self
                    .status_message
                    .as_ref()
                    .is_some_and(|m| m.starts_with("Playback error"))
                {
                    self.status_message = None;
                }
            }
            Err(e) => {
                let err_msg = format!("Playback error: {}", e);
                if self.status_message.as_ref() != Some(&err_msg) {
                    self.status_message = Some(err_msg);
                }
            }
        }
    }

    /// Handle track auto-advance in remote mode
    /// Called when Spotify advances to the next track within a context
    async fn handle_remote_track_advance(&mut self) {
        // Advance our internal queue tracking to stay in sync with Spotify
        let queue = self.queue_state.playback_queue_mut();
        if queue.has_context() {
            // Spotify advanced within the context - advance our position tracker
            // but don't actually play anything (Spotify is already playing)
            let _ = queue.advance();
            tracing::info!(
                "Advanced queue position to {} (context: {})",
                queue.context_position(),
                queue.context().name()
            );
        }
    }

    /// Trigger next track in remote mode
    /// Called when current track ends and we need to continue playback
    async fn trigger_remote_advance(&mut self, client: &Arc<Mutex<joshify::api::SpotifyClient>>) {
        // Check if we have items in the up_next queue
        let next_from_queue = {
            let queue = self.queue_state.playback_queue_mut();
            queue.advance()
        };

        if let Some(next_uri) = next_from_queue {
            // Play the track the user actually queued. This used to call
            // playback_next(), which advances *Spotify's* queue - so the track
            // queued here was dropped and whatever Spotify had lined up played
            // instead.
            tracing::info!("Advancing to next track from queue: {}", next_uri);
            let c = client.clone();
            let preferred = self.selected_device_id.clone();
            tokio::spawn(async move {
                let guard = c.lock().await;
                let device_id = guard
                    .device_to_play_on(preferred.as_deref())
                    .await
                    .ok()
                    .flatten();
                if let Err(e) = guard
                    .start_playback_on(
                        vec![next_uri],
                        None,
                        device_id.as_ref().and_then(|d| d.id.as_deref()),
                    )
                    .await
                {
                    tracing::warn!("Failed to advance to next track: {}", e);
                }
            });
        } else {
            // No queue items - check if we have context tracks to continue with
            let queue = self.queue_state.playback_queue();
            if queue.has_context() && queue.remaining_context_tracks() > 0 {
                // Spotify is already auto-advancing within the context
                // since we started playback with start_context_playback()
                // Just update our position tracker to stay in sync
                tracing::info!(
                    "No queue items, Spotify auto-advancing within context ({} tracks remaining)",
                    queue.remaining_context_tracks()
                );
                // The handle_remote_track_advance() will be called by the poll loop
                // when Spotify reports the track change, which will advance our position
            } else {
                tracing::info!("No more tracks in queue or context - playback will stop");
            }
        }
    }
}
use ratatui::backend::CrosstermBackend;
use ratatui::{prelude::*, widgets::Paragraph};
use rspotify::prelude::{BaseClient, Id};
use std::io;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments FIRST (before any terminal initialization)
    let args = CliArgs::parse();

    // Handle --help flag (before any terminal initialization)
    if args.help {
        CliArgs::print_help();
        return Ok(());
    }

    // Self-management subcommands. These run before logging and the TUI: they
    // are ordinary CLI tools and must not touch the terminal.
    if let Some(command) = args.command.clone() {
        return match command {
            joshify::Subcommand::Update(options) => joshify::manage::run_update(&options).await,
            joshify::Subcommand::Uninstall(options) => joshify::manage::run_uninstall(&options),
        };
    }

    // Handle --version. This must stay reachable and must print on stdout
    // without starting anything: install.sh uses it to detect an existing
    // install and to smoke-test a downloaded binary before installing it.
    if args.version {
        CliArgs::print_version();
        return Ok(());
    }

    // Handle --test-search flag (test search API without TUI)
    if args.test_search {
        return run_search_test(args).await;
    }

    // Handle --setup: configure credentials and authorize, then exit without
    // ever entering the TUI. This is the supported headless path (issue #47).
    if args.setup {
        return run_setup_only(args).await;
    }

    // Initialize tracing to file (before terminal init to avoid polluting TUI)
    let log_dir = std::env::var("HOME")
        .map(|h| format!("{}/.cache/joshify", h))
        .unwrap_or_else(|_| "/tmp/joshify".to_string());
    let _ = std::fs::create_dir_all(&log_dir);
    let log_file = std::fs::File::create(format!("{}/joshify.log", log_dir))?;
    tracing_subscriber::fmt()
        .with_writer(log_file)
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // Setup Ctrl-C handler for clean exit
    let result = tokio::select! {
        res = run_with_args(args) => res,
        _ = tokio::signal::ctrl_c() => {
            // Clean exit on Ctrl-C
            let _ = crossterm::execute!(
                io::stdout(),
                crossterm::event::DisableMouseCapture
            );
            ratatui::restore();
            println!("Goodbye!");
            return Ok(());
        }
    };

    // Restore terminal on exit - disable mouse capture first
    let _ = crossterm::execute!(io::stdout(), crossterm::event::DisableMouseCapture);
    ratatui::restore();

    result
}

/// Run credential setup and the OAuth flow, then exit.
///
/// Deliberately never touches the terminal: no raw mode, no alternate screen.
/// This is the path for headless boxes, SSH sessions and WSL, where the TUI is
/// either unwanted or in the way (issue #47).
async fn run_setup_only(args: CliArgs) -> Result<()> {
    let config = OAuthConfig::from_args(&args);

    let config = if config.client_id.is_empty() || config.client_secret.is_empty() {
        joshify::setup::run_setup()?
    } else {
        println!("Using credentials from CLI arguments / environment.");
        config
    };

    match joshify::setup::run_oauth_flow(&config).await {
        Ok(true) => println!("Already authorized - existing credentials are still valid."),
        Ok(false) => println!("Authorization complete."),
        Err(e) => {
            eprintln!("Authorization failed: {e}");
            return Err(e);
        }
    }

    if let Ok(config_dir) = joshify::auth::get_config_dir() {
        println!("\nCredentials are stored in {}.", config_dir.display());
    }
    println!("Run 'joshify' to start the app.");
    Ok(())
}

/// Explain, in one status-bar line, why local playback is not available.
///
/// Running as root is called out separately: it is the single most common cause
/// under WSL, because a root session has no `PULSE_SERVER` and a different
/// `$HOME`, so neither the audio bridge nor the OS keyring is reachable.
fn no_audio_message(probe: &joshify::player::AudioProbe) -> String {
    let reason = match probe {
        joshify::player::AudioProbe::Available => return String::new(),
        joshify::player::AudioProbe::Unavailable(reason) => reason,
    };

    if is_root() {
        format!(
            "Remote playback only - no audio device as root ({reason}) - run as your normal user"
        )
    } else {
        format!("Remote playback only - no audio device ({reason}) - press 'd' to pick a device")
    }
}

/// Whether the process is running as root.
///
/// `/proc/self/status` would be Linux-only, and macOS is a shipped target.
fn is_root() -> bool {
    // SAFETY: geteuid() takes no arguments, cannot fail, and only reads the
    // calling process's effective uid.
    unsafe { libc::geteuid() == 0 }
}

/// Hand the terminal back to the shell, run `f`, then restore the TUI.
///
/// Interactive prompts (dialoguer, `println!`) are unusable while raw mode and
/// the alternate screen are active: `\n` does not carriage-return, the cursor
/// is hidden, and mouse capture injects escape sequences into stdin. Anything
/// that talks to the user over plain stdio must run inside this.
fn suspend_tui<T, F>(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, f: F) -> Result<T>
where
    F: FnOnce() -> T,
{
    crossterm::execute!(io::stdout(), crossterm::event::DisableMouseCapture)?;
    crossterm::execute!(io::stdout(), crossterm::cursor::Show)?;
    ratatui::restore();

    let result = f();

    ratatui::init();
    crossterm::execute!(io::stdout(), crossterm::event::EnableMouseCapture)?;
    crossterm::execute!(io::stdout(), crossterm::cursor::Hide)?;
    terminal.clear()?;

    Ok(result)
}

async fn run_with_args(args: CliArgs) -> Result<()> {
    // Load config from CLI args (args take precedence over env vars and config file)
    let config = OAuthConfig::from_args(&args);

    // Check if we have credentials from env vars or CLI args
    let has_tokens = !config.client_id.is_empty()
        && !config.client_secret.is_empty()
        && (std::env::var("SPOTIFY_ACCESS_TOKEN").is_ok()
            || std::env::var("SPOTIFY_REFRESH_TOKEN").is_ok()
            || args.access_token.is_some()
            || args.refresh_token.is_some());

    let mut app = App::new();

    // Mock mode (JOSHIFY_MOCK=1): demo data, no Spotify auth or network
    let mock_mode = joshify::state::mock_data::is_mock_mode();
    if mock_mode {
        joshify::state::mock_data::init_mock_state(
            &mut app.is_authenticated,
            &mut app.content_state,
            &mut app.player_state,
        );
        app.status_message = Some("Mock mode - demo data - Press ? for help".to_string());
    } else if has_tokens {
        app.is_authenticated = true;
        app.status_message =
            Some("Connected to Spotify (non-interactive) - Press ? for help".to_string());
    } else {
        // Ensure we have credentials configured (runs interactive setup if needed)
        let config = joshify::setup::ensure_configured()?;

        // Run OAuth browser flow to get access tokens
        match joshify::setup::run_oauth_flow(&config).await {
            Ok(true) => {
                // Already authenticated with valid credentials
                app.is_authenticated = true;
                app.status_message = Some("Connected to Spotify - Press ? for help".to_string());
            }
            Ok(false) => {
                // Fresh authentication completed
                app.is_authenticated = true;
                app.status_message = Some("Connected to Spotify - Press ? for help".to_string());
            }
            Err(e) => {
                app.status_message = Some(format!("OAuth error: {}", e));
                // Continue anyway - may have cached credentials
            }
        }
    }

    // Detect the terminal's image capability once. Windows Terminal and most
    // other terminals cannot show inline images, and producing a Kitty payload
    // for them makes the render loop erase the ASCII fallback (issue #59).
    let inline_images_supported =
        joshify::ui::image_renderer::Protocol::detect().supports_inline_image();
    if !inline_images_supported {
        tracing::info!("Terminal has no inline image support; using ASCII album art");
    }

    // Probe the audio device before the TUI takes the screen. audio_backend::find
    // succeeds even with no working audio, so without this the app claims local
    // playback is active and then plays silence (issue #49). ALSA also writes
    // diagnostics straight to stderr from C, which would otherwise land in the
    // middle of a frame.
    let audio_probe = joshify::player::probe_audio_output();
    if let joshify::player::AudioProbe::Unavailable(ref reason) = audio_probe {
        tracing::warn!("Audio output unavailable: {}", reason);
    }
    let audio_available = matches!(audio_probe, joshify::player::AudioProbe::Available);

    // Initialize the terminal only now that any interactive setup is done.
    // setup::ensure_configured() and run_oauth_flow() above print with
    // println! and read with dialoguer; doing this first would put them in raw
    // mode on the alternate screen, where \n does not carriage-return (text
    // staircases), the cursor is hidden (you type blind), and mouse capture
    // injects escape sequences into the prompt. See issue #46.
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    ratatui::init();

    // Enable mouse capture and hide cursor
    crossterm::execute!(io::stdout(), crossterm::event::EnableMouseCapture)?;
    crossterm::execute!(io::stdout(), crossterm::cursor::Hide)?;

    // Clear any leftover output and force redraw
    terminal.clear()?;

    // Initialize Spotify client wrapped in Arc<Mutex> for shared access
    let client = if mock_mode {
        None
    } else {
        match joshify::api::SpotifyClient::new(&config).await {
            Ok(client) => {
                app.is_authenticated = true;
                app.status_message = Some("Connected to Spotify - Press ? for help".to_string());
                Some(Arc::new(Mutex::new(client)))
            }
            Err(e) => {
                app.status_message = Some(format!("Spotify auth error: {}", e));
                None
            }
        }
    };

    // If using non-interactive tokens, apply them to the client
    if has_tokens {
        if let Some(ref client) = client {
            let client_guard = client.lock().await;
            if let Ok(mut token_guard) = client_guard.oauth.token.lock().await {
                let access_token = args
                    .access_token
                    .clone()
                    .or_else(|| std::env::var("SPOTIFY_ACCESS_TOKEN").ok())
                    .unwrap_or_default();
                let refresh_token = args
                    .refresh_token
                    .clone()
                    .or_else(|| std::env::var("SPOTIFY_REFRESH_TOKEN").ok());

                // Calculate expires_at (assume token is fresh if not specified)
                let expires_at = std::env::var("SPOTIFY_TOKEN_EXPIRES_AT")
                    .ok()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or_else(|| {
                        chrono::Utc::now().timestamp() + 3600 // 1 hour from now
                    });

                *token_guard = Some(rspotify::Token {
                    access_token,
                    refresh_token,
                    expires_at: Some(
                        chrono::DateTime::from_timestamp(expires_at, 0)
                            .unwrap_or(chrono::DateTime::UNIX_EPOCH),
                    ),
                    expires_in: chrono::TimeDelta::seconds(3600),
                    scopes: std::collections::HashSet::new(),
                });
            };
        };
    }

    // Extract access token from the rspotify client (works for OAuth flow too)
    let mut client_access_token: Option<String> = None;
    if let Some(ref client) = client {
        let client_guard = client.lock().await;
        let token_result = client_guard.oauth.token.lock().await;
        if let Ok(token_guard) = token_result {
            if let Some(ref token) = *token_guard {
                client_access_token = Some(token.access_token.clone());
            }
        }
    }

    // Initialize local playback (librespot) - try all token sources
    let access_token = args
        .access_token
        .clone()
        .or_else(|| std::env::var("SPOTIFY_ACCESS_TOKEN").ok())
        .or(client_access_token);

    async fn init_local_player(
        token: &str,
    ) -> Option<(
        Arc<LocalSession>,
        Arc<LocalPlayer>,
        tokio::sync::mpsc::UnboundedReceiver<librespot::playback::player::PlayerEvent>,
    )> {
        match LocalSession::from_access_token(token).await {
            Ok(local_session) => {
                let session = Arc::new(local_session);
                match LocalPlayer::new(&session.session) {
                    Ok(mut player) => {
                        let event_rx = player.take_event_channel()?;
                        let player = Arc::new(player);
                        Some((session, player, event_rx))
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create local player: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to create local session: {}", e);
                None
            }
        }
    }

    if !audio_available {
        // Do not build a local player at all. It would install a live sink that
        // cannot make sound, and app.local_player is consulted in a dozen key
        // handlers regardless of playback_mode, so playback commands would route
        // into a dead path. Registering as a Spotify Connect device would also
        // advertise a device that plays silence.
        app.playback_mode = PlaybackMode::Remote;
        app.status_message = Some(no_audio_message(&audio_probe));
    } else if let Some(ref token) = access_token {
        if let Some((session, player, event_rx)) = init_local_player(token).await {
            // Start Spotify Connect to make joshify appear as a device
            let credentials = Credentials::with_access_token(token.clone());
            let mut connect_mgr =
                joshify::connect::ConnectManager::new(joshify::connect::default_device_name());
            if let Err(e) = connect_mgr
                .start(
                    &session.session,
                    credentials,
                    player.player(),
                    player.mixer(),
                )
                .await
            {
                tracing::warn!("Spotify Connect failed to start: {}", e);
            }

            app.local_session = Some(session);
            app.local_player = Some(player);
            app.player_event_rx = Some(event_rx);
            app.playback_mode = PlaybackMode::Local;
            app.last_progress_tick_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time before epoch")
                .as_millis() as u64;
            app.status_message =
                Some("Connected to Spotify - Local playback active - Press ? for help".to_string());
            tracing::info!("Local playback initialized successfully");
        } else {
            app.playback_mode = PlaybackMode::Remote;
        }
    } else if let Ok(local_session) = LocalSession::from_cache().await {
        let session = Arc::new(local_session);
        if let Ok(mut player) = LocalPlayer::new(&session.session) {
            // Try to get token from cache for Connect
            if let Ok(token) = std::fs::read_to_string(
                std::env::var("HOME")
                    .map(|h| format!("{}/.cache/joshify/credentials.json", h))
                    .unwrap_or_default(),
            ) {
                if let Ok(creds) = serde_json::from_str::<serde_json::Value>(&token) {
                    if let Some(token_str) = creds.get("access_token").and_then(|v| v.as_str()) {
                        let credentials = Credentials::with_access_token(token_str.to_string());
                        let mut connect_mgr = joshify::connect::ConnectManager::new(
                            joshify::connect::default_device_name(),
                        );
                        let _ = connect_mgr
                            .start(
                                &session.session,
                                credentials,
                                player.player(),
                                player.mixer(),
                            )
                            .await;
                    }
                }
            }

            let event_rx = player.take_event_channel();
            let player = Arc::new(player);
            app.local_session = Some(session);
            app.local_player = Some(player);
            app.player_event_rx = event_rx;
            app.playback_mode = PlaybackMode::Local;
            app.last_progress_tick_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time before epoch")
                .as_millis() as u64;
            app.status_message =
                Some("Connected to Spotify - Local playback active - Press ? for help".to_string());
            tracing::info!("Local playback restored from cache");
        }
    }

    // Channel for async data loading results (128 capacity for bursty loads)
    let (tx, mut rx) = tokio::sync::mpsc::channel::<ContentState>(128);

    // Channel for album art data (128 capacity for bursty loads)
    let (tx_art, mut rx_art) = tokio::sync::mpsc::channel::<(String, Vec<u8>)>(128);

    // Channel for playback results from spawned command tasks
    let (tx_play, mut rx_play) = tokio::sync::mpsc::channel::<PlaybackFeedback>(32);

    // Channel for radio station seed tracks
    let (tx_radio, mut rx_radio) = tokio::sync::mpsc::channel::<Vec<rspotify::model::FullTrack>>(8);

    // Main loop
    loop {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before epoch")
            .as_millis() as u64;

        // Poll playback state at interval (only when in remote mode)
        if let Some(ref client) = client {
            if now - app.last_poll_ms >= app.poll_interval_ms {
                if app.playback_mode == PlaybackMode::Remote {
                    app.poll_playback(client, &tx_art).await;
                }
                app.last_poll_ms = now;
            }
        }

        // Fill the radio station from the seed tracks that just arrived.
        while let Ok(tracks) = rx_radio.try_recv() {
            if !app.queue_state.radio_mode {
                continue; // toggled back off while the fetch was in flight
            }
            let already: std::collections::HashSet<String> = app
                .queue_state
                .local_queue
                .iter()
                .map(|e| e.uri.clone())
                .chain(app.player_state.current_track_uri.clone())
                .collect();
            let added = radio_entries_from(&tracks, &already);
            if added.is_empty() {
                app.queue_state.radio_mode = false;
                app.status_message = Some(
                    "Radio needs listening history to build a station from - none found"
                        .to_string(),
                );
            } else {
                let count = added.len();
                for entry in added {
                    app.queue_state.add(entry);
                }
                app.status_message = Some(format!("Radio Mode: ON - {} tracks queued", count));
            }
        }

        // Apply playback results so the UI reflects what Spotify actually did
        // rather than an optimistic guess made at keypress time.
        while let Ok(feedback) = rx_play.try_recv() {
            match feedback {
                PlaybackFeedback::Started { name, artist, uri } => {
                    app.player_state.current_track_name = Some(name.clone());
                    app.player_state.current_artist_name = Some(artist);
                    app.player_state.current_track_uri = Some(uri);
                    app.player_state.is_playing = true;
                    app.player_state.progress_ms = 0;
                    app.player_state.reset_scroll();
                    app.status_message = Some(format!("Playing: {}", name));
                }
                PlaybackFeedback::Failed(msg) => {
                    app.player_state.is_playing = false;
                    tracing::warn!("{}", msg);
                    app.status_message = Some(msg);
                }
                PlaybackFeedback::Transferred { device_name, error } => match error {
                    None => {
                        app.status_message = Some(format!("Playing on {}", device_name));
                    }
                    Some(e) => {
                        // Put the user back where they were: the transfer did
                        // not happen, so remote mode is a lie.
                        app.playback_mode = PlaybackMode::Local;
                        app.selected_device_id = None;
                        app.status_message =
                            Some(format!("Could not switch to {}: {}", device_name, e));
                    }
                },
                PlaybackFeedback::CommandFailed { message, revert } => {
                    match revert {
                        Revert::Nothing => {}
                        Revert::Shuffle(previous) => app.player_state.shuffle = previous,
                        Revert::Repeat(previous) => app.player_state.repeat_mode = previous,
                        Revert::Volume(previous) => app.player_state.volume = previous,
                    }
                    app.status_message = Some(message);
                }
            }
        }

        // Check for async data loading results
        while let Ok(state) = rx.try_recv() {
            match state {
                ContentState::SearchResultsLive(results) => {
                    tracing::debug!(
                        "Received SearchResultsLive: {} items, active={}, pending={:?}, current={}",
                        results.len(),
                        app.search_state.is_active,
                        app.search_state.pending_query,
                        app.search_state.query,
                    );
                    if app.search_state.is_active
                        && app.search_state.pending_query.as_ref() == Some(&app.search_state.query)
                    {
                        app.search_state.set_results(results);
                        tracing::info!("Search results applied successfully");
                    } else {
                        tracing::debug!(
                            "Search results discarded (stale): pending={:?}, current={}",
                            app.search_state.pending_query,
                            app.search_state.query,
                        );
                    }
                }
                ContentState::SearchErrorLive(error) => {
                    tracing::debug!("Received SearchErrorLive: {}", error);
                    if app.search_state.is_active
                        && app.search_state.pending_query.as_ref() == Some(&app.search_state.query)
                    {
                        // No expiry: the error used to clear itself after 5s,
                        // leaving an empty result list that the overlay renders
                        // as "No results found" - turning a failure into a
                        // confident, wrong answer. Editing the query clears it.
                        app.search_state.set_error(error);
                    } else {
                        tracing::debug!(
                            "Search error discarded (stale): pending={:?}, current={}, error={}",
                            app.search_state.pending_query,
                            app.search_state.query,
                            error,
                        );
                    }
                }
                other => {
                    app.loading_more_liked_songs = false;
                    if let ContentState::LikedSongsPage {
                        tracks: new_tracks,
                        total,
                        next_offset,
                    } = other
                    {
                        match &app.content_state {
                            ContentState::LikedSongsPage { tracks, .. } => {
                                let mut combined = tracks.clone();
                                combined.extend(new_tracks);
                                let mut seen = std::collections::HashSet::new();
                                combined.retain(|t| seen.insert(t.uri.clone()));
                                app.content_state = ContentState::LikedSongsPage {
                                    tracks: combined,
                                    total,
                                    next_offset,
                                };
                            }
                            ContentState::LikedSongs(existing_tracks) => {
                                let mut combined = existing_tracks.clone();
                                combined.extend(new_tracks);
                                let mut seen = std::collections::HashSet::new();
                                combined.retain(|t| seen.insert(t.uri.clone()));
                                app.content_state = ContentState::LikedSongsPage {
                                    tracks: combined,
                                    total,
                                    next_offset,
                                };
                            }
                            ContentState::Loading(LoadAction::LikedSongs)
                            | ContentState::LoadingInProgress(LoadAction::LikedSongs) => {
                                // Initial load — replace loading state with results
                                app.content_state = ContentState::LikedSongsPage {
                                    tracks: new_tracks,
                                    total,
                                    next_offset,
                                };
                            }
                            _ => {
                                // Discard stale LikedSongsPage — user navigated away
                            }
                        }
                    } else {
                        app.content_state = other;
                    }
                }
            }
        }

        // Check for album art data results
        while let Ok((track_uri, art_data)) = rx_art.try_recv() {
            if app.player_state.current_track_uri.as_ref() == Some(&track_uri) {
                app.player_state.current_album_art_data = Some(art_data.clone());
                if let Some(frame_area) = app.area {
                    let player_bar_height = 6u16;
                    let sidebar_width = 20u16;
                    let album_art_width = 12u16;
                    let album_area = Rect::new(
                        sidebar_width,
                        frame_area.height.saturating_sub(player_bar_height),
                        album_art_width,
                        player_bar_height,
                    );
                    app.player_state.current_album_art_kitty = if inline_images_supported {
                        joshify::ui::image_renderer::prepare_kitty_image(&art_data, album_area)
                    } else {
                        None
                    };
                    app.player_state.current_album_art_ascii =
                        Some(joshify::ui::image_renderer::render_album_art_as_lines(
                            &art_data, album_area,
                        ));
                    app.player_state.art_rendered_for_area = Some(album_area);
                }
            }
        }

        // Re-process album art if terminal was resized (area changed)
        // Clear the old Kitty image area before re-rendering at the new position
        if let Some(frame_area) = app.area {
            let player_bar_height = 6u16;
            let sidebar_width = 20u16;
            let album_art_width = 12u16;
            let current_album_area = Rect::new(
                sidebar_width,
                frame_area.height.saturating_sub(player_bar_height),
                album_art_width,
                player_bar_height,
            );
            if app.player_state.art_rendered_for_area != Some(current_album_area) {
                // Invalidate last Kitty render area so the old position gets cleared
                // on the next frame render. This prevents ghost images on resize.
                if let Some(ref art_data) = app.player_state.current_album_art_data {
                    app.player_state.current_album_art_kitty = if inline_images_supported {
                        joshify::ui::image_renderer::prepare_kitty_image(
                            art_data,
                            current_album_area,
                        )
                    } else {
                        None
                    };
                    app.player_state.current_album_art_ascii =
                        Some(joshify::ui::image_renderer::render_album_art_as_lines(
                            art_data,
                            current_album_area,
                        ));
                    app.player_state.art_rendered_for_area = Some(current_album_area);
                }
            }
        }

        // Process local player events in batches (max 32 per loop iteration)
        if let Some(ref mut event_rx) = app.player_event_rx {
            let batch_limit = 32;
            app.event_batch.clear();
            while app.event_batch.len() < batch_limit {
                if let Ok(event) = event_rx.try_recv() {
                    app.event_batch.push(event);
                } else {
                    break;
                }
            }

            // Process batched events. Take the batch out so event handlers
            // can take `&mut App` (shared advance logic) while iterating;
            // the Vec is restored afterwards to keep its capacity.
            let batch = std::mem::take(&mut app.event_batch);
            for event in batch.iter() {
                use librespot::playback::player::PlayerEvent;
                match event {
                    PlayerEvent::Playing {
                        track_id,
                        position_ms,
                        ..
                    } => {
                        app.player_state.is_playing = true;
                        app.player_state.current_track_uri = Some(track_id.to_uri());
                        app.player_state.progress_ms = *position_ms;
                    }
                    PlayerEvent::Paused {
                        track_id,
                        position_ms,
                        ..
                    } => {
                        app.player_state.is_playing = false;
                        app.player_state.current_track_uri = Some(track_id.to_uri());
                        app.player_state.progress_ms = *position_ms;
                    }
                    PlayerEvent::Stopped { .. } => {
                        // spirc stops the player at track boundaries and on
                        // Connect takeovers; treating this as an advance
                        // trigger raced with our own load (double-advance /
                        // dead audio). Advance decisions happen exclusively
                        // in the EndOfTrack arm.
                        app.player_state.is_playing = false;
                    }
                    PlayerEvent::EndOfTrack { track_id, .. } => {
                        app.player_state.is_playing = false;

                        // Ignore stale end-of-track echoes for a track we've
                        // already moved past (spirc races our optimistic load).
                        let event_uri = track_id.to_uri();
                        if !joshify::playback::domain::should_auto_advance(
                            &event_uri,
                            app.player_state.current_track_uri.as_deref(),
                        ) {
                            tracing::debug!(
                                "Ignoring EndOfTrack for {}: already playing {}",
                                event_uri,
                                app.player_state.current_track_uri.as_deref().unwrap_or("?")
                            );
                            continue;
                        }

                        // Advance via the shared helper (user queue,
                        // then context tracks, then report end of playback).
                        advance_local_playback(&mut app);
                    }
                    PlayerEvent::TrackChanged { audio_item } => {
                        app.player_state.current_track_name = Some(audio_item.name.clone());
                        app.player_state.duration_ms = audio_item.duration_ms;
                        app.player_state.current_track_uri = Some(audio_item.uri.clone());
                        // The artist is in the event; not reading it left the
                        // previous track's artist on screen (issue #58).
                        if let Some(artist) =
                            joshify::player::artist_from_unique_fields(&audio_item.unique_fields)
                        {
                            app.player_state.current_artist_name = Some(artist);
                        }
                        app.player_state.progress_ms = 0;

                        // Debounce album art fetch (2 second cooldown to prevent storm during seeking)
                        let art_cooldown_ms = 2000u64;
                        let can_fetch_art = now.saturating_sub(app.last_art_fetch_ms)
                            >= art_cooldown_ms
                            && app.last_fetched_art_uri.as_ref() != Some(&audio_item.uri);

                        if can_fetch_art {
                            app.last_art_fetch_ms = now;
                            app.last_fetched_art_uri = Some(audio_item.uri.clone());

                            // Single-level async task (no nested spawn)
                            if let Some(ref client) = client {
                                let c = client.clone();
                                let tx_art = tx_art.clone();
                                let uri = audio_item.uri.clone();
                                tokio::spawn(async move {
                                    if let Some(track_id) = uri.strip_prefix("spotify:track:") {
                                        if let Ok(id) = rspotify::model::TrackId::from_id(track_id)
                                        {
                                            if let Ok(track) =
                                                c.lock().await.oauth.track(id, None).await
                                            {
                                                if let Some(art_url) = track
                                                    .album
                                                    .images
                                                    .first()
                                                    .map(|i| i.url.clone())
                                                {
                                                    if let Ok(resp) = reqwest::get(&art_url).await {
                                                        if let Ok(data) = resp.bytes().await {
                                                            tracing::info!("Album art received: {} bytes for {}", data.len(), uri);
                                                            let _ = tx_art
                                                                .send((uri, data.to_vec()))
                                                                .await;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    }
                    PlayerEvent::VolumeChanged { volume } => {
                        app.player_state.volume = *volume as u32 * 100 / 65535;
                    }
                    PlayerEvent::Seeked { position_ms, .. }
                    | PlayerEvent::PositionCorrection { position_ms, .. } => {
                        app.player_state.progress_ms = *position_ms;
                        // Reset the wall-clock fallback so it doesn't double-count.
                        app.last_progress_tick_ms = now;
                    }
                    PlayerEvent::PositionChanged {
                        track_id,
                        position_ms,
                        ..
                    } => {
                        // Periodic REAL position from librespot (1s interval).
                        // Only accepted for the track we believe is playing so
                        // a stale event can't rewind a freshly-loaded track.
                        if Some(&track_id.to_uri()) == app.player_state.current_track_uri.as_ref() {
                            app.player_state.progress_ms = *position_ms;
                            app.last_progress_tick_ms = now;
                        }
                    }
                    PlayerEvent::Loading {
                        track_id,
                        position_ms,
                        ..
                    } => {
                        app.player_state.current_track_uri = Some(track_id.to_uri());
                        app.player_state.progress_ms = *position_ms;
                    }
                    PlayerEvent::Unavailable { track_id, .. } => {
                        // Load failed inside librespot (region block, removed
                        // track, dead session). Previously swallowed, the UI
                        // kept "playing" silently forever.
                        let uri = track_id.to_uri();
                        if Some(&uri) == app.player_state.current_track_uri.as_ref() {
                            app.player_state.is_playing = false;
                            let name = app
                                .player_state
                                .current_track_name
                                .clone()
                                .unwrap_or_else(|| "track".to_string());
                            app.status_message =
                                Some(format!("Couldn't play '{}' — unavailable", name));
                            tracing::warn!("Track unavailable: {}", uri);
                        }
                    }
                    PlayerEvent::SessionDisconnected { user_name, .. } => {
                        app.player_state.is_playing = false;
                        app.status_message =
                            Some("Spotify session disconnected — restart to reconnect".to_string());
                        tracing::error!("librespot session disconnected for user {}", user_name);
                    }
                    PlayerEvent::SessionConnected { .. } => {
                        tracing::info!("librespot session connected");
                    }
                    _ => {}
                }
            }
            app.event_batch = batch;
        }

        // Increment progress locally when playing based on real elapsed time
        if app.playback_mode == PlaybackMode::Local && app.player_state.is_playing {
            let elapsed = now.saturating_sub(app.last_progress_tick_ms);
            if elapsed >= 1000 {
                app.player_state.progress_ms = app
                    .player_state
                    .progress_ms
                    .saturating_add(elapsed as u32)
                    .min(app.player_state.duration_ms);
                app.last_progress_tick_ms = now;
            }
        }

        // Live search debounce: trigger search after cooldown
        if app.search_state.is_active && app.search_state.should_search(now) {
            if mock_mode {
                let query = app.search_state.query.to_lowercase();
                if !query.is_empty() {
                    app.search_state.mark_search_started(now);
                    let results: Vec<_> = joshify::state::mock_data::get_mock_tracks()
                        .into_iter()
                        .filter(|t| {
                            t.name.to_lowercase().contains(&query)
                                || t.artist.to_lowercase().contains(&query)
                        })
                        .collect();
                    app.search_state.set_results(results);
                }
            } else if let Some(ref client) = client {
                let query = app.search_state.query.clone();
                if !query.is_empty() {
                    app.search_state.mark_search_started(now);
                    let c = client.clone();
                    let tx_clone = tx.clone();
                    tokio::spawn(async move {
                        let guard = c.lock().await;
                        match guard.search(&query, 15).await {
                            Ok(tracks) => {
                                tracing::info!(
                                    "Search spawned {} results for '{}'",
                                    tracks.len(),
                                    query
                                );
                                let items: Vec<joshify::state::app_state::TrackListItem> = tracks
                                    .into_iter()
                                    .filter_map(|t| {
                                        t.id.map(|id| {
                                            let artist = t
                                                .artists
                                                .first()
                                                .map(|a| a.name.clone())
                                                .unwrap_or_else(|| {
                                                    tracing::warn!(
                                                        "track '{}' has no artists",
                                                        t.name
                                                    );
                                                    String::new()
                                                });
                                            joshify::state::app_state::TrackListItem {
                                                name: t.name,
                                                artist,
                                                uri: format!("spotify:track:{}", id.id()),
                                            }
                                        })
                                    })
                                    .collect();
                                tracing::info!("Sending {} TrackListItems to channel", items.len());
                                let _ = tx_clone.send(ContentState::SearchResultsLive(items)).await;
                            }
                            Err(e) => {
                                tracing::error!("Search async error for '{}': {}", query, e);
                                let _ = tx_clone
                                    .send(ContentState::SearchErrorLive(format!(
                                        "Search failed: {}",
                                        e
                                    )))
                                    .await;
                            }
                        }
                    });
                }
            } else if !app.search_state.query.is_empty() {
                // Without a client there is nothing to search. Rendering "No
                // results found" blamed the query for an auth failure.
                app.search_state.mark_search_started(now);
                app.search_state
                    .set_error("Not connected to Spotify - restart to sign in".to_string());
            }
        }

        // Frame rate limiting (max 30fps = 33ms between frames)
        let frame_interval_ms = 33u64;
        let should_draw = now.saturating_sub(app.last_frame_time_ms) >= frame_interval_ms;

        if should_draw {
            app.last_frame_time_ms = now;

            // Advance scrolling title animation
            if let Some(ref title) = app.player_state.current_track_name {
                let title_width = unicode_width::UnicodeWidthStr::width(title.as_str());
                let info_width = app
                    .area
                    .map(|a| a.width.saturating_sub(20 + 12 + 4) as usize)
                    .unwrap_or(0);
                app.player_state.tick_scroll(title_width, info_width);
            }

            terminal.draw(|frame| {
                let area = frame.area();

                // Clear layout cache at start of each frame for fresh hit testing
                app.layout_cache.clear();

                // Check minimum terminal size
                if area.width < 50 || area.height < 20 {
                    let warning = Paragraph::new(
                        "Terminal too small!\n\nMinimum: 50x20\n\nPlease resize your terminal.",
                    )
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::Yellow));
                    frame.render_widget(warning, area);
                    return;
                }

                // Status bar at top (if present)
                let top_area = if let Some(ref msg) = app.status_message {
                    let [top, rest] =
                        Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
                    let status = Paragraph::new(msg.as_str())
                        .style(Style::default().fg(Color::Black).bg(Color::Blue));
                    frame.render_widget(status, top);
                    rest
                } else {
                    area
                };

                // Sidebar: fixed width for logo + nav
                let sidebar_width = 20u16;

                // Split into sidebar and main content
                let [sidebar, main] =
                    Layout::horizontal([Constraint::Length(sidebar_width), Constraint::Min(0)])
                        .areas(top_area);

                // Player bar: 6 rows at bottom (includes album art)
                let player_bar_height = 6u16;
                let [main_content, player_bar] =
                    Layout::vertical([Constraint::Min(0), Constraint::Length(player_bar_height)])
                        .areas(main);

                // Render all components with focus highlighting
                let sidebar_focused = app.focus == FocusTarget::Sidebar;
                let main_focused = app.focus == FocusTarget::MainContent;
                let player_focused = app.focus == FocusTarget::PlayerBar;

                joshify::ui::render_sidebar(
                    frame,
                    sidebar,
                    app.selected_nav,
                    sidebar_focused,
                    &mut app.layout_cache,
                );
                joshify::ui::render_main_view(
                    frame,
                    main_content,
                    &app.content_state,
                    app.selected_index,
                    app.scroll_offset,
                    app.is_authenticated,
                    if main_focused {
                        Color::Yellow
                    } else {
                        Color::Green
                    },
                    app.player_state.current_track_uri.as_deref(),
                    &mut app.layout_cache,
                    Some(&app.nav_stack.breadcrumb()),
                );

                let track_name = app
                    .player_state
                    .current_track_name
                    .as_deref()
                    .unwrap_or("Not Playing");
                let artist_name = app
                    .player_state
                    .current_artist_name
                    .as_deref()
                    .unwrap_or("");

                joshify::ui::render_player_bar(
                    frame,
                    player_bar,
                    track_name,
                    artist_name,
                    app.player_state.is_playing,
                    app.player_state.progress_ms,
                    app.player_state.duration_ms,
                    app.player_state.volume,
                    app.player_state.current_album_art_url.as_deref(),
                    app.player_state.current_album_art_ascii.as_deref(),
                    app.queue_state.local_queue.len(),
                    player_focused,
                    app.player_state.shuffle,
                    app.player_state.repeat_mode,
                    app.queue_state.radio_mode,
                    &app.player_state.title_scroll_state,
                    &mut app.layout_cache,
                );

                // Overlays (rendered last so they appear on top)
                if app.show_queue {
                    joshify::ui::render_queue_overlay(
                        frame,
                        area,
                        &app.queue_state,
                        app.queue_selected_index,
                    );
                }
                if let (Some(ref content), Some(ref mut state)) =
                    (&app.help_content, &mut app.help_state)
                {
                    joshify::ui::render_help_overlay(frame, area, content, state);
                }

                // Search overlay - clean modal with live results
                if app.search_state.is_active {
                    joshify::ui::render_search_overlay(frame, area, &app.search_state);
                }

                // Store frame area for mouse handling
                app.area = Some(area);

                // Show cursor only when search overlay is active
                if app.search_state.is_active {
                    let _ = crossterm::execute!(io::stdout(), crossterm::cursor::Show);
                } else {
                    let _ = crossterm::execute!(io::stdout(), crossterm::cursor::Hide);
                }
            })?;
        }

        // Write album art image directly to stdout (bypasses ratatui buffer)
        // Uses pre-processed Kitty escape sequence (no per-frame image processing)
        // The write is gated on a payload signature so an unchanged image is
        // NOT deleted/rewritten every loop iteration (~7x/sec idle previously,
        // causing visible flicker). Redraw happens only when the image or its
        // area actually changed; Clear erases leftovers exactly once.
        let kitty_sig = match (&app.player_state.current_album_art_kitty, app.area) {
            (Some(kitty_data), Some(frame_area)) => {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                frame_area.hash(&mut hasher);
                // Hash the FULL payload: the first 64 bytes are identical
                // boilerplate for every cover at the fixed resize, so a short
                // hash could collide two different covers of equal length and
                // silently keep the old one on screen.
                kitty_data.hash(&mut hasher);
                Some(hasher.finish())
            }
            _ => None,
        };

        match joshify::ui::image_renderer::kitty_action(
            kitty_sig,
            app.player_state.kitty_written_sig,
        ) {
            joshify::ui::image_renderer::KittyAction::Skip => {}
            joshify::ui::image_renderer::KittyAction::Clear => {
                if let Some(old_area) = app.player_state.last_kitty_render_area.take() {
                    let _ = joshify::ui::image_renderer::delete_kitty_image_in_area(old_area);
                    let _ = joshify::ui::image_renderer::clear_terminal_area(old_area);
                }
                app.player_state.kitty_written_sig = None;
            }
            joshify::ui::image_renderer::KittyAction::Redraw => {
                // Delete the previous image (only removes pixels in that area),
                // then clear the rectangle as a fallback for non-Kitty terminals.
                if let Some(old_area) = app.player_state.last_kitty_render_area {
                    let _ = joshify::ui::image_renderer::delete_kitty_image_in_area(old_area);
                    let _ = joshify::ui::image_renderer::clear_terminal_area(old_area);
                }
                if let (Some(kitty_data), Some(frame_area)) = (
                    app.player_state.current_album_art_kitty.as_deref(),
                    app.area,
                ) {
                    let _ = joshify::ui::image_renderer::write_prepared_kitty_image(kitty_data);
                    let player_bar_height = 6u16;
                    let sidebar_width = 20u16;
                    let album_art_width = 12u16;
                    app.player_state.last_kitty_render_area = Some(Rect::new(
                        sidebar_width,
                        frame_area.height.saturating_sub(player_bar_height),
                        album_art_width,
                        player_bar_height,
                    ));
                    app.player_state.kitty_written_sig = kitty_sig;
                }
            }
        }

        // Handle async data loading based on current state
        // Only spawn tasks when in Loading state, not LoadingInProgress (prevents duplicate spawns)
        let load_action = match &app.content_state {
            ContentState::Loading(action) => Some(action.clone()),
            _ => None,
        };

        if let Some(action) = load_action {
            if mock_mode {
                // Serve demo data instantly instead of spawning API tasks
                use joshify::state::mock_data::get_mock_content_state;
                app.content_state = match action {
                    LoadAction::LikedSongs | LoadAction::LikedSongsPage { .. } => {
                        get_mock_content_state(&NavItem::LikedSongs)
                    }
                    LoadAction::Playlists => get_mock_content_state(&NavItem::Playlists),
                    LoadAction::LibraryAlbums | LoadAction::LibraryArtists => {
                        get_mock_content_state(&NavItem::Library)
                    }
                    _ => ContentState::Home,
                };
                app.selected_index = 0;
                app.scroll_offset = 0;
            } else if client.is_none() {
                // Nothing will ever answer, and content_state was left on
                // Loading - the spinner ran forever with no explanation.
                app.content_state = ContentState::Error(
                    "Not connected to Spotify - restart to sign in".to_string(),
                );
            } else if let Some(ref client) = client {
                match action {
                    LoadAction::Devices => {
                        let c = client.clone();
                        let tx_clone = tx.clone();
                        // Offer "This device" whenever a local player exists -
                        // gating on the current mode meant that once you switched
                        // away to a remote device, this entry vanished and there
                        // was no way back.
                        let has_local = app.local_player.is_some();
                        let local_active = app.playback_mode == PlaybackMode::Local;
                        tokio::spawn(async move {
                            let guard = c.lock().await;
                            let devices = match guard.available_devices().await {
                                Ok(devs) => devs,
                                Err(e) => {
                                    let _ = tx_clone
                                        .send(ContentState::Error(format!(
                                            "Failed to load devices: {}",
                                            e
                                        )))
                                        .await;
                                    return;
                                }
                            };
                            let mut entries = Vec::new();
                            if has_local {
                                entries.push(joshify::state::app_state::DeviceEntry::ThisDevice {
                                    active: local_active,
                                });
                            }
                            for device in devices {
                                entries
                                    .push(joshify::state::app_state::DeviceEntry::Remote(device));
                            }
                            let _ = tx_clone.send(ContentState::DeviceSelector(entries)).await;
                        });
                        app.content_state = ContentState::LoadingInProgress(LoadAction::Devices);
                    }
                    LoadAction::LikedSongs => {
                        let c = client.clone();
                        let tx_clone = tx.clone();
                        tokio::spawn(async move {
                            let guard = c.lock().await;
                            match guard.current_user_saved_tracks_paginated(50, 0).await {
                                Ok((tracks, total, next_offset)) => {
                                    let items: Vec<TrackListItem> = tracks
                                        .into_iter()
                                        .filter_map(|t| {
                                            t.track.id.map(|id| {
                                                let artist = t
                                                    .track
                                                    .artists
                                                    .first()
                                                    .map(|a| a.name.clone())
                                                    .unwrap_or_default();
                                                TrackListItem {
                                                    name: t.track.name,
                                                    artist,
                                                    uri: format!("spotify:track:{}", id.id()),
                                                }
                                            })
                                        })
                                        .collect();
                                    let _ = tx_clone
                                        .send(ContentState::LikedSongsPage {
                                            tracks: items,
                                            total,
                                            next_offset,
                                        })
                                        .await;
                                }
                                Err(e) => {
                                    let _ = tx_clone
                                        .send(ContentState::Error(format!(
                                            "Failed to load liked songs: {}",
                                            e
                                        )))
                                        .await;
                                }
                            }
                        });
                        app.content_state = ContentState::LoadingInProgress(LoadAction::LikedSongs);
                    }
                    LoadAction::LikedSongsPage { offset } => {
                        let c = client.clone();
                        let tx_clone = tx.clone();
                        tokio::spawn(async move {
                            let guard = c.lock().await;
                            match guard.current_user_saved_tracks_paginated(50, offset).await {
                                Ok((tracks, total, next_offset)) => {
                                    let items: Vec<TrackListItem> = tracks
                                        .into_iter()
                                        .filter_map(|t| {
                                            t.track.id.map(|id| {
                                                let artist = t
                                                    .track
                                                    .artists
                                                    .first()
                                                    .map(|a| a.name.clone())
                                                    .unwrap_or_default();
                                                TrackListItem {
                                                    name: t.track.name,
                                                    artist,
                                                    uri: format!("spotify:track:{}", id.id()),
                                                }
                                            })
                                        })
                                        .collect();
                                    let _ = tx_clone
                                        .send(ContentState::LikedSongsPage {
                                            tracks: items,
                                            total,
                                            next_offset,
                                        })
                                        .await;
                                }
                                Err(e) => {
                                    let _ = tx_clone
                                        .send(ContentState::Error(format!(
                                            "Failed to load more liked songs: {}",
                                            e
                                        )))
                                        .await;
                                }
                            }
                        });
                    }
                    LoadAction::Playlists => {
                        let c = client.clone();
                        let tx_clone = tx.clone();
                        tokio::spawn(async move {
                            let guard = c.lock().await;
                            match guard.current_users_playlists(50).await {
                                Ok(playlists) => {
                                    let items: Vec<PlaylistListItem> = playlists
                                        .into_iter()
                                        .map(|p| PlaylistListItem {
                                            name: p.name,
                                            id: p.id.id().to_string(),
                                            track_count: p.items.total,
                                        })
                                        .collect();
                                    let _ = tx_clone.send(ContentState::Playlists(items)).await;
                                }
                                Err(e) => {
                                    let _ = tx_clone
                                        .send(ContentState::Error(format!(
                                            "Failed to load playlists: {}",
                                            e
                                        )))
                                        .await;
                                }
                            }
                        });
                        app.content_state = ContentState::LoadingInProgress(LoadAction::Playlists);
                    }
                    LoadAction::PlaylistTracks { name, id } => {
                        let c = client.clone();
                        let tx_clone = tx.clone();
                        let name_clone = name.clone();
                        let id_clone = id.clone();
                        tokio::spawn(async move {
                            let guard = c.lock().await;
                            match guard.playlist_get_items(&id_clone).await {
                                Ok(items) => {
                                    let tracks: Vec<TrackListItem> = items
                                        .into_iter()
                                        .filter_map(|pi| {
                                            pi.item.and_then(|t| {
                                                if let rspotify::model::PlayableItem::Track(track) =
                                                    t
                                                {
                                                    track.id.map(|id| {
                                                        let artist = track
                                                            .artists
                                                            .first()
                                                            .map(|a| a.name.clone())
                                                            .unwrap_or_else(|| {
                                                                tracing::warn!(
                                                                    "track '{}' has no artists",
                                                                    track.name
                                                                );
                                                                String::new()
                                                            });
                                                        TrackListItem {
                                                            name: track.name,
                                                            artist,
                                                            uri: format!(
                                                                "spotify:track:{}",
                                                                id.id()
                                                            ),
                                                        }
                                                    })
                                                } else {
                                                    None
                                                }
                                            })
                                        })
                                        .collect();
                                    let _ = tx_clone
                                        .send(ContentState::PlaylistTracks(
                                            name_clone.clone(),
                                            tracks,
                                        ))
                                        .await;
                                }
                                Err(e) => {
                                    let _ = tx_clone
                                        .send(ContentState::Error(format!(
                                            "Failed to load playlist: {}",
                                            e
                                        )))
                                        .await;
                                }
                            }
                        });
                        // Set playlist context for context playback
                        app.current_context = Some(PlaybackContext::Playlist {
                            uri: format!("spotify:playlist:{}", id),
                            name: name.clone(),
                            start_index: 0,
                        });
                        app.content_state =
                            ContentState::LoadingInProgress(LoadAction::PlaylistTracks {
                                name,
                                id,
                            });
                    }
                    LoadAction::Search { query } => {
                        let c = client.clone();
                        let tx_clone = tx.clone();
                        let query_clone = query.clone();
                        tokio::spawn(async move {
                            let guard = c.lock().await;
                            match guard.search(&query_clone, 15).await {
                                Ok(tracks) => {
                                    let items: Vec<TrackListItem> = tracks
                                        .into_iter()
                                        .filter_map(|t| {
                                            t.id.map(|id| {
                                                let artist = t
                                                    .artists
                                                    .first()
                                                    .map(|a| a.name.clone())
                                                    .unwrap_or_else(|| {
                                                        tracing::warn!(
                                                            "track '{}' has no artists",
                                                            t.name
                                                        );
                                                        String::new()
                                                    });
                                                TrackListItem {
                                                    name: t.name,
                                                    artist,
                                                    uri: format!("spotify:track:{}", id.id()),
                                                }
                                            })
                                        })
                                        .collect();
                                    let _ = tx_clone
                                        .send(ContentState::SearchResults(query_clone, items))
                                        .await;
                                }
                                Err(e) => {
                                    let _ = tx_clone
                                        .send(ContentState::Error(format!("Search failed: {}", e)))
                                        .await;
                                }
                            }
                        });
                        app.content_state =
                            ContentState::LoadingInProgress(LoadAction::Search { query });
                    }
                    LoadAction::HomeData => {
                        let c = client.clone();
                        let tx_clone = tx.clone();
                        tokio::spawn(async move {
                            let guard = c.lock().await;
                            match guard.get_recently_played(20).await {
                                Ok(history) => {
                                    let items: Vec<joshify::state::home_state::RecentlyPlayedItem> = history
                                        .into_iter()
                                        .map(|h| {
                                            let context = h.context.map(|ctx| {
                                                use rspotify::model::Type;
                                                let ctx_type = match ctx._type {
                                                    Type::Album => joshify::state::home_state::ContextType::Album,
                                                    Type::Playlist => joshify::state::home_state::ContextType::Playlist,
                                                    _ => joshify::state::home_state::ContextType::Album,
                                                };
                                                joshify::state::home_state::PlayContext {
                                                    context_type: ctx_type,
                                                    id: ctx.uri,
                                                    name: String::new(), // Will need to fetch separately
                                                }
                                            });
                                            joshify::state::home_state::RecentlyPlayedItem {
                                                track: joshify::state::home_state::TrackSummary {
                                                    name: h.track.name,
                                                    artist: h.track.artists.first().map(|a| a.name.clone()).unwrap_or_default(),
                                                    uri: h.track.id.map(|i| i.to_string()).unwrap_or_default(),
                                                    duration_ms: h.track.duration.num_milliseconds() as u32,
                                                },
                                                played_at: h.played_at,
                                                context,
                                            }
                                        })
                                        .collect();
                                    // Calculate jump back in (empty for now, needs saved data)
                                    let jump_back_in =
                                        joshify::state::home_state::calculate_jump_back_in(
                                            &items, None, None,
                                        );
                                    let _ = tx_clone
                                        .send(ContentState::HomeDashboard(
                                            joshify::state::home_state::HomeState {
                                                recently_played: items,
                                                jump_back_in,
                                                is_loading: false,
                                                last_updated: Some(std::time::Instant::now()),
                                            },
                                        ))
                                        .await;
                                }
                                Err(e) => {
                                    let _ = tx_clone
                                        .send(ContentState::Error(format!(
                                            "Failed to load home data: {}",
                                            e
                                        )))
                                        .await;
                                }
                            }
                        });
                        app.content_state = ContentState::LoadingInProgress(LoadAction::HomeData);
                    }
                    LoadAction::LibraryAlbums => {
                        let c = client.clone();
                        let tx_clone = tx.clone();
                        tokio::spawn(async move {
                            let guard = c.lock().await;
                            // Followed artists were never fetched, so the
                            // Artists tab was always empty and told the user
                            // they follow nobody. Load both in one pass.
                            let artists: Vec<ArtistListItem> =
                                match guard.get_user_artists(50).await {
                                    Ok(list) => list
                                        .into_iter()
                                        .map(|a| ArtistListItem {
                                            name: a.name,
                                            id: a.id.id().to_string(),
                                            image_url: a.images.first().map(|i| i.url.clone()),
                                            genres: a.genres,
                                            // Spotify removed the followers field
                                            // from this payload; showing 0 would be
                                            // a fabricated number.
                                            follower_count: None,
                                        })
                                        .collect(),
                                    Err(e) => {
                                        tracing::warn!("Failed to load followed artists: {}", e);
                                        Vec::new()
                                    }
                                };
                            match guard.get_user_albums(50).await {
                                Ok(saved_albums) => {
                                    let albums: Vec<joshify::state::app_state::AlbumListItem> =
                                        saved_albums
                                            .into_iter()
                                            .map(|sa| {
                                                let release_year: Option<u32> =
                                                    Some(&sa.album.release_date)
                                                        .filter(|s| !s.is_empty())
                                                        .and_then(|d| d.split('-').next())
                                                        .and_then(|y: &str| y.parse().ok());
                                                let artist_name = sa
                                                    .album
                                                    .artists
                                                    .first()
                                                    .map(|a| a.name.clone())
                                                    .unwrap_or_default();
                                                joshify::state::app_state::AlbumListItem {
                                                    name: sa.album.name,
                                                    artist: artist_name,
                                                    id: sa.album.id.id().to_string(),
                                                    image_url: sa
                                                        .album
                                                        .images
                                                        .first()
                                                        .map(|i| i.url.clone()),
                                                    total_tracks: sa.album.tracks.total,
                                                    release_year,
                                                }
                                            })
                                            .collect();
                                    let _ = tx_clone
                                        .send(ContentState::Library {
                                            albums,
                                            artists,
                                            selected_tab:
                                                joshify::state::app_state::LibraryTab::Albums,
                                        })
                                        .await;
                                }
                                Err(e) => {
                                    let _ = tx_clone
                                        .send(ContentState::Error(format!(
                                            "Failed to load albums: {}",
                                            e
                                        )))
                                        .await;
                                }
                            }
                        });
                        app.content_state =
                            ContentState::LoadingInProgress(LoadAction::LibraryAlbums);
                    }
                    LoadAction::LibraryArtists => {
                        // Albums and artists arrive together; reuse that load
                        // rather than reporting "not yet implemented".
                        app.content_state = ContentState::Loading(LoadAction::LibraryAlbums);
                    }
                    LoadAction::AlbumTracks {
                        album_id,
                        name,
                        artist,
                        image_url,
                    } => {
                        let c = client.clone();
                        let tx_clone = tx.clone();
                        let album_id_clone = album_id.clone();
                        let name_clone = name.clone();
                        let artist_clone = artist.clone();
                        let image_url_clone = image_url.clone();
                        tokio::spawn(async move {
                            let guard = c.lock().await;
                            match guard.get_album_tracks(&album_id_clone).await {
                                Ok(tracks) => {
                                    let items: Vec<TrackListItem> = tracks
                                        .into_iter()
                                        .filter_map(|t| {
                                            t.id.map(|id| {
                                                let artist = t
                                                    .artists
                                                    .first()
                                                    .map(|a| a.name.clone())
                                                    .unwrap_or_default();
                                                TrackListItem {
                                                    name: t.name,
                                                    artist,
                                                    uri: format!("spotify:track:{}", id.id()),
                                                }
                                            })
                                        })
                                        .collect();
                                    let album_item = AlbumListItem {
                                        name: name_clone.clone(),
                                        artist: artist_clone.clone(),
                                        id: album_id_clone,
                                        image_url: image_url_clone.clone(),
                                        total_tracks: items.len() as u32,
                                        release_year: None,
                                    };
                                    let _ = tx_clone
                                        .send(ContentState::AlbumDetail {
                                            album: album_item,
                                            tracks: items,
                                        })
                                        .await;
                                }
                                Err(e) => {
                                    let _ = tx_clone
                                        .send(ContentState::Error(format!(
                                            "Failed to load album tracks: {}",
                                            e
                                        )))
                                        .await;
                                }
                            }
                        });
                        app.content_state =
                            ContentState::LoadingInProgress(LoadAction::AlbumTracks {
                                album_id,
                                name,
                                artist,
                                image_url,
                            });
                    }
                    LoadAction::ArtistTopTracks { artist_id, name } => {
                        let artist_item = ArtistListItem {
                            name: name.clone(),
                            id: artist_id.clone(),
                            image_url: None,
                            genres: vec![],
                            follower_count: None,
                        };
                        let _ = tx
                            .send(ContentState::ArtistDetail {
                                artist: artist_item,
                            })
                            .await;
                        app.content_state =
                            ContentState::LoadingInProgress(LoadAction::ArtistTopTracks {
                                artist_id,
                                name,
                            });
                    }
                }
            }
        }

        // Handle input (150ms poll interval for better performance)
        if crossterm::event::poll(std::time::Duration::from_millis(150))? {
            match crossterm::event::read()? {
                crossterm::event::Event::Key(key) => {
                    // GLOBAL QUIT: Check FIRST so it works from ANY context
                    // Standard TUI convention: q or Ctrl+C to quit (like lazygit, btop, etc.)
                    if key.code == crossterm::event::KeyCode::Char('q')
                        || key.code == crossterm::event::KeyCode::Char('c')
                            && key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::CONTROL)
                    {
                        break;
                    }

                    // Search overlay has priority - intercept all keys when active
                    if app.search_state.is_active {
                        match key.code {
                            crossterm::event::KeyCode::Enter => {
                                // Play on this machine by default, exactly like
                                // Enter on a playlist track. Only a device picked
                                // with 'd' (remote mode) sends the track elsewhere;
                                // this path used to go remote unconditionally and
                                // told local users to go pick a device.
                                let picked = app
                                    .search_state
                                    .selected_track()
                                    .map(|t| (t.name.clone(), t.artist.clone(), t.uri.clone()));
                                if let Some(picked) = picked {
                                    play_track(&mut app, client.as_ref(), picked, None, &tx_play);
                                }
                                app.search_state.deactivate();
                            }
                            crossterm::event::KeyCode::Esc => {
                                app.search_state.deactivate();
                            }
                            crossterm::event::KeyCode::Backspace => {
                                app.search_state.delete_char(now);
                            }
                            crossterm::event::KeyCode::Left => {
                                app.search_state.move_cursor_left();
                            }
                            crossterm::event::KeyCode::Right => {
                                app.search_state.move_cursor_right();
                            }
                            crossterm::event::KeyCode::Up => {
                                app.search_state.select_up();
                            }
                            crossterm::event::KeyCode::Down => {
                                app.search_state.select_down(app.search_state.results.len());
                            }
                            crossterm::event::KeyCode::Tab => {
                                if let Some(track) = app.search_state.selected_track() {
                                    // joshify drives its own queue: auto-advance
                                    // plays the next local entry explicitly. This
                                    // also pushed the track into *Spotify's* queue
                                    // with the result discarded, so "Queued:"
                                    // appeared even when that failed - and when it
                                    // succeeded the track could play twice. One
                                    // queue, one source of truth.
                                    let queue_pos = app.queue_state.total_count() + 1;
                                    app.queue_state
                                        .add(joshify::state::queue_state::QueueEntry {
                                            uri: track.uri.clone(),
                                            name: track.name.clone(),
                                            artist: track.artist.clone(),
                                            added_by_user: true,
                                            is_recommendation: false,
                                        });
                                    app.status_message = Some(format!(
                                        "Queued: {} - {} (#{})",
                                        track.name, track.artist, queue_pos
                                    ));
                                }
                            }
                            crossterm::event::KeyCode::Char(c) => {
                                app.search_state.insert_char(c, now);
                            }
                            _ => {}
                        }
                        continue; // Skip all other key handling while searching
                    }

                    // Queue overlay - handle navigation and management
                    if app.show_queue {
                        let queue_len = app.queue_state.local_queue.len();
                        match key.code {
                            crossterm::event::KeyCode::Esc => {
                                app.show_queue = false;
                                continue;
                            }
                            crossterm::event::KeyCode::Char('j')
                            | crossterm::event::KeyCode::Down => {
                                if queue_len > 0 {
                                    app.queue_selected_index =
                                        (app.queue_selected_index + 1).min(queue_len - 1);
                                }
                                continue;
                            }
                            crossterm::event::KeyCode::Char('k')
                            | crossterm::event::KeyCode::Up => {
                                app.queue_selected_index =
                                    app.queue_selected_index.saturating_sub(1);
                                continue;
                            }
                            crossterm::event::KeyCode::Enter => {
                                // The footer has always advertised "Enter: Play"
                                // while no arm handled it.
                                if let Some(entry) =
                                    app.queue_state.local_queue.get(app.queue_selected_index)
                                {
                                    let entry = entry.clone();
                                    // Only consume the entry once something has
                                    // actually taken responsibility for playing
                                    // it - otherwise a missing local player
                                    // silently discarded the track.
                                    let picked = (
                                        entry.name.clone(),
                                        entry.artist.clone(),
                                        entry.uri.clone(),
                                    );
                                    let outcome = play_track(
                                        &mut app,
                                        client.as_ref(),
                                        picked,
                                        None,
                                        &tx_play,
                                    );
                                    if outcome == PlayOutcome::StartedLocally {
                                        app.last_progress_tick_ms = now;
                                    }

                                    if outcome != PlayOutcome::NotStarted {
                                        // Remove by index, not by URI: the same
                                        // track queued twice must lose only the
                                        // copy that is now playing.
                                        app.queue_state
                                            .local_queue
                                            .remove(app.queue_selected_index);
                                        app.queue_state.sync_from_playback_queue();
                                        app.queue_selected_index = app.queue_selected_index.min(
                                            app.queue_state.local_queue.len().saturating_sub(1),
                                        );
                                        app.show_queue = false;
                                    }
                                }
                                continue;
                            }
                            crossterm::event::KeyCode::Char('c') => {
                                // clear() also reset the playback context, which
                                // silently killed auto-advance through the rest
                                // of the playlist. Only the pending queue goes.
                                let removed = app.queue_state.local_queue.len();
                                app.queue_state.clear_pending();
                                app.queue_selected_index = 0;
                                app.status_message = Some(if removed == 0 {
                                    "Queue was already empty".to_string()
                                } else {
                                    format!("Cleared {} queued track(s)", removed)
                                });
                                continue;
                            }
                            crossterm::event::KeyCode::Char('D') => {
                                // Acted on the *main list's* highlight, so it
                                // removed the wrong entry or silently no-oped.
                                if app.queue_selected_index < app.queue_state.local_queue.len() {
                                    let removed = app
                                        .queue_state
                                        .local_queue
                                        .remove(app.queue_selected_index);
                                    app.queue_state.sync_from_playback_queue();
                                    app.queue_selected_index = app
                                        .queue_selected_index
                                        .min(app.queue_state.local_queue.len().saturating_sub(1));
                                    app.status_message =
                                        Some(format!("Removed from queue: {}", removed.name));
                                } else {
                                    app.status_message = Some("Queue is empty".to_string());
                                }
                                continue;
                            }
                            _ => {
                                app.show_queue = false;
                                // Fall through to normal key handling
                            }
                        }
                    }

                    // Device selector overlay - handle navigation and dismissal
                    if matches!(app.content_state, ContentState::DeviceSelector(_)) {
                        match key.code {
                            crossterm::event::KeyCode::Esc
                            | crossterm::event::KeyCode::Char('d') => {
                                app.content_state = ContentState::Home;
                                continue;
                            }
                            crossterm::event::KeyCode::Char('j')
                            | crossterm::event::KeyCode::Down => {
                                if let ContentState::DeviceSelector(ref entries) = app.content_state
                                {
                                    if !entries.is_empty() {
                                        app.selected_index =
                                            (app.selected_index + 1).min(entries.len() - 1);
                                    }
                                }
                                continue;
                            }
                            crossterm::event::KeyCode::Char('k')
                            | crossterm::event::KeyCode::Up => {
                                if app.selected_index > 0 {
                                    app.selected_index -= 1;
                                }
                                continue;
                            }
                            crossterm::event::KeyCode::Enter => {
                                if let ContentState::DeviceSelector(ref entries) = app.content_state
                                {
                                    if !entries.is_empty() && app.selected_index < entries.len() {
                                        match &entries[app.selected_index] {
                                            joshify::state::app_state::DeviceEntry::ThisDevice {
                                                ..
                                            } => {
                                                // Pause whatever remote device was
                                                // playing, or it keeps going while
                                                // the user believes they moved
                                                // playback back to this terminal.
                                                if app.playback_mode == PlaybackMode::Remote {
                                                    if let Some(ref client) = client {
                                                        spawn_remote_command(
                                                            client,
                                                            RemoteCommand::Pause,
                                                            app.selected_device_id.clone(),
                                                            Revert::Nothing,
                                                            tx_play.clone(),
                                                        );
                                                    }
                                                    // The bar is showing polled
                                                    // remote state that will never
                                                    // update again.
                                                    app.player_state.is_playing = false;
                                                    app.player_state.progress_ms = 0;
                                                }
                                                app.playback_mode = PlaybackMode::Local;
                                                app.selected_device_id = None;
                                                app.status_message =
                                                    Some("Switched to local playback".to_string());
                                            }
                                            joshify::state::app_state::DeviceEntry::Remote(
                                                device,
                                            ) => {
                                                if let Some(ref device_id) = device.id {
                                                    if let Some(ref client) = client {
                                                        let c = client.clone();
                                                        let device_name = device.name.clone();
                                                        let id_for_task = device_id.clone();
                                                        let name_for_task = device_name.clone();
                                                        let tx_t = tx_play.clone();
                                                        tokio::spawn(async move {
                                                            let guard = c.lock().await;
                                                            let error = guard
                                                                .transfer_playback(&id_for_task)
                                                                .await
                                                                .err()
                                                                .map(|e| e.to_string());
                                                            let _ = tx_t
                                                                .send(PlaybackFeedback::Transferred {
                                                                    device_name: name_for_task,
                                                                    error,
                                                                })
                                                                .await;
                                                        });
                                                        // Handing audio to another
                                                        // device means this one must
                                                        // stop, or both play at once.
                                                        if let Some(ref player) = app.local_player {
                                                            player.pause();
                                                        }
                                                        app.playback_mode = PlaybackMode::Remote;
                                                        // Remember the choice so later
                                                        // commands target this device
                                                        // instead of re-guessing.
                                                        app.selected_device_id =
                                                            Some(device_id.clone());
                                                        app.status_message = Some(format!(
                                                            "Switching to {}...",
                                                            device_name
                                                        ));
                                                    }
                                                }
                                            }
                                        }
                                        app.content_state = ContentState::Home;
                                        app.selected_index = 0;
                                    }
                                }
                                continue;
                            }
                            _ => {}
                        }
                    }

                    // Global play/pause - works from ANY focus
                    if key.code == crossterm::event::KeyCode::Char(' ') {
                        if app.playback_mode == PlaybackMode::Local {
                            if let Some(ref player) = app.local_player {
                                if app.player_state.is_playing {
                                    player.pause();
                                } else {
                                    player.play();
                                }
                            }
                        } else if let Some(ref client) = client {
                            let command = if app.player_state.is_playing {
                                RemoteCommand::Pause
                            } else {
                                RemoteCommand::Resume
                            };
                            spawn_remote_command(
                                client,
                                command,
                                app.selected_device_id.clone(),
                                Revert::Nothing,
                                tx_play.clone(),
                            );
                        }
                        continue;
                    }

                    // Shuffle toggle (s) - works from ANY focus
                    if key.code == crossterm::event::KeyCode::Char('s') {
                        // Local playback has no shuffle: the queue is walked in
                        // order by advance(). Claiming "Shuffle: ON" here lit an
                        // indicator that changed nothing about what played next.
                        if app.playback_mode == PlaybackMode::Local {
                            app.status_message = Some(
                                "Shuffle applies to remote devices - press 'd' to pick one"
                                    .to_string(),
                            );
                            continue;
                        }
                        if let Some(ref client) = client {
                            let previous = app.player_state.shuffle;
                            let new_shuffle = !previous;
                            app.player_state.shuffle = new_shuffle;
                            spawn_remote_command(
                                client,
                                RemoteCommand::Shuffle(new_shuffle),
                                app.selected_device_id.clone(),
                                Revert::Shuffle(previous),
                                tx_play.clone(),
                            );
                            app.status_message = Some(if new_shuffle {
                                "Shuffle: ON".to_string()
                            } else {
                                "Shuffle: OFF".to_string()
                            });
                            continue;
                        }
                    }

                    // Repeat toggle (r) - cycles Off → Context → Track → Off
                    if key.code == crossterm::event::KeyCode::Char('r') {
                        // Same as shuffle: local playback ignores repeat entirely.
                        if app.playback_mode == PlaybackMode::Local {
                            app.status_message = Some(
                                "Repeat applies to remote devices - press 'd' to pick one"
                                    .to_string(),
                            );
                            continue;
                        }
                        if let Some(ref client) = client {
                            let previous = app.player_state.repeat_mode;
                            app.player_state.repeat_mode = previous.cycle();
                            let new_mode = app.player_state.repeat_mode;
                            let spotify_state = match new_mode {
                                joshify::state::player_state::RepeatMode::Off => {
                                    rspotify::model::RepeatState::Off
                                }
                                joshify::state::player_state::RepeatMode::Context => {
                                    rspotify::model::RepeatState::Context
                                }
                                joshify::state::player_state::RepeatMode::Track => {
                                    rspotify::model::RepeatState::Track
                                }
                            };
                            spawn_remote_command(
                                client,
                                RemoteCommand::Repeat(spotify_state),
                                app.selected_device_id.clone(),
                                Revert::Repeat(previous),
                                tx_play.clone(),
                            );
                            let label = match new_mode {
                                joshify::state::player_state::RepeatMode::Off => "OFF",
                                joshify::state::player_state::RepeatMode::Context => "ALL",
                                joshify::state::player_state::RepeatMode::Track => "ONE",
                            };
                            app.status_message = Some(format!("Repeat: {}", label));
                            continue;
                        }
                    }

                    // Radio mode toggle (Shift+R) - works from ANY focus
                    if key.code == crossterm::event::KeyCode::Char('R') {
                        app.queue_state.radio_mode = !app.queue_state.radio_mode;
                        if app.queue_state.radio_mode {
                            // radio_mode was read by exactly one place: the player
                            // bar renderer. Toggling it lit a badge and changed
                            // nothing about what played. Seed it with real tracks.
                            if let Some(ref client) = client {
                                let c = client.clone();
                                let tx_r = tx_radio.clone();
                                tokio::spawn(async move {
                                    let guard = c.lock().await;
                                    match guard.get_top_tracks(50, "medium").await {
                                        Ok(tracks) => {
                                            let _ = tx_r.send(tracks).await;
                                        }
                                        Err(e) => {
                                            tracing::warn!("Radio seed failed: {}", e);
                                            let _ = tx_r.send(Vec::new()).await;
                                        }
                                    }
                                });
                                app.status_message =
                                    Some("Radio Mode: ON - building station...".to_string());
                            } else {
                                app.queue_state.radio_mode = false;
                                app.status_message =
                                    Some("Radio needs a Spotify connection".to_string());
                            }
                        } else {
                            // Drop what radio added; keep what the user queued.
                            app.queue_state.local_queue.retain(|e| !e.is_recommendation);
                            app.queue_state.sync_from_playback_queue();
                            app.status_message = Some("Radio Mode: OFF".to_string());
                        }
                        continue;
                    }

                    match key.code {
                        // Focus navigation
                        crossterm::event::KeyCode::Tab => {
                            if key
                                .modifiers
                                .contains(crossterm::event::KeyModifiers::SHIFT)
                            {
                                app.focus_previous();
                            } else if app.focus == FocusTarget::MainContent {
                                // When main content is focused, Tab switches tabs in Library view
                                if matches!(app.content_state, ContentState::Library { .. }) {
                                    // Switch library tab
                                    if let ContentState::Library {
                                        albums,
                                        artists,
                                        selected_tab,
                                    } = &app.content_state
                                    {
                                        let new_tab = match selected_tab {
                                            joshify::state::app_state::LibraryTab::Albums => {
                                                joshify::state::app_state::LibraryTab::Artists
                                            }
                                            joshify::state::app_state::LibraryTab::Artists => {
                                                joshify::state::app_state::LibraryTab::Albums
                                            }
                                        };
                                        app.content_state = ContentState::Library {
                                            albums: albums.clone(),
                                            artists: artists.clone(),
                                            selected_tab: new_tab,
                                        };
                                        app.selected_index = 0;
                                        app.scroll_offset = 0;
                                    }
                                } else {
                                    app.focus_next();
                                }
                            } else {
                                app.focus_next();
                            }
                        }
                        crossterm::event::KeyCode::BackTab => {
                            app.focus_previous();
                        }

                        // Enter key - action based on current focus
                        crossterm::event::KeyCode::Enter => {
                            match app.focus {
                                FocusTarget::Sidebar => {
                                    // Select current nav item - show content AND transfer focus to main content
                                    app.loading_more_liked_songs = false;
                                    match app.selected_nav {
                                        joshify::ui::NavItem::LikedSongs => {
                                            app.content_state =
                                                ContentState::Loading(LoadAction::LikedSongs);
                                            app.selected_index = 0;
                                            app.scroll_offset = 0;
                                            app.focus = FocusTarget::MainContent;
                                        }
                                        joshify::ui::NavItem::Playlists => {
                                            app.content_state =
                                                ContentState::Loading(LoadAction::Playlists);
                                            app.selected_index = 0;
                                            app.scroll_offset = 0;
                                            app.focus = FocusTarget::MainContent;
                                        }
                                        joshify::ui::NavItem::Home => {
                                            app.content_state = ContentState::Home;
                                            app.selected_index = 0;
                                            app.scroll_offset = 0;
                                            app.focus = FocusTarget::MainContent;
                                        }
                                        joshify::ui::NavItem::Library => {
                                            app.content_state =
                                                ContentState::Loading(LoadAction::LibraryAlbums);
                                            app.selected_index = 0;
                                            app.scroll_offset = 0;
                                            app.focus = FocusTarget::MainContent;
                                        }
                                    }
                                }
                                FocusTarget::MainContent => {
                                    if let ContentState::LikedSongsPage {
                                        tracks,
                                        next_offset: Some(offset),
                                        ..
                                    } = &app.content_state
                                    {
                                        if !app.loading_more_liked_songs
                                            && app.selected_index >= tracks.len().saturating_sub(3)
                                        {
                                            let load_offset = *offset;
                                            app.loading_more_liked_songs = true;
                                            if let Some(ref client) = client {
                                                let c = client.clone();
                                                let tx_clone = tx.clone();
                                                tokio::spawn(async move {
                                                    let guard = c.lock().await;
                                                    match guard
                                                        .current_user_saved_tracks_paginated(
                                                            50,
                                                            load_offset,
                                                        )
                                                        .await
                                                    {
                                                        Ok((tracks, total, next_offset)) => {
                                                            let items: Vec<TrackListItem> = tracks
                                                                .into_iter()
                                                                .filter_map(|t| {
                                                                    t.track.id.map(|id| {
                                                                        let artist = t
                                                                            .track
                                                                            .artists
                                                                            .first()
                                                                            .map(|a| a.name.clone())
                                                                            .unwrap_or_default();
                                                                        TrackListItem {
                                                                            name: t.track.name,
                                                                            artist,
                                                                            uri: format!(
                                                                                "spotify:track:{}",
                                                                                id.id()
                                                                            ),
                                                                        }
                                                                    })
                                                                })
                                                                .collect();
                                                            let _ = tx_clone
                                                                .send(
                                                                    ContentState::LikedSongsPage {
                                                                        tracks: items,
                                                                        total,
                                                                        next_offset,
                                                                    },
                                                                )
                                                                .await;
                                                        }
                                                        Err(e) => {
                                                            tracing::warn!("Failed to load more liked songs on Enter: {}", e);
                                                            let _ = tx_clone.send(ContentState::Error(format!("Failed to load more liked songs: {}", e))).await;
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                    }
                                    match &app.content_state {
                                        ContentState::LikedSongs(tracks)
                                        | ContentState::LikedSongsPage { tracks, .. }
                                        | ContentState::PlaylistTracks(_, tracks)
                                        | ContentState::SearchResults(_, tracks) => {
                                            if !tracks.is_empty()
                                                && app.selected_index < tracks.len()
                                            {
                                                let track = &tracks[app.selected_index];

                                                // Ensure the context has the correct start_index
                                                // This is critical for URI-based offset playback
                                                if let Some(PlaybackContext::Playlist {
                                                    uri,
                                                    name,
                                                    ..
                                                }) = &app.current_context
                                                {
                                                    let uri = uri.clone();
                                                    let name = name.clone();
                                                    app.current_context =
                                                        Some(PlaybackContext::Playlist {
                                                            uri: uri.clone(),
                                                            name: name.clone(),
                                                            start_index: app.selected_index,
                                                        });
                                                    tracing::info!(
                                                        "Enter key: Updated playlist context start_index to {} for track {}",
                                                        app.selected_index,
                                                        track.name
                                                    );
                                                }

                                                // Track the highlighted item for queue operations
                                                app.highlighted_item = Some(HighlightedItem {
                                                    uri: track.uri.clone(),
                                                    name: track.name.clone(),
                                                    artist: track.artist.clone(),
                                                    _context: app.current_context.clone(),
                                                });

                                                // Populate playback queue for BOTH local and remote modes
                                                // This ensures auto-advance works regardless of playback mode
                                                if let Some(ref ctx) = app.current_context {
                                                    let track_uris: Vec<String> = tracks
                                                        .iter()
                                                        .map(|t| t.uri.clone())
                                                        .collect();
                                                    app.context_track_meta = tracks
                                                        .iter()
                                                        .map(|t| {
                                                            (
                                                                t.uri.clone(),
                                                                (t.name.clone(), t.artist.clone()),
                                                            )
                                                        })
                                                        .collect();
                                                    app.queue_state
                                                        .playback_queue_mut()
                                                        .set_context(
                                                            ctx.clone(),
                                                            track_uris.clone(),
                                                        );
                                                    // Set the position to the selected track
                                                    // advance() will return this track if called, but since we play
                                                    // directly via API/player, we need to advance manually after playback
                                                    app.queue_state
                                                        .playback_queue_mut()
                                                        .set_context_position(app.selected_index);
                                                    app.queue_state.sync_from_playback_queue();
                                                    tracing::info!(
                                                        "Populated playback queue with {} tracks for context playback. Position set to {} (track at index {}: {})",
                                                        track_uris.len(),
                                                        app.selected_index,
                                                        app.selected_index,
                                                        track.name
                                                    );
                                                }

                                                let context_uri = match &app.current_context {
                                                    Some(PlaybackContext::Playlist {
                                                        uri, ..
                                                    }) => Some(uri.clone()),
                                                    _ => None,
                                                };
                                                let picked = (
                                                    track.name.clone(),
                                                    track.artist.clone(),
                                                    track.uri.clone(),
                                                );
                                                if play_track(
                                                    &mut app,
                                                    client.as_ref(),
                                                    picked,
                                                    context_uri,
                                                    &tx_play,
                                                ) == PlayOutcome::StartedLocally
                                                {
                                                    // Advance queue position so the selected track is "consumed"
                                                    // This ensures when track ends, advance() returns the NEXT track
                                                    let _ = app
                                                        .queue_state
                                                        .playback_queue_mut()
                                                        .advance();
                                                    tracing::info!(
                                                        "Local playback started: consumed selected track, queue position now at {} ({} remaining)",
                                                        app.queue_state.playback_queue().context_position(),
                                                        app.queue_state.playback_queue().remaining_context_tracks()
                                                    );
                                                }
                                            }
                                        }
                                        ContentState::Playlists(playlists) => {
                                            // Enter on playlist - show its tracks
                                            if !playlists.is_empty()
                                                && app.selected_index < playlists.len()
                                            {
                                                let playlist = &playlists[app.selected_index];
                                                app.content_state = ContentState::Loading(
                                                    LoadAction::PlaylistTracks {
                                                        name: playlist.name.clone(),
                                                        id: playlist.id.clone(),
                                                    },
                                                );
                                                app.selected_index = 0;
                                                app.scroll_offset = 0;
                                            }
                                        }
                                        ContentState::Library {
                                            albums,
                                            artists,
                                            selected_tab,
                                        } => {
                                            match selected_tab {
                                                LibraryTab::Albums => {
                                                    if !albums.is_empty()
                                                        && app.selected_index < albums.len()
                                                    {
                                                        let album = &albums[app.selected_index];
                                                        // Push current state to nav stack before navigating
                                                        app.nav_stack.push(joshify::state::navigation_stack::NavigationEntry::Library {
                                                            albums: albums.clone(),
                                                            artists: artists.clone()
                                                        });
                                                        // Load album tracks
                                                        app.content_state = ContentState::Loading(
                                                            LoadAction::AlbumTracks {
                                                                album_id: album.id.clone(),
                                                                name: album.name.clone(),
                                                                artist: album.artist.clone(),
                                                                image_url: album.image_url.clone(),
                                                            },
                                                        );
                                                        app.selected_index = 0;
                                                        app.scroll_offset = 0;
                                                    }
                                                }
                                                LibraryTab::Artists => {
                                                    if !artists.is_empty()
                                                        && app.selected_index < artists.len()
                                                    {
                                                        let artist = &artists[app.selected_index];
                                                        // Push current state to nav stack before navigating
                                                        app.nav_stack.push(joshify::state::navigation_stack::NavigationEntry::Library {
                                                            albums: albums.clone(),
                                                            artists: artists.clone()
                                                        });
                                                        // Load artist detail
                                                        // Routing through
                                                        // ArtistTopTracks rebuilt an
                                                        // empty placeholder, throwing
                                                        // away the genres and image
                                                        // the library load fetched, so
                                                        // the detail view showed a
                                                        // bare name.
                                                        app.content_state =
                                                            ContentState::ArtistDetail {
                                                                artist: artist.clone(),
                                                            };
                                                        app.selected_index = 0;
                                                        app.scroll_offset = 0;
                                                    }
                                                }
                                            }
                                        }
                                        ContentState::AlbumDetail { album, tracks } => {
                                            // Push current state to nav stack before navigating
                                            app.nav_stack.push(joshify::state::navigation_stack::NavigationEntry::AlbumDetail {
                                                album: album.clone(),
                                                tracks: tracks.clone()
                                            });
                                            // Play selected track from album
                                            if !tracks.is_empty()
                                                && app.selected_index < tracks.len()
                                            {
                                                let track = &tracks[app.selected_index];

                                                // Give playback an album context so
                                                // auto-advance continues through the
                                                // album instead of ending after one track.
                                                let album_ctx = PlaybackContext::Album {
                                                    uri: format!("spotify:album:{}", album.id),
                                                    name: album.name.clone(),
                                                };
                                                let album_track_uris: Vec<String> =
                                                    tracks.iter().map(|t| t.uri.clone()).collect();
                                                app.current_context = Some(album_ctx.clone());
                                                {
                                                    let queue =
                                                        app.queue_state.playback_queue_mut();
                                                    app.context_track_meta = tracks
                                                        .iter()
                                                        .map(|t| {
                                                            (
                                                                t.uri.clone(),
                                                                (t.name.clone(), t.artist.clone()),
                                                            )
                                                        })
                                                        .collect();
                                                    queue.set_context(album_ctx, album_track_uris);
                                                    queue.set_context_position(app.selected_index);
                                                }
                                                app.queue_state.sync_from_playback_queue();

                                                // Remember current track for local `p` (previous).
                                                if let Some(hist_uri) =
                                                    app.player_state.current_track_uri.clone()
                                                {
                                                    if app.local_history.last() != Some(&hist_uri) {
                                                        app.local_history.push(hist_uri);
                                                        if app.local_history.len() > 50 {
                                                            app.local_history.remove(0);
                                                        }
                                                    }
                                                }

                                                // Track the highlighted item for queue operations
                                                app.highlighted_item = Some(HighlightedItem {
                                                    uri: track.uri.clone(),
                                                    name: track.name.clone(),
                                                    artist: track.artist.clone(),
                                                    _context: app.current_context.clone(),
                                                });

                                                // `track` borrows content_state; copy what
                                                // the player needs first.
                                                let picked = (
                                                    track.name.clone(),
                                                    track.artist.clone(),
                                                    track.uri.clone(),
                                                );
                                                play_track(
                                                    &mut app,
                                                    client.as_ref(),
                                                    picked,
                                                    None,
                                                    &tx_play,
                                                );
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                FocusTarget::PlayerBar => {
                                    // Toggle play/pause from player bar
                                    if app.playback_mode == PlaybackMode::Local {
                                        if let Some(ref player) = app.local_player {
                                            if app.player_state.is_playing {
                                                player.pause();
                                            } else {
                                                player.play();
                                            }
                                        }
                                    } else if let Some(ref client) = client {
                                        let command = if app.player_state.is_playing {
                                            RemoteCommand::Pause
                                        } else {
                                            RemoteCommand::Resume
                                        };
                                        spawn_remote_command(
                                            client,
                                            command,
                                            app.selected_device_id.clone(),
                                            Revert::Nothing,
                                            tx_play.clone(),
                                        );
                                    }
                                }
                            }
                        }

                        // h - Navigate left / back to sidebar
                        crossterm::event::KeyCode::Char('h') => {
                            if app.focus == FocusTarget::MainContent {
                                app.focus = FocusTarget::Sidebar;
                            }
                        }

                        // l - Navigate right / into main content
                        crossterm::event::KeyCode::Char('l') => {
                            if app.focus == FocusTarget::Sidebar {
                                app.focus = FocusTarget::MainContent;
                            }
                        }

                        // Sidebar navigation (when sidebar focused)
                        crossterm::event::KeyCode::Char('j') | crossterm::event::KeyCode::Down => {
                            if app.focus == FocusTarget::Sidebar {
                                let current_idx = app.selected_nav as usize;
                                let next_idx =
                                    (current_idx + 1) % joshify::ui::NavItem::all().len();
                                app.selected_nav = joshify::ui::NavItem::all()[next_idx];
                            } else if app.focus == FocusTarget::MainContent {
                                // Scroll list down based on current content
                                let len = match &app.content_state {
                                    ContentState::LikedSongs(t)
                                    | ContentState::LikedSongsPage { tracks: t, .. } => t.len(),
                                    ContentState::Playlists(p) => p.len(),
                                    ContentState::PlaylistTracks(_, t) => t.len(),
                                    ContentState::SearchResults(_, t) => t.len(),
                                    ContentState::AlbumDetail { tracks, .. } => tracks.len(),
                                    ContentState::Library {
                                        albums,
                                        artists,
                                        selected_tab,
                                    } => match selected_tab {
                                        joshify::state::app_state::LibraryTab::Albums => {
                                            albums.len()
                                        }
                                        joshify::state::app_state::LibraryTab::Artists => {
                                            artists.len()
                                        }
                                    },
                                    _ => 0,
                                };
                                if len > 0 {
                                    app.selected_index = (app.selected_index + 1).min(len - 1);
                                    // Auto-scroll if selection moves out of view
                                    if app.selected_index >= app.scroll_offset + 10 {
                                        app.scroll_offset = app.selected_index - 9;
                                    }
                                    // Update highlighted item
                                    app.update_highlighted_item();
                                    if let ContentState::LikedSongsPage {
                                        next_offset: Some(offset),
                                        ..
                                    } = &app.content_state
                                    {
                                        if !app.loading_more_liked_songs
                                            && app.selected_index >= len.saturating_sub(5)
                                        {
                                            let load_offset = *offset;
                                            app.loading_more_liked_songs = true;
                                            if let Some(ref client) = client {
                                                let c = client.clone();
                                                let tx_clone = tx.clone();
                                                tokio::spawn(async move {
                                                    let guard = c.lock().await;
                                                    match guard
                                                        .current_user_saved_tracks_paginated(
                                                            50,
                                                            load_offset,
                                                        )
                                                        .await
                                                    {
                                                        Ok((tracks, total, next_offset)) => {
                                                            let items: Vec<TrackListItem> = tracks
                                                                .into_iter()
                                                                .filter_map(|t| {
                                                                    t.track.id.map(|id| {
                                                                        let artist = t
                                                                            .track
                                                                            .artists
                                                                            .first()
                                                                            .map(|a| a.name.clone())
                                                                            .unwrap_or_default();
                                                                        TrackListItem {
                                                                            name: t.track.name,
                                                                            artist,
                                                                            uri: format!(
                                                                                "spotify:track:{}",
                                                                                id.id()
                                                                            ),
                                                                        }
                                                                    })
                                                                })
                                                                .collect();
                                                            let _ = tx_clone
                                                                .send(
                                                                    ContentState::LikedSongsPage {
                                                                        tracks: items,
                                                                        total,
                                                                        next_offset,
                                                                    },
                                                                )
                                                                .await;
                                                        }
                                                        Err(e) => {
                                                            tracing::warn!("Failed to load more liked songs: {}", e);
                                                            let _ = tx_clone.send(ContentState::Error(format!("Failed to load more liked songs: {}", e))).await;
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                    }
                                }
                            } else if app.focus == FocusTarget::PlayerBar {
                                // Volume down when player focused
                                let previous_volume = app.player_state.volume;
                                app.player_state.volume = app.player_state.volume.saturating_sub(5);
                                if app.playback_mode == PlaybackMode::Local {
                                    if let Some(ref player) = app.local_player {
                                        let new_vol = joshify::player::percent_to_volume(
                                            app.player_state.volume,
                                        );
                                        player.set_volume(new_vol);
                                    }
                                } else if let Some(ref client) = client {
                                    spawn_remote_command(
                                        client,
                                        RemoteCommand::Volume(app.player_state.volume),
                                        app.selected_device_id.clone(),
                                        Revert::Volume(previous_volume),
                                        tx_play.clone(),
                                    );
                                }
                            }
                        }
                        crossterm::event::KeyCode::Char('k') | crossterm::event::KeyCode::Up => {
                            if app.focus == FocusTarget::Sidebar {
                                let current_idx = app.selected_nav as usize;
                                let next_idx = if current_idx == 0 {
                                    joshify::ui::NavItem::all().len() - 1
                                } else {
                                    current_idx - 1
                                };
                                app.selected_nav = joshify::ui::NavItem::all()[next_idx];
                            } else if app.focus == FocusTarget::MainContent {
                                // Scroll list up based on current content
                                let len = match &app.content_state {
                                    ContentState::LikedSongs(t)
                                    | ContentState::LikedSongsPage { tracks: t, .. } => t.len(),
                                    ContentState::Playlists(p) => p.len(),
                                    ContentState::PlaylistTracks(_, t) => t.len(),
                                    ContentState::SearchResults(_, t) => t.len(),
                                    ContentState::AlbumDetail { tracks, .. } => tracks.len(),
                                    ContentState::Library {
                                        albums,
                                        artists,
                                        selected_tab,
                                    } => match selected_tab {
                                        joshify::state::app_state::LibraryTab::Albums => {
                                            albums.len()
                                        }
                                        joshify::state::app_state::LibraryTab::Artists => {
                                            artists.len()
                                        }
                                    },
                                    _ => 0,
                                };
                                if len > 0 && app.selected_index > 0 {
                                    app.selected_index -= 1;
                                    // Auto-scroll if selection moves out of view
                                    if app.selected_index < app.scroll_offset {
                                        app.scroll_offset = app.selected_index;
                                    }
                                    // Update highlighted item
                                    app.update_highlighted_item();
                                }
                            } else if app.focus == FocusTarget::PlayerBar {
                                // Volume up when player focused
                                let previous_volume = app.player_state.volume;
                                app.player_state.volume = (app.player_state.volume + 5).min(100);
                                if app.playback_mode == PlaybackMode::Local {
                                    if let Some(ref player) = app.local_player {
                                        let new_vol = joshify::player::percent_to_volume(
                                            app.player_state.volume,
                                        );
                                        player.set_volume(new_vol);
                                    }
                                } else if let Some(ref client) = client {
                                    spawn_remote_command(
                                        client,
                                        RemoteCommand::Volume(app.player_state.volume),
                                        app.selected_device_id.clone(),
                                        Revert::Volume(previous_volume),
                                        tx_play.clone(),
                                    );
                                }
                            }
                        }

                        // Playback controls (work from any focus)
                        crossterm::event::KeyCode::Char('n') => {
                            if app.playback_mode == PlaybackMode::Local {
                                // Explicit advance through the same queue →
                                // context path as EndOfTrack (stop() relied on
                                // the old Stopped-triggers-advance behaviour).
                                advance_local_playback(&mut app);
                            } else if let Some(ref client) = client {
                                spawn_remote_command(
                                    client,
                                    RemoteCommand::Next,
                                    app.selected_device_id.clone(),
                                    Revert::Nothing,
                                    tx_play.clone(),
                                );
                            }
                        }
                        crossterm::event::KeyCode::Char('p') => {
                            if app.playback_mode == PlaybackMode::Local {
                                match app.local_history.pop() {
                                    Some(prev_uri) => {
                                        if let Some(ref player) = app.local_player {
                                            match player.load_uri(&prev_uri, true, 0) {
                                                Ok(_) => {
                                                    app.player_state.current_track_uri =
                                                        Some(prev_uri.clone());
                                                    app.player_state.is_playing = true;
                                                    app.player_state.progress_ms = 0;
                                                    app.player_state
                                                        .clear_stale_art_if_track_changed(None);
                                                    app.status_message =
                                                        Some("Playing previous".to_string());
                                                }
                                                Err(e) => {
                                                    app.status_message = Some(format!(
                                                        "Previous track failed: {}",
                                                        e
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                    None => {
                                        // Nowhere to go back to: restart the
                                        // current track instead of doing nothing.
                                        if let Some(ref player) = app.local_player {
                                            player.seek(0);
                                            app.player_state.progress_ms = 0;
                                        }
                                    }
                                }
                            } else if let Some(ref client) = client {
                                spawn_remote_command(
                                    client,
                                    RemoteCommand::Previous,
                                    app.selected_device_id.clone(),
                                    Revert::Nothing,
                                    tx_play.clone(),
                                );
                            }
                        }
                        crossterm::event::KeyCode::Left => {
                            if app.playback_mode == PlaybackMode::Local {
                                if let Some(ref player) = app.local_player {
                                    let new_pos = joshify::playback_keys::seek_back_position(
                                        app.player_state.progress_ms,
                                    );
                                    player.seek(new_pos);
                                    app.player_state.progress_ms = new_pos;
                                }
                            } else if let Some(ref client) = client {
                                // Seek back 10s. (Previously this re-applied
                                // the volume — a copy-paste from the volume
                                // handler; Right seeks forward correctly.)
                                let new_pos = joshify::playback_keys::seek_back_position(
                                    app.player_state.progress_ms,
                                );
                                app.player_state.progress_ms = new_pos;
                                spawn_remote_command(
                                    client,
                                    RemoteCommand::Seek(new_pos),
                                    app.selected_device_id.clone(),
                                    Revert::Nothing,
                                    tx_play.clone(),
                                );
                            }
                        }
                        crossterm::event::KeyCode::Right => {
                            if app.playback_mode == PlaybackMode::Local {
                                if let Some(ref player) = app.local_player {
                                    let new_pos = joshify::playback_keys::seek_forward_position(
                                        app.player_state.progress_ms,
                                        app.player_state.duration_ms,
                                    );
                                    player.seek(new_pos);
                                    app.player_state.progress_ms = new_pos;
                                }
                            } else if let Some(ref client) = client {
                                let new_pos = joshify::playback_keys::seek_forward_position(
                                    app.player_state.progress_ms,
                                    app.player_state.duration_ms,
                                );
                                app.player_state.progress_ms = new_pos;
                                spawn_remote_command(
                                    client,
                                    RemoteCommand::Seek(new_pos),
                                    app.selected_device_id.clone(),
                                    Revert::Nothing,
                                    tx_play.clone(),
                                );
                            }
                        }
                        crossterm::event::KeyCode::Char('+') => {
                            let previous_volume = app.player_state.volume;
                            app.player_state.volume = (app.player_state.volume + 5).min(100);
                            if app.playback_mode == PlaybackMode::Local {
                                if let Some(ref player) = app.local_player {
                                    let new_vol =
                                        joshify::player::percent_to_volume(app.player_state.volume);
                                    player.set_volume(new_vol);
                                }
                            } else if let Some(ref client) = client {
                                spawn_remote_command(
                                    client,
                                    RemoteCommand::Volume(app.player_state.volume),
                                    app.selected_device_id.clone(),
                                    Revert::Volume(previous_volume),
                                    tx_play.clone(),
                                );
                            }
                        }
                        crossterm::event::KeyCode::Char('-') => {
                            let previous_volume = app.player_state.volume;
                            app.player_state.volume = app.player_state.volume.saturating_sub(5);
                            if app.playback_mode == PlaybackMode::Local {
                                if let Some(ref player) = app.local_player {
                                    let new_vol =
                                        joshify::player::percent_to_volume(app.player_state.volume);
                                    player.set_volume(new_vol);
                                }
                            } else if let Some(ref client) = client {
                                spawn_remote_command(
                                    client,
                                    RemoteCommand::Volume(app.player_state.volume),
                                    app.selected_device_id.clone(),
                                    Revert::Volume(previous_volume),
                                    tx_play.clone(),
                                );
                            }
                        }

                        // Device selector
                        crossterm::event::KeyCode::Char('d') => {
                            app.content_state = ContentState::Loading(LoadAction::Devices);
                            app.selected_index = 0;
                        }
                        // Queue toggle
                        crossterm::event::KeyCode::Char('Q') => {
                            app.show_queue = !app.show_queue;
                        }
                        crossterm::event::KeyCode::Char('a') => {
                            // Add highlighted track to local queue
                            if let Some(ref highlighted) = app.highlighted_item {
                                let entry = joshify::state::queue_state::QueueEntry {
                                    uri: highlighted.uri.clone(),
                                    name: highlighted.name.clone(),
                                    artist: highlighted.artist.clone(),
                                    added_by_user: true,
                                    is_recommendation: false,
                                };
                                let queue_pos = app.queue_state.total_count() + 1;
                                app.queue_state.add(entry);
                                app.status_message = Some(format!(
                                    "Added to queue (#{}) {} - {}",
                                    queue_pos, highlighted.name, highlighted.artist
                                ));
                            } else if let Some(ref track_uri) = app.player_state.current_track_uri {
                                // Fallback: add currently playing track
                                let name = app
                                    .player_state
                                    .current_track_name
                                    .clone()
                                    .unwrap_or_default();
                                let artist = app
                                    .player_state
                                    .current_artist_name
                                    .clone()
                                    .unwrap_or_default();
                                let entry = joshify::state::queue_state::QueueEntry {
                                    uri: track_uri.clone(),
                                    name,
                                    artist,
                                    added_by_user: true,
                                    is_recommendation: false,
                                };
                                let queue_pos = app.queue_state.total_count() + 1;
                                app.queue_state.add(entry);
                                app.status_message =
                                    Some(format!("Added current track to queue (#{queue_pos})"));
                            } else {
                                app.status_message = Some("No track to add".to_string());
                            }
                        }

                        // Settings. run_setup() prints with println! and reads
                        // with dialoguer, so the TUI has to give the terminal
                        // back for the duration or the prompts are unreadable
                        // (issue #46).
                        crossterm::event::KeyCode::Char('c') => {
                            let result = suspend_tui(&mut terminal, joshify::setup::run_setup);
                            app.status_message = Some(match result {
                                Ok(Ok(_)) => "Config updated - restart app to apply".to_string(),
                                Ok(Err(_)) => "Setup cancelled".to_string(),
                                Err(e) => format!("Could not open setup: {e}"),
                            });
                        }

                        // Search - '/' key starts search overlay
                        crossterm::event::KeyCode::Char('/') => {
                            app.search_state.activate();
                            app.focus = FocusTarget::MainContent;
                        }

                        // Help
                        crossterm::event::KeyCode::Char('?') => {
                            if app.help_content.is_some() {
                                app.help_content = None;
                                app.help_state = None;
                            } else {
                                app.help_content = Some(joshify::ui::HelpContent::joshify_help());
                                app.help_state = Some(joshify::ui::HelpOverlayState::default());
                            }
                        }
                        // Backspace - browser back navigation
                        crossterm::event::KeyCode::Backspace => {
                            if app.nav_stack.can_go_back() {
                                app.nav_stack.back();
                                if let Some(entry) = app.nav_stack.current().cloned() {
                                    use joshify::state::navigation_stack::NavigationEntry;
                                    match entry {
                                        NavigationEntry::Home => {
                                            app.content_state = ContentState::Home;
                                            app.selected_nav = NavItem::Home;
                                        }
                                        NavigationEntry::Library { albums, artists } => {
                                            app.content_state = ContentState::Library {
                                                albums,
                                                artists,
                                                selected_tab: LibraryTab::Albums,
                                            };
                                            app.selected_nav = NavItem::Library;
                                        }
                                        NavigationEntry::AlbumDetail { album, tracks } => {
                                            app.content_state =
                                                ContentState::AlbumDetail { album, tracks };
                                            app.selected_nav = NavItem::Library;
                                        }
                                        NavigationEntry::ArtistDetail { artist } => {
                                            app.content_state =
                                                ContentState::ArtistDetail { artist };
                                            app.selected_nav = NavItem::Library;
                                        }
                                        NavigationEntry::Playlists(playlists) => {
                                            app.content_state = ContentState::Playlists(playlists);
                                            app.selected_nav = NavItem::Playlists;
                                        }
                                        NavigationEntry::PlaylistTracks { playlist, tracks } => {
                                            app.content_state =
                                                ContentState::PlaylistTracks(playlist.name, tracks);
                                            app.selected_nav = NavItem::Playlists;
                                        }
                                        NavigationEntry::LikedSongs(tracks) => {
                                            app.content_state = ContentState::LikedSongs(tracks);
                                            app.selected_nav = NavItem::LikedSongs;
                                        }
                                        NavigationEntry::SearchResults { query, tracks } => {
                                            app.content_state =
                                                ContentState::SearchResults(query, tracks);
                                        }
                                    }
                                    app.selected_index = 0;
                                    app.scroll_offset = 0;
                                }
                            }
                        }
                        crossterm::event::KeyCode::Esc => {
                            app.show_queue = false;
                            app.help_content = None;
                            app.help_state = None;
                        }
                        _ => {}
                    }
                }
                crossterm::event::Event::Mouse(mouse) => {
                    let action = joshify::ui::handle_mouse_event(
                        mouse,
                        &app.layout_cache,
                        &mut app.mouse_state,
                    );

                    match action {
                        joshify::ui::MouseAction::SelectNavItem(nav) => {
                            app.selected_nav = nav;
                            match nav {
                                NavItem::LikedSongs => {
                                    app.content_state = ContentState::Loading(
                                        joshify::state::LoadAction::LikedSongs,
                                    );
                                    app.selected_index = 0;
                                    app.scroll_offset = 0;
                                }
                                NavItem::Playlists => {
                                    app.content_state = ContentState::Loading(
                                        joshify::state::LoadAction::Playlists,
                                    );
                                    app.selected_index = 0;
                                    app.scroll_offset = 0;
                                }
                                NavItem::Home => {
                                    app.content_state = ContentState::Home;
                                }
                                NavItem::Library => {
                                    app.content_state = ContentState::Loading(
                                        joshify::state::LoadAction::LibraryAlbums,
                                    );
                                    app.selected_index = 0;
                                    app.scroll_offset = 0;
                                }
                            }
                        }
                        joshify::ui::MouseAction::SelectTrack(index) => {
                            app.selected_index = index;
                            if app.selected_index < app.scroll_offset {
                                app.scroll_offset = app.selected_index;
                            }
                        }
                        joshify::ui::MouseAction::SelectPlaylist(index) => {
                            app.selected_index = index;
                            if app.selected_index < app.scroll_offset {
                                app.scroll_offset = app.selected_index;
                            }
                        }
                        joshify::ui::MouseAction::OpenPlaylist(index) => {
                            // Double-click on playlist - open its tracks
                            if let ContentState::Playlists(playlists) = &app.content_state {
                                if !playlists.is_empty() && index < playlists.len() {
                                    let playlist = &playlists[index];
                                    app.content_state = ContentState::Loading(
                                        joshify::state::LoadAction::PlaylistTracks {
                                            name: playlist.name.clone(),
                                            id: playlist.id.clone(),
                                        },
                                    );
                                    app.selected_index = 0;
                                    app.scroll_offset = 0;
                                }
                            }
                        }
                        joshify::ui::MouseAction::PlayTrack(index) => {
                            // Double-click on track - play with playlist context if available
                            let tracks = match &app.content_state {
                                ContentState::LikedSongs(t)
                                | ContentState::LikedSongsPage { tracks: t, .. }
                                | ContentState::PlaylistTracks(_, t)
                                | ContentState::SearchResults(_, t) => Some(t),
                                _ => None,
                            };

                            if let Some(tracks) = tracks {
                                if !tracks.is_empty() && index < tracks.len() {
                                    let track = &tracks[index];
                                    app.selected_index = index;

                                    // Set up playlist context if viewing a playlist
                                    if let ContentState::PlaylistTracks(playlist_id, _) =
                                        &app.content_state
                                    {
                                        let playlist_uri =
                                            format!("spotify:playlist:{}", playlist_id);
                                        let context = PlaybackContext::Playlist {
                                            uri: playlist_uri,
                                            name: playlist_id.clone(),
                                            start_index: index,
                                        };
                                        app.current_context = Some(context.clone());

                                        // Also populate the playback queue with context tracks
                                        // so queue advancement works correctly
                                        let track_uris: Vec<String> =
                                            tracks.iter().map(|t| t.uri.clone()).collect();
                                        app.context_track_meta = tracks
                                            .iter()
                                            .map(|t| {
                                                (t.uri.clone(), (t.name.clone(), t.artist.clone()))
                                            })
                                            .collect();
                                        app.queue_state
                                            .playback_queue_mut()
                                            .set_context(context, track_uris);
                                    }

                                    // Track the highlighted item for queue operations
                                    app.highlighted_item = Some(HighlightedItem {
                                        uri: track.uri.clone(),
                                        name: track.name.clone(),
                                        artist: track.artist.clone(),
                                        _context: app.current_context.clone(),
                                    });

                                    // If we have a playlist context, populate the
                                    // playback queue with context tracks so queue
                                    // advancement works correctly
                                    if let ContentState::PlaylistTracks(_, ref ctx_tracks) =
                                        app.content_state
                                    {
                                        if let Some(ref ctx) = app.current_context {
                                            let track_uris: Vec<String> =
                                                ctx_tracks.iter().map(|t| t.uri.clone()).collect();
                                            app.context_track_meta = ctx_tracks
                                                .iter()
                                                .map(|t| {
                                                    (
                                                        t.uri.clone(),
                                                        (t.name.clone(), t.artist.clone()),
                                                    )
                                                })
                                                .collect();
                                            app.queue_state
                                                .playback_queue_mut()
                                                .set_context(ctx.clone(), track_uris.clone());
                                            // Set position to the selected track
                                            app.queue_state
                                                .playback_queue_mut()
                                                .set_context_position(index);
                                            app.queue_state.sync_from_playback_queue();
                                            tracing::info!(
                                                            "Mouse: Populated playback queue with {} tracks. Position set to {} (track at index {})",
                                                            track_uris.len(),
                                                            index,
                                                            index
                                                        );
                                        }
                                    }

                                    // Play within the playlist context when the click
                                    // came from a playlist view, otherwise fall back to
                                    // whatever context is currently loaded. Local vs
                                    // remote is play_track's decision, not ours.
                                    let context_uri = match &app.content_state {
                                        ContentState::PlaylistTracks(pid, _) => {
                                            Some(format!("spotify:playlist:{}", pid))
                                        }
                                        _ => match &app.current_context {
                                            Some(PlaybackContext::Playlist { uri, .. }) => {
                                                Some(uri.clone())
                                            }
                                            _ => None,
                                        },
                                    };
                                    let picked = (
                                        track.name.clone(),
                                        track.artist.clone(),
                                        track.uri.clone(),
                                    );
                                    if play_track(
                                        &mut app,
                                        client.as_ref(),
                                        picked,
                                        context_uri,
                                        &tx_play,
                                    ) == PlayOutcome::StartedLocally
                                    {
                                        // Advance queue position so the selected track is "consumed"
                                        let _ = app.queue_state.playback_queue_mut().advance();
                                        tracing::info!(
                                            "Mouse: Local playback started - consumed selected track, position now at {} ({} remaining)",
                                            app.queue_state.playback_queue().context_position(),
                                            app.queue_state.playback_queue().remaining_context_tracks()
                                        );
                                    }
                                }
                            }
                        }
                        joshify::ui::MouseAction::SetFocus(focus) => {
                            app.focus = focus;
                        }
                        joshify::ui::MouseAction::TogglePlayPause => {
                            // Trigger play/pause
                            if app.playback_mode == PlaybackMode::Local {
                                if let Some(ref player) = app.local_player {
                                    if app.player_state.is_playing {
                                        player.pause();
                                    } else {
                                        player.play();
                                    }
                                }
                            } else if let Some(ref client) = client {
                                let command = if app.player_state.is_playing {
                                    RemoteCommand::Pause
                                } else {
                                    RemoteCommand::Resume
                                };
                                spawn_remote_command(
                                    client,
                                    command,
                                    app.selected_device_id.clone(),
                                    Revert::Nothing,
                                    tx_play.clone(),
                                );
                            }
                        }
                        joshify::ui::MouseAction::SkipNext => {
                            // Next track
                            if let Some(ref client) = client {
                                spawn_remote_command(
                                    client,
                                    RemoteCommand::Next,
                                    app.selected_device_id.clone(),
                                    Revert::Nothing,
                                    tx_play.clone(),
                                );
                            }
                        }
                        joshify::ui::MouseAction::SkipPrevious => {
                            // Previous track
                            if let Some(ref client) = client {
                                spawn_remote_command(
                                    client,
                                    RemoteCommand::Previous,
                                    app.selected_device_id.clone(),
                                    Revert::Nothing,
                                    tx_play.clone(),
                                );
                            }
                        }
                        joshify::ui::MouseAction::ToggleQueue => {
                            app.show_queue = !app.show_queue;
                        }
                        joshify::ui::MouseAction::CloseOverlay => {
                            app.show_queue = false;
                            app.help_content = None;
                            app.help_state = None;
                        }
                        joshify::ui::MouseAction::ScrollUp => {
                            // Handle scroll up based on focus
                            match app.focus {
                                FocusTarget::Sidebar => {
                                    // Navigate sidebar up
                                    let nav_items = NavItem::all();
                                    let current_idx = nav_items
                                        .iter()
                                        .position(|&n| n == app.selected_nav)
                                        .unwrap_or(0);
                                    if current_idx > 0 {
                                        app.selected_nav = nav_items[current_idx - 1];
                                    }
                                }
                                FocusTarget::MainContent
                                    // Scroll up in list
                                    if app.selected_index > 0 => {
                                        app.selected_index -= 1;
                                        if app.selected_index < app.scroll_offset {
                                            app.scroll_offset = app.selected_index;
                                        }
                                    }
                                _ => {}
                            }
                        }
                        joshify::ui::MouseAction::ScrollDown => {
                            // Handle scroll down based on focus
                            match app.focus {
                                FocusTarget::Sidebar => {
                                    // Navigate sidebar down
                                    let nav_items = NavItem::all();
                                    let current_idx = nav_items
                                        .iter()
                                        .position(|&n| n == app.selected_nav)
                                        .unwrap_or(0);
                                    if current_idx < nav_items.len() - 1 {
                                        app.selected_nav = nav_items[current_idx + 1];
                                    }
                                }
                                FocusTarget::MainContent => {
                                    // Scroll down in list
                                    let len = match &app.content_state {
                                        ContentState::LikedSongs(t) => t.len(),
                                        ContentState::LikedSongsPage { tracks, .. } => tracks.len(),
                                        ContentState::PlaylistTracks(_, t) => t.len(),
                                        ContentState::SearchResults(_, t) => t.len(),
                                        _ => 0,
                                    };
                                    if len > 0 && app.selected_index < len - 1 {
                                        app.selected_index += 1;
                                        if app.selected_index >= app.scroll_offset + 10 {
                                            app.scroll_offset = app.selected_index - 9;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        joshify::ui::MouseAction::AdjustVolume(delta) => {
                            // Adjust volume
                            let new_volume =
                                (app.player_state.volume as i32 + delta).clamp(0, 100) as u32;
                            let previous_volume = app.player_state.volume;
                            app.player_state.volume = new_volume;

                            if app.playback_mode == PlaybackMode::Local {
                                // Use local player for volume control
                                if let Some(ref player) = app.local_player {
                                    let new_vol = joshify::player::percent_to_volume(new_volume);
                                    player.set_volume(new_vol);
                                }
                            } else if let Some(ref client) = client {
                                spawn_remote_command(
                                    client,
                                    RemoteCommand::Volume(new_volume),
                                    app.selected_device_id.clone(),
                                    Revert::Volume(previous_volume),
                                    tx_play.clone(),
                                );
                            }
                        }
                        joshify::ui::MouseAction::ToggleShuffle => {
                            if app.playback_mode == PlaybackMode::Local {
                                app.status_message = Some(
                                    "Shuffle applies to remote devices - press 'd' to pick one"
                                        .to_string(),
                                );
                            } else if let Some(ref client) = client {
                                let previous = app.player_state.shuffle;
                                app.player_state.shuffle = !previous;
                                spawn_remote_command(
                                    client,
                                    RemoteCommand::Shuffle(!previous),
                                    app.selected_device_id.clone(),
                                    Revert::Shuffle(previous),
                                    tx_play.clone(),
                                );
                            }
                        }
                        joshify::ui::MouseAction::CycleRepeat => {
                            if app.playback_mode == PlaybackMode::Local {
                                app.status_message = Some(
                                    "Repeat applies to remote devices - press 'd' to pick one"
                                        .to_string(),
                                );
                            } else if let Some(ref client) = client {
                                let previous = app.player_state.repeat_mode;
                                app.player_state.repeat_mode = previous.cycle();
                                let mode = match app.player_state.repeat_mode {
                                    joshify::state::player_state::RepeatMode::Off => {
                                        rspotify::model::RepeatState::Off
                                    }
                                    joshify::state::player_state::RepeatMode::Track => {
                                        rspotify::model::RepeatState::Track
                                    }
                                    joshify::state::player_state::RepeatMode::Context => {
                                        rspotify::model::RepeatState::Context
                                    }
                                };
                                spawn_remote_command(
                                    client,
                                    RemoteCommand::Repeat(mode),
                                    app.selected_device_id.clone(),
                                    Revert::Repeat(previous),
                                    tx_play.clone(),
                                );
                            }
                        }
                        // Clicking the progress bar and the volume bar both
                        // emitted actions that no arm handled - they fell into
                        // the catch-all below and did nothing at all, while the
                        // help screen advertised both.
                        joshify::ui::MouseAction::Seek(percent) => {
                            let new_pos =
                                position_from_percent(percent, app.player_state.duration_ms);
                            if app.playback_mode == PlaybackMode::Local {
                                if let Some(ref player) = app.local_player {
                                    player.seek(new_pos);
                                    app.player_state.progress_ms = new_pos;
                                }
                            } else if let Some(ref client) = client {
                                spawn_remote_command(
                                    client,
                                    RemoteCommand::Seek(new_pos),
                                    app.selected_device_id.clone(),
                                    Revert::Nothing,
                                    tx_play.clone(),
                                );
                            }
                        }
                        joshify::ui::MouseAction::SetVolume(percent) => {
                            let previous_volume = app.player_state.volume;
                            app.player_state.volume = (percent as u32).min(100);
                            if app.playback_mode == PlaybackMode::Local {
                                if let Some(ref player) = app.local_player {
                                    player.set_volume(joshify::player::percent_to_volume(
                                        app.player_state.volume,
                                    ));
                                }
                            } else if let Some(ref client) = client {
                                spawn_remote_command(
                                    client,
                                    RemoteCommand::Volume(app.player_state.volume),
                                    app.selected_device_id.clone(),
                                    Revert::Volume(previous_volume),
                                    tx_play.clone(),
                                );
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

/// Test search API functionality without TUI
async fn run_search_test(args: CliArgs) -> Result<()> {
    use joshify::api::SpotifyClient;
    use joshify::auth::OAuthConfig;

    println!("🔍 Testing Spotify Search API...\n");

    // Load config
    let config = OAuthConfig::from_args(&args);

    // Check for credentials
    if config.client_id.is_empty() || config.client_secret.is_empty() {
        eprintln!("❌ Error: Client ID and Secret required");
        eprintln!("   Set SPOTIFY_CLIENT_ID and SPOTIFY_CLIENT_SECRET env vars");
        eprintln!("   Or use --client-id and --client-secret flags");
        std::process::exit(1);
    }

    // Check for access token
    let has_token = std::env::var("SPOTIFY_ACCESS_TOKEN").is_ok() || args.access_token.is_some();

    if !has_token {
        eprintln!("❌ Error: Access token required");
        eprintln!("   Set SPOTIFY_ACCESS_TOKEN env var");
        eprintln!("   Or use --access-token flag");
        std::process::exit(1);
    }

    println!("✅ Credentials found");
    println!("📡 Connecting to Spotify API...");

    // Create client
    let client = match SpotifyClient::new(&config).await {
        Ok(c) => {
            println!("✅ Connected to Spotify API");
            c
        }
        Err(e) => {
            eprintln!("❌ Connection failed: {}", e);
            std::process::exit(1);
        }
    };

    // Test searches
    let test_queries = vec![
        "abba",
        "beatles",
        "taylor swift",
        "rock & roll",
        "テスト", // Japanese characters
    ];

    println!("\n🎵 Running test searches...\n");

    let mut success_count = 0;
    let mut fail_count = 0;

    for query in test_queries {
        print!("   Searching '{}': ", query);
        match client.search(query, 5).await {
            Ok(tracks) => {
                if tracks.is_empty() {
                    println!("⚠️  No results (may be region-locked)");
                } else {
                    println!("✅ {} results", tracks.len());
                    for (i, track) in tracks.iter().take(3).enumerate() {
                        let artist = track
                            .artists
                            .first()
                            .map(|a| a.name.as_str())
                            .unwrap_or("Unknown");
                        println!("      {}. {} - {}", i + 1, artist, track.name);
                    }
                    success_count += 1;
                }
            }
            Err(e) => {
                println!("❌ Failed: {}", e);
                fail_count += 1;
            }
        }
    }

    println!("\n📊 Test Results:");
    println!("   ✅ Passed: {}", success_count);
    println!("   ❌ Failed: {}", fail_count);

    if fail_count > 0 {
        println!("\n💡 Check logs at ~/.cache/joshify/joshify.log for details");
        std::process::exit(1);
    } else {
        println!("\n🎉 All searches working!");
        Ok(())
    }
}

// =============================================================================
// Tests for Auto-Advance and Queue Management
// =============================================================================

#[cfg(test)]
mod playback_tests {

    use joshify::playback::domain::{PlaybackContext, PlaybackQueue, QueueEntry};

    /// Test that PlaybackQueue correctly advances through context tracks
    #[test]
    fn test_queue_advances_through_context_tracks() {
        let mut queue = PlaybackQueue::new();

        // Set up a playlist context with 5 tracks
        queue.set_context(
            PlaybackContext::Playlist {
                uri: "spotify:playlist:test".to_string(),
                name: "Test Playlist".to_string(),
                start_index: 0,
            },
            vec![
                "spotify:track:1".to_string(),
                "spotify:track:2".to_string(),
                "spotify:track:3".to_string(),
                "spotify:track:4".to_string(),
                "spotify:track:5".to_string(),
            ],
        );

        // Verify initial state
        assert_eq!(queue.context_position(), 0);
        assert_eq!(queue.remaining_context_tracks(), 5);

        // Advance through tracks
        assert_eq!(queue.advance(), Some("spotify:track:1".to_string()));
        assert_eq!(queue.context_position(), 1);
        assert_eq!(queue.remaining_context_tracks(), 4);

        assert_eq!(queue.advance(), Some("spotify:track:2".to_string()));
        assert_eq!(queue.context_position(), 2);
        assert_eq!(queue.remaining_context_tracks(), 3);

        assert_eq!(queue.advance(), Some("spotify:track:3".to_string()));
        assert_eq!(queue.advance(), Some("spotify:track:4".to_string()));
        assert_eq!(queue.advance(), Some("spotify:track:5".to_string()));

        // Queue exhausted
        assert_eq!(queue.advance(), None);
        assert_eq!(queue.remaining_context_tracks(), 0);
    }

    /// Test that up_next queue takes priority over context tracks
    #[test]
    fn test_up_next_queue_priority() {
        let mut queue = PlaybackQueue::new();

        // Set up context
        queue.set_context(
            PlaybackContext::Playlist {
                uri: "spotify:playlist:test".to_string(),
                name: "Test Playlist".to_string(),
                start_index: 0,
            },
            vec![
                "spotify:track:ctx1".to_string(),
                "spotify:track:ctx2".to_string(),
            ],
        );

        // Add user-queued tracks
        queue.add_to_up_next(QueueEntry {
            uri: "spotify:track:queue1".to_string(),
            name: "Queue Track 1".to_string(),
            artist: "Artist".to_string(),
            album: None,
            duration_ms: None,
            added_by_user: true,
            is_recommendation: false,
        });

        queue.add_to_up_next(QueueEntry {
            uri: "spotify:track:queue2".to_string(),
            name: "Queue Track 2".to_string(),
            artist: "Artist".to_string(),
            album: None,
            duration_ms: None,
            added_by_user: true,
            is_recommendation: false,
        });

        // User queue plays first
        assert_eq!(queue.advance(), Some("spotify:track:queue1".to_string()));
        assert_eq!(queue.advance(), Some("spotify:track:queue2".to_string()));

        // Then context tracks
        assert_eq!(queue.advance(), Some("spotify:track:ctx1".to_string()));
        assert_eq!(queue.advance(), Some("spotify:track:ctx2".to_string()));

        // Exhausted
        assert_eq!(queue.advance(), None);
    }

    /// Test queue behavior when empty
    #[test]
    fn test_empty_queue_behavior() {
        let mut queue = PlaybackQueue::new();

        // Empty queue returns None
        assert_eq!(queue.advance(), None);
        assert_eq!(queue.remaining_context_tracks(), 0);
        assert!(queue.is_exhausted());

        // Add context
        queue.set_context(
            PlaybackContext::Playlist {
                uri: "spotify:playlist:test".to_string(),
                name: "Test".to_string(),
                start_index: 0,
            },
            vec!["spotify:track:1".to_string()],
        );

        assert!(!queue.is_exhausted());
        assert_eq!(queue.advance(), Some("spotify:track:1".to_string()));
        assert!(queue.is_exhausted());
    }

    /// Test that queue correctly tracks position after multiple advances
    #[test]
    fn test_queue_position_tracking() {
        let mut queue = PlaybackQueue::new();

        queue.set_context(
            PlaybackContext::Album {
                uri: "spotify:album:test".to_string(),
                name: "Test Album".to_string(),
            },
            (1..=10).map(|i| format!("spotify:track:{}", i)).collect(),
        );

        // Advance 5 times
        for i in 1..=5 {
            queue.advance();
            assert_eq!(queue.context_position(), i);
            assert_eq!(queue.remaining_context_tracks(), 10 - i);
        }

        // Current position should be 5, 5 tracks remaining
        assert_eq!(queue.context_position(), 5);
        assert_eq!(queue.remaining_context_tracks(), 5);
    }
}

#[cfg(test)]
mod audio_probe_tests {
    use joshify::player::AudioProbe;

    #[test]
    fn available_probe_produces_no_message() {
        assert_eq!(super::no_audio_message(&AudioProbe::Available), "");
    }

    #[test]
    fn unavailable_probe_explains_the_reason() {
        let msg = super::no_audio_message(&AudioProbe::Unavailable("no such device".into()));
        assert!(
            msg.contains("Remote playback only"),
            "should say which mode the user actually got: {msg}"
        );
        assert!(
            msg.contains("no such device"),
            "should carry the underlying reason: {msg}"
        );
    }

    // Serialized: probe_audio_output swaps the process-global panic hook, so
    // these must not run concurrently with each other.
    #[test]
    #[serial_test::serial(panic_hook)]
    fn probing_restores_the_previous_panic_hook() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        // Install a marker hook, probe, then confirm our marker is still the
        // one in force - the probe must not leave its muting hook behind, nor
        // revert to the default and discard ours.
        let fired = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&fired);
        let original = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |_| flag.store(true, Ordering::SeqCst)));

        let _ = joshify::player::probe_audio_output();

        let caught = std::panic::catch_unwind(|| panic!("marker"));
        assert!(caught.is_err(), "the panic should still have been caught");
        assert!(
            fired.load(Ordering::SeqCst),
            "probe_audio_output must restore the hook that was installed before it"
        );

        std::panic::set_hook(original);
    }

    #[test]
    #[serial_test::serial(panic_hook)]
    fn probing_audio_never_panics() {
        // CI runners have no sound card, so this exercises the Unavailable
        // path; on a desktop it exercises Available. Either is fine - the
        // point is that probing is safe to call unconditionally at startup.
        let _ = joshify::player::probe_audio_output();
    }
}

#[cfg(test)]
mod now_playing_regression_tests {
    /// This file's source, with this module removed so the assertions below do
    /// not match their own literals.
    fn program_source() -> &'static str {
        include_str!("main.rs")
            .split("mod now_playing_regression_tests")
            .next()
            .expect("split always yields at least one part")
    }

    /// Regression for #58: the album-detail header fabricated
    /// `artist: "Unknown".to_string()` because the load action dropped the real
    /// artist the caller already had.
    #[test]
    fn album_header_does_not_fabricate_an_unknown_artist() {
        let placeholder = format!("artist: {}Unknown{}.to_string()", '"', '"');
        assert!(
            !program_source().contains(&placeholder),
            "the album header must use the artist carried on LoadAction::AlbumTracks,              not a hardcoded placeholder (issue #58)"
        );
    }

    /// Regression for #58: the artist arrives on every TrackChanged event.
    #[test]
    fn track_changed_reads_the_artist() {
        assert!(
            program_source().contains("artist_from_unique_fields"),
            "the TrackChanged handler must set the artist from the event (issue #58)"
        );
    }

    /// Regression for #59: a Kitty payload must only be built when the terminal
    /// can display one, because the render loop space-fills the album-art
    /// rectangle before writing it and would otherwise erase the ASCII art.
    #[test]
    fn kitty_payload_is_gated_on_terminal_support() {
        let src = program_source();
        assert!(
            src.contains("supports_inline_image"),
            "the render path must consult Protocol::supports_inline_image (issue #59)"
        );

        // Every prepare_kitty_image call must sit behind the capability check.
        for (index, _) in src.match_indices("prepare_kitty_image") {
            let window_start = index.saturating_sub(200);
            let window = &src[window_start..index];
            assert!(
                window.contains("inline_images_supported"),
                "a prepare_kitty_image call is not gated on inline_images_supported                  (issue #59); ungated calls erase the ASCII fallback"
            );
        }
    }
}

#[cfg(test)]
mod version_flag_tests {
    use joshify::CliArgs;

    /// install.sh runs `joshify --version` to detect an existing install and to
    /// smoke-test a freshly downloaded binary before installing it. This was
    /// broken for an entire release: `--version` was only handled in
    /// `src/cli.rs`, which is unreachable from the binary (issue #48), so the
    /// flag fell through and launched the whole TUI. Every install silently
    /// fell back to a source build.
    #[test]
    fn version_flag_is_parsed_by_the_reachable_parser() {
        let src = include_str!("../src/lib.rs");
        assert!(
            src.contains("\"--version\" | \"-V\""),
            "CliArgs::parse must handle --version; the handler in src/cli.rs is \
             dead code and does not count (issue #48)"
        );
    }

    /// The output has to stay parseable: install.sh takes the last
    /// whitespace-separated field of the first line.
    #[test]
    fn version_output_last_field_is_the_crate_version() {
        let expected = env!("CARGO_PKG_VERSION");
        let line = format!("Joshify {expected}");
        let parsed = line.split_whitespace().last().expect("non-empty");
        assert_eq!(parsed, expected);
    }

    /// --version must be dispatched before anything initializes the terminal.
    #[test]
    fn version_is_handled_before_terminal_init() {
        let src = include_str!("main.rs");
        let main_body = src.split_once("async fn main()").expect("main exists").1;
        let version_branch = main_body
            .find("print_version")
            .expect("main should dispatch --version");
        let tui_init = main_body
            .find("ratatui::init()")
            .expect("main should eventually initialize the TUI");
        assert!(
            version_branch < tui_init,
            "--version must not start the TUI"
        );
    }

    #[test]
    fn version_flag_sets_the_field() {
        // Guards the struct/flag wiring itself.
        let args = CliArgs {
            version: true,
            ..Default::default()
        };
        assert!(args.version);
    }
}

#[cfg(test)]
mod radio_entries_tests {
    use super::radio_entries_from;
    use std::collections::HashSet;

    fn track(id: &str, name: &str) -> rspotify::model::FullTrack {
        let json = format!(
            r#"{{
              "album": {{
                "album_type": "album", "artists": [], "available_markets": [],
                "external_urls": {{}}, "href": "h", "id": "alb1", "images": [],
                "name": "An Album", "release_date": "2020-01-01",
                "release_date_precision": "day", "type": "album",
                "uri": "spotify:album:alb1"
              }},
              "artists": [{{
                "external_urls": {{}}, "href": "h", "id": "art1",
                "name": "An Artist", "type": "artist", "uri": "spotify:artist:art1"
              }}],
              "available_markets": [], "disc_number": 1, "duration_ms": 1000,
              "explicit": false, "external_ids": {{}}, "external_urls": {{}},
              "href": "h", "id": "{id}", "is_local": false, "name": "{name}",
              "popularity": 1, "preview_url": null, "track_number": 1,
              "type": "track", "uri": "spotify:track:{id}"
            }}"#
        );
        serde_json::from_str(&json).expect("fixture must deserialize")
    }

    #[test]
    fn builds_entries_marked_as_recommendations() {
        // The flag is what lets toggling radio off remove exactly these and
        // leave hand-queued tracks alone.
        let entries = radio_entries_from(&[track("t1", "One")], &HashSet::new());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].uri, "spotify:track:t1");
        assert_eq!(entries[0].name, "One");
        assert_eq!(entries[0].artist, "An Artist");
        assert!(entries[0].is_recommendation);
        assert!(!entries[0].added_by_user);
    }

    #[test]
    fn skips_tracks_already_queued_or_playing() {
        let mut exclude = HashSet::new();
        exclude.insert("spotify:track:t1".to_string());
        let entries = radio_entries_from(&[track("t1", "One"), track("t2", "Two")], &exclude);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].uri, "spotify:track:t2");
    }

    #[test]
    fn an_empty_seed_produces_nothing_to_queue() {
        // The caller relies on this to turn radio back off rather than leave a
        // badge lit over an empty station.
        assert!(radio_entries_from(&[], &HashSet::new()).is_empty());
    }

    #[test]
    fn everything_excluded_produces_nothing_to_queue() {
        let mut exclude = HashSet::new();
        exclude.insert("spotify:track:t1".to_string());
        assert!(radio_entries_from(&[track("t1", "One")], &exclude).is_empty());
    }
}

#[cfg(test)]
mod position_from_percent_tests {
    use super::position_from_percent;

    #[test]
    fn maps_the_ends_and_the_middle() {
        assert_eq!(position_from_percent(0, 200_000), 0);
        assert_eq!(position_from_percent(50, 200_000), 100_000);
        assert_eq!(position_from_percent(100, 200_000), 200_000);
    }

    #[test]
    fn never_runs_past_the_end_of_the_track() {
        assert_eq!(position_from_percent(200, 200_000), 200_000);
        assert_eq!(position_from_percent(u8::MAX, 200_000), 200_000);
    }

    #[test]
    fn a_long_track_does_not_overflow_the_multiply() {
        // u32 * 100 overflows 32 bits for anything over ~11.9 hours, so the
        // arithmetic has to widen before multiplying.
        assert_eq!(position_from_percent(100, u32::MAX), u32::MAX);
        assert_eq!(position_from_percent(50, u32::MAX), u32::MAX / 2);
    }

    #[test]
    fn a_track_with_no_duration_seeks_to_zero() {
        // Nothing is loaded yet - this must not divide by zero or panic.
        assert_eq!(position_from_percent(75, 0), 0);
    }
}

#[cfg(test)]
mod headless_setup_tests {
    /// `--setup` must be handled before anything touches the terminal, or the
    /// prompts it runs land in raw mode on the alternate screen (issue #47,
    /// same failure mode as #46).
    #[test]
    fn setup_flag_is_handled_before_terminal_init() {
        let src = include_str!("main.rs");
        let main_body = src
            .split_once("async fn main()")
            .expect("main should exist")
            .1;

        let setup_branch = main_body
            .find("run_setup_only")
            .expect("main should dispatch --setup");
        let tui_init = main_body
            .find("ratatui::init()")
            .expect("main should eventually initialize the TUI");

        assert!(
            setup_branch < tui_init,
            "--setup must be dispatched before ratatui::init(), so the setup \
             prompts run on a normal screen (issue #47)"
        );
    }
}

#[cfg(test)]
mod tui_init_order_tests {
    /// The source of this file, with this test module removed.
    ///
    /// These tests search main.rs for call-site patterns, and the patterns
    /// appear verbatim in the assertions below - so without this the module
    /// matches itself and the tests are meaningless.
    fn program_source() -> &'static str {
        include_str!("main.rs")
            .split("mod tui_init_order_tests")
            .next()
            .expect("split always yields at least one part")
    }

    /// The interactive setup prompts use `println!` and dialoguer, which are
    /// unreadable once the terminal is in raw mode on the alternate screen:
    /// `\n` stops implying a carriage return so text staircases, the cursor is
    /// hidden, and mouse capture injects escape sequences into stdin.
    ///
    /// This was issue #46. Ordering is easy to reintroduce by accident when
    /// editing `run_with_args`, and it cannot be caught by a behavioural test
    /// without a real terminal, so assert it against the source directly.
    #[test]
    fn tui_is_initialized_after_interactive_setup() {
        // Scope to run_with_args so the helper's own init/restore, which is
        // defined earlier in the file, does not confuse the comparison.
        let body = program_source()
            .split_once("async fn run_with_args")
            .expect("run_with_args should exist")
            .1;

        let setup = body
            .find("setup::ensure_configured")
            .expect("run_with_args should call ensure_configured");
        let init = body
            .find("ratatui::init()")
            .expect("run_with_args should initialize the terminal");

        assert!(
            setup < init,
            "ratatui::init() must come after setup::ensure_configured() in \
             run_with_args, or the first-run setup prompts render in raw mode \
             on the alternate screen and are unusable (issue #46)"
        );
    }

    /// The in-app settings key must not drop the user into dialoguer while the
    /// TUI still owns the terminal.
    #[test]
    fn in_app_setup_suspends_the_tui() {
        let code = program_source();

        let direct_call = format!("=> match joshify::setup::{}()", "run_setup");
        assert!(
            !code.contains(&direct_call),
            "the settings key must call run_setup() through suspend_tui(), not \
             directly inside the event loop (issue #46)"
        );

        let suspended = format!(
            "suspend_tui(&mut terminal, joshify::setup::{})",
            "run_setup"
        );
        assert!(
            code.contains(&suspended),
            "the settings key should run setup inside suspend_tui()"
        );
    }
}

#[cfg(test)]
mod play_locally_tests {
    use super::*;

    /// In local mode Enter must never turn into "go pick a device". With no
    /// local player the helper reports that plainly and leaves the player bar
    /// alone, so a caller that trusts its return value cannot claim playback.
    #[test]
    fn test_play_locally_without_player_reports_and_does_not_claim_playback() {
        let mut app = App::new();
        app.playback_mode = PlaybackMode::Local;
        assert!(app.local_player.is_none());

        let started = app.play_locally("Song", "Artist", "spotify:track:abc");

        assert!(!started);
        assert_eq!(
            app.status_message.as_deref(),
            Some("Local player not initialized")
        );
        assert!(!app.player_state.is_playing);
        assert!(app.player_state.current_track_uri.is_none());
        let msg = app.status_message.unwrap();
        assert!(
            !msg.contains("press 'd'"),
            "local mode must not send the user to the device picker: {msg}"
        );
    }
}

#[cfg(test)]
mod play_path_invariants {
    /// Every way to start a track goes through `play_track`, which is where
    /// "local by default, remote only after 'd'" is decided. 0.8.3 shipped one
    /// Enter handler that called the remote path directly and told local users
    /// to pick a device. A new direct call to either half fails here.
    ///
    /// The needles are assembled at compile time so this test's own source does
    /// not count as a call site.
    #[test]
    fn remote_play_is_only_reachable_through_play_track() {
        let src = include_str!("main.rs");
        let needle = concat!("spawn_remote_", "play(");
        let count = src.matches(needle).count();
        // The fn definition and the single call inside play_track.
        assert_eq!(
            count, 2,
            "spawn_remote_play must only be called from play_track; found {} occurrences",
            count
        );
    }

    #[test]
    fn local_play_is_only_reachable_through_play_track() {
        let src = include_str!("main.rs");
        let needle = concat!("play_", "locally(");
        let count = src.matches(needle).count();
        // The fn definition, the call inside play_track, and the unit tests
        // below that exercise it directly.
        let in_tests = src
            .split_once("mod play_locally_tests")
            .map(|(_, tests)| tests.matches(needle).count())
            .unwrap_or(0);
        assert_eq!(
            count - in_tests,
            2,
            "play_locally must only be called from play_track; found {} non-test occurrences",
            count - in_tests
        );
    }
}

#[cfg(test)]
mod play_track_tests {
    use super::*;

    fn feedback() -> tokio::sync::mpsc::Sender<PlaybackFeedback> {
        tokio::sync::mpsc::channel(1).0
    }

    /// Local mode with no client must still try the local player - it must
    /// never route around it to Spotify, and never mention the device picker.
    #[test]
    fn local_mode_stays_local_even_with_a_client_absent() {
        let mut app = App::new();
        app.playback_mode = PlaybackMode::Local;
        let outcome = play_track(
            &mut app,
            None,
            ("Song".into(), "Artist".into(), "spotify:track:abc".into()),
            None,
            &feedback(),
        );
        assert_eq!(outcome, PlayOutcome::NotStarted);
        let msg = app.status_message.clone().unwrap_or_default();
        assert_eq!(msg, "Local player not initialized");
        assert!(!msg.contains("press 'd'"), "{msg}");
    }

    /// Remote mode without a Spotify client cannot play and must say so rather
    /// than report "Starting" or silently do nothing.
    #[test]
    fn remote_mode_without_a_client_explains_itself() {
        let mut app = App::new();
        app.playback_mode = PlaybackMode::Remote;
        let outcome = play_track(
            &mut app,
            None,
            ("Song".into(), "Artist".into(), "spotify:track:abc".into()),
            None,
            &feedback(),
        );
        assert_eq!(outcome, PlayOutcome::NotStarted);
        assert_eq!(
            app.status_message.as_deref(),
            Some("Not connected to Spotify")
        );
        assert!(!app.player_state.is_playing);
    }
}
