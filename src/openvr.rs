mod error;
mod loader;
mod sys;

use std::{
    ffi::{CStr, CString, c_char},
    path::Path,
    ptr::null,
};

use crate::openvr::{
    loader::Library,
    sys::{
        APPLICATION_ERROR_NONE, APPLICATION_TYPE_OVERLAY, APPLICATION_TYPE_UTILITY, EVENT_QUIT,
        EvrApplicationError, EvrApplicationType, EvrOverlayError, HmdMatrix34, INIT_ERROR_NONE,
        IVR_APPLICATIONS_FN_TABLE, IVR_OVERLAY_FN_TABLE, IVR_SYSTEM_FN_TABLE,
        IvrApplicationsFnTable, IvrOverlayFnTable, IvrSystemFnTable, OVERLAY_ERROR_NONE,
        TRACKED_DEVICE_INDEX_HMD, VrEvent, VrOverlayHandle,
    },
};

pub use crate::openvr::error::OpenvrError;

pub type Transform = [[f32; 4]; 3];

pub struct Context {
    library: Library,
    system: *const IvrSystemFnTable,
    overlay: *const IvrOverlayFnTable,
    applications: *const IvrApplicationsFnTable,
}

impl Context {
    pub fn init_overlay_app() -> Result<Self, OpenvrError> {
        Context::init(APPLICATION_TYPE_OVERLAY)
    }

    pub fn init_utility() -> Result<Self, OpenvrError> {
        Context::init(APPLICATION_TYPE_UTILITY)
    }

    fn init(kind: EvrApplicationType) -> Result<Self, OpenvrError> {
        let library = Library::load()?;

        let mut code = INIT_ERROR_NONE;
        unsafe { (library.init_internal)(&mut code, kind) };
        if code != INIT_ERROR_NONE {
            return Err(library.init_error(code));
        }

        let mut context = Context {
            library,
            system: null(),
            overlay: null(),
            applications: null(),
        };
        if kind != APPLICATION_TYPE_UTILITY {
            context.system = context.library.fn_table(IVR_SYSTEM_FN_TABLE)?;
            context.overlay = context.library.fn_table(IVR_OVERLAY_FN_TABLE)?;
        }
        context.applications = context.library.fn_table(IVR_APPLICATIONS_FN_TABLE)?;
        context.verify_fn_tables()?;

        Ok(context)
    }

    pub fn add_application_manifest(&self, manifest: &Path) -> Result<(), OpenvrError> {
        let path = c_string(manifest.to_string_lossy().as_ref(), "manifest path")?;
        let add = self.applications_table()?.AddApplicationManifest.ok_or(
            OpenvrError::MissingFunction {
                name: "AddApplicationManifest",
            },
        )?;

        let code = unsafe { add(path.as_ptr(), false) };
        self.check_application("AddApplicationManifest", code)
    }

    pub fn remove_application_manifest(&self, manifest: &Path) -> Result<(), OpenvrError> {
        let path = c_string(manifest.to_string_lossy().as_ref(), "manifest path")?;
        let remove = self.applications_table()?.RemoveApplicationManifest.ok_or(
            OpenvrError::MissingFunction {
                name: "RemoveApplicationManifest",
            },
        )?;

        let code = unsafe { remove(path.as_ptr()) };
        self.check_application("RemoveApplicationManifest", code)
    }

    pub fn set_application_auto_launch(
        &self,
        app_key: &str,
        enabled: bool,
    ) -> Result<(), OpenvrError> {
        let key = c_string(app_key, "application key")?;
        let set = self.applications_table()?.SetApplicationAutoLaunch.ok_or(
            OpenvrError::MissingFunction {
                name: "SetApplicationAutoLaunch",
            },
        )?;

        let code = unsafe { set(key.as_ptr(), enabled) };
        self.check_application("SetApplicationAutoLaunch", code)
    }

    pub fn application_auto_launch(&self, app_key: &str) -> bool {
        let Ok(key) = c_string(app_key, "application key") else {
            return false;
        };
        let Some(get) = self
            .applications_table()
            .ok()
            .and_then(|applications| applications.GetApplicationAutoLaunch)
        else {
            return false;
        };
        unsafe { get(key.as_ptr()) }
    }

