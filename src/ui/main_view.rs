//! Main content view rendering

use crate::state::app_state::{ContentState, PlaylistListItem, TrackListItem};
use crate::state::load_coordinator::LoadAction;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, Paragraph},
};

// MainContentState is an alias for ContentState - they are the same type
type MainContentState = ContentState;

/// Get a display message from a LoadAction (owned version)
fn load_action_display_owned(action: &LoadAction) -> String {
    match action {
        LoadAction::LikedSongs => "Loading liked songs...".to_string(),
        LoadAction::Playlists => "Loading playlists...".to_string(),
        LoadAction::PlaylistTracks { name, .. } => format!("Loading {}...", name),
        LoadAction::Search { query } => format!("Searching: {}", query),
        LoadAction::Devices => "Loading devices...".to_string(),
    }
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
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let content = vec![
        Line::from(""),
        Line::styled(
            "Search Spotify",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(""),
        input_line,
        Line::from(""),
        Line::styled(
            "Enter to search | Esc to cancel",
            Style::default().fg(Color::Gray),
        ),
    ];

    let widget = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Search ")
            .border_style(
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().bg(Color::Black)),
    );

    frame.render_widget(widget, search_area);

    // Set cursor position
    frame.set_cursor_position((search_area.x + 2 + cursor_pos, search_area.y + 3));
}

/// Render a list of tracks
fn render_track_list(
    frame: &mut ratatui::Frame,
    area: Rect,
    tracks: &[TrackListItem],
    selected_index: usize,
    scroll_offset: usize,
    title: &str,
    border_color: Color,
) {
    if tracks.is_empty() {
        let widget = Paragraph::new("No tracks found").block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(border_color)),
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
    let header = Paragraph::new(format!(
        "{} tracks | Enter to play | ↑/↓ to navigate",
        tracks.len()
    ))
    .style(Style::default().fg(Color::Gray))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color)),
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
                (
                    "▶ ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ("  ", Style::default().fg(Color::White))
            };
            Line::styled(
                format!("{}{} - {}", prefix, track.name, track.artist),
                style,
            )
        })
        .collect();

    let list = List::new(lines).block(Block::default().borders(Borders::NONE));

    frame.render_widget(list, list_area);
}

/// Render a list of playlists
fn render_playlist_list(
    frame: &mut ratatui::Frame,
    area: Rect,
    playlists: &[PlaylistListItem],
    selected_index: usize,
    scroll_offset: usize,
    title: &str,
    border_color: Color,
) {
    if playlists.is_empty() {
        let widget = Paragraph::new("No playlists found").block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(border_color)),
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
    let header = Paragraph::new(format!(
        "{} playlists | Enter to view | ↑/↓ to navigate",
        playlists.len()
    ))
    .style(Style::default().fg(Color::Gray))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color)),
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
                (
                    "▶ ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ("  ", Style::default().fg(Color::White))
            };
            Line::styled(
                format!(
                    "{}{} ({} tracks)",
                    prefix, playlist.name, playlist.track_count
                ),
                style,
            )
        })
        .collect();

    let list = List::new(lines).block(Block::default().borders(Borders::NONE));

    frame.render_widget(list, list_area);
}

/// Compact track list rendering for small terminals
fn render_track_list_compact(
    frame: &mut ratatui::Frame,
    area: Rect,
    tracks: &[TrackListItem],
    title: &str,
    border_color: Color,
) {
    let content = if tracks.is_empty() {
        "No tracks".to_string()
    } else {
        format!("{} ({} tracks)", title, tracks.len())
    };
    let widget = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color)),
    );
    frame.render_widget(widget, area);
}

/// Compact playlist list rendering for small terminals
fn render_playlist_list_compact(
    frame: &mut ratatui::Frame,
    area: Rect,
    playlists: &[PlaylistListItem],
    title: &str,
    border_color: Color,
) {
    let content = if playlists.is_empty() {
        "No playlists".to_string()
    } else {
        format!("{} ({} playlists)", title, playlists.len())
    };
    let widget = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(border_color)),
    );
    frame.render_widget(widget, area);
}

