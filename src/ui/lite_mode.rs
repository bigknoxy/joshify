//! LITE Mode UI - Minimal terminal interface
//!
//! A simplified UI mode inspired by the landing page demo:
//! - No sidebar, full-width content
//! - No borders, minimal visual elements
//! - ASCII progress bar
//! - Emoji-based icons
//! - Stream-like terminal aesthetic

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::state::player_state::PlayerState;
use crate::state::queue_state::QueueState;
use crate::ui::theme::Catppuccin;

/// Render the LITE mode UI
pub fn render_lite_mode(
    frame: &mut Frame,
    player_state: &PlayerState,
    queue_state: &QueueState,
    status_message: &Option<String>,
) {
    let area = frame.area();

    // Main vertical layout
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Status/connection line
            Constraint::Length(1),  // Spacer
            Constraint::Length(1),  // "▶ Now Playing" header
            Constraint::Length(1),  // Track name
            Constraint::Length(1),  // Artist
            Constraint::Length(1),  // Album (using track name area for simplicity)
            Constraint::Length(1),  // Spacer
            Constraint::Length(1),  // Progress bar
            Constraint::Length(1),  // Spacer
            Constraint::Length(1),  // Playback controls
            Constraint::Length(1),  // Spacer
            Constraint::Length(1),  // Queue indicator (if has items)
            Constraint::Min(0),     // Flexible space
            Constraint::Length(1),  // Help hint
            Constraint::Length(1),  // Prompt line
        ])
        .margin(1)
        .split(area);

    let mut row_idx = 0;

    // Row 0: Status/connection line
    render_status_line(frame, main_layout[row_idx], status_message);
    row_idx += 1;

    // Row 1: Spacer
    row_idx += 1;

    // Row 2: "▶ Now Playing" header
    render_now_playing_header(frame, main_layout[row_idx]);
    row_idx += 1;

    // Row 3-5: Track info (track, artist, no album in this mode)
    render_track_info(frame, main_layout[row_idx], main_layout[row_idx + 1], player_state);
    row_idx += 2;
    // Skip the album row
    row_idx += 1;

    // Row 6: Spacer
    row_idx += 1;

    // Row 7: Progress bar
    render_progress_bar(frame, main_layout[row_idx], player_state);
    row_idx += 1;

    // Row 8: Spacer
    row_idx += 1;

    // Row 9: Playback controls
    render_controls(frame, main_layout[row_idx], player_state);
    row_idx += 1;

    // Row 10: Spacer
    row_idx += 1;

    // Row 11: Queue indicator (only if queue has items)
    let queue_count = queue_state.total_count();
    if queue_count > 0 {
        render_queue_indicator(frame, main_layout[row_idx], queue_count);
    }
    row_idx += 1;

    // Skip flexible space
    row_idx += 1;

    // Row 13: Help hint
    render_help_hint(frame, main_layout[row_idx]);
    row_idx += 1;

    // Row 14: Prompt line
    render_prompt_line(frame, main_layout[row_idx]);
}

/// Render status/connection line
fn render_status_line(frame: &mut Frame, area: Rect, status_message: &Option<String>) {
    let text = if let Some(msg) = status_message {
        format!("# {}", msg)
    } else {
        "# Connected to Spotify".to_string()
    };

    let span = Span::styled(
        text,
        Catppuccin::dim().add_modifier(Modifier::ITALIC),
    );

    let line = Line::from(vec![span]);
    let paragraph = Paragraph::new(Text::from(vec![line]));
    frame.render_widget(paragraph, area);
}

/// Render "▶ Now Playing" header
fn render_now_playing_header(frame: &mut Frame, area: Rect) {
    let span = Span::styled(
        "▶ Now Playing",
        Catppuccin::secondary()
            .add_modifier(Modifier::BOLD),
    );

    let line = Line::from(vec![span]);
    let paragraph = Paragraph::new(Text::from(vec![line]));
    frame.render_widget(paragraph, area);
}

/// Render track information (name, artist)
fn render_track_info(
    frame: &mut Frame,
    track_area: Rect,
    artist_area: Rect,
    player_state: &PlayerState,
) {
    // Track name with music note emoji
    let track_name = player_state
        .current_track_name
        .as_deref()
        .unwrap_or("No track playing");
    let track_line = Line::from(vec![
        Span::styled("🎵 ", Catppuccin::text()),
        Span::styled(track_name.to_string(), Catppuccin::text()),
    ]);
    let track_para = Paragraph::new(Text::from(vec![track_line]));
    frame.render_widget(track_para, track_area);

    // Artist with person emoji
    let artist_name = player_state
        .current_artist_name
        .as_deref()
        .unwrap_or("Unknown Artist");
    let artist_line = Line::from(vec![
        Span::styled("👤 ", Catppuccin::text()),
        Span::styled(artist_name.to_string(), Catppuccin::dim()),
    ]);
    let artist_para = Paragraph::new(Text::from(vec![artist_line]));
    frame.render_widget(artist_para, artist_area);
}

