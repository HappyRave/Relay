use relay_core::model::WindowInfo;
use windows::Win32::Foundation::{CloseHandle, POINT, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GA_ROOT, GetAncestor, GetClassNameW, GetWindowTextW, GetWindowThreadProcessId, WindowFromPoint,
};
use windows::core::PWSTR;

use super::screen::rect;
use crate::WindowQuery;

pub struct WinWindows;

impl WindowQuery for WinWindows {
    fn root_window_at(&self, x: i32, y: i32) -> Option<WindowInfo> {
        unsafe {
            let hwnd = WindowFromPoint(POINT { x, y });
            if hwnd.is_invalid() {
                return None;
            }
            let root = GetAncestor(hwnd, GA_ROOT);
            let root = if root.is_invalid() { hwnd } else { root };

            let mut class = [0u16; 256];
            let n = GetClassNameW(root, &mut class) as usize;
            let mut title = [0u16; 512];
            let t = GetWindowTextW(root, &mut title) as usize;

            // The visible frame; GetWindowRect includes invisible resize borders.
            let mut r = RECT::default();
            DwmGetWindowAttribute(root, DWMWA_EXTENDED_FRAME_BOUNDS, &mut r as *mut _ as *mut _, size_of::<RECT>() as u32)
                .ok()?;

            let mut pid = 0u32;
            GetWindowThreadProcessId(root, Some(&mut pid));
            Some(WindowInfo {
                exe: exe_name(pid).unwrap_or_default(),
                class: String::from_utf16_lossy(&class[..n]),
                title: String::from_utf16_lossy(&title[..t]),
                rect: rect(r),
            })
        }
    }
}

/// The executable file name ("EXCEL.EXE") of a process.
fn exe_name(pid: u32) -> Option<String> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
        let _ = CloseHandle(h);
        ok.then(|| {
            let path = String::from_utf16_lossy(&buf[..len as usize]);
            path.rsplit(['\\', '/']).next().unwrap_or(&path).to_string()
        })
    }
}
