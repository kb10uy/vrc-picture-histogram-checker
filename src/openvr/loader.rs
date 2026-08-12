use std::{
    ffi::{CStr, OsString, c_char},
    fs::read_to_string,
    path::{Path, PathBuf},
};

use libloading::{Library as DynamicLibrary, Symbol};
use serde::Deserialize;

use crate::openvr::{
    error::Error,
    sys::{EvrApplicationType, EvrInitError, INIT_ERROR_NONE},
};

const DLL_FILE_NAME: &str = "openvr_api.dll";

#[derive(Deserialize)]
struct VrPaths {
    #[serde(default)]
    runtime: Vec<PathBuf>,
}

pub struct Library {
    _library: DynamicLibrary,
    pub init_internal: unsafe extern "C" fn(*mut EvrInitError, EvrApplicationType) -> isize,
    pub shutdown_internal: unsafe extern "C" fn(),
    pub get_generic_interface: unsafe extern "C" fn(*const c_char, *mut EvrInitError) -> isize,
    pub init_error_as_symbol: unsafe extern "C" fn(EvrInitError) -> *const c_char,
    pub init_error_as_description: unsafe extern "C" fn(EvrInitError) -> *const c_char,
}

impl Library {
    pub fn load() -> Result<Self, Error> {
        let path = installed_runtime_library().unwrap_or_else(|| PathBuf::from(DLL_FILE_NAME));
        let library = unsafe { DynamicLibrary::new(&path) }
            .map_err(|source| Error::Library { path, source })?;

        Ok(Library {
            init_internal: unsafe { symbol(&library, b"VR_InitInternal\0")? },
            shutdown_internal: unsafe { symbol(&library, b"VR_ShutdownInternal\0")? },
            get_generic_interface: unsafe { symbol(&library, b"VR_GetGenericInterface\0")? },
            init_error_as_symbol: unsafe { symbol(&library, b"VR_GetVRInitErrorAsSymbol\0")? },
            init_error_as_description: unsafe {
                symbol(&library, b"VR_GetVRInitErrorAsEnglishDescription\0")?
            },
            _library: library,
        })
    }

    pub fn fn_table<T>(&self, version: &CStr) -> Result<*const T, Error> {
        let mut code = INIT_ERROR_NONE;
        let pointer = unsafe { (self.get_generic_interface)(version.as_ptr(), &mut code) };
        if pointer == 0 || code != INIT_ERROR_NONE {
            return Err(Error::Interface {
                version: version.to_string_lossy().into_owned(),
                code,
                symbol: self.init_error_symbol(code),
            });
        }
        Ok(pointer as *const T)
    }

    pub fn init_error(&self, code: EvrInitError) -> Error {
        Error::Init {
            code,
            symbol: self.init_error_symbol(code),
            description: unsafe { owned_string((self.init_error_as_description)(code)) },
        }
    }

    fn init_error_symbol(&self, code: EvrInitError) -> String {
        unsafe { owned_string((self.init_error_as_symbol)(code)) }
    }
}

unsafe fn symbol<T: Copy>(library: &DynamicLibrary, name: &[u8]) -> Result<T, Error> {
    let symbol: Symbol<T> = unsafe { library.get(name) }.map_err(|source| Error::Symbol {
        name: String::from_utf8_lossy(&name[..name.len() - 1]).into_owned(),
        source,
    })?;
    Ok(*symbol)
}

unsafe fn owned_string(pointer: *const c_char) -> String {
    if pointer.is_null() {
        return String::from("<null>");
    }
    unsafe { CStr::from_ptr(pointer) }
        .to_string_lossy()
        .into_owned()
}

fn installed_runtime_library() -> Option<PathBuf> {
    let local_app_data = std::env::var_os("LOCALAPPDATA")?;
    runtime_library_paths(&local_app_data)
        .into_iter()
        .find(|path| path.is_file())
}

fn runtime_library_paths(local_app_data: &OsString) -> Vec<PathBuf> {
    let vrpath = Path::new(local_app_data)
        .join("openvr")
        .join("openvrpaths.vrpath");
    let Ok(contents) = read_to_string(vrpath) else {
        return Vec::new();
    };
    runtime_directories(&contents)
        .into_iter()
        .map(|directory| directory.join("bin").join("win64").join(DLL_FILE_NAME))
        .collect()
}

fn runtime_directories(contents: &str) -> Vec<PathBuf> {
    serde_json::from_str::<VrPaths>(contents)
        .map(|paths| paths.runtime)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::runtime_directories;

    #[rstest]
    #[case::single(
        r#"{"runtime": ["C:\\Steam\\SteamVR"], "version": 1}"#,
        &[r"C:\Steam\SteamVR"]
    )]
    #[case::multiple(
        r#"{"runtime": ["C:\\Steam\\SteamVR", "D:\\Other\\SteamVR"]}"#,
        &[r"C:\Steam\SteamVR", r"D:\Other\SteamVR"]
    )]
    #[case::empty(r#"{"runtime": []}"#, &[])]
    #[case::missing_key(r#"{"config": ["C:\\Steam\\config"]}"#, &[])]
    #[case::not_an_object(r#"[]"#, &[])]
    #[case::malformed(r#"{"runtime": "#, &[])]
    fn parses_runtime_directories(#[case] contents: &str, #[case] expected: &[&str]) {
        let directories = runtime_directories(contents);
        let actual: Vec<_> = directories
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect();
        assert_eq!(actual, expected);
    }
}
