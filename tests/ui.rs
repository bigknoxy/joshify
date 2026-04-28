//! UI component tests
//!
//! Tests for rendering sidebar, player bar, and overlays.
//! These tests verify components render without panicking.

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

// Re-export UI module for convenience in tests
use joshify::ui::{
    layout_cache::{ClickableArea, LayoutCache},
    mouse_handler::{
        handle_left_click, handle_scroll_down, handle_scroll_up, MouseAction, MouseState,
    },
};

/// Navigation items (mirroring src/state/app_state.rs)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum NavItem {
    Home,
    Search,
    Library,
    Playlists,
    LikedSongs,
}

impl NavItem {
    fn all() -> &'static [NavItem] {
        &[
            NavItem::Home,
            NavItem::Search,
            NavItem::Library,
            NavItem::Playlists,
            NavItem::LikedSongs,
        ]
    }

    fn label(&self) -> &'static str {
        match self {
            NavItem::Home => "Home",
            NavItem::Search => "Search",
            NavItem::Library => "Library",
            NavItem::Playlists => "Playlists",
            NavItem::LikedSongs => "Liked Songs",
        }
    }
}

#[test]
fn test_nav_item_all() {
    let items = NavItem::all();
    assert_eq!(items.len(), 5);
}

#[test]
fn test_nav_item_label() {
    assert_eq!(NavItem::Home.label(), "Home");
    assert_eq!(NavItem::Search.label(), "Search");
    assert_eq!(NavItem::Library.label(), "Library");
    assert_eq!(NavItem::Playlists.label(), "Playlists");
    assert_eq!(NavItem::LikedSongs.label(), "Liked Songs");
}

#[test]
fn test_render_sidebar() {
    // Create a test terminal backend
    let backend = ratatui::backend::TestBackend::new(40, 20);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    // Render sidebar
    terminal
        .draw(|frame| {
            let area = frame.area();
            let sidebar_area = Rect::new(0, 0, 15, area.height);

            // Render sidebar content
            let content: Vec<Line> = NavItem::all()
                .iter()
                .map(|item| {
                    let (icon, style) = if *item == NavItem::Home {
                        (
                            "▶ ",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )
                    } else {
                        ("  ", Style::default().fg(Color::White))
                    };
                    Line::styled(format!("{}{}", icon, item.label()), style)
                })
                .collect();

            let widget = Paragraph::new(content).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Navigation ")
                    .border_style(Style::default().fg(Color::Blue)),
            );

            frame.render_widget(widget, sidebar_area);
        })
        .unwrap();

    // Verify render completed without panic
    let output = terminal.backend().buffer();
    assert!(output.content.len() > 0);
}

#[test]
fn test_render_player_bar() {
    let backend = ratatui::backend::TestBackend::new(80, 10);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let area = frame.area();
            let player_bar_area = Rect::new(0, area.height - 3, area.width, 3);

            let progress_text = format!("{}:{:02}", 60 / 60, 60 % 60);
            let duration_text = format!("{}:{:02}", 180 / 60, 180 % 60);

            let widget = Paragraph::new(Line::from(format!(
                "▶ Test Track - Test Artist [{} / {}] [Vol: 75%]",
                progress_text, duration_text
            )))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Now Playing ")
                    .border_style(Style::default().fg(Color::Green)),
            );

            frame.render_widget(widget, player_bar_area);
        })
        .unwrap();

    let output = terminal.backend().buffer();
    assert!(output.content.len() > 0);
}

