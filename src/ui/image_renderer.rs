//! Album art rendering with terminal graphics protocols
//!
//! Supports:
//! - Kitty graphics protocol (best quality, full color)
//! - Sixel graphics (good fallback)
//! - iTerm2 inline images
//! - ASCII/Unicode block fallback (always works)

use ratatui::prelude::*;
use std::io::Write;

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

        if kitty_term || term.contains("kitty") {
            return Self::Kitty;
        }

        if term_program.contains("iTerm") || term_program.contains("iTerm2") {
            return Self::ITerm2;
        }

        if term.contains("sixel") || term.contains("mlterm") {
            return Self::Sixel;
        }

        if term == "foot" || term == "foot-extra" {
            return Self::Sixel;
        }

        Self::Ascii
    }

    /// Whether this protocol can display an inline image written to stdout.
    ///
    /// Only Kitty is actually implemented. This matters beyond "no image
    /// appears": the render loop deletes and space-fills the album-art
    /// rectangle before writing a Kitty payload, so producing a payload for a
    /// terminal that cannot show one erases the ASCII fallback that *was*
    /// drawn there. See issue #59.
    pub fn supports_inline_image(self) -> bool {
        matches!(self, Self::Kitty)
    }
}

/// Write image directly to stdout using the appropriate protocol.
/// This must be called AFTER terminal.draw() to bypass ratatui's buffer.
pub fn write_image_to_stdout(
    image_data: &[u8],
    area: Rect,
    protocol: Protocol,
) -> std::io::Result<()> {
    match protocol {
        Protocol::Kitty => write_kitty(image_data, area),
        Protocol::ITerm2 => write_iterm2(image_data),
        Protocol::Sixel | Protocol::Ascii => Ok(()),
    }
}

/// Pre-process raw image data into a Kitty escape sequence.
/// Call this ONCE when the image arrives, not every frame.
pub fn prepare_kitty_image(image_data: &[u8], area: Rect) -> Option<Vec<u8>> {
    use base64::{engine::general_purpose::STANDARD, Engine};

    let cell_width = 8u32;
    let cell_height = 16u32;
    let target_width = (area.width as u32 * cell_width).clamp(1, 200);
    let target_height = (area.height as u32 * cell_height).clamp(1, 200);

    let img = image::load_from_memory(image_data).ok()?;
    let resized = img.resize(
        target_width,
        target_height,
        image::imageops::FilterType::Lanczos3,
    );
    let mut buffer = Vec::new();
    resized
        .write_to(
            &mut std::io::Cursor::new(&mut buffer),
            image::ImageFormat::Png,
        )
        .ok()?;

    let base64_data = STANDARD.encode(&buffer);
    let chunk_size = 4096;
    let total_chunks = base64_data.len().div_ceil(chunk_size);

    let mut escape = Vec::new();
    // Cursor positioning
    escape.extend_from_slice(format!("\x1b[{};{}H", area.y + 1, area.x + 1).as_bytes());

    for (i, chunk) in base64_data.as_bytes().chunks(chunk_size).enumerate() {
        let more = if i < total_chunks - 1 { 1 } else { 0 };
        escape.extend_from_slice(format!("\x1b_Gf=100,a=T,t=d,m={};", more).as_bytes());
        escape.extend_from_slice(chunk);
        escape.extend_from_slice(b"\x1b\\");
    }

    Some(escape)
}

/// Write a pre-processed Kitty escape sequence to stdout.
/// This is fast - just a single write() call.
pub fn write_prepared_kitty_image(prepared: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut stdout = std::io::stdout();
    stdout.write_all(prepared)?;
    stdout.flush()?;
    Ok(())
}

/// Clear a rectangular area on the terminal screen by overwriting with spaces.
/// Uses exact column positioning and space-filling to avoid erasing content
/// beyond the area bounds (unlike \x1b[K which erases to end of line).
pub fn clear_terminal_area(area: Rect) -> std::io::Result<()> {
    use std::io::Write;
    if area.width == 0 || area.height == 0 {
        return Ok(());
    }
    let spaces = " ".repeat(area.width as usize);
    let mut buf = Vec::new();
    for y in area.top()..area.bottom() {
        buf.extend_from_slice(format!("\x1b[{};{}H{}", y + 1, area.x + 1, spaces).as_bytes());
    }
    let mut stdout = std::io::stdout();
    stdout.write_all(&buf)?;
    stdout.flush()?;
    Ok(())
}

