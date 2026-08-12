use std::path::PathBuf;

use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum OpenvrError {
    #[error("failed to load {path:?}")]
    Library {
        path: PathBuf,
        #[source]
        source: libloading::Error,
    },

    #[error("openvr_api.dll does not export {name}")]
    Symbol {
        name: String,
        #[source]
        source: libloading::Error,
    },

    #[error("OpenVR initialization failed: {symbol} ({code}): {description}")]
    Init {
        code: i32,
        symbol: String,
        description: String,
    },

    #[error("OpenVR interface {version} is unavailable: {symbol} ({code})")]
    Interface {
        version: String,
        code: i32,
        symbol: String,
    },

    #[error("the OpenVR function table does not provide {name}")]
    MissingFunction { name: &'static str },

    #[error("{operation} failed: {name} ({code})")]
    Call {
        operation: &'static str,
        code: u32,
        name: String,
    },

    #[error(
        "the OpenVR function table layout does not match the expected interface version \
         (GetOverlayErrorNameFromEnum returned {returned:?})"
    )]
    FnTableMismatch { returned: Option<String> },

    #[error("{field} must not contain a NUL byte")]
    InvalidString { field: &'static str },

    #[error("a {width}x{height} RGBA8 frame needs {expected} bytes but {actual} were given")]
    FrameSize {
        width: u32,
        height: u32,
        expected: usize,
        actual: usize,
    },
}
