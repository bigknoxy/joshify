//! Theme system for Joshify
//!
//! Provides multiple color themes including:
//! - Catppuccin Mocha/Latte
//! - Gruvbox Dark/Light
//! - Nord
//! - Tokyo Night
//! - Dracula

use ratatui::style::Color;

/// Theme trait defining color interface
pub trait Theme {
    /// Theme name
    fn name(&self) -> &str;

    /// Background color
    fn background(&self) -> Color;
    /// Foreground/text color
    fn foreground(&self) -> Color;
    /// Accent/highlight color
    fn accent(&self) -> Color;
    /// Border color
    fn border(&self) -> Color;
    /// Selection background
    fn selection_bg(&self) -> Color;
    /// Selection foreground
    fn selection_fg(&self) -> Color;
    /// Status: playing
    fn status_playing(&self) -> Color;
    /// Status: paused
    fn status_paused(&self) -> Color;
    /// Status: error
    fn status_error(&self) -> Color;
    /// Primary color
    fn primary(&self) -> Color;
    /// Secondary color
    fn secondary(&self) -> Color;
    /// Muted/subtle color
    fn muted(&self) -> Color;
}

/// Built-in themes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltInTheme {
    CatppuccinMocha,
    CatppuccinLatte,
    GruvboxDark,
    GruvboxLight,
    Nord,
    TokyoNight,
    Dracula,
}

impl BuiltInTheme {
    /// Get all available theme names
    pub fn all_names() -> Vec<&'static str> {
        vec![
            "catppuccin_mocha",
            "catppuccin_latte",
            "gruvbox_dark",
            "gruvbox_light",
            "nord",
            "tokyo_night",
            "dracula",
        ]
    }

    /// Get theme by name
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "catppuccin_mocha" => Some(Self::CatppuccinMocha),
            "catppuccin_latte" => Some(Self::CatppuccinLatte),
            "gruvbox_dark" => Some(Self::GruvboxDark),
            "gruvbox_light" => Some(Self::GruvboxLight),
            "nord" => Some(Self::Nord),
            "tokyo_night" => Some(Self::TokyoNight),
            "dracula" => Some(Self::Dracula),
            _ => None,
        }
    }

    /// Get theme name as string
    pub fn as_name(&self) -> &'static str {
        match self {
            Self::CatppuccinMocha => "catppuccin_mocha",
            Self::CatppuccinLatte => "catppuccin_latte",
            Self::GruvboxDark => "gruvbox_dark",
            Self::GruvboxLight => "gruvbox_light",
            Self::Nord => "nord",
            Self::TokyoNight => "tokyo_night",
            Self::Dracula => "dracula",
        }
    }

    /// Convert to theme implementation
    pub fn to_theme(&self) -> Box<dyn Theme> {
        match self {
            Self::CatppuccinMocha => Box::new(CatppuccinMocha),
            Self::CatppuccinLatte => Box::new(CatppuccinLatte),
            Self::GruvboxDark => Box::new(GruvboxDark),
            Self::GruvboxLight => Box::new(GruvboxLight),
            Self::Nord => Box::new(Nord),
            Self::TokyoNight => Box::new(TokyoNight),
            Self::Dracula => Box::new(Dracula),
        }
    }
}

impl Default for BuiltInTheme {
    fn default() -> Self {
        Self::CatppuccinMocha
    }
}

/// Catppuccin Mocha theme (default)
pub struct CatppuccinMocha;

impl Theme for CatppuccinMocha {
    fn name(&self) -> &str {
        "Catppuccin Mocha"
    }
    fn background(&self) -> Color {
        Color::Rgb(30, 30, 46)
    }
    fn foreground(&self) -> Color {
        Color::Rgb(205, 214, 244)
    }
    fn accent(&self) -> Color {
        Color::Rgb(203, 166, 247)
    }
    fn border(&self) -> Color {
        Color::Rgb(88, 91, 112)
    }
    fn selection_bg(&self) -> Color {
        Color::Rgb(69, 71, 90)
    }
    fn selection_fg(&self) -> Color {
        Color::Rgb(205, 214, 244)
    }
    fn status_playing(&self) -> Color {
        Color::Rgb(166, 227, 161)
    }
    fn status_paused(&self) -> Color {
        Color::Rgb(243, 139, 168)
    }
    fn status_error(&self) -> Color {
        Color::Rgb(243, 139, 168)
    }
    fn primary(&self) -> Color {
        Color::Rgb(137, 180, 250)
    }
    fn secondary(&self) -> Color {
        Color::Rgb(250, 179, 135)
    }
    fn muted(&self) -> Color {
        Color::Rgb(108, 112, 134)
    }
}

