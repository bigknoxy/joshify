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
    let overlay_width = (area.width as f32 * 0.7).clamp(40.0, area.width as f32) as u16;
    let overlay_height = (area.height as f32 * 0.7).clamp(15.0, area.height as f32) as u16;
    let overlay_x = (area.width - overlay_width) / 2;
    let overlay_y = (area.height - overlay_height) / 2;
    let overlay_area = Rect::new(overlay_x, overlay_y, overlay_width, overlay_height);

    frame.render_widget(Clear, overlay_area);

    let bg = Block::default()
        .style(Style::default().bg(Catppuccin::CRUST).fg(Catppuccin::TEXT))
        .borders(Borders::ALL)
        .border_style(Catppuccin::border_focused().add_modifier(Modifier::BOLD))
        .title(" Queue ")
        .title_style(Catppuccin::focused());
    let inner = bg.inner(overlay_area);
    frame.render_widget(bg, overlay_area);

    let content_width = inner.width.saturating_sub(2) as usize;
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
            let truncated = truncate_from_start(&text, content_width);
            lines.push(Line::styled(truncated, Catppuccin::text()));
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
                let text = format!("▶ Now: {} - {}", name, artist);
                let truncated = truncate_from_start(&text, content_width);
                lines.push(Line::styled(
                    truncated,
                    Catppuccin::success().add_modifier(Modifier::BOLD),
                ));
                lines.push(Line::from(""));
            }

            for (i, item) in spotify_queue.queue.iter().take(15).enumerate() {
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
                let text = format!("{}. {} - {}", i + 1, name, artist);
                let truncated = truncate_from_start(&text, content_width);
                lines.push(Line::from(truncated));
            }

            if spotify_queue.queue.len() > 15 {
                lines.push(Line::styled(
                    format!("... and {} more", spotify_queue.queue.len() - 10),
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
    frame.render_widget(widget, inner);
}

/// Get display width of a string in terminal columns
fn display_width(s: &str) -> usize {
    use unicode_width::UnicodeWidthStr;
    UnicodeWidthStr::width(s)
}

/// Truncate text from the start to fit within max_width display columns, adding ellipsis prefix
fn truncate_from_start(text: &str, max_width: usize) -> String {
    use unicode_truncate::UnicodeTruncateStr;
    if display_width(text) <= max_width {
        text.to_string()
    } else {
        let (truncated, _) = text.unicode_truncate_start(max_width.saturating_sub(1));
        format!("…{}", truncated)
    }
}

#[cfg(test)]
mod tests {
    use unicode_truncate::UnicodeTruncateStr;
    use unicode_width::UnicodeWidthStr;

    fn test_display_width(s: &str) -> usize {
        UnicodeWidthStr::width(s)
    }

    fn test_truncate_from_start(text: &str, max_width: usize) -> String {
        if test_display_width(text) <= max_width {
            text.to_string()
        } else {
            let (truncated, _) = text.unicode_truncate_start(max_width.saturating_sub(1));
            format!("…{}", truncated)
        }
    }

    #[test]
    fn test_display_width_ascii() {
        assert_eq!(test_display_width("hello"), 5);
        assert_eq!(test_display_width(""), 0);
        assert_eq!(test_display_width("test123"), 7);
    }

    #[test]
    fn test_display_width_emoji() {
        assert_eq!(test_display_width("🦀"), 2);
        assert_eq!(test_display_width("h🦀llo"), 6);
        assert_eq!(test_display_width("🔍"), 2);
        assert_eq!(test_display_width("🔍🔍"), 4);
    }

    #[test]
    fn test_display_width_mixed() {
        assert_eq!(test_display_width("test🦀"), 6);
        assert_eq!(test_display_width("🦀test"), 6);
        assert_eq!(test_display_width("a🦀b"), 4);
    }

    #[test]
    fn test_truncate_from_start_no_truncation() {
        assert_eq!(test_truncate_from_start("hello", 10), "hello");
        assert_eq!(test_truncate_from_start("test", 4), "test");
    }

    #[test]
    fn test_truncate_from_start_with_truncation() {
        let result = test_truncate_from_start("hello world", 5);
        assert!(result.starts_with("…"));
        assert!(test_display_width(&result) <= 5);
    }

    #[test]
    fn test_truncate_from_start_with_emoji() {
        let result = test_truncate_from_start("hello🦀world", 6);
        assert!(result.starts_with("…"));
        assert!(test_display_width(&result) <= 6);
    }

    #[test]
    fn test_truncate_from_start_empty() {
        assert_eq!(test_truncate_from_start("", 5), "");
    }

    #[test]
    fn test_prefix_width_correct() {
        const SEARCH_PREFIX: &str = "  / ";
        assert_eq!(test_display_width(SEARCH_PREFIX), 4);
    }
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

    // Prefix for search input: "/ " = 2 display columns
    const SEARCH_PREFIX: &str = "/ ";
    let prefix_width = display_width(SEARCH_PREFIX);

    // Calculate available widths for content with better padding
    let input_max_width = inner.width.saturating_sub(prefix_width as u16 + 2) as usize;
    let separator_width = inner.width.saturating_sub(2) as usize;
    let result_max_width = inner.width.saturating_sub(4) as usize;

    let mut lines: Vec<Line> = Vec::new();

    // Search input line with display-width-aware truncation
    let display_query = if search_state.query.is_empty() {
        "Type to search...".to_string()
    } else {
        truncate_from_start(&search_state.query, input_max_width)
    };

    let input_style = if search_state.query.is_empty() {
        Catppuccin::dim()
    } else {
        Catppuccin::search_input().add_modifier(Modifier::BOLD)
    };

    // Add empty line at top for breathing room
    lines.push(Line::from(""));
    
    lines.push(Line::styled(
        format!(" {}{}", SEARCH_PREFIX, display_query),
        input_style,
    ));

    // Dynamic separator width
    lines.push(Line::styled(
        format!(" {}", "─".repeat(separator_width)),
        Catppuccin::dim(),
    ));

    // Add spacing before content
    lines.push(Line::from(""));
    
    // Loading indicator
    if search_state.is_loading {
        lines.push(Line::styled(
            " ⏳ Searching...",
            Catppuccin::loading().add_modifier(Modifier::BOLD),
        ));
    } else if let Some(ref error) = search_state.error {
        let error_text = truncate_from_start(error, result_max_width);
        lines.push(Line::styled(
            format!(" ❌ {}", error_text),
            Catppuccin::error(),
        ));
    } else if search_state.query.is_empty() {
        lines.push(Line::styled(
            " Start typing to search Spotify...",
            Catppuccin::dim(),
        ));
    } else if search_state.results.is_empty() {
        lines.push(Line::styled(" No results found", Catppuccin::warning()));
    } else {
        // Results header
        lines.push(Line::styled(
            format!(" {} results", search_state.results.len()),
            Catppuccin::secondary().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(""));

        // Render results with selection
        let max_visible = (inner.height as usize).saturating_sub(8);
        let results_to_show = search_state.results.iter().take(20).enumerate();

        for (i, track) in results_to_show {
            if i < search_state.scroll_offset {
                continue;
            }
            if lines.len() >= max_visible + 4 {
                break;
            }

            let is_selected = i == search_state.selected_index;
            let marker = if is_selected { "▶" } else { " " };
            let number = format!("{}.", i + 1);
            let style = if is_selected {
                Catppuccin::primary().add_modifier(Modifier::BOLD)
            } else {
                Catppuccin::text()
            };

            // Format: ▶ 1. Track Name - Artist
            let text = format!("{}{} {} - {}", marker, number, track.name, track.artist);
            let truncated = truncate_from_start(&text, result_max_width);
            lines.push(Line::styled(truncated, style));
        }

        if search_state.results.len() > 20 {
            lines.push(Line::styled(
                format!(" ... and {} more", search_state.results.len() - 20),
                Catppuccin::dim(),
            ));
        }
    }

    // Footer with key hints
    lines.push(Line::from(""));
    lines.push(Line::styled(
        " Enter: Play │ Tab: Queue │ ↑↓: Navigate │ Esc: Close",
        Catppuccin::help(),
    ));

    let widget = Paragraph::new(lines).alignment(Alignment::Left);
    frame.render_widget(widget, inner);

    // Set cursor position using display width (not character count)
    // Line 0 is empty (breathing room), Line 1 is the search input
    let cursor_y = inner.y + 1;
    let cursor_x = {
        // Use the SearchState helper methods for display width calculations
        let cursor_display_offset = search_state.cursor_display_offset();
        let query_width = search_state.query_display_width();

        if query_width > input_max_width {
            // Text is truncated from start: "…{visible portion}"
            // Need to compute how much display width was skipped
            let visible_start_width = query_width.saturating_sub(input_max_width.saturating_sub(1));
            let visible_cursor_offset = cursor_display_offset.saturating_sub(visible_start_width);
            // +1 for the "…" prefix
            inner.x + prefix_width as u16 + 1 + visible_cursor_offset as u16
        } else {
            // No truncation - use raw display offset
            inner.x + prefix_width as u16 + cursor_display_offset as u16
        }
    };
    frame.set_cursor_position((cursor_x, cursor_y));
}
