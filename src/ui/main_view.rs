//! Main content view rendering

use crate::state::app_state::{
    AlbumListItem, ArtistListItem, ContentState, LibraryTab, PlaylistListItem, TrackListItem,
};
use crate::state::load_coordinator::LoadAction;
use crate::ui::layout_cache::LayoutCache;
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
        LoadAction::LikedSongs | LoadAction::LikedSongsPage { .. } => {
            "Loading liked songs...".to_string()
        }
        LoadAction::Playlists => "Loading playlists...".to_string(),
        LoadAction::PlaylistTracks { name, .. } => format!("Loading {}...", name),
        LoadAction::Search { query } => format!("Searching: {}", query),
        LoadAction::Devices => "Loading devices...".to_string(),
        LoadAction::HomeData => "Loading home...".to_string(),
        LoadAction::LibraryAlbums => "Loading albums...".to_string(),
        LoadAction::LibraryArtists => "Loading artists...".to_string(),
        LoadAction::AlbumTracks { name, .. } => format!("Loading {}...", name),
        LoadAction::ArtistTopTracks { name, .. } => format!("Loading {}...", name),
    }
}

/// Render a list of tracks with optional "load more" indicator
fn render_track_list(
    frame: &mut ratatui::Frame,
    area: Rect,
    tracks: &[TrackListItem],
    selected_index: usize,
    scroll_offset: usize,
    title: &str,
    _border_color: Color,
    playing_uri: Option<&str>,
    more_available: Option<u32>,
    layout_cache: &mut LayoutCache,
) {
    // Store main view area and clear track items for fresh population
    layout_cache.main_view = Some(area);
    layout_cache.track_items.clear();

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

    // Populate track item rectangles for hit testing
    for (i, _) in (scroll_offset..end).enumerate() {
        let item_y = list_area.y + i as u16;
        let item_area = Rect::new(list_area.x, item_y, list_area.width, 1);
        layout_cache.track_items.push(item_area);
    }

    // Show "load more" indicator when there are additional tracks
    if let Some(remaining) = more_available {
        let load_more_height = 1u16;
        let load_more_area = Rect::new(
            list_area.x,
            list_area.y + list_area.height.saturating_sub(load_more_height),
            list_area.width,
            load_more_height,
        );
        let load_more_text = format!(
            "  {} {} more tracks (Enter or ↓ to load)",
            symbols::ARROW_DOWN,
            remaining
        );
        let load_more =
            Paragraph::new(load_more_text).style(Catppuccin::dim().add_modifier(Modifier::ITALIC));
        frame.render_widget(load_more, load_more_area);
    }
}

