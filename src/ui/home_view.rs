//! Home dashboard view rendering
//!
//! Displays:
//! - Jump Back In section (unfinished contexts)
//! - Recently Played section (last 20 tracks)
//! - Quick Access buttons (Liked Songs, Playlists, Library)
//! - Empty states for new users

use crate::state::home_state::{format_relative_time, ContinueContext, HomeState, RecentlyPlayedItem};
use crate::ui::layout_cache::LayoutCache;
use crate::ui::theme::{symbols, Catppuccin};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

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

/// Render the home dashboard
pub fn render_home_dashboard(
    frame: &mut ratatui::Frame,
    area: Rect,
    home_state: &HomeState,
    _selected_index: usize,
    _scroll_offset: usize,
    _layout_cache: &mut LayoutCache,
) {
    // Store main view area
    _layout_cache.main_view = Some(area);
    _layout_cache.track_items.clear();

    // Calculate available content width
    let content_width = area.width.saturating_sub(4) as usize; // 2 borders + 2 padding

    // Check if we have data
    if home_state.is_loading && home_state.recently_played.is_empty() {
        render_loading_state(frame, area);
        return;
    }

    // Check if user is new (no data)
    if home_state.recently_played.is_empty() && home_state.jump_back_in.is_empty() {
        render_empty_state(frame, area, content_width);
        return;
    }

    // Split area into sections
    let sections = Layout::vertical([
        Constraint::Length(3), // Title bar
        Constraint::Min(8),    // Jump Back In
        Constraint::Min(12),   // Recently Played
        Constraint::Length(5), // Quick Access
    ])
    .split(area);

    // Render title
    render_title_bar(frame, sections[0], content_width);

    // Render Jump Back In section (if we have items)
    if !home_state.jump_back_in.is_empty() {
        render_jump_back_in_section(frame, sections[1], &home_state.jump_back_in, content_width);
    }

    // Render Recently Played section
    render_recently_played_section(
        frame,
        sections[2],
        &home_state.recently_played,
        _selected_index,
        content_width,
    );

    // Render Quick Access buttons
    render_quick_access_section(frame, sections[3], content_width);
}

/// Render title bar for Home dashboard
fn render_title_bar(frame: &mut ratatui::Frame, area: Rect, _content_width: usize) {
    let title = Paragraph::new(format!("  {} Home", symbols::HOME))
        .style(Catppuccin::primary().add_modifier(Modifier::BOLD))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Catppuccin::border()),
        );
    frame.render_widget(title, area);
}

/// Render "Jump Back In" section with horizontal cards
fn render_jump_back_in_section(
    frame: &mut ratatui::Frame,
    area: Rect,
    items: &[ContinueContext],
    _content_width: usize,
) {
    // Header
    let header_height = 1u16;
    let content_area = Rect::new(
        area.x,
        area.y + header_height,
        area.width,
        area.height.saturating_sub(header_height),
    );

    // Split into header and content
    let header = Paragraph::new(format!("  {} Jump Back In", symbols::ARROW_RIGHT))
        .style(Catppuccin::secondary().add_modifier(Modifier::BOLD));
    frame.render_widget(header, Rect::new(area.x, area.y, area.width, header_height));

    // Calculate card width (aim for 4 cards, minimum 15 chars each)
    let card_width = (area.width.saturating_sub(4) / 4).max(15);
    let visible_cards = (area.width.saturating_sub(4) / card_width).max(1) as usize;

    // Create horizontal layout for cards
    let constraints: Vec<Constraint> = (0..visible_cards)
        .map(|_| Constraint::Length(card_width))
        .collect();
    let card_areas = Layout::horizontal(constraints).split(content_area);

    // Render cards
    for (i, item) in items.iter().take(visible_cards).enumerate() {
        if i < card_areas.len() {
            render_jump_back_in_card(frame, card_areas[i], item);
        }
    }
}

