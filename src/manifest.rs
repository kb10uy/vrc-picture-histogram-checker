use std::{
    env::current_exe,
    fs::write,
    io,
    path::{Path, PathBuf},
};

use serde::Serialize;
use thiserror::Error as ThisError;

pub const APP_KEY: &str = "org.kb10uy.vrc-pichistcheck";

const FILE_NAME: &str = "vrc-pichistcheck.vrmanifest";
const APPLICATION_NAME: &str = "VRC Picture Histogram Checker";
const DESCRIPTION: &str = "Shows the histogram of pictures saved by VRChat";

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("failed to locate the executable")]
    Executable(#[source] io::Error),

    #[error("the executable path has no file name")]
    NamelessExecutable,

    #[error("failed to write {path:?}")]
    Write {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to build the manifest")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Serialize)]
struct Manifest {
    source: &'static str,
    applications: [Application; 1],
}

#[derive(Serialize)]
struct Application {
    app_key: &'static str,
    launch_type: &'static str,
    binary_path_windows: String,
    is_dashboard_overlay: bool,
    strings: Strings,
}

#[derive(Serialize)]
struct Strings {
    en_us: Localized,
}

#[derive(Serialize)]
struct Localized {
    name: &'static str,
    description: &'static str,
}

pub fn path() -> Result<PathBuf, Error> {
    Ok(directory()?.join(FILE_NAME))
}

pub fn write_beside_executable() -> Result<PathBuf, Error> {
    let executable = current_exe().map_err(Error::Executable)?;
    let binary = executable
        .file_name()
        .ok_or(Error::NamelessExecutable)?
        .to_string_lossy()
        .into_owned();

    let path = path()?;
    write(&path, build(binary)?).map_err(|source| Error::Write {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

fn directory() -> Result<PathBuf, Error> {
    let executable = current_exe().map_err(Error::Executable)?;
    executable
        .parent()
        .map(Path::to_path_buf)
        .ok_or(Error::NamelessExecutable)
}

fn build(binary_path_windows: String) -> Result<String, Error> {
    let manifest = Manifest {
        source: "builtin",
        applications: [Application {
            app_key: APP_KEY,
            launch_type: "binary",
            binary_path_windows,
            is_dashboard_overlay: true,
            strings: Strings {
                en_us: Localized {
                    name: APPLICATION_NAME,
                    description: DESCRIPTION,
                },
            },
        }],
    };

    Ok(serde_json::to_string_pretty(&manifest)?)
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{APP_KEY, build, path};

    #[test]
    fn describes_the_application_as_an_overlay() {
        let document = build(String::from("vrc-pichistcheck.exe")).expect("should build");
        let manifest: Value = serde_json::from_str(&document).expect("should be valid JSON");
        let application = &manifest["applications"][0];

        assert_eq!(application["app_key"], APP_KEY);
        assert_eq!(application["launch_type"], "binary");
        assert_eq!(application["binary_path_windows"], "vrc-pichistcheck.exe");
        assert_eq!(application["is_dashboard_overlay"], true);
        assert!(application["strings"]["en_us"]["name"].as_str().is_some());
    }

    #[test]
    fn places_the_manifest_next_to_the_executable() {
        let manifest = path().expect("should resolve");
        let executable = std::env::current_exe().expect("should resolve");

        assert_eq!(manifest.parent(), executable.parent());
        assert_eq!(
            manifest
                .extension()
                .and_then(|extension| extension.to_str()),
            Some("vrmanifest")
        );
    }
}
