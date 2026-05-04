//! Help overlay with comprehensive keyboard and mouse shortcuts
//!
//! Provides a polished, organized help screen showing all available controls.
//! Organized by category for easy reference.

use crate::ui::theme::{symbols, Catppuccin};
use ratatui::{
    layout::{Alignment, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

/// Help content organized by category
#[derive(Debug, Clone)]
pub struct HelpContent {
    pub categories: Vec<HelpCategory>,
}

/// A category of help items (e.g., "Navigation", "Playback")
#[derive(Debug, Clone)]
pub struct HelpCategory {
    pub name: &'static str,
    pub icon: &'static str,
    pub items: Vec<HelpItem>,
}

/// A single help entry with key/command and description
#[derive(Debug, Clone)]
pub struct HelpItem {
    pub keys: &'static str,
    pub description: &'static str,
    pub mouse: Option<&'static str>,
}

impl HelpContent {
    /// Create the complete help content for Joshify
    pub fn joshify_help() -> Self {
        Self {
            categories: vec![
                HelpCategory {
                    name: "Global",
                    icon: symbols::HOME,
                    items: vec![
                        HelpItem {
                            keys: "q, Ctrl+C",
                            description: "Quit application",
                            mouse: None,
                        },
                        HelpItem {
                            keys: "?",
                            description: "Toggle this help",
                            mouse: None,
                        },
                        HelpItem {
                            keys: "Tab, Shift+Tab",
                            description: "Cycle focus between sections",
                            mouse: Some("Click section to focus"),
                        },
                        HelpItem {
                            keys: "Esc",
                            description: "Close overlays / Cancel",
                            mouse: Some("Click outside to close"),
                        },
                    ],
                },
                HelpCategory {
                    name: "Navigation",
                    icon: symbols::ARROW_RIGHT,
                    items: vec![
                        HelpItem {
                            keys: "↑ / k, ↓ / j",
                            description: "Navigate up/down in lists",
                            mouse: Some("Click item to select"),
                        },
                        HelpItem {
                            keys: "Enter",
                            description: "Select / Play highlighted item",
                            mouse: Some("Double-click to play"),
                        },
                        HelpItem {
                            keys: "g, G",
                            description: "Jump to top / bottom of list",
                            mouse: None,
                        },
                        HelpItem {
                            keys: "PgUp, PgDown",
                            description: "Scroll page up/down",
                            mouse: Some("Scroll wheel"),
                        },
                    ],
                },
                HelpCategory {
                    name: "Playback",
                    icon: symbols::PLAY,
                    items: vec![
                        HelpItem {
                            keys: "Space",
                            description: "Play / Pause",
                            mouse: Some("Click play button"),
                        },
                        HelpItem {
                            keys: "n",
                            description: "Next track",
                            mouse: Some("Click next button"),
                        },
                        HelpItem {
                            keys: "p",
                            description: "Previous track",
                            mouse: Some("Click previous button"),
                        },
                        HelpItem {
                            keys: "← / →",
                            description: "Seek backward / forward 10s",
                            mouse: Some("Click progress bar"),
                        },
                        HelpItem {
                            keys: "s",
                            description: "Toggle shuffle",
                            mouse: None,
                        },
                        HelpItem {
                            keys: "r",
                            description: "Cycle repeat mode",
                            mouse: None,
                        },
                    ],
                },
                HelpCategory {
                    name: "Volume",
                    icon: symbols::VOL_HIGH,
                    items: vec![
                        HelpItem {
                            keys: "+, =",
                            description: "Volume up",
                            mouse: Some("Scroll up on volume"),
                        },
                        HelpItem {
                            keys: "-, _",
                            description: "Volume down",
                            mouse: Some("Scroll down on volume"),
                        },
                        HelpItem {
                            keys: "m",
                            description: "Mute / Unmute",
                            mouse: None,
                        },
                    ],
                },
                HelpCategory {
                    name: "Search",
                    icon: symbols::SEARCH,
                    items: vec![
                        HelpItem {
                            keys: "/",
                            description: "Start search",
                            mouse: Some("Click search box"),
                        },
                        HelpItem {
                            keys: "Esc",
                            description: "Cancel search",
                            mouse: None,
                        },
                        HelpItem {
                            keys: "Enter",
                            description: "Execute search",
                            mouse: Some("Click search button"),
                        },
                        HelpItem {
                            keys: "Ctrl+U",
                            description: "Clear search input",
                            mouse: None,
                        },
                    ],
                },
                HelpCategory {
                    name: "Queue",
                    icon: symbols::QUEUE,
                    items: vec![
                        HelpItem {
                            keys: "Q",
                            description: "Toggle queue view",
                            mouse: Some("Click queue button"),
                        },
                        HelpItem {
                            keys: "a",
                            description: "Add to queue",
                            mouse: Some("Right-click → Add to queue"),
                        },
                        HelpItem {
                            keys: "c",
                            description: "Clear queue",
                            mouse: None,
                        },
                        HelpItem {
                            keys: "D",
                            description: "Remove from queue",
                            mouse: Some("Right-click → Remove"),
                        },
                        HelpItem {
                            keys: "Enter",
                            description: "Play from queue",
                            mouse: Some("Double-click track"),
                        },
                    ],
                },
                HelpCategory {
                    name: "Device",
                    icon: symbols::DEVICE_SPEAKER,
                    items: vec![
                        HelpItem {
                            keys: "d",
                            description: "Open device selector",
                            mouse: Some("Click device button"),
                        },
                        HelpItem {
                            keys: "↑ / ↓",
                            description: "Select device",
                            mouse: Some("Click device"),
                        },
                        HelpItem {
                            keys: "Enter",
                            description: "Switch to selected device",
                            mouse: Some("Double-click device"),
                        },
                        HelpItem {
                            keys: "Esc",
                            description: "Close device selector",
                            mouse: Some("Click outside"),
                        },
                    ],
                },
                HelpCategory {
                    name: "Library",
                    icon: symbols::LIBRARY,
                    items: vec![
                        HelpItem {
                            keys: "l",
                            description: "Go to Liked Songs",
                            mouse: Some("Click sidebar item"),
                        },
                        HelpItem {
                            keys: "L",
                            description: "Go to Playlists",
                            mouse: Some("Click sidebar item"),
                        },
                        HelpItem {
                            keys: "h",
                            description: "Go to Home",
                            mouse: Some("Click sidebar item"),
                        },
                        HelpItem {
                            keys: "Enter",
                            description: "Load more (at end of list)",
                            mouse: Some("Scroll to bottom"),
                        },
                    ],
                },
                HelpCategory {
                    name: "Appearance",
                    icon: "🎨",
                    items: vec![
                        HelpItem {
                            keys: "T",
                            description: "Cycle color theme",
                            mouse: None,
                        },
                        HelpItem {
                            keys: "",
                            description: "Themes: Catppuccin, Gruvbox, Nord, Tokyo Night, Dracula",
                            mouse: None,
                        },
                    ],
                },
            ],
        }
    }
}

/// State for scrollable help overlay
#[derive(Debug, Default)]
pub struct HelpOverlayState {
    pub scroll_offset: usize,
    pub max_scroll: usize,
}

impl HelpOverlayState {
    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = (self.scroll_offset + amount).min(self.max_scroll);
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.scroll_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.scroll_down(page_size);
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.max_scroll;
    }
}

/// Render the help overlay with scrollable content
pub fn render_help_overlay(
    frame: &mut Frame,
    area: Rect,
    content: &HelpContent,
    state: &mut HelpOverlayState,
) {
    // Calculate overlay dimensions (70% of screen, min 50x20, max 80x40)
    let overlay_width = (area.width as f32 * 0.75).clamp(50.0, 80.0) as u16;
    let overlay_height = (area.height as f32 * 0.8).clamp(20.0, 40.0) as u16;
    let overlay_x = (area.width - overlay_width) / 2;
    let overlay_y = (area.height - overlay_height) / 2;
    let overlay_area = Rect::new(overlay_x, overlay_y, overlay_width, overlay_height);

    // Clear background
    frame.render_widget(Clear, overlay_area);

    // Draw border block
    let block = Block::default()
        .title(format!(" {} Help {} ", symbols::HELP, symbols::HELP))
        .title_style(Catppuccin::focused())
        .borders(Borders::ALL)
        .border_style(Catppuccin::border_focused().add_modifier(Modifier::BOLD))
        .style(Style::default().bg(Catppuccin::CRUST));

    let inner = block.inner(overlay_area);
    frame.render_widget(block, overlay_area);

    // Calculate layout
    let content_width = inner.width.saturating_sub(4) as usize; // Account for scrollbar + padding
    let visible_height = inner.height.saturating_sub(2) as usize; // Account for padding

    // Build all lines of content
    let mut all_lines: Vec<Line> = Vec::new();

    // Header
    all_lines.push(Line::styled(
        "Joshify - Keyboard & Mouse Controls",
        Catppuccin::primary().add_modifier(Modifier::BOLD),
    ));
    all_lines.push(Line::from(""));
    all_lines.push(Line::styled(
        format!(
            "Use {} / {} to scroll, Esc to close",
            symbols::ARROW_UP,
            symbols::ARROW_DOWN
        ),
        Catppuccin::dim(),
    ));
    all_lines.push(Line::from(""));

    // Separator line
    let separator = "─".repeat(content_width.min(60));
    all_lines.push(Line::styled(separator, Catppuccin::border()));
    all_lines.push(Line::from(""));

    // Calculate column widths for key/command descriptions
    let key_col_width = 18usize; // Width for key column
    let desc_col_width = content_width.saturating_sub(key_col_width + 4); // Remaining for description

    // Render each category
    for category in &content.categories {
        // Category header with icon
        let header = format!("{} {}", category.icon, category.name);
        all_lines.push(Line::styled(
            header,
            Catppuccin::secondary().add_modifier(Modifier::BOLD),
        ));

        // Category items
        for item in &category.items {
            let key_text = format!("  {:<16}", item.keys);
            let truncated_desc = truncate(&item.description, desc_col_width);

            let mut spans = vec![
                Span::styled(key_text, Catppuccin::primary()),
                Span::styled(truncated_desc, Catppuccin::text()),
            ];

            // Add mouse hint if available
            if let Some(mouse) = item.mouse {
                let mouse_hint = format!("  [{}]", mouse);
                spans.push(Span::styled(mouse_hint, Catppuccin::dim()));
            }

            all_lines.push(Line::from(spans));
        }

        // Space between categories
        all_lines.push(Line::from(""));
    }

    // Footer with quick tips
    all_lines.push(Line::styled(
        "─".repeat(content_width.min(60)),
        Catppuccin::border(),
    ));
    all_lines.push(Line::from(""));
    all_lines.push(Line::styled(
        format!(
            "{} Tip: Use mouse for quick navigation and keyboard for precise control",
            symbols::STAR
        ),
        Catppuccin::info(),
    ));

    // Update max scroll
    let total_lines = all_lines.len();
    state.max_scroll = total_lines.saturating_sub(visible_height);

    // Slice visible lines based on scroll offset
    let end = (state.scroll_offset + visible_height).min(total_lines);
    let visible_lines: Vec<Line> = all_lines[state.scroll_offset..end].to_vec();

    // Create paragraph with visible content
    let content_area = inner.inner(Margin {
        horizontal: 1,
        vertical: 1,
    });

    let paragraph = Paragraph::new(visible_lines)
        .alignment(Alignment::Left)
        .wrap(ratatui::widgets::Wrap { trim: false });

    frame.render_widget(paragraph, content_area);

    // Render scrollbar if needed
    if total_lines > visible_height {
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .thumb_symbol("█")
            .track_symbol(Some("│"))
            .begin_symbol(None)
            .end_symbol(None);

        let mut scrollbar_state = ScrollbarState::new(total_lines)
            .position(state.scroll_offset)
            .viewport_content_length(visible_height);

        frame.render_stateful_widget(
            scrollbar,
            inner.inner(Margin {
                horizontal: 0,
                vertical: 1,
            }),
            &mut scrollbar_state,
        );
    }
}

/// Truncate text to fit within display width
fn truncate(text: &str, max_width: usize) -> String {
    if unicode_width::UnicodeWidthStr::width(text) <= max_width {
        text.to_string()
    } else {
        let (truncated, _) = unicode_truncate::UnicodeTruncateStr::unicode_truncate(
            text,
            max_width.saturating_sub(1),
        );
        format!("{}…", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_content_creation() {
        let help = HelpContent::joshify_help();
        assert!(!help.categories.is_empty());

        // Check that Global category exists
        let global = help.categories.iter().find(|c| c.name == "Global");
        assert!(global.is_some());

        // Check that Navigation category exists
        let nav = help.categories.iter().find(|c| c.name == "Navigation");
        assert!(nav.is_some());
    }

    #[test]
    fn test_help_overlay_state() {
        let mut state = HelpOverlayState {
            scroll_offset: 10,
            max_scroll: 20,
        };

        state.scroll_up(5);
        assert_eq!(state.scroll_offset, 5);

        state.scroll_down(3);
        assert_eq!(state.scroll_offset, 8);

        state.scroll_up(100); // Should not go below 0
        assert_eq!(state.scroll_offset, 0);

        state.scroll_down(100); // Should not exceed max_scroll
        assert_eq!(state.scroll_offset, 20);

        state.scroll_to_top();
        assert_eq!(state.scroll_offset, 0);

        state.scroll_to_bottom();
        assert_eq!(state.scroll_offset, 20);
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        // "hello world" (11 chars) truncated to 8: keep 7 chars + ellipsis
        assert_eq!(truncate("hello world", 8), "hello w…");
        assert_eq!(truncate("test", 3), "te…");
    }
}