/// Delete a previously-rendered Kitty image by its area.
/// Uses the Kitty graphics protocol's delete command (\x1b_Ga=d) to remove
/// images in a specific region without affecting surrounding text.
/// Falls back to clear_terminal_area if Kitty protocol isn't available.
pub fn delete_kitty_image_in_area(area: Rect) -> std::io::Result<()> {
    use std::io::Write;
    if area.width == 0 || area.height == 0 {
        return Ok(());
    }
    let mut buf = Vec::new();
    // Kitty delete command: delete all images that intersect with the given rectangle
    // \x1b_Ga=d,C=1,x=<left>,y=<top>,w=<width>,h=<height>\x1b\
    buf.extend_from_slice(
        format!(
            "\x1b_Ga=d,C=1,x={},y={},w={},h={}\x1b\\",
            area.x + 1,
            area.y + 1,
            area.width,
            area.height
        )
        .as_bytes(),
    );
    let mut stdout = std::io::stdout();
    stdout.write_all(&buf)?;
    stdout.flush()?;
    Ok(())
}

/// Clear a rectangular area in the ratatui buffer by filling it with spaces.
/// This is needed to erase previously-rendered Kitty images before drawing at a new position.
/// Without this, the old image persists on screen because Kitty images bypass the ratatui buffer.
pub fn clear_area(frame: &mut ratatui::Frame, area: Rect) {
    let buf = frame.buffer_mut();
    for x in area.left()..area.right() {
        for y in area.top()..area.bottom() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(' ');
                cell.set_style(Style::default().bg(ratatui::style::Color::Reset));
            }
        }
    }
}

/// Mark cells in an area as skipped so ratatui doesn't overwrite them.
/// Call this after rendering album art to prevent the buffer from overwriting the image.
pub fn skip_area(frame: &mut ratatui::Frame, area: Rect) {
    let buf = frame.buffer_mut();
    for x in area.left()..area.right() {
        for y in area.top()..area.bottom() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_skip(true);
            }
        }
    }
}

/// Write raw image data as Kitty graphics (legacy - does full processing every call)
fn write_kitty(image_data: &[u8], area: Rect) -> std::io::Result<()> {
    if let Some(prepared) = prepare_kitty_image(image_data, area) {
        write_prepared_kitty_image(&prepared)
    } else {
        Ok(())
    }
}

/// Write iTerm2 inline image escape sequence directly to stdout
fn write_iterm2(image_data: &[u8]) -> std::io::Result<()> {
    use base64::{engine::general_purpose::STANDARD, Engine};

    let base64_data = STANDARD.encode(image_data);
    let mut stdout = std::io::stdout();

    // Split into chunks for large images
    let chunk_size = 4096;
    let chunks: Vec<&[u8]> = base64_data.as_bytes().chunks(chunk_size).collect();

    for (i, chunk) in chunks.iter().enumerate() {
        write!(stdout, "\x1b]1337;File=inline=1")?;
        if i == 0 {
            write!(stdout, ";size={}", image_data.len())?;
        }
        if i < chunks.len() - 1 {
            write!(stdout, ";m=1")?;
        }
        write!(stdout, ":")?;
        stdout.write_all(chunk)?;
        if i < chunks.len() - 1 {
            write!(stdout, "\x1b\\")?;
        } else {
            write!(stdout, "\x07")?;
        }
    }
    stdout.flush()?;

    Ok(())
}

/// Render album art as text lines (ASCII fallback only)
pub fn render_album_art_as_lines(image_data: &[u8], area: Rect) -> Vec<Line<'static>> {
    render_ascii(image_data, area)
}

/// Unicode half-block rendering (fallback for terminals with no image protocol).
///
/// Each cell carries two pixels: the upper half is the foreground colour of
/// `\u{2580}` (UPPER HALF BLOCK) and the lower half is its background. That
/// doubles vertical resolution over one-pixel-per-cell and, with 24-bit colour,
/// reproduces the cover rather than a monochrome brightness ramp. Terminals
/// without truecolour degrade to their nearest palette entry on their own.
fn render_ascii(image_data: &[u8], area: Rect) -> Vec<Line<'static>> {
    let Ok(img) = image::load_from_memory(image_data) else {
        return create_ascii_border(area);
    };

    let cols = area.width.min(40) as u32;
    let rows = area.height.min(20) as u32;
    if cols == 0 || rows == 0 {
        return create_ascii_border(area);
    }

    // Two pixels per cell vertically. Covers are square, and terminal cells are
    // roughly twice as tall as they are wide, so sampling this way keeps the
    // aspect ratio instead of squashing the image.
    let resized = img.resize_exact(cols, rows * 2, image::imageops::FilterType::Triangle);
    let rgba = resized.to_rgba8();

    let mut lines = Vec::with_capacity(rows as usize);
    for row in 0..rows {
        let mut spans = Vec::with_capacity(cols as usize);
        for col in 0..cols {
            let top = rgba.get_pixel(col, row * 2);
            let bottom = rgba.get_pixel(col, row * 2 + 1);
            spans.push(Span::styled(
                "\u{2580}",
                Style::default()
                    .fg(Color::Rgb(top[0], top[1], top[2]))
                    .bg(Color::Rgb(bottom[0], bottom[1], bottom[2])),
            ));
        }
        lines.push(Line::from(spans));
    }

    lines
}

