use std::{
    ffi::OsString,
    fs::{create_dir_all, read_to_string, write},
    os::windows::ffi::OsStringExt,
    path::{Path, PathBuf},
    ptr::null_mut,
    slice::from_raw_parts,
};

use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;
use windows_sys::{
    Win32::{
        System::Com::CoTaskMemFree,
        UI::Shell::{FOLDERID_Pictures, SHGetKnownFolderPath},
    },
    core::PWSTR,
};

use crate::paths::beside_executable;

const CONFIG_FILE_NAME: &str = "config.toml";
const VRCHAT_DIRECTORY: &str = "VRChat";
const DEFAULT_PANEL_WIDTH_METERS: f32 = 0.4;
const DEFAULT_PANEL_ALPHA: f32 = 0.75;
const DEFAULT_DISPLAY_SECONDS: f32 = 3.0;
const DEFAULT_FADE_SECONDS: f32 = 1.0;

#[derive(Debug, ThisError)]
pub enum ConfigError {
    #[error("failed to locate the executable")]
    Executable(#[source] std::io::Error),

    #[error("failed to access {path:?}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{path:?} is not valid configuration")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("failed to serialize the default configuration")]
    Serialize(#[from] toml::ser::Error),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_watch_directory")]
    pub watch_directory: PathBuf,

    #[serde(default = "default_panel_width_meters")]
    pub panel_width_meters: f32,

    #[serde(default = "default_panel_alpha")]
    pub panel_alpha: f32,

    #[serde(default = "default_display_seconds")]
    pub display_seconds: f32,

    #[serde(default = "default_fade_seconds")]
    pub fade_seconds: f32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            watch_directory: default_watch_directory(),
            panel_width_meters: default_panel_width_meters(),
            panel_alpha: default_panel_alpha(),
            display_seconds: default_display_seconds(),
            fade_seconds: default_fade_seconds(),
        }
    }
}

impl Config {
    pub fn path() -> Result<PathBuf, ConfigError> {
        beside_executable(CONFIG_FILE_NAME).map_err(ConfigError::Executable)
    }

    pub fn load_or_create(path: &Path) -> Result<Self, ConfigError> {
        match read_to_string(path) {
            Ok(contents) => toml::from_str(&contents).map_err(|source| ConfigError::Parse {
                path: path.to_path_buf(),
                source,
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let config = Config::default();
                config.save(path)?;
                Ok(config)
            }
            Err(source) => Err(ConfigError::Io {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    fn save(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            create_dir_all(parent).map_err(|source| ConfigError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        write(path, toml::to_string_pretty(self)?).map_err(|source| ConfigError::Io {
            path: path.to_path_buf(),
            source,
        })
    }
}

fn default_panel_width_meters() -> f32 {
    DEFAULT_PANEL_WIDTH_METERS
}

fn default_panel_alpha() -> f32 {
    DEFAULT_PANEL_ALPHA
}

fn default_display_seconds() -> f32 {
    DEFAULT_DISPLAY_SECONDS
}

fn default_fade_seconds() -> f32 {
    DEFAULT_FADE_SECONDS
}

fn default_watch_directory() -> PathBuf {
    pictures_directory().join(VRCHAT_DIRECTORY)
}

fn pictures_directory() -> PathBuf {
    if let Some(directory) = known_pictures_directory() {
        return directory;
    }
    match std::env::var_os("USERPROFILE") {
        Some(profile) => Path::new(&profile).join("Pictures"),
        None => PathBuf::from("Pictures"),
    }
}

fn known_pictures_directory() -> Option<PathBuf> {
    let mut raw: PWSTR = null_mut();
    let result = unsafe { SHGetKnownFolderPath(&FOLDERID_Pictures, 0, null_mut(), &mut raw) };
    if result < 0 || raw.is_null() {
        return None;
    }

    let mut length = 0;
    while unsafe { *raw.add(length) } != 0 {
        length += 1;
    }
    let directory = OsString::from_wide(unsafe { from_raw_parts(raw, length) });
    unsafe { CoTaskMemFree(raw.cast()) };

    Some(PathBuf::from(directory))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use tempfile::tempdir;

    use super::{
        Config, DEFAULT_DISPLAY_SECONDS, DEFAULT_FADE_SECONDS, DEFAULT_PANEL_ALPHA,
        DEFAULT_PANEL_WIDTH_METERS, known_pictures_directory,
    };

    #[test]
    fn resolves_the_pictures_known_folder() {
        let directory = known_pictures_directory().expect("Pictures should be resolvable");
        assert!(directory.is_absolute());
    }

    #[rstest]
    #[case::complete(
        "watch_directory = 'D:\\Shots'\npanel_width_meters = 0.6\n",
        "D:\\Shots",
        0.6
    )]
    #[case::width_omitted(
        "watch_directory = 'D:\\Shots'\n",
        "D:\\Shots",
        DEFAULT_PANEL_WIDTH_METERS
    )]
    fn fills_omitted_fields_with_defaults(
        #[case] contents: &str,
        #[case] directory: &str,
        #[case] width: f32,
    ) {
        let config: Config = toml::from_str(contents).expect("should parse");
        assert_eq!(config.watch_directory.to_string_lossy(), directory);
        assert_eq!(config.panel_width_meters, width);
    }

    #[test]
    fn places_the_configuration_next_to_the_executable() {
        let path = Config::path().expect("should resolve");
        let executable = std::env::current_exe().expect("should resolve");

        assert_eq!(path.parent(), executable.parent());
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("config.toml")
        );
    }

    #[test]
    fn fills_the_presentation_defaults() {
        let config: Config = toml::from_str("watch_directory = 'D:\\Shots'\n").expect("parses");

        assert_eq!(config.panel_alpha, DEFAULT_PANEL_ALPHA);
        assert_eq!(config.display_seconds, DEFAULT_DISPLAY_SECONDS);
        assert_eq!(config.fade_seconds, DEFAULT_FADE_SECONDS);
    }

    #[test]
    fn rejects_unknown_fields() {
        let result = toml::from_str::<Config>("panel_widht_meters = 0.6\n");
        assert!(result.is_err());
    }

    #[test]
    fn creates_the_file_when_it_is_missing() {
        let directory = tempdir().expect("should create a temporary directory");
        let path = directory.path().join("nested").join("config.toml");

        let created = Config::load_or_create(&path).expect("should create the configuration");
        assert!(path.is_file());
        assert_eq!(created, Config::default());

        let reloaded = Config::load_or_create(&path).expect("should load the configuration");
        assert_eq!(reloaded, created);
    }

    #[test]
    fn reports_the_path_of_broken_configuration() {
        let directory = tempdir().expect("should create a temporary directory");
        let path = directory.path().join("config.toml");
        std::fs::write(&path, "panel_width_meters = ").expect("should write");

        let error = Config::load_or_create(&path).expect_err("should fail to parse");
        assert!(error.to_string().contains("config.toml"));
    }
}
