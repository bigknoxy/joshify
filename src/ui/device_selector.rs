//! Device selector UI rendering

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

/// Render device type icon
fn device_icon(device_type: &rspotify::model::DeviceType) -> &'static str {
    use rspotify::model::DeviceType;
    match device_type {
        DeviceType::Computer => "💻",
        DeviceType::Smartphone => "📱",
        DeviceType::Speaker => "🔊",
        DeviceType::Tv => "📺",
        DeviceType::Avr => "🎵",
        DeviceType::Stb => "📡",
        DeviceType::AudioDongle => "🔌",
        DeviceType::GameConsole => "🎮",
        DeviceType::CastVideo => "📹",
        DeviceType::CastAudio => "🔈",
        DeviceType::Automobile => "🚗",
        _ => "📻",
    }
}

/// Render the device selector overlay
pub fn render_device_selector(
    frame: &mut ratatui::Frame,
    area: Rect,
    devices: &[rspotify::model::Device],
    selected_index: usize,
) {
    // Create centered overlay area
    let overlay_width = 60u16.min(area.width.saturating_sub(4));
    let overlay_height = 15u16.min(area.height.saturating_sub(4));
    let x = area.x + (area.width - overlay_width) / 2;
    let y = area.y + (area.height - overlay_height) / 2;
    let overlay_area = Rect::new(x, y, overlay_width, overlay_height);

    // Clear background
    frame.render_widget(
        Block::default()
            .style(Style::default().bg(Color::Black).fg(Color::White))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Select Playback Device ")
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
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
    let device_items: Vec<ListItem> = devices
        .iter()
        .enumerate()
        .map(|(i, device)| {
            let icon = device_icon(&device._type);
            let active_marker = if device.is_active { " ▶" } else { "   " };
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
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if device.is_active {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else if device.is_restricted {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };

            let text = format!(
                "{} {} {}{}{}",
                active_marker, icon, device.name, volume, restricted_marker
            );
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

    let footer_text = " Enter: Switch  │  Esc: Cancel  │  j/k: Navigate ";
    let footer = Paragraph::new(footer_text)
        .style(Style::default().fg(Color::Yellow))
        .alignment(Alignment::Center);
    frame.render_widget(footer, footer_area);

    // Show "No devices" message if empty
    if devices.is_empty() {
        let msg_area = Rect::new(
            overlay_area.x + 1,
            overlay_area.y + 3,
            overlay_area.width - 2,
            3,
        );
        let msg = Paragraph::new("No devices found.\n\nOpen Spotify on another device first.")
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
        frame.render_widget(msg, msg_area);
    }
}
