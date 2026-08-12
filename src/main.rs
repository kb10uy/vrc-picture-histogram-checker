mod analysis;
mod config;
mod openvr;
mod panel;
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
    config::Config,
    openvr::{Context, Overlay, Transform},
    panel::PanelRenderer,
    watcher::PhotoWatcher,
};

const OVERLAY_KEY: &str = "org.kb10uy.vrc-pichistcheck.notification";
const OVERLAY_NAME: &str = "VRC Picture Histogram Checker";
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

    let renderer = PanelRenderer::new();
    let context = Context::init_overlay_app()?;
    let overlay = context.create_overlay(OVERLAY_KEY, OVERLAY_NAME)?;

    overlay.set_hmd_relative_transform(&PANEL_OFFSET)?;
    overlay.set_width_meters(config.panel_width_meters)?;
    overlay.set_alpha(1.0)?;

    while running.load(Ordering::Relaxed) {
        for path in watcher.poll() {
            show(&renderer, &overlay, &path)?;
        }
        if context.poll_quit() {
            break;
        }
        sleep(TICK);
    }

    Ok(())
}

fn show(renderer: &PanelRenderer, overlay: &Overlay<'_>, path: &Path) -> Result<(), Error> {
    let analysis = match analysis::analyze(path) {
        Ok(analysis) => analysis,
        Err(error) => {
            eprintln!("{error}");
            return Ok(());
        }
    };

    println!(
        "{}: highlight {:.2}%  shadow {:.2}%",
        path.file_name().unwrap_or(path.as_os_str()).display(),
        analysis.clipped_highlights * 100.0,
        analysis.clipped_shadows * 100.0
    );

    let frame = match renderer.render(&analysis) {
        Ok(frame) => frame,
        Err(error) => {
            eprintln!("{error}");
            return Ok(());
        }
    };

    overlay.set_raw_rgba8(&frame.pixels, frame.width, frame.height)?;
    overlay.set_visible(true)?;
    Ok(())
}
