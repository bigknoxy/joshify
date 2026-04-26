use anyhow::Result;
use joshify::auth::OAuthConfig;
use joshify::player::LocalPlayer;
use joshify::session::LocalSession;
use joshify::state::app_state::{PlaylistListItem, TrackListItem};
use joshify::state::player_state::PlayerState;
use joshify::state::search_state::SearchState;
use joshify::state::{ContentState, FocusTarget, LoadAction, NavItem};
use joshify::CliArgs;
use librespot::core::authentication::Credentials;
use rspotify::clients::OAuthClient;
use std::sync::Arc;

/// Playback mode - local or remote
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum PlaybackMode {
    #[default]
    Local,
    Remote,
}

/// Highlighted item in the current view (for queue operations)
#[derive(Debug, Clone)]
struct HighlightedItem {
    uri: String,
    name: String,
    artist: String,
    _context: Option<PlaybackContext>,
}

/// Playback context - what collection the current track came from
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum PlaybackContext {
    Playlist {
        uri: String,
        name: String,
        track_index: usize,
    },
    Album {
        uri: String,
        name: String,
    },
    Artist {
        uri: String,
        name: String,
    },
}

/// Application state
struct App {
    selected_nav: NavItem,
    is_authenticated: bool,
    player_state: PlayerState,
    queue_state: joshify::state::queue_state::QueueState,
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
    local_session: Option<Arc<LocalSession>>,
    local_player: Option<Arc<LocalPlayer>>,
    player_event_rx:
        Option<tokio::sync::mpsc::UnboundedReceiver<librespot::playback::player::PlayerEvent>>,
    loading_more_liked_songs: bool,
    /// Layout cache for mouse hit testing
    layout_cache: joshify::ui::LayoutCache,
    /// Mouse state for tracking double-clicks
    mouse_state: joshify::ui::MouseState,
}

impl App {
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
            local_session: None,
            local_player: None,
            player_event_rx: None,
            loading_more_liked_songs: false,
            layout_cache: joshify::ui::LayoutCache::new(),
            mouse_state: joshify::ui::MouseState::new(),
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
            ContentState::LikedSongs(t) | ContentState::LikedSongsPage { tracks: t, .. } => Some((t.as_slice(), None::<&str>)),
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

