//! The picker on Windows: three windows that never take the focus, so a menu
//! or a tooltip open in the app under them stays open.
//! - The catcher covers the virtual desktop and takes the mouse. Live, it's
//!   all but invisible (alpha 1); frozen, it shows the screen as it was, with
//!   corner marks on each monitor.
//! - The loupe follows the cursor: the 11 × 11 pixels around it, magnified,
//!   and a tag with the coordinates and the color. It lets clicks through.
//! - The help bar says what to do, LIVE or FROZEN and the time, with a
//!   Freeze key.
//!
//! All three are layered windows, which a screen copy without `CAPTUREBLT`
//! leaves out: the loupe and the frozen screen never show the picker itself.
//! A low-level keyboard hook takes the arrows, Space, Enter and Esc.

use std::cell::{Cell, RefCell};
use std::sync::Once;

use relay_core::model::{Rect, Rgb};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, COLORONCOLOR, CreateCompatibleBitmap,
    CreateCompatibleDC, CreateEllipticRgn, CreateFontW, CreatePen, CreateSolidBrush, DEFAULT_CHARSET, DT_CENTER,
    DT_LEFT, DT_SINGLELINE, DT_VCENTER, DeleteDC, DeleteObject, DrawTextW, Ellipse, EndPaint, FW_BOLD, FW_SEMIBOLD,
    FillRect, GetDC, GetMonitorInfoW, GetPixel, GetStockObject, GetTextExtentPoint32W, HBITMAP, HDC, HFONT, HGDIOBJ,
    InvalidateRect, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, NULL_BRUSH, OUT_DEFAULT_PRECIS,
    PAINTSTRUCT, PS_SOLID, ReleaseDC, RoundRect, SRCCOPY, SelectClipRgn, SelectObject, SetBkMode, SetStretchBltMode,
    SetTextColor, StretchBlt, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_SHIFT};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetCursorPos,
    GetMessageW, GetSystemMetrics, HHOOK, HWND_TOPMOST, IDC_CROSS, KBDLLHOOKSTRUCT, LWA_ALPHA, LWA_COLORKEY,
    LoadCursorW, MA_NOACTIVATE, MSG, PostMessageW, PostQuitMessage, RegisterClassW, SM_CXVIRTUALSCREEN,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SWP_NOACTIVATE, SWP_SHOWWINDOW, SetCursor, SetCursorPos,
    SetLayeredWindowAttributes, SetTimer, SetWindowPos, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
    WH_KEYBOARD_LL, WM_APP, WM_DESTROY, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_PAINT,
    WM_RBUTTONDOWN, WM_SETCURSOR, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::{PCWSTR, w};

use crate::picker::{LOUPE, PickKey, Picked, SPAN, TAG_H, help_rect, hex, loupe_rect, pick_key};

const KEY: COLORREF = rgb(253, 1, 254);
const INK: COLORREF = rgb(0x14, 0x14, 0x15);
const PAPER: COLORREF = rgb(0xF4, 0xF3, 0xEF);
const LINE: COLORREF = rgb(0xC9, 0xC8, 0xC3);
const LCD: COLORREF = rgb(0x10, 0x10, 0x11);
const ALU: COLORREF = rgb(0xCF, 0xCD, 0xC6);
const RED: COLORREF = rgb(0xE0, 0x30, 0x0F);
const WHITE: COLORREF = rgb(255, 255, 255);

const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF(r as u32 | (g as u32) << 8 | (b as u32) << 16)
}

const MOVE: u32 = WM_APP + 1;
const FREEZE: u32 = WM_APP + 2;
const PICK: u32 = WM_APP + 3;
const CANCEL: u32 = WM_APP + 4;
const HELP_TEXT: &str = "Click the pixel, or move with the arrow keys and press Enter. Esc cancels.";

/// A memory DC with its bitmap selected, and what was selected before.
type Mem = (HDC, HBITMAP, HGDIOBJ);

#[derive(Clone, Copy)]
struct State {
    catcher: HWND,
    loupe: HWND,
    help: HWND,
    /// The magnified pixels, in a memory DC.
    sample: Mem,
    /// The frozen screen and when it was taken (h, m, s).
    frozen: Option<(Mem, (u16, u16, u16))>,
    desktop: Rect,
    at: (i32, i32),
    color: Rgb,
    scale: f64,
    fonts: (HFONT, HFONT),
    /// The help bar's size, and where its Freeze key is in it.
    help_size: (i32, i32),
    freeze_key: RECT,
    hook: HHOOK,
    done: Option<Option<Picked>>,
}