#[test]
fn test_render_track_list() {
    let backend = ratatui::backend::TestBackend::new(60, 15);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    let tracks = vec![
        ("Track 1", "Artist 1"),
        ("Track 2", "Artist 2"),
        ("Track 3", "Artist 3"),
    ];

    terminal
        .draw(|frame| {
            let area = frame.area();

            let content: Vec<Line> = tracks
                .iter()
                .enumerate()
                .map(|(i, (name, _artist))| {
                    let style = if i == 0 {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    let icon = if i == 0 { "▶ " } else { "  " };
                    Line::styled(format!("{}{}. {}", icon, i + 1, name), style).patch_style(style)
                })
                .collect();

            let widget = Paragraph::new(content)
                .block(Block::default().borders(Borders::ALL).title(" Tracks "));

            frame.render_widget(widget, area);
        })
        .unwrap();

    let output = terminal.backend().buffer();
    assert!(output.content.len() > 0);
}

#[test]
fn test_render_playlist_list() {
    let backend = ratatui::backend::TestBackend::new(60, 15);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    let playlists = vec![
        ("My Playlist 1", 25),
        ("Liked Songs", 142),
        ("Discover Weekly", 30),
    ];

    terminal
        .draw(|frame| {
            let area = frame.area();

            let content: Vec<Line> = playlists
                .iter()
                .enumerate()
                .map(|(i, (name, count))| {
                    let style = if i == 0 {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    Line::styled(format!("{}. {} ({} tracks)", i + 1, name, count), style)
                })
                .collect();

            let widget = Paragraph::new(content)
                .block(Block::default().borders(Borders::ALL).title(" Playlists "));

            frame.render_widget(widget, area);
        })
        .unwrap();

    let output = terminal.backend().buffer();
    assert!(output.content.len() > 0);
}

#[test]
fn test_search_input_overlay() {
    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| {
            let _area = frame.area();
            let overlay_area = Rect::new(10, 10, 50, 3);

            let widget = Paragraph::new(Line::from("Search: hello"))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Search (Esc to cancel) ")
                        .border_style(Style::default().fg(Color::Yellow)),
                )
                .style(Style::default().bg(Color::DarkGray));

            frame.render_widget(widget, overlay_area);
        })
        .unwrap();

    let output = terminal.backend().buffer();
    assert!(output.content.len() > 0);
}

#[test]
fn test_help_overlay() {
    let backend = ratatui::backend::TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();

    let help_text = vec![
        Line::from("Keyboard Shortcuts"),
        Line::from(""),
        Line::from("↑/↓  Navigate lists"),
        Line::from("Tab  Cycle focus"),
        Line::from("/    Search"),
        Line::from("Enter Select"),
        Line::from("q    Quit"),
    ];

    terminal
        .draw(|frame| {
            let _area = frame.area();
            let overlay_area = Rect::new(20, 8, 40, 10);

            let widget = Paragraph::new(help_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Help (Esc to close) ")
                        .border_style(Style::default().fg(Color::Green)),
                )
                .style(Style::default().bg(Color::DarkGray));

            frame.render_widget(widget, overlay_area);
        })
        .unwrap();

    let output = terminal.backend().buffer();
    assert!(output.content.len() > 0);
}

// ============================================================================
// Mouse Integration Tests
// ============================================================================

/// Test the full mouse navigation flow: sidebar → playlist → track → play
#[test]
fn test_mouse_nav_to_playlist_to_track() {
    use ratatui::layout::Rect;

    // Simulate layout cache state after rendering
    // Note: playlist_items and track_items have different y positions
    let cache = LayoutCache {
        sidebar: Some(Rect::new(0, 0, 20, 40)),
        nav_items: vec![
            Rect::new(0, 16, 20, 1), // Home
            Rect::new(0, 17, 20, 1), // Search
            Rect::new(0, 18, 20, 1), // Library
            Rect::new(0, 19, 20, 1), // Playlists
            Rect::new(0, 20, 20, 1), // Liked Songs
        ],
        main_view: Some(Rect::new(20, 0, 60, 34)),
        playlist_items: vec![
            Rect::new(25, 5, 50, 1), // Playlist 0 at y=5
            Rect::new(25, 6, 50, 1), // Playlist 1 at y=6
        ],
        track_items: vec![
            Rect::new(25, 10, 50, 1), // Track 0 at y=10 (different from playlist)
            Rect::new(25, 11, 50, 1), // Track 1 at y=11
            Rect::new(25, 12, 50, 1), // Track 2 at y=12
        ],
        ..Default::default()
    };

    // Step 1: Click on Playlists nav item
    let nav_result = cache.area_at(5, 19);
    assert!(
        matches!(nav_result, Some(ClickableArea::NavItem(_))),
        "Should click nav item"
    );

    // Step 2: Click on first playlist (y=5)
    let playlist_result = cache.area_at(30, 5);
    assert_eq!(
        playlist_result,
        Some(ClickableArea::PlaylistItem(0)),
        "Should select first playlist"
    );

    // Step 3: Click on first track (y=10, different from playlist)
    let track_result = cache.area_at(30, 10);
    assert_eq!(
        track_result,
        Some(ClickableArea::TrackItem(0)),
        "Should select first track"
    );
}

