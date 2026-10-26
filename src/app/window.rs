use std::cell::RefCell;

use thiserror::Error as ThisError;
use windows::{
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BeginPaint, CreateSolidBrush, DIB_RGB_COLORS,
            EndPaint, HDC, InvalidateRect, PAINTSTRUCT, SRCCOPY, StretchDIBits, UpdateWindow,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext},
            WindowsAndMessaging::{
                CREATESTRUCTW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow,
                DispatchMessageW, GWLP_USERDATA, GetClientRect, GetWindowLongPtrW, GetWindowRect,
                IDC_ARROW, LoadCursorW, MB_ICONERROR, MB_OK, MSG, MessageBoxW, PM_REMOVE,
                PeekMessageW, PostQuitMessage, RegisterClassW, SC_MINIMIZE, SW_HIDE, SW_SHOW,
                SWP_NOMOVE, SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos, ShowWindow,
                TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CREATE, WM_DESTROY, WM_PAINT,
                WM_QUIT, WM_SYSCOMMAND, WNDCLASSW, WS_CAPTION, WS_MINIMIZEBOX, WS_SYSMENU,
            },
        },
    },
    core::{Error as WindowsError, HSTRING, PCWSTR, w},
};

use crate::app::{panel::Frame, tray};

const CLASS_NAME: PCWSTR = w!("org.kb10uy.vrc-pichistcheck.window");
const CAPTION: PCWSTR = w!("VRC Picture Histogram Checker");
const STYLE: WINDOW_STYLE = WINDOW_STYLE(WS_CAPTION.0 | WS_SYSMENU.0 | WS_MINIMIZEBOX.0);
const BACKDROP: [u8; 3] = [0x14, 0x16, 0x1a];
const BACKDROP_BRUSH: COLORREF = COLORREF(0x001a_1614);

#[derive(Debug, ThisError)]
pub enum WindowError {
    #[error("failed to register the window class")]
    Class(#[source] WindowsError),

    #[error("failed to create the window")]
    Create(#[source] WindowsError),
}

struct Bitmap {
    pixels: Vec<u8>,
    width: i32,
    height: i32,
}

impl Bitmap {
    fn new(width: u32, height: u32) -> Self {
        let pixel = [BACKDROP[2], BACKDROP[1], BACKDROP[0], 0];
        Bitmap {
            pixels: pixel.repeat(width as usize * height as usize),
            width: width as i32,
            height: height as i32,
        }
    }

    fn replace_with(&mut self, frame: &Frame) {
        self.width = frame.width as i32;
        self.height = frame.height as i32;
        self.pixels.clear();
        self.pixels.reserve(frame.pixels.len());
        for pixel in frame.pixels.chunks_exact(4) {
            let alpha = pixel[3] as u32;
            self.pixels.extend_from_slice(&[
                blend(pixel[2], BACKDROP[2], alpha),
                blend(pixel[1], BACKDROP[1], alpha),
                blend(pixel[0], BACKDROP[0], alpha),
                0,
            ]);
        }
    }
}

pub struct Window {
    hwnd: HWND,
    bitmap: *mut RefCell<Bitmap>,
}

impl Window {
    pub fn new(title: &str, width: u32, height: u32, visible: bool) -> Result<Self, WindowError> {
        let _ =
            unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };

        let instance = unsafe { GetModuleHandleW(None) }.map_err(WindowError::Class)?;
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance.into(),
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.map_err(WindowError::Class)?,
            hbrBackground: unsafe { CreateSolidBrush(BACKDROP_BRUSH) },
            lpszClassName: CLASS_NAME,
            ..WNDCLASSW::default()
        };
        if unsafe { RegisterClassW(&class) } == 0 {
            return Err(WindowError::Class(WindowsError::from_thread()));
        }

        let bitmap = Box::into_raw(Box::new(RefCell::new(Bitmap::new(width, height))));
        let created = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                CLASS_NAME,
                &HSTRING::from(title),
                STYLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width as i32,
                height as i32,
                None,
                None,
                Some(instance.into()),
                Some(bitmap.cast()),
            )
        };
        let hwnd = match created {
            Ok(hwnd) => hwnd,
            Err(error) => {
                drop(unsafe { Box::from_raw(bitmap) });
                return Err(WindowError::Create(error));
            }
        };

        let window = Window { hwnd, bitmap };
        window.fit_client_area(width as i32, height as i32);
        if visible {
            let _ = unsafe { ShowWindow(hwnd, SW_SHOW) };
        }
        Ok(window)
    }

    pub fn set_frame(&self, frame: &Frame) {
        unsafe { &*self.bitmap }.borrow_mut().replace_with(frame);
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
            let _ = UpdateWindow(self.hwnd);
        }
    }

    pub fn pump(&self) -> bool {
        let mut message = MSG::default();
        while unsafe { PeekMessageW(&mut message, None, 0, 0, PM_REMOVE) }.as_bool() {
            if message.message == WM_QUIT {
                return false;
            }
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        true
    }

    fn fit_client_area(&self, width: i32, height: i32) {
        let mut client = RECT::default();
        let mut window = RECT::default();
        if unsafe { GetClientRect(self.hwnd, &mut client) }.is_err()
            || unsafe { GetWindowRect(self.hwnd, &mut window) }.is_err()
        {
            return;
        }

        let outer_width = window.right - window.left + width - (client.right - client.left);
        let outer_height = window.bottom - window.top + height - (client.bottom - client.top);
        let _ = unsafe {
            SetWindowPos(
                self.hwnd,
                None,
                0,
                0,
                outer_width,
                outer_height,
                SWP_NOMOVE | SWP_NOZORDER,
            )
        };
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        let _ = unsafe { DestroyWindow(self.hwnd) };
        drop(unsafe { Box::from_raw(self.bitmap) });
    }
}

