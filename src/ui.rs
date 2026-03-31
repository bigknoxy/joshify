//! UI components and layout

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

/// Navigation items for the sidebar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavItem {
    Home,
    Search,
    Library,
    Playlists,
    LikedSongs,
}

impl NavItem {
    pub fn all() -> &'static [NavItem] {
        &[NavItem::Home, NavItem::Search, NavItem::Library, NavItem::Playlists, NavItem::LikedSongs]
    }

    pub fn label(&self) -> &'static str {
        match self {
            NavItem::Home => "Home",
            NavItem::Search => "Search",
            NavItem::Library => "Library",
            NavItem::Playlists => "Playlists",
            NavItem::LikedSongs => "Liked Songs",
        }
    }
}

/// The Joshify mascot - original kaiju-inspired digital creature
fn joshify_logo() -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::styled("     ⚡ JOSHIFY ⚡", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Line::styled("    ╱▔▔▔▔▔▔▔▔▔╲", Style::default().fg(Color::Green)),
        Line::styled("   ╱  ▀▄   ▄▀  ╲", Style::default().fg(Color::Green)),
        Line::styled("  │   ▄▀▀▀▀▄   │", Style::default().fg(Color::Green)),
        Line::styled("  │  │ ▀▀ │  │", Style::default().fg(Color::Green)),
        Line::styled("   ╲  ╲__╱  ╱", Style::default().fg(Color::Green)),
        Line::styled("    ╲_____╱", Style::default().fg(Color::Green)),
        Line::from(""),
    ]
}

/// Render search input overlay
fn render_search_input(frame: &mut ratatui::Frame, area: Rect, query: &str, border_color: Color) {
    use ratatui::widgets::Clear;

    // Create centered search box
    let search_width = 50u16.min(area.width);
    let search_height = 7u16.min(area.height);
    let search_x = (area.width - search_width) / 2;
    let search_y = (area.height - search_height) / 2;
    let search_area = Rect::new(search_x, search_y, search_width, search_height);

    // Clear the area first (solid background, no transparency)
    frame.render_widget(Clear, search_area);

    // Build the input line with cursor
    let cursor_pos = query.len() as u16;
    let input_line = Line::styled(
        format!("{}█", query),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    );

    let content = vec![
        Line::from(""),
        Line::styled("Search Spotify", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Line::from(""),
        input_line,
        Line::from(""),
        Line::styled("Enter to search | Esc to cancel", Style::default().fg(Color::Gray)),
    ];

    let widget = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Search ")
                .border_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD))
                .style(Style::default().bg(Color::Black))
        );

    frame.render_widget(widget, search_area);

    // Set cursor position
    frame.set_cursor_position((search_area.x + 2 + cursor_pos, search_area.y + 3));
}

/// Render the sidebar navigation
pub fn render_sidebar(frame: &mut ratatui::Frame, area: Rect, selected: NavItem, focused: bool) {
    let border_color = if focused { Color::Yellow } else { Color::Blue };
    let title = if focused { " Navigation (↑/↓) " } else { " Navigation " };

    // Build content with logo at top
    let mut content = joshify_logo();
    content.push(Line::from("")); // Spacer

    let items: Vec<Line> = NavItem::all()
        .iter()
        .map(|item| {
            let (icon, style) = if *item == selected {
                ("▶ ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            } else {
                ("  ", Style::default().fg(Color::White))
            };
            Line::styled(format!("{}{}", icon, item.label()), style)
        })
        .collect();
    content.extend(items);

    let widget = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD))
                .title_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD))
        );

    frame.render_widget(widget, area);
}