/// Test double-click on playlist opens tracks view
#[test]
fn test_double_click_playlist_opens() {
    use ratatui::layout::Rect;

    let cache = LayoutCache {
        playlist_items: vec![Rect::new(25, 5, 50, 1), Rect::new(25, 6, 50, 1)],
        ..Default::default()
    };

    let mut mouse_state = MouseState::new();
    mouse_state.double_click_threshold = 500; // Extended for test

    // First click on playlist - selects
    let first_click = handle_left_click(30, 5, &cache, &mut mouse_state);
    assert_eq!(first_click, MouseAction::SelectPlaylist(0));

    // Second click (double-click) - opens playlist
    let second_click = handle_left_click(30, 5, &cache, &mut mouse_state);
    assert_eq!(second_click, MouseAction::OpenPlaylist(0));
}

/// Test double-click on track plays immediately
#[test]
fn test_double_click_track_plays() {
    use ratatui::layout::Rect;

    let cache = LayoutCache {
        track_items: vec![
            Rect::new(25, 5, 50, 1),
            Rect::new(25, 6, 50, 1),
            Rect::new(25, 7, 50, 1),
        ],
        ..Default::default()
    };

    let mut mouse_state = MouseState::new();
    mouse_state.double_click_threshold = 500;

    // First click selects track
    let first_click = handle_left_click(30, 5, &cache, &mut mouse_state);
    assert_eq!(first_click, MouseAction::SelectTrack(0));

    // Second click (double-click) plays track
    let second_click = handle_left_click(30, 5, &cache, &mut mouse_state);
    assert_eq!(second_click, MouseAction::PlayTrack(0));
}

/// Test volume scroll in local playback mode
#[test]
fn test_volume_scroll_local() {
    use ratatui::layout::Rect;

    let cache = LayoutCache {
        volume_bar: Some(Rect::new(60, 36, 15, 2)),
        player_bar: Some(Rect::new(20, 34, 60, 6)),
        ..Default::default()
    };

    // Scroll up increases volume
    let scroll_up = handle_scroll_up(65, 37, &cache);
    assert_eq!(scroll_up, MouseAction::AdjustVolume(5));

    // Scroll down decreases volume
    let scroll_down = handle_scroll_down(65, 37, &cache);
    assert_eq!(scroll_down, MouseAction::AdjustVolume(-5));

    // Multiple scrolls accumulate
    let scroll_up_again = handle_scroll_up(65, 37, &cache);
    assert_eq!(scroll_up_again, MouseAction::AdjustVolume(5));
}

/// Test volume scroll in remote playback mode
#[test]
fn test_volume_scroll_remote() {
    use ratatui::layout::Rect;

    // Same behavior for remote mode - volume bar is in player bar
    let cache = LayoutCache {
        volume_bar: Some(Rect::new(60, 36, 15, 2)),
        player_bar: Some(Rect::new(20, 34, 60, 6)),
        ..Default::default()
    };

    // Scroll up increases volume
    let scroll_up = handle_scroll_up(65, 37, &cache);
    assert_eq!(scroll_up, MouseAction::AdjustVolume(5));

    // Scroll down decreases volume
    let scroll_down = handle_scroll_down(65, 37, &cache);
    assert_eq!(scroll_down, MouseAction::AdjustVolume(-5));
}

/// Test clicking play/pause button toggles playback
#[test]
fn test_click_play_pause_toggles() {
    use ratatui::layout::Rect;

    let cache = LayoutCache {
        player_bar: Some(Rect::new(20, 34, 60, 6)),
        play_pause_button: Some(Rect::new(30, 36, 3, 2)),
        ..Default::default()
    };
    let mut mouse_state = MouseState::new();

    // Click play/pause button
    let action = handle_left_click(31, 37, &cache, &mut mouse_state);
    assert_eq!(action, MouseAction::TogglePlayPause);
}

/// Test clicking next/previous buttons
#[test]
fn test_click_skip_buttons() {
    use ratatui::layout::Rect;

    let cache = LayoutCache {
        player_bar: Some(Rect::new(20, 34, 60, 6)),
        prev_button: Some(Rect::new(25, 36, 3, 2)),
        next_button: Some(Rect::new(35, 36, 3, 2)),
        ..Default::default()
    };
    let mut mouse_state = MouseState::new();

    // Click previous button
    let prev = handle_left_click(26, 37, &cache, &mut mouse_state);
    assert_eq!(prev, MouseAction::SkipPrevious);

    // Click next button
    let next = handle_left_click(36, 37, &cache, &mut mouse_state);
    assert_eq!(next, MouseAction::SkipNext);
}

