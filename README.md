# VRC Picture Histogram Checker

Watches for pictures saved by VRChat and shows a SteamVR overlay panel with the
thumbnail, an RGB histogram, and the proportion of clipped highlights and
shadows.

Windows with SteamVR (OpenVR) is the only supported environment.

## Running

Start it while SteamVR is running.

```console
cargo run
```

## The desktop window

The executable is built for the Windows subsystem, so starting it opens no
console. What it opens instead is a window holding the same panel the overlay
last showed, at its native 512x424 pixels, and it keeps showing it rather than
fading it out. Until the first picture arrives the window carries the paths it
resolved at startup, and a picture that cannot be read or drawn replaces the
panel with the reason. Closing the window stops the application.

Output from `--register`, `--unregister`, `--help` and `--version` still reaches
the console the application was started from. A Windows subsystem process is
handed no console of its own, so it attaches to the parent's, which is also why
the shell prompt comes back before the text is printed. Redirecting to a file or
a pipe works as usual. A failure that stops `run` from starting is reported in a
message box, because there may be no console to report it to.

`openvr_api.dll` is not bundled. It is loaded at runtime from the SteamVR
runtime recorded in `%LOCALAPPDATA%\openvr\openvrpaths.vrpath`, at
`bin\win64\openvr_api.dll`. If that lookup fails, the plain name is loaded
instead, which searches beside the executable and along `PATH`.

## Letting SteamVR start it

To have it come up together with SteamVR, put the executable where it is going
to live and then register it.

```console
vrc-picture-histogram-checker --register
```

This writes `vrc-pichistcheck.vrmanifest` beside the executable, hands it to
`AddApplicationManifest` from `IVRApplications`, and turns on
`SetApplicationAutoLaunch`. Once registered, the application appears in
SteamVR's list of startup add-ons, where the same switch can be flipped.

To undo it:

```console
vrc-picture-histogram-checker --unregister
```

What gets registered is **the absolute path of the executable at that moment**.
If you move the executable, or rebuild it somewhere else, run `--register`
again from its new location. Registering the copy under `target\release\` that
`cargo run` uses ties the registration to that path.

## Configuration

The configuration file is `config.toml` in the same folder as the executable.
It is created with defaults on the first run. Putting the executable somewhere
unwritable, such as under `Program Files`, means the file cannot be created, so
keep it somewhere you can write to.

```toml
watch_directory = 'J:\Pictures\VRChat'
panel_width_meters = 0.4
panel_alpha = 0.75
display_seconds = 3.0
fade_seconds = 1.0
```

| Key | Meaning |
| --- | --- |
| `watch_directory` | Folder to watch |
| `panel_width_meters` | Width of the panel in metres |
| `panel_alpha` | Opacity while the panel is up |
| `display_seconds` | How long the panel holds before it starts fading |
| `fade_seconds` | How long the fade takes |

`watch_directory` defaults to `VRChat` under the Pictures known folder. The path
is not assembled from `%USERPROFILE%\Pictures`, because Pictures is often
redirected to another drive or to OneDrive.

Each saved picture replaces the panel and restarts the countdown, so a burst of
shots leaves the last one on screen.

## About the OpenVR binding

The function tables in `src/openvr/sys.rs` were transcribed by hand from
`openvr_capi.h`, the official OpenVR header. These are the interface versions
they target.

| Interface | Version | Entries |
| --- | --- | --- |
| `IVRSystem` | `IVRSystem_026` | 51 |
| `IVROverlay` | `IVROverlay_028` | 82 |
| `IVRApplications` | `IVRApplications_008` | 31 |

A function table is a struct of function pointers, so **the order of the fields
must match the header exactly, including the entries that are never called**.
Get the order wrong and a different function is called. When updating against a
newer header, check the sequence of names it produces against the fields in
`sys.rs`:

```console
awk '/^struct VR_IVROverlay_FnTable/,/^};/' openvr_capi.h \
  | sed -n 's/.*\*\([A-Za-z0-9_]*\))(.*/\1/p'
```

Which interfaces a process may ask for depends on the application type it
passed to `VR_InitInternal`. `--register` and `--unregister` init as
`VRApplication_Utility` so that they never bring up a scene session, and a
utility application is handed `IVRApplications` but not `IVRSystem` or
`IVROverlay`: asking for either fails with
`VRInitError_Init_NotAvailableToUtilityApps`. `Context::init` therefore takes
the system and overlay tables only for `VRApplication_Overlay`, and the
accessors for them return an error rather than dereferencing a null pointer.

The calling convention is whatever `OPENVR_FNTABLE_CALLTYPE` expands to, which
is `__stdcall` on Win32, so the Rust side declares `extern "system"`. The
functions exported by the DLL, such as `VR_InitInternal`, are `__cdecl` and are
declared `extern "C"` instead.

At startup `GetOverlayErrorNameFromEnum` is called and its result checked, which
catches an obviously misaligned function table before a wrong call is made.

## License

Dual licensed under MIT or Apache-2.0.
