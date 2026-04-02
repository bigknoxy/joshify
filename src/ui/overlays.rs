//! Overlay rendering - search input, help, queue

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

/// Render queue overlay
pub fn render_queue_overlay(frame: &mut ratatui::Frame, area: Rect, _player_state: &crate::state::player_state::PlayerState) {
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
