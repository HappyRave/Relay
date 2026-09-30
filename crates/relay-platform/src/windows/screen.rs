use relay_core::model::{MonitorInfo, Rect, Rgb};
use windows::Win32::Foundation::{LPARAM, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetDC, GetMonitorInfoW, GetPixel, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW, ReleaseDC,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime;
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, MONITORINFOF_PRIMARY, SM_CXDOUBLECLK, SM_CXVIRTUALSCREEN, SM_CYDOUBLECLK,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};
use windows::core::BOOL;

use crate::Screen;
use crate::types::{Snapshot, snapshot_size};

pub struct WinScreen;

pub fn rect(r: RECT) -> Rect {
    Rect { x: r.left, y: r.top, w: r.right - r.left, h: r.bottom - r.top }
}

unsafe extern "system" fn collect(monitor: HMONITOR, _: HDC, _: *mut RECT, out: LPARAM) -> BOOL {
    let out = unsafe { &mut *(out.0 as *mut Vec<MonitorInfo>) };
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
    if unsafe { GetMonitorInfoW(monitor, &mut info as *mut _ as *mut MONITORINFO) }.as_bool() {
        let (mut dx, mut dy) = (96u32, 96u32);
        let _ = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dx, &mut dy) };
        let name_len = info.szDevice.iter().position(|&c| c == 0).unwrap_or(info.szDevice.len());
        out.push(MonitorInfo {
            name: String::from_utf16_lossy(&info.szDevice[..name_len]),
            rect: rect(info.monitorInfo.rcMonitor),
            work: rect(info.monitorInfo.rcWork),
            dpi: dx,
            primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
        });
    }
    true.into()
}

impl Screen for WinScreen {
    fn monitors(&self) -> Vec<MonitorInfo> {
        let mut out: Vec<MonitorInfo> = Vec::new();
        unsafe {
            let _ = EnumDisplayMonitors(None, None, Some(collect), LPARAM(&mut out as *mut _ as isize));
        }
        out
    }

    fn virtual_desktop(&self) -> Rect {
        unsafe {
            Rect {
                x: GetSystemMetrics(SM_XVIRTUALSCREEN),
                y: GetSystemMetrics(SM_YVIRTUALSCREEN),
                w: GetSystemMetrics(SM_CXVIRTUALSCREEN),
                h: GetSystemMetrics(SM_CYVIRTUALSCREEN),
            }
        }
    }

    fn cursor_pos(&self) -> (i32, i32) {
        let mut p = POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut p);
        }
        (p.x, p.y)
    }

    fn double_click(&self) -> (u32, u32) {
        unsafe {
            // SM_CXDOUBLECLK is the width of the whole rectangle around the first click;
            // step grouping takes half of it on each side.
            let px = GetSystemMetrics(SM_CXDOUBLECLK).max(GetSystemMetrics(SM_CYDOUBLECLK));
            (GetDoubleClickTime(), px.max(0) as u32)
        }
    }

    /// About 10 ms: reading the screen waits for the compositor, whichever
    /// GDI call does it (GetPixel, BitBlt), with or without a cached DC.
    fn pixel(&self, x: i32, y: i32) -> Option<Rgb> {
        unsafe {
            // The screen DC spans the virtual desktop, primary monitor at (0, 0).
            let dc = GetDC(None);
            if dc.is_invalid() {
                return None;
            }
            let c = GetPixel(dc, x, y).0;
            ReleaseDC(None, dc);
            // CLR_INVALID: off-screen, or a protected surface.
            (c != 0xFFFF_FFFF).then_some(Rgb((c & 0xFF) as u8, ((c >> 8) & 0xFF) as u8, ((c >> 16) & 0xFF) as u8))
        }
    }

    fn mark(&self, area: Rect, dot: (i32, i32), ms: u32) {
        super::marker::mark(area, dot, ms);
    }

    /// Leaves `exclude` out with `WDA_EXCLUDEFROMCAPTURE` (Windows 10 2004
    /// and later; on older versions the window shows in the picture) for the
    /// moment of the capture only, so other screenshot tools still see it.
    fn capture(&self, area: Rect, max_w: u32, exclude: isize) -> Option<Snapshot> {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE};
        // One at a time: another capture ending would show the window again
        // during this one.
        static CAPTURING: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _one = CAPTURING.lock().unwrap_or_else(|e| e.into_inner());
        let hwnd = HWND(exclude as *mut _);
        // The compositor applies the change on its next frame.
        let excluded = exclude != 0 && unsafe { SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE) }.is_ok();
        if excluded {
            unsafe {
                let _ = windows::Win32::Graphics::Dwm::DwmFlush();
                let _ = windows::Win32::Graphics::Dwm::DwmFlush();
            }
        }
        let snapshot = unsafe { stretch_capture(area, max_w) };
        if excluded {
            unsafe {
                let _ = SetWindowDisplayAffinity(hwnd, WDA_NONE);
            }
        }
        snapshot
    }
}

/// Copies `area` of the screen into a top-down 32-bit bitmap of the snapshot
/// size (halftone scaling, which averages pixels and keeps text readable).
unsafe fn stretch_capture(area: Rect, max_w: u32) -> Option<Snapshot> {
    use windows::Win32::Graphics::Gdi::{
        BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CAPTUREBLT, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS,
        DeleteDC, DeleteObject, HALFTONE, SRCCOPY, SelectObject, SetBrushOrgEx, SetStretchBltMode, StretchBlt,
    };
    let (w, h) = snapshot_size(area.w, area.h, max_w);
    unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            return None;
        }
        let mem = CreateCompatibleDC(Some(screen));
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w as i32,
                biHeight: -(h as i32), // negative: top-down rows
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let out = match CreateDIBSection(Some(mem), &info, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(bitmap) if !bits.is_null() => {
                let old = SelectObject(mem, bitmap.into());
                SetStretchBltMode(mem, HALFTONE);
                let _ = SetBrushOrgEx(mem, 0, 0, None);
                let ok = StretchBlt(
                    mem,
                    0,
                    0,
                    w as i32,
                    h as i32,
                    Some(screen),
                    area.x,
                    area.y,
                    area.w,
                    area.h,
                    SRCCOPY | CAPTUREBLT,
                )
                .as_bool();
                let pixels = std::slice::from_raw_parts(bits as *const u8, (w * h * 4) as usize);
                // BGRA to RGB.
                let rgb = ok.then(|| pixels.as_chunks::<4>().0.iter().flat_map(|p| [p[2], p[1], p[0]]).collect());
                SelectObject(mem, old);
                let _ = DeleteObject(bitmap.into());
                rgb.map(|rgb| Snapshot { w, h, rgb })
            }
            _ => None,
        };
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        out
    }
}
