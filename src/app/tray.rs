use std::sync::LazyLock;

use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, POINT, WPARAM},
        UI::{
            Shell::{
                NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
                Shell_NotifyIconW,
            },
            WindowsAndMessaging::{
                AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, IDI_APPLICATION,
                LoadIconW, MF_STRING, PostMessageW, RegisterWindowMessageW, SW_RESTORE,
                SetForegroundWindow, ShowWindow, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON,
                TrackPopupMenu, WM_APP, WM_CLOSE, WM_CONTEXTMENU, WM_LBUTTONDBLCLK, WM_LBUTTONUP,
                WM_NULL, WM_RBUTTONUP,
            },
        },
    },
    core::{PCWSTR, w},
};

pub const CALLBACK: u32 = WM_APP + 1;
const ICON_ID: u32 = 1;
const MENU_SHOW: usize = 1;
const MENU_QUIT: usize = 2;

static TASKBAR_CREATED: LazyLock<u32> =
    LazyLock::new(|| unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) });

pub fn taskbar_created() -> u32 {
    *TASKBAR_CREATED
}

pub fn add(hwnd: HWND, tip: PCWSTR) {
    let mut data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: ICON_ID,
        uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
        uCallbackMessage: CALLBACK,
        hIcon: unsafe { LoadIconW(None, IDI_APPLICATION) }.unwrap_or_default(),
        ..NOTIFYICONDATAW::default()
    };
    let tip = unsafe { tip.as_wide() };
    for (slot, unit) in data.szTip.iter_mut().zip(tip.iter().take(127)) {
        *slot = *unit;
    }
    let _ = unsafe { Shell_NotifyIconW(NIM_ADD, &data) };
}

pub fn remove(hwnd: HWND) {
    let data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: ICON_ID,
        ..NOTIFYICONDATAW::default()
    };
    let _ = unsafe { Shell_NotifyIconW(NIM_DELETE, &data) };
}

pub fn handle(hwnd: HWND, lparam: LPARAM) {
    match (lparam.0 & 0xffff) as u32 {
        WM_LBUTTONUP | WM_LBUTTONDBLCLK => restore(hwnd),
        WM_RBUTTONUP | WM_CONTEXTMENU => match menu(hwnd) {
            MENU_SHOW => restore(hwnd),
            MENU_QUIT => {
                let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
            }
            _ => {}
        },
        _ => {}
    }
}

fn restore(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = SetForegroundWindow(hwnd);
    }
}

fn menu(hwnd: HWND) -> usize {
    let Ok(menu) = (unsafe { CreatePopupMenu() }) else {
        return 0;
    };
    let mut cursor = POINT::default();
    let chosen = unsafe {
        let _ = AppendMenuW(menu, MF_STRING, MENU_SHOW, w!("Show"));
        let _ = AppendMenuW(menu, MF_STRING, MENU_QUIT, w!("Quit"));
        let _ = GetCursorPos(&mut cursor);
        let _ = SetForegroundWindow(hwnd);
        let chosen = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_NONOTIFY,
            cursor.x,
            cursor.y,
            None,
            hwnd,
            None,
        );
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
        chosen
    };
    chosen.0 as usize
}
