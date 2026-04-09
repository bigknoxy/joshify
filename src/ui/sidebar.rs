//! Sidebar navigation rendering

use crate::state::app_state::NavItem;
use crate::ui::theme::{symbols, Catppuccin};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

/// The Joshify mascot - gorilla with headphones
fn joshify_logo() -> Vec<Line<'static>> {
    let g = Style::default().fg(Catppuccin::GREEN);
    let gb = Style::default()
        .fg(Catppuccin::GREEN)
        .add_modifier(Modifier::BOLD);
    let hp = Style::default().fg(Catppuccin::MAUVE);
    vec![
        Line::from(""),
        Line::styled("      ╭═══════╮", hp),
        Line::styled("    ╔═╝ ◉   ◉ ╚═╗", hp),
        Line::styled("    ║ ╭───────╮ ║", g),
        Line::styled("    ║ │ ██████ │ ║", g),
        Line::styled("    ║ │ ▀▀▀▀▀ │ ║", g),
        Line::styled("    ╚═╧ ▼▼▼▼▼ ╧═╝", g),
        Line::styled("        ╲▄▄▄▄▄╱", g),
        Line::styled("       ╱▓▓▌ ▐▓▓╲", g),
        Line::styled("      │▓▓▓▌ ▐▓▓▓│", g),
        Line::styled("      │ ║║   ║║ │", g),
        Line::styled("      ╰─╯    ╰─╯", g),
        Line::styled(
            format!(
                "     {} JOSHIFY {}",
                symbols::MUSIC_NOTE,
                symbols::MUSIC_NOTE
            ),
            gb,
        ),
        Line::from(""),
    ]
}

/// Render the sidebar navigation
pub fn render_sidebar(frame: &mut ratatui::Frame, area: Rect, selected: NavItem, focused: bool) {
    let border_style = if focused {
        Catppuccin::border_focused()
    } else {
        Catppuccin::border()
    };
    let title = if focused {
        " Navigation (↑/↓) "
    } else {
        " Navigation "
    };

    // Build content with logo at top
    let mut content = joshify_logo();
    content.push(Line::from("")); // Spacer

    let items: Vec<Line> = NavItem::all()
        .iter()
        .map(|item| {
            let (icon, style) = if *item == selected {
                (
                    format!("{} ", symbols::ARROW_RIGHT),
                    Catppuccin::sidebar_item_selected(),
                )
            } else {
                ("  ".to_string(), Catppuccin::sidebar_item())
            };
            Line::styled(format!("{}{}", icon, item.label()), style)
        })
        .collect();
    content.extend(items);

    let widget = Paragraph::new(content).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(border_style.add_modifier(Modifier::BOLD))
            .title_style(if focused {
                Catppuccin::focused()
            } else {
                Catppuccin::secondary()
            }),
    );

    frame.render_widget(widget, area);
}