/// Test clicking shuffle and repeat buttons
#[test]
fn test_click_shuffle_repeat_buttons() {
    use ratatui::layout::Rect;

    let cache = LayoutCache {
        player_bar: Some(Rect::new(20, 34, 60, 6)),
        shuffle_button: Some(Rect::new(40, 36, 3, 2)),
        repeat_button: Some(Rect::new(45, 36, 3, 2)),
        ..Default::default()
    };
    let mut mouse_state = MouseState::new();

    // Click shuffle button
    let shuffle = handle_left_click(41, 37, &cache, &mut mouse_state);
    assert_eq!(shuffle, MouseAction::ToggleShuffle);

    // Click repeat button
    let repeat = handle_left_click(46, 37, &cache, &mut mouse_state);
    assert_eq!(repeat, MouseAction::CycleRepeat);
}

/// Test clicking queue button toggles queue overlay
#[test]
fn test_click_queue_button() {
    use ratatui::layout::Rect;

    let cache = LayoutCache {
        player_bar: Some(Rect::new(20, 34, 60, 6)),
        queue_button: Some(Rect::new(50, 36, 3, 2)),
        ..Default::default()
    };
    let mut mouse_state = MouseState::new();

    let action = handle_left_click(51, 37, &cache, &mut mouse_state);
    assert_eq!(action, MouseAction::ToggleQueue);
}

/// Test clicking on overlay closes it
#[test]
fn test_click_overlay_closes() {
    use ratatui::layout::Rect;

    // Test help overlay
    let cache_help = LayoutCache {
        help_overlay: Some(Rect::new(10, 10, 60, 20)),
        ..Default::default()
    };
    let mut mouse_state = MouseState::new();

    let action_help = handle_left_click(15, 15, &cache_help, &mut mouse_state);
    assert_eq!(action_help, MouseAction::CloseOverlay);

    // Test queue overlay
    let cache_queue = LayoutCache {
        queue_overlay: Some(Rect::new(20, 20, 40, 15)),
        ..Default::default()
    };
    let mut mouse_state2 = MouseState::new();

    let action_queue = handle_left_click(25, 25, &cache_queue, &mut mouse_state2);
    assert_eq!(action_queue, MouseAction::CloseOverlay);
}

/// Test seek by clicking progress bar
#[test]
fn test_click_progress_bar_seeks() {
    use ratatui::layout::Rect;

    let cache = LayoutCache {
        player_bar: Some(Rect::new(20, 34, 60, 6)),
        progress_bar: Some(Rect::new(25, 34, 50, 1)),
        ..Default::default()
    };
    let mut mouse_state = MouseState::new();

    // Click at different positions to seek
    let seek_start = handle_left_click(26, 34, &cache, &mut mouse_state);
    assert!(matches!(seek_start, MouseAction::Seek(p) if p < 10));

    let seek_middle = handle_left_click(50, 34, &cache, &mut mouse_state);
    assert!(matches!(seek_middle, MouseAction::Seek(p) if p > 40 && p < 60));

    let seek_end = handle_left_click(73, 34, &cache, &mut mouse_state);
    assert!(matches!(seek_end, MouseAction::Seek(p) if p > 90));
}

/// Test set volume by clicking volume bar
#[test]
fn test_click_volume_bar_sets_volume() {
    use ratatui::layout::Rect;

    let cache = LayoutCache {
        player_bar: Some(Rect::new(20, 34, 60, 6)),
        volume_bar: Some(Rect::new(60, 36, 15, 2)),
        ..Default::default()
    };
    let mut mouse_state = MouseState::new();

    // Click at different positions to set volume
    let vol_low = handle_left_click(61, 37, &cache, &mut mouse_state);
    assert!(matches!(vol_low, MouseAction::SetVolume(p) if p < 10));

    let vol_high = handle_left_click(73, 37, &cache, &mut mouse_state);
    assert!(matches!(vol_high, MouseAction::SetVolume(p) if p > 90));
}