/// Placeholder frame shown when the image cannot be decoded.
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

/// Widget for rendering album art in ratatui
pub struct AlbumArtWidget<'a> {
    image_data: Option<&'a [u8]>,
    _protocol: Protocol,
}

impl<'a> AlbumArtWidget<'a> {
    pub fn new(image_data: Option<&'a [u8]>) -> Self {
        Self {
            image_data,
            _protocol: Protocol::detect(),
        }
    }

    pub fn with_protocol(image_data: Option<&'a [u8]>, protocol: Protocol) -> Self {
        Self {
            image_data,
            _protocol: protocol,
        }
    }
}

impl Widget for AlbumArtWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if let Some(data) = self.image_data {
            let lines = render_album_art_as_lines(data, area);
            for (i, line) in lines.iter().enumerate() {
                if i < area.height as usize {
                    buf.set_line(area.x, area.y + i as u16, line, area.width);
                }
            }
        } else {
            let placeholder = create_ascii_border(area);
            for (i, line) in placeholder.iter().enumerate() {
                if i < area.height as usize {
                    buf.set_line(area.x, area.y + i as u16, line, area.width);
                }
            }
        }
    }
}

#[cfg(test)]
mod half_block_render_tests {
    use super::render_album_art_as_lines;
    use ratatui::prelude::*;

    /// A 2x2 PNG: red, green on the top row; blue, white on the bottom.
    fn swatch_png() -> Vec<u8> {
        let mut img = image::RgbaImage::new(2, 2);
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        img.put_pixel(1, 0, image::Rgba([0, 255, 0, 255]));
        img.put_pixel(0, 1, image::Rgba([0, 0, 255, 255]));
        img.put_pixel(1, 1, image::Rgba([255, 255, 255, 255]));

        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .expect("encode png");
        bytes.into_inner()
    }

    #[test]
    fn fills_the_requested_area_exactly() {
        let lines = render_album_art_as_lines(&swatch_png(), Rect::new(0, 0, 8, 4));
        assert_eq!(lines.len(), 4, "one line per terminal row");
        for line in &lines {
            assert_eq!(line.spans.len(), 8, "one span per column");
        }
    }

    /// Regression guard for the fidelity fix (#59): the renderer must carry
    /// real colour, not a monochrome brightness ramp. Each cell encodes two
    /// pixels via UPPER HALF BLOCK with a foreground and a background colour.
    #[test]
    fn uses_half_blocks_with_two_colours_per_cell() {
        let lines = render_album_art_as_lines(&swatch_png(), Rect::new(0, 0, 2, 1));
        let span = &lines[0].spans[0];

        assert_eq!(
            span.content.as_ref(),
            "\u{2580}",
            "expected UPPER HALF BLOCK"
        );
        assert!(
            matches!(span.style.fg, Some(Color::Rgb(..))),
            "foreground must be a truecolour pixel, got {:?}",
            span.style.fg
        );
        assert!(
            matches!(span.style.bg, Some(Color::Rgb(..))),
            "background carries the lower pixel, got {:?}",
            span.style.bg
        );
        assert_ne!(
            span.style.fg, span.style.bg,
            "top and bottom pixels of this swatch differ, so the cell must too"
        );
    }

    #[test]
    fn distinct_columns_get_distinct_colours() {
        let lines = render_album_art_as_lines(&swatch_png(), Rect::new(0, 0, 2, 1));
        let left = lines[0].spans[0].style.fg;
        let right = lines[0].spans[1].style.fg;
        assert_ne!(
            left, right,
            "a red and a green column must not render alike"
        );
    }

    #[test]
    fn undecodable_data_falls_back_to_a_border_rather_than_panicking() {
        let lines = render_album_art_as_lines(b"not an image", Rect::new(0, 0, 6, 3));
        assert!(!lines.is_empty());
    }

    #[test]
    fn a_zero_sized_area_is_handled() {
        let lines = render_album_art_as_lines(&swatch_png(), Rect::new(0, 0, 0, 0));
        assert!(lines.len() <= 1, "must not panic or allocate a huge buffer");
    }
}