/// Catppuccin Latte theme
pub struct CatppuccinLatte;

impl Theme for CatppuccinLatte {
    fn name(&self) -> &str {
        "Catppuccin Latte"
    }
    fn background(&self) -> Color {
        Color::Rgb(239, 241, 245)
    }
    fn foreground(&self) -> Color {
        Color::Rgb(76, 79, 105)
    }
    fn accent(&self) -> Color {
        Color::Rgb(136, 57, 239)
    }
    fn border(&self) -> Color {
        Color::Rgb(184, 192, 224)
    }
    fn selection_bg(&self) -> Color {
        Color::Rgb(204, 208, 218)
    }
    fn selection_fg(&self) -> Color {
        Color::Rgb(76, 79, 105)
    }
    fn status_playing(&self) -> Color {
        Color::Rgb(64, 160, 43)
    }
    fn status_paused(&self) -> Color {
        Color::Rgb(210, 15, 57)
    }
    fn status_error(&self) -> Color {
        Color::Rgb(210, 15, 57)
    }
    fn primary(&self) -> Color {
        Color::Rgb(30, 102, 245)
    }
    fn secondary(&self) -> Color {
        Color::Rgb(223, 142, 29)
    }
    fn muted(&self) -> Color {
        Color::Rgb(140, 143, 161)
    }
}

/// Gruvbox Dark theme
pub struct GruvboxDark;

impl Theme for GruvboxDark {
    fn name(&self) -> &str {
        "Gruvbox Dark"
    }
    fn background(&self) -> Color {
        Color::Rgb(40, 40, 40)
    }
    fn foreground(&self) -> Color {
        Color::Rgb(251, 241, 199)
    }
    fn accent(&self) -> Color {
        Color::Rgb(184, 187, 38)
    }
    fn border(&self) -> Color {
        Color::Rgb(102, 92, 84)
    }
    fn selection_bg(&self) -> Color {
        Color::Rgb(80, 73, 69)
    }
    fn selection_fg(&self) -> Color {
        Color::Rgb(251, 241, 199)
    }
    fn status_playing(&self) -> Color {
        Color::Rgb(184, 187, 38)
    }
    fn status_paused(&self) -> Color {
        Color::Rgb(251, 73, 52)
    }
    fn status_error(&self) -> Color {
        Color::Rgb(251, 73, 52)
    }
    fn primary(&self) -> Color {
        Color::Rgb(131, 165, 152)
    }
    fn secondary(&self) -> Color {
        Color::Rgb(254, 128, 25)
    }
    fn muted(&self) -> Color {
        Color::Rgb(146, 131, 116)
    }
}

/// Gruvbox Light theme
pub struct GruvboxLight;

impl Theme for GruvboxLight {
    fn name(&self) -> &str {
        "Gruvbox Light"
    }
    fn background(&self) -> Color {
        Color::Rgb(251, 241, 199)
    }
    fn foreground(&self) -> Color {
        Color::Rgb(60, 56, 54)
    }
    fn accent(&self) -> Color {
        Color::Rgb(121, 116, 14)
    }
    fn border(&self) -> Color {
        Color::Rgb(189, 174, 147)
    }
    fn selection_bg(&self) -> Color {
        Color::Rgb(213, 196, 161)
    }
    fn selection_fg(&self) -> Color {
        Color::Rgb(60, 56, 54)
    }
    fn status_playing(&self) -> Color {
        Color::Rgb(121, 116, 14)
    }
    fn status_paused(&self) -> Color {
        Color::Rgb(157, 0, 6)
    }
    fn status_error(&self) -> Color {
        Color::Rgb(157, 0, 6)
    }
    fn primary(&self) -> Color {
        Color::Rgb(66, 123, 88)
    }
    fn secondary(&self) -> Color {
        Color::Rgb(175, 58, 3)
    }
    fn muted(&self) -> Color {
        Color::Rgb(124, 111, 100)
    }
}

/// Nord theme
pub struct Nord;

