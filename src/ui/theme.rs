//! Dynamic theme system for Joshify
//!
//! Supports multiple color themes including:
//! - Catppuccin Mocha/Latte
//! - Gruvbox Dark/Light
//! - Nord
//! - Tokyo Night
//! - Dracula
//!
//! Themes can be switched at runtime using the 'T' key.

use ratatui::style::{Color, Modifier, Style};
use std::cell::RefCell;

thread_local! {
    static CURRENT_THEME: RefCell<CurrentTheme> = RefCell::new(CurrentTheme::default());
}

/// Current theme state
#[derive(Clone, Copy, Debug)]
struct CurrentTheme {
    theme: crate::themes::BuiltInTheme,
}

impl Default for CurrentTheme {
    fn default() -> Self {
        Self {
            theme: crate::themes::BuiltInTheme::CatppuccinMocha,
        }
    }
}

impl CurrentTheme {
    fn get_color(&self, color_type: ColorType) -> Color {
        let theme = self.theme.to_theme();
        
        match color_type {
            ColorType::Base => theme.background(),
            ColorType::Text => theme.foreground(),
            ColorType::Primary => theme.primary(),
            ColorType::Secondary => theme.secondary(),
            ColorType::Accent => theme.accent(),
            ColorType::Border => theme.border(),
            ColorType::SelectionBg => theme.selection_bg(),
            ColorType::SelectionFg => theme.selection_fg(),
            ColorType::Success => theme.status_playing(),
            ColorType::Error => theme.status_error(),
            ColorType::Warning => theme.secondary(), // Approximation
            ColorType::Muted => theme.muted(),
            ColorType::Info => theme.accent(),
            ColorType::Mauve => theme.secondary(), // Fallback
            ColorType::Pink => theme.accent(),
            ColorType::Green => theme.status_playing(),
            ColorType::Yellow => theme.secondary(),
            ColorType::Red => theme.status_error(),
            ColorType::Blue => theme.primary(),
            ColorType::Teal => theme.accent(),
            ColorType::Overlay0 => theme.border(),
            ColorType::Surface0 => theme.background(),
            ColorType::Surface1 => theme.border(),
            ColorType::Surface2 => theme.muted(),
        }
    }
}

/// Types of colors needed by the UI
/// Note: Some variants are kept for future use and API completeness
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
enum ColorType {
    Base, Text, Primary, Secondary, Accent, Border,
    SelectionBg, SelectionFg, Success, Error, Warning, Muted, Info,
    Mauve, Pink, Green, Yellow, Red, Blue, Teal, Overlay0,
    Surface0, Surface1, Surface2,
}

/// Set the current theme globally
pub fn set_current_theme(theme: crate::themes::BuiltInTheme) {
    CURRENT_THEME.with(|t| {
        *t.borrow_mut() = CurrentTheme { theme };
    });
}

/// Get the current theme name
pub fn current_theme_name() -> String {
    CURRENT_THEME.with(|t| {
        use crate::themes::BuiltInTheme;
        let theme = t.borrow().theme;
        match theme {
            BuiltInTheme::CatppuccinMocha => "Catppuccin Mocha",
            BuiltInTheme::CatppuccinLatte => "Catppuccin Latte",
            BuiltInTheme::GruvboxDark => "Gruvbox Dark",
            BuiltInTheme::GruvboxLight => "Gruvbox Light",
            BuiltInTheme::Nord => "Nord",
            BuiltInTheme::TokyoNight => "Tokyo Night",
            BuiltInTheme::Dracula => "Dracula",
        }.to_string()
    })
}

/// Catppuccin Mocha theme - DEPRECATED: Use dynamic theme functions instead
/// This struct is kept for backward compatibility but now delegates to the current theme
pub struct Catppuccin;