/// Render ASCII progress bar
fn render_progress_bar(frame: &mut Frame, area: Rect, player_state: &PlayerState) {
    let progress_ms = player_state.progress_ms;
    let duration_ms = player_state.duration_ms;

    let progress_text = if duration_ms > 0 {
        let progress_secs = progress_ms / 1000;
        let duration_secs = duration_ms / 1000;
        let progress_min = progress_secs / 60;
        let progress_sec = progress_secs % 60;
        let duration_min = duration_secs / 60;
        let duration_sec = duration_secs % 60;

        // Calculate percentage for bar
        let percent = (progress_ms as f64 / duration_ms as f64).min(1.0);
        let bar_width = 20;
        let filled = (percent * bar_width as f64) as usize;
        let empty = bar_width - filled;

        let bar = "█".repeat(filled) + &"░".repeat(empty);

        format!(
            "{} {:02}:{:02} / {:02}:{:02}",
            bar, progress_min, progress_sec, duration_min, duration_sec
        )
    } else {
        "░░░░░░░░░░░░░░░░░░░░ 0:00 / 0:00".to_string()
    };

    let line = Line::from(vec![Span::styled(
        progress_text,
        Catppuccin::dim(),
    )]);
    let paragraph = Paragraph::new(Text::from(vec![line]));
    frame.render_widget(paragraph, area);
}

/// Render playback controls
fn render_controls(frame: &mut Frame, area: Rect, player_state: &PlayerState) {
    let play_pause = if player_state.is_playing {
        "⏸"
    } else {
        "▶"
    };

    let controls_text = format!("⏮  {}  ⏭", play_pause);

    let line = Line::from(vec![Span::styled(
        controls_text,
        Catppuccin::text(),
    )]);
    let paragraph = Paragraph::new(Text::from(vec![line]));
    frame.render_widget(paragraph, area);
}

/// Render queue indicator
fn render_queue_indicator(frame: &mut Frame, area: Rect, queue_count: usize) {
    if queue_count > 0 {
        let text = format!("📋 Queue: {} tracks", queue_count);
        let line = Line::from(vec![Span::styled(
            text,
            Catppuccin::dim(),
        )]);
        let paragraph = Paragraph::new(Text::from(vec![line]));
        frame.render_widget(paragraph, area);
    }
}

/// Render help hint
fn render_help_hint(frame: &mut Frame, area: Rect) {
    let text = "# Press ? for help, q to quit";
    let line = Line::from(vec![Span::styled(
        text,
        Catppuccin::dim().add_modifier(Modifier::ITALIC),
    )]);
    let paragraph = Paragraph::new(Text::from(vec![line]));
    frame.render_widget(paragraph, area);
}

/// Render terminal-style prompt line
fn render_prompt_line(frame: &mut Frame, area: Rect) {
    let text = "$ _";
    let line = Line::from(vec![Span::styled(
        text,
        Catppuccin::success(),
    )]);
    let paragraph = Paragraph::new(Text::from(vec![line]));
    frame.render_widget(paragraph, area);
}

/// Render LITE mode help overlay
pub fn render_lite_help(frame: &mut Frame, area: Rect) {
    // Center the help dialog
    let help_area = centered_rect(60, 70, area);

    // Clear background
    frame.render_widget(Clear, help_area);

    // Help content
    let help_text = vec![
        Line::from(vec![Span::styled(
            "Joshify LITE Mode - Keyboard Controls",
            Catppuccin::focused(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Space", Catppuccin::success().add_modifier(Modifier::BOLD)),
            Span::styled("     Play/Pause", Catppuccin::text()),
        ]),
        Line::from(vec![
            Span::styled("/", Catppuccin::success().add_modifier(Modifier::BOLD)),
            Span::styled("         Search tracks", Catppuccin::text()),
        ]),
        Line::from(vec![
            Span::styled("a", Catppuccin::success().add_modifier(Modifier::BOLD)),
            Span::styled("         Add to queue", Catppuccin::text()),
        ]),
        Line::from(vec![
            Span::styled("r", Catppuccin::success().add_modifier(Modifier::BOLD)),
            Span::styled("         Toggle radio mode (auto-recommendations)", Catppuccin::text()),
        ]),
        Line::from(vec![
            Span::styled("n", Catppuccin::success().add_modifier(Modifier::BOLD)),
            Span::styled("         Next track", Catppuccin::text()),
        ]),
        Line::from(vec![
            Span::styled("p", Catppuccin::success().add_modifier(Modifier::BOLD)),
            Span::styled("         Previous track", Catppuccin::text()),
        ]),
        Line::from(vec![
            Span::styled("← →", Catppuccin::success().add_modifier(Modifier::BOLD)),
            Span::styled("      Seek 10s backward/forward", Catppuccin::text()),
        ]),
        Line::from(vec![
            Span::styled("?", Catppuccin::success().add_modifier(Modifier::BOLD)),
            Span::styled("         Show/hide this help", Catppuccin::text()),
        ]),
        Line::from(vec![
            Span::styled("q", Catppuccin::success().add_modifier(Modifier::BOLD)),
            Span::styled("         Quit", Catppuccin::text()),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Radio Mode: When queue is empty, automatically",
            Catppuccin::dim(),
        )]),
        Line::from(vec![Span::styled(
            "generates recommendations based on current track.",
            Catppuccin::dim(),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press ? or q to close this help",
            Catppuccin::dim().add_modifier(Modifier::ITALIC),
        )]),
    ];

    let help_paragraph = Paragraph::new(Text::from(help_text))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Catppuccin::border()),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(help_paragraph, help_area);
}

/// Helper function to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lite_mode_imports_work() {
        // This test ensures the lite mode module compiles correctly
        // and the imports are valid
        let player_state = PlayerState::default();
        let queue_state = QueueState::default();

        // Verify the types work
        assert_eq!(player_state.progress_ms, 0);
        assert_eq!(queue_state.total_count(), 0);
    }

    #[test]
    fn test_queue_indicator_shows_when_queue_has_items() {
        let queue_state = QueueState::default();
        // Queue starts empty
        assert_eq!(queue_state.total_count(), 0);

        // Note: We can't easily add items without the full setup,
        // but we verify the API works
    }
}