thread_local! {
    static STATE: Cell<Option<State>> = const { Cell::new(None) };
    /// The monitors the frozen screen's corner marks frame.
    static MONITORS: RefCell<Vec<Rect>> = const { RefCell::new(Vec::new()) };
}

fn state() -> Option<State> {
    STATE.get()
}

fn set(s: State) {
    STATE.set(Some(s));
}

/// Lets the user pick a pixel; blocks until a pick or a cancel (`None`).
pub fn pick() -> Option<Picked> {
    std::thread::spawn(|| unsafe { run() }).join().ok().flatten()
}

unsafe fn class(name: PCWSTR, instance: HINSTANCE, cursor: bool) {
    unsafe {
        let class = WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: instance,
            hCursor: if cursor { LoadCursorW(None, IDC_CROSS).unwrap_or_default() } else { Default::default() },
            lpszClassName: name,
            ..Default::default()
        };
        RegisterClassW(&class);
    }
}

unsafe fn run() -> Option<Picked> {
    unsafe {
        let instance = HINSTANCE(GetModuleHandleW(None).ok()?.0);
        static REGISTER: Once = Once::new();
        REGISTER.call_once(|| {
            class(w!("RelayPickerCatcher"), instance, true);
            class(w!("RelayPickerPart"), instance, false);
        });
        let desktop = Rect {
            x: GetSystemMetrics(SM_XVIRTUALSCREEN),
            y: GetSystemMetrics(SM_YVIRTUALSCREEN),
            w: GetSystemMetrics(SM_CXVIRTUALSCREEN),
            h: GetSystemMetrics(SM_CYVIRTUALSCREEN),
        };
        let ex = WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
        let window = |class: PCWSTR, ex, r: Rect| {
            CreateWindowExW(ex, class, w!(""), WS_POPUP, r.x, r.y, r.w, r.h, None, None, Some(instance), None).ok()
        };
        let catcher = window(w!("RelayPickerCatcher"), ex, desktop)?;
        let small = Rect { x: desktop.x, y: desktop.y, w: 10, h: 10 };
        let loupe = window(w!("RelayPickerPart"), ex | WS_EX_TRANSPARENT, small)?;
        let help = window(w!("RelayPickerPart"), ex, small)?;
        let _ = SetLayeredWindowAttributes(catcher, COLORREF(0), 1, LWA_ALPHA);
        let _ = SetLayeredWindowAttributes(loupe, KEY, 255, LWA_COLORKEY);
        let _ = SetLayeredWindowAttributes(help, KEY, 255, LWA_COLORKEY);

        let screen = GetDC(None);
        let dc = CreateCompatibleDC(Some(screen));
        let bmp = CreateCompatibleBitmap(screen, SPAN, SPAN);
        let old = SelectObject(dc, bmp.into());
        ReleaseDC(None, screen);
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keys), Some(instance), 0).ok()?;
        set(State {
            catcher,
            loupe,
            help,
            sample: (dc, bmp, old),
            frozen: None,
            desktop,
            at: (0, 0),
            color: Rgb(0, 0, 0),
            scale: 0.0,
            fonts: (HFONT::default(), HFONT::default()),
            help_size: (0, 0),
            freeze_key: RECT::default(),
            hook,
            done: None,
        });
        let _ = SetWindowPos(
            catcher,
            Some(HWND_TOPMOST),
            desktop.x,
            desktop.y,
            desktop.w,
            desktop.h,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        update();
        // The live screen changes under the loupe even when the mouse doesn't move.
        SetTimer(Some(catcher), 1, 40, None);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let s = state()?;
        let _ = UnhookWindowsHookEx(s.hook);
        unfreeze(s);
        let (dc, bmp, old) = s.sample;
        SelectObject(dc, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(dc);
        let _ = DeleteObject(s.fonts.0.into());
        let _ = DeleteObject(s.fonts.1.into());
        STATE.set(None);
        s.done.flatten()
    }
}