                // Update playlist context track_index when navigating
                if let Some(PlaybackContext::Playlist { uri, name, .. }) = &self.current_context {
                    self.current_context = Some(PlaybackContext::Playlist {
                        uri: uri.clone(),
                        name: name.clone(),
                        track_index: self.selected_index,
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
        let client_guard = client.lock().await;
        match client_guard.current_playback().await {
            Ok(Some(ctx)) => {
                let old_track_uri = self.player_state.current_track_uri.clone();
                self.player_state = PlayerState::from_context(&ctx);

                let new_track_uri = self.player_state.current_track_uri.clone();
                if old_track_uri != new_track_uri {
                    self.player_state.reset_scroll();
                }
                let new_album_art_url = self.player_state.current_album_art_url.clone();

                if new_track_uri != old_track_uri
                    && new_track_uri.is_some()
                    && new_album_art_url.is_some()
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
                    }
                }
            }
            Ok(None) => {
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

    // Handle --test-search flag (test search API without TUI)
    if args.test_search {
        return run_search_test(args).await;
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

    // Initialize terminal
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    ratatui::init();

    // Enable mouse capture and hide cursor
    crossterm::execute!(io::stdout(), crossterm::event::EnableMouseCapture)?;
    crossterm::execute!(io::stdout(), crossterm::cursor::Hide)?;

    let mut app = App::new();

    // If we have tokens from env/CLI, skip interactive setup
    if has_tokens {
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

    // Clear any leftover output and force redraw
    terminal.clear()?;

    // Initialize Spotify client wrapped in Arc<Mutex> for shared access
    let client = match joshify::api::SpotifyClient::new(&config).await {
        Ok(client) => {
            app.is_authenticated = true;
            app.status_message = Some("Connected to Spotify - Press ? for help".to_string());
            Some(Arc::new(Mutex::new(client)))
        }
        Err(e) => {
            app.status_message = Some(format!("Spotify auth error: {}", e));
            None
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

    if let Some(ref token) = access_token {
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

        // Auto-clear expired search errors
        if let Some(expiry) = app.search_state.error_display_until_ms {
            if now >= expiry {
                app.search_state.error = None;
                app.search_state.error_display_until_ms = None;
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
                        app.search_state.set_error(error);
                        app.search_state.error_display_until_ms = Some(now + 5000);
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
                    if let ContentState::LikedSongsPage { tracks: new_tracks, total, next_offset } = other {
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
                    app.player_state.current_album_art_kitty =
                        joshify::ui::image_renderer::prepare_kitty_image(&art_data, album_area);
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
                    app.player_state.current_album_art_kitty =
                        joshify::ui::image_renderer::prepare_kitty_image(
                            art_data,
                            current_album_area,
                        );
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

            // Process batched events (reuse buffer, no allocations)
            for event in app.event_batch.iter() {
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
                    PlayerEvent::Stopped { .. } | PlayerEvent::EndOfTrack { .. } => {
                        app.player_state.is_playing = false;
                        // Auto-advance to next queue item when track ends
                        if !app.queue_state.local_queue.is_empty() {
                            if let Some(next_entry) = app.queue_state.next_track() {
                                if let Some(ref player) = app.local_player {
                                    match player.load_uri(&next_entry.uri, true, 0) {
                                        Ok(_) => {
                                            app.player_state.current_track_name =
                                                Some(next_entry.name.clone());
                                            app.player_state.current_artist_name =
                                                Some(next_entry.artist.clone());
                                            app.player_state.current_track_uri =
                                                Some(next_entry.uri.clone());
                                            app.player_state.is_playing = true;
                                            app.player_state.progress_ms = 0;
                                            app.status_message = Some(format!(
                                                "Playing next from queue: {}",
                                                next_entry.name
                                            ));
                                            tracing::info!(
                                                "Auto-advanced to queue item: {}",
                                                next_entry.name
                                            );
                                        }
                                        Err(e) => {
                                            app.status_message =
                                                Some(format!("Queue playback error: {}", e));
                                            tracing::warn!("Queue playback failed: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    PlayerEvent::TrackChanged { audio_item } => {
                        app.player_state.current_track_name = Some(audio_item.name.clone());
                        app.player_state.duration_ms = audio_item.duration_ms;
                        app.player_state.current_track_uri = Some(audio_item.uri.clone());
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
                    | PlayerEvent::PositionChanged { position_ms, .. }
                    | PlayerEvent::PositionCorrection { position_ms, .. } => {
                        app.player_state.progress_ms = *position_ms;
                    }
                    PlayerEvent::Loading {
                        track_id,
                        position_ms,
                        ..
                    } => {
                        app.player_state.current_track_uri = Some(track_id.to_uri());
                        app.player_state.progress_ms = *position_ms;
                    }
                    _ => {}
                }
            }
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
            if let Some(ref client) = client {
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

                joshify::ui::render_sidebar(frame, sidebar, app.selected_nav, sidebar_focused, &mut app.layout_cache);
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
                    joshify::ui::render_queue_overlay(frame, area, &app.queue_state);
                }
                if let (Some(ref content), Some(ref mut state)) = (&app.help_content, &mut app.help_state) {
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
        // Important: Must delete the old Kitty image before drawing at a new position.
        // Kitty images persist on screen until explicitly deleted. On resize, we use
        // the Kitty delete protocol command which removes only the image pixels without
        // affecting surrounding text content.
        if let Some(ref kitty_data) = app.player_state.current_album_art_kitty {
            // Delete the old Kitty image using the Kitty graphics protocol delete command.
            // This only removes the image in the specified area, not surrounding text.
            if let Some(old_area) = app.player_state.last_kitty_render_area {
                let _ = joshify::ui::image_renderer::delete_kitty_image_in_area(old_area);
                // Also clear the area with spaces as a fallback for non-Kitty terminals
                let _ = joshify::ui::image_renderer::clear_terminal_area(old_area);
            }
            let _ = joshify::ui::image_renderer::write_prepared_kitty_image(kitty_data);
            // Record where we just rendered so we can delete it next time
            if let Some(frame_area) = app.area {
                let player_bar_height = 6u16;
                let sidebar_width = 20u16;
                let album_art_width = 12u16;
                app.player_state.last_kitty_render_area = Some(Rect::new(
                    sidebar_width,
                    frame_area.height.saturating_sub(player_bar_height),
                    album_art_width,
                    player_bar_height,
                ));
            }
        } else {
            // No current image - clear any previous render area
            if let Some(old_area) = app.player_state.last_kitty_render_area.take() {
                let _ = joshify::ui::image_renderer::delete_kitty_image_in_area(old_area);
                let _ = joshify::ui::image_renderer::clear_terminal_area(old_area);
            }
        }

        // Handle async data loading based on current state
        // Only spawn tasks when in Loading state, not LoadingInProgress (prevents duplicate spawns)
        let load_action = match &app.content_state {
            ContentState::Loading(action) => Some(action.clone()),
            _ => None,
        };

        if let Some(action) = load_action {
            if let Some(ref client) = client {
                match action {
                    LoadAction::Devices => {
                        let c = client.clone();
                        let tx_clone = tx.clone();
                        let has_local = app.playback_mode == PlaybackMode::Local;
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
                                    active: true,
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
                                    let _ = tx_clone.send(ContentState::LikedSongsPage {
                                        tracks: items,
                                        total,
                                        next_offset,
                                    }).await;
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
                                    let _ = tx_clone.send(ContentState::LikedSongsPage {
                                        tracks: items,
                                        total,
                                        next_offset,
                                    }).await;
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
                            track_index: 0,
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
                                if let Some(track) = app.search_state.selected_track() {
                                    if let Some(ref client) = client {
                                        let c = client.clone();
                                        let uri = track.uri.clone();
                                        let tx_clone = tx.clone();
                                        tokio::spawn(async move {
                                            let guard = c.lock().await;
                                            if let Ok(devices) = guard.available_devices().await {
                                                if let Some(device) = devices.first() {
                                                    if let Some(ref device_id) = device.id {
                                                        let _ = guard
                                                            .transfer_playback(device_id)
                                                            .await;
                                                    }
                                                }
                                            }
                                            if let Err(e) =
                                                guard.start_playback(vec![uri], None).await
                                            {
                                                tracing::error!("Search playback error: {}", e);
                                                let _ = tx_clone
                                                    .send(ContentState::SearchErrorLive(format!(
                                                        "Playback failed: {}",
                                                        e
                                                    )))
                                                    .await;
                                            }
                                        });
                                    }
                                    app.status_message = Some(format!("Playing: {}", track.name));
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
                                    if let Some(ref client) = client {
                                        let c = client.clone();
                                        let uri = track.uri.clone();
                                        tokio::spawn(async move {
                                            let guard = c.lock().await;
                                            let _ = guard.add_to_queue(&uri).await;
                                        });
                                    }
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
                        match key.code {
                            crossterm::event::KeyCode::Esc => {
                                app.show_queue = false;
                                continue;
                            }
                            crossterm::event::KeyCode::Char('c') => {
                                app.queue_state.clear();
                                app.status_message = Some("Queue cleared".to_string());
                                continue;
                            }
                            crossterm::event::KeyCode::Char('D') => {
                                // Remove highlighted item from queue
                                if let Some(ref highlighted) = app.highlighted_item {
                                    let idx = app
                                        .queue_state
                                        .local_queue
                                        .iter()
                                        .position(|e| e.uri == highlighted.uri);
                                    if let Some(i) = idx {
                                        app.queue_state.local_queue.remove(i);
                                        app.status_message = Some(format!(
                                            "Removed from queue: {}",
                                            highlighted.name
                                        ));
                                    }
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
                                                app.playback_mode = PlaybackMode::Local;
                                                app.status_message =
                                                    Some("Switched to local playback".to_string());
                                            }
                                            joshify::state::app_state::DeviceEntry::Remote(
                                                device,
                                            ) => {
                                                if let Some(ref device_id) = device.id {
                                                    if let Some(ref client) = client {
                                                        let c = client.clone();
                                                        let device_id = device_id.clone();
                                                        let device_name = device.name.clone();
                                                        tokio::spawn(async move {
                                                            let guard = c.lock().await;
                                                            let _ = guard.transfer_playback(&device_id).await;
                                                        });
                                                        app.playback_mode = PlaybackMode::Remote;
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
                            let c = client.clone();
                            let is_playing = app.player_state.is_playing;
                            tokio::spawn(async move {
                                let guard = c.lock().await;
                                if is_playing {
                                    let _ = guard.playback_pause().await;
                                } else {
                                    let _ = guard.playback_resume().await;
                                }
                            });
                        }
                        continue;
                    }

                    // Shuffle toggle (s) - works from ANY focus
                    if key.code == crossterm::event::KeyCode::Char('s') {
                        if let Some(ref client) = client {
                            let new_shuffle = !app.player_state.shuffle;
                            app.player_state.shuffle = new_shuffle;
                            let c = client.clone();
                            tokio::spawn(async move {
                                let guard = c.lock().await;
                                let _ = guard.toggle_shuffle(new_shuffle).await;
                            });
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
                        if let Some(ref client) = client {
                            app.player_state.repeat_mode = app.player_state.repeat_mode.cycle();
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
                            let c = client.clone();
                            tokio::spawn(async move {
                                let guard = c.lock().await;
                                let _ = guard.set_repeat(spotify_state).await;
                            });
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
                        app.status_message = Some(if app.queue_state.radio_mode {
                            "Radio Mode: ON".to_string()
                        } else {
                            "Radio Mode: OFF".to_string()
                        });
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
                                    // Select current nav item - show content
                                    app.loading_more_liked_songs = false;
                                    match app.selected_nav {
                                        joshify::ui::NavItem::LikedSongs => {
                                            app.content_state =
                                                ContentState::Loading(LoadAction::LikedSongs);
                                            app.selected_index = 0;
                                            app.scroll_offset = 0;
                                        }
                                        joshify::ui::NavItem::Playlists => {
                                            app.content_state =
                                                ContentState::Loading(LoadAction::Playlists);
                                            app.selected_index = 0;
                                            app.scroll_offset = 0;
                                        }
                                        joshify::ui::NavItem::Home => {
                                            app.content_state = ContentState::Home;
                                        }
                                        joshify::ui::NavItem::Library => {
                                            app.content_state =
                                                ContentState::Loading(LoadAction::LibraryAlbums);
                                            app.selected_index = 0;
                                            app.scroll_offset = 0;
                                        }
                                    }
                                }
                                FocusTarget::MainContent => {
                                    if let ContentState::LikedSongsPage { tracks, next_offset: Some(offset), .. } = &app.content_state {
                                        if !app.loading_more_liked_songs && app.selected_index >= tracks.len().saturating_sub(3) {
                                            let load_offset = *offset;
                                            app.loading_more_liked_songs = true;
                                            if let Some(ref client) = client {
                                                let c = client.clone();
                                                let tx_clone = tx.clone();
                                                tokio::spawn(async move {
                                                    let guard = c.lock().await;
                                                    match guard.current_user_saved_tracks_paginated(50, load_offset).await {
                                                        Ok((tracks, total, next_offset)) => {
                                                            let items: Vec<TrackListItem> = tracks
                                                                .into_iter()
                                                                .filter_map(|t| {
                                                                    t.track.id.map(|id| {
                                                                        let artist = t.track.artists.first().map(|a| a.name.clone()).unwrap_or_default();
                                                                        TrackListItem {
                                                                            name: t.track.name,
                                                                            artist,
                                                                            uri: format!("spotify:track:{}", id.id()),
                                                                        }
                                                                    })
                                                                })
                                                                .collect();
                                                            let _ = tx_clone.send(ContentState::LikedSongsPage {
                                                                tracks: items,
                                                                total,
                                                                next_offset,
                                                            }).await;
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

                                                // Track the highlighted item for queue operations
                                                app.highlighted_item = Some(HighlightedItem {
                                                    uri: track.uri.clone(),
                                                    name: track.name.clone(),
                                                    artist: track.artist.clone(),
                                                    _context: app.current_context.clone(),
                                                });

                                                if app.playback_mode == PlaybackMode::Local {
                                                    // Play locally with librespot
                                                    if let Some(ref player) = app.local_player {
                                                        match player.load_uri(&track.uri, true, 0) {
                                                            Ok(_) => {
                                                                app.player_state
                                                                    .current_track_name =
                                                                    Some(track.name.clone());
                                                                app.player_state
                                                                    .current_artist_name =
                                                                    Some(track.artist.clone());
                                                                app.player_state
                                                                    .current_track_uri =
                                                                    Some(track.uri.clone());
                                                                app.player_state.is_playing = true;
                                                                app.player_state.progress_ms = 0;
                                                                app.status_message = Some(format!(
                                                                    "Playing locally: {}",
                                                                    track.name
                                                                ));
                                                            }
                                                            Err(e) => {
                                                                app.status_message = Some(format!(
                                                                    "Local playback error: {}",
                                                                    e
                                                                ));
                                                            }
                                                        }
                                                    } else {
                                                        app.status_message = Some(
                                                            "Local player not initialized"
                                                                .to_string(),
                                                        );
                                                    }
                                                } else {
                                                    // Remote playback via Spotify API
                                                    if let Some(ref client) = client {
                                                        let c = client.clone();
                                                        let track_uri = track.uri.clone();
                                                        let track_name = track.name.clone();
                                                        let context = app.current_context.clone();
                                                        tokio::spawn(async move {
                                                            let guard = c.lock().await;
                                                            if let Ok(devices) =
                                                                guard.available_devices().await
                                                            {
                                                                if let Some(device) =
                                                                    devices.first()
                                                                {
                                                                    if let Some(ref device_id) =
                                                                        device.id
                                                                    {
                                                                        let _ = guard
                                                                            .transfer_playback(
                                                                                device_id,
                                                                            )
                                                                            .await;
                                                                    }
                                                                }
                                                            }
                                                            if let Some(
                                                                PlaybackContext::Playlist {
                                                                    uri,
                                                                    ..
                                                                },
                                                            ) = &context
                                                            {
                                                                let playlist_id_str = uri
                                                                    .strip_prefix(
                                                                        "spotify:playlist:",
                                                                    )
                                                                    .unwrap_or(uri);
                                                                if let Ok(playlist_id) =
                                                                    rspotify::model::PlaylistId::from_id(
                                                                        playlist_id_str,
                                                                    )
                                                                {
                                                                    let _ = guard.oauth.start_context_playback(
                                                                        rspotify::model::PlayContextId::from(playlist_id),
                                                                        None,
                                                                        Some(rspotify::model::Offset::Uri(track_uri.clone())),
                                                                        None,
                                                                    ).await;
                                                                } else {
                                                                    let _ = guard.start_playback(vec![track_uri], None).await;
                                                                }
                                                            } else {
                                                                let _ = guard
                                                                    .start_playback(
                                                                        vec![track_uri],
                                                                        None,
                                                                    )
                                                                    .await;
                                                            }
                                                        });
                                                        app.status_message = Some(format!(
                                                            "Playing: {}",
                                                            track_name
                                                        ));
                                                    }
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
                                        let c = client.clone();
                                        let is_playing = app.player_state.is_playing;
                                        tokio::spawn(async move {
                                            let guard = c.lock().await;
                                            if is_playing {
                                                let _ = guard.playback_pause().await;
                                            } else {
                                                let _ = guard.playback_resume().await;
                                            }
                                        });
                                    }
                                }
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
                                    ContentState::LikedSongs(t) | ContentState::LikedSongsPage { tracks: t, .. } => t.len(),
                                    ContentState::Playlists(p) => p.len(),
                                    ContentState::PlaylistTracks(_, t) => t.len(),
                                    ContentState::SearchResults(_, t) => t.len(),
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
                                    if let ContentState::LikedSongsPage { next_offset: Some(offset), .. } = &app.content_state {
                                        if !app.loading_more_liked_songs && app.selected_index >= len.saturating_sub(5) {
                                            let load_offset = *offset;
                                            app.loading_more_liked_songs = true;
                                            if let Some(ref client) = client {
                                                let c = client.clone();
                                                let tx_clone = tx.clone();
                                                tokio::spawn(async move {
                                                    let guard = c.lock().await;
                                                    match guard.current_user_saved_tracks_paginated(50, load_offset).await {
                                                        Ok((tracks, total, next_offset)) => {
                                                            let items: Vec<TrackListItem> = tracks
                                                                .into_iter()
                                                                .filter_map(|t| {
                                                                    t.track.id.map(|id| {
                                                                        let artist = t.track.artists.first().map(|a| a.name.clone()).unwrap_or_default();
                                                                        TrackListItem {
                                                                            name: t.track.name,
                                                                            artist,
                                                                            uri: format!("spotify:track:{}", id.id()),
                                                                        }
                                                                    })
                                                                })
                                                                .collect();
                                                            let _ = tx_clone.send(ContentState::LikedSongsPage {
                                                                tracks: items,
                                                                total,
                                                                next_offset,
                                                            }).await;
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
                                app.player_state.volume = app.player_state.volume.saturating_sub(5);
                                if app.playback_mode == PlaybackMode::Local {
                                    if let Some(ref player) = app.local_player {
                                        let new_vol = app.player_state.volume as u16 * 65535 / 100;
                                        player.set_volume(new_vol);
                                    }
                                } else if let Some(ref client) = client {
                                    let new_vol = app.player_state.volume;
                                    let c = client.clone();
                                    tokio::spawn(async move {
                                        let guard = c.lock().await;
                                        let _ = guard.set_volume(new_vol).await;
                                    });
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
                                    ContentState::LikedSongs(t) | ContentState::LikedSongsPage { tracks: t, .. } => t.len(),
                                    ContentState::Playlists(p) => p.len(),
                                    ContentState::PlaylistTracks(_, t) => t.len(),
                                    ContentState::SearchResults(_, t) => t.len(),
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
                                app.player_state.volume = (app.player_state.volume + 5).min(100);
                                if app.playback_mode == PlaybackMode::Local {
                                    if let Some(ref player) = app.local_player {
                                        let new_vol = app.player_state.volume as u16 * 65535 / 100;
                                        player.set_volume(new_vol);
                                    }
                                } else if let Some(ref client) = client {
                                    let new_vol = app.player_state.volume;
                                    let c = client.clone();
                                    tokio::spawn(async move {
                                        let guard = c.lock().await;
                                        if let Err(e) = guard.set_volume(new_vol).await {
                                            tracing::error!("Volume up failed: {}", e);
                                        }
                                    });
                                }
                            }
                        }

                        // Playback controls (work from any focus)
                        crossterm::event::KeyCode::Char('n') => {
                            if app.playback_mode == PlaybackMode::Local {
                                if let Some(ref player) = app.local_player {
                                    player.stop();
                                }
                            } else if let Some(ref client) = client {
                                let c = client.clone();
                                tokio::spawn(async move {
                                    let guard = c.lock().await;
                                    let _ = guard.playback_next().await;
                                });
                            }
                        }
                        crossterm::event::KeyCode::Char('p') => {
                            if let Some(ref client) = client {
                                let c = client.clone();
                                tokio::spawn(async move {
                                    let guard = c.lock().await;
                                    let _ = guard.playback_previous().await;
                                });
                            }
                        }
                        crossterm::event::KeyCode::Left => {
                            if app.playback_mode == PlaybackMode::Local {
                                if let Some(ref player) = app.local_player {
                                    let new_pos =
                                        app.player_state.progress_ms.saturating_sub(10000);
                                    player.seek(new_pos);
                                }
                                 } else if let Some(ref client) = client {
                                        let new_vol = app.player_state.volume;
                                        let c = client.clone();
                                        tokio::spawn(async move {
                                            let guard = c.lock().await;
                                            if let Err(e) = guard.set_volume(new_vol).await {
                                                tracing::error!("Volume down failed: {}", e);
                                            }
                                        });
                            }
                        }
                        crossterm::event::KeyCode::Right => {
                            if app.playback_mode == PlaybackMode::Local {
                                if let Some(ref player) = app.local_player {
                                    let new_pos = app
                                        .player_state
                                        .progress_ms
                                        .saturating_add(10000)
                                        .min(app.player_state.duration_ms);
                                    player.seek(new_pos);
                                }
                            } else if let Some(ref client) = client {
                                let new_pos = app
                                    .player_state
                                    .progress_ms
                                    .saturating_add(10000)
                                    .min(app.player_state.duration_ms);
                                let c = client.clone();
                                tokio::spawn(async move {
                                    let guard = c.lock().await;
                                    let _ = guard.seek(new_pos, None).await;
                                });
                            }
                        }
                        crossterm::event::KeyCode::Char('+') => {
                            app.player_state.volume = (app.player_state.volume + 5).min(100);
                            if app.playback_mode == PlaybackMode::Local {
                                if let Some(ref player) = app.local_player {
                                    let new_vol = app.player_state.volume as u16 * 65535 / 100;
                                    player.set_volume(new_vol);
                                }
                            } else if let Some(ref client) = client {
                                let new_vol = app.player_state.volume;
                                let c = client.clone();
                                tokio::spawn(async move {
                                    let guard = c.lock().await;
                                    if let Err(e) = guard.set_volume(new_vol).await {
                                        tracing::error!("Volume up (+) failed: {}", e);
                                    }
                                });
                            }
                        }
                        crossterm::event::KeyCode::Char('-') => {
                            app.player_state.volume = app.player_state.volume.saturating_sub(5);
                            if app.playback_mode == PlaybackMode::Local {
                                if let Some(ref player) = app.local_player {
                                    let new_vol = app.player_state.volume as u16 * 65535 / 100;
                                    player.set_volume(new_vol);
                                }
                            } else if let Some(ref client) = client {
                                let new_vol = app.player_state.volume;
                                let c = client.clone();
                                tokio::spawn(async move {
                                    let guard = c.lock().await;
                                    if let Err(e) = guard.set_volume(new_vol).await {
                                        tracing::error!("Volume down (-) failed: {}", e);
                                    }
                                });
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

                        // Settings
                        crossterm::event::KeyCode::Char('c') => match joshify::setup::run_setup() {
                            Ok(_) => {
                                app.status_message =
                                    Some("Config updated - restart app to apply".to_string());
                            }
                            Err(_) => {
                                app.status_message = Some("Setup cancelled".to_string());
                            }
                        },

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
                                    app.content_state = ContentState::Loading(joshify::state::LoadAction::LikedSongs);
                                    app.selected_index = 0;
                                    app.scroll_offset = 0;
                                }
                                NavItem::Playlists => {
                                    app.content_state = ContentState::Loading(joshify::state::LoadAction::Playlists);
                                    app.selected_index = 0;
                                    app.scroll_offset = 0;
                                }
                                NavItem::Home => {
                                    app.content_state = ContentState::Home;
                                }
                                NavItem::Library => {
                                    app.content_state = ContentState::Loading(joshify::state::LoadAction::LibraryAlbums);
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
                                    if let ContentState::PlaylistTracks(playlist_id, _) = &app.content_state {
                                        let playlist_uri = format!("spotify:playlist:{}", playlist_id);
                                        app.current_context = Some(PlaybackContext::Playlist {
                                            uri: playlist_uri,
                                            name: playlist_id.clone(),
                                            track_index: index,
                                        });
                                    }

                                    // Track the highlighted item for queue operations
                                    app.highlighted_item = Some(HighlightedItem {
                                        uri: track.uri.clone(),
                                        name: track.name.clone(),
                                        artist: track.artist.clone(),
                                        _context: app.current_context.clone(),
                                    });

                                    if app.playback_mode == PlaybackMode::Local {
                                        // Play locally with librespot
                                        if let Some(ref player) = app.local_player {
                                            match player.load_uri(&track.uri, true, 0) {
                                                Ok(_) => {
                                                    app.player_state.current_track_name = Some(track.name.clone());
                                                    app.player_state.current_artist_name = Some(track.artist.clone());
                                                    app.player_state.current_track_uri = Some(track.uri.clone());
                                                    app.player_state.is_playing = true;
                                                    app.player_state.progress_ms = 0;
                                                    app.status_message = Some(format!(
                                                        "Playing locally: {}",
                                                        track.name
                                                    ));
                                                }
                                                Err(e) => {
                                                    app.status_message = Some(format!(
                                                        "Local playback error: {}",
                                                        e
                                                    ));
                                                }
                                            }
                                        } else {
                                            app.status_message = Some(
                                                "Local player not initialized".to_string(),
                                            );
                                        }
                                    } else {
                                        // Remote playback via Spotify API
                                        if let Some(ref client) = client {
                                            let c = client.clone();
                                            let track_uri = track.uri.clone();
                                            let track_name = track.name.clone();
                                            let context = app.current_context.clone();
                                            let playlist_id_for_context = if let ContentState::PlaylistTracks(pid, _) = &app.content_state {
                                                Some(pid.clone())
                                            } else {
                                                None
                                            };

                                            tokio::spawn(async move {
                                                let guard = c.lock().await;
                                                if let Ok(devices) = guard.available_devices().await {
                                                    if let Some(device) = devices.first() {
                                                        if let Some(ref device_id) = device.id {
                                                            let _ = guard.transfer_playback(device_id).await;
                                                        }
                                                    }
                                                }

                                                // Use playlist context if available
                                                if let Some(pid) = playlist_id_for_context {
                                                    let _playlist_uri = format!("spotify:playlist:{}", pid);
                                                    if let Ok(playlist_id) = rspotify::model::PlaylistId::from_id(&pid) {
                                                        let _ = guard.oauth.start_context_playback(
                                                            rspotify::model::PlayContextId::from(playlist_id),
                                                            None,
                                                            Some(rspotify::model::Offset::Uri(track_uri.clone())),
                                                            None,
                                                        ).await;
                                                    } else {
                                                        // Fallback to direct track playback
                                                        let _ = guard.start_playback(vec![track_uri], None).await;
                                                    }
                                                } else if let Some(PlaybackContext::Playlist { uri, .. }) = &context {
                                                    // Use existing context if available
                                                    let playlist_id_str = uri.strip_prefix("spotify:playlist:").unwrap_or(uri);
                                                    if let Ok(playlist_id) = rspotify::model::PlaylistId::from_id(playlist_id_str) {
                                                        let _ = guard.oauth.start_context_playback(
                                                            rspotify::model::PlayContextId::from(playlist_id),
                                                            None,
                                                            Some(rspotify::model::Offset::Uri(track_uri.clone())),
                                                            None,
                                                        ).await;
                                                    } else {
                                                        let _ = guard.start_playback(vec![track_uri], None).await;
                                                    }
                                                } else {
                                                    // No context - play track directly
                                                    let _ = guard.start_playback(vec![track_uri], None).await;
                                                }
                                            });
                                            app.status_message = Some(format!("Playing: {}", track_name));
                                        }
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
                                let c = client.clone();
                                let is_playing = app.player_state.is_playing;
                                tokio::spawn(async move {
                                    let guard = c.lock().await;
                                    if is_playing {
                                        let _ = guard.playback_pause().await;
                                    } else {
                                        let _ = guard.playback_resume().await;
                                    }
                                });
                            }
                        }
                        joshify::ui::MouseAction::SkipNext => {
                            // Next track
                            if let Some(ref client) = client {
                                let c = client.clone();
                                tokio::spawn(async move {
                                    let guard = c.lock().await;
                                    let _ = guard.playback_next().await;
                                });
                            }
                        }
                        joshify::ui::MouseAction::SkipPrevious => {
                            // Previous track
                            if let Some(ref client) = client {
                                let c = client.clone();
                                tokio::spawn(async move {
                                    let guard = c.lock().await;
                                    let _ = guard.playback_previous().await;
                                });
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
                                    let current_idx = nav_items.iter().position(|&n| n == app.selected_nav).unwrap_or(0);
                                    if current_idx > 0 {
                                        app.selected_nav = nav_items[current_idx - 1];
                                    }
                                }
                                FocusTarget::MainContent => {
                                    // Scroll up in list
                                    if app.selected_index > 0 {
                                        app.selected_index -= 1;
                                        if app.selected_index < app.scroll_offset {
                                            app.scroll_offset = app.selected_index;
                                        }
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
                                    let current_idx = nav_items.iter().position(|&n| n == app.selected_nav).unwrap_or(0);
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
                            let new_volume = (app.player_state.volume as i32 + delta).clamp(0, 100) as u32;
                            app.player_state.volume = new_volume;
                            
                            if app.playback_mode == PlaybackMode::Local {
                                // Use local player for volume control
                                if let Some(ref player) = app.local_player {
                                    // Convert 0-100 percentage to 0-65535 for librespot
                                    // Use u32 for calculation to prevent overflow, then cast to u16
                                    let new_vol = (new_volume as u32 * 65535 / 100) as u16;
                                    player.set_volume(new_vol);
                                }
                            } else if let Some(ref client) = client {
                                // Use Spotify API for remote playback
                                let c = client.clone();
                                let volume = new_volume;
                                tokio::spawn(async move {
                                    let guard = c.lock().await;
                                    let _ = guard.set_volume(volume).await;
                                });
                            }
                        }
                        joshify::ui::MouseAction::ToggleShuffle => {
                            // Toggle shuffle
                            if let Some(ref client) = client {
                                let new_shuffle = !app.player_state.shuffle;
                                app.player_state.shuffle = new_shuffle;
                                let c = client.clone();
                                tokio::spawn(async move {
                                    let guard = c.lock().await;
                                    let _ = guard.toggle_shuffle(new_shuffle).await;
                                });
                            }
                        }
                        joshify::ui::MouseAction::CycleRepeat => {
                            // Cycle repeat mode
                            if let Some(ref client) = client {
                                app.player_state.repeat_mode = app.player_state.repeat_mode.cycle();
                                let c = client.clone();
                                let mode = match app.player_state.repeat_mode {
                                    joshify::state::player_state::RepeatMode::Off => rspotify::model::RepeatState::Off,
                                    joshify::state::player_state::RepeatMode::Track => rspotify::model::RepeatState::Track,
                                    joshify::state::player_state::RepeatMode::Context => rspotify::model::RepeatState::Context,
                                };
                                tokio::spawn(async move {
                                    let guard = c.lock().await;
                                    let _ = guard.set_repeat(mode).await;
                                });
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
