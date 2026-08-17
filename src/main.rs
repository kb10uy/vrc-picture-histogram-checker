#![windows_subsystem = "windows"]

mod app;
mod config;
mod openvr;
mod paths;

use std::{
    path::Path,
    process::exit,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::sleep,
    time::{Duration, Instant},
};

use clap::Parser;
use thiserror::Error as ThisError;

use crate::{
    app::{
        analysis, console, manifest,
        notification::Notification,
        panel::{PANEL_HEIGHT, PANEL_WIDTH, PanelRenderer},
        watcher::{PhotoWatcher, WatcherError},
        window::{Window, WindowError, show_error},
    },
    config::{Config, ConfigError},
    openvr::{Context, OpenvrError, Overlay, Transform},
};

const OVERLAY_KEY: &str = "org.kb10uy.vrc-pichistcheck.notification";
const OVERLAY_NAME: &str = "VRC Picture Histogram Checker";
const PANEL_OFFSET: Transform = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, -0.25],
    [0.0, 0.0, 1.0, -1.0],
];
const TICK: Duration = Duration::from_millis(33);

#[derive(Parser)]
#[command(version, about)]
struct Arguments {
    /// Let SteamVR start this application on its own
    #[arg(long, conflicts_with = "unregister")]
    register: bool,

    /// Undo --register
    #[arg(long)]
    unregister: bool,
}

#[derive(Debug, ThisError)]
enum Error {
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error("{0} is not a directory")]
    NotADirectory(String),

    #[error(transparent)]
    Manifest(#[from] manifest::ManifestError),

    #[error(transparent)]
    Watch(#[from] WatcherError),

    #[error(transparent)]
    Openvr(#[from] OpenvrError),

    #[error(transparent)]
    Window(#[from] WindowError),
}

fn main() {
    console::attach_to_parent();

    let arguments = Arguments::parse();
    let (result, graphical) = match (arguments.register, arguments.unregister) {
        (true, _) => (set_auto_launch(true), false),
        (_, true) => (set_auto_launch(false), false),
        _ => (run(), true),
    };

    if let Err(error) = result {
        eprintln!("{error}");
        if graphical {
            show_error(&error.to_string());
        }
        exit(1);
    }
}

fn set_auto_launch(enabled: bool) -> Result<(), Error> {
    let manifest = if enabled {
        manifest::write_beside_executable()?
    } else {
        manifest::path()?
    };

    let context = Context::init_utility()?;
    if enabled {
        context.add_application_manifest(&manifest)?;
        context.set_application_auto_launch(manifest::APP_KEY, true)?;
    } else {
        context.set_application_auto_launch(manifest::APP_KEY, false)?;
        context.remove_application_manifest(&manifest)?;
    }

    println!("manifest: {}", manifest.display());
    println!(
        "SteamVR starts this application on its own: {}",
        context.application_auto_launch(manifest::APP_KEY)
    );
    Ok(())
}

fn run() -> Result<(), Error> {
    let running = Arc::new(AtomicBool::new(true));
    let interrupted = Arc::clone(&running);
    ctrlc::set_handler(move || interrupted.store(false, Ordering::Relaxed))
        .expect("failed to install the Ctrl-C handler");

    let config_path = Config::path()?;
    let config = Config::load_or_create(&config_path)?;
    if !config.watch_directory.is_dir() {
        return Err(Error::NotADirectory(
            config.watch_directory.display().to_string(),
        ));
    }
    let mut watcher = PhotoWatcher::new(&config.watch_directory)?;

    let renderer = PanelRenderer::new();
    let window = Window::new(OVERLAY_NAME, PANEL_WIDTH, PANEL_HEIGHT)?;
    show_message(
        &renderer,
        &window,
        &format!(
            "configuration\n{}\n\nwatching\n{}",
            config_path.display(),
            config.watch_directory.display()
        ),
    );

    let context = Context::init_overlay_app()?;
    let overlay = context.create_overlay(OVERLAY_KEY, OVERLAY_NAME)?;

    overlay.set_hmd_relative_transform(&PANEL_OFFSET)?;
    overlay.set_width_meters(config.panel_width_meters)?;

    let mut notification = Notification::new(
        config.display_seconds,
        config.fade_seconds,
        config.panel_alpha,
    );
    let mut visible = false;

    while running.load(Ordering::Relaxed) {
        if !window.pump() {
            break;
        }

        for path in watcher.poll() {
            if show(&renderer, &overlay, &window, &path)? {
                notification.trigger(Instant::now());
            }
        }

        match notification.alpha(Instant::now()) {
            Some(alpha) => {
                overlay.set_alpha(alpha)?;
                if !visible {
                    overlay.set_visible(true)?;
                    visible = true;
                }
            }
            None if visible => {
                overlay.set_visible(false)?;
                visible = false;
            }
            None => {}
        }

        if context.poll_quit() {
            break;
        }
        sleep(TICK);
    }

    Ok(())
}

fn show(
    renderer: &PanelRenderer,
    overlay: &Overlay<'_>,
    window: &Window,
    path: &Path,
) -> Result<bool, Error> {
    let analysis = match analysis::analyze(path) {
        Ok(analysis) => analysis,
        Err(error) => {
            show_message(renderer, window, &error.to_string());
            return Ok(false);
        }
    };

    let frame = match renderer.render(&analysis) {
        Ok(frame) => frame,
        Err(error) => {
            show_message(renderer, window, &error.to_string());
            return Ok(false);
        }
    };

    overlay.set_raw_rgba8(&frame.pixels, frame.width, frame.height)?;
    window.set_frame(&frame);
    Ok(true)
}

fn show_message(renderer: &PanelRenderer, window: &Window, message: &str) {
    if let Ok(frame) = renderer.render_message(message) {
        window.set_frame(&frame);
    }
}
