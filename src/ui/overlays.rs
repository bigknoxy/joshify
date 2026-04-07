//! Overlay rendering - search input, help, queue

use crate::state::queue_state::QueueState;
use crate::state::search_state::SearchState;
use crate::ui::theme::Catppuccin;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph},
};

/// Render queue overlay with actual queue data
pub fn render_queue_overlay(frame: &mut ratatui::Frame, area: Rect, queue_state: &QueueState) {
    // Create centered overlay area
    let overlay_width = (area.width as f32 * 0.7).clamp(40.0, area.width as f32) as u16;
    let overlay_height = (area.height as f32 * 0.7).clamp(15.0, area.height as f32) as u16;
    let overlay_x = (area.width - overlay_width) / 2;
    let overlay_y = (area.height - overlay_height) / 2;
    let overlay_area = Rect::new(overlay_x, overlay_y, overlay_width, overlay_height);

    // CRITICAL: Clear first to prevent any bleed-through
    frame.render_widget(Clear, overlay_area);

    // Then render solid background block
    let bg = Block::default()
        .style(Style::default().bg(Catppuccin::CRUST).fg(Catppuccin::TEXT))
        .borders(Borders::ALL)
        .border_style(Catppuccin::secondary().add_modifier(Modifier::BOLD))
        .title(" Queue ")
        .title_style(Catppuccin::focused());
    frame.render_widget(bg, overlay_area);

    // Build queue content
    let _content_area = Rect::new(
        overlay_area.x + 1,
        overlay_area.y + 1,
        overlay_area.width - 2,
        overlay_area.height - 4,
    );

    let mut lines: Vec<Line> = Vec::new();

    // Currently playing
    lines.push(Line::styled(
        "=== Now Playing ===",
        Catppuccin::primary().add_modifier(Modifier::BOLD),
    ));
    lines.push(Line::from(""));

    // Local queue items
    if !queue_state.local_queue.is_empty() {
        lines.push(Line::styled(
            "=== Up Next (Local) ===",
            Catppuccin::secondary().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(""));

        for (i, entry) in queue_state.local_queue.iter().enumerate() {
            let marker = if entry.is_recommendation {
                "🎵"
            } else if entry.added_by_user {
                "👤"
            } else {
                "•"
            };
            let text = format!("{} {}. {} - {}", marker, i + 1, entry.name, entry.artist);
            lines.push(Line::styled(text, Catppuccin::text()));
        }
        lines.push(Line::from(""));
    }

    // Spotify queue items
    if let Some(ref spotify_queue) = queue_state.spotify_queue {
        let has_currently_playing = spotify_queue.currently_playing.is_some();
        let has_queued_tracks = !spotify_queue.queue.is_empty();

        if has_currently_playing || has_queued_tracks {
            lines.push(Line::styled(
                "=== Spotify Queue ===",
                Catppuccin::secondary().add_modifier(Modifier::BOLD),
            ));
            lines.push(Line::from(""));

            if let Some(ref item) = spotify_queue.currently_playing {
                let (name, artist) = match item {
                    rspotify::model::PlayableItem::Track(track) => (
                        track.name.clone(),
                        track
                            .artists
                            .first()
                            .map(|a| a.name.clone())
                            .unwrap_or_else(|| "Unknown Artist".to_string()),
                    ),
                    rspotify::model::PlayableItem::Episode(episode) =>
                    {
                        #[allow(deprecated)]
                        (episode.name.clone(), episode.show.publisher.clone())
                    }
                    _ => ("Unknown".to_string(), "Unknown Artist".to_string()),
                };
                lines.push(Line::styled(
                    format!("▶ Now: {} - {}", name, artist),
                    Catppuccin::success().add_modifier(Modifier::BOLD),
                ));
                lines.push(Line::from(""));
            }

            for (i, item) in spotify_queue.queue.iter().take(5).enumerate() {
                let (name, artist) = match item {
                    rspotify::model::PlayableItem::Track(track) => (
                        track.name.clone(),
                        track
                            .artists
                            .first()
                            .map(|a| a.name.clone())
                            .unwrap_or_else(|| "Unknown Artist".to_string()),
                    ),
                    rspotify::model::PlayableItem::Episode(episode) =>
                    {
                        #[allow(deprecated)]
                        (episode.name.clone(), episode.show.publisher.clone())
                    }
                    _ => ("Unknown".to_string(), "Unknown Artist".to_string()),
                };
                lines.push(Line::from(format!("{}. {} - {}", i + 1, name, artist)));
            }

            if spotify_queue.queue.len() > 5 {
                lines.push(Line::styled(
                    format!("... and {} more", spotify_queue.queue.len() - 5),
                    Catppuccin::dim(),
                ));
            }
            lines.push(Line::from(""));
        }
    }

    if queue_state.local_queue.is_empty()
        && queue_state
            .spotify_queue
            .as_ref()
            .is_none_or(|q| q.currently_playing.is_none() && q.queue.is_empty())
    {
        lines.push(Line::styled("Queue is empty", Catppuccin::warning()));
        lines.push(Line::from(""));
        lines.push(Line::from("Add tracks to queue from search results"));
        lines.push(Line::from("or liked songs to see them here."));
    }

    // Footer
    lines.push(Line::styled("Press Esc to close", Catppuccin::help()));

    let widget = Paragraph::new(lines).alignment(Alignment::Left);
    frame.render_widget(widget, overlay_area);
}

/// Render help overlay
pub fn render_help_overlay(frame: &mut ratatui::Frame, area: Rect, help_lines: &[String]) {
    let lines: Vec<Line> = help_lines
        .iter()
        .map(|l| {
            if l.starts_with("===") {
                Line::styled(
                    l.clone(),
                    Catppuccin::secondary().add_modifier(Modifier::BOLD),
                )
            } else {
                Line::styled(l.clone(), Catppuccin::text())
            }
        })
        .collect();

    render_overlay_base(frame, area, " Help (?/Esc) ", lines);
}

/// Base overlay renderer - ensures NON-TRANSPARENT background
fn render_overlay_base(frame: &mut ratatui::Frame, area: Rect, title: &str, content: Vec<Line>) {
    // Create centered overlay area
    let overlay_width = (area.width as f32 * 0.7).clamp(40.0, area.width as f32) as u16;
    let overlay_height = (area.height as f32 * 0.7).clamp(15.0, area.height as f32) as u16;
    let overlay_x = (area.width - overlay_width) / 2;
    let overlay_y = (area.height - overlay_height) / 2;
    let overlay_area = Rect::new(overlay_x, overlay_y, overlay_width, overlay_height);

    // CRITICAL: Clear first to prevent any bleed-through
    frame.render_widget(Clear, overlay_area);

    // Then render solid background block
    let bg = Block::default()
        .style(Style::default().bg(Catppuccin::CRUST).fg(Catppuccin::TEXT))
        .borders(Borders::ALL)
        .border_style(Catppuccin::secondary().add_modifier(Modifier::BOLD))
        .title(title)
        .title_style(Catppuccin::focused());
    frame.render_widget(bg, overlay_area);

    // Render content on top
    let widget = Paragraph::new(content).alignment(Alignment::Left);
    frame.render_widget(widget, overlay_area);
}

/// Render search overlay with live results
pub fn render_search_overlay(frame: &mut ratatui::Frame, area: Rect, search_state: &SearchState) {
    let overlay_width = (area.width as f32 * 0.7).clamp(50.0, area.width as f32) as u16;
    let overlay_height = (area.height as f32 * 0.7).clamp(12.0, area.height as f32) as u16;
    let overlay_x = (area.width - overlay_width) / 2;
    let overlay_y = (area.height - overlay_height) / 2;
    let overlay_area = Rect::new(overlay_x, overlay_y, overlay_width, overlay_height);

    // Clear first to prevent bleed-through
    frame.render_widget(Clear, overlay_area);

    // Solid background block
    let bg = Block::default()
        .style(Style::default().bg(Catppuccin::CRUST).fg(Catppuccin::TEXT))
        .borders(Borders::ALL)
        .border_style(Catppuccin::border_focused().add_modifier(Modifier::BOLD))
        .title(" Search ")
        .title_style(Catppuccin::focused());
    let inner = bg.inner(overlay_area);
    frame.render_widget(bg, overlay_area);

    // Calculate available widths for content
    let input_max_width = inner.width.saturating_sub(4) as usize;
    let separator_width = inner.width.saturating_sub(4) as usize;
    let result_max_width = inner.width.saturating_sub(6) as usize;

    let mut lines: Vec<Line> = Vec::new();

    // Search input line with truncation
    let display_query = if search_state.query.is_empty() {
        "Type to search...".to_string()
    } else if search_state.query.chars().count() > input_max_width {
        let skip = search_state
            .query
            .chars()
            .count()
            .saturating_sub(input_max_width)
            .saturating_sub(1);
        format!(
            "…{}",
            search_state.query.chars().skip(skip).collect::<String>()
        )
    } else {
        search_state.query.clone()
    };

    let input_style = if search_state.query.is_empty() {
        Catppuccin::dim()
    } else {
        Catppuccin::search_input().add_modifier(Modifier::BOLD)
    };

    lines.push(Line::styled(format!("  🔍 {}", display_query), input_style));

    // Dynamic separator width
    lines.push(Line::styled(
        format!("  {}", "─".repeat(separator_width)),
        Catppuccin::dim(),
    ));

    // Loading indicator
    if search_state.is_loading {
        lines.push(Line::styled(
            "  ⏳ Searching...",
            Catppuccin::loading().add_modifier(Modifier::BOLD),
        ));
    } else if let Some(ref error) = search_state.error {
        let error_text = if error.chars().count() > result_max_width {
            let skip = error
                .chars()
                .count()
                .saturating_sub(result_max_width)
                .saturating_sub(1);
            format!("…{}", error.chars().skip(skip).collect::<String>())
        } else {
            error.clone()
        };
        lines.push(Line::styled(
            format!("  ❌ {}", error_text),
            Catppuccin::error(),
        ));
    } else if search_state.query.is_empty() {
        lines.push(Line::styled(
            "  Start typing to search Spotify...",
            Catppuccin::dim(),
        ));
    } else if search_state.results.is_empty() {
        lines.push(Line::styled("  No results found", Catppuccin::warning()));
    } else {
        // Results header
        lines.push(Line::styled(
            format!("  {} results", search_state.results.len()),
            Catppuccin::secondary().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(""));

        // Render results with selection
        let max_visible = (inner.height as usize).saturating_sub(6);
        let results_to_show = search_state.results.iter().take(25).enumerate();

        for (i, track) in results_to_show {
            if i < search_state.scroll_offset {
                continue;
            }
            if lines.len() >= max_visible + 4 {
                break;
            }

            let is_selected = i == search_state.selected_index;
            let marker = if is_selected { "▶" } else { "  " };
            let style = if is_selected {
                Catppuccin::primary().add_modifier(Modifier::BOLD)
            } else {
                Catppuccin::text()
            };

            let text = format!("{} {}. {} - {}", marker, i + 1, track.name, track.artist);
            let truncated = if text.chars().count() > result_max_width {
                let skip = text
                    .chars()
                    .count()
                    .saturating_sub(result_max_width)
                    .saturating_sub(1);
                format!("…{}", text.chars().skip(skip).collect::<String>())
            } else {
                text
            };
            lines.push(Line::styled(truncated, style));
        }

        if search_state.results.len() > 25 {
            lines.push(Line::styled(
                format!("  ... and {} more", search_state.results.len() - 25),
                Catppuccin::dim(),
            ));
        }
    }

    // Footer with key hints
    lines.push(Line::from(""));
    lines.push(Line::styled(
        "  Enter: Play  │  a: Add to queue  │  ↑↓: Navigate  │  Esc: Close",
        Catppuccin::help(),
    ));

    let widget = Paragraph::new(lines).alignment(Alignment::Left);
    frame.render_widget(widget, inner);

    // Set cursor position - use actual cursor_pos from search_state
    // The input line starts at inner.x + 3 (after "🔍 ")
    let cursor_x = inner.x + 3 + (search_state.cursor_pos as u16);
    let cursor_y = inner.y;
    frame.set_cursor_position((cursor_x, cursor_y));
}
