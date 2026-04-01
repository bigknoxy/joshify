mod auth;
mod api;
mod state;
mod ui;
mod setup;
mod album_art;
mod keyring_store;

use anyhow::Result;
use crate::auth::OAuthConfig;
use crate::state::{NavItem, FocusTarget, ContentState, LoadAction};
use crate::state::player_state::PlayerState;
use crate::state::app_state::{TrackListItem, PlaylistListItem};

/// Application state
struct App {
    selected_nav: NavItem,
    is_authenticated: bool,
    player_state: PlayerState,
    status_message: Option<String>,
    last_poll_ms: u64,
    poll_interval_ms: u64,
    focus: FocusTarget,
    show_queue: bool,
    help_lines: Option<Vec<String>>,
    area: Option<Rect>,
    content_state: ContentState,
    selected_index: usize,
    scroll_offset: usize,
    search_query: String,
    is_searching: bool,
    album_art_cache: album_art::AlbumArtCache,
    last_fetched_art_uri: Option<String>,
}

impl App {
    fn new() -> Self {
        Self {
            selected_nav: NavItem::Home,
            is_authenticated: false,
            player_state: PlayerState::default(),
            status_message: None,
            last_poll_ms: 0,
            poll_interval_ms: 1000,
            focus: FocusTarget::Sidebar,
            show_queue: false,
            help_lines: None,
            area: None,
            content_state: ContentState::Home,
            selected_index: 0,
            scroll_offset: 0,
            search_query: String::new(),
            is_searching: false,
            album_art_cache: album_art::AlbumArtCache::new(),
            last_fetched_art_uri: None,
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

    async fn poll_playback(
        &mut self,
        client: &Arc<Mutex<api::SpotifyClient>>,
        tx_art: &tokio::sync::mpsc::Sender<(String, Vec<u8>)>,
    ) {
        let client_guard = client.lock().await;
        match client_guard.current_playback().await {
            Ok(Some(ctx)) => {
                let old_track_uri = self.player_state.current_track_uri.clone();
                self.player_state = PlayerState::from_context(&ctx);

                if self.status_message.as_ref().map_or(false, |m| m.starts_with("Playback error")) {
                    self.status_message = None;
                }

                let new_track_uri = self.player_state.current_track_uri.clone();
                let new_album_art_url = self.player_state.current_album_art_url.clone();

                if new_track_uri != old_track_uri && new_track_uri.is_some() && new_album_art_url.is_some() {
                    if let (Some(art_url), Some(art_uri)) = (new_album_art_url, new_track_uri) {
                        let cache = self.album_art_cache.clone();
                        let tx_art_clone = tx_art.clone();
                        let art_uri_for_closure = art_uri.clone();

                        tokio::spawn(async move {
                            match cache.get_or_fetch(&art_url).await {
                                Some(image_data) => {
                                    println!("Fetched album art for {}", art_uri_for_closure);
                                    let _ = tx_art_clone.send((art_uri_for_closure, image_data)).await;
                                }
                                None => {
                                    eprintln!("Failed to fetch album art for {}", art_url);
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
                if self.status_message.as_ref().map_or(false, |m| m.starts_with("Playback error")) {
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
use ratatui::{
    prelude::*,
    widgets::Paragraph,
};
use ratatui::backend::CrosstermBackend;
use rspotify::prelude::Id;
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;

/// CLI arguments for non-interactive mode
#[derive(Debug, Clone, Default)]
struct CliArgs {
    client_id: Option<String>,
    client_secret: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
    redirect_uri: Option<String>,
    help: bool,
}

impl CliArgs {
    fn parse() -> Self {
        let mut args = CliArgs::default();
        let mut i = 1;
        let cli_args: Vec<String> = std::env::args().collect();

        while i < cli_args.len() {
            match cli_args[i].as_str() {
                "--client-id" => {
                    if i + 1 < cli_args.len() {
                        args.client_id = Some(cli_args[i + 1].clone());
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--client-secret" => {
                    if i + 1 < cli_args.len() {
                        args.client_secret = Some(cli_args[i + 1].clone());
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--access-token" => {
                    if i + 1 < cli_args.len() {
                        args.access_token = Some(cli_args[i + 1].clone());
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--refresh-token" => {
                    if i + 1 < cli_args.len() {
                        args.refresh_token = Some(cli_args[i + 1].clone());
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--redirect-uri" => {
                    if i + 1 < cli_args.len() {
                        args.redirect_uri = Some(cli_args[i + 1].clone());
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                "--help" | "-h" => {
                    args.help = true;
                    i += 1;
                }
                _ => {
                    i += 1;
                }
            }
        }

        args
    }

    fn print_help() {
        println!("Joshify - Terminal Spotify Client");
        println!();
        println!("USAGE:");
        println!("    joshify [OPTIONS]");
        println!();
        println!("OPTIONS:");
        println!("    --client-id <ID>       Spotify Client ID (or SPOTIFY_CLIENT_ID)");
        println!("    --client-secret <SEC>  Spotify Client Secret (or SPOTIFY_CLIENT_SECRET)");
        println!("    --access-token <TOK>   Spotify Access Token (or SPOTIFY_ACCESS_TOKEN)");
        println!("    --refresh-token <TOK>  Spotify Refresh Token (or SPOTIFY_REFRESH_TOKEN)");
        println!("    --redirect-uri <URI>   OAuth Redirect URI (default: http://127.0.0.1:8888/callback)");
        println!("    --help, -h             Show this help message");
        println!();
        println!("ENVIRONMENT VARIABLES:");
        println!("    SPOTIFY_CLIENT_ID      Spotify Client ID");
        println!("    SPOTIFY_CLIENT_SECRET  Spotify Client Secret");
        println!("    SPOTIFY_ACCESS_TOKEN   Spotify Access Token");
        println!("    SPOTIFY_REFRESH_TOKEN  Spotify Refresh Token");
        println!();
        println!("EXAMPLES:");
        println!("    # Interactive mode (default)");
        println!("    joshify");
        println!();
        println!("    # Non-interactive with environment variables");
        println!("    export SPOTIFY_CLIENT_ID=xxx");
        println!("    export SPOTIFY_CLIENT_SECRET=yyy");
        println!("    export SPOTIFY_ACCESS_TOKEN=zzz");
        println!("    joshify");
        println!();
        println!("    # Non-interactive with CLI flags");
        println!("    joshify --client-id xxx --access-token zzz");
    }
}



#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments FIRST (before any terminal initialization)
    let args = CliArgs::parse();

    // Handle --help flag (before any terminal initialization)
    if args.help {
        CliArgs::print_help();
        return Ok(());
    }

    // Initialize terminal
    ratatui::init();

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
    let _ = crossterm::execute!(
        io::stdout(),
        crossterm::event::DisableMouseCapture
    );
    ratatui::restore();

    result
}

async fn run_with_args(args: CliArgs) -> Result<()> {

    // Load config from CLI args (args take precedence over env vars and config file)
    let config = OAuthConfig::from_args(&args);

    // Check if we have credentials from env vars or CLI args
    let has_tokens = !config.client_id.is_empty() && !config.client_secret.is_empty()
        && (std::env::var("SPOTIFY_ACCESS_TOKEN").is_ok()
            || std::env::var("SPOTIFY_REFRESH_TOKEN").is_ok()
            || args.access_token.is_some()
            || args.refresh_token.is_some());

    // Initialize terminal
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    ratatui::init();

    // Enable mouse capture and cursor visibility
    crossterm::execute!(
        io::stdout(),
        crossterm::event::EnableMouseCapture
    )?;
    crossterm::execute!(
        io::stdout(),
        crossterm::cursor::Show
    )?;

    let mut app = App::new();

    // If we have tokens from env/CLI, skip interactive setup
    if has_tokens {
        app.is_authenticated = true;
        app.status_message = Some("Connected to Spotify (non-interactive) - Press ? for help".to_string());
    } else {
        // Ensure we have credentials configured (runs interactive setup if needed)
        let config = setup::ensure_configured()?;

        // Run OAuth browser flow to get access tokens
        match setup::run_oauth_flow(&config).await {
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
    let client = match api::SpotifyClient::new(&config).await {
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
                let access_token = args.access_token
                    .or_else(|| std::env::var("SPOTIFY_ACCESS_TOKEN").ok())
                    .unwrap_or_default();
                let refresh_token = args.refresh_token
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
                    expires_at: Some(chrono::DateTime::from_timestamp(expires_at, 0)
                        .unwrap_or(chrono::DateTime::UNIX_EPOCH)),
                    expires_in: chrono::TimeDelta::seconds(3600),
                    scopes: std::collections::HashSet::new(),
                });
            };
        };
    }

    // Channel for async data loading results
    let (tx, mut rx) = tokio::sync::mpsc::channel::<ContentState>(16);

    // Channel for album art data
    let (tx_art, mut rx_art) = tokio::sync::mpsc::channel::<(String, Vec<u8>)>(16);

    // Main loop
    loop {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before epoch")
            .as_millis() as u64;

        // Poll playback state at interval
        if let Some(ref client) = client {
            if now - app.last_poll_ms >= app.poll_interval_ms {
                app.poll_playback(client, &tx_art).await;
                app.last_poll_ms = now;
            }
        }

        // Check for async data loading results
        if let Ok(state) = rx.try_recv() {
            app.content_state = state;
        }

        // Check for album art data results
        while let Ok((track_uri, art_data)) = rx_art.try_recv() {
            // Only update if this is still the current track
            if app.player_state.current_track_uri.as_ref() == Some(&track_uri) {
                app.player_state.current_album_art_data = Some(art_data);
            }
        }

        terminal.draw(|frame| {
            let area = frame.area();

            // Check minimum terminal size
            if area.width < 50 || area.height < 20 {
                let warning = Paragraph::new("Terminal too small!\n\nMinimum: 50x20\n\nPlease resize your terminal.")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::Yellow));
                frame.render_widget(warning, area);
                return;
            }

            // Status bar at top (if present)
            let top_area = if let Some(ref msg) = app.status_message {
                let [top, rest] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)])
                    .areas(area);
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
            let [sidebar, main] = Layout::horizontal([
                Constraint::Length(sidebar_width),
                Constraint::Min(0),
            ])
            .areas(top_area);

            // Player bar: 5 rows at bottom (includes album art)
            let player_bar_height = 5u16;
            let [main_content, player_bar] = Layout::vertical([
                Constraint::Min(0),
                Constraint::Length(player_bar_height),
            ])
            .areas(main);

            // Render all components with focus highlighting
            let sidebar_focused = app.focus == FocusTarget::Sidebar;
            let main_focused = app.focus == FocusTarget::MainContent;
            let player_focused = app.focus == FocusTarget::PlayerBar;

            // Update search prompt in content state if actively typing
            if app.is_searching {
                let query = if app.search_query.is_empty() {
                    "Type search query...".to_string()
                } else {
                    format!("Search: {}", app.search_query)
                };
                // Update state for search input display
                match &app.content_state {
                    ContentState::Loading(LoadAction::Search { query: existing }) if existing != &query => {
                        app.content_state = ContentState::Loading(LoadAction::Search { query });
                    }
                    ContentState::LoadingInProgress(LoadAction::Search { query: existing }) if existing != &query => {
                        app.content_state = ContentState::Loading(LoadAction::Search { query });
                    }
                    ContentState::Home => {
                        app.content_state = ContentState::Loading(LoadAction::Search { query });
                    }
                    _ => {}
                }
            }

            ui::render_sidebar(frame, sidebar, app.selected_nav, sidebar_focused);
            ui::render_main_view(
                frame,
                main_content,
                app.selected_nav,
                app.is_authenticated,
                main_focused,
                &app.content_state,
                app.selected_index,
                app.scroll_offset,
                app.is_searching,
                &app.search_query,
            );

            let track_name = app.player_state.current_track_name.as_deref().unwrap_or("Not Playing");
            let artist_name = app.player_state.current_artist_name.as_deref().unwrap_or("");

            ui::render_player_bar(
                frame,
                player_bar,
                track_name,
                artist_name,
                app.player_state.is_playing,
                app.player_state.progress_ms,
                app.player_state.duration_ms,
                app.player_state.volume,
                app.player_state.current_album_art_url.as_deref(),
                app.player_state.current_album_art_data.as_deref(),
                player_focused,
            );

            // Overlays (rendered last so they appear on top)
            if app.show_queue {
                ui::render_queue_overlay(frame, area, &app.player_state);
            }
            if let Some(ref help_lines) = app.help_lines {
                ui::render_help_overlay(frame, area, help_lines);
            }

            // Store frame area for mouse handling
            app.area = Some(area);

            // Show cursor only when searching
            if app.is_searching {
                let _ = crossterm::execute!(io::stdout(), crossterm::cursor::Show);
            } else {
                let _ = crossterm::execute!(io::stdout(), crossterm::cursor::Hide);
            }
        })?;

        // Handle async data loading based on current state
        // Only spawn tasks when in Loading state, not LoadingInProgress (prevents duplicate spawns)
        let load_action = match &app.content_state {
            ContentState::Loading(action) => Some(action.clone()),
            _ => None,
        };

        if let Some(action) = load_action {
            if let Some(ref client) = client {
                match action {
                    LoadAction::LikedSongs => {
                        let c = client.clone();
                        let tx_clone = tx.clone();
                        tokio::spawn(async move {
                            let guard = c.lock().await;
                            match guard.current_user_saved_tracks(50).await {
                                Ok(tracks) => {
                                    let items: Vec<TrackListItem> = tracks.into_iter().filter_map(|t| {
                                        t.track.id.map(|id| {
                                            let artist = t.track.artists.first()
                                                .map(|a| a.name.clone())
                                                .unwrap_or_else(|| {
                                                    eprintln!("Warning: track '{}' has no artists", t.track.name);
                                                    String::new()
                                                });
                                            TrackListItem {
                                                name: t.track.name,
                                                artist,
                                                uri: format!("spotify:track:{}", id.id()),
                                            }
                                        })
                                    }).collect();
                                    let _ = tx_clone.send(ContentState::LikedSongs(items)).await;
                                }
                                Err(e) => {
                                    let _ = tx_clone.send(ContentState::Error(format!("Failed to load liked songs: {}", e))).await;
                                }
                            }
                        });
                        app.content_state = ContentState::LoadingInProgress(LoadAction::LikedSongs);
                    }
                    LoadAction::Playlists => {
                        let c = client.clone();
                        let tx_clone = tx.clone();
                        tokio::spawn(async move {
                            let guard = c.lock().await;
                            match guard.current_users_playlists(50).await {
                                Ok(playlists) => {
                                    let items: Vec<PlaylistListItem> = playlists.into_iter().map(|p| PlaylistListItem {
                                        name: p.name,
                                        id: p.id.id().to_string(),
                                        track_count: p.tracks.total,
                                    }).collect();
                                    let _ = tx_clone.send(ContentState::Playlists(items)).await;
                                }
                                Err(e) => {
                                    let _ = tx_clone.send(ContentState::Error(format!("Failed to load playlists: {}", e))).await;
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
                                    let tracks: Vec<TrackListItem> = items.into_iter().filter_map(|pi| {
                                        pi.track.and_then(|t| {
                                            if let rspotify::model::PlayableItem::Track(track) = t {
                                                track.id.map(|id| {
                                                    let artist = track.artists.first()
                                                        .map(|a| a.name.clone())
                                                        .unwrap_or_else(|| {
                                                            eprintln!("Warning: track '{}' has no artists", track.name);
                                                            String::new()
                                                        });
                                                    TrackListItem {
                                                        name: track.name,
                                                        artist,
                                                        uri: format!("spotify:track:{}", id.id()),
                                                    }
                                                })
                                            } else {
                                                None
                                            }
                                        })
                                    }).collect();
                                    let _ = tx_clone.send(ContentState::PlaylistTracks(name_clone, tracks)).await;
                                }
                                Err(e) => {
                                    let _ = tx_clone.send(ContentState::Error(format!("Failed to load playlist: {}", e))).await;
                                }
                            }
                        });
                        app.content_state = ContentState::LoadingInProgress(LoadAction::PlaylistTracks { name, id });
                    }
                    LoadAction::Search { query } => {
                        let c = client.clone();
                        let tx_clone = tx.clone();
                        let query_clone = query.clone();
                        tokio::spawn(async move {
                            let guard = c.lock().await;
                            match guard.search(&query_clone, 50).await {
                                Ok(tracks) => {
                                    let items: Vec<TrackListItem> = tracks.into_iter().filter_map(|t| {
                                        t.id.map(|id| {
                                            let artist = t.artists.first()
                                                .map(|a| a.name.clone())
                                                .unwrap_or_else(|| {
                                                    eprintln!("Warning: track '{}' has no artists", t.name);
                                                    String::new()
                                                });
                                            TrackListItem {
                                                name: t.name,
                                                artist,
                                                uri: format!("spotify:track:{}", id.id()),
                                            }
                                        })
                                    }).collect();
                                    let _ = tx_clone.send(ContentState::SearchResults(query_clone, items)).await;
                                }
                                Err(e) => {
                                    let _ = tx_clone.send(ContentState::Error(format!("Search failed: {}", e))).await;
                                }
                            }
                        });
                        app.content_state = ContentState::LoadingInProgress(LoadAction::Search { query });
                    }
                }
            }
        }

        // Handle input
        if crossterm::event::poll(std::time::Duration::from_millis(50))? {
            match crossterm::event::read()? {
                crossterm::event::Event::Key(key) => {
                    // Global keys work regardless of focus
                    if key.code == crossterm::event::KeyCode::Char('q') {
                        break;
                    }

                    // Close overlays
                    if app.show_queue && key.code != crossterm::event::KeyCode::Char('q') {
                        app.show_queue = false;
                        continue;
                    }

                    match key.code {
                        // Focus navigation
                        crossterm::event::KeyCode::Tab => {
                            if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                                app.focus_previous();
                            } else {
                                app.focus_next();
                            }
                        }

                        // Enter key - action based on current focus
                        crossterm::event::KeyCode::Enter => {
                            match app.focus {
                                FocusTarget::Sidebar => {
                                    // Select current nav item - show content
                                    match app.selected_nav {
                                        ui::NavItem::LikedSongs => {
                                            app.content_state = ContentState::Loading(LoadAction::LikedSongs);
                                            app.selected_index = 0;
                                            app.scroll_offset = 0;
                                        }
                                        ui::NavItem::Playlists => {
                                            app.content_state = ContentState::Loading(LoadAction::Playlists);
                                            app.selected_index = 0;
                                            app.scroll_offset = 0;
                                        }
                                        ui::NavItem::Home => {
                                            app.content_state = ContentState::Home;
                                        }
                                        ui::NavItem::Search => {
                                            app.content_state = ContentState::Loading(LoadAction::Search { query: "Type to search...".to_string() });
                                        }
                                        ui::NavItem::Library => {
                                            app.content_state = ContentState::Loading(LoadAction::Search { query: "Loading library...".to_string() });
                                        }
                                    }
                                }
                                FocusTarget::MainContent => {
                                    // Act on current content - play selected track
                                    match &app.content_state {
                                        ContentState::LikedSongs(tracks) |
                                        ContentState::PlaylistTracks(_, tracks) |
                                        ContentState::SearchResults(_, tracks) => {
                                            if !tracks.is_empty() && app.selected_index < tracks.len() {
                                                if let Some(ref client) = client {
                                                    let c = client.lock().await;
                                                    let track = &tracks[app.selected_index];
                                                    match c.start_playback(vec![track.uri.clone()], None).await {
                                                        Ok(_) => {
                                                            app.status_message = Some(format!("Playing: {}", track.name));
                                                        }
                                                        Err(e) => {
                                                            app.status_message = Some(format!("Playback error: {}", e));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        ContentState::Playlists(playlists) => {
                                            // Enter on playlist - show its tracks
                                            if !playlists.is_empty() && app.selected_index < playlists.len() {
                                                let playlist = &playlists[app.selected_index];
                                                app.content_state = ContentState::Loading(
                                                    LoadAction::PlaylistTracks { name: playlist.name.clone(), id: playlist.id.clone() }
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
                                    if let Some(ref client) = client {
                                        let c = client.lock().await;
                                        if app.player_state.is_playing {
                                            let _ = c.playback_pause().await;
                                        } else {
                                            let _ = c.playback_resume().await;
                                        }
                                    }
                                }
                            }
                        }

                        // Sidebar navigation (when sidebar focused)
                        crossterm::event::KeyCode::Char('j') | crossterm::event::KeyCode::Down => {
                            if app.focus == FocusTarget::Sidebar {
                                let current_idx = app.selected_nav as usize;
                                let next_idx = (current_idx + 1) % ui::NavItem::all().len();
                                app.selected_nav = ui::NavItem::all()[next_idx];
                            } else if app.focus == FocusTarget::MainContent {
                                // Scroll list down based on current content
                                let len = match &app.content_state {
                                    ContentState::LikedSongs(t) => t.len(),
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
                                }
                            } else if app.focus == FocusTarget::PlayerBar {
                                // Volume down when player focused
                                if let Some(ref client) = client {
                                    let new_vol = app.player_state.volume.saturating_sub(5);
                                    let c = client.lock().await;
                                    let _ = c.set_volume(new_vol).await;
                                }
                            }
                        }
                        crossterm::event::KeyCode::Char('k') | crossterm::event::KeyCode::Up => {
                            if app.focus == FocusTarget::Sidebar {
                                let current_idx = app.selected_nav as usize;
                                let next_idx = if current_idx == 0 {
                                    ui::NavItem::all().len() - 1
                                } else {
                                    current_idx - 1
                                };
                                app.selected_nav = ui::NavItem::all()[next_idx];
                            } else if app.focus == FocusTarget::MainContent {
                                // Scroll list up based on current content
                                let len = match &app.content_state {
                                    ContentState::LikedSongs(t) => t.len(),
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
                                }
                            } else if app.focus == FocusTarget::PlayerBar {
                                // Volume up when player focused
                                if let Some(ref client) = client {
                                    let new_vol = (app.player_state.volume + 5).min(100);
                                    let c = client.lock().await;
                                    let _ = c.set_volume(new_vol).await;
                                }
                            }
                        }

                        // Playback controls (work from any focus)
                        crossterm::event::KeyCode::Char(' ') => {
                            if app.focus == FocusTarget::PlayerBar {
                                // Space only toggles when player bar is focused
                                if let Some(ref client) = client {
                                    let c = client.lock().await;
                                    if app.player_state.is_playing {
                                        let _ = c.playback_pause().await;
                                    } else {
                                        let _ = c.playback_resume().await;
                                    }
                                }
                            }
                        }
                        crossterm::event::KeyCode::Char('n') => {
                            if let Some(ref client) = client {
                                let c = client.lock().await;
                                let _ = c.playback_next().await;
                            }
                        }
                        crossterm::event::KeyCode::Char('p') => {
                            if let Some(ref client) = client {
                                let c = client.lock().await;
                                let _ = c.playback_previous().await;
                            }
                        }
                        crossterm::event::KeyCode::Left => {
                            // Seek backward 10 seconds
                            if let Some(ref client) = client {
                                let new_pos = app.player_state.progress_ms.saturating_sub(10000);
                                let c = client.lock().await;
                                let _ = c.seek(new_pos, None).await;
                            }
                        }
                        crossterm::event::KeyCode::Right => {
                            // Seek forward 10 seconds
                            if let Some(ref client) = client {
                                let new_pos = app.player_state.progress_ms.saturating_add(10000)
                                    .min(app.player_state.duration_ms);
                                let c = client.lock().await;
                                let _ = c.seek(new_pos, None).await;
                            }
                        }
                        crossterm::event::KeyCode::Char('+') => {
                            if let Some(ref client) = client {
                                let new_vol = (app.player_state.volume + 5).min(100);
                                let c = client.lock().await;
                                let _ = c.set_volume(new_vol).await;
                            }
                        }
                        crossterm::event::KeyCode::Char('-') => {
                            if let Some(ref client) = client {
                                let new_vol = app.player_state.volume.saturating_sub(5);
                                let c = client.lock().await;
                                let _ = c.set_volume(new_vol).await;
                            }
                        }

                        // Queue toggle
                        crossterm::event::KeyCode::Char('Q') => {
                            if let Some(ref client) = client {
                                let c = client.lock().await;
                                let _ = c.get_queue().await;
                            }
                            app.show_queue = !app.show_queue;
                        }
                        crossterm::event::KeyCode::Char('a') => {
                            // Add current track to queue
                            if let Some(ref client) = client {
                                if let Some(track) = &app.player_state.current_track_uri {
                                    let c = client.lock().await;
                                    match c.add_to_queue(track).await {
                                        Ok(_) => {
                                            app.status_message = Some("Added to queue".to_string());
                                        }
                                        Err(e) => {
                                            app.status_message = Some(format!("Queue error: {}", e));
                                        }
                                    }
                                }
                            }
                        }

                        // Settings
                        crossterm::event::KeyCode::Char('c') => {
                            match setup::run_setup() {
                                Ok(_) => {
                                    app.status_message = Some("Config updated - restart app to apply".to_string());
                                }
                                Err(_) => {
                                    app.status_message = Some("Setup cancelled".to_string());
                                }
                            }
                        }

                        // Search - '/' key starts search input
                        crossterm::event::KeyCode::Char('/') => {
                            app.is_searching = true;
                            app.search_query.clear();
                            app.focus = FocusTarget::MainContent;
                            app.content_state = ContentState::Loading(LoadAction::Search { query: "Type search query...".to_string() });
                        }

                        // Search input handling
                        _ if app.is_searching => {
                            match key.code {
                                crossterm::event::KeyCode::Enter => {
                                    // Execute search
                                    if !app.search_query.is_empty() {
                                        app.content_state = ContentState::Loading(
                                            LoadAction::Search { query: app.search_query.clone() }
                                        );
                                        app.selected_index = 0;
                                        app.scroll_offset = 0;
                                    }
                                    app.is_searching = false;
                                }
                                crossterm::event::KeyCode::Esc => {
                                    app.is_searching = false;
                                    app.content_state = ContentState::Home;
                                }
                                crossterm::event::KeyCode::Backspace => {
                                    app.search_query.pop();
                                    app.content_state = ContentState::Loading(
                                        LoadAction::Search {
                                            query: if app.search_query.is_empty() {
                                                "Type search query...".to_string()
                                            } else {
                                                format!("Search: {}", app.search_query)
                                            }
                                        }
                                    );
                                }
                                crossterm::event::KeyCode::Char(c) => {
                                    app.search_query.push(c);
                                    app.content_state = ContentState::Loading(
                                        LoadAction::Search { query: format!("Search: {}", app.search_query) }
                                    );
                                }
                                _ => {}
                            }
                            continue; // Skip other key handling while searching
                        }

                        // Help
                        crossterm::event::KeyCode::Char('?') => {
                            if app.help_lines.is_some() {
                                app.help_lines = None;
                            } else {
                                app.help_lines = Some(vec![
                                    "=== Navigation ===".into(),
                                    "Tab/Shift+Tab: Focus sections".into(),
                                    "j/k or ↑/↓: Navigate".into(),
                                    "Enter: Select/Play".into(),
                                    "".into(),
                                    "=== Search ===".into(),
                                    "/: Start search".into(),
                                    "Esc: Cancel search".into(),
                                    "".into(),
                                    "=== Playback ===".into(),
                                    "Space: Play/Pause".into(),
                                    "n: Next track".into(),
                                    "p: Previous track".into(),
                                    "←/→: Seek ±10s".into(),
                                    "".into(),
                                    "=== Queue ===".into(),
                                    "Q: Toggle queue view".into(),
                                    "a: Add current to queue".into(),
                                    "".into(),
                                    "=== Volume ===".into(),
                                    "+/-: Volume up/down".into(),
                                    "".into(),
                                    "=== System ===".into(),
                                    "c: Reconfigure".into(),
                                    "q: Quit".into(),
                                    "Esc: Close overlays".into(),
                                ]);
                            }
                        }
                        crossterm::event::KeyCode::Esc => {
                            app.show_queue = false;
                            app.help_lines = None;
                        }
                        _ => {}
                    }
                }
                crossterm::event::Event::Mouse(mouse) => {
                    if let crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) = mouse.kind {
                        // Click on top area to focus sidebar, middle for main, bottom for player
                        if let Some(area) = app.area {
                            let ratio = mouse.row as f32 / area.height as f32;
                            if ratio < 0.1 {
                                app.focus = FocusTarget::Sidebar;
                            } else if ratio < 0.8 {
                                app.focus = FocusTarget::MainContent;
                            } else {
                                app.focus = FocusTarget::PlayerBar;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}