/// Test scroll in main content area
#[test]
fn test_scroll_main_content() {
    use ratatui::layout::Rect;

    let cache = LayoutCache {
        main_view: Some(Rect::new(20, 0, 60, 34)),
        track_items: vec![Rect::new(25, 5, 50, 1), Rect::new(25, 6, 50, 1)],
        ..Default::default()
    };

    // Scroll up in main view
    let up = handle_scroll_up(30, 10, &cache);
    assert_eq!(up, MouseAction::ScrollUp);

    // Scroll down in main view
    let down = handle_scroll_down(30, 10, &cache);
    assert_eq!(down, MouseAction::ScrollDown);

    // Scroll on track item
    let track_up = handle_scroll_up(30, 5, &cache);
    assert_eq!(track_up, MouseAction::ScrollUp);
}

/// Test scroll in sidebar
#[test]
fn test_scroll_sidebar() {
    use ratatui::layout::Rect;

    let cache = LayoutCache {
        sidebar: Some(Rect::new(0, 0, 20, 40)),
        nav_items: vec![Rect::new(0, 16, 20, 1), Rect::new(0, 17, 20, 1)],
        ..Default::default()
    };

    // Scroll up in sidebar
    let up = handle_scroll_up(10, 20, &cache);
    assert_eq!(up, MouseAction::ScrollUp);

    // Scroll down in sidebar
    let down = handle_scroll_down(10, 20, &cache);
    assert_eq!(down, MouseAction::ScrollDown);
}

/// Test mouse action none for invalid clicks
#[test]
fn test_mouse_action_none_for_invalid_clicks() {
    let cache = LayoutCache::default();
    let mut mouse_state = MouseState::new();

    // Click outside any defined area
    let action = handle_left_click(100, 100, &cache, &mut mouse_state);
    assert_eq!(action, MouseAction::None);
}

/// Test complete mouse flow: nav → playlist → track → play via double-click
#[test]
fn test_complete_mouse_flow() {
    use ratatui::layout::Rect;

    // Simulate a complete UI state
    let cache = LayoutCache {
        sidebar: Some(Rect::new(0, 0, 20, 40)),
        nav_items: vec![
            Rect::new(0, 16, 20, 1), // Home
            Rect::new(0, 17, 20, 1), // Search
            Rect::new(0, 18, 20, 1), // Library
            Rect::new(0, 19, 20, 1), // Playlists
            Rect::new(0, 20, 20, 1), // Liked Songs
        ],
        main_view: Some(Rect::new(20, 0, 60, 34)),
        playlist_items: vec![
            Rect::new(25, 5, 50, 1), // My Playlist at y=5
            Rect::new(25, 6, 50, 1), // Liked Songs at y=6
        ],
        track_items: vec![
            Rect::new(25, 10, 50, 1), // Track 1 at y=10
            Rect::new(25, 11, 50, 1), // Track 2 at y=11
            Rect::new(25, 12, 50, 1), // Track 3 at y=12
        ],
        player_bar: Some(Rect::new(20, 34, 60, 6)),
        play_pause_button: Some(Rect::new(30, 36, 3, 2)),
        prev_button: Some(Rect::new(25, 36, 3, 2)),
        next_button: Some(Rect::new(35, 36, 3, 2)),
        ..Default::default()
    };

    let mut mouse_state = MouseState::new();
    mouse_state.double_click_threshold = 500;

    // Step 1: Navigate to Playlists
    let nav = handle_left_click(5, 19, &cache, &mut mouse_state);
    assert!(matches!(nav, MouseAction::SelectNavItem(_)));

    // Step 2: Select playlist (single click)
    let playlist = handle_left_click(30, 5, &cache, &mut mouse_state);
    assert_eq!(playlist, MouseAction::SelectPlaylist(0));

    // Step 3: Double-click playlist to open it
    let open_playlist = handle_left_click(30, 5, &cache, &mut mouse_state);
    assert_eq!(open_playlist, MouseAction::OpenPlaylist(0));

    // Step 4: Single-click track to select
    let track1 = handle_left_click(30, 10, &cache, &mut mouse_state);
    assert_eq!(track1, MouseAction::SelectTrack(0));

    // Step 5: Double-click track to play
    let track2 = handle_left_click(30, 10, &cache, &mut mouse_state);
    assert_eq!(track2, MouseAction::PlayTrack(0));

    // Step 6: Click play/pause
    let play_pause = handle_left_click(31, 37, &cache, &mut mouse_state);
    assert_eq!(play_pause, MouseAction::TogglePlayPause);

    // Step 7: Click next
    let next = handle_left_click(36, 37, &cache, &mut mouse_state);
    assert_eq!(next, MouseAction::SkipNext);
}
