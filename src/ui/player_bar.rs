//! Player bar rendering - Now playing with album art

use crate::ui::theme::{self, symbols, Catppuccin};
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
    album_art_ascii: Option<&[Line<'static>]>,
    queue_count: usize,
    focused: bool,
) {
    let play_icon = if is_playing {
        symbols::PLAY
    } else {
        symbols::PAUSE
    };
    let display_name = track_name;

    // Split player bar into album art (left) and info (right)
    let album_art_width = 10u16;
    let [album_area, info_area] =
        Layout::horizontal([Constraint::Length(album_art_width), Constraint::Min(0)]).areas(area);

    // Render pre-rendered ASCII art (zero per-frame processing)
    if let Some(lines) = album_art_ascii {
        let art_paragraph = Paragraph::new(lines.to_vec())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Catppuccin::border())
                    .title(" Album Art ")
                    .title_style(Catppuccin::success()),
            )
            .alignment(Alignment::Center);
        frame.render_widget(art_paragraph, album_area);
    } else if album_art_url.is_some() {
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
                    .border_style(Catppuccin::loading())
                    .title(" Loading ")
                    .title_style(Catppuccin::loading()),
            )
            .alignment(Alignment::Center);
        frame.render_widget(loading, album_area);
    } else {
        let art = vec![
            Line::from("       "),
            Line::from(format!("  {}    ", symbols::MUSIC_NOTE)),
            Line::from("       "),
            Line::from("       "),
        ];
        let placeholder = Paragraph::new(art)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Catppuccin::border())
                    .title(" No Art ")
                    .title_style(Catppuccin::dim()),
            )
            .alignment(Alignment::Center);
        frame.render_widget(placeholder, album_area);
    }

    // Queue indicator
    let queue_indicator = if queue_count > 0 {
        format!("  🎵{}", queue_count)
    } else {
        String::new()
    };

    // Track info with progress gauge
    let progress_text = format!(
        "{} / {}",
        crate::state::player_state::format_duration(progress_ms),
        crate::state::player_state::format_duration(duration_ms)
    );

    // Volume indicator with icon
    let (vol_icon, vol_style) = theme::volume_indicator(volume);
    let vol_bars = match volume {
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
            "{}… / {}…",
            &display_name.chars().take(half).collect::<String>(),
            &artist_name.chars().take(half).collect::<String>()
        )
    } else {
        format!("{} - {}", display_name, artist_name)
    };

    let lines = vec![
        Line::styled(
            format!(" {}  {}{}", play_icon, name_text, queue_indicator),
            Style::default()
                .fg(Catppuccin::TEXT)
                .add_modifier(Modifier::BOLD),
        ),
        Line::styled(
            format!(
                " {}  |  {}{}  |  ←/→:Seek  |  ↑/↓:Vol",
                progress_text, vol_icon, vol_bars
            ),
            vol_style,
        ),
    ];

    let border_style = if focused {
        Catppuccin::border_focused()
    } else {
        Catppuccin::success()
    };
    let focus_hint = if focused {
        " Now Playing (Enter:Play/Pause) "
    } else {
        " Now Playing "
    };

    let widget = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(focus_hint)
            .border_style(border_style.add_modifier(Modifier::BOLD))
            .title_style(if focused {
                Catppuccin::focused()
            } else {
                Catppuccin::success()
            }),
    );

    frame.render_widget(widget, info_area);
}