/// Main content view based on navigation selection
pub fn render_main_view(
    frame: &mut ratatui::Frame,
    area: Rect,
    _selected_nav: crate::state::app_state::NavItem,
    _is_authenticated: bool,
    focused: bool,
    content_state: &MainContentState,
    selected_index: usize,
    scroll_offset: usize,
    is_searching: bool,
    search_query: &str,
) {
    let border_color = if focused { Color::Yellow } else { Color::White };
    let title = if focused {
        " Main (Enter to play) "
    } else {
        " Main "
    };

    // Show search input overlay when actively searching
    if is_searching {
        render_search_input(frame, area, search_query, border_color);
        return;
    }

    // Check for small terminal - show compact view
    if area.width < 50 || area.height < 15 {
        let content = match content_state {
            ContentState::Home => {
                vec![Line::from("")]
            }
            ContentState::Loading(action) | ContentState::LoadingInProgress(action) => {
                vec![Line::from(load_action_display_owned(action))]
            }
            ContentState::LikedSongs(tracks) => {
                return render_track_list(
                    frame,
                    area,
                    tracks,
                    selected_index,
                    scroll_offset,
                    " Liked Songs ",
                    border_color,
                );
            }
            ContentState::Playlists(playlists) => {
                return render_playlist_list(
                    frame,
                    area,
                    playlists,
                    selected_index,
                    scroll_offset,
                    " Playlists ",
                    border_color,
                );
            }
            ContentState::PlaylistTracks(name, tracks) => {
                return render_track_list(
                    frame,
                    area,
                    tracks,
                    selected_index,
                    scroll_offset,
                    &format!(" {} ", name),
                    border_color,
                );
            }
            ContentState::SearchResults(query, tracks) => {
                return render_track_list_compact(
                    frame,
                    area,
                    tracks,
                    &format!("Results: {}", query),
                    border_color,
                );
            }
            ContentState::DeviceSelector(entries) => {
                return crate::ui::device_selector::render_device_selector(
                    frame,
                    area,
                    entries,
                    selected_index,
                );
            }
            ContentState::Error(msg) => {
                let widget = Paragraph::new(msg.as_str()).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Error ")
                        .border_style(Style::default().fg(Color::Red)),
                );
                frame.render_widget(widget, area);
                return;
            }
        };
        let widget = Paragraph::new(content).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(border_color)),
        );
        frame.render_widget(widget, area);
        return;
    }

    // Render based on content state
    match content_state {
        ContentState::Home => {
            let content = if _is_authenticated {
                "Home\n\n• Recently played\n• Featured playlists\n• Made for you\n\nSelect 'Liked Songs' or 'Playlists' from sidebar"
            } else {
                "Not connected to Spotify\n\nPress 'c' to configure"
            };
            let widget = Paragraph::new(content).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(border_color)),
            );
            frame.render_widget(widget, area);
        }
        ContentState::Loading(action) | ContentState::LoadingInProgress(action) => {
            let display_msg = format!("{} ⏳", load_action_display_owned(action));
            let widget = Paragraph::new(display_msg.as_str()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(border_color)),
            );
            frame.render_widget(widget, area);
        }
        ContentState::Error(msg) => {
            let widget = Paragraph::new(msg.as_str())
                .style(Style::default().fg(Color::Red))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(title)
                        .border_style(Style::default().fg(border_color)),
                );
            frame.render_widget(widget, area);
        }
        ContentState::LikedSongs(tracks)
        | ContentState::PlaylistTracks(_, tracks)
        | ContentState::SearchResults(_, tracks) => {
            render_track_list(
                frame,
                area,
                tracks,
                selected_index,
                scroll_offset,
                title,
                border_color,
            );
        }
        ContentState::Playlists(playlists) => {
            render_playlist_list(
                frame,
                area,
                playlists,
                selected_index,
                scroll_offset,
                title,
                border_color,
            );
        }
        ContentState::DeviceSelector(entries) => {
            crate::ui::render_device_selector(frame, area, entries, selected_index);
        }
    }
}