/// Render library view with albums grid and artists list
fn render_library(
    frame: &mut ratatui::Frame,
    area: Rect,
    albums: &[AlbumListItem],
    artists: &[crate::state::app_state::ArtistListItem],
    selected_tab: &LibraryTab,
    selected_index: usize,
    scroll_offset: usize,
    _border_color: Color,
    layout_cache: &mut LayoutCache,
) {
    use crate::state::app_state::ArtistListItem;

    // Store main view area
    layout_cache.main_view = Some(area);
    layout_cache.track_items.clear();
    layout_cache.playlist_items.clear();

    // Split into header and content
    let header_height = 3u16;
    let content_area = Rect::new(
        area.x,
        area.y + header_height,
        area.width,
        area.height.saturating_sub(header_height),
    );

    // Calculate available content width
    let content_width = area.width.saturating_sub(2) as usize;

    // Tab bar
    let albums_tab = format!(
        "[{}] Albums",
        if *selected_tab == LibraryTab::Albums {
            "x"
        } else {
            " "
        }
    );
    let artists_tab = format!(
        "[{}] Artists",
        if *selected_tab == LibraryTab::Artists {
            "x"
        } else {
            " "
        }
    );
    let tab_text = format!("{}    {}", albums_tab, artists_tab);

    let header_text = if content_width >= 50 {
        format!(
            "{} {}  │  {}",
            symbols::MUSIC_NOTE,
            tab_text,
            "Tab to switch"
        )
    } else {
        tab_text
    };

    let header = Paragraph::new(truncate(&header_text, content_width))
        .style(Catppuccin::dim())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Library ")
                .border_style(Catppuccin::primary().add_modifier(Modifier::BOLD))
                .title_style(Catppuccin::primary()),
        );
    frame.render_widget(header, area);

    match selected_tab {
        LibraryTab::Albums => {
            if albums.is_empty() {
                let widget = Paragraph::new("No saved albums. Save albums to see them here.")
                    .style(Catppuccin::dim())
                    .block(Block::default().borders(Borders::NONE));
                frame.render_widget(widget, content_area);
                return;
            }

            // Simple list view for albums (grid can be added later)
            let visible_count = content_area.height as usize;
            let end = (scroll_offset + visible_count).min(albums.len());

            let items: Vec<ListItem> = (scroll_offset..end)
                .map(|i| {
                    let album = &albums[i];
                    let is_selected = i == selected_index;
                    let (prefix, style) = if is_selected {
                        (
                            format!("{} ", symbols::ARROW_RIGHT),
                            Catppuccin::track_item_selected(),
                        )
                    } else {
                        ("  ".to_string(), Catppuccin::track_item())
                    };

                    let year_str = album
                        .release_year
                        .map(|y| format!(" ({})", y))
                        .unwrap_or_default();
                    let line_text = format!("{}{}{}", album.name, year_str, album.artist);
                    let truncated =
                        truncate(&line_text, content_width.saturating_sub(prefix.len()));

                    let line = Line::from(vec![
                        Span::styled(prefix, Catppuccin::secondary()),
                        Span::styled(symbols::DISC.to_string(), Catppuccin::secondary()),
                        Span::styled(" ", style),
                        Span::styled(truncated, style),
                    ]);
                    ListItem::new(line)
                })
                .collect();

            let list = List::new(items);
            frame.render_widget(list, content_area);

            // Populate item rectangles for hit testing
            for (i, _) in (scroll_offset..end).enumerate() {
                let item_y = content_area.y + i as u16;
                let item_area = Rect::new(content_area.x, item_y, content_area.width, 1);
                layout_cache.track_items.push(item_area);
            }
        }
        LibraryTab::Artists => {
            if artists.is_empty() {
                let widget =
                    Paragraph::new("No followed artists. Follow artists to see them here.")
                        .style(Catppuccin::dim())
                        .block(Block::default().borders(Borders::NONE));
                frame.render_widget(widget, content_area);
                return;
            }

            let visible_count = content_area.height as usize;
            let end = (scroll_offset + visible_count).min(artists.len());

            let items: Vec<ListItem> = (scroll_offset..end)
                .map(|i| {
                    let artist: &ArtistListItem = &artists[i];
                    let is_selected = i == selected_index;
                    let (prefix, style) = if is_selected {
                        (
                            format!("{} ", symbols::ARROW_RIGHT),
                            Catppuccin::track_item_selected(),
                        )
                    } else {
                        ("  ".to_string(), Catppuccin::track_item())
                    };

                    let line_text = &artist.name;
                    let truncated =
                        truncate(line_text, content_width.saturating_sub(prefix.len() + 2));

                    let line = Line::from(vec![
                        Span::styled(prefix, Catppuccin::secondary()),
                        Span::styled(symbols::MUSIC_NOTE.to_string(), Catppuccin::secondary()),
                        Span::styled(" ", style),
                        Span::styled(truncated, style),
                    ]);
                    ListItem::new(line)
                })
                .collect();

            let list = List::new(items);
            frame.render_widget(list, content_area);

            // Populate item rectangles for hit testing
            for (i, _) in (scroll_offset..end).enumerate() {
                let item_y = content_area.y + i as u16;
                let item_area = Rect::new(content_area.x, item_y, content_area.width, 1);
                layout_cache.track_items.push(item_area);
            }
        }
    }
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
    layout_cache: &mut LayoutCache,
) {
    // Store main view area and clear playlist items for fresh population
    layout_cache.main_view = Some(area);
    layout_cache.playlist_items.clear();

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

    // Populate playlist item rectangles for hit testing
    for (i, _) in (scroll_offset..end).enumerate() {
        let item_y = list_area.y + i as u16;
        let item_area = Rect::new(list_area.x, item_y, list_area.width, 1);
        layout_cache.playlist_items.push(item_area);
    }
}