pub fn show_error(message: &str) {
    unsafe { MessageBoxW(None, &HSTRING::from(message), CAPTION, MB_OK | MB_ICONERROR) };
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CREATE => {
            let create = lparam.0 as *const CREATESTRUCTW;
            let bitmap = unsafe { (*create).lpCreateParams };
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, bitmap as isize) };
            tray::add(hwnd, CAPTION);
            LRESULT(0)
        }
        WM_SYSCOMMAND if (wparam.0 & 0xfff0) as u32 == SC_MINIMIZE => {
            let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
            LRESULT(0)
        }
        tray::CALLBACK => {
            tray::handle(hwnd, lparam);
            LRESULT(0)
        }
        message if message == tray::taskbar_created() => {
            tray::add(hwnd, CAPTION);
            LRESULT(0)
        }
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let device = unsafe { BeginPaint(hwnd, &mut paint) };
            let bitmap =
                unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const RefCell<Bitmap>;
            if let Some(bitmap) = unsafe { bitmap.as_ref() } {
                blit(device, &bitmap.borrow());
            }
            let _ = unsafe { EndPaint(hwnd, &paint) };
            LRESULT(0)
        }
        WM_DESTROY => {
            tray::remove(hwnd);
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

fn blit(device: HDC, bitmap: &Bitmap) {
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: bitmap.width,
            biHeight: -bitmap.height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..BITMAPINFOHEADER::default()
        },
        ..BITMAPINFO::default()
    };

    unsafe {
        StretchDIBits(
            device,
            0,
            0,
            bitmap.width,
            bitmap.height,
            0,
            0,
            bitmap.width,
            bitmap.height,
            Some(bitmap.pixels.as_ptr().cast()),
            &info,
            DIB_RGB_COLORS,
            SRCCOPY,
        )
    };
}

fn blend(source: u8, backdrop: u8, alpha: u32) -> u8 {
    ((source as u32 * alpha + backdrop as u32 * (255 - alpha)) / 255) as u8
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetPixel,
        ReleaseDC, SelectObject,
    };

    use super::{BACKDROP, Bitmap, blend, blit};
    use crate::app::panel::Frame;

    #[rstest]
    #[case::opaque(200, 255, 200)]
    #[case::transparent(200, 0, 20)]
    #[case::halfway(255, 128, 137)]
    fn composites_over_the_backdrop(#[case] source: u8, #[case] alpha: u32, #[case] expected: u8) {
        assert_eq!(blend(source, 20, alpha), expected);
    }

    #[test]
    fn starts_filled_with_the_backdrop() {
        let bitmap = Bitmap::new(2, 3);

        assert_eq!(bitmap.pixels.len(), 2 * 3 * 4);
        assert_eq!(
            bitmap.pixels[..4],
            [BACKDROP[2], BACKDROP[1], BACKDROP[0], 0]
        );
    }

    #[test]
    fn stores_frames_as_opaque_bgrx() {
        let mut bitmap = Bitmap::new(1, 1);
        bitmap.replace_with(&Frame {
            pixels: vec![10, 20, 30, 255, 0, 0, 0, 0],
            width: 2,
            height: 1,
        });

        assert_eq!(bitmap.width, 2);
        assert_eq!(bitmap.height, 1);
        assert_eq!(bitmap.pixels[..4], [30, 20, 10, 0]);
        assert_eq!(
            bitmap.pixels[4..],
            [BACKDROP[2], BACKDROP[1], BACKDROP[0], 0]
        );
    }

    #[test]
    fn blits_top_down_without_swapping_the_channels() {
        let mut bitmap = Bitmap::new(2, 2);
        bitmap.replace_with(&Frame {
            #[rustfmt::skip]
            pixels: vec![
                255, 0, 0, 255,   0, 255, 0, 255,
                0, 0, 255, 255,   255, 255, 255, 255,
            ],
            width: 2,
            height: 2,
        });

        let screen = unsafe { GetDC(None) };
        let device = unsafe { CreateCompatibleDC(Some(screen)) };
        let surface = unsafe { CreateCompatibleBitmap(screen, 2, 2) };
        let previous = unsafe { SelectObject(device, surface.into()) };
        blit(device, &bitmap);

        let sampled =
            [(0, 0), (1, 0), (0, 1), (1, 1)].map(|(x, y)| unsafe { GetPixel(device, x, y) }.0);

        unsafe {
            SelectObject(device, previous);
            let _ = DeleteObject(surface.into());
            let _ = DeleteDC(device);
            ReleaseDC(None, screen);
        }

        assert_eq!(
            sampled,
            [0x0000_00ff, 0x0000_ff00, 0x00ff_0000, 0x00ff_ffff]
        );
    }
}