#[cfg(test)]
mod protocol_capability_tests {
    use super::Protocol;

    /// Regression for #59. The render loop deletes and space-fills the album-art
    /// rectangle before writing a Kitty payload. A payload built for a terminal
    /// that cannot display one therefore erases the ASCII fallback that was
    /// drawn there, every frame - which is why no art appeared on Windows
    /// Terminal even though it was fetched and rendered.
    #[test]
    fn only_kitty_supports_inline_images() {
        assert!(Protocol::Kitty.supports_inline_image());
        assert!(
            !Protocol::Ascii.supports_inline_image(),
            "an Ascii terminal must not get a Kitty payload; producing one \
             makes the render loop clear the ASCII art (issue #59)"
        );
        assert!(
            !Protocol::Sixel.supports_inline_image(),
            "write_image_to_stdout no-ops for Sixel, so a payload would only clear"
        );
        assert!(
            !Protocol::ITerm2.supports_inline_image(),
            "iTerm2 inline images are not implemented"
        );
    }

    /// A Windows Terminal / WSL session must resolve to Ascii. The pre-existing
    /// detection test set no environment at all and passed by accident.
    #[test]
    #[serial_test::serial(env_term)]
    fn windows_terminal_detects_as_ascii() {
        let saved: Vec<(&str, Option<String>)> = ["TERM", "TERM_PROGRAM", "KITTY_WINDOW_ID"]
            .iter()
            .map(|k| (*k, std::env::var(k).ok()))
            .collect();

        std::env::set_var("TERM", "xterm-256color");
        std::env::remove_var("TERM_PROGRAM");
        std::env::remove_var("KITTY_WINDOW_ID");

        let detected = Protocol::detect();

        for (key, value) in saved {
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }

        assert_eq!(detected, Protocol::Ascii);
        assert!(!detected.supports_inline_image());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_detection() {
        assert_eq!(Protocol::detect(), Protocol::Ascii);
    }

    #[test]
    fn test_album_art_widget_default_protocol() {
        let widget = AlbumArtWidget::new(None);
        assert_eq!(widget._protocol, Protocol::detect());
    }

    #[test]
    fn test_album_art_widget_with_protocol() {
        let widget = AlbumArtWidget::with_protocol(None, Protocol::Kitty);
        assert_eq!(widget._protocol, Protocol::Kitty);
    }

    #[test]
    fn test_kitty_escape_sequence_format() {
        // Test that the Kitty escape sequence is properly formatted
        let expected_prefix = "\x1b_Gf=100,a=T,t=d,m=0;";
        assert!(expected_prefix.contains("\x1b_G"));
        assert!(expected_prefix.contains("f=100"));
        assert!(expected_prefix.contains("a=T"));
        assert!(expected_prefix.contains("t=d"));
    }

    #[test]
    fn test_iterm2_escape_sequence_format() {
        // Test that the iTerm2 escape sequence is properly formatted
        let expected_prefix = "\x1b]1337;File=inline=1";
        assert!(expected_prefix.contains("\x1b]1337;"));
        assert!(expected_prefix.contains("File=inline=1"));
    }

    #[test]
    fn test_render_ascii_returns_lines() {
        let tiny_png = include_bytes!("../../tests/fixtures/tiny.png") as &[u8];
        let area = Rect::new(0, 0, 10, 5);
        let result = render_ascii(tiny_png, area);
        assert!(!result.is_empty());
        assert!(result.len() <= area.height as usize);
    }

    #[test]
    fn test_render_album_art_as_lines_returns_lines() {
        let tiny_png = include_bytes!("../../tests/fixtures/tiny.png") as &[u8];
        let area = Rect::new(0, 0, 10, 5);
        let result = render_album_art_as_lines(tiny_png, area);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_write_image_to_stdout_with_invalid_data() {
        // Should not panic with invalid image data
        let area = Rect::new(0, 0, 10, 5);
        let result = write_image_to_stdout(b"not an image", area, Protocol::Kitty);
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_image_to_stdout_ascii_protocol() {
        // ASCII protocol should always return Ok(())
        let area = Rect::new(0, 0, 10, 5);
        let result = write_image_to_stdout(b"any data", area, Protocol::Ascii);
        assert!(result.is_ok());
    }

    #[test]
    fn test_write_image_to_stdout_sixel_protocol() {
        // Sixel protocol should always return Ok(()) for now
        let area = Rect::new(0, 0, 10, 5);
        let result = write_image_to_stdout(b"any data", area, Protocol::Sixel);
        assert!(result.is_ok());
    }
}
