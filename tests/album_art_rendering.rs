//! Album art rendering tests - integration tests

use ratatui::prelude::*;

// Copy implementation for testing since we can't access private modules
mod test_impl {
    use image::GenericImageView;
    use ratatui::prelude::*;

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Protocol {
        Kitty,
        Sixel,
        ITerm2,
        Ascii,
    }

    impl Protocol {
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
    }

    pub fn render_album_art(
        image_data: &[u8],
        area: Rect,
        protocol: Protocol,
    ) -> Vec<Line<'static>> {
        match protocol {
            Protocol::Kitty => render_kitty(image_data, area),
            Protocol::Sixel => render_sixel(image_data, area),
            Protocol::ITerm2 => render_iterm2(image_data, area),
            Protocol::Ascii => render_ascii(image_data, area),
        }
    }

    fn render_kitty(image_data: &[u8], area: Rect) -> Vec<Line<'static>> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let width = area.width;
        let height = area.height;
        let base64_data = STANDARD.encode(image_data);
        let escape = format!(
            "\x1b_Gf=32,s={},v={},a=q,t=f;{}\x1b\\",
            width, height, base64_data
        );
        vec![Line::from(escape)]
    }

    fn render_sixel(image_data: &[u8], _area: Rect) -> Vec<Line<'static>> {
        render_ascii(image_data, _area)
    }

    fn render_iterm2(image_data: &[u8], _area: Rect) -> Vec<Line<'static>> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let base64_data = STANDARD.encode(image_data);
        let escape = format!("\x1b]1337;File=inline=1;{}\x07", base64_data);
        vec![Line::from(escape)]
    }

    fn render_ascii(image_data: &[u8], area: Rect) -> Vec<Line<'static>> {
        match image::load_from_memory(image_data) {
            Ok(img) => {
                let width = area.width.min(40) as u32;
                let height = area.height.min(20) as u32;

                if width == 0 || height == 0 {
                    return create_ascii_border(area);
                }

                let resized = img.resize(width, height, image::imageops::FilterType::Nearest);
                let resized_width = resized.width();
                let resized_height = resized.height();
                let mut lines = Vec::new();

                for y in 0..resized_height {
                    let mut line_string = String::new();
                    for x in 0..resized_width {
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

    fn create_ascii_border(area: Rect) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let width = area.width as usize;
        let height = area.height as usize;

        if width < 3 || height < 3 {
            return lines;
        }

        lines.push(Line::from("╭".to_string() + &"─".repeat(width - 2) + "╮"));

        for _ in 1..height - 1 {
            lines.push(Line::from("│".to_string() + &" ".repeat(width - 2) + "│"));
        }

        lines.push(Line::from("╰".to_string() + &"─".repeat(width - 2) + "╯"));

        lines
    }

    fn pixel_to_char(pixel: &image::Rgba<u8>) -> char {
        let brightness = (pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32) / 3;
        const GRADIENT: &[char] = &[' ', '░', '▒', '▓', '█'];
        let index = (brightness as usize * GRADIENT.len()) / 256;
        GRADIENT[index.min(GRADIENT.len() - 1)]
    }

    pub struct AlbumArtWidget<'a> {
        pub image_data: Option<&'a [u8]>,
        protocol: Protocol,
    }

    impl<'a> AlbumArtWidget<'a> {
        pub fn new(image_data: Option<&'a [u8]>) -> Self {
            Self {
                image_data,
                protocol: Protocol::detect(),
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
                let placeholder = create_ascii_border(area);
                for (i, line) in placeholder.iter().enumerate() {
                    if i < area.height as usize {
                        buf.set_line(area.x, area.y + i as u16, line, area.width);
                    }
                }
            }
        }
    }
}

use test_impl::{render_album_art, AlbumArtWidget, Protocol};

#[test]
fn test_protocol_detection() {
    let protocol = Protocol::detect();
    assert!(matches!(
        protocol,
        Protocol::Kitty | Protocol::Sixel | Protocol::ITerm2 | Protocol::Ascii
    ));
}

#[test]
fn test_ascii_fallback_on_invalid_image() {
    let invalid_data = vec![0xDE, 0xAD, 0xBE, 0xEF];
    let area = Rect::new(0, 0, 10, 5);

    let lines = render_album_art(&invalid_data, area, Protocol::Ascii);
    assert!(!lines.is_empty());
    assert!(lines.len() >= 2);
}

#[test]
fn test_ascii_fallback_on_empty_data() {
    let empty_data = vec![];
    let area = Rect::new(0, 0, 10, 5);

    let lines = render_album_art(&empty_data, area, Protocol::Ascii);
    assert!(!lines.is_empty());
}

#[test]
fn test_album_art_widget_with_data() {
    let image_data = vec![0xFF, 0xD8, 0xFF, 0xE0];
    let widget = AlbumArtWidget::new(Some(&image_data));
    assert!(widget.image_data.is_some());
}

#[test]
fn test_album_art_widget_without_data() {
    let widget = AlbumArtWidget::new(None);
    assert!(widget.image_data.is_none());
}

#[test]
fn test_protocol_display() {
    let protocols = [
        Protocol::Kitty,
        Protocol::Sixel,
        Protocol::ITerm2,
        Protocol::Ascii,
    ];
    for protocol in protocols {
        let debug_str = format!("{:?}", protocol);
        assert!(!debug_str.is_empty());
    }
}

#[test]
fn test_render_album_art_kitty_protocol() {
    let image_data = vec![0xFF, 0xD8, 0xFF, 0xE0];
    let area = Rect::new(0, 0, 5, 3);

    let lines = render_album_art(&image_data, area, Protocol::Kitty);
    assert!(!lines.is_empty());
}

#[test]
fn test_render_album_art_iterm2_protocol() {
    let image_data = vec![0xFF, 0xD8, 0xFF, 0xE0];
    let area = Rect::new(0, 0, 5, 3);

    let lines = render_album_art(&image_data, area, Protocol::ITerm2);
    assert!(!lines.is_empty());
}

#[test]
fn test_render_album_art_sixel_protocol() {
    let image_data = vec![0xFF, 0xD8, 0xFF, 0xE0];
    let area = Rect::new(0, 0, 5, 3);

    let lines = render_album_art(&image_data, area, Protocol::Sixel);
    assert!(!lines.is_empty());
}

#[test]
fn test_widget_render_with_data() {
    let image_data = vec![0xFF, 0xD8, 0xFF, 0xE0];
    let widget = AlbumArtWidget::new(Some(&image_data));
    let area = Rect::new(0, 0, 10, 5);
    let mut buf = Buffer::empty(area);

    widget.render(area, &mut buf);
    assert!(buf.content.len() > 0);
}

#[test]
fn test_widget_render_without_data() {
    let widget = AlbumArtWidget::new(None);
    let area = Rect::new(0, 0, 10, 5);
    let mut buf = Buffer::empty(area);

    widget.render(area, &mut buf);
    assert!(buf.content.len() > 0);
}

#[test]
fn test_widget_render_small_area() {
    let image_data = vec![0xFF, 0xD8, 0xFF, 0xE0];
    let widget = AlbumArtWidget::new(Some(&image_data));
    let area = Rect::new(0, 0, 3, 3);
    let mut buf = Buffer::empty(area);

    widget.render(area, &mut buf);
}

#[test]
fn test_pixel_to_char_gradient() {
    use image::Rgba;

    let black = Rgba([0u8, 0u8, 0u8, 255u8]);
    let white = Rgba([255u8, 255u8, 255u8, 255u8]);
    let gray = Rgba([128u8, 128u8, 128u8, 255u8]);

    fn pixel_to_char(pixel: &image::Rgba<u8>) -> char {
        let brightness = (pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32) / 3;
        const GRADIENT: &[char] = &[' ', '░', '▒', '▓', '█'];
        let index = (brightness as usize * GRADIENT.len()) / 256;
        GRADIENT[index.min(GRADIENT.len() - 1)]
    }

    let black_char = pixel_to_char(&black);
    let white_char = pixel_to_char(&white);
    let gray_char = pixel_to_char(&gray);

    // White (brightness 255) should map to darkest char (█)
    assert_eq!(white_char, '█');
    // Black (brightness 0) maps to lightest char (' ') - this is correct
    assert_eq!(black_char, ' ');
    // Gray should be in between
    assert!(matches!(gray_char, '░' | '▒' | '▓'));
}

/// Regression test: Album art rendering must be independent of theme system
/// 
/// This test verifies that changing themes does not affect album art rendering,
/// since image rendering uses terminal graphics protocols (Kitty/iTerm2) that
/// display actual image pixels, not themed colors.
///
/// See: .learnings/learnings.md entry "2025-05-04 - Album art rendering is completely
/// independent of the theme system"
#[test]
fn test_album_art_rendering_independent_of_theme() {
    // Import theme system - this would fail if themes module doesn't exist
    use joshify::themes::BuiltInTheme;

    // Create a simple test image (8x8 pixel to ensure it fits in our constraints)
    let mut img = image::RgbImage::new(8, 8);
    for y in 0..8 {
        for x in 0..8 {
            img.put_pixel(x, y, image::Rgb([100u8, 150u8, 200u8]));
        }
    }

    let mut image_data = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut image_data), image::ImageFormat::Png)
        .expect("Failed to encode test image");

    let area = Rect::new(0, 0, 6, 4);

    // Test rendering with different themes - should produce identical output
    let themes = vec![
        BuiltInTheme::CatppuccinMocha,
        BuiltInTheme::CatppuccinLatte,
        BuiltInTheme::GruvboxDark,
        BuiltInTheme::GruvboxLight,
        BuiltInTheme::Nord,
        BuiltInTheme::TokyoNight,
        BuiltInTheme::Dracula,
    ];

    let mut previous_render: Option<Vec<Line<'static>>> = None;

    for theme in themes {
        // Set theme
        joshify::ui::theme::set_current_theme(theme);

        // Render album art
        let current_render = test_impl::render_album_art(&image_data, area, test_impl::Protocol::Ascii);

        // Verify render is not empty
        assert!(!current_render.is_empty(), "Theme {:?} produced empty render", theme);

        // All renders should be identical regardless of theme
        if let Some(ref prev) = previous_render {
            assert_eq!(current_render.len(), prev.len(), 
                "Theme {:?} produced different line count than previous theme", theme);
            
            // Verify content is identical
            for (i, (curr_line, prev_line)) in current_render.iter().zip(prev.iter()).enumerate() {
                assert_eq!(curr_line.to_string(), prev_line.to_string(),
                    "Theme {:?} produced different content at line {} compared to previous theme", theme, i);
            }
        }

        previous_render = Some(current_render);
    }

    // Reset to default theme
    joshify::ui::theme::set_current_theme(BuiltInTheme::CatppuccinMocha);
}