/// Render the player bar at the bottom with integrated album art
pub fn render_player_bar(
    frame: &mut ratatui::Frame,
    area: Rect,
    track_name: &str,
    artist_name: &str,
    is_playing: bool,
    progress_ms: u32,
    duration_ms: u32,
    volume: u32,
    album_art_url: Option<&str>,
    album_art_data: Option<&[u8]>,
    focused: bool,
) {
    use crate::player::format_duration;

    let play_icon = if is_playing { "▶" } else { "||" };
    let display_name = if track_name.is_empty() { "Not Playing" } else { track_name };

    // Split player bar into album art (left) and info (right)
    let album_art_width = 7u16; // Space for album art
    let [album_area, info_area] = Layout::horizontal([
        Constraint::Length(album_art_width),
        Constraint::Min(0),
    ])
    .areas(area);

    // Render album art - use actual image data when available, ASCII fallback otherwise
    let album_art_widget: Paragraph = if let Some(_data) = album_art_data {
        // Have image data - show enhanced ASCII art indicating real art is loaded
        // (Full image rendering requires ImageState which needs picker from main app)
        let art = vec![
            Line::from("  ╭───╮  "),
            Line::from("  │▓▓▓│  "),
            Line::from("  │▓▓▓│  "),
            Line::from("  ╰───╯  "),
        ];
        Paragraph::new(art)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                    .title("Album")
                    .title_style(Style::default().fg(Color::Green))
            )
            .alignment(Alignment::Center)
    } else if album_art_url.is_some() {
        // Have URL but no data yet - show loading indicator
        let art = vec![
            Line::from("  ╭───╮  "),
            Line::from("  │...│  "),
            Line::from("  │...│  "),
            Line::from("  ╰───╯  "),
        ];
        Paragraph::new(art)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title("Loading...")
                    .title_style(Style::default().fg(Color::Yellow))
            )
            .alignment(Alignment::Center)
    } else {
        // No album art - show music note placeholder
        let art = vec![
            Line::from("       "),
            Line::from("  ♪    "),
            Line::from("       "),
            Line::from("       "),
        ];
        Paragraph::new(art)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title("No Art")
                    .title_style(Style::default().fg(Color::DarkGray))
            )
            .alignment(Alignment::Center)
    };
    frame.render_widget(album_art_widget, album_area);

    // Track info
    let progress_text = format!("{} / {}", format_duration(progress_ms), format_duration(duration_ms));
    let volume_bars = match volume {
        0 => "░░░░",
        1..=25 => "█░░░",
        26..=50 => "██░░",
        51..=75 => "███░",
        _ => "████",
    };

    // Truncate for available space
    let max_len = info_area.width.saturating_sub(2) as usize;
    let name_text = if display_name.len() + artist_name.len() + 3 > max_len {
        let half = max_len / 2 - 2;
        format!("{}... / {}...",
            &display_name.chars().take(half).collect::<String>(),
            &artist_name.chars().take(half).collect::<String>())
    } else {
        format!("{} - {}", display_name, artist_name)
    };

    let lines = vec![
        Line::styled(
            format!(" {}  {}", play_icon, name_text),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
        ),
        Line::styled(
            format!(" {}  |  Vol:{}  |  ←/→:Seek  |  ↑/↓:Vol", progress_text, volume_bars),
            Style::default().fg(Color::Gray)
        ),
    ];

    let border_color = if focused { Color::Yellow } else { Color::Green };
    let focus_hint = if focused { " Now Playing (Enter:Play/Pause) " } else { " Now Playing " };

    let widget = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(focus_hint)
                .border_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD))
                .title_style(Style::default().fg(border_color).add_modifier(Modifier::BOLD))
        );

    frame.render_widget(widget, info_area);
}


