//! Device selector UI rendering

use crate::state::app_state::DeviceEntry;
use crate::ui::theme::{symbols, Catppuccin};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

/// Render device type icon
fn device_icon(device_type: &rspotify::model::DeviceType) -> &'static str {
    use rspotify::model::DeviceType;
    match device_type {
        DeviceType::Computer => symbols::DEVICE_COMPUTER,
        DeviceType::Smartphone => symbols::DEVICE_PHONE,
        DeviceType::Speaker => symbols::DEVICE_SPEAKER,
        DeviceType::Tv => symbols::DEVICE_TV,
        DeviceType::Avr => symbols::MUSIC,
        DeviceType::Stb => "📡",
        DeviceType::AudioDongle => "🔌",
        DeviceType::GameConsole => "🎮",
        DeviceType::CastVideo => "📹",
        DeviceType::CastAudio => symbols::DEVICE_SPEAKER,
        DeviceType::Automobile => symbols::DEVICE_CAR,
        _ => "📻",
    }
}

/// Render the device selector overlay
pub fn render_device_selector(
    frame: &mut ratatui::Frame,
    area: Rect,
    entries: &[DeviceEntry],
    selected_index: usize,
) {
    // Create centered overlay area
    let overlay_width = 60u16.min(area.width.saturating_sub(4));
    let overlay_height = 15u16.min(area.height.saturating_sub(4));
    let x = area.x + (area.width - overlay_width) / 2;
    let y = area.y + (area.height - overlay_height) / 2;
    let overlay_area = Rect::new(x, y, overlay_width, overlay_height);

    // CRITICAL: Clear background first to prevent bleed-through
    frame.render_widget(Clear, overlay_area);

    // Render modal background
    frame.render_widget(
        Block::default()
            .style(Style::default().bg(Catppuccin::SURFACE_0))
            .borders(Borders::ALL)
            .border_style(Catppuccin::secondary().add_modifier(Modifier::BOLD))
            .title(" Select Playback Device ")
            .title_style(Catppuccin::focused()),
        overlay_area,
    );

    // Split into content area (leave room for borders and footer)
    let content_area = Rect::new(
        overlay_area.x + 1,
        overlay_area.y + 1,
        overlay_area.width - 2,
        overlay_area.height - 4,
    );

    // Build device list
    let device_items: Vec<ListItem> = entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let (text, style) = match entry {
                DeviceEntry::ThisDevice { active } => {
                    let hostname = hostname::get()
                        .ok()
                        .and_then(|h| h.into_string().ok())
                        .unwrap_or_else(|| "this device".to_string());
                    let marker = if *active {
                        format!("{} ", symbols::ARROW_RIGHT)
                    } else {
                        "  ".to_string()
                    };
                    let text = format!(
                        "{} {} This Device ({})",
                        marker,
                        symbols::DEVICE_LOCAL,
                        hostname
                    );
                    let style = if i == selected_index {
                        Catppuccin::selected()
                    } else if *active {
                        Catppuccin::success().add_modifier(Modifier::BOLD)
                    } else {
                        Catppuccin::text()
                    };
                    (text, style)
                }
                DeviceEntry::Remote(device) => {
                    let icon = device_icon(&device._type);
                    let active_marker = if device.is_active {
                        format!("{} ", symbols::ARROW_RIGHT)
                    } else {
                        "  ".to_string()
                    };
                    let restricted_marker = if device.is_restricted {
                        " [restricted]"
                    } else {
                        ""
                    };
                    let volume = device
                        .volume_percent
                        .map(|v| format!(" {}%", v))
                        .unwrap_or_default();

                    let style = if i == selected_index {
                        Catppuccin::selected()
                    } else if device.is_active {
                        Catppuccin::success().add_modifier(Modifier::BOLD)
                    } else if device.is_restricted {
                        Catppuccin::dim()
                    } else {
                        Catppuccin::text()
                    };

                    let text = format!(
                        "{} {} {}{}{}",
                        active_marker, icon, device.name, volume, restricted_marker
                    );
                    (text, style)
                }
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let device_list = List::new(device_items);
    frame.render_widget(device_list, content_area);

    // Render footer with instructions
    let footer_area = Rect::new(
        overlay_area.x + 1,
        overlay_area.y + overlay_area.height - 3,
        overlay_area.width - 2,
        2,
    );

    let footer_text = format!(
        " {} Enter  │  {} Esc  │  j/k Navigate ",
        symbols::ARROW_RIGHT,
        symbols::ARROW_LEFT
    );
    let footer = Paragraph::new(footer_text)
        .style(Catppuccin::help())
        .alignment(Alignment::Center);
    frame.render_widget(footer, footer_area);

    // Show "No devices" message if empty (excluding "This Device")
    if entries.len() <= 1 {
        let msg_area = Rect::new(
            overlay_area.x + 1,
            overlay_area.y + 3,
            overlay_area.width - 2,
            3,
        );
        let msg = Paragraph::new(format!(
            "No remote devices found.\n\n{} Open Spotify on another device to see it here.",
            symbols::WARNING
        ))
        .style(Catppuccin::warning())
        .alignment(Alignment::Center);
        frame.render_widget(msg, msg_area);
    }
}
