//! Catppuccin Mocha theme system
//!
//! Modern, vibrant, accessible color palette for the entire UI.
//! Dark theme only (light theme planned for future).

use ratatui::style::{Color, Modifier, Style};

/// Catppuccin Mocha color palette
/// https://github.com/catppuccin/catppuccin
pub struct Catppuccin;

impl Catppuccin {
    // ─── Base colors ───
    pub const BASE: Color = Color::Rgb(30, 30, 46);
    pub const MANTLE: Color = Color::Rgb(24, 24, 37);
    pub const CRUST: Color = Color::Rgb(17, 17, 27);

    // ─── Surface colors (panels, cards) ───
    pub const SURFACE_0: Color = Color::Rgb(49, 50, 68);
    pub const SURFACE_1: Color = Color::Rgb(69, 71, 90);
    pub const SURFACE_2: Color = Color::Rgb(88, 91, 112);

    // ─── Overlay colors (borders, separators) ───
    pub const OVERLAY_0: Color = Color::Rgb(108, 112, 134);
    pub const OVERLAY_1: Color = Color::Rgb(127, 132, 156);
    pub const OVERLAY_2: Color = Color::Rgb(147, 153, 178);

    // ─── Text colors ───
    pub const TEXT: Color = Color::Rgb(205, 214, 244);
    pub const SUBTEXT_0: Color = Color::Rgb(166, 173, 200);
    pub const SUBTEXT_1: Color = Color::Rgb(186, 194, 222);

    // ─── Accent colors ───
    pub const BLUE: Color = Color::Rgb(137, 180, 250);
    pub const SAPPHIRE: Color = Color::Rgb(116, 199, 236);
    pub const SKY: Color = Color::Rgb(137, 221, 255);
    pub const TEAL: Color = Color::Rgb(148, 226, 213);
    pub const GREEN: Color = Color::Rgb(166, 227, 161);
    pub const YELLOW: Color = Color::Rgb(249, 226, 175);
    pub const PEACH: Color = Color::Rgb(250, 179, 135);
    pub const MAROON: Color = Color::Rgb(235, 160, 172);
    pub const RED: Color = Color::Rgb(243, 139, 168);
    pub const MAUVE: Color = Color::Rgb(203, 166, 247);
    pub const PINK: Color = Color::Rgb(245, 194, 231);
    pub const FLAMINGO: Color = Color::Rgb(240, 198, 198);
    pub const ROSEWATER: Color = Color::Rgb(245, 224, 220);

    // ─── Semantic styles ───

    /// Primary accent (blue) - main actions, links, focus
    pub fn primary() -> Style {
        Style::default().fg(Self::BLUE)
    }

    /// Secondary accent (mauve) - secondary elements
    pub fn secondary() -> Style {
        Style::default().fg(Self::MAUVE)
    }

    /// Success state (green)
    pub fn success() -> Style {
        Style::default().fg(Self::GREEN)
    }

    /// Warning state (yellow)
    pub fn warning() -> Style {
        Style::default().fg(Self::YELLOW)
    }

    /// Error state (red)
    pub fn error() -> Style {
        Style::default().fg(Self::RED)
    }

    /// Info state (teal)
    pub fn info() -> Style {
        Style::default().fg(Self::TEAL)
    }

    /// Focused element (pink + bold)
    pub fn focused() -> Style {
        Style::default().fg(Self::PINK).add_modifier(Modifier::BOLD)
    }

    /// Selected element (inverted: crust on blue)
    pub fn selected() -> Style {
        Style::default()
            .fg(Self::BASE)
            .bg(Self::BLUE)
            .add_modifier(Modifier::BOLD)
    }

    /// Hover state (pink, underlined)
    pub fn hover() -> Style {
        Style::default()
            .fg(Self::PINK)
            .add_modifier(Modifier::UNDERLINED)
    }

    /// Dim/subtle text (subtext_0)
    pub fn dim() -> Style {
        Style::default().fg(Self::SUBTEXT_0)
    }

    /// Normal text
    pub fn text() -> Style {
        Style::default().fg(Self::TEXT)
    }

    /// Border style (surface_1)
    pub fn border() -> Style {
        Style::default().fg(Self::SURFACE_1)
    }