/// Main content view based on navigation selection
pub fn render_main_view(
    frame: &mut ratatui::Frame,
    area: Rect,
    _selected_nav: NavItem,
    is_authenticated: bool,
    focused: bool,
    content_state: &crate::MainContentState,
    selected_index: usize,
    scroll_offset: usize,
    is_searching: bool,
    search_query: &str,
) {
    let border_color = if focused { Color::Yellow } else { Color::White };
    let title = if focused { " Main (Enter to play) " } else { " Main " };

    // Show search input overlay when actively searching
    if is_searching {
        render_search_input(frame, area, search_query, border_color);
        return;
    }

    // Check for small terminal - show compact view
    if area.width < 50 || area.height < 15 {
        let content = match content_state {
            crate::MainContentState::Home => {
                if is_authenticated { "Home" } else { "Not connected" }
            }
            crate::MainContentState::Loading(msg) | crate::MainContentState::LoadingInProgress(msg) => msg.as_str(),
            crate::MainContentState::LikedSongs(tracks) => {
                format!("Liked Songs ({} tracks)", tracks.len()).leak()
            }
            crate::MainContentState::Playlists(playlists) => {
                format!("Playlists ({})", playlists.len()).leak()
            }
            crate::MainContentState::PlaylistTracks(name, tracks) => {
                format!("{} ({} tracks)", name, tracks.len()).leak()
            }
            crate::MainContentState::SearchResults(query, tracks) => {
                format!("Results for '{}': {}", query, tracks.len()).leak()
            }
            crate::MainContentState::Error(msg) => msg.as_str(),
        };
        let widget = Paragraph::new(content.to_string())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(border_color))
            );
        frame.render_widget(widget, area);
        return;
    }

    // Render based on content state
    match content_state {
        crate::MainContentState::Home => {
            let content = if is_authenticated {
                "Home\n\n• Recently played\n• Featured playlists\n• Made for you\n\nSelect 'Liked Songs' or 'Playlists' from sidebar"
            } else {
                "Not connected to Spotify\n\nPress 'c' to configure"
            };
            let widget = Paragraph::new(content)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(title)
                        .border_style(Style::default().fg(border_color))
                );
            frame.render_widget(widget, area);
        }
        crate::MainContentState::Loading(msg) | crate::MainContentState::LoadingInProgress(msg) => {
            let display_msg = format!("{} ⏳", msg);
            let widget = Paragraph::new(display_msg.as_str())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(title)
                        .border_style(Style::default().fg(border_color))
                );
            frame.render_widget(widget, area);
        }
        crate::MainContentState::Error(msg) => {
            let widget = Paragraph::new(msg.as_str())
                .style(Style::default().fg(Color::Red))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(title)
                        .border_style(Style::default().fg(border_color))
                );
            frame.render_widget(widget, area);
        }
        crate::MainContentState::LikedSongs(tracks) |
        crate::MainContentState::PlaylistTracks(_, tracks) |
        crate::MainContentState::SearchResults(_, tracks) => {
            render_track_list(frame, area, tracks, selected_index, scroll_offset, title, border_color);
        }
        crate::MainContentState::Playlists(playlists) => {
            render_playlist_list(frame, area, playlists, selected_index, scroll_offset, title, border_color);
        }
    }
}

/// Render a list of tracks
fn render_track_list(
    frame: &mut ratatui::Frame,
    area: Rect,
    tracks: &[crate::TrackListItem],
    selected_index: usize,
    scroll_offset: usize,
    title: &str,
    border_color: ratatui::prelude::Color,
) {
    use ratatui::widgets::List;

    if tracks.is_empty() {
        let widget = Paragraph::new("No tracks found")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(border_color))
            );
        frame.render_widget(widget, area);
        return;
    }

    // Calculate visible area (leave room for header)
    let header_height = 3u16;
    let list_area = Rect::new(
        area.x,
        area.y + header_height as u16,
        area.width,
        area.height.saturating_sub(header_height),
    );

    // Render header
    let header = Paragraph::new(format!("{} tracks | Enter to play | ↑/↓ to navigate", tracks.len()))
        .style(Style::default().fg(Color::Gray))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(border_color))
        );
    frame.render_widget(header, area);

    // Build visible lines
    let visible_count = list_area.height as usize;
    let end = (scroll_offset + visible_count).min(tracks.len());

    let lines: Vec<Line> = (scroll_offset..end)
        .map(|i| {
            let track = &tracks[i];
            let is_selected = i == selected_index;
            let (prefix, style) = if is_selected {
                ("▶ ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            } else {
                ("  ", Style::default().fg(Color::White))
            };
            Line::styled(
                format!("{}{} - {}", prefix, track.name, track.artist),
                style
            )
        })
        .collect();

    let list = List::new(lines)
        .block(
            Block::default()
                .borders(Borders::NONE)
        );

    frame.render_widget(list, list_area);
}

