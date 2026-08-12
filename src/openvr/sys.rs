#![allow(non_snake_case)]

use std::ffi::{CStr, c_char, c_void};

pub type EvrInitError = i32;
pub type EvrOverlayError = u32;
pub type EvrApplicationError = u32;
pub type EvrApplicationType = u32;
pub type TrackingUniverseOrigin = u32;
pub type TrackedDeviceIndex = u32;
pub type VrOverlayHandle = u64;

pub const APPLICATION_TYPE_OVERLAY: EvrApplicationType = 2;
pub const APPLICATION_TYPE_UTILITY: EvrApplicationType = 4;
pub const TRACKED_DEVICE_INDEX_HMD: TrackedDeviceIndex = 0;
pub const INIT_ERROR_NONE: EvrInitError = 0;
pub const OVERLAY_ERROR_NONE: EvrOverlayError = 0;
pub const APPLICATION_ERROR_NONE: EvrApplicationError = 0;
pub const EVENT_QUIT: u32 = 700;

pub const IVR_SYSTEM_FN_TABLE: &CStr = c"FnTable:IVRSystem_026";
pub const IVR_OVERLAY_FN_TABLE: &CStr = c"FnTable:IVROverlay_028";
pub const IVR_APPLICATIONS_FN_TABLE: &CStr = c"FnTable:IVRApplications_008";

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HmdMatrix34 {
    pub m: [[f32; 4]; 3],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct VrEvent {
    pub event_type: u32,
    pub tracked_device_index: TrackedDeviceIndex,
    pub event_age_seconds: f32,
    pub data: [u64; 6],
}

const _: () = assert!(size_of::<VrEvent>() == 64);
const _: () = assert!(size_of::<HmdMatrix34>() == 48);

#[repr(C)]
pub struct IvrSystemFnTable {
    pub GetRecommendedRenderTargetSize: *const c_void,
    pub GetProjectionMatrix: *const c_void,
    pub GetProjectionRaw: *const c_void,
    pub ComputeDistortion: *const c_void,
    pub ComputeDistortionSet: *const c_void,
    pub GetEyeToHeadTransform: *const c_void,
    pub GetTimeSinceLastVsync: *const c_void,
    pub GetD3D9AdapterIndex: *const c_void,
    pub GetDXGIOutputInfo: *const c_void,
    pub GetOutputDevice: *const c_void,
    pub IsDisplayOnDesktop: *const c_void,
    pub SetDisplayVisibility: *const c_void,
    pub GetDeviceToAbsoluteTrackingPose: *const c_void,
    pub GetSeatedZeroPoseToStandingAbsoluteTrackingPose: *const c_void,
    pub GetRawZeroPoseToStandingAbsoluteTrackingPose: *const c_void,
    pub GetSortedTrackedDeviceIndicesOfClass: *const c_void,
    pub GetTrackedDeviceActivityLevel: *const c_void,
    pub ApplyTransform: *const c_void,
    pub GetTrackedDeviceIndexForControllerRole: *const c_void,
    pub GetControllerRoleForTrackedDeviceIndex: *const c_void,
    pub GetTrackedDeviceClass: *const c_void,
    pub IsTrackedDeviceConnected: *const c_void,
    pub GetBoolTrackedDeviceProperty: *const c_void,
    pub GetFloatTrackedDeviceProperty: *const c_void,
    pub GetInt32TrackedDeviceProperty: *const c_void,
    pub GetUint64TrackedDeviceProperty: *const c_void,
    pub GetMatrix34TrackedDeviceProperty: *const c_void,
    pub GetArrayTrackedDeviceProperty: *const c_void,
    pub GetStringTrackedDeviceProperty: *const c_void,
    pub GetPropErrorNameFromEnum: *const c_void,
    pub PollNextEvent: Option<unsafe extern "system" fn(*mut VrEvent, u32) -> bool>,
    pub PollNextEventWithPose: *const c_void,
    pub PollNextEventWithPoseAndOverlays: *const c_void,
    pub GetEventTypeNameFromEnum: *const c_void,
    pub GetHiddenAreaMesh: *const c_void,
    pub GetEyeTrackedFoveationCenter: *const c_void,
    pub GetEyeTrackedFoveationCenterForProjection: *const c_void,
    pub GetControllerState: *const c_void,
    pub GetControllerStateWithPose: *const c_void,
    pub TriggerHapticPulse: *const c_void,
    pub GetButtonIdNameFromEnum: *const c_void,
    pub GetControllerAxisTypeNameFromEnum: *const c_void,
    pub IsInputAvailable: *const c_void,
    pub IsSteamVRDrawingControllers: *const c_void,
    pub ShouldApplicationPause: *const c_void,
    pub ShouldApplicationReduceRenderingWork: *const c_void,
    pub PerformFirmwareUpdate: *const c_void,
    pub AcknowledgeQuit_Exiting: Option<unsafe extern "system" fn()>,
    pub GetAppContainerFilePaths: *const c_void,
    pub GetRuntimeVersion: *const c_void,
    pub SetSDKVersion: *const c_void,
}

#[repr(C)]
pub struct IvrApplicationsFnTable {
    pub AddApplicationManifest:
        Option<unsafe extern "system" fn(*const c_char, bool) -> EvrApplicationError>,
    pub RemoveApplicationManifest:
        Option<unsafe extern "system" fn(*const c_char) -> EvrApplicationError>,
    pub IsApplicationInstalled: Option<unsafe extern "system" fn(*const c_char) -> bool>,
    pub GetApplicationCount: *const c_void,
    pub GetApplicationKeyByIndex: *const c_void,
    pub GetApplicationKeyByProcessId: *const c_void,
    pub LaunchApplication: *const c_void,
    pub LaunchTemplateApplication: *const c_void,
    pub LaunchApplicationFromMimeType: *const c_void,
    pub LaunchDashboardOverlay: *const c_void,
    pub CancelApplicationLaunch: *const c_void,
    pub IdentifyApplication: *const c_void,
    pub GetApplicationProcessId: *const c_void,
    pub GetApplicationsErrorNameFromEnum:
        Option<unsafe extern "system" fn(EvrApplicationError) -> *const c_char>,
    pub GetApplicationPropertyString: *const c_void,
    pub GetApplicationPropertyBool: *const c_void,
    pub GetApplicationPropertyUint64: *const c_void,
    pub SetApplicationAutoLaunch:
        Option<unsafe extern "system" fn(*const c_char, bool) -> EvrApplicationError>,
    pub GetApplicationAutoLaunch: Option<unsafe extern "system" fn(*const c_char) -> bool>,
    pub SetDefaultApplicationForMimeType: *const c_void,
    pub GetDefaultApplicationForMimeType: *const c_void,
    pub GetApplicationSupportedMimeTypes: *const c_void,
    pub GetApplicationsThatSupportMimeType: *const c_void,
    pub GetApplicationLaunchArguments: *const c_void,
    pub GetStartingApplication: *const c_void,
    pub GetSceneApplicationState: *const c_void,
    pub PerformApplicationPrelaunchCheck: *const c_void,
    pub GetSceneApplicationStateNameFromEnum: *const c_void,
    pub LaunchInternalProcess: *const c_void,
    pub RegisterSubprocess: *const c_void,
    pub GetCurrentSceneProcessId: *const c_void,
}

#[repr(C)]
pub struct IvrOverlayFnTable {
    pub FindOverlay: *const c_void,
    pub CreateOverlay: Option<
        unsafe extern "system" fn(
            *const c_char,
            *const c_char,
            *mut VrOverlayHandle,
        ) -> EvrOverlayError,
    >,
    pub CreateSubviewOverlay: *const c_void,
    pub DestroyOverlay: Option<unsafe extern "system" fn(VrOverlayHandle) -> EvrOverlayError>,
    pub GetOverlayKey: *const c_void,
    pub GetOverlayName: *const c_void,
    pub SetOverlayName: *const c_void,
    pub GetOverlayImageData: *const c_void,
    pub GetOverlayErrorNameFromEnum:
        Option<unsafe extern "system" fn(EvrOverlayError) -> *const c_char>,
    pub SetOverlayRenderingPid: *const c_void,
    pub GetOverlayRenderingPid: *const c_void,
    pub SetOverlayFlag: *const c_void,
    pub GetOverlayFlag: *const c_void,
    pub GetOverlayFlags: *const c_void,
    pub SetOverlayColor: *const c_void,
    pub GetOverlayColor: *const c_void,
    pub SetOverlayAlpha: Option<unsafe extern "system" fn(VrOverlayHandle, f32) -> EvrOverlayError>,
    pub GetOverlayAlpha: *const c_void,
    pub SetOverlayTexelAspect: *const c_void,
    pub GetOverlayTexelAspect: *const c_void,
    pub SetOverlaySortOrder: *const c_void,
    pub GetOverlaySortOrder: *const c_void,
    pub SetOverlayWidthInMeters:
        Option<unsafe extern "system" fn(VrOverlayHandle, f32) -> EvrOverlayError>,
    pub GetOverlayWidthInMeters: *const c_void,
    pub SetOverlayCurvature: *const c_void,
    pub GetOverlayCurvature: *const c_void,
    pub SetOverlayPreCurvePitch: *const c_void,
    pub GetOverlayPreCurvePitch: *const c_void,
    pub SetOverlayTextureColorSpace: *const c_void,
    pub GetOverlayTextureColorSpace: *const c_void,
    pub SetOverlayTextureBounds: *const c_void,
    pub GetOverlayTextureBounds: *const c_void,
    pub GetOverlayTransformType: *const c_void,
    pub SetOverlayTransformAbsolute: Option<
        unsafe extern "system" fn(
            VrOverlayHandle,
            TrackingUniverseOrigin,
            *const HmdMatrix34,
        ) -> EvrOverlayError,
    >,
    pub GetOverlayTransformAbsolute: *const c_void,
    pub SetOverlayTransformTrackedDeviceRelative: Option<
        unsafe extern "system" fn(
            VrOverlayHandle,
            TrackedDeviceIndex,
            *const HmdMatrix34,
        ) -> EvrOverlayError,
    >,
    pub GetOverlayTransformTrackedDeviceRelative: *const c_void,
    pub SetOverlayTransformTrackedDeviceComponent: *const c_void,
    pub GetOverlayTransformTrackedDeviceComponent: *const c_void,
    pub SetOverlayTransformCursor: *const c_void,
    pub GetOverlayTransformCursor: *const c_void,
    pub SetOverlayTransformProjection: *const c_void,
    pub SetSubviewPosition: *const c_void,
    pub ShowOverlay: Option<unsafe extern "system" fn(VrOverlayHandle) -> EvrOverlayError>,
    pub HideOverlay: Option<unsafe extern "system" fn(VrOverlayHandle) -> EvrOverlayError>,
    pub IsOverlayVisible: *const c_void,
    pub GetTransformForOverlayCoordinates: *const c_void,
    pub WaitFrameSync: *const c_void,
    pub PollNextOverlayEvent: *const c_void,
    pub GetOverlayInputMethod: *const c_void,
    pub SetOverlayInputMethod: *const c_void,
    pub GetOverlayMouseScale: *const c_void,
    pub SetOverlayMouseScale: *const c_void,
    pub ComputeOverlayIntersection: *const c_void,
    pub IsHoverTargetOverlay: *const c_void,
    pub SetOverlayIntersectionMask: *const c_void,
    pub TriggerLaserMouseHapticVibration: *const c_void,
    pub SetOverlayCursor: *const c_void,
    pub SetOverlayCursorPositionOverride: *const c_void,
    pub ClearOverlayCursorPositionOverride: *const c_void,
    pub SetOverlayTexture: *const c_void,
    pub ClearOverlayTexture: *const c_void,
    pub SetOverlayRaw: Option<
        unsafe extern "system" fn(VrOverlayHandle, *const c_void, u32, u32, u32) -> EvrOverlayError,
    >,
    pub SetOverlayFromFile: *const c_void,
    pub GetOverlayTexture: *const c_void,
    pub ReleaseNativeOverlayHandle: *const c_void,
    pub GetOverlayTextureSize: *const c_void,
    pub CreateDashboardOverlay: *const c_void,
    pub IsDashboardVisible: *const c_void,
    pub IsActiveDashboardOverlay: *const c_void,
    pub SetDashboardOverlaySceneProcess: *const c_void,
    pub GetDashboardOverlaySceneProcess: *const c_void,
    pub ShowDashboard: *const c_void,
    pub GetPrimaryDashboardDevice: *const c_void,
    pub ShowKeyboard: *const c_void,
    pub ShowKeyboardForOverlay: *const c_void,
    pub GetKeyboardText: *const c_void,
    pub HideKeyboard: *const c_void,
    pub SetKeyboardTransformAbsolute: *const c_void,
    pub SetKeyboardPositionForOverlay: *const c_void,
    pub ShowMessageOverlay: *const c_void,
    pub CloseMessageOverlay: *const c_void,
}