    /// Focused border (pink)
    pub fn border_focused() -> Style {
        Style::default().fg(Self::PINK)
    }

    // ─── Component styles ───

    /// Sidebar item (default state)
    pub fn sidebar_item() -> Style {
        Style::default().fg(Self::SUBTEXT_1)
    }

    /// Sidebar item (selected)
    pub fn sidebar_item_selected() -> Style {
        Style::default()
            .fg(Self::BASE)
            .bg(Self::BLUE)
            .add_modifier(Modifier::BOLD)
    }

    /// Sidebar item (hovered)
    pub fn sidebar_item_hovered() -> Style {
        Style::default()
            .fg(Self::PINK)
            .add_modifier(Modifier::UNDERLINED)
    }

    /// Track list item (default)
    pub fn track_item() -> Style {
        Style::default().fg(Self::TEXT)
    }

    /// Track list item (selected)
    pub fn track_item_selected() -> Style {
        Style::default()
            .fg(Self::BASE)
            .bg(Self::MAUVE)
            .add_modifier(Modifier::BOLD)
    }

    /// Track number/index (dim)
    pub fn track_number() -> Style {
        Style::default().fg(Self::OVERLAY_1)
    }

    /// Artist name in track list (teal)
    pub fn artist_name() -> Style {
        Style::default().fg(Self::TEAL)
    }

    /// Duration text (dim)
    pub fn duration() -> Style {
        Style::default().fg(Self::OVERLAY_0)
    }

    /// Progress bar color (blue to green gradient concept)
    pub fn progress() -> Style {
        Style::default().fg(Self::BLUE)
    }

    /// Volume indicator (green)
    pub fn volume() -> Style {
        Style::default().fg(Self::GREEN)
    }

    /// Status message (sky blue)
    pub fn status() -> Style {
        Style::default().fg(Self::SKY)
    }

    /// Loading spinner (yellow)
    pub fn loading() -> Style {
        Style::default().fg(Self::YELLOW)
    }

    /// Help text (overlay_1)
    pub fn help() -> Style {
        Style::default().fg(Self::OVERLAY_1)
    }

    /// Search input (rosewater)
    pub fn search_input() -> Style {
        Style::default().fg(Self::ROSEWATER)
    }

    /// Playback mode indicator (teal for local, peach for remote)
    pub fn playback_local() -> Style {
        Style::default().fg(Self::TEAL)
    }

    pub fn playback_remote() -> Style {
        Style::default().fg(Self::PEACH)
    }
}

/// Layout breakpoints
pub struct Layout;

impl Layout {
    pub fn is_compact(width: u16) -> bool {
        width < 60
    }

    pub fn is_medium(width: u16) -> bool {
        (60..100).contains(&width)
    }

    pub fn is_full(width: u16) -> bool {
        width >= 100
    }

    pub fn sidebar_width(width: u16) -> u16 {
        if Self::is_compact(width) {
            0
        } else if Self::is_medium(width) {
            20
        } else {
            28
        }
    }

    pub fn player_bar_height(height: u16) -> u16 {
        if Self::is_compact(height) {
            3
        } else {
            5
        }
    }
}

/// Unicode symbols for UI elements
pub mod symbols {
    // Playback
    pub const PLAY: &str = "▶";
    pub const PAUSE: &str = "⏸";
    pub const STOP: &str = "⏹";
    pub const SKIP_NEXT: &str = "⏭";
    pub const SKIP_PREV: &str = "⏮";

    // Volume
    pub const VOL_MUTE: &str = "🔇";
    pub const VOL_LOW: &str = "🔉";
    pub const VOL_HIGH: &str = "🔊";

    // Repeat/Shuffle
    pub const SHUFFLE: &str = "⇄";
    pub const REPEAT: &str = "↻";
    pub const REPEAT_ONE: &str = "↻¹";
    pub const RADIO: &str = "📻";

    // Status
    pub const ACTIVE: &str = "●";
    pub const INACTIVE: &str = "○";
    pub const LOADING: &str = "◐";
    pub const WARNING: &str = "⚠";
    pub const ERROR: &str = "✗";
    pub const SUCCESS: &str = "✓";
    pub const STAR: &str = "★";
    pub const STAR_EMPTY: &str = "☆";