    pub fn create_overlay(&self, key: &str, name: &str) -> Result<Overlay<'_>, OpenvrError> {
        let key = c_string(key, "overlay key")?;
        let name = c_string(name, "overlay name")?;
        let create = self
            .overlay_table()?
            .CreateOverlay
            .ok_or(OpenvrError::MissingFunction {
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
        let Ok(system) = self.system_table() else {
            return false;
        };
        let Some(poll) = system.PollNextEvent else {
            return false;
        };

        let mut quitting = false;
        let mut event = VrEvent::default();
        while unsafe { poll(&mut event, size_of::<VrEvent>() as u32) } {
            quitting |= event.event_type == EVENT_QUIT;
        }

        if quitting && let Some(acknowledge) = system.AcknowledgeQuit_Exiting {
            unsafe { acknowledge() };
        }
        quitting
    }

    fn system_table(&self) -> Result<&IvrSystemFnTable, OpenvrError> {
        unsafe { self.system.as_ref() }
            .ok_or(OpenvrError::UnavailableInterface { name: "IVRSystem" })
    }

    fn overlay_table(&self) -> Result<&IvrOverlayFnTable, OpenvrError> {
        unsafe { self.overlay.as_ref() }
            .ok_or(OpenvrError::UnavailableInterface { name: "IVROverlay" })
    }

    fn applications_table(&self) -> Result<&IvrApplicationsFnTable, OpenvrError> {
        unsafe { self.applications.as_ref() }.ok_or(OpenvrError::UnavailableInterface {
            name: "IVRApplications",
        })
    }

    fn check_application(
        &self,
        operation: &'static str,
        code: EvrApplicationError,
    ) -> Result<(), OpenvrError> {
        if code == APPLICATION_ERROR_NONE {
            return Ok(());
        }
        Err(OpenvrError::Call {
            operation,
            code,
            name: self
                .application_error_name(code)
                .unwrap_or_else(|| String::from("unknown")),
        })
    }

    fn verify_fn_tables(&self) -> Result<(), OpenvrError> {
        if !self.overlay.is_null() {
            verify(
                "GetOverlayErrorNameFromEnum",
                self.overlay_error_name(OVERLAY_ERROR_NONE),
            )?;
        }
        verify(
            "GetApplicationsErrorNameFromEnum",
            self.application_error_name(APPLICATION_ERROR_NONE),
        )
    }

    fn check(&self, operation: &'static str, code: EvrOverlayError) -> Result<(), OpenvrError> {
        if code == OVERLAY_ERROR_NONE {
            return Ok(());
        }
        Err(OpenvrError::Call {
            operation,
            code,
            name: self
                .overlay_error_name(code)
                .unwrap_or_else(|| String::from("unknown")),
        })
    }

    fn overlay_error_name(&self, code: EvrOverlayError) -> Option<String> {
        let name_of = self.overlay_table().ok()?.GetOverlayErrorNameFromEnum?;
        owned_string(unsafe { name_of(code) })
    }

    fn application_error_name(&self, code: EvrApplicationError) -> Option<String> {
        let name_of = self
            .applications_table()
            .ok()?
            .GetApplicationsErrorNameFromEnum?;
        owned_string(unsafe { name_of(code) })
    }
}

fn verify(probe: &'static str, returned: Option<String>) -> Result<(), OpenvrError> {
    match returned {
        Some(name) if !name.is_empty() => Ok(()),
        returned => Err(OpenvrError::FnTableMismatch { probe, returned }),
    }
}

fn c_string(value: &str, field: &'static str) -> Result<CString, OpenvrError> {
    CString::new(value).map_err(|_| OpenvrError::InvalidString { field })
}

fn owned_string(pointer: *const c_char) -> Option<String> {
    if pointer.is_null() {
        return None;
    }
    Some(
        unsafe { CStr::from_ptr(pointer) }
            .to_string_lossy()
            .into_owned(),
    )
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
    pub fn set_raw_rgba8(&self, pixels: &[u8], width: u32, height: u32) -> Result<(), OpenvrError> {
        let expected = width as usize * height as usize * 4;
        if pixels.len() != expected {
            return Err(OpenvrError::FrameSize {
                width,
                height,
                expected,
                actual: pixels.len(),
            });
        }

        let set = self
            .table()?
            .SetOverlayRaw
            .ok_or(OpenvrError::MissingFunction {
                name: "SetOverlayRaw",
            })?;
        let code = unsafe { set(self.handle, pixels.as_ptr().cast(), width, height, 4) };
        self.context.check("SetOverlayRaw", code)
    }

    pub fn set_hmd_relative_transform(&self, transform: &Transform) -> Result<(), OpenvrError> {
        let set = self
            .table()?
            .SetOverlayTransformTrackedDeviceRelative
            .ok_or(OpenvrError::MissingFunction {
                name: "SetOverlayTransformTrackedDeviceRelative",
            })?;
        let matrix = HmdMatrix34 { m: *transform };
        let code = unsafe { set(self.handle, TRACKED_DEVICE_INDEX_HMD, &matrix) };
        self.context
            .check("SetOverlayTransformTrackedDeviceRelative", code)
    }

    pub fn set_width_meters(&self, width: f32) -> Result<(), OpenvrError> {
        let set = self
            .table()?
            .SetOverlayWidthInMeters
            .ok_or(OpenvrError::MissingFunction {
                name: "SetOverlayWidthInMeters",
            })?;
        let code = unsafe { set(self.handle, width) };
        self.context.check("SetOverlayWidthInMeters", code)
    }

    pub fn set_alpha(&self, alpha: f32) -> Result<(), OpenvrError> {
        let set = self
            .table()?
            .SetOverlayAlpha
            .ok_or(OpenvrError::MissingFunction {
                name: "SetOverlayAlpha",
            })?;
        let code = unsafe { set(self.handle, alpha.clamp(0.0, 1.0)) };
        self.context.check("SetOverlayAlpha", code)
    }

    pub fn set_visible(&self, visible: bool) -> Result<(), OpenvrError> {
        let table = self.table()?;
        let (operation, function) = if visible {
            ("ShowOverlay", table.ShowOverlay)
        } else {
            ("HideOverlay", table.HideOverlay)
        };
        let show_or_hide = function.ok_or(OpenvrError::MissingFunction { name: operation })?;
        let code = unsafe { show_or_hide(self.handle) };
        self.context.check(operation, code)
    }

    fn table(&self) -> Result<&IvrOverlayFnTable, OpenvrError> {
        self.context.overlay_table()
    }
}

impl Drop for Overlay<'_> {
    fn drop(&mut self) {
        if let Ok(table) = self.table()
            && let Some(destroy) = table.DestroyOverlay
        {
            unsafe { destroy(self.handle) };
        }
    }
}