/// Render a list of playlists
fn render_playlist_list(
    frame: &mut ratatui::Frame,
    area: Rect,
    playlists: &[crate::PlaylistListItem],
    selected_index: usize,
    scroll_offset: usize,
    title: &str,
    border_color: ratatui::prelude::Color,
) {
    use ratatui::widgets::List;

    if playlists.is_empty() {
        let widget = Paragraph::new("No playlists found")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(border_color))
            );
        frame.render_widget(widget, area);
        return;
    }

    // Calculate visible area (leave room for header)
    let header_height = 3u16;
    let list_area = Rect::new(
        area.x,
        area.y + header_height as u16,
        area.width,
        area.height.saturating_sub(header_height),
    );

    // Render header
    let header = Paragraph::new(format!("{} playlists | Enter to view | ↑/↓ to navigate", playlists.len()))
        .style(Style::default().fg(Color::Gray))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(border_color))
        );
    frame.render_widget(header, area);

    // Build visible lines
    let visible_count = list_area.height as usize;
    let end = (scroll_offset + visible_count).min(playlists.len());

    let lines: Vec<Line> = (scroll_offset..end)
        .map(|i| {
            let playlist = &playlists[i];
            let is_selected = i == selected_index;
            let (prefix, style) = if is_selected {
                ("▶ ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            } else {
                ("  ", Style::default().fg(Color::White))
            };
            Line::styled(
                format!("{}{} ({} tracks)", prefix, playlist.name, playlist.track_count),
                style
            )
        })
        .collect();

    let list = List::new(lines)
        .block(
            Block::default()
                .borders(Borders::NONE)
        );

    frame.render_widget(list, list_area);
}

/// Render queue overlay
pub fn render_queue_overlay(frame: &mut ratatui::Frame, area: Rect, _player_state: &crate::player::PlayerState) {
    render_overlay_base(frame, area, " Queue ", vec![
        Line::from(""),
        Line::styled("=== Current Queue ===", Style::default().add_modifier(Modifier::BOLD)),
        Line::from(""),
        Line::from("• Up next: Will be shown here"),
        Line::from(""),
        Line::from("Press Esc to close"),
    ]);
}

/// Render help overlay
pub fn render_help_overlay(frame: &mut ratatui::Frame, area: Rect, help_lines: &[String]) {
    let lines: Vec<Line> = help_lines.iter()
        .map(|l| {
            if l.starts_with("===") {
                Line::styled(l.clone(), Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
            } else {
                Line::from(l.clone())
            }
        })
        .collect();

    render_overlay_base(frame, area, " Help (?/Esc) ", lines);
}

/// Base overlay renderer
fn render_overlay_base(frame: &mut ratatui::Frame, area: Rect, title: &str, content: Vec<Line>) {
    // Create centered overlay area
    let overlay_width = (area.width as f32 * 0.7).clamp(40.0, area.width as f32) as u16;
    let overlay_height = (area.height as f32 * 0.7).clamp(15.0, area.height as f32) as u16;
    let overlay_x = (area.width - overlay_width) / 2;
    let overlay_y = (area.height - overlay_height) / 2;
    let overlay_area = Rect::new(overlay_x, overlay_y, overlay_width, overlay_height);

    // Fill background with solid color first (no transparency)
    let bg = Block::default()
        .style(Style::default().bg(Color::Black));
    frame.render_widget(bg, overlay_area);

    let widget = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
                .style(Style::default().bg(Color::Black))
        )
        .alignment(Alignment::Left);

    frame.render_widget(widget, overlay_area);
}
