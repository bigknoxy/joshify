//! Player bar rendering - Now playing with album art, scrolling title, progress bar

use crate::state::player_state::{RepeatMode, TitleScrollState};
use crate::ui::layout_cache::LayoutCache;
use crate::ui::theme::{self, symbols, Catppuccin};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Gauge, Paragraph},
};

/// Button dimensions for hit testing
const BUTTON_HEIGHT: u16 = 1;
const PREV_BUTTON_WIDTH: u16 = 6;
const PLAY_PAUSE_BUTTON_WIDTH: u16 = 10;
const NEXT_BUTTON_WIDTH: u16 = 6;
const SHUFFLE_BUTTON_WIDTH: u16 = 8;
const REPEAT_BUTTON_WIDTH: u16 = 8;
const QUEUE_BUTTON_WIDTH: u16 = 8;

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
    layout_cache: &mut LayoutCache,
) {
    // Store player bar area in cache
    layout_cache.player_bar = Some(area);

    // Clear player control areas (will be populated as we render)
    layout_cache.prev_button = None;
    layout_cache.play_pause_button = None;
    layout_cache.next_button = None;
    layout_cache.shuffle_button = None;
    layout_cache.repeat_button = None;
    layout_cache.queue_button = None;
    layout_cache.progress_bar = None;
    layout_cache.volume_bar = None;

    let (play_icon, play_text) = if is_playing {
        (symbols::PAUSE, "Pause")
    } else {
        (symbols::PLAY, "Play")
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

    // Row 1: Scrolling title with playback controls
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
    // vol_bars is now computed in the rendering section below

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

    // Row 1: Title with playback controls
    // Layout: [<< prev][ play/pause ][next >>][title text...]
    let title_row_width = rows[0].width as usize;
    let controls_width =
        PREV_BUTTON_WIDTH as usize + PLAY_PAUSE_BUTTON_WIDTH as usize + NEXT_BUTTON_WIDTH as usize;
    let title_available = title_row_width.saturating_sub(controls_width);

    let (truncated_title, _) = unicode_truncate::UnicodeTruncateStr::unicode_truncate(
        title_text.as_str(),
        title_available,
    );

    // Calculate button positions in title row
    let title_row_x = rows[0].x;
    let title_row_y = rows[0].y;

    // Previous button at far left
    let prev_x = title_row_x.saturating_add(1); // +1 for left border padding
    layout_cache.prev_button = Some(Rect::new(
        prev_x,
        title_row_y,
        PREV_BUTTON_WIDTH,
        BUTTON_HEIGHT,
    ));

    // Play/Pause button after prev
    let play_x = prev_x.saturating_add(PREV_BUTTON_WIDTH);
    layout_cache.play_pause_button = Some(Rect::new(
        play_x,
        title_row_y,
        PLAY_PAUSE_BUTTON_WIDTH,
        BUTTON_HEIGHT,
    ));

    // Next button after play/pause
    let next_x = play_x.saturating_add(PLAY_PAUSE_BUTTON_WIDTH);
    layout_cache.next_button = Some(Rect::new(
        next_x,
        title_row_y,
        NEXT_BUTTON_WIDTH,
        BUTTON_HEIGHT,
    ));

    // Render the title row with controls
    let title_line = Line::from(vec![
        Span::styled(
            format!("{:<width$}", "<<", width = PREV_BUTTON_WIDTH as usize),
            Style::default()
                .fg(Catppuccin::BLUE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "{:^width$}",
                format!("{} {}", play_icon, play_text),
                width = PLAY_PAUSE_BUTTON_WIDTH.saturating_sub(1) as usize
            ),
            Style::default()
                .fg(Catppuccin::MAUVE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:<width$}", ">>", width = NEXT_BUTTON_WIDTH as usize),
            Style::default()
                .fg(Catppuccin::BLUE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {}", truncated_title),
            Style::default()
                .fg(Catppuccin::MAUVE)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(title_line), rows[0]);

    // Row 2: Artist + badges + hints
    // Shuffle and repeat badges are clickable
    let artist_row_x = rows[1].x;
    let artist_row_y = rows[1].y;

    // Calculate badge positions
    // Artist text starts at position 2 (after "  ")
    let artist_start = artist_row_x.saturating_add(2);
    let artist_end = artist_start
        .saturating_add(unicode_width::UnicodeWidthStr::width(artist_text.as_str()) as u16);

    // Shuffle badge position (after artist, with space)
    let shuffle_x = artist_end.saturating_add(1);
    layout_cache.shuffle_button = Some(Rect::new(
        shuffle_x,
        artist_row_y,
        SHUFFLE_BUTTON_WIDTH,
        BUTTON_HEIGHT,
    ));

    // Repeat badge position (after shuffle)
    let repeat_x = shuffle_x.saturating_add(SHUFFLE_BUTTON_WIDTH);
    layout_cache.repeat_button = Some(Rect::new(
        repeat_x,
        artist_row_y,
        REPEAT_BUTTON_WIDTH,
        BUTTON_HEIGHT,
    ));

    // Queue badge position (after repeat, before hints)
    let queue_x = repeat_x.saturating_add(REPEAT_BUTTON_WIDTH);
    layout_cache.queue_button = Some(Rect::new(
        queue_x,
        artist_row_y,
        QUEUE_BUTTON_WIDTH,
        BUTTON_HEIGHT,
    ));

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

    // Store progress bar area for hit testing (seek functionality)
    layout_cache.progress_bar = Some(progress_layout[1]);

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

    // Row 4: Volume bar with enhanced visual indicator
    // Store volume bar area for hit testing
    let volume_row_width = rows[3].width;
    let volume_bar_x = rows[3].x.saturating_add(2);
    let volume_bar_y = rows[3].y;
    layout_cache.volume_bar = Some(Rect::new(
        volume_bar_x,
        volume_bar_y,
        volume_row_width.saturating_sub(4),
        BUTTON_HEIGHT,
    ));

    // Create a more visual volume bar
    let vol_visual = theme::volume_bars(volume);
    let vol_color = if volume > 80 {
        Catppuccin::GREEN
    } else if volume > 50 {
        Catppuccin::TEAL
    } else if volume > 20 {
        Catppuccin::YELLOW
    } else if volume > 0 {
        Catppuccin::PEACH
    } else {
        Catppuccin::SURFACE_1
    };

    let volume_line = Line::from(vec![
        Span::styled(format!(" {} ", vol_icon), vol_style),
        Span::styled(format!("[{}]", vol_visual), Style::default().fg(vol_color)),
        Span::styled(
            format!(" {:>3}%", volume),
            Style::default().fg(Catppuccin::SUBTEXT_0),
        ),
        Span::styled("  +/- : adjust", Style::default().fg(Catppuccin::SURFACE_2)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::layout_cache::ClickableArea;

    #[test]
    fn test_button_dimensions_are_reasonable() {
        // Verify button dimension constants are defined
        assert_eq!(BUTTON_HEIGHT, 1);
        assert_eq!(PREV_BUTTON_WIDTH, 6);
        assert_eq!(PLAY_PAUSE_BUTTON_WIDTH, 10);
        assert_eq!(NEXT_BUTTON_WIDTH, 6);
        assert_eq!(SHUFFLE_BUTTON_WIDTH, 8);
        assert_eq!(REPEAT_BUTTON_WIDTH, 8);
        assert_eq!(QUEUE_BUTTON_WIDTH, 8);
    }

    #[test]
    fn test_cache_player_bar_field_exists() {
        let mut cache = LayoutCache::new();
        let area = Rect::new(0, 0, 80, 6);
        cache.player_bar = Some(area);
        assert_eq!(cache.player_bar, Some(area));
    }

    #[test]
    fn test_cache_button_fields_exist() {
        let mut cache = LayoutCache::new();
        let btn_area = Rect::new(10, 10, 8, 1);

        cache.prev_button = Some(btn_area);
        cache.play_pause_button = Some(btn_area);
        cache.next_button = Some(btn_area);
        cache.shuffle_button = Some(btn_area);
        cache.repeat_button = Some(btn_area);
        cache.queue_button = Some(btn_area);
        cache.progress_bar = Some(btn_area);
        cache.volume_bar = Some(btn_area);

        assert!(cache.prev_button.is_some());
        assert!(cache.play_pause_button.is_some());
        assert!(cache.next_button.is_some());
        assert!(cache.shuffle_button.is_some());
        assert!(cache.repeat_button.is_some());
        assert!(cache.queue_button.is_some());
        assert!(cache.progress_bar.is_some());
        assert!(cache.volume_bar.is_some());
    }

    #[test]
    fn test_cache_clear_resets_player_controls() {
        let mut cache = LayoutCache {
            player_bar: Some(Rect::new(0, 0, 80, 6)),
            prev_button: Some(Rect::new(10, 1, 6, 1)),
            play_pause_button: Some(Rect::new(16, 1, 10, 1)),
            next_button: Some(Rect::new(26, 1, 6, 1)),
            shuffle_button: Some(Rect::new(10, 2, 8, 1)),
            repeat_button: Some(Rect::new(18, 2, 8, 1)),
            queue_button: Some(Rect::new(26, 2, 8, 1)),
            progress_bar: Some(Rect::new(10, 3, 50, 1)),
            volume_bar: Some(Rect::new(10, 4, 15, 1)),
            ..Default::default()
        };

        cache.clear();

        assert!(cache.player_bar.is_none());
        assert!(cache.prev_button.is_none());
        assert!(cache.play_pause_button.is_none());
        assert!(cache.next_button.is_none());
        assert!(cache.shuffle_button.is_none());
        assert!(cache.repeat_button.is_none());
        assert!(cache.queue_button.is_none());
        assert!(cache.progress_bar.is_none());
        assert!(cache.volume_bar.is_none());
    }

    #[test]
    fn test_clickable_area_enum_has_player_controls() {
        // Verify ClickableArea enum has all player control variants
        let _prev = ClickableArea::PrevButton;
        let _play = ClickableArea::PlayPauseButton;
        let _next = ClickableArea::NextButton;
        let _progress = ClickableArea::ProgressBar;
        let _volume = ClickableArea::VolumeBar;
        let _shuffle = ClickableArea::ShuffleButton;
        let _repeat = ClickableArea::RepeatButton;
        let _queue = ClickableArea::QueueButton;
    }

    #[test]
    fn test_area_at_finds_player_buttons() {
        let cache = LayoutCache {
            player_bar: Some(Rect::new(20, 34, 60, 6)),
            prev_button: Some(Rect::new(25, 35, 6, 1)),
            play_pause_button: Some(Rect::new(31, 35, 10, 1)),
            next_button: Some(Rect::new(41, 35, 6, 1)),
            shuffle_button: Some(Rect::new(25, 36, 8, 1)),
            repeat_button: Some(Rect::new(33, 36, 8, 1)),
            queue_button: Some(Rect::new(41, 36, 8, 1)),
            progress_bar: Some(Rect::new(25, 37, 40, 1)),
            volume_bar: Some(Rect::new(25, 38, 15, 1)),
            ..Default::default()
        };

        // Test each button is found at its position
        assert_eq!(cache.area_at(27, 35), Some(ClickableArea::PrevButton));
        assert_eq!(cache.area_at(35, 35), Some(ClickableArea::PlayPauseButton));
        assert_eq!(cache.area_at(43, 35), Some(ClickableArea::NextButton));
        assert_eq!(cache.area_at(27, 36), Some(ClickableArea::ShuffleButton));
        assert_eq!(cache.area_at(35, 36), Some(ClickableArea::RepeatButton));
        assert_eq!(cache.area_at(43, 36), Some(ClickableArea::QueueButton));
        assert_eq!(cache.area_at(35, 37), Some(ClickableArea::ProgressBar));
        assert_eq!(cache.area_at(30, 38), Some(ClickableArea::VolumeBar));
    }

    #[test]
    fn test_specific_button_takes_precedence_over_player_bar() {
        let cache = LayoutCache {
            player_bar: Some(Rect::new(20, 34, 60, 6)),
            play_pause_button: Some(Rect::new(25, 35, 10, 1)),
            ..Default::default()
        };

        // Click on play button should return PlayPauseButton, not PlayerBar
        assert_eq!(cache.area_at(30, 35), Some(ClickableArea::PlayPauseButton));
    }

    #[test]
    fn test_saturating_add_prevents_overflow() {
        // Test that saturating math is used correctly
        let small_x: u16 = 1;
        let width: u16 = 6;
        let result = small_x.saturating_add(width);
        assert_eq!(result, 7);

        // Test with max value - should not overflow
        let max_x: u16 = u16::MAX;
        let overflow_result = max_x.saturating_add(1);
        assert_eq!(overflow_result, u16::MAX); // Saturates, doesn't wrap
    }

    #[test]
    fn test_saturating_sub_prevents_underflow() {
        // Test that saturating_sub prevents underflow
        let zero: u16 = 0;
        let result = zero.saturating_sub(10);
        assert_eq!(result, 0); // Saturates at 0, doesn't wrap to u16::MAX
    }
}
