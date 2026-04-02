//! Album art rendering with terminal graphics protocols
//!
//! Supports:
//! - Kitty graphics protocol (best quality, full color)
//! - Sixel graphics (good fallback)
//! - iTerm2 inline images
//! - ASCII/Unicode block fallback (always works)

use image::GenericImageView;
use ratatui::prelude::*;

/// Image rendering protocol support
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Protocol {
    Kitty,
    Sixel,
    ITerm2,
    Ascii,
}

impl Protocol {
    /// Detect best supported protocol from TERM and environment
    pub fn detect() -> Self {
        let term = std::env::var("TERM").unwrap_or_default();
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
        let kitty_term = std::env::var("KITTY_WINDOW_ID").is_ok();

        // Kitty detection
        if kitty_term || term.contains("kitty") {
            return Self::Kitty;
        }

        // iTerm2 detection
        if term_program.contains("iTerm") || term_program.contains("iTerm2") {
            return Self::ITerm2;
        }

        // Sixel detection (check for sixel support in TERM)
        if term.contains("sixel") || term.contains("mlterm") {
            return Self::Sixel;
        }

        // Foot terminal with sixel
        if term == "foot" || term == "foot-extra" {
            return Self::Sixel;
        }

        // Default to ASCII fallback
        Self::Ascii
    }
}

/// Render album art to terminal using detected protocol
pub fn render_album_art(image_data: &[u8], area: Rect, protocol: Protocol) -> Vec<Line<'static>> {
    match protocol {
        Protocol::Kitty => render_kitty(image_data, area),
        Protocol::Sixel => render_sixel(image_data, area),
        Protocol::ITerm2 => render_iterm2(image_data, area),
        Protocol::Ascii => render_ascii(image_data, area),
    }
}

/// Kitty graphics protocol rendering
fn render_kitty(image_data: &[u8], area: Rect) -> Vec<Line<'static>> {
    use base64::{engine::general_purpose::STANDARD, Engine};

    let width = area.width;
    let height = area.height;

    // Kitty escape sequence: \033_Gf=32,s=<width>,v=<height>,a=q,t=f;<base64_data>\033\\
    let base64_data = STANDARD.encode(image_data);
    let escape = format!(
        "\x1b_Gf=32,s={},v={},a=q,t=f;{}\x1b\\",
        width, height, base64_data
    );

    vec![Line::from(escape)]
}

/// Sixel graphics rendering
fn render_sixel(image_data: &[u8], _area: Rect) -> Vec<Line<'static>> {
    // Sixel requires converting image to sixel format
    // For now, fall back to ASCII - proper sixel needs image processing
    render_ascii(image_data, _area)
}

/// iTerm2 inline image rendering
fn render_iterm2(image_data: &[u8], _area: Rect) -> Vec<Line<'static>> {
    use base64::{engine::general_purpose::STANDARD, Engine};

    let base64_data = STANDARD.encode(image_data);
    // iTerm2 escape: \033]1337;File=inline=1;<base64>\007
    let escape = format!("\x1b]1337;File=inline=1;{}\x07", base64_data);

    vec![Line::from(escape)]
}

/// ASCII/Unicode block rendering (fallback)
fn render_ascii(image_data: &[u8], area: Rect) -> Vec<Line<'static>> {
    // Decode image and create ASCII art representation
    match image::load_from_memory(image_data) {
        Ok(img) => {
            let width = area.width.min(40) as u32;
            let height = area.height.min(20) as u32;

            if width == 0 || height == 0 {
                return create_ascii_border(area);
            }

            let resized = img.resize(width, height, image::imageops::FilterType::Nearest);

            let mut lines = Vec::new();

            for y in 0..height {
                let mut line_string = String::new();
                for x in 0..width {
                    let pixel = resized.get_pixel(x, y);
                    let c = pixel_to_char(&pixel);
                    line_string.push(c);
                }
                lines.push(Line::from(line_string));
            }

            lines
        }
        Err(_) => create_ascii_border(area),
    }
}

/// Create simple ASCII box border as fallback
fn create_ascii_border(area: Rect) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let width = area.width as usize;
    let height = area.height as usize;

    if width < 3 || height < 3 {
        return lines;
    }

    // Top border
    lines.push(Line::from("╭".to_string() + &"─".repeat(width - 2) + "╮"));

    // Middle rows
    for _ in 1..height - 1 {
        lines.push(Line::from("│".to_string() + &" ".repeat(width - 2) + "│"));
    }

    // Bottom border
    lines.push(Line::from("╰".to_string() + &"─".repeat(width - 2) + "╯"));

    lines
}

/// Convert pixel to ASCII character based on brightness
fn pixel_to_char(pixel: &image::Rgba<u8>) -> char {
    // Calculate brightness (0-255)
    let brightness = (pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32) / 3;

    // Map brightness to character
    const GRADIENT: &[char] = &[' ', '░', '▒', '▓', '█'];
    let index = (brightness as usize * GRADIENT.len()) / 256;
    GRADIENT[index.min(GRADIENT.len() - 1)]
}

/// Widget for rendering album art in ratatui
pub struct AlbumArtWidget<'a> {
    image_data: Option<&'a [u8]>,
    protocol: Protocol,
}

impl<'a> AlbumArtWidget<'a> {
    pub fn new(image_data: Option<&'a [u8]>) -> Self {
        Self {
            image_data,
            protocol: Protocol::detect(),
        }
    }

    pub fn with_protocol(image_data: Option<&'a [u8]>, protocol: Protocol) -> Self {
        Self {
            image_data,
            protocol,
        }
    }
}

impl Widget for AlbumArtWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if let Some(data) = self.image_data {
            let lines = render_album_art(data, area, self.protocol);
            for (i, line) in lines.iter().enumerate() {
                if i < area.height as usize {
                    buf.set_line(area.x, area.y + i as u16, line, area.width);
                }
            }
        } else {
            // No image data - render placeholder
            let placeholder = create_ascii_border(area);
            for (i, line) in placeholder.iter().enumerate() {
                if i < area.height as usize {
                    buf.set_line(area.x, area.y + i as u16, line, area.width);
                }
            }
        }
    }
}
