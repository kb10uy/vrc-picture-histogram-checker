mod analysis;
mod config;
mod manifest;
mod notification;
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
    time::{Duration, Instant},
};

use thiserror::Error as ThisError;

use crate::{
    config::Config,
    manifest::APP_KEY,
    notification::Notification,
    openvr::{Context, Overlay, Transform},
    panel::PanelRenderer,
    watcher::PhotoWatcher,
};

const USAGE: &str = concat!(
    "usage: vrc-pichistcheck [--register | --unregister]\n",
    "\n",
    "  (no option)   watch the picture folder and show the panel in SteamVR\n",
    "  --register    let SteamVR start this application on its own\n",
    "  --unregister  undo --register",
);
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
    Manifest(#[from] manifest::Error),

    #[error(transparent)]
    Watch(#[from] watcher::Error),

    #[error(transparent)]
    OpenVr(#[from] openvr::Error),
}

fn main() {
    let result = match std::env::args().nth(1).as_deref() {
        None => run(),
        Some("--register") => set_auto_launch(true),
        Some("--unregister") => set_auto_launch(false),
        Some("--help" | "-h") => {
            println!("{USAGE}");
            return;
        }
        Some(argument) => {
            eprintln!("unknown option: {argument}\n\n{USAGE}");
            exit(2);
        }
    };

    if let Err(error) = result {
        eprintln!("{error}");
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
        context.set_application_auto_launch(APP_KEY, true)?;
    } else {
        context.set_application_auto_launch(APP_KEY, false)?;
        context.remove_application_manifest(&manifest)?;
    }

    println!("manifest: {}", manifest.display());
    println!(
        "SteamVR starts this application on its own: {}",
        context.application_auto_launch(APP_KEY)
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

    let mut notification = Notification::new(
        config.display_seconds,
        config.fade_seconds,
        config.panel_alpha,
    );
    let mut visible = false;

    while running.load(Ordering::Relaxed) {
        for path in watcher.poll() {
            if show(&renderer, &overlay, &path)? {
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

fn show(renderer: &PanelRenderer, overlay: &Overlay<'_>, path: &Path) -> Result<bool, Error> {
    let analysis = match analysis::analyze(path) {
        Ok(analysis) => analysis,
        Err(error) => {
            eprintln!("{error}");
            return Ok(false);
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
            return Ok(false);
        }
    };

    overlay.set_raw_rgba8(&frame.pixels, frame.width, frame.height)?;
    Ok(true)
}
