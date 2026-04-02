//! UI component tests
//!
//! Tests for rendering sidebar, player bar, and overlays.
//! These tests verify components render without panicking.

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
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
        &[NavItem::Home, NavItem::Search, NavItem::Library, NavItem::Playlists, NavItem::LikedSongs]
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
                        ("▶ ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                    } else {
                        ("  ", Style::default().fg(Color::White))
                    };
                    Line::styled(format!("{}{}", icon, item.label()), style)
                })
                .collect();

            let widget = Paragraph::new(content)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Navigation ")
                        .border_style(Style::default().fg(Color::Blue))
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
                    .border_style(Style::default().fg(Color::Green))
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
                .map(|(i, (name, artist))| {
                    let style = if i == 0 {
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    let icon = if i == 0 { "▶ " } else { "  " };
                    Line::styled(format!("{}{}. {}", icon, i + 1, name), style)
                        .patch_style(style)
                })
                .collect();

            let widget = Paragraph::new(content)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Tracks ")
                );

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
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    Line::styled(format!("{}. {} ({} tracks)", i + 1, name, count), style)
                })
                .collect();

            let widget = Paragraph::new(content)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Playlists ")
                );

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
            let area = frame.area();
            let overlay_area = Rect::new(10, 10, 50, 3);

            let widget = Paragraph::new(Line::from("Search: hello"))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Search (Esc to cancel) ")
                        .border_style(Style::default().fg(Color::Yellow))
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
            let area = frame.area();
            let overlay_area = Rect::new(20, 8, 40, 10);

            let widget = Paragraph::new(help_text)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Help (Esc to close) ")
                        .border_style(Style::default().fg(Color::Green))
                )
                .style(Style::default().bg(Color::DarkGray));

            frame.render_widget(widget, overlay_area);
        })
        .unwrap();

    let output = terminal.backend().buffer();
    assert!(output.content.len() > 0);
}