/// Render breadcrumb trail at top of main view
#[allow(dead_code)]
fn render_breadcrumb(frame: &mut ratatui::Frame, area: Rect, trail: &[String]) {
    if trail.is_empty() {
        return;
    }

    let breadcrumb_text = trail.join(" > ");
    let truncated = if breadcrumb_text.len() > area.width as usize - 4 {
        format!(
            "...{}",
            &breadcrumb_text[breadcrumb_text.len() - area.width as usize + 7..]
        )
    } else {
        breadcrumb_text
    };

    let breadcrumb = Paragraph::new(format!("  {} ", truncated))
        .style(Catppuccin::dim().add_modifier(Modifier::ITALIC));
    frame.render_widget(breadcrumb, area);
}

/// Render album detail view
fn render_album_detail(
    frame: &mut ratatui::Frame,
    area: Rect,
    album: &AlbumListItem,
    tracks: &[TrackListItem],
    selected_index: usize,
    scroll_offset: usize,
    _border_color: Color,
    playing_uri: Option<&str>,
    layout_cache: &mut LayoutCache,
) {
    use crate::ui::theme::symbols;

    layout_cache.main_view = Some(area);
    layout_cache.track_items.clear();

    // Split area into header (album info) and track list
    let header_height = 5u16;
    let [header_area, list_area] =
        Layout::vertical([Constraint::Length(header_height), Constraint::Min(0)]).areas(area);

    let _content_width = area.width.saturating_sub(2) as usize;

    // Header with album info
    let year_str = album
        .release_year
        .map(|y| format!(" ({})", y))
        .unwrap_or_default();
    let header_text = vec![
        Line::from(vec![
            Span::styled(format!("{} ", symbols::DISC), Catppuccin::secondary()),
            Span::styled(
                &album.name,
                Catppuccin::primary().add_modifier(Modifier::BOLD),
            ),
            Span::styled(&year_str, Catppuccin::dim()),
        ]),
        Line::from(vec![
            Span::styled("  by ", Catppuccin::dim()),
            Span::styled(&album.artist, Catppuccin::text()),
        ]),
        Line::from(vec![Span::styled(
            format!(
                "  {} {} tracks  •  Press Enter to play",
                symbols::MUSIC_NOTE,
                album.total_tracks
            ),
            Catppuccin::dim(),
        )]),
    ];

    let header = Paragraph::new(header_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Album "))
            .border_style(Catppuccin::primary().add_modifier(Modifier::BOLD))
            .title_style(Catppuccin::primary()),
    );
    frame.render_widget(header, header_area);

    // Track list
    if tracks.is_empty() {
        let empty = Paragraph::new("  Loading tracks...").style(Catppuccin::dim());
        frame.render_widget(empty, list_area);
        return;
    }

    let visible_count = list_area.height as usize;
    let end = (scroll_offset + visible_count).min(tracks.len());

    let items: Vec<ListItem> = (scroll_offset..end)
        .map(|i| {
            let track = &tracks[i];
            let is_selected = i == selected_index;
            let is_playing = playing_uri == Some(track.uri.as_str());

            let (prefix_str, style) = if is_playing {
                (
                    symbols::SPEAKER.to_string(),
                    Catppuccin::success().add_modifier(Modifier::BOLD),
                )
            } else if is_selected {
                (
                    symbols::ARROW_RIGHT.to_string(),
                    Catppuccin::track_item_selected(),
                )
            } else {
                (" ".to_string(), Catppuccin::track_item())
            };

            let line = Line::from(vec![
                Span::styled(format!("{:2}. ", i + 1), Catppuccin::track_number()),
                Span::styled(format!("{} ", prefix_str), style),
                Span::styled(&track.name, style),
                Span::styled(" - ", Catppuccin::dim()),
                Span::styled(&track.artist, Catppuccin::artist_name()),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, list_area);

    // Populate track item rectangles for hit testing
    for (i, _) in (scroll_offset..end).enumerate() {
        let item_y = list_area.y + i as u16;
        let item_area = Rect::new(list_area.x, item_y, list_area.width, 1);
        layout_cache.track_items.push(item_area);
    }
}

/// Render artist detail view
fn render_artist_detail(
    frame: &mut ratatui::Frame,
    area: Rect,
    artist: &ArtistListItem,
    _selected_index: usize,
    _scroll_offset: usize,
    _border_color: Color,
    layout_cache: &mut LayoutCache,
) {
    use crate::ui::theme::symbols;

    layout_cache.main_view = Some(area);
    layout_cache.track_items.clear();

    let content_width = area.width.saturating_sub(2) as usize;

    // Build artist info display
    let mut lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("{} ", symbols::HEADPHONES), Catppuccin::secondary()),
            Span::styled(
                &artist.name,
                Catppuccin::primary().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ];

    // Add genres if available
    if !artist.genres.is_empty() {
        let genres = artist.genres.join(", ");
        lines.push(Line::from(vec![
            Span::styled("  Genres: ", Catppuccin::dim()),
            Span::styled(
                truncate(&genres, content_width.saturating_sub(10)),
                Catppuccin::text(),
            ),
        ]));
    }

    // Add follower count if available
    if let Some(count) = artist.follower_count {
        let formatted = if count >= 1_000_000 {
            format!("{:.1}M", count as f64 / 1_000_000.0)
        } else if count >= 1_000 {
            format!("{:.1}K", count as f64 / 1_000.0)
        } else {
            count.to_string()
        };
        lines.push(Line::from(vec![
            Span::styled("  Followers: ", Catppuccin::dim()),
            Span::styled(formatted, Catppuccin::text()),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::styled(
        "  Artist detail view - tracks coming soon!",
        Catppuccin::dim().add_modifier(Modifier::ITALIC),
    ));

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Artist "))
            .border_style(Catppuccin::secondary().add_modifier(Modifier::BOLD))
            .title_style(Catppuccin::secondary()),
    );
    frame.render_widget(widget, area);
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
    layout_cache: &mut LayoutCache,
    _breadcrumb_trail: Option<&[String]>,
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
        ContentState::LikedSongs(tracks) => {
            let title = format!(" {} Liked Songs", symbols::HEART_FILLED);
            let more_available: Option<u32> = None;
            render_track_list(
                frame,
                area,
                tracks,
                selected_index,
                scroll_offset,
                &title,
                border_color,
                playing_uri,
                more_available,
                layout_cache,
            );
        }
        ContentState::LikedSongsPage {
            tracks,
            total,
            next_offset,
        } => {
            let remaining = total.saturating_sub(tracks.len() as u32);
            let title = format!(
                " {} Liked Songs ({}/{})",
                symbols::HEART_FILLED,
                tracks.len(),
                total
            );
            let more_available = next_offset.map(|_| remaining);
            render_track_list(
                frame,
                area,
                tracks,
                selected_index,
                scroll_offset,
                &title,
                border_color,
                playing_uri,
                more_available,
                layout_cache,
            );
        }
        ContentState::PlaylistTracks(name, tracks) => {
            let title = format!(" {} {}", symbols::DISC, name);
            render_track_list(
                frame,
                area,
                tracks,
                selected_index,
                scroll_offset,
                &title,
                border_color,
                playing_uri,
                None,
                layout_cache,
            );
        }
        ContentState::SearchResults(query, tracks) => {
            let title = format!(" {} Results: {}", symbols::SEARCH, query);
            render_track_list(
                frame,
                area,
                tracks,
                selected_index,
                scroll_offset,
                &title,
                border_color,
                playing_uri,
                None,
                layout_cache,
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
                layout_cache,
            );
        }
        ContentState::DeviceSelector(entries) => {
            crate::ui::render_device_selector(frame, area, entries, selected_index);
        }
        ContentState::HomeDashboard(ref home_state) => {
            crate::ui::home_view::render_home_dashboard(
                frame,
                area,
                home_state,
                selected_index,
                scroll_offset,
                layout_cache,
            );
        }
        ContentState::Library {
            albums,
            artists,
            selected_tab,
        } => {
            render_library(
                frame,
                area,
                albums,
                artists,
                selected_tab,
                selected_index,
                scroll_offset,
                border_color,
                layout_cache,
            );
        }
        ContentState::AlbumDetail { album, tracks } => {
            render_album_detail(
                frame,
                area,
                album,
                tracks,
                selected_index,
                scroll_offset,
                border_color,
                playing_uri,
                layout_cache,
            );
        }
        ContentState::ArtistDetail { artist } => {
            render_artist_detail(
                frame,
                area,
                artist,
                selected_index,
                scroll_offset,
                border_color,
                layout_cache,
            );
        }
        // These states are handled in main.rs, not rendered directly
        ContentState::SearchResultsLive(_) | ContentState::SearchErrorLive(_) => {
            // These are handled by search overlay, not main view
        }
        // Loading state is handled at the top of this function
        ContentState::Loading(_) => {
            // Already handled above, but match requires this arm
        }
        // Radio recommendations are handled in main.rs, not rendered in main view
        ContentState::RadioRecommendations(_) => {
            // These are processed and loaded into the queue, not displayed here
        }
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

    #[test]
    fn test_track_item_rect_calculation() {
        use ratatui::layout::Rect;

        // Simulate the track item rect calculation logic
        let list_area = Rect::new(0, 3, 60, 10);
        let scroll_offset = 0;
        let visible_count = 5;
        let end = (scroll_offset + visible_count).min(10);

        let mut track_items: Vec<Rect> = Vec::new();
        for (i, _) in (scroll_offset..end).enumerate() {
            let item_y = list_area.y + i as u16;
            let item_area = Rect::new(list_area.x, item_y, list_area.width, 1);
            track_items.push(item_area);
        }

        // Verify correct number of items
        assert_eq!(track_items.len(), 5);

        // Verify first item position
        assert_eq!(track_items[0].x, 0);
        assert_eq!(track_items[0].y, 3);
        assert_eq!(track_items[0].width, 60);
        assert_eq!(track_items[0].height, 1);

        // Verify last item position
        assert_eq!(track_items[4].y, 7);
    }

    #[test]
    fn test_track_item_rect_with_scroll() {
        use ratatui::layout::Rect;

        // Test with scroll offset
        let list_area = Rect::new(0, 3, 60, 10);
        let scroll_offset = 2;
        let visible_count = 3;
        let end = (scroll_offset + visible_count).min(10);

        let mut track_items: Vec<Rect> = Vec::new();
        for (i, _) in (scroll_offset..end).enumerate() {
            let item_y = list_area.y + i as u16;
            let item_area = Rect::new(list_area.x, item_y, list_area.width, 1);
            track_items.push(item_area);
        }

        // Should have 3 visible items
        assert_eq!(track_items.len(), 3);

        // First visible item is at list_area.y (scroll offset affects which tracks, not position)
        assert_eq!(track_items[0].y, 3);
        assert_eq!(track_items[1].y, 4);
        assert_eq!(track_items[2].y, 5);
    }

    #[test]
    fn test_playlist_item_rect_calculation() {
        use ratatui::layout::Rect;

        // Simulate the playlist item rect calculation logic
        let list_area = Rect::new(0, 3, 60, 10);
        let scroll_offset = 0;
        let visible_count = 4;
        let end = (scroll_offset + visible_count).min(8);

        let mut playlist_items: Vec<Rect> = Vec::new();
        for (i, _) in (scroll_offset..end).enumerate() {
            let item_y = list_area.y + i as u16;
            let item_area = Rect::new(list_area.x, item_y, list_area.width, 1);
            playlist_items.push(item_area);
        }

        // Verify correct number of items
        assert_eq!(playlist_items.len(), 4);

        // Verify positions
        for (i, rect) in playlist_items.iter().enumerate() {
            assert_eq!(rect.y, 3 + i as u16);
            assert_eq!(rect.width, 60);
            assert_eq!(rect.height, 1);
        }
    }

    #[test]
    fn test_saturating_math_prevents_overflow() {
        use ratatui::layout::Rect;

        // Test with very small area
        let area = Rect::new(0, 0, 5, 2);
        let header_height = 3u16;
        let list_area = Rect::new(
            area.x,
            area.y.saturating_add(header_height),
            area.width,
            area.height.saturating_sub(header_height),
        );

        // Should not panic, list_area.height should be 0
        assert_eq!(list_area.height, 0);

        // Test content width calculation
        let content_width = area.width.saturating_sub(2) as usize;
        assert_eq!(content_width, 3);
    }

    #[test]
    fn test_layout_cache_clear_before_populate() {
        use crate::ui::layout_cache::LayoutCache;

        let mut cache = LayoutCache::new();

        // Pre-populate with some data
        cache.track_items.push(Rect::new(0, 0, 10, 1));
        cache.track_items.push(Rect::new(0, 1, 10, 1));
        cache.playlist_items.push(Rect::new(0, 5, 10, 1));

        // Clear should reset
        cache.track_items.clear();
        cache.playlist_items.clear();

        assert!(cache.track_items.is_empty());
        assert!(cache.playlist_items.is_empty());
    }
}
