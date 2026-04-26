//! Sidebar navigation rendering

use crate::state::app_state::NavItem;
use crate::ui::layout_cache::LayoutCache;
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
pub fn render_sidebar(
    frame: &mut ratatui::Frame,
    area: Rect,
    selected: NavItem,
    focused: bool,
    layout_cache: &mut LayoutCache,
) {
    // Store the sidebar area for hit testing
    layout_cache.sidebar = Some(area);
    layout_cache.nav_items.clear();

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
    let logo = joshify_logo();
    let logo_lines = logo.len() as u16 + 1; // +1 for spacer
    // Account for top border (1 row) - content starts at area.y + 1
    let content_start_y = area.y + 1;
    let nav_start_y = content_start_y + logo_lines;

    let mut content = logo;
    content.push(Line::from("")); // Spacer

    let items: Vec<Line> = NavItem::all()
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let (icon, style) = if *item == selected {
                (
                    format!("{} ", symbols::ARROW_RIGHT),
                    Catppuccin::sidebar_item_selected(),
                )
            } else {
                ("  ".to_string(), Catppuccin::sidebar_item())
            };

            // Store the area for this nav item for hit testing
            // Each nav item is one line
            let item_area = Rect::new(area.x, nav_start_y + idx as u16, area.width, 1);
            layout_cache.nav_items.push(item_area);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::layout_cache::LayoutCache;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_nav_item_areas_account_for_border() {
        // Create a test backend with known dimensions
        let backend = TestBackend::new(80, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();

        let mut layout_cache = LayoutCache::new();

        // Render sidebar at a specific area
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 20, 40);
                render_sidebar(frame, area, NavItem::Home, false, &mut layout_cache);
            })
            .unwrap();

        // Verify nav items were added to cache
        assert!(!layout_cache.nav_items.is_empty());
        assert_eq!(layout_cache.nav_items.len(), NavItem::all().len());

        // Verify first nav item accounts for top border
        // Logo starts at y=1 (after border), logo has 14 lines + spacer = 15 lines
        // Nav items should start at y=1+15=16
        let first_nav_area = &layout_cache.nav_items[0];
        assert_eq!(first_nav_area.y, 16); // y=1 (border) + 15 (logo+spacer)
        assert_eq!(first_nav_area.height, 1);
        assert_eq!(first_nav_area.width, 20);

        // Verify second nav item is at y=17
        let second_nav_area = &layout_cache.nav_items[1];
        assert_eq!(second_nav_area.y, 17);

        // Verify last nav item position
        let last_idx = NavItem::all().len() - 1;
        let last_nav_area = &layout_cache.nav_items[last_idx];
        assert_eq!(last_nav_area.y, 16 + last_idx as u16);
    }

    #[test]
    fn test_logo_content_count() {
        // Verify logo line count for accurate nav positioning
        let logo = joshify_logo();
        // Logo has: empty line + 12 styled lines + empty line = 14 lines
        assert_eq!(logo.len(), 14);
    }

    #[test]
    fn test_nav_item_click_areas_are_contiguous() {
        let backend = TestBackend::new(80, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut layout_cache = LayoutCache::new();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 20, 40);
                render_sidebar(frame, area, NavItem::Home, false, &mut layout_cache);
            })
            .unwrap();

        // Verify nav items are contiguous (no gaps between them)
        for i in 0..layout_cache.nav_items.len().saturating_sub(1) {
            let current = &layout_cache.nav_items[i];
            let next = &layout_cache.nav_items[i + 1];
            
            // Next item should start exactly one row after current
            assert_eq!(next.y, current.y + 1, "Gap between nav items {} and {}", i, i + 1);
            assert_eq!(current.height, 1, "Nav item {} should be 1 row tall", i);
            assert_eq!(next.height, 1, "Nav item {} should be 1 row tall", i + 1);
        }
    }

    #[test]
    fn test_nav_item_areas_have_full_sidebar_width() {
        let backend = TestBackend::new(80, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut layout_cache = LayoutCache::new();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 25, 40);
                render_sidebar(frame, area, NavItem::Home, false, &mut layout_cache);
            })
            .unwrap();

        // All nav items should have the full sidebar width
        for (i, nav_area) in layout_cache.nav_items.iter().enumerate() {
            assert_eq!(
                nav_area.width, 25,
                "Nav item {} should have full sidebar width",
                i
            );
        }
    }

    #[test]
    fn test_nav_item_selected_state() {
        let backend = TestBackend::new(80, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut layout_cache = LayoutCache::new();

        // Test with Search selected
        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 20, 40);
                render_sidebar(frame, area, NavItem::Search, true, &mut layout_cache);
            })
            .unwrap();

        // Verify all nav items still have areas
        assert_eq!(layout_cache.nav_items.len(), NavItem::all().len());
    }

    #[test]
    fn test_nav_item_click_edge_cases() {
        let backend = TestBackend::new(80, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut layout_cache = LayoutCache::new();

        let sidebar_area = Rect::new(5, 5, 20, 40); // Offset sidebar

        terminal
            .draw(|frame| {
                render_sidebar(frame, sidebar_area, NavItem::Home, false, &mut layout_cache);
            })
            .unwrap();

        // Verify nav items are positioned relative to sidebar area
        let first_nav = &layout_cache.nav_items[0];
        assert_eq!(first_nav.x, 5, "Nav item x should match sidebar x");
        
        // Nav items should be within sidebar bounds
        for nav_area in &layout_cache.nav_items {
            assert!(nav_area.x >= sidebar_area.x);
            assert!(nav_area.x + nav_area.width <= sidebar_area.x + sidebar_area.width);
        }
    }

    #[test]
    fn test_sidebar_area_stored_for_hit_testing() {
        let backend = TestBackend::new(80, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut layout_cache = LayoutCache::new();

        let sidebar_area = Rect::new(0, 0, 20, 40);

        terminal
            .draw(|frame| {
                render_sidebar(frame, sidebar_area, NavItem::Home, false, &mut layout_cache);
            })
            .unwrap();

        // Verify sidebar area was stored
        assert_eq!(layout_cache.sidebar, Some(sidebar_area));
    }

    #[test]
    fn test_nav_items_cleared_before_render() {
        let backend = TestBackend::new(80, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut layout_cache = LayoutCache::new();

        // Pre-populate nav_items
        layout_cache.nav_items.push(Rect::new(0, 0, 20, 1));
        layout_cache.nav_items.push(Rect::new(0, 1, 20, 1));

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 20, 40);
                render_sidebar(frame, area, NavItem::Home, false, &mut layout_cache);
            })
            .unwrap();

        // Verify old items were replaced (not appended)
        assert_eq!(layout_cache.nav_items.len(), NavItem::all().len());
    }

    #[test]
    fn test_all_nav_items_rendered() {
        let backend = TestBackend::new(80, 40);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let mut layout_cache = LayoutCache::new();

        terminal
            .draw(|frame| {
                let area = Rect::new(0, 0, 20, 40);
                render_sidebar(frame, area, NavItem::Home, false, &mut layout_cache);
            })
            .unwrap();

        // Verify all nav items have areas
        let all_nav_items = NavItem::all();
        assert_eq!(
            layout_cache.nav_items.len(),
            all_nav_items.len(),
            "Should have area for each nav item"
        );

        // Verify we can map each area back to a nav item
        for (i, area) in layout_cache.nav_items.iter().enumerate() {
            assert!(i < all_nav_items.len(), "Extra nav area at index {}", i);
            assert_eq!(area.height, 1, "Nav item {} should be 1 row", i);
        }
    }
}
