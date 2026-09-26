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
            // SM_CXDOUBLECLK is the width of the whole rectangle around the first click.
            let px = GetSystemMetrics(SM_CXDOUBLECLK).max(GetSystemMetrics(SM_CYDOUBLECLK)) / 2;
            (GetDoubleClickTime(), px.max(1) as u32)
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
}
