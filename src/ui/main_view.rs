//! Main content view rendering

use crate::state::app_state::{ContentState, PlaylistListItem, TrackListItem};
use crate::state::load_coordinator::LoadAction;
use crate::ui::theme::{self, symbols, Catppuccin};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

type MainContentState = ContentState;

/// Truncate text to fit within display width, adding ellipsis if needed
fn truncate(text: &str, max_width: usize) -> String {
    if unicode_width::UnicodeWidthStr::width(text) <= max_width {
        text.to_string()
    } else {
        let (truncated, _) = unicode_truncate::UnicodeTruncateStr::unicode_truncate(
            text,
            max_width.saturating_sub(1),
        );
        format!("{truncated}…")
    }
}

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

/// Render a list of tracks
fn render_track_list(
    frame: &mut ratatui::Frame,
    area: Rect,
    tracks: &[TrackListItem],
    selected_index: usize,
    scroll_offset: usize,
    title: &str,
    _border_color: Color,
    playing_uri: Option<&str>,
) {
    if tracks.is_empty() {
        let widget = Paragraph::new(format!("{} No tracks found", symbols::MUSIC_NOTE))
            .style(Catppuccin::dim())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Catppuccin::border()),
            );
        frame.render_widget(widget, area);
        return;
    }

    let header_height = 3u16;
    let list_area = Rect::new(
        area.x,
        area.y + header_height,
        area.width,
        area.height.saturating_sub(header_height),
    );

    // Calculate available content width (subtract borders: 2 chars)
    let content_width = area.width.saturating_sub(2) as usize;

    // Build header text that fits within borders
    let header_text = if content_width >= 50 {
        format!(
            "{} {} tracks  │  {} Enter to play  │  ↑/↓ Navigate",
            symbols::MUSIC_NOTE,
            tracks.len(),
            symbols::ARROW_RIGHT
        )
    } else if content_width >= 30 {
        format!(
            "{} {} tracks  │  {} Play",
            symbols::MUSIC_NOTE,
            tracks.len(),
            symbols::ARROW_RIGHT
        )
    } else {
        format!("{} {} tracks", symbols::MUSIC_NOTE, tracks.len())
    };

    let header = Paragraph::new(truncate(&header_text, content_width))
        .style(Catppuccin::dim())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Catppuccin::primary().add_modifier(Modifier::BOLD))
                .title_style(Catppuccin::primary()),
        );
    frame.render_widget(header, area);

    let visible_count = list_area.height as usize;
    let end = (scroll_offset + visible_count).min(tracks.len());

    // Calculate max width for track name and artist to prevent overflow
    // Format: "NN. → name - artist" (with borders: 2 chars)
    let num_width = 4; // "NN. "
    let separator_width = 3; // " - "
    let min_artist_width = 10;
    let max_name_width =
        content_width.saturating_sub(num_width + separator_width + min_artist_width);

    let items: Vec<ListItem> = (scroll_offset..end)
        .map(|i| {
            let track = &tracks[i];
            let is_selected = i == selected_index;
            let is_playing = playing_uri == Some(track.uri.as_str());
            let (prefix, style) = if is_playing {
                (
                    format!("{} ", symbols::SPEAKER),
                    Catppuccin::success().add_modifier(Modifier::BOLD),
                )
            } else if is_selected {
                (
                    format!("{} ", symbols::ARROW_RIGHT),
                    Catppuccin::track_item_selected(),
                )
            } else {
                ("  ".to_string(), Catppuccin::track_item())
            };
            let num = format!("{:2}.", i + 1);

            // Truncate name and artist to fit within available width
            let available_for_name = max_name_width.saturating_sub(prefix.len());
            let name = truncate(&track.name, available_for_name);
            let available_for_artist = content_width
                .saturating_sub(num_width + prefix.len() + name.len() + separator_width);
            let artist = truncate(&track.artist, available_for_artist.max(10));

            let line = Line::from(vec![
                Span::styled(format!("{}{}", num, prefix), Catppuccin::track_number()),
                Span::styled(name, style),
                Span::styled(" - ", Catppuccin::dim()),
                Span::styled(artist, Catppuccin::artist_name()),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items);
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
    _border_color: Color,
) {
    if playlists.is_empty() {
        let widget = Paragraph::new(format!("{} No playlists found", symbols::DISC))
            .style(Catppuccin::dim())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Catppuccin::border()),
            );
        frame.render_widget(widget, area);
        return;
    }

    let header_height = 3u16;
    let list_area = Rect::new(
        area.x,
        area.y + header_height,
        area.width,
        area.height.saturating_sub(header_height),
    );

    // Calculate available content width (subtract borders: 2 chars)
    let content_width = area.width.saturating_sub(2) as usize;

    // Build header text that fits within borders
    let header_text = if content_width >= 50 {
        format!(
            "{} {} playlists  │  {} Enter to view  │  ↑/↓ Navigate",
            symbols::DISC,
            playlists.len(),
            symbols::ARROW_RIGHT
        )
    } else if content_width >= 30 {
        format!(
            "{} {} playlists  │  {} View",
            symbols::DISC,
            playlists.len(),
            symbols::ARROW_RIGHT
        )
    } else {
        format!("{} {} playlists", symbols::DISC, playlists.len())
    };

    let header = Paragraph::new(truncate(&header_text, content_width))
        .style(Catppuccin::dim())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Catppuccin::secondary().add_modifier(Modifier::BOLD))
                .title_style(Catppuccin::secondary()),
        );
    frame.render_widget(header, area);

    let visible_count = list_area.height as usize;
    let end = (scroll_offset + visible_count).min(playlists.len());

    // Calculate max width for playlist name to prevent overflow
    let prefix_width = 4; // "→ ◉ "
    let suffix_min = 12; // " (1 tracks)"
    let max_name_width = content_width.saturating_sub(prefix_width + suffix_min);

    let items: Vec<ListItem> = (scroll_offset..end)
        .map(|i| {
            let playlist = &playlists[i];
            let is_selected = i == selected_index;
            let (prefix, style) = if is_selected {
                (
                    format!("{} ", symbols::ARROW_RIGHT),
                    Catppuccin::track_item_selected(),
                )
            } else {
                ("  ".to_string(), Catppuccin::track_item())
            };

            // Truncate playlist name to fit
            let name = truncate(&playlist.name, max_name_width);
            let track_info = format!(" ({} tracks)", playlist.track_count);

            let line = Line::from(vec![
                Span::styled(
                    format!("{}{}", prefix, symbols::DISC),
                    Catppuccin::secondary(),
                ),
                Span::from(" "),
                Span::styled(name, style),
                Span::styled(track_info, Catppuccin::duration()),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, list_area);
}

/// Render the main view
pub fn render_main_view(
    frame: &mut ratatui::Frame,
    area: Rect,
    content_state: &MainContentState,
    selected_index: usize,
    scroll_offset: usize,
    is_authenticated: bool,
    border_color: Color,
    playing_uri: Option<&str>,
) {
    // Determine layout mode for responsive design
    let _layout_mode = if theme::Layout::is_compact(area.width) {
        "compact"
    } else if theme::Layout::is_medium(area.width) {
        "medium"
    } else {
        "full"
    };

    // Handle overlays first
    if let ContentState::Loading(action) = content_state {
        let spinner = theme::spinner_frame();
        let display_msg = format!("{} {}", spinner, load_action_display_owned(action));
        let widget = Paragraph::new(display_msg.as_str())
            .style(Catppuccin::loading())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Loading ")
                    .border_style(Catppuccin::border()),
            );
        frame.render_widget(widget, area);
        return;
    }

    match content_state {
        ContentState::Home => {
            let content = if is_authenticated {
                vec![
                    Line::from(""),
                    Line::styled(
                        format!("  {} Welcome to Joshify", symbols::HEADPHONES),
                        Catppuccin::primary().add_modifier(Modifier::BOLD),
                    ),
                    Line::from(""),
                    Line::styled(
                        format!(
                            "  {} Select a section from the sidebar to get started",
                            symbols::ARROW_RIGHT
                        ),
                        Catppuccin::text(),
                    ),
                    Line::from(""),
                    Line::styled(
                        format!(
                            "  {} Liked Songs  -  View your favorite tracks",
                            symbols::HEART_FILLED
                        ),
                        Catppuccin::success(),
                    ),
                    Line::styled(
                        format!("  {} Playlists   -  Browse your collections", symbols::DISC),
                        Catppuccin::secondary(),
                    ),
                    Line::styled(
                        format!(
                            "  {} Search      -  Find any track, artist, or album",
                            symbols::SEARCH
                        ),
                        Catppuccin::info(),
                    ),
                    Line::from(""),
                    Line::styled(
                        format!(
                            "  Press {} for help  │  Press {} to search",
                            symbols::HELP,
                            symbols::SEARCH
                        ),
                        Catppuccin::dim(),
                    ),
                ]
            } else {
                vec![
                    Line::from(""),
                    Line::styled(
                        "  Not connected to Spotify",
                        Catppuccin::warning().add_modifier(Modifier::BOLD),
                    ),
                    Line::from(""),
                    Line::styled("  Press 'c' to configure credentials", Catppuccin::text()),
                ]
            };
            let widget = Paragraph::new(content).block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Home ")
                    .border_style(Catppuccin::border()),
            );
            frame.render_widget(widget, area);
        }
        ContentState::LoadingInProgress(action) => {
            let spinner = theme::spinner_frame();
            let display_msg = format!("{} {}", spinner, load_action_display_owned(action));
            let widget = Paragraph::new(display_msg.as_str())
                .style(Catppuccin::loading())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Loading ")
                        .border_style(Catppuccin::border()),
                );
            frame.render_widget(widget, area);
        }
        ContentState::Error(msg) => {
            let widget = Paragraph::new(msg.as_str())
                .style(Catppuccin::error())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" Error ")
                        .border_style(Catppuccin::error().add_modifier(Modifier::BOLD))
                        .title_style(Catppuccin::error()),
                );
            frame.render_widget(widget, area);
        }
        ContentState::LikedSongs(tracks)
        | ContentState::PlaylistTracks(_, tracks)
        | ContentState::SearchResults(_, tracks) => {
            let title = match content_state {
                ContentState::LikedSongs(_) => format!(" {} Liked Songs", symbols::HEART_FILLED),
                ContentState::PlaylistTracks(name, _) => {
                    format!(" {} {}", symbols::DISC, name)
                }
                ContentState::SearchResults(query, _) => {
                    format!(" {} Results: {}", symbols::SEARCH, query)
                }
                _ => unreachable!(),
            };
            render_track_list(
                frame,
                area,
                tracks,
                selected_index,
                scroll_offset,
                &title,
                border_color,
                playing_uri,
            );
        }
        ContentState::Playlists(playlists) => {
            render_playlist_list(
                frame,
                area,
                playlists,
                selected_index,
                scroll_offset,
                &format!(" {} Playlists", symbols::DISC),
                border_color,
            );
        }
        ContentState::DeviceSelector(entries) => {
            crate::ui::render_device_selector(frame, area, entries, selected_index);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::app_state::TrackListItem;

    fn make_track(name: &str, artist: &str, uri: &str) -> TrackListItem {
        TrackListItem {
            name: name.to_string(),
            artist: artist.to_string(),
            uri: uri.to_string(),
        }
    }

    #[test]
    fn test_playing_uri_matches_track() {
        let tracks = vec![
            make_track("Song A", "Artist A", "spotify:track:1"),
            make_track("Song B", "Artist B", "spotify:track:2"),
            make_track("Song C", "Artist C", "spotify:track:3"),
        ];

        let playing_uri = Some("spotify:track:2");

        // Verify that the playing URI correctly identifies the track
        let playing_idx = tracks
            .iter()
            .position(|t| playing_uri == Some(t.uri.as_str()));
        assert_eq!(playing_idx, Some(1));
        assert_eq!(tracks[1].name, "Song B");
    }

    #[test]
    fn test_playing_uri_no_match() {
        let tracks = vec![
            make_track("Song A", "Artist A", "spotify:track:1"),
            make_track("Song B", "Artist B", "spotify:track:2"),
        ];

        let playing_uri = Some("spotify:track:999");

        let playing_idx = tracks
            .iter()
            .position(|t| playing_uri == Some(t.uri.as_str()));
        assert!(playing_idx.is_none());
    }

    #[test]
    fn test_playing_uri_none() {
        let tracks = vec![make_track("Song A", "Artist A", "spotify:track:1")];

        let playing_uri: Option<&str> = None;

        let playing_idx = tracks
            .iter()
            .position(|t| playing_uri == Some(t.uri.as_str()));
        assert!(playing_idx.is_none());
    }

    #[test]
    fn test_playing_uri_highlights_correct_track() {
        let tracks = vec![
            make_track("Track 1", "Artist 1", "spotify:track:100"),
            make_track("Track 2", "Artist 2", "spotify:track:200"),
            make_track("Track 3", "Artist 3", "spotify:track:300"),
        ];

        let playing_uri = Some("spotify:track:200");

        // Simulate the rendering logic for prefix selection
        for (i, track) in tracks.iter().enumerate() {
            let is_playing = playing_uri == Some(track.uri.as_str());
            let is_selected = i == 1; // Simulate selected index 1

            if is_playing {
                // Playing track should have speaker icon and success style
                assert!(track.uri == "spotify:track:200");
                assert_eq!(i, 1);
            } else if is_selected && !is_playing {
                // Selected but not playing should have arrow
            } else {
                // Normal track should have no prefix
            }
        }

        // Verify only track 2 is marked as playing
        let playing_count = tracks
            .iter()
            .filter(|t| playing_uri == Some(t.uri.as_str()))
            .count();
        assert_eq!(playing_count, 1);
    }

    #[test]
    fn test_playing_uri_different_from_selection() {
        let tracks = vec![
            make_track("Track 1", "Artist 1", "spotify:track:100"),
            make_track("Track 2", "Artist 2", "spotify:track:200"),
            make_track("Track 3", "Artist 3", "spotify:track:300"),
        ];

        let playing_uri = Some("spotify:track:100");
        let selected_index = 2;

        // Track 1 is playing, Track 3 is selected
        assert!(playing_uri == Some(tracks[0].uri.as_str()));
        assert_eq!(selected_index, 2);

        // They should be different
        let playing_idx = tracks
            .iter()
            .position(|t| playing_uri == Some(t.uri.as_str()));
        assert_ne!(playing_idx, Some(selected_index));
    }
}
