mod analysis;
mod config;
mod openvr;
mod watcher;

use std::{
    path::Path,
    process::exit,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::sleep,
    time::Duration,
};

use thiserror::Error as ThisError;

use crate::{
    analysis::{Analysis, BINS},
    config::Config,
    openvr::{Context, Transform},
    watcher::PhotoWatcher,
};

const OVERLAY_KEY: &str = "org.kb10uy.vrc-pichistcheck.notification";
const OVERLAY_NAME: &str = "VRC Picture Histogram Checker";
const PANEL_PIXELS: u32 = 256;
const PANEL_OFFSET: Transform = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, -0.25],
    [0.0, 0.0, 1.0, -1.0],
];
const TICK: Duration = Duration::from_millis(33);
const SUMMARY_BUCKETS: usize = 32;
const SUMMARY_HEIGHT: usize = 8;

#[derive(Debug, ThisError)]
enum Error {
    #[error(transparent)]
    Config(#[from] config::Error),

    #[error("{0} is not a directory")]
    NotADirectory(String),

    #[error(transparent)]
    Watch(#[from] watcher::Error),

    #[error(transparent)]
    OpenVr(#[from] openvr::Error),
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        exit(1);
    }
}

fn run() -> Result<(), Error> {
    let running = Arc::new(AtomicBool::new(true));
    let interrupted = Arc::clone(&running);
    ctrlc::set_handler(move || interrupted.store(false, Ordering::Relaxed))
        .expect("failed to install the Ctrl-C handler");

    let config_path = Config::path()?;
    let config = Config::load_or_create(&config_path)?;
    println!("configuration: {}", config_path.display());
    println!("watching: {}", config.watch_directory.display());
    if !config.watch_directory.is_dir() {
        return Err(Error::NotADirectory(
            config.watch_directory.display().to_string(),
        ));
    }
    let mut watcher = PhotoWatcher::new(&config.watch_directory)?;

    let context = Context::init_overlay_app()?;
    let overlay = context.create_overlay(OVERLAY_KEY, OVERLAY_NAME)?;

    overlay.set_raw_rgba8(&checkerboard(PANEL_PIXELS), PANEL_PIXELS, PANEL_PIXELS)?;
    overlay.set_hmd_relative_transform(&PANEL_OFFSET)?;
    overlay.set_width_meters(config.panel_width_meters)?;
    overlay.set_alpha(1.0)?;
    overlay.set_visible(true)?;

    while running.load(Ordering::Relaxed) {
        for path in watcher.poll() {
            match analysis::analyze(&path) {
                Ok(analysis) => report(&path, &analysis),
                Err(error) => eprintln!("{error}"),
            }
        }
        if context.poll_quit() {
            break;
        }
        sleep(TICK);
    }

    Ok(())
}

fn report(path: &Path, analysis: &Analysis) {
    let name = path.file_name().unwrap_or(path.as_os_str());
    println!(
        "{} ({}x{}, {} samples)",
        name.display(),
        analysis.width,
        analysis.height,
        analysis.sampled_pixels
    );
    println!(
        "  highlights {:.2}%  shadows {:.2}%",
        analysis.clipped_highlights * 100.0,
        analysis.clipped_shadows * 100.0
    );
    for (channel, label) in ["R", "G", "B"].iter().enumerate() {
        println!("  {label} {}", sparkline(&analysis.histogram[channel]));
    }
}

fn sparkline(histogram: &[u32; BINS]) -> String {
    const BLOCKS: [char; SUMMARY_HEIGHT] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

    let width = BINS / SUMMARY_BUCKETS;
    let buckets: Vec<u32> = histogram
        .chunks(width)
        .map(|bins| bins.iter().sum())
        .collect();
    let peak = buckets.iter().copied().max().unwrap_or(0);
    if peak == 0 {
        return String::new();
    }

    buckets
        .iter()
        .map(|count| {
            let scaled = (*count as f64 / peak as f64).sqrt() * (SUMMARY_HEIGHT - 1) as f64;
            BLOCKS[scaled.round() as usize]
        })
        .collect()
}

fn checkerboard(size: u32) -> Vec<u8> {
    let cell = size / 8;
    let mut pixels = Vec::with_capacity(size as usize * size as usize * 4);
    for y in 0..size {
        for x in 0..size {
            let luminance = if (x / cell + y / cell).is_multiple_of(2) {
                224
            } else {
                48
            };
            pixels.extend_from_slice(&[luminance, luminance, luminance, 255]);
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::checkerboard;

    #[rstest]
    #[case(8)]
    #[case(64)]
    #[case(256)]
    fn checkerboard_has_rgba8_length(#[case] size: u32) {
        assert_eq!(checkerboard(size).len(), size as usize * size as usize * 4);
    }

    #[test]
    fn checkerboard_alternates_between_cells() {
        let size = 64;
        let pixels = checkerboard(size);
        let at = |x: usize, y: usize| pixels[(y * size as usize + x) * 4];
        assert_ne!(at(0, 0), at(size as usize / 8, 0));
        assert_eq!(at(0, 0), at(size as usize / 4, 0));
    }
}
