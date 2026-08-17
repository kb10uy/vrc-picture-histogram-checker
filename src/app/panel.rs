use std::{fmt::Write, mem::take, sync::Arc};

use image::{ExtendedColorType, ImageEncoder, RgbImage, codecs::png::PngEncoder};
use resvg::{
    render,
    tiny_skia::{Pixmap, Transform},
    usvg::{ImageHrefResolver, ImageKind, Options, Tree, fontdb::Database},
};
use thiserror::Error as ThisError;

use crate::app::analysis::{Analysis, BINS, THUMBNAIL_MAX_HEIGHT, THUMBNAIL_MAX_WIDTH};

pub const PANEL_WIDTH: u32 = 512;
pub const PANEL_HEIGHT: u32 = 424;

const PADDING: f32 = 16.0;
const THUMBNAIL_TOP: f32 = 16.0;
const HISTOGRAM_TOP: f32 = 298.0;
const HISTOGRAM_HEIGHT: f32 = 90.0;
const TEXT_BASELINE: f32 = 408.0;
const CONTENT_WIDTH: f32 = PANEL_WIDTH as f32 - PADDING * 2.0;
const THUMBNAIL_HREF: &str = "thumbnail";
const FONT_FAMILY: &str = "Segoe UI";
const CHANNEL_COLORS: [&str; 3] = ["#ff5a5a", "#5aff8c", "#5aa8ff"];
const MESSAGE_LINE_HEIGHT: f32 = 24.0;
const MESSAGE_COLUMNS: usize = 64;

#[derive(Debug, ThisError)]
pub enum PanelError {
    #[error("failed to encode the thumbnail")]
    Thumbnail(#[from] image::ImageError),

    #[error("failed to build the panel document")]
    Document(#[from] resvg::usvg::Error),

    #[error("failed to allocate a {PANEL_WIDTH}x{PANEL_HEIGHT} pixmap")]
    Pixmap,
}

pub struct Frame {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub struct PanelRenderer {
    fonts: Arc<Database>,
}

impl PanelRenderer {
    pub fn new() -> Self {
        let mut fonts = Database::new();
        fonts.load_system_fonts();
        PanelRenderer {
            fonts: Arc::new(fonts),
        }
    }

    pub fn render(&self, analysis: &Analysis) -> Result<Frame, PanelError> {
        let thumbnail = encode_png(&analysis.thumbnail)?;
        let options = Options {
            image_href_resolver: ImageHrefResolver {
                resolve_string: Box::new(move |href, _| {
                    (href == THUMBNAIL_HREF).then(|| ImageKind::PNG(Arc::clone(&thumbnail)))
                }),
                ..ImageHrefResolver::default()
            },
            ..self.options()
        };

        rasterize(&build_document(analysis), &options)
    }

    pub fn render_message(&self, message: &str) -> Result<Frame, PanelError> {
        rasterize(&build_message_document(message), &self.options())
    }

    fn options(&self) -> Options<'static> {
        Options {
            font_family: FONT_FAMILY.to_owned(),
            fontdb: Arc::clone(&self.fonts),
            ..Options::default()
        }
    }
}

fn rasterize(document: &str, options: &Options) -> Result<Frame, PanelError> {
    let tree = Tree::from_str(document, options)?;
    let mut pixmap = Pixmap::new(PANEL_WIDTH, PANEL_HEIGHT).ok_or(PanelError::Pixmap)?;
    render(&tree, Transform::identity(), &mut pixmap.as_mut());

    Ok(Frame {
        pixels: to_straight_alpha(&pixmap),
        width: PANEL_WIDTH,
        height: PANEL_HEIGHT,
    })
}

fn build_document(analysis: &Analysis) -> String {
    let thumbnail_width = analysis.thumbnail.width() as f32;
    let thumbnail_height = analysis.thumbnail.height() as f32;
    let thumbnail_left = PADDING + (THUMBNAIL_MAX_WIDTH as f32 - thumbnail_width) / 2.0;
    let thumbnail_top = THUMBNAIL_TOP + (THUMBNAIL_MAX_HEIGHT as f32 - thumbnail_height) / 2.0;

    let peak = analysis
        .histogram
        .iter()
        .flat_map(|channel| channel.iter())
        .copied()
        .max()
        .unwrap_or(0);

    let mut document = String::with_capacity(8 * 1024);
    let _ = write!(
        document,
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" "#,
            r#"viewBox="0 0 {width} {height}">"#,
            r##"<rect width="{width}" height="{height}" rx="14" fill="#14161a"/>"##,
            r##"<rect x="{padding}" y="{thumbnail_backdrop_top}" width="{content}" "##,
            r##"height="{thumbnail_box_height}" rx="6" fill="#0b0d10"/>"##,
            r#"<image href="{href}" x="{thumbnail_left:.1}" y="{thumbnail_top:.1}" "#,
            r#"width="{thumbnail_width:.1}" height="{thumbnail_height:.1}"/>"#,
            r##"<rect x="{padding}" y="{histogram_top}" width="{content}" "##,
            r##"height="{histogram_height}" rx="6" fill="#0b0d10"/>"##,
        ),
        width = PANEL_WIDTH,
        height = PANEL_HEIGHT,
        padding = PADDING,
        content = CONTENT_WIDTH,
        href = THUMBNAIL_HREF,
        thumbnail_backdrop_top = THUMBNAIL_TOP,
        thumbnail_box_height = THUMBNAIL_MAX_HEIGHT,
        thumbnail_left = thumbnail_left,
        thumbnail_top = thumbnail_top,
        thumbnail_width = thumbnail_width,
        thumbnail_height = thumbnail_height,
        histogram_top = HISTOGRAM_TOP,
        histogram_height = HISTOGRAM_HEIGHT,
    );

    for (channel, color) in CHANNEL_COLORS.iter().enumerate() {
        let _ = write!(
            document,
            r#"<path d="{}" fill="{color}" fill-opacity="0.45"/>"#,
            channel_outline(&analysis.histogram[channel], peak)
        );
    }

    let _ = write!(
        document,
        concat!(
            r##"<text x="{padding}" y="{baseline}" font-family="{font}" font-size="16" "##,
            r##"fill="#c9ced6">{image_width}x{image_height}</text>"##,
            r##"<text x="{right}" y="{baseline}" font-family="{font}" font-size="16" "##,
            r##"text-anchor="end" fill="#c9ced6">"##,
            "highlight {highlights:.2}%  shadow {shadows:.2}%</text></svg>",
        ),
        padding = PADDING,
        right = PANEL_WIDTH as f32 - PADDING,
        baseline = TEXT_BASELINE,
        font = FONT_FAMILY,
        image_width = analysis.width,
        image_height = analysis.height,
        highlights = analysis.clipped_highlights * 100.0,
        shadows = analysis.clipped_shadows * 100.0,
    );

    document
}

fn build_message_document(message: &str) -> String {
    let lines = wrap(message, MESSAGE_COLUMNS);
    let top = (PANEL_HEIGHT as f32 - MESSAGE_LINE_HEIGHT * lines.len() as f32) / 2.0;

    let mut document = String::with_capacity(1024);
    let _ = write!(
        document,
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" "#,
            r#"viewBox="0 0 {width} {height}">"#,
            r##"<rect width="{width}" height="{height}" rx="14" fill="#14161a"/>"##,
        ),
        width = PANEL_WIDTH,
        height = PANEL_HEIGHT,
    );

