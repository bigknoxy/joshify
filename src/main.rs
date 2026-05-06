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
use rspotify::clients::OAuthClient;
use std::sync::Arc;

/// Highlighted item in the current view (for queue operations)
#[derive(Debug, Clone)]
struct HighlightedItem {
    uri: String,
    name: String,
    artist: String,
    _context: Option<PlaybackContext>,
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
    /// Navigation stack for drill-down browsing
    nav_stack: joshify::state::navigation_stack::NavigationStack,
    /// Theme registry for managing color themes
    theme_registry: joshify::themes::ThemeRegistry,
    /// LITE mode - minimal UI with simplified controls
    lite_mode: bool,
}

impl App {
    fn new(lite_mode: bool) -> Self {
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
            nav_stack: joshify::state::navigation_stack::NavigationStack::new(),
            theme_registry: joshify::themes::ThemeRegistry::default(),
            lite_mode,
        }
    }

    /// Cycle to the next theme
    fn cycle_theme(&mut self) {
        use joshify::themes::BuiltInTheme;
        use joshify::ui::theme;
        
        let current = self.theme_registry.current();
        let (next_theme, theme_name) = match current {
            BuiltInTheme::CatppuccinMocha => (BuiltInTheme::CatppuccinLatte, "Catppuccin Latte"),
            BuiltInTheme::CatppuccinLatte => (BuiltInTheme::GruvboxDark, "Gruvbox Dark"),
            BuiltInTheme::GruvboxDark => (BuiltInTheme::GruvboxLight, "Gruvbox Light"),
            BuiltInTheme::GruvboxLight => (BuiltInTheme::Nord, "Nord"),
            BuiltInTheme::Nord => (BuiltInTheme::TokyoNight, "Tokyo Night"),
            BuiltInTheme::TokyoNight => (BuiltInTheme::Dracula, "Dracula"),
            BuiltInTheme::Dracula => (BuiltInTheme::CatppuccinMocha, "Catppuccin Mocha"),
        };
        
        self.theme_registry.switch_theme(next_theme);
        
        // Update the global theme so UI renders with new colors
        theme::set_current_theme(next_theme);
        
        self.status_message = Some(format!("Theme: {}", theme_name));
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
                self.player_state = PlayerState::from_context(&ctx);

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
            // Play from user queue
            tracing::info!("Advancing to next track from queue: {}", next_uri);
            let c = client.clone();
            tokio::spawn(async move {
                let guard = c.lock().await;
                if let Err(e) = guard.playback_next().await {
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

    let mut app = App::new(args.lite);

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
                ContentState::RadioRecommendations(entries) => {
                    // Radio mode: add recommendations to queue and start playing first one
                    tracing::info!("Received {} radio recommendations", entries.len());
                    
                    if !entries.is_empty() {
                        // Add all entries to queue (skip first since we're playing it now)
                        for entry in entries.iter().skip(1) {
                            app.queue_state.add(joshify::state::queue_state::QueueEntry {
                                uri: entry.uri.clone(),
                                name: entry.name.clone(),
                                artist: entry.artist.clone(),
                                added_by_user: false,
                                is_recommendation: true,
                            });
                        }
                        tracing::info!("Added {} tracks to queue", entries.len() - 1);
                        
                        // Play first recommendation immediately
                        let first_entry = &entries[0];
                        tracing::info!("Playing first radio recommendation: {} - {}", first_entry.name, first_entry.uri);
                        
                        if let Some(ref player) = app.local_player {
                            match player.load_uri(&first_entry.uri, true, 0) {
                                Ok(_) => {
                                    app.player_state.current_track_name = Some(first_entry.name.clone());
                                    app.player_state.current_artist_name = Some(first_entry.artist.clone());
                                    app.player_state.current_track_uri = Some(first_entry.uri.clone());
                                    app.player_state.is_playing = true;
                                    app.player_state.progress_ms = 0;
                                    app.status_message = Some(format!(
                                        "Radio: {} - {}",
                                        first_entry.name, first_entry.artist
                                    ));
                                    tracing::info!(
                                        "Started radio playback: {} ({} more in queue)",
                                        first_entry.name,
                                        entries.len() - 1
                                    );
                                }
                                Err(e) => {
                                    app.status_message = Some(format!("Radio playback error: {}", e));
                                    tracing::error!("Failed to start radio track: {} - Error: {}", first_entry.uri, e);
                                }
                            }
                        } else {
                            tracing::error!("No local player available for radio playback");
                            app.status_message = Some("Radio error: No player available".to_string());
                        }
                    } else {
                        tracing::warn!("Received empty radio recommendations");
                        app.status_message = Some("No recommendations found".to_string());
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
                            ContentState::RadioRecommendations(_) => {
                                // Radio recommendations handled separately, not in this branch
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

                        // PHASE 1: Check user-added queue (up_next) first - highest priority
                        if !app.queue_state.local_queue.is_empty() {
                            tracing::info!(
                                "EndOfTrack: Found {} items in user queue, advancing",
                                app.queue_state.local_queue.len()
                            );
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
                                                "Auto-advanced to user queue item: {}",
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
                        // PHASE 2: Check context tracks if user queue is empty
                        else if app.queue_state.playback_queue().remaining_context_tracks() > 0 {
                            let remaining =
                                app.queue_state.playback_queue().remaining_context_tracks();
                            tracing::info!(
                                "EndOfTrack: User queue empty, {} context tracks remaining. Advancing...",
                                remaining
                            );

                            // Advance to next context track
                            if let Some(next_uri) = app.queue_state.playback_queue_mut().advance() {
                                tracing::info!(
                                    "EndOfTrack: Advancing to next context track: {}",
                                    next_uri
                                );

                                if let Some(ref player) = app.local_player {
                                    match player.load_uri(&next_uri, true, 0) {
                                        Ok(_) => {
                                            // Try to get track info from the content state
                                            let track_name = app
                                                .player_state
                                                .current_track_name
                                                .clone()
                                                .unwrap_or_else(|| "Unknown".to_string());
                                            let artist_name = app
                                                .player_state
                                                .current_artist_name
                                                .clone()
                                                .unwrap_or_else(|| "Unknown".to_string());

                                            app.player_state.current_track_uri =
                                                Some(next_uri.clone());
                                            app.player_state.is_playing = true;
                                            app.player_state.progress_ms = 0;
                                            app.status_message = Some(format!(
                                                "Playing next from playlist: {} - {}",
                                                track_name, artist_name
                                            ));
                                            tracing::info!(
                                                "Auto-advanced to context track: {} ({} remaining)",
                                                next_uri,
                                                app.queue_state
                                                    .playback_queue()
                                                    .remaining_context_tracks()
                                            );
                                        }
                                        Err(e) => {
                                            app.status_message =
                                                Some(format!("Context playback error: {}", e));
                                            tracing::warn!(
                                                "Failed to load next context track {}: {}",
                                                next_uri,
                                                e
                                            );
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
                        // PHASE 3: Radio mode - fetch recommendations when queue is empty
                        else if app.queue_state.radio_mode {
                            if let Some(ref current_uri) = app.player_state.current_track_uri {
                                if let Some(track_id) = current_uri.strip_prefix("spotify:track:") {
                                    tracing::info!(
                                        "EndOfTrack: Radio mode enabled, fetching recommendations for track {}",
                                        track_id
                                    );
                                    
                                    if let Some(ref client) = client {
                                        let c = client.clone();
                                        let seed_id = track_id.to_string();
                                        let tx_clone = tx.clone();
                                        
                                        tokio::spawn(async move {
                                            let guard = c.lock().await;
                                            match guard.get_recommendations(vec![seed_id], Some(20)).await {
                                                Ok(tracks) if !tracks.is_empty() => {
                                                    // Filter out tracks with no ID and map to QueueEntry
                                                    let entries: Vec<_> = tracks.iter().filter_map(|track| {
                                                        track.id.as_ref().map(|id| {
                                                            joshify::playback::domain::QueueEntry {
                                                                uri: format!("spotify:track:{}", id.id()),
                                                                name: track.name.clone(),
                                                                artist: track.artists.first().map(|a| a.name.clone()).unwrap_or_default(),
                                                                album: track.album.as_ref().map(|a| a.name.clone()),
                                                                duration_ms: Some(track.duration.num_milliseconds() as u32),
                                                                added_by_user: false,
                                                                is_recommendation: true,
                                                            }
                                                        })
                                                    }).collect();
                                                    
                                                    if !entries.is_empty() {
                                                        tracing::info!("Sending {} valid radio recommendations", entries.len());
                                                        let _ = tx_clone.send(ContentState::RadioRecommendations(entries)).await;
                                                    } else {
                                                        tracing::warn!("Radio mode: All recommendations had missing track IDs");
                                                        let _ = tx_clone.send(ContentState::Error("No valid recommendations".to_string())).await;
                                                    }
                                                }
                                                Ok(_) => {
                                                    tracing::warn!("Radio mode: No recommendations returned");
                                                    let _ = tx_clone.send(ContentState::Error("No recommendations available".to_string())).await;
                                                }
                                                Err(e) => {
                                                    tracing::warn!("Radio mode: Failed to get recommendations: {}", e);
                                                    let _ = tx_clone.send(ContentState::Error(format!("Radio error: {}", e))).await;
                                                }
                                            }
                                        });
                                        
                                        app.status_message = Some("Fetching recommendations...".to_string());
                                    }
                                }
                            }
                        }
                        // PHASE 4: Nothing left to play and radio disabled
                        else {
                            tracing::info!(
                                "EndOfTrack: No more tracks to play (queue empty, context exhausted, radio disabled)"
                            );
                            app.status_message = Some("Playback ended".to_string());
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

                if app.lite_mode {
                    // LITE Mode: Minimal UI with simplified layout
                    app.layout_cache.clear(); // Still clear for any mouse usage

                    // Check minimum terminal size (smaller for lite mode)
                    if area.width < 40 || area.height < 10 {
                        let warning = Paragraph::new(
                            "Terminal too small!\n\nMinimum: 40x10\n\nPlease resize your terminal.",
                        )
                        .alignment(Alignment::Center)
                        .style(Style::default().fg(Color::Yellow));
                        frame.render_widget(warning, area);
                        return;
                    }

                    // Use LITE mode renderer
                    joshify::ui::render_lite_mode(
                        frame,
                        &app.player_state,
                        &app.queue_state,
                        &app.status_message,
                    );

                    // LITE mode help overlay
                    if app.help_content.is_some() {
                        joshify::ui::render_lite_help(frame, area);
                    }

                    // Search overlay - use full search UI even in LITE mode
                    // (users need to see results to select tracks)
                    if app.search_state.is_active {
                        joshify::ui::render_search_overlay(frame, area, &app.search_state);
                    }

                    app.area = Some(area);
                } else {
                    // Full Mode: Original complex UI
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
                        joshify::ui::render_queue_overlay(frame, area, &app.queue_state);
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
                }
            })?;
        }

        // Write album art image directly to stdout (bypasses ratatui buffer)
        // Uses pre-processed Kitty escape sequence (no per-frame image processing)
        // Important: Must delete the old Kitty image before drawing at a new position.
        // Kitty images persist on screen until explicitly deleted. On resize, we use
        // the Kitty delete protocol command which removes only the image pixels without
        // affecting surrounding text content.
        // 
        // NOTE: Album art is disabled in LITE mode for cleaner terminal output
        if !app.lite_mode {
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
                                                    total_tracks: sa.album.tracks.total as u32,
                                                    release_year,
                                                }
                                            })
                                            .collect();
                                    let _ = tx_clone
                                        .send(ContentState::Library {
                                            albums,
                                            artists: vec![], // Load artists separately
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
                        // TODO: Implement artists loading
                        app.content_state =
                            ContentState::Error("Library artists not yet implemented".to_string());
                    }
                    LoadAction::AlbumTracks { album_id, name } => {
                        let c = client.clone();
                        let tx_clone = tx.clone();
                        let album_id_clone = album_id.clone();
                        let name_clone = name.clone();
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
                                        artist: "Unknown".to_string(),
                                        id: album_id_clone,
                                        image_url: None,
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
                        let _ = tx.send(ContentState::ArtistDetail {
                            artist: artist_item,
                        });
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
                                if let Some(track) = app.search_state.selected_track() {
                                    // Try local playback first (for LITE mode with local player)
                                    if let Some(ref player) = app.local_player {
                                        match player.load_uri(&track.uri, true, 0) {
                                            Ok(_) => {
                                                app.player_state.current_track_name = Some(track.name.clone());
                                                app.player_state.current_artist_name = Some(track.artist.clone());
                                                app.player_state.current_track_uri = Some(track.uri.clone());
                                                app.player_state.is_playing = true;
                                                app.player_state.progress_ms = 0;
                                                app.status_message = Some(format!("Playing: {}", track.name));
                                                tracing::info!("Playing track via local player: {}", track.name);
                                            }
                                            Err(e) => {
                                                app.status_message = Some(format!("Playback error: {}", e));
                                                tracing::error!("Local playback error: {}", e);
                                            }
                                        }
                                    } else if let Some(ref client) = client {
                                        // Fallback to Spotify API if no local player
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
                                        // Update player_state so UI shows the track immediately
                                        app.player_state.current_track_name = Some(track.name.clone());
                                        app.player_state.current_artist_name = Some(track.artist.clone());
                                        app.player_state.current_track_uri = Some(track.uri.clone());
                                        app.player_state.is_playing = true;
                                        app.player_state.progress_ms = 0;
                                        app.status_message = Some(format!("Playing: {}", track.name));
                                    }
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
                                                                        // Small delay to let device transfer settle before play command
                                                                        // This prevents race where transfer and play commands conflict
                                                                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                                                                    }
                                                                }
                                                            }
                                                            if let Some(
                                                                PlaybackContext::Playlist {
                                                                    uri,
                                                                    start_index,
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
                                                                    // Use URI-based offset for unambiguous track selection
                                                                    // This is more reliable than index-based offsets which can drift
                                                                    tracing::info!(
                                                                        "Starting playlist playback: playlist_id={}, track_uri={}, start_index={}",
                                                                        playlist_id_str,
                                                                        track_uri,
                                                                        *start_index
                                                                    );
                                                                    let offset = rspotify::model::Offset::Uri(track_uri.clone());
                                                                    let _ = guard.oauth.start_context_playback(
                                                                        rspotify::model::PlayContextId::from(playlist_id),
                                                                        None,
                                                                        Some(offset),
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
                                                        app.content_state = ContentState::Loading(
                                                            LoadAction::ArtistTopTracks {
                                                                artist_id: artist.id.clone(),
                                                                name: artist.name.clone(),
                                                            },
                                                        );
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

                                                // Track the highlighted item for queue operations
                                                app.highlighted_item = Some(HighlightedItem {
                                                    uri: track.uri.clone(),
                                                    name: track.name.clone(),
                                                    artist: track.artist.clone(),
                                                    _context: app.current_context.clone(),
                                                });

                                                if app.playback_mode == PlaybackMode::Local {
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
                                                    }
                                                } else if let Some(ref client) = client {
                                                    let c = client.clone();
                                                    let track_uri = track.uri.clone();
                                                    let track_name = track.name.clone();
                                                    tokio::spawn(async move {
                                                        let guard = c.lock().await;
                                                        if let Ok(devices) =
                                                            guard.available_devices().await
                                                        {
                                                            if let Some(device) = devices.first() {
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
                                                        let _ = guard
                                                            .start_playback(vec![track_uri], None)
                                                            .await;
                                                    });
                                                    app.status_message =
                                                        Some(format!("Playing: {}", track_name));
                                                }
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

                        // Theme switching - 'T' to cycle through themes
                        crossterm::event::KeyCode::Char('T') => {
                            app.cycle_theme();
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

                                    if app.playback_mode == PlaybackMode::Local {
                                        // Play locally with librespot
                                        if let Some(ref player) = app.local_player {
                                            match player.load_uri(&track.uri, true, 0) {
                                                Ok(_) => {
                                                    app.player_state.current_track_name =
                                                        Some(track.name.clone());
                                                    app.player_state.current_artist_name =
                                                        Some(track.artist.clone());
                                                    app.player_state.current_track_uri =
                                                        Some(track.uri.clone());
                                                    app.player_state.is_playing = true;
                                                    app.player_state.progress_ms = 0;
                                                    app.status_message = Some(format!(
                                                        "Playing locally: {}",
                                                        track.name
                                                    ));
                                                    // Advance queue position so the selected track is "consumed"
                                                    let _ = app
                                                        .queue_state
                                                        .playback_queue_mut()
                                                        .advance();
                                                    tracing::info!(
                                                        "Mouse: Local playback started - consumed selected track, position now at {} ({} remaining)",
                                                        app.queue_state.playback_queue().context_position(),
                                                        app.queue_state.playback_queue().remaining_context_tracks()
                                                    );
                                                }
                                                Err(e) => {
                                                    app.status_message = Some(format!(
                                                        "Local playback error: {}",
                                                        e
                                                    ));
                                                }
                                            }
                                        } else {
                                            app.status_message =
                                                Some("Local player not initialized".to_string());
                                        }
                                    } else {
                                        // Remote playback via Spotify API
                                        if let Some(ref client) = client {
                                            let c = client.clone();
                                            let track_uri = track.uri.clone();
                                            let track_name = track.name.clone();
                                            let context = app.current_context.clone();
                                            let track_index = index; // Capture index for async block
                                            let playlist_id_for_context =
                                                if let ContentState::PlaylistTracks(pid, _) =
                                                    &app.content_state
                                                {
                                                    Some(pid.clone())
                                                } else {
                                                    None
                                                };

                                            tokio::spawn(async move {
                                                let guard = c.lock().await;
                                                if let Ok(devices) = guard.available_devices().await
                                                {
                                                    if let Some(device) = devices.first() {
                                                        if let Some(ref device_id) = device.id {
                                                            let _ = guard
                                                                .transfer_playback(device_id)
                                                                .await;
                                                        }
                                                    }
                                                }

                                                // Use playlist context if available
                                                if let Some(pid) = playlist_id_for_context {
                                                    let _playlist_uri =
                                                        format!("spotify:playlist:{}", pid);
                                                    if let Ok(playlist_id) =
                                                        rspotify::model::PlaylistId::from_id(&pid)
                                                    {
                                                        // Use URI-based offset for unambiguous track selection
                                                        // This is more reliable than index-based offsets
                                                        tracing::info!(
                                                            "Mouse: Starting playlist playback: playlist_id={}, track_uri={}, track_index={}",
                                                            pid,
                                                            track_uri,
                                                            track_index
                                                        );
                                                        let offset = rspotify::model::Offset::Uri(
                                                            track_uri.clone(),
                                                        );
                                                        let _ = guard.oauth.start_context_playback(
                                                            rspotify::model::PlayContextId::from(playlist_id),
                                                            None,
                                                            Some(offset),
                                                            None,
                                                        ).await;
                                                    } else {
                                                        // Fallback to direct track playback
                                                        let _ = guard
                                                            .start_playback(vec![track_uri], None)
                                                            .await;
                                                    }
                                                } else if let Some(PlaybackContext::Playlist {
                                                    uri,
                                                    start_index,
                                                    ..
                                                }) = &context
                                                {
                                                    // Use existing context if available
                                                    let playlist_id_str = uri
                                                        .strip_prefix("spotify:playlist:")
                                                        .unwrap_or(uri);
                                                    if let Ok(playlist_id) =
                                                        rspotify::model::PlaylistId::from_id(
                                                            playlist_id_str,
                                                        )
                                                    {
                                                        // Use URI-based offset for unambiguous track selection
                                                        tracing::info!(
                                                            "Existing context: Starting playlist playback: playlist_id={}, track_uri={}, start_index={}",
                                                            playlist_id_str,
                                                            track_uri,
                                                            *start_index
                                                        );
                                                        let offset = rspotify::model::Offset::Uri(
                                                            track_uri.clone(),
                                                        );
                                                        let _ = guard.oauth.start_context_playback(
                                                            rspotify::model::PlayContextId::from(playlist_id),
                                                            None,
                                                            Some(offset),
                                                            None,
                                                        ).await;
                                                    } else {
                                                        let _ = guard
                                                            .start_playback(vec![track_uri], None)
                                                            .await;
                                                    }
                                                } else {
                                                    // No context - play track directly
                                                    let _ = guard
                                                        .start_playback(vec![track_uri], None)
                                                        .await;
                                                }
                                            });
                                            app.status_message =
                                                Some(format!("Playing: {}", track_name));
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
                                    let current_idx = nav_items
                                        .iter()
                                        .position(|&n| n == app.selected_nav)
                                        .unwrap_or(0);
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

// =============================================================================
// Tests for Auto-Advance and Queue Management
// =============================================================================

#[cfg(test)]
mod playback_tests {
    use super::*;
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