/// Render a single "Jump Back In" card
fn render_jump_back_in_card(frame: &mut ratatui::Frame, area: Rect, item: &ContinueContext) {
    // Card border
    let border_style = Catppuccin::border();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Card content
    let icon = match item.context_type {
        crate::state::home_state::ContextType::Album => symbols::DISC,
        crate::state::home_state::ContextType::Playlist => symbols::MUSIC,
        crate::state::home_state::ContextType::Artist => symbols::HEADPHONES,
    };

    let content_width = inner.width.saturating_sub(2) as usize;
    let name = truncate(&item.name, content_width);
    let progress = item.format_progress();

    let lines = vec![
        Line::from(vec![
            Span::styled(format!(" {} ", icon), Catppuccin::success()),
        ]),
        Line::from(vec![
            Span::styled(name, Catppuccin::text()),
        ]),
        Line::from(vec![
            Span::styled(format!("  {}%", progress), Catppuccin::dim()),
        ]),
    ];

    let content = Paragraph::new(lines);
    frame.render_widget(content, inner);
}

/// Render "Recently Played" section
fn render_recently_played_section(
    frame: &mut ratatui::Frame,
    area: Rect,
    items: &[RecentlyPlayedItem],
    _selected_index: usize,
    content_width: usize,
) {
    // Header
    let header_height = 1u16;
    let list_area = Rect::new(
        area.x,
        area.y + header_height,
        area.width,
        area.height.saturating_sub(header_height),
    );

    let header = Paragraph::new(format!("  {} Recently Played", symbols::ARROW_RIGHT))
        .style(Catppuccin::secondary().add_modifier(Modifier::BOLD));
    frame.render_widget(header, Rect::new(area.x, area.y, area.width, header_height));

    // Render track list
    let visible_count = list_area.height as usize;
    let end = visible_count.min(items.len());

    let mut lines: Vec<Line> = Vec::with_capacity(end);
    for (i, item) in items.iter().take(end).enumerate() {
        let time_ago = format_relative_time(item.played_at);
        let track_name = truncate(&item.track.name, content_width.saturating_sub(25)); // Reserve space for time
        let artist_name = truncate(&item.track.artist, content_width.saturating_sub(25).saturating_sub(track_name.len() + 3));

        let is_selected = i == _selected_index;
        let style = if is_selected {
            Catppuccin::track_item_selected()
        } else {
            Catppuccin::text()
        };

        let context_icon = item.context.as_ref().map(|ctx| {
            match ctx.context_type {
                crate::state::home_state::ContextType::Album => symbols::DISC,
                crate::state::home_state::ContextType::Playlist => symbols::MUSIC,
                crate::state::home_state::ContextType::Artist => symbols::HEADPHONES,
            }
        }).unwrap_or(symbols::MUSIC_NOTE);

        lines.push(Line::from(vec![
            Span::styled(format!("   {} ", context_icon), Catppuccin::dim()),
            Span::styled(format!("{} - {}", track_name, artist_name), style),
            Span::styled(format!("  {}", time_ago), Catppuccin::dim()),
        ]));
    }

    // Show "more" indicator if there are more tracks
    if items.len() > end {
        lines.push(Line::from(vec![
            Span::styled(format!("   ... and {} more", items.len() - end), Catppuccin::dim()),
        ]));
    }

    let list = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(list, list_area);
}

