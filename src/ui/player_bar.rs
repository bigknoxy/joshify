//! Player bar rendering - Now playing with album art

use crate::ui::image_renderer::AlbumArtWidget;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

/// Render the player bar at the bottom with integrated album art
pub fn render_player_bar(
    frame: &mut ratatui::Frame,
    area: Rect,
    track_name: &str,
    artist_name: &str,
    is_playing: bool,
    progress_ms: u32,
    duration_ms: u32,
    volume: u32,
    album_art_url: Option<&str>,
    album_art_data: Option<&[u8]>,
    focused: bool,
) {
    let play_icon = if is_playing { "▶" } else { "||" };
    let display_name = track_name;

    // Split player bar into album art (left) and info (right)
    let album_art_width = 7u16; // Space for album art
    let [album_area, info_area] =
        Layout::horizontal([Constraint::Length(album_art_width), Constraint::Min(0)]).areas(area);

    // Render album art using protocol-aware renderer
    if album_art_data.is_some() {
        // Have image data - render with detected protocol
        let album_art = AlbumArtWidget::new(album_art_data);
        frame.render_widget(album_art, album_area);
    } else if album_art_url.is_some() {
        // Have URL but no data yet - show loading indicator
        let art = vec![
            Line::from("  ╭───╮  "),
            Line::from("  │...│  "),
            Line::from("  │...│  "),
            Line::from("  ╰───╯  "),
        ];
        let loading = Paragraph::new(art)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title("Loading...")
                    .title_style(Style::default().fg(Color::Yellow)),
            )
            .alignment(Alignment::Center);
        frame.render_widget(loading, album_area);
    } else {
        // No album art - show music note placeholder
        let art = vec![
            Line::from("       "),
            Line::from("  ♪    "),
            Line::from("       "),
            Line::from("       "),
        ];
        let placeholder = Paragraph::new(art)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title("No Art")
                    .title_style(Style::default().fg(Color::DarkGray)),
            )
            .alignment(Alignment::Center);
        frame.render_widget(placeholder, album_area);
    }

    // Track info
    let progress_text = format!(
        "{} / {}",
        crate::state::player_state::format_duration(progress_ms),
        crate::state::player_state::format_duration(duration_ms)
    );
    let volume_bars = match volume {
        0 => "░░░░",
        1..=25 => "█░░░",
        26..=50 => "██░░",
        51..=75 => "███░",
        _ => "████",
    };

    // Truncate for available space
    let max_len = info_area.width.saturating_sub(2) as usize;
    let name_text = if display_name.len() + artist_name.len() + 3 > max_len {
        let half = max_len / 2 - 2;
        format!(
            "{}... / {}...",
            &display_name.chars().take(half).collect::<String>(),
            &artist_name.chars().take(half).collect::<String>()
        )
    } else {
        format!("{} - {}", display_name, artist_name)
    };

    let lines = vec![
        Line::styled(
            format!(" {}  {}", play_icon, name_text),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Line::styled(
            format!(
                " {}  |  Vol:{}  |  ←/→:Seek  |  ↑/↓:Vol",
                progress_text, volume_bars
            ),
            Style::default().fg(Color::Gray),
        ),
    ];

    let border_color = if focused { Color::Yellow } else { Color::Green };
    let focus_hint = if focused {
        " Now Playing (Enter:Play/Pause) "
    } else {
        " Now Playing "
    };

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(focus_hint)
            .border_style(
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            )
            .title_style(
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            ),
    );

    frame.render_widget(widget, info_area);
}
