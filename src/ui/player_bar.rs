//! Player bar rendering - Now playing with album art, scrolling title, progress bar

use crate::state::player_state::{RepeatMode, TitleScrollState};
use crate::ui::theme::{self, symbols, Catppuccin};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Gauge, Paragraph},
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
    shuffle: bool,
    repeat_mode: RepeatMode,
    radio_mode: bool,
    title_scroll_state: &TitleScrollState,
) {
    let play_icon = if is_playing {
        symbols::PLAY
    } else {
        symbols::PAUSE
    };

    let album_art_width = 12u16;
    let [album_area, info_area] =
        Layout::horizontal([Constraint::Length(album_art_width), Constraint::Min(0)]).areas(area);

    // Render album art
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
            Line::from("    ╭───╮    "),
            Line::from("    │...│    "),
            Line::from("    │...│    "),
            Line::from("    ╰───╯    "),
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
            Line::from("            "),
            Line::from(format!("    {}      ", symbols::MUSIC_NOTE)),
            Line::from("            "),
            Line::from("            "),
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

    // Interior width of info panel (subtract borders)
    let inner_width = info_area.width.saturating_sub(2) as usize;
    if inner_width < 4 {
        return;
    }

    // Row 1: Scrolling title
    let title_display_width = unicode_width::UnicodeWidthStr::width(track_name);
    let title_text = if title_display_width > inner_width.saturating_sub(2) {
        match title_scroll_state {
            TitleScrollState::Static => {
                let (truncated, _) = unicode_truncate::UnicodeTruncateStr::unicode_truncate(
                    track_name,
                    inner_width.saturating_sub(3),
                );
                format!("{}…", truncated)
            }
            TitleScrollState::PausedAtStart { .. } => track_name.to_string(),
            TitleScrollState::Scrolling { fractional_offset } => {
                let offset = *fractional_offset as usize;
                let suffix = &track_name[byte_offset(track_name, offset)..];
                let (truncated, _) = unicode_truncate::UnicodeTruncateStr::unicode_truncate(
                    suffix,
                    inner_width.saturating_sub(2),
                );
                truncated.to_string()
            }
            TitleScrollState::PausedAtEnd { .. } => {
                let max_offset = title_display_width - inner_width.saturating_sub(2);
                let suffix = &track_name[byte_offset(track_name, max_offset)..];
                let (truncated, _) = unicode_truncate::UnicodeTruncateStr::unicode_truncate(
                    suffix,
                    inner_width.saturating_sub(2),
                );
                truncated.to_string()
            }
        }
    } else {
        track_name.to_string()
    };

    // Row 2: Artist + badges + key hints
    let mut badges = String::new();
    if shuffle {
        badges.push_str(&format!(" {} ", symbols::SHUFFLE));
    }
    match repeat_mode {
        RepeatMode::Off => {}
        RepeatMode::Context => {
            badges.push_str(&format!(" {} ", symbols::REPEAT));
        }
        RepeatMode::Track => {
            badges.push_str(&format!(" {} ", symbols::REPEAT_ONE));
        }
    }
    if radio_mode {
        badges.push_str(&format!(" {} ", symbols::RADIO));
    }
    if queue_count > 0 {
        badges.push_str(&format!(" {}{}", symbols::MUSIC_NOTE, queue_count));
    }

    let artist_display_width = unicode_width::UnicodeWidthStr::width(artist_name);
    let badges_width = unicode_width::UnicodeWidthStr::width(badges.as_str());
    let hints = "  s:⇄ r:↻ ←→:Seek";
    let hints_width = unicode_width::UnicodeWidthStr::width(hints);
    let available_artist_width = inner_width
        .saturating_sub(badges_width)
        .saturating_sub(hints_width)
        .saturating_sub(4);

    let artist_text = if artist_display_width > available_artist_width {
        let max = available_artist_width.saturating_sub(1);
        let (truncated, _) =
            unicode_truncate::UnicodeTruncateStr::unicode_truncate(artist_name, max);
        format!("{}…", truncated)
    } else {
        artist_name.to_string()
    };

    // Progress calculation
    let progress_ratio = if duration_ms > 0 {
        progress_ms as f64 / duration_ms as f64
    } else {
        0.0
    };

    // Volume bar
    let (vol_icon, vol_style) = theme::volume_indicator(volume);
    let vol_bars = match volume {
        0 => "░░░░░░",
        1..=16 => "█░░░░░",
        17..=33 => "██░░░░",
        34..=50 => "███░░░",
        51..=66 => "████░░",
        67..=83 => "█████░",
        _ => "██████",
    };

    // Time labels for progress bar
    let elapsed_text = crate::state::player_state::format_duration(progress_ms);
    let remaining_text = crate::state::player_state::format_duration(duration_ms);

    // Border and layout
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

    let info_block = Block::default()
        .borders(Borders::ALL)
        .title(focus_hint)
        .border_style(border_style.add_modifier(Modifier::BOLD))
        .title_style(if focused {
            Catppuccin::focused()
        } else {
            Catppuccin::success()
        });
    let info_inner = info_block.inner(info_area);
    frame.render_widget(info_block, info_area);

    // Split info inner into 4 rows
    let rows = Layout::vertical([
        Constraint::Length(1), // title
        Constraint::Length(1), // artist + badges
        Constraint::Length(1), // progress
        Constraint::Length(1), // volume
    ])
    .split(info_inner);

    // Row 1: Title
    let title_line = Line::styled(
        format!(" {} {}", play_icon, title_text),
        Style::default()
            .fg(Catppuccin::MAUVE)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(Paragraph::new(title_line), rows[0]);

    // Row 2: Artist + badges + hints
    let artist_spans = vec![
        Span::styled(
            format!("  {}", artist_text),
            Style::default().fg(Catppuccin::SUBTEXT_0),
        ),
        Span::styled(badges, Style::default().fg(Catppuccin::TEAL)),
        Span::styled(
            hints.to_string(),
            Style::default().fg(Catppuccin::SURFACE_2),
        ),
    ];
    frame.render_widget(Paragraph::new(Line::from(artist_spans)), rows[1]);

    // Row 3: Progress gauge with time labels
    // " 2:30 [████████░░░░] 4:15 "
    let progress_layout = Layout::horizontal([
        Constraint::Length(7), // " MM:SS "
        Constraint::Min(0),    // gauge bar
        Constraint::Length(8), // "  MM:SS "
    ])
    .split(rows[2]);

    frame.render_widget(
        Paragraph::new(format!(" {}", elapsed_text))
            .style(Style::default().fg(Catppuccin::SUBTEXT_0))
            .alignment(Alignment::Right),
        progress_layout[0],
    );

    let gauge = Gauge::default()
        .gauge_style(
            Style::default()
                .fg(Catppuccin::GREEN)
                .bg(Catppuccin::SURFACE_0),
        )
        .ratio(progress_ratio.clamp(0.0, 1.0));
    frame.render_widget(gauge, progress_layout[1]);

    frame.render_widget(
        Paragraph::new(format!("{} ", remaining_text))
            .style(Style::default().fg(Catppuccin::SUBTEXT_0))
            .alignment(Alignment::Left),
        progress_layout[2],
    );

    // Row 4: Volume
    let volume_line = Line::from(vec![
        Span::styled(format!(" {}{}", vol_icon, vol_bars), vol_style),
        Span::styled(
            format!("  {}%", volume),
            Style::default().fg(Catppuccin::SUBTEXT_0),
        ),
    ]);
    frame.render_widget(Paragraph::new(volume_line), rows[3]);
}

/// Compute the byte offset in a string for a given display-width offset
fn byte_offset(s: &str, display_offset: usize) -> usize {
    use unicode_width::UnicodeWidthStr;
    let mut width_so_far = 0usize;
    for (byte_idx, ch) in s.char_indices() {
        if width_so_far >= display_offset {
            return byte_idx;
        }
        width_so_far += UnicodeWidthStr::width(ch.to_string().as_str());
    }
    s.len()
}
