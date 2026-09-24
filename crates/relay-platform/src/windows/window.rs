use std::ffi::c_void;
use std::sync::Once;
use std::sync::atomic::{AtomicIsize, Ordering};

use relay_core::model::WindowInfo;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, POINT, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentProcessId, OpenProcess, OpenProcessToken, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, EVENT_SYSTEM_FOREGROUND, EnumWindows, GA_ROOT, GW_HWNDNEXT, GWL_EXSTYLE, GetAncestor,
    GetClassNameW, GetForegroundWindow, GetMessageW, GetWindow, GetWindowLongPtrW, GetWindowTextW,
    GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, MSG, SetForegroundWindow, TranslateMessage,
    WINEVENT_OUTOFCONTEXT, WS_EX_TOOLWINDOW, WindowFromPoint,
};
use windows::core::{BOOL, PWSTR};

use super::screen::rect;
use crate::{WindowQuery, WindowRef};

pub struct WinWindows;

/// The last foreground window that belonged to another process.
static LAST_EXTERNAL: AtomicIsize = AtomicIsize::new(0);

unsafe extern "system" fn on_foreground(_: HWINEVENTHOOK, _: u32, hwnd: HWND, _: i32, _: i32, _: u32, _: u32) {
    if !hwnd.is_invalid() && pid_of(hwnd) != unsafe { GetCurrentProcessId() } {
        LAST_EXTERNAL.store(hwnd.0 as isize, Ordering::Relaxed);
    }
}

impl WinWindows {
    /// Starts tracking foreground changes (once per process).
    pub fn new() -> Self {
        static START: Once = Once::new();
        START.call_once(|| {
            if let Some(w) = WinWindows.foreground()
                && w.pid != unsafe { GetCurrentProcessId() }
            {
                LAST_EXTERNAL.store(w.hwnd, Ordering::Relaxed);
            }
            std::thread::Builder::new()
                .name("relay-foreground".into())
                .spawn(|| unsafe {
                    let _hook = SetWinEventHook(
                        EVENT_SYSTEM_FOREGROUND,
                        EVENT_SYSTEM_FOREGROUND,
                        None,
                        Some(on_foreground),
                        0,
                        0,
                        WINEVENT_OUTOFCONTEXT,
                    );
                    let mut msg = MSG::default();
                    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                })
                .expect("spawn foreground tracker");
        });
        WinWindows
    }
}

fn pid_of(hwnd: HWND) -> u32 {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    pid
}

fn class_of(hwnd: HWND) -> String {
    let mut class = [0u16; 256];
    let n = unsafe { GetClassNameW(hwnd, &mut class) } as usize;
    String::from_utf16_lossy(&class[..n])
}

/// The visible frame; GetWindowRect includes invisible resize borders.
fn frame_of(hwnd: HWND) -> Option<RECT> {
    let mut r = RECT::default();
    unsafe { DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, &mut r as *mut _ as *mut c_void, size_of::<RECT>() as u32) }
        .ok()?;
    Some(r)
}

fn info_of(hwnd: HWND) -> Option<WindowInfo> {
    let mut title = [0u16; 512];
    let t = unsafe { GetWindowTextW(hwnd, &mut title) } as usize;
    Some(WindowInfo {
        exe: exe_name(pid_of(hwnd)).unwrap_or_default(),
        class: class_of(hwnd),
        title: String::from_utf16_lossy(&title[..t]),
        rect: rect(frame_of(hwnd)?),
    })
}

/// A window the user could have been working in: visible, not minimized,
/// not a tool window, not cloaked (e.g. on another virtual desktop).
fn is_app_window(hwnd: HWND) -> bool {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            return false;
        }
        if GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0 != 0 {
            return false;
        }
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut _ as *mut c_void, 4);
        cloaked == 0 && frame_of(hwnd).is_some_and(|r| r.right > r.left && r.bottom > r.top)
    }
}

impl WindowQuery for WinWindows {
    fn root_window_at(&self, x: i32, y: i32) -> Option<WindowInfo> {
        unsafe {
            let hwnd = WindowFromPoint(POINT { x, y });
            if hwnd.is_invalid() {
                return None;
            }
            let root = GetAncestor(hwnd, GA_ROOT);
            info_of(if root.is_invalid() { hwnd } else { root })
        }
    }

    fn foreground(&self) -> Option<WindowRef> {
        let hwnd = unsafe { GetForegroundWindow() };
        (!hwnd.is_invalid()).then(|| WindowRef { hwnd: hwnd.0 as isize, pid: pid_of(hwnd) })
    }

    fn restore_previous(&self, own: isize) -> Option<WindowRef> {
        let me = unsafe { GetCurrentProcessId() };
        let last = HWND(LAST_EXTERNAL.load(Ordering::Relaxed) as _);
        if !last.is_invalid() && unsafe { IsWindow(Some(last)) }.as_bool() && is_app_window(last) {
            unsafe {
                let _ = SetForegroundWindow(last);
            }
            return Some(WindowRef { hwnd: last.0 as isize, pid: pid_of(last) });
        }
        let mut h = unsafe { GetWindow(HWND(own as _), GW_HWNDNEXT) }.ok()?;
        loop {
            if is_app_window(h) && pid_of(h) != me {
                unsafe {
                    let _ = SetForegroundWindow(h);
                }
                return Some(WindowRef { hwnd: h.0 as isize, pid: pid_of(h) });
            }
            h = unsafe { GetWindow(h, GW_HWNDNEXT) }.ok()?;
        }
    }

    fn find_window(&self, exe: &str, class: &str) -> Option<WindowInfo> {
        struct Search<'a> {
            exe: &'a str,
            class: &'a str,
            found: Option<HWND>,
        }
        unsafe extern "system" fn visit(hwnd: HWND, lp: LPARAM) -> BOOL {
            let s = unsafe { &mut *(lp.0 as *mut Search) };
            if is_app_window(hwnd)
                && class_of(hwnd) == s.class
                && exe_name(pid_of(hwnd)).is_some_and(|e| e.eq_ignore_ascii_case(s.exe))
            {
                s.found = Some(hwnd);
                return false.into(); // EnumWindows goes top to bottom: the first match is topmost
            }
            true.into()
        }
        let mut s = Search { exe, class, found: None };
        unsafe {
            let _ = EnumWindows(Some(visit), LPARAM(&mut s as *mut _ as isize));
        }
        info_of(s.found?)
    }

    fn is_elevated(&self, pid: u32) -> bool {
        match unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) } {
            Ok(h) => {
                let e = token_elevated(h);
                unsafe {
                    let _ = CloseHandle(h);
                }
                e
            }
            Err(_) => false,
        }
    }

    fn self_elevated(&self) -> bool {
        token_elevated(unsafe { GetCurrentProcess() })
    }
}

fn token_elevated(process: HANDLE) -> bool {
    unsafe {
        let mut token = HANDLE::default();
        // A non-elevated process can't open an elevated process's token.
        if OpenProcessToken(process, TOKEN_QUERY, &mut token).is_err() {
            return true;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut len = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut c_void),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut len,
        )
        .is_ok();
        let _ = CloseHandle(token);
        ok && elevation.TokenIsElevated != 0
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