/// Render Quick Access section with buttons
fn render_quick_access_section(frame: &mut ratatui::Frame, area: Rect, content_width: usize) {
    // Header
    let header_height = 1u16;
    let content_area = Rect::new(
        area.x,
        area.y + header_height,
        area.width,
        area.height.saturating_sub(header_height),
    );

    let header = Paragraph::new(format!("  {} Quick Access", symbols::ARROW_RIGHT))
        .style(Catppuccin::secondary().add_modifier(Modifier::BOLD));
    frame.render_widget(header, Rect::new(area.x, area.y, area.width, header_height));

    // Quick access items
    let items = vec![
        (symbols::HEART_FILLED, "Liked Songs", 'L'),
        (symbols::MUSIC, "Playlists", 'P'),
        (symbols::DISC, "Albums", 'A'),
        (symbols::HEADPHONES, "Artists", 'R'),
    ];

    // Layout items horizontally
    let item_width = (content_width / items.len()).max(15) as u16;
    let constraints: Vec<Constraint> = items
        .iter()
        .map(|_| Constraint::Length(item_width))
        .collect();
    let item_areas = Layout::horizontal(constraints).split(content_area);

    // Render each quick access button
    for (i, (icon, label, key)) in items.iter().enumerate() {
        if i < item_areas.len() {
            let button = Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(format!(" {} ", icon), Catppuccin::success()),
                    Span::styled(*label, Catppuccin::text()),
                ]),
                Line::from(vec![
                    Span::styled(format!("   Press '{}'", key), Catppuccin::dim()),
                ]),
            ])
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Catppuccin::border()),
            );
            frame.render_widget(button, item_areas[i]);
        }
    }
}

/// Render loading state
fn render_loading_state(frame: &mut ratatui::Frame, area: Rect) {
    let spinner = crate::ui::theme::spinner_frame();
    let content = Paragraph::new(format!(
        "\n\n  {}  Loading your music...",
        spinner
    ))
    .style(Catppuccin::loading())
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Catppuccin::border()),
    );
    frame.render_widget(content, area);
}

/// Render empty state for new users
fn render_empty_state(frame: &mut ratatui::Frame, area: Rect, _content_width: usize) {
    let content = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {} Welcome to Joshify!", symbols::HOME), Catppuccin::primary().add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::styled("  Start listening to see your recently played tracks here.", Catppuccin::text()),
        Line::from(""),
        Line::styled(format!("  {} Press '/' to search for music", symbols::SEARCH), Catppuccin::info()),
        Line::styled(format!("  {} Press 'l' to view your Liked Songs", symbols::HEART_FILLED), Catppuccin::success()),
        Line::styled(format!("  {} Press 'p' to browse your Playlists", symbols::MUSIC), Catppuccin::secondary()),
    ];

    let widget = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Home ")
                .border_style(Catppuccin::border()),
        );
    frame.render_widget(widget, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_track(name: &str, artist: &str) -> crate::state::home_state::TrackSummary {
        crate::state::home_state::TrackSummary {
            name: name.to_string(),
            artist: artist.to_string(),
            uri: format!("spotify:track:{}", name.replace(' ', "_")),
            duration_ms: 180000,
        }
    }

    fn create_test_home_state() -> HomeState {
        HomeState {
            recently_played: vec![
                RecentlyPlayedItem {
                    track: create_test_track("Test Track 1", "Artist 1"),
                    played_at: Utc::now(),
                    context: None,
                },
                RecentlyPlayedItem {
                    track: create_test_track("Test Track 2", "Artist 2"),
                    played_at: Utc::now(),
                    context: Some(crate::state::home_state::PlayContext {
                        context_type: crate::state::home_state::ContextType::Album,
                        id: "album1".to_string(),
                        name: "Test Album".to_string(),
                    }),
                },
            ],
            jump_back_in: vec![
                ContinueContext {
                    context_type: crate::state::home_state::ContextType::Playlist,
                    id: "playlist1".to_string(),
                    name: "Test Playlist".to_string(),
                    progress_percent: 45,
                    last_played: Utc::now(),
                    total_tracks: 20,
                    completed_tracks: 9,
                },
            ],
            is_loading: false,
            last_updated: None,
        }
    }

    #[test]
    fn test_truncate_short_text() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_long_text() {
        let text = "This is a very long text that needs truncation";
        let result = truncate(text, 20);
        assert!(result.ends_with('…'));
        // Unicode width may differ from byte length
        assert!(unicode_width::UnicodeWidthStr::width(result.as_str()) <= 20);
    }

    #[test]
    fn test_truncate_exact_width() {
        let text = "exactlytwentychars!!";
        let result = truncate(text, 20);
        assert_eq!(result, text);
    }
}
