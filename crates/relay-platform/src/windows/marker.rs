//! A mark on the screen for a moment: a red outline around where an image
//! was found, and a dot where it would be clicked. A borderless, topmost,
//! click-through window that never takes focus, left out of screen captures
//! (before it's shown), so it can't disturb a search.

use std::cell::Cell;
use std::sync::Once;

use relay_core::model::Rect;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreatePen, CreateSolidBrush, DeleteObject, Ellipse, EndPaint, FillRect, GetStockObject, NULL_BRUSH,
    PAINTSTRUCT, PS_SOLID, Rectangle, SelectObject,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetMessageW, LWA_COLORKEY, MSG,
    PostQuitMessage, RegisterClassW, SW_SHOWNOACTIVATE, SetLayeredWindowAttributes, SetTimer, SetWindowDisplayAffinity,
    ShowWindow, TranslateMessage, WDA_EXCLUDEFROMCAPTURE, WM_DESTROY, WM_PAINT, WM_TIMER, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::w;

/// Drawn as transparent (a color the marks never use).
const KEY: COLORREF = COLORREF(0x00FF_00FF);
/// Relay's red, #EC3013, as 0x00BBGGRR.
const RED: COLORREF = COLORREF(0x0013_30EC);
const WHITE: COLORREF = COLORREF(0x00FF_FFFF);
/// Room around the outline, and the dot's radius.
const PAD: i32 = 6;
const DOT: i32 = 7;

thread_local! {
    /// What this thread's marker draws, relative to its window: the image's
    /// rectangle and the dot. Each marker has a thread of its own.
    static MARKS: Cell<(Rect, (i32, i32))> = const { Cell::new((Rect { x: 0, y: 0, w: 0, h: 0 }, (0, 0))) };
}

/// Shows the marks for `ms` on a thread of their own.
pub fn mark(area: Rect, dot: (i32, i32), ms: u32) {
    std::thread::spawn(move || unsafe { show(area, dot, ms) });
}

unsafe fn show(area: Rect, (dx, dy): (i32, i32), ms: u32) {
    unsafe {
        let Ok(module) = GetModuleHandleW(None) else { return };
        let instance = HINSTANCE(module.0);
        static REGISTER: Once = Once::new();
        REGISTER.call_once(|| {
            let class = WNDCLASSW {
                lpfnWndProc: Some(proc),
                hInstance: instance,
                hbrBackground: CreateSolidBrush(KEY),
                lpszClassName: w!("RelayMarker"),
                ..Default::default()
            };
            RegisterClassW(&class);
        });
        // The outline and the dot, which may be outside the image.
        let (x0, y0) = ((area.x - PAD).min(dx - DOT - 2), (area.y - PAD).min(dy - DOT - 2));
        let (x1, y1) = ((area.x + area.w + PAD).max(dx + DOT + 2), (area.y + area.h + PAD).max(dy + DOT + 2));
        let ex = WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
        let Ok(hwnd) = CreateWindowExW(
            ex,
            w!("RelayMarker"),
            w!(""),
            WS_POPUP,
            x0,
            y0,
            x1 - x0,
            y1 - y0,
            None,
            None,
            Some(instance),
            None,
        ) else {
            return;
        };
        MARKS.set((Rect { x: area.x - x0, y: area.y - y0, ..area }, (dx - x0, dy - y0)));
        let _ = SetLayeredWindowAttributes(hwnd, KEY, 0, LWA_COLORKEY);
        let _ = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        SetTimer(Some(hwnd), 1, ms, None);
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let dc = BeginPaint(hwnd, &mut ps);
                let mut client = Default::default();
                let _ = GetClientRect(hwnd, &mut client);
                let key = CreateSolidBrush(KEY);
                FillRect(dc, &client, key);
                let (a, (dx, dy)) = MARKS.get();
                // The outline, just outside the image.
                let pen = CreatePen(PS_SOLID, 3, RED);
                let old_pen = SelectObject(dc, pen.into());
                let old_brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
                let _ = Rectangle(dc, a.x - 3, a.y - 3, a.x + a.w + 3, a.y + a.h + 3);
                // The dot, ringed in white so it shows on red too.
                let white = CreatePen(PS_SOLID, 2, WHITE);
                SelectObject(dc, white.into());
                let red = CreateSolidBrush(RED);
                SelectObject(dc, red.into());
                let _ = Ellipse(dc, dx - DOT, dy - DOT, dx + DOT + 1, dy + DOT + 1);
                SelectObject(dc, old_pen);
                SelectObject(dc, old_brush);
                let _ = DeleteObject(pen.into());
                let _ = DeleteObject(white.into());
                let _ = DeleteObject(red.into());
                let _ = DeleteObject(key.into());
                let _ = EndPaint(hwnd, &ps);
                LRESULT(0)
            }
            WM_TIMER => {
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}
