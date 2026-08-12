mod error;
mod loader;
mod sys;

use std::{
    ffi::{CStr, CString},
    ptr::null,
};

use crate::openvr::{
    loader::Library,
    sys::{
        APPLICATION_TYPE_OVERLAY, EVENT_QUIT, EvrOverlayError, HmdMatrix34, INIT_ERROR_NONE,
        IVR_OVERLAY_FN_TABLE, IVR_SYSTEM_FN_TABLE, IvrOverlayFnTable, IvrSystemFnTable,
        OVERLAY_ERROR_NONE, TRACKED_DEVICE_INDEX_HMD, VrEvent, VrOverlayHandle,
    },
};

pub use crate::openvr::error::Error;

pub type Transform = [[f32; 4]; 3];

pub struct Context {
    library: Library,
    system: *const IvrSystemFnTable,
    overlay: *const IvrOverlayFnTable,
}

impl Context {
    pub fn init_overlay_app() -> Result<Self, Error> {
        let library = Library::load()?;

        let mut code = INIT_ERROR_NONE;
        unsafe { (library.init_internal)(&mut code, APPLICATION_TYPE_OVERLAY) };
        if code != INIT_ERROR_NONE {
            return Err(library.init_error(code));
        }

        let mut context = Context {
            library,
            system: null(),
            overlay: null(),
        };
        context.system = context.library.fn_table(IVR_SYSTEM_FN_TABLE)?;
        context.overlay = context.library.fn_table(IVR_OVERLAY_FN_TABLE)?;
        context.verify_fn_table()?;

        Ok(context)
    }

    pub fn create_overlay(&self, key: &str, name: &str) -> Result<Overlay<'_>, Error> {
        let key = CString::new(key).map_err(|_| Error::InvalidString {
            field: "overlay key",
        })?;
        let name = CString::new(name).map_err(|_| Error::InvalidString {
            field: "overlay name",
        })?;
        let create = self
            .overlay_table()
            .CreateOverlay
            .ok_or(Error::MissingFunction {
                name: "CreateOverlay",
            })?;

        let mut handle = 0;
        let code = unsafe { create(key.as_ptr(), name.as_ptr(), &mut handle) };
        self.check("CreateOverlay", code)?;

        Ok(Overlay {
            context: self,
            handle,
        })
    }

    pub fn poll_quit(&self) -> bool {
        let Some(poll) = self.system_table().PollNextEvent else {
            return false;
        };

        let mut quitting = false;
        let mut event = VrEvent::default();
        while unsafe { poll(&mut event, size_of::<VrEvent>() as u32) } {
            quitting |= event.event_type == EVENT_QUIT;
        }

        if quitting && let Some(acknowledge) = self.system_table().AcknowledgeQuit_Exiting {
            unsafe { acknowledge() };
        }
        quitting
    }

    fn system_table(&self) -> &IvrSystemFnTable {
        unsafe { &*self.system }
    }

    fn overlay_table(&self) -> &IvrOverlayFnTable {
        unsafe { &*self.overlay }
    }

    fn verify_fn_table(&self) -> Result<(), Error> {
        match self.overlay_error_name(OVERLAY_ERROR_NONE) {
            Some(name) if !name.is_empty() => Ok(()),
            returned => Err(Error::FnTableMismatch { returned }),
        }
    }

    fn check(&self, operation: &'static str, code: EvrOverlayError) -> Result<(), Error> {
        if code == OVERLAY_ERROR_NONE {
            return Ok(());
        }
        Err(Error::Overlay {
            operation,
            code,
            name: self
                .overlay_error_name(code)
                .unwrap_or_else(|| String::from("unknown")),
        })
    }

    fn overlay_error_name(&self, code: EvrOverlayError) -> Option<String> {
        let name_of = self.overlay_table().GetOverlayErrorNameFromEnum?;
        let pointer = unsafe { name_of(code) };
        if pointer.is_null() {
            return None;
        }
        Some(
            unsafe { CStr::from_ptr(pointer) }
                .to_string_lossy()
                .into_owned(),
        )
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { (self.library.shutdown_internal)() };
    }
}

pub struct Overlay<'a> {
    context: &'a Context,
    handle: VrOverlayHandle,
}

impl Overlay<'_> {
    pub fn set_raw_rgba8(&self, pixels: &[u8], width: u32, height: u32) -> Result<(), Error> {
        let expected = width as usize * height as usize * 4;
        if pixels.len() != expected {
            return Err(Error::FrameSize {
                width,
                height,
                expected,
                actual: pixels.len(),
            });
        }

        let set = self.table().SetOverlayRaw.ok_or(Error::MissingFunction {
            name: "SetOverlayRaw",
        })?;
        let code = unsafe { set(self.handle, pixels.as_ptr().cast(), width, height, 4) };
        self.context.check("SetOverlayRaw", code)
    }

    pub fn set_hmd_relative_transform(&self, transform: &Transform) -> Result<(), Error> {
        let set = self
            .table()
            .SetOverlayTransformTrackedDeviceRelative
            .ok_or(Error::MissingFunction {
                name: "SetOverlayTransformTrackedDeviceRelative",
            })?;
        let matrix = HmdMatrix34 { m: *transform };
        let code = unsafe { set(self.handle, TRACKED_DEVICE_INDEX_HMD, &matrix) };
        self.context
            .check("SetOverlayTransformTrackedDeviceRelative", code)
    }

    pub fn set_width_meters(&self, width: f32) -> Result<(), Error> {
        let set = self
            .table()
            .SetOverlayWidthInMeters
            .ok_or(Error::MissingFunction {
                name: "SetOverlayWidthInMeters",
            })?;
        let code = unsafe { set(self.handle, width) };
        self.context.check("SetOverlayWidthInMeters", code)
    }

    pub fn set_alpha(&self, alpha: f32) -> Result<(), Error> {
        let set = self.table().SetOverlayAlpha.ok_or(Error::MissingFunction {
            name: "SetOverlayAlpha",
        })?;
        let code = unsafe { set(self.handle, alpha.clamp(0.0, 1.0)) };
        self.context.check("SetOverlayAlpha", code)
    }

    pub fn set_visible(&self, visible: bool) -> Result<(), Error> {
        let (operation, function) = if visible {
            ("ShowOverlay", self.table().ShowOverlay)
        } else {
            ("HideOverlay", self.table().HideOverlay)
        };
        let show_or_hide = function.ok_or(Error::MissingFunction { name: operation })?;
        let code = unsafe { show_or_hide(self.handle) };
        self.context.check(operation, code)
    }

    fn table(&self) -> &IvrOverlayFnTable {
        self.context.overlay_table()
    }
}

impl Drop for Overlay<'_> {
    fn drop(&mut self) {
        if let Some(destroy) = self.table().DestroyOverlay {
            unsafe { destroy(self.handle) };
        }
    }
}