impl Theme for Nord {
    fn name(&self) -> &str {
        "Nord"
    }
    fn background(&self) -> Color {
        Color::Rgb(46, 52, 64)
    }
    fn foreground(&self) -> Color {
        Color::Rgb(216, 222, 233)
    }
    fn accent(&self) -> Color {
        Color::Rgb(136, 192, 208)
    }
    fn border(&self) -> Color {
        Color::Rgb(76, 86, 106)
    }
    fn selection_bg(&self) -> Color {
        Color::Rgb(67, 76, 94)
    }
    fn selection_fg(&self) -> Color {
        Color::Rgb(216, 222, 233)
    }
    fn status_playing(&self) -> Color {
        Color::Rgb(163, 190, 140)
    }
    fn status_paused(&self) -> Color {
        Color::Rgb(191, 97, 106)
    }
    fn status_error(&self) -> Color {
        Color::Rgb(191, 97, 106)
    }
    fn primary(&self) -> Color {
        Color::Rgb(129, 161, 193)
    }
    fn secondary(&self) -> Color {
        Color::Rgb(235, 203, 139)
    }
    fn muted(&self) -> Color {
        Color::Rgb(143, 188, 187)
    }
}

/// Tokyo Night theme
pub struct TokyoNight;

impl Theme for TokyoNight {
    fn name(&self) -> &str {
        "Tokyo Night"
    }
    fn background(&self) -> Color {
        Color::Rgb(26, 27, 38)
    }
    fn foreground(&self) -> Color {
        Color::Rgb(192, 202, 245)
    }
    fn accent(&self) -> Color {
        Color::Rgb(187, 154, 247)
    }
    fn border(&self) -> Color {
        Color::Rgb(65, 72, 104)
    }
    fn selection_bg(&self) -> Color {
        Color::Rgb(41, 46, 66)
    }
    fn selection_fg(&self) -> Color {
        Color::Rgb(192, 202, 245)
    }
    fn status_playing(&self) -> Color {
        Color::Rgb(158, 206, 106)
    }
    fn status_paused(&self) -> Color {
        Color::Rgb(247, 118, 142)
    }
    fn status_error(&self) -> Color {
        Color::Rgb(247, 118, 142)
    }
    fn primary(&self) -> Color {
        Color::Rgb(122, 162, 247)
    }
    fn secondary(&self) -> Color {
        Color::Rgb(255, 158, 100)
    }
    fn muted(&self) -> Color {
        Color::Rgb(86, 95, 137)
    }
}

/// Dracula theme
pub struct Dracula;

impl Theme for Dracula {
    fn name(&self) -> &str {
        "Dracula"
    }
    fn background(&self) -> Color {
        Color::Rgb(40, 42, 54)
    }
    fn foreground(&self) -> Color {
        Color::Rgb(248, 248, 242)
    }
    fn accent(&self) -> Color {
        Color::Rgb(189, 147, 249)
    }
    fn border(&self) -> Color {
        Color::Rgb(68, 71, 90)
    }
    fn selection_bg(&self) -> Color {
        Color::Rgb(68, 71, 90)
    }
    fn selection_fg(&self) -> Color {
        Color::Rgb(255, 121, 198)
    }
    fn status_playing(&self) -> Color {
        Color::Rgb(80, 250, 123)
    }
    fn status_paused(&self) -> Color {
        Color::Rgb(255, 85, 85)
    }
    fn status_error(&self) -> Color {
        Color::Rgb(255, 85, 85)
    }
    fn primary(&self) -> Color {
        Color::Rgb(139, 233, 253)
    }
    fn secondary(&self) -> Color {
        Color::Rgb(241, 250, 140)
    }
    fn muted(&self) -> Color {
        Color::Rgb(98, 114, 164)
    }
}

/// Theme registry for managing available themes
pub struct ThemeRegistry {
    current: BuiltInTheme,
}

impl ThemeRegistry {
    /// Create new registry with default theme
    pub fn new() -> Self {
        Self {
            current: BuiltInTheme::default(),
        }
    }

    /// Create with specific theme
    pub fn with_theme(theme: BuiltInTheme) -> Self {
        Self { current: theme }
    }

    /// Get current theme
    pub fn current(&self) -> BuiltInTheme {
        self.current
    }

    /// Get current theme implementation
    pub fn current_theme(&self) -> Box<dyn Theme> {
        self.current.to_theme()
    }

    /// Switch theme
    pub fn switch_theme(&mut self, theme: BuiltInTheme) {
        self.current = theme;
    }

    /// Switch theme by name
    pub fn switch_theme_by_name(&mut self, name: &str) -> Result<(), ThemeError> {
        match BuiltInTheme::from_name(name) {
            Some(theme) => {
                self.current = theme;
                Ok(())
            }
            None => Err(ThemeError::UnknownTheme(name.to_string())),
        }
    }