    // Navigation
    pub const ARROW_RIGHT: &str = "→";
    pub const ARROW_LEFT: &str = "←";
    pub const ARROW_UP: &str = "↑";
    pub const ARROW_DOWN: &str = "↓";
    pub const CHEVRON: &str = "›";

    // Music
    pub const MUSIC: &str = "🎵";
    pub const MUSIC_NOTE: &str = "♪";
    pub const HEADPHONES: &str = "🎧";
    pub const DISC: &str = "💿";
    pub const MICROPHONE: &str = "🎤";
    pub const SPEAKER: &str = "🔊";

    // Devices
    pub const DEVICE_LOCAL: &str = "🔊";
    pub const DEVICE_REMOTE: &str = "📱";
    pub const DEVICE_COMPUTER: &str = "💻";
    pub const DEVICE_PHONE: &str = "📱";
    pub const DEVICE_SPEAKER: &str = "🔈";
    pub const DEVICE_TV: &str = "📺";
    pub const DEVICE_CAR: &str = "🚗";

    // UI
    pub const SEARCH: &str = "🔍";
    pub const SEARCH_PROMPT: &str = "/";
    pub const SETTINGS: &str = "⚙";
    pub const HELP: &str = "❓";
    pub const QUEUE: &str = "📋";
    pub const LIBRARY: &str = "📚";
    pub const HOME: &str = "🏠";
    pub const HEART: &str = "❤";
    pub const HEART_FILLED: &str = "❤️";

    // Loading spinner frames
    pub const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
}

/// Get current spinner frame based on time
pub fn spinner_frame() -> &'static str {
    let idx = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        / 100
        % symbols::SPINNER.len() as u128) as usize;
    symbols::SPINNER[idx]
}

/// Format a volume level (0-100) into a visual indicator
pub fn volume_indicator(volume: u32) -> (&'static str, Style) {
    match volume {
        0 => (symbols::VOL_MUTE, Catppuccin::dim()),
        1..=33 => (symbols::VOL_LOW, Catppuccin::info()),
        34..=66 => (symbols::VOL_HIGH, Catppuccin::volume()),
        _ => (symbols::VOL_HIGH, Catppuccin::success()),
    }
}

/// Format a play state into an icon
pub fn play_state_icon(is_playing: bool) -> &'static str {
    if is_playing {
        symbols::PLAY
    } else {
        symbols::PAUSE
    }
}

/// Get visual bars for volume level (0-100)
pub fn volume_bars(volume: u32) -> String {
    let filled = (volume as f32 / 100.0 * 10.0) as usize;
    let empty = 10 - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// Get visual bars for progress (0-100)
pub fn progress_bars(percent: u32) -> String {
    let filled = (percent as f32 / 100.0 * 20.0) as usize;
    let empty = 20 - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// Create a styled progress bar
pub fn styled_progress_bar(percent: u32, width: usize) -> String {
    let filled = (percent as f32 / 100.0 * width as f32) as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// Get gradient color for progress (blue to green)
pub fn progress_gradient_color(percent: u32) -> Color {
    // Interpolate between blue and green
    let t = percent as f32 / 100.0;
    Color::Rgb(
        (137.0 + (166.0 - 137.0) * t) as u8, // R: blue(137) to green(166)
        (180.0 + (227.0 - 180.0) * t) as u8, // G: blue(180) to green(227)
        (250.0 + (161.0 - 250.0) * t) as u8, // B: blue(250) to green(161)
    )
}

/// Format duration as mm:ss
pub fn format_duration(ms: u32) -> String {
    let seconds = ms / 1000;
    let minutes = seconds / 60;
    let secs = seconds % 60;
    format!("{}:{:02}", minutes, secs)
}

/// Create a horizontal separator line
pub fn separator(width: u16) -> String {
    "─".repeat(width as usize)
}

/// Create a styled separator with corner characters
pub fn styled_separator(width: u16) -> String {
    format!("╭{}╮", "─".repeat((width as usize).saturating_sub(2)))
}

/// Check if terminal supports true color
pub fn supports_true_color() -> bool {
    // Check COLORTERM environment variable
    std::env::var("COLORTERM")
        .map(|v| v == "truecolor" || v == "24bit")
        .unwrap_or(false)
}