impl Catppuccin {
    // ─── Legacy constants (deprecated, kept for compatibility) ───
    // These are the Catppuccin Mocha values as fallback
    pub const BASE: Color = Color::Rgb(30, 30, 46);
    pub const MANTLE: Color = Color::Rgb(24, 24, 37);
    pub const CRUST: Color = Color::Rgb(17, 17, 27);
    pub const SURFACE_0: Color = Color::Rgb(49, 50, 68);
    pub const SURFACE_1: Color = Color::Rgb(69, 71, 90);
    pub const SURFACE_2: Color = Color::Rgb(88, 91, 112);
    pub const OVERLAY_0: Color = Color::Rgb(108, 112, 134);
    pub const OVERLAY_1: Color = Color::Rgb(127, 132, 156);
    pub const OVERLAY_2: Color = Color::Rgb(147, 153, 178);
    pub const TEXT: Color = Color::Rgb(205, 214, 244);
    pub const SUBTEXT_0: Color = Color::Rgb(166, 173, 200);
    pub const SUBTEXT_1: Color = Color::Rgb(186, 194, 222);
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

    // Helper to get current theme color
    fn get_color(color_type: ColorType) -> Color {
        CURRENT_THEME.with(|t| t.borrow().get_color(color_type))
    }

    // ─── Semantic styles (now dynamic based on current theme) ───

    /// Primary accent - main actions, links, focus
    pub fn primary() -> Style {
        Style::default().fg(Self::get_color(ColorType::Primary))
    }

    /// Secondary accent - secondary elements
    pub fn secondary() -> Style {
        Style::default().fg(Self::get_color(ColorType::Secondary))
    }

    /// Success state
    pub fn success() -> Style {
        Style::default().fg(Self::get_color(ColorType::Success))
    }

    /// Warning state
    pub fn warning() -> Style {
        Style::default().fg(Self::get_color(ColorType::Warning))
    }

    /// Error state
    pub fn error() -> Style {
        Style::default().fg(Self::get_color(ColorType::Error))
    }

    /// Info state
    pub fn info() -> Style {
        Style::default().fg(Self::get_color(ColorType::Info))
    }

    /// Focused element
    pub fn focused() -> Style {
        Style::default()
            .fg(Self::get_color(ColorType::Accent))
            .add_modifier(Modifier::BOLD)
    }

    /// Selected element (inverted: bg on fg)
    pub fn selected() -> Style {
        Style::default()
            .fg(Self::get_color(ColorType::Base))
            .bg(Self::get_color(ColorType::Primary))
            .add_modifier(Modifier::BOLD)
    }

    /// Hover state
    pub fn hover() -> Style {
        Style::default()
            .fg(Self::get_color(ColorType::Accent))
            .add_modifier(Modifier::UNDERLINED)
    }

    /// Dim/subtle text
    pub fn dim() -> Style {
        Style::default().fg(Self::get_color(ColorType::Muted))
    }

    /// Normal text
    pub fn text() -> Style {
        Style::default().fg(Self::get_color(ColorType::Text))
    }

    /// Border style
    pub fn border() -> Style {
        Style::default().fg(Self::get_color(ColorType::Border))
    }

    /// Focused border
    pub fn border_focused() -> Style {
        Style::default().fg(Self::get_color(ColorType::Accent))
    }

    // ─── Component styles ───

    /// Sidebar item (default state)
    pub fn sidebar_item() -> Style {
        Style::default().fg(Self::get_color(ColorType::Muted))
    }

    /// Sidebar item (selected)
    pub fn sidebar_item_selected() -> Style {
        Style::default()
            .fg(Self::get_color(ColorType::Base))
            .bg(Self::get_color(ColorType::Primary))
            .add_modifier(Modifier::BOLD)
    }

    /// Sidebar item (hovered)
    pub fn sidebar_item_hovered() -> Style {
        Style::default()
            .fg(Self::get_color(ColorType::Accent))
            .add_modifier(Modifier::UNDERLINED)
    }

    /// Track list item (default)
    pub fn track_item() -> Style {
        Style::default().fg(Self::get_color(ColorType::Text))
    }

    /// Track list item (selected)
    pub fn track_item_selected() -> Style {
        Style::default()
            .fg(Self::get_color(ColorType::Base))
            .bg(Self::get_color(ColorType::Secondary))
            .add_modifier(Modifier::BOLD)
    }

    /// Track number/index (dim)
    pub fn track_number() -> Style {
        Style::default().fg(Self::get_color(ColorType::Muted))
    }

    /// Artist name in track list
    pub fn artist_name() -> Style {
        Style::default().fg(Self::get_color(ColorType::Accent))
    }

    /// Duration text (dim)
    pub fn duration() -> Style {
        Style::default().fg(Self::get_color(ColorType::Muted))
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