    /// Get list of available themes
    pub fn available_themes(&self) -> Vec<&'static str> {
        BuiltInTheme::all_names()
    }

    /// Get theme by name
    pub fn get_theme(&self, name: &str) -> Option<Box<dyn Theme>> {
        BuiltInTheme::from_name(name).map(|t| t.to_theme())
    }
}

impl Default for ThemeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Theme errors
#[derive(Debug, Clone, PartialEq)]
pub enum ThemeError {
    UnknownTheme(String),
}

impl std::fmt::Display for ThemeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTheme(name) => write!(f, "Unknown theme: {}", name),
        }
    }
}

impl std::error::Error for ThemeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_theme_names() {
        let names = BuiltInTheme::all_names();
        assert_eq!(names.len(), 7);
        assert!(names.contains(&"catppuccin_mocha"));
        assert!(names.contains(&"nord"));
        assert!(names.contains(&"dracula"));
    }

    #[test]
    fn test_theme_from_name() {
        assert_eq!(
            BuiltInTheme::from_name("catppuccin_mocha"),
            Some(BuiltInTheme::CatppuccinMocha)
        );
        assert_eq!(BuiltInTheme::from_name("nord"), Some(BuiltInTheme::Nord));
        assert_eq!(BuiltInTheme::from_name("unknown"), None);
    }

    #[test]
    fn test_theme_colors() {
        let theme = CatppuccinMocha;
        assert_eq!(theme.name(), "Catppuccin Mocha");
        assert_eq!(theme.background(), Color::Rgb(30, 30, 46));
        assert_eq!(theme.foreground(), Color::Rgb(205, 214, 244));
    }

    #[test]
    fn test_theme_registry() {
        let mut registry = ThemeRegistry::new();
        assert_eq!(registry.current(), BuiltInTheme::CatppuccinMocha);

        registry.switch_theme(BuiltInTheme::Nord);
        assert_eq!(registry.current(), BuiltInTheme::Nord);
    }

    #[test]
    fn test_theme_registry_by_name() {
        let mut registry = ThemeRegistry::new();

        assert!(registry.switch_theme_by_name("gruvbox_dark").is_ok());
        assert_eq!(registry.current(), BuiltInTheme::GruvboxDark);

        assert!(registry.switch_theme_by_name("unknown").is_err());
    }

    #[test]
    fn test_get_theme() {
        let registry = ThemeRegistry::new();

        let theme = registry.get_theme("tokyo_night");
        assert!(theme.is_some());
        assert_eq!(theme.unwrap().name(), "Tokyo Night");

        assert!(registry.get_theme("unknown").is_none());
    }

    #[test]
    fn test_theme_error_display() {
        let err = ThemeError::UnknownTheme("test".to_string());
        assert_eq!(err.to_string(), "Unknown theme: test");
    }

    #[test]
    fn test_all_themes() {
        let themes = vec![
            BuiltInTheme::CatppuccinMocha,
            BuiltInTheme::CatppuccinLatte,
            BuiltInTheme::GruvboxDark,
            BuiltInTheme::GruvboxLight,
            BuiltInTheme::Nord,
            BuiltInTheme::TokyoNight,
            BuiltInTheme::Dracula,
        ];

        for theme in themes {
            let t = theme.to_theme();
            // Verify all colors are accessible
            t.background();
            t.foreground();
            t.accent();
            t.border();
            t.selection_bg();
            t.selection_fg();
            t.status_playing();
            t.status_paused();
            t.status_error();
            t.primary();
            t.secondary();
            t.muted();
        }
    }

    #[test]
    fn test_theme_as_name() {
        assert_eq!(BuiltInTheme::CatppuccinMocha.as_name(), "catppuccin_mocha");
        assert_eq!(BuiltInTheme::Dracula.as_name(), "dracula");
    }

    #[test]
    fn test_default_theme() {
        let default: BuiltInTheme = Default::default();
        assert_eq!(default, BuiltInTheme::CatppuccinMocha);
    }

    #[test]
    fn test_theme_registry_default() {
        let registry: ThemeRegistry = Default::default();
        assert_eq!(registry.current(), BuiltInTheme::CatppuccinMocha);
    }

    #[test]
    fn test_available_themes() {
        let registry = ThemeRegistry::new();
        let themes = registry.available_themes();
        assert_eq!(themes.len(), 7);
    }
}
