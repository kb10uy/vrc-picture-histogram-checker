use std::{
    collections::HashMap,
    fs::OpenOptions,
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::mpsc::{Receiver, channel},
    time::{Duration, Instant},
};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher, recommended_watcher};
use thiserror::Error as ThisError;

const PHOTO_PREFIX: &str = "VRChat_";
const PHOTO_EXTENSION: &str = "png";
const ABANDON_AFTER: Duration = Duration::from_secs(30);

#[derive(Debug, ThisError)]
pub enum WatcherError {
    #[error("failed to watch {path:?}")]
    Watch {
        path: PathBuf,
        #[source]
        source: notify::Error,
    },
}

pub struct PhotoWatcher {
    _watcher: RecommendedWatcher,
    incoming: Receiver<PathBuf>,
    pending: HashMap<PathBuf, Instant>,
}

impl PhotoWatcher {
    pub fn new(directory: &Path) -> Result<Self, WatcherError> {
        let (sender, incoming) = channel();
        let mut watcher = recommended_watcher(move |event: notify::Result<notify::Event>| {
            let Ok(event) = event else {
                return;
            };
            if !matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                return;
            }
            for path in event.paths.into_iter().filter(|path| is_photo(path)) {
                let _ = sender.send(path);
            }
        })
        .map_err(|source| WatcherError::Watch {
            path: directory.to_path_buf(),
            source,
        })?;

        watcher
            .watch(directory, RecursiveMode::Recursive)
            .map_err(|source| WatcherError::Watch {
                path: directory.to_path_buf(),
                source,
            })?;

        Ok(PhotoWatcher {
            _watcher: watcher,
            incoming,
            pending: HashMap::new(),
        })
    }

    pub fn poll(&mut self) -> Vec<PathBuf> {
        while let Ok(path) = self.incoming.try_recv() {
            self.pending.entry(path).or_insert_with(Instant::now);
        }

        let now = Instant::now();
        let mut completed = Vec::new();
        self.pending.retain(|path, first_seen| {
            if is_closed_by_writer(path) {
                completed.push(path.clone());
                return false;
            }
            now.duration_since(*first_seen) < ABANDON_AFTER
        });
        completed.sort();
        completed
    }
}

fn is_photo(path: &Path) -> bool {
    let in_month_directory = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .is_some_and(is_month_directory);
    if !in_month_directory {
        return false;
    }

    let named_like_a_photo = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(PHOTO_PREFIX));
    let is_png = path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(PHOTO_EXTENSION));

    named_like_a_photo && is_png
}

fn is_month_directory(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() == 7
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..].iter().all(u8::is_ascii_digit)
}

fn is_closed_by_writer(path: &Path) -> bool {
    OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(path)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use rstest::rstest;

    use super::{is_month_directory, is_photo};

    #[rstest]
    #[case::current(r"J:\Pictures\VRChat\2026-08\VRChat_2026-08-01_01-40-44.004_3840x2160.png")]
    #[case::uppercase_extension(r"J:\Pictures\VRChat\2026-08\VRChat_2026-08-01_01-40-44.PNG")]
    fn accepts_photos_written_by_vrchat(#[case] path: &str) {
        assert!(is_photo(Path::new(path)));
    }

    #[rstest]
    #[case::manually_sorted(r"J:\Pictures\VRChat\_misc\VRChat_2026-08-01_01-40-44.004.png")]
    #[case::directly_in_the_root(r"J:\Pictures\VRChat\VRChat_2026-08-01_01-40-44.004.png")]
    #[case::another_application(r"J:\Pictures\VRChat\2026-08\Screenshot_2026-08-01.png")]
    #[case::not_a_png(r"J:\Pictures\VRChat\2026-08\VRChat_2026-08-01_01-40-44.004.txt")]
    #[case::no_extension(r"J:\Pictures\VRChat\2026-08\VRChat_2026-08-01_01-40-44")]
    fn rejects_everything_else(#[case] path: &str) {
        assert!(!is_photo(Path::new(path)));
    }

    #[rstest]
    #[case("2026-08", true)]
    #[case("1999-12", true)]
    #[case("_misc", false)]
    #[case("2026-8", false)]
    #[case("2026-08-01", false)]
    #[case("20260-8", false)]
    #[case("abcd-ef", false)]
    fn recognizes_month_directories(#[case] name: &str, #[case] expected: bool) {
        assert_eq!(is_month_directory(name), expected);
    }
}