/// The monitor's bounds and work area at `p`, and its scale.
unsafe fn monitor_at(p: (i32, i32)) -> (Rect, Rect, f64) {
    unsafe {
        let m = MonitorFromPoint(POINT { x: p.0, y: p.1 }, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(m, &mut info);
        let (mut dx, mut dy) = (96u32, 96u32);
        let _ = GetDpiForMonitor(m, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
        (super::screen::rect(info.rcMonitor), super::screen::rect(info.rcWork), dx as f64 / 96.0)
    }
}

unsafe fn font(px: i32, weight: i32) -> HFONT {
    unsafe {
        CreateFontW(
            -px,
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            0,
            w!("Segoe UI"),
        )
    }
}

/// Fonts and the help bar's layout for a monitor at `scale`.
unsafe fn rescale(s: &mut State, scale: f64) {
    unsafe {
        let _ = DeleteObject(s.fonts.0.into());
        let _ = DeleteObject(s.fonts.1.into());
        let px = |v: f64| (v * scale).round() as i32;
        s.fonts = (font(px(13.0), FW_SEMIBOLD.0 as i32), font(px(12.0), FW_BOLD.0 as i32));
        s.scale = scale;
        let dc = GetDC(Some(s.help));
        let old = SelectObject(dc, s.fonts.0.into());
        let width = |text: &str| {
            let t: Vec<u16> = text.encode_utf16().collect();
            let mut size = Default::default();
            let _ = GetTextExtentPoint32W(dc, &t, &mut size);
            size.cx
        };
        let text = width(HELP_TEXT);
        let freeze = width("Freeze") + width("Space") + px(30.0);
        SelectObject(dc, old);
        ReleaseDC(Some(s.help), dc);
        let (pad, gap, lcd) = (px(14.0), px(10.0), px(118.0));
        let w = pad + text + gap + lcd + gap + freeze + px(8.0);
        let h = px(44.0);
        s.help_size = (w, h);
        let left = w - px(8.0) - freeze;
        s.freeze_key = RECT { left, top: px(7.0), right: left + freeze, bottom: h - px(7.0) };
    }
}

/// Reads the pixels around the cursor and puts the loupe and the help bar where they go.
unsafe fn update() {
    unsafe {
        let Some(mut s) = state() else { return };
        let mut p = POINT::default();
        let _ = GetCursorPos(&mut p);
        let at = (p.x, p.y);
        let (monitor, work, scale) = monitor_at(at);
        if scale != s.scale {
            rescale(&mut s, scale);
        }
        let half = SPAN / 2;
        match s.frozen {
            Some(((dc, ..), _)) => {
                let (x, y) = (at.0 - half - s.desktop.x, at.1 - half - s.desktop.y);
                let _ = BitBlt(s.sample.0, 0, 0, SPAN, SPAN, Some(dc), x, y, SRCCOPY);
            }
            None => {
                let screen = GetDC(None);
                let _ = BitBlt(s.sample.0, 0, 0, SPAN, SPAN, Some(screen), at.0 - half, at.1 - half, SRCCOPY);
                ReleaseDC(None, screen);
            }
        }
        let c = GetPixel(s.sample.0, half, half).0;
        s.color = Rgb((c & 0xFF) as u8, ((c >> 8) & 0xFF) as u8, ((c >> 16) & 0xFF) as u8);
        s.at = at;
        set(s);
        let l = loupe_rect(at, monitor, scale);
        let h = help_rect(work, s.help_size, at, scale);
        let flags = SWP_NOACTIVATE | SWP_SHOWWINDOW;
        let _ = SetWindowPos(s.loupe, Some(HWND_TOPMOST), l.x, l.y, l.w, l.h, flags);
        let _ = SetWindowPos(s.help, Some(HWND_TOPMOST), h.x, h.y, h.w, h.h, flags);
        let _ = InvalidateRect(Some(s.loupe), None, false);
        let _ = InvalidateRect(Some(s.help), None, false);
    }
}

unsafe fn unfreeze(s: State) {
    if let Some(((dc, bmp, old), _)) = s.frozen {
        unsafe {
            SelectObject(dc, old);
            let _ = DeleteObject(bmp.into());
            let _ = DeleteDC(dc);
        }
    }
}

/// Space: holds the screen as it is, or goes back to the live screen.
unsafe fn toggle_freeze() {
    unsafe {
        let Some(mut s) = state() else { return };
        if s.frozen.is_some() {
            unfreeze(s);
            s.frozen = None;
            let _ = SetLayeredWindowAttributes(s.catcher, COLORREF(0), 1, LWA_ALPHA);
        } else {
            let d = s.desktop;
            let screen = GetDC(None);
            let dc = CreateCompatibleDC(Some(screen));
            let bmp = CreateCompatibleBitmap(screen, d.w, d.h);
            let old = SelectObject(dc, bmp.into());
            // Without CAPTUREBLT: layered windows (the picker's) are left out.
            let _ = BitBlt(dc, 0, 0, d.w, d.h, Some(screen), d.x, d.y, SRCCOPY);
            ReleaseDC(None, screen);
            let t = GetLocalTime();
            s.frozen = Some(((dc, bmp, old), (t.wHour, t.wMinute, t.wSecond)));
            MONITORS.set(crate::Screen::monitors(&super::screen::WinScreen).into_iter().map(|m| m.rect).collect());
            let _ = SetLayeredWindowAttributes(s.catcher, COLORREF(0), 255, LWA_ALPHA);
        }
        set(s);
        let _ = InvalidateRect(Some(s.catcher), None, false);
        update();
    }
}

unsafe fn finish(picked: bool) {
    unsafe {
        update();
        let Some(mut s) = state() else { return };
        s.done = Some(picked.then_some(Picked { x: s.at.0, y: s.at.1, color: s.color, frozen: s.frozen.is_some() }));
        set(s);
        let _ = DestroyWindow(s.loupe);
        let _ = DestroyWindow(s.help);
        let _ = DestroyWindow(s.catcher);
    }
}

unsafe extern "system" fn keys(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        if code >= 0
            && let Some(s) = state()
        {
            let info = &*(lp.0 as *const KBDLLHOOKSTRUCT);
            let shift = GetAsyncKeyState(VK_SHIFT.0 as i32) < 0;
            if let Some(key) = pick_key(info.vkCode as u16, shift) {
                let down = matches!(wp.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
                if down {
                    let (msg, w, l) = match key {
                        PickKey::Move(dx, dy) => (MOVE, dx as isize, dy as isize),
                        PickKey::Freeze => (FREEZE, 0, 0),
                        PickKey::Pick => (PICK, 0, 0),
                        PickKey::Cancel => (CANCEL, 0, 0),
                    };
                    let _ = PostMessageW(Some(s.catcher), msg, WPARAM(w as usize), LPARAM(l));
                }
                // The release goes with the press: neither reaches the app.
                if down || matches!(wp.0 as u32, WM_KEYUP | WM_SYSKEYUP) {
                    return LRESULT(1);
                }
            }
        }
        CallNextHookEx(None, code, wp, lp)
    }
}

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        let Some(s) = state() else { return DefWindowProcW(hwnd, msg, wp, lp) };
        match msg {
            WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
            WM_SETCURSOR if hwnd == s.catcher => {
                SetCursor(LoadCursorW(None, IDC_CROSS).ok());
                LRESULT(1)
            }
            // Moving the loupe makes Windows send a mouse move of its own: only a real one counts.
            WM_MOUSEMOVE if hwnd == s.catcher => {
                let mut p = POINT::default();
                let _ = GetCursorPos(&mut p);
                if (p.x, p.y) != s.at {
                    update();
                }
                LRESULT(0)
            }
            WM_TIMER if s.frozen.is_none() => {
                update();
                LRESULT(0)
            }
            WM_LBUTTONDOWN if hwnd == s.catcher => {
                finish(true);
                LRESULT(0)
            }
            WM_LBUTTONDOWN if hwnd == s.help => {
                let (x, y) = ((lp.0 & 0xFFFF) as i16 as i32, ((lp.0 >> 16) & 0xFFFF) as i16 as i32);
                let k = s.freeze_key;
                if x >= k.left && x < k.right && y >= k.top && y < k.bottom {
                    toggle_freeze();
                }
                LRESULT(0)
            }
            WM_RBUTTONDOWN => {
                finish(false);
                LRESULT(0)
            }
            MOVE => {
                let (dx, dy) = (wp.0 as isize as i32, lp.0 as i32);
                let d = s.desktop;
                let x = (s.at.0 + dx).clamp(d.x, d.x + d.w - 1);
                let y = (s.at.1 + dy).clamp(d.y, d.y + d.h - 1);
                let _ = SetCursorPos(x, y);
                update();
                LRESULT(0)
            }
            FREEZE => {
                toggle_freeze();
                LRESULT(0)
            }
            PICK => {
                finish(true);
                LRESULT(0)
            }
            CANCEL => {
                finish(false);
                LRESULT(0)
            }
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let dc = BeginPaint(hwnd, &mut ps);
                if hwnd == s.catcher {
                    paint_catcher(dc, &s);
                } else if hwnd == s.loupe {
                    paint_loupe(dc, &s);
                } else {
                    paint_help(dc, &s);
                }
                let _ = EndPaint(hwnd, &ps);
                LRESULT(0)
            }
            WM_DESTROY if hwnd == s.catcher => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

unsafe fn fill(dc: HDC, r: RECT, color: COLORREF) {
    unsafe {
        let brush = CreateSolidBrush(color);
        FillRect(dc, &r, brush);
        let _ = DeleteObject(brush.into());
    }
}

/// A rounded box, filled with `bg` and outlined with `edge`.
unsafe fn round_box(dc: HDC, r: RECT, radius: i32, bg: COLORREF, edge: COLORREF) {
    unsafe {
        let brush = CreateSolidBrush(bg);
        let pen = CreatePen(PS_SOLID, 1, edge);
        let (ob, op) = (SelectObject(dc, brush.into()), SelectObject(dc, pen.into()));
        let _ = RoundRect(dc, r.left, r.top, r.right, r.bottom, radius, radius);
        SelectObject(dc, ob);
        SelectObject(dc, op);
        let _ = DeleteObject(brush.into());
        let _ = DeleteObject(pen.into());
    }
}

unsafe fn text(dc: HDC, s: &str, mut r: RECT, color: COLORREF, center: bool) {
    unsafe {
        let mut t: Vec<u16> = s.encode_utf16().collect();
        SetTextColor(dc, color);
        let align = if center { DT_CENTER } else { DT_LEFT };
        DrawTextW(dc, &mut t, &mut r, align | DT_VCENTER | DT_SINGLELINE);
    }
}

unsafe fn paint_catcher(dc: HDC, s: &State) {
    unsafe {
        let d = s.desktop;
        let Some(((frozen, ..), _)) = s.frozen else {
            fill(dc, RECT { left: 0, top: 0, right: d.w, bottom: d.h }, INK);
            return;
        };
        let _ = BitBlt(dc, 0, 0, d.w, d.h, Some(frozen), 0, 0, SRCCOPY);
        // Drafting corner marks on each monitor, like a print of it: ink, with a white edge.
        let k = (28.0 * s.scale) as i32;
        let (t, inset) = ((3.0 * s.scale).max(2.0) as i32, (8.0 * s.scale) as i32);
        MONITORS.with_borrow(|monitors| {
            for m in monitors {
                let (l, r) = (m.x - d.x + inset, m.x - d.x + m.w - inset);
                let (top, b) = (m.y - d.y + inset, m.y - d.y + m.h - inset);
                for (x, y, sx, sy) in [(l, top, 1, 1), (r, top, -1, 1), (r, b, -1, -1), (l, b, 1, -1)] {
                    let bar = |x0: i32, y0: i32, x1: i32, y1: i32| RECT {
                        left: x0.min(x1),
                        top: y0.min(y1),
                        right: x0.max(x1),
                        bottom: y0.max(y1),
                    };
                    for (grow, color) in [(1, WHITE), (0, INK)] {
                        let h = bar(x - sx * grow, y - sy * grow, x + sx * (k + grow), y + sy * (t + grow));
                        let v = bar(x - sx * grow, y - sy * grow, x + sx * (t + grow), y + sy * (k + grow));
                        fill(dc, h, color);
                        fill(dc, v, color);
                    }
                }
            }
        });
    }
}

unsafe fn paint_loupe(dc: HDC, s: &State) {
    unsafe {
        let mut client = RECT::default();
        let _ = GetClientRect(s.loupe, &mut client);
        fill(dc, client, KEY);
        let d = (LOUPE as f64 * s.scale).round() as i32;
        // The pixels, square and sharp, in a circle.
        let clip = CreateEllipticRgn(0, 0, d + 1, d + 1);
        SelectClipRgn(dc, Some(clip));
        SetStretchBltMode(dc, COLORONCOLOR);
        let _ = StretchBlt(dc, 0, 0, d, d, Some(s.sample.0), 0, 0, SPAN, SPAN, SRCCOPY);
        SelectClipRgn(dc, None);
        let _ = DeleteObject(clip.into());
        let old_brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
        let ring = |width: f64, color: COLORREF, inset: i32| {
            let pen = CreatePen(PS_SOLID, (width * s.scale).max(1.0) as i32, color);
            let old = SelectObject(dc, pen.into());
            let _ = Ellipse(dc, inset, inset, d - inset, d - inset);
            SelectObject(dc, old);
            let _ = DeleteObject(pen.into());
        };
        ring(4.0, WHITE, 1);
        ring(2.0, INK, 1);
        // The picked pixel, outlined.
        let cell = d as f64 / SPAN as f64;
        let (c0, c1) = ((cell * 5.0).round() as i32, (cell * 6.0).round() as i32);
        for (grow, color) in [(2, INK), (1, WHITE)] {
            let pen = CreatePen(PS_SOLID, 1, color);
            let old = SelectObject(dc, pen.into());
            let _ = windows::Win32::Graphics::Gdi::Rectangle(dc, c0 - grow, c0 - grow, c1 + grow, c1 + grow);
            SelectObject(dc, old);
            let _ = DeleteObject(pen.into());
        }
        SelectObject(dc, old_brush);
        // The tag: where, and which color.
        let top = d + (6.0 * s.scale) as i32;
        let tag = RECT { left: 0, top, right: d, bottom: top + (TAG_H as f64 * s.scale) as i32 };
        round_box(dc, tag, (8.0 * s.scale) as i32, LCD, LCD);
        SetBkMode(dc, TRANSPARENT);
        let old_font = SelectObject(dc, s.fonts.1.into());
        text(dc, &format!("{}, {} · {}", s.at.0, s.at.1, hex(s.color)), tag, PAPER, true);
        SelectObject(dc, old_font);
    }
}

unsafe fn paint_help(dc: HDC, s: &State) {
    unsafe {
        let px = |v: f64| (v * s.scale).round() as i32;
        let (w, h) = s.help_size;
        fill(dc, RECT { left: 0, top: 0, right: w, bottom: h }, KEY);
        round_box(dc, RECT { left: 0, top: 0, right: w, bottom: h }, px(16.0), PAPER, LINE);
        SetBkMode(dc, TRANSPARENT);
        let old_font = SelectObject(dc, s.fonts.0.into());
        let k = s.freeze_key;
        let lcd = RECT { left: k.left - px(10.0) - px(118.0), top: k.top, right: k.left - px(10.0), bottom: k.bottom };
        text(dc, HELP_TEXT, RECT { left: px(14.0), top: 0, right: lcd.left - px(10.0), bottom: h }, INK, false);
        // LIVE with a red dot, or FROZEN and the time.
        round_box(dc, lcd, px(8.0), LCD, LCD);
        SelectObject(dc, s.fonts.1.into());
        match s.frozen {
            Some((_, (hh, mm, ss))) => {
                let label = RECT { left: lcd.left + px(10.0), ..lcd };
                text(dc, "FROZEN", label, PAPER, false);
                let time = RECT { right: lcd.right - px(10.0), ..lcd };
                let mut t: Vec<u16> = format!("{hh:02}:{mm:02}:{ss:02}").encode_utf16().collect();
                SetTextColor(dc, ALU);
                let mut r = time;
                DrawTextW(dc, &mut t, &mut r, windows::Win32::Graphics::Gdi::DT_RIGHT | DT_VCENTER | DT_SINGLELINE);
            }
            None => {
                let cy = (lcd.top + lcd.bottom) / 2;
                let (dl, r) = (lcd.left + px(12.0), px(3.5));
                let brush = CreateSolidBrush(RED);
                let pen = CreatePen(PS_SOLID, 1, RED);
                let (ob, op) = (SelectObject(dc, brush.into()), SelectObject(dc, pen.into()));
                let _ = Ellipse(dc, dl, cy - r, dl + 2 * r, cy + r);
                SelectObject(dc, ob);
                SelectObject(dc, op);
                let _ = DeleteObject(brush.into());
                let _ = DeleteObject(pen.into());
                text(dc, "LIVE", RECT { left: dl + 2 * r + px(6.0), ..lcd }, PAPER, false);
            }
        }
        // The Freeze key, pressed (ink) while frozen.
        let (bg, fg) = if s.frozen.is_some() { (INK, PAPER) } else { (WHITE, INK) };
        round_box(dc, k, px(10.0), bg, if s.frozen.is_some() { INK } else { LINE });
        SelectObject(dc, s.fonts.0.into());
        text(dc, "Freeze", RECT { left: k.left + px(10.0), ..k }, fg, false);
        let mut t: Vec<u16> = "Space".encode_utf16().collect();
        SetTextColor(dc, if s.frozen.is_some() { ALU } else { rgb(0x6B, 0x6A, 0x66) });
        let mut r = RECT { right: k.right - px(10.0), ..k };
        DrawTextW(dc, &mut t, &mut r, windows::Win32::Graphics::Gdi::DT_RIGHT | DT_VCENTER | DT_SINGLELINE);
        SelectObject(dc, old_font);
    }
}
