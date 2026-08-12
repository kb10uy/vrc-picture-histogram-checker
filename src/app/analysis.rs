use std::path::{Path, PathBuf};

use image::{ImageReader, RgbImage, imageops::thumbnail};
use thiserror::Error as ThisError;

pub const BINS: usize = 256;
pub const THUMBNAIL_MAX_WIDTH: u32 = 480;
pub const THUMBNAIL_MAX_HEIGHT: u32 = 270;

const SAMPLE_TARGET: usize = 400_000;
const HIGHLIGHT_THRESHOLD: u8 = 254;
const SHADOW_THRESHOLD: u8 = 1;

#[derive(Debug, ThisError)]
pub enum AnalysisError {
    #[error("failed to read {path:?}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to decode {path:?}")]
    Decode {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },
}

pub struct Analysis {
    pub width: u32,
    pub height: u32,
    pub histogram: [[u32; BINS]; 3],
    pub clipped_highlights: f32,
    pub clipped_shadows: f32,
    pub thumbnail: RgbImage,
}

pub fn analyze(path: &Path) -> Result<Analysis, AnalysisError> {
    let reader = ImageReader::open(path).map_err(|source| AnalysisError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let image = reader
        .with_guessed_format()
        .map_err(|source| AnalysisError::Io {
            path: path.to_path_buf(),
            source,
        })?
        .decode()
        .map_err(|source| AnalysisError::Decode {
            path: path.to_path_buf(),
            source,
        })?;

    Ok(analyze_image(&image.to_rgb8()))
}

pub fn analyze_image(image: &RgbImage) -> Analysis {
    let total = image.width() as usize * image.height() as usize;
    let step = total.div_ceil(SAMPLE_TARGET).max(1);

    let mut histogram = [[0u32; BINS]; 3];
    let mut highlights = 0u32;
    let mut shadows = 0u32;
    let mut sampled = 0u32;

    for pixel in image.pixels().step_by(step) {
        let [red, green, blue] = pixel.0;
        histogram[0][red as usize] += 1;
        histogram[1][green as usize] += 1;
        histogram[2][blue as usize] += 1;

        let peak = red.max(green).max(blue);
        if peak >= HIGHLIGHT_THRESHOLD {
            highlights += 1;
        }
        if peak <= SHADOW_THRESHOLD {
            shadows += 1;
        }
        sampled += 1;
    }

    let (width, height) = fit_within(
        image.width(),
        image.height(),
        THUMBNAIL_MAX_WIDTH,
        THUMBNAIL_MAX_HEIGHT,
    );

    Analysis {
        width: image.width(),
        height: image.height(),
        histogram,
        clipped_highlights: ratio(highlights, sampled),
        clipped_shadows: ratio(shadows, sampled),
        thumbnail: thumbnail(image, width, height),
    }
}

fn ratio(count: u32, total: u32) -> f32 {
    if total == 0 {
        return 0.0;
    }
    count as f32 / total as f32
}

fn fit_within(width: u32, height: u32, max_width: u32, max_height: u32) -> (u32, u32) {
    let horizontal = max_width as f64 / width as f64;
    let vertical = max_height as f64 / height as f64;
    let scale = horizontal.min(vertical).min(1.0);

    (
        ((width as f64 * scale).round() as u32).max(1),
        ((height as f64 * scale).round() as u32).max(1),
    )
}

#[cfg(test)]
mod tests {
    use image::{Rgb, RgbImage};
    use rstest::rstest;

    use super::{Analysis, THUMBNAIL_MAX_HEIGHT, THUMBNAIL_MAX_WIDTH, analyze_image, fit_within};

    fn filled(width: u32, height: u32, color: [u8; 3]) -> RgbImage {
        RgbImage::from_pixel(width, height, Rgb(color))
    }

    #[rstest]
    #[case::white([255, 255, 255], 1.0, 0.0)]
    #[case::just_below_the_highlight_threshold([253, 253, 253], 0.0, 0.0)]
    #[case::black([0, 0, 0], 0.0, 1.0)]
    #[case::just_above_the_shadow_threshold([2, 2, 2], 0.0, 0.0)]
    #[case::middle_grey([128, 128, 128], 0.0, 0.0)]
    #[case::blown_red_channel_only([255, 10, 10], 1.0, 0.0)]
    fn measures_clipping(#[case] color: [u8; 3], #[case] highlights: f32, #[case] shadows: f32) {
        let analysis = analyze_image(&filled(64, 64, color));
        assert_eq!(analysis.clipped_highlights, highlights);
        assert_eq!(analysis.clipped_shadows, shadows);
    }

    #[test]
    fn counts_every_sample_into_each_channel() {
        let analysis = analyze_image(&filled(32, 16, [10, 20, 30]));

        assert_eq!(analysis.width, 32);
        assert_eq!(analysis.height, 16);
        assert_eq!(analysis.histogram[0][10], 512);
        assert_eq!(analysis.histogram[1][20], 512);
        assert_eq!(analysis.histogram[2][30], 512);
        assert_eq!(channel_total(&analysis, 0), 512);
        assert_eq!(channel_total(&analysis, 1), 512);
        assert_eq!(channel_total(&analysis, 2), 512);
    }

    #[test]
    fn subsamples_images_larger_than_the_target() {
        let analysis = analyze_image(&filled(2000, 2000, [64, 64, 64]));

        assert!(channel_total(&analysis, 0) < 4_000_000);
        assert_eq!(channel_total(&analysis, 0), channel_total(&analysis, 1));
        assert_eq!(analysis.width, 2000);
    }

    #[test]
    fn separates_the_channels_of_a_gradient() {
        let mut image = RgbImage::new(256, 1);
        for (x, _, pixel) in image.enumerate_pixels_mut() {
            *pixel = Rgb([x as u8, 0, 255]);
        }

        let analysis = analyze_image(&image);
        assert!(analysis.histogram[0].iter().all(|count| *count == 1));
        assert_eq!(analysis.histogram[1][0], 256);
        assert_eq!(analysis.histogram[2][255], 256);
        assert_eq!(analysis.clipped_highlights, 1.0);
    }

    #[rstest]
    #[case::landscape_4k(3840, 2160, 480, 270)]
    #[case::portrait(2160, 3840, 152, 270)]
    #[case::square(1000, 1000, 270, 270)]
    #[case::already_small(100, 50, 100, 50)]
    #[case::one_pixel(1, 1, 1, 1)]
    fn fits_thumbnails_inside_the_box(
        #[case] width: u32,
        #[case] height: u32,
        #[case] expected_width: u32,
        #[case] expected_height: u32,
    ) {
        let fitted = fit_within(width, height, THUMBNAIL_MAX_WIDTH, THUMBNAIL_MAX_HEIGHT);
        assert_eq!(fitted, (expected_width, expected_height));
    }

    #[test]
    fn produces_a_thumbnail_that_keeps_the_aspect_ratio() {
        let analysis = analyze_image(&filled(1920, 1080, [40, 80, 120]));

        assert_eq!(analysis.thumbnail.width(), THUMBNAIL_MAX_WIDTH);
        assert_eq!(analysis.thumbnail.height(), THUMBNAIL_MAX_HEIGHT);
    }

    fn channel_total(analysis: &Analysis, channel: usize) -> u32 {
        analysis.histogram[channel].iter().sum()
    }
}
