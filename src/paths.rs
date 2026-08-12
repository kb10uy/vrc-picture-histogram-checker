use std::{
    env::current_exe,
    io::{Error, ErrorKind, Result},
    path::PathBuf,
};

pub fn beside_executable(file_name: &str) -> Result<PathBuf> {
    let executable = current_exe()?;
    let directory = executable
        .parent()
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "the executable path has no parent"))?;
    Ok(directory.join(file_name))
}

pub fn executable_file_name() -> Result<String> {
    let executable = current_exe()?;
    let name = executable
        .file_name()
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "the executable path has no file name"))?;
    Ok(name.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::{beside_executable, executable_file_name};

    #[test]
    fn resolves_siblings_of_the_executable() {
        let sibling = beside_executable("config.toml").expect("should resolve");
        let executable = std::env::current_exe().expect("should resolve");

        assert_eq!(sibling.parent(), executable.parent());
        assert_eq!(
            sibling.file_name().and_then(|name| name.to_str()),
            Some("config.toml")
        );
    }

    #[test]
    fn reports_the_name_of_the_executable() {
        let name = executable_file_name().expect("should resolve");

        assert!(!name.is_empty());
        assert!(!name.contains(std::path::MAIN_SEPARATOR));
    }
}
