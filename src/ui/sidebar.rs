//! Sidebar navigation rendering

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};
use crate::state::app_state::NavItem;

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
