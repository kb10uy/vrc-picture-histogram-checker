mod config;
mod openvr;

use std::{
    process::exit,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::sleep,
    time::Duration,
};

use thiserror::Error as ThisError;

use crate::{config::Config, openvr::Context, openvr::Transform};

const OVERLAY_KEY: &str = "org.kb10uy.vrc-pichistcheck.notification";
const OVERLAY_NAME: &str = "VRC Picture Histogram Checker";
const PANEL_PIXELS: u32 = 256;
const PANEL_OFFSET: Transform = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, -0.25],
    [0.0, 0.0, 1.0, -1.0],
];
const TICK: Duration = Duration::from_millis(33);

#[derive(Debug, ThisError)]
enum Error {
    #[error(transparent)]
    Config(#[from] config::Error),

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
        eprintln!("note: the directory does not exist yet");
    }

    let context = Context::init_overlay_app()?;
    let overlay = context.create_overlay(OVERLAY_KEY, OVERLAY_NAME)?;

    overlay.set_raw_rgba8(&checkerboard(PANEL_PIXELS), PANEL_PIXELS, PANEL_PIXELS)?;
    overlay.set_hmd_relative_transform(&PANEL_OFFSET)?;
    overlay.set_width_meters(config.panel_width_meters)?;
    overlay.set_alpha(1.0)?;
    overlay.set_visible(true)?;

    while running.load(Ordering::Relaxed) {
        if context.poll_quit() {
            break;
        }
        sleep(TICK);
    }

    Ok(())
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