    for (index, line) in lines.iter().enumerate() {
        let _ = write!(
            document,
            concat!(
                r##"<text x="{centre}" y="{baseline:.1}" font-family="{font}" font-size="16" "##,
                r##"text-anchor="middle" fill="#c9ced6">{text}</text>"##,
            ),
            centre = PANEL_WIDTH as f32 / 2.0,
            baseline = top + MESSAGE_LINE_HEIGHT * (index + 1) as f32,
            font = FONT_FAMILY,
            text = escape(line),
        );
    }

    document.push_str("</svg>");
    document
}

fn wrap(text: &str, columns: usize) -> Vec<String> {
    let mut lines = Vec::new();

    for paragraph in text.lines() {
        let mut current = String::new();
        for mut word in paragraph.split_whitespace() {
            while word.chars().count() > columns {
                if !current.is_empty() {
                    lines.push(take(&mut current));
                }
                let split = word
                    .char_indices()
                    .nth(columns)
                    .map_or(word.len(), |(index, _)| index);
                let (head, tail) = word.split_at(split);
                lines.push(head.to_owned());
                word = tail;
            }

            if word.is_empty() {
                continue;
            }
            if current.is_empty() {
                current.push_str(word);
            } else if current.chars().count() + 1 + word.chars().count() <= columns {
                current.push(' ');
                current.push_str(word);
            } else {
                lines.push(take(&mut current));
                current.push_str(word);
            }
        }

        if !current.is_empty() || paragraph.is_empty() {
            lines.push(current);
        }
    }

    lines
}

fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn channel_outline(histogram: &[u32; BINS], peak: u32) -> String {
    let bottom = HISTOGRAM_TOP + HISTOGRAM_HEIGHT;
    let mut path = String::with_capacity(BINS * 16);
    let _ = write!(path, "M {PADDING:.1} {bottom:.1}");

    for (index, count) in histogram.iter().enumerate() {
        let x = PADDING + CONTENT_WIDTH * index as f32 / (BINS - 1) as f32;
        let normalized = if peak == 0 {
            0.0
        } else {
            (*count as f32 / peak as f32).sqrt()
        };
        let y = bottom - HISTOGRAM_HEIGHT * normalized;
        let _ = write!(path, " L {x:.1} {y:.1}");
    }

    let _ = write!(path, " L {:.1} {bottom:.1} Z", PADDING + CONTENT_WIDTH);
    path
}

fn to_straight_alpha(pixmap: &Pixmap) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(pixmap.pixels().len() * 4);
    for premultiplied in pixmap.pixels() {
        let color = premultiplied.demultiply();
        pixels.extend_from_slice(&[color.red(), color.green(), color.blue(), color.alpha()]);
    }
    pixels
}

fn encode_png(image: &RgbImage) -> Result<Arc<Vec<u8>>, PanelError> {
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes).write_image(
        image.as_raw(),
        image.width(),
        image.height(),
        ExtendedColorType::Rgb8,
    )?;
    Ok(Arc::new(bytes))
}

#[cfg(test)]
mod tests {
    use image::{Rgb, RgbImage};
    use rstest::rstest;

    use super::{
        MESSAGE_COLUMNS, PANEL_HEIGHT, PANEL_WIDTH, PanelRenderer, build_document,
        build_message_document, escape, wrap,
    };
    use crate::app::analysis::{Analysis, analyze_image};

    fn sample(color: [u8; 3]) -> Analysis {
        analyze_image(&RgbImage::from_pixel(1920, 1080, Rgb(color)))
    }

    #[test]
    fn renders_a_panel_of_the_declared_size() {
        let frame = PanelRenderer::new()
            .render(&sample([90, 120, 150]))
            .expect("should render");

        assert_eq!(frame.width, PANEL_WIDTH);
        assert_eq!(frame.height, PANEL_HEIGHT);
        assert_eq!(
            frame.pixels.len(),
            PANEL_WIDTH as usize * PANEL_HEIGHT as usize * 4
        );
    }

    #[test]
    fn paints_the_centre_of_the_panel_opaque() {
        let frame = PanelRenderer::new()
            .render(&sample([90, 120, 150]))
            .expect("should render");

        let centre =
            (PANEL_HEIGHT as usize / 2 * PANEL_WIDTH as usize + PANEL_WIDTH as usize / 2) * 4;
        assert_eq!(frame.pixels[centre + 3], 255);
    }

    #[test]
    fn draws_the_thumbnail_and_every_channel() {
        let document = build_document(&sample([10, 20, 30]));

        assert!(document.starts_with("<svg"));
        assert!(document.ends_with("</svg>"));
        assert!(document.contains(r#"href="thumbnail""#));
        assert_eq!(document.matches("<path").count(), 3);
        assert!(document.contains("1920x1080"));
    }

    #[test]
    fn keeps_an_all_black_photo_from_dividing_by_zero() {
        let document = build_document(&sample([0, 0, 0]));

        assert!(!document.contains("NaN"));
        assert!(!document.contains("inf"));
    }

    #[test]
    fn renders_a_message_of_the_declared_size() {
        let frame = PanelRenderer::new()
            .render_message("watching\nJ:\\Pictures\\VRChat")
            .expect("should render");

        assert_eq!(frame.width, PANEL_WIDTH);
        assert_eq!(frame.height, PANEL_HEIGHT);
    }

    #[test]
    fn draws_one_text_element_per_line() {
        let document = build_message_document("watching\n\nJ:\\Pictures\\VRChat");

        assert!(document.starts_with("<svg"));
        assert!(document.ends_with("</svg>"));
        assert_eq!(document.matches("<text").count(), 3);
        assert!(document.contains("J:\\Pictures\\VRChat"));
    }

    #[rstest]
    #[case::empty("", 0)]
    #[case::single_line("watching", 1)]
    #[case::blank_line_kept("watching\n\nnowhere", 3)]
    #[case::wrapped(
        "failed to read C:\\Users\\someone\\Pictures\\VRChat\\2026-08\\VRChat_2026-08-01.png",
        2
    )]
    #[case::unbroken_word(
        "CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC",
        2
    )]
    fn wraps_into_lines(#[case] text: &str, #[case] expected: usize) {
        let lines = wrap(text, MESSAGE_COLUMNS);

        assert_eq!(lines.len(), expected);
        assert!(
            lines
                .iter()
                .all(|line| line.chars().count() <= MESSAGE_COLUMNS)
        );
    }

    #[test]
    fn escapes_markup_in_messages() {
        assert_eq!(escape("a & b < c > d"), "a &amp; b &lt; c &gt; d");
    }
}
