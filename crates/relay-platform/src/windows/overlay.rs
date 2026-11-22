//! *Show targets* on Windows (the look is [`crate::overlay`]).
//! - Live marks: each is a small layered window of its own, topmost,
//!   click-through, never focused and left out of screen captures, drawn with
//!   Direct2D into a bitmap. One thread animates them all and closes each
//!   when it has faded.
//! - The still overlay: one layered window over the virtual desktop, drawn
//!   once, on the caller's thread until a click or Esc (a low-level keyboard
//!   hook, so the app under it keeps the focus).

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Once, OnceLock};
use std::time::Instant;

use crossbeam_channel::{Receiver, Sender, unbounded};
use relay_core::marks::{Mark, MarkKind, route, spots};
use relay_core::model::{MouseBtn, Rect};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_FIGURE_BEGIN_HOLLOW, D2D1_FIGURE_END_OPEN,
    D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_CAP_STYLE_ROUND, D2D1_DASH_STYLE_CUSTOM, D2D1_DASH_STYLE_SOLID, D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_ELLIPSE,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT, D2D1_LINE_JOIN_ROUND, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE, D2D1_ROUNDED_RECT, D2D1_STROKE_STYLE_PROPERTIES,
    D2D1CreateFactory, ID2D1DCRenderTarget, ID2D1Factory, ID2D1SolidColorBrush, ID2D1StrokeStyle,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL,
    DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
    DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_METRICS, DWRITE_WORD_WRAPPING_NO_WRAP,
    DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC,
    CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetMonitorInfoW, HBITMAP, HDC, HGDIOBJ,
    MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, SelectObject,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW,
    GetSystemMetrics, IDC_ARROW, KBDLLHOOKSTRUCT, KillTimer, LoadCursorW, MA_NOACTIVATE, MSG, PM_NOREMOVE,
    PeekMessageW, PostQuitMessage, PostThreadMessageW, RegisterClassW, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_SHOWNOACTIVATE, SetTimer, SetWindowDisplayAffinity, SetWindowsHookExW,
    ShowWindow, TranslateMessage, ULW_ALPHA, UnhookWindowsHookEx, UpdateLayeredWindow, WDA_EXCLUDEFROMCAPTURE,
    WH_KEYBOARD_LL, WM_APP, WM_KEYDOWN, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_MOUSEACTIVATE, WM_RBUTTONDOWN,
    WM_SYSKEYDOWN, WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP,
};
use windows::core::{PCWSTR, w};
use windows_numerics::Vector2;

use crate::overlay::{self as look, Caption};

type Rgb = (u8, u8, u8);
const WHITE: Rgb = (0xFF, 0xFF, 0xFF);
const KEYCAP: Rgb = (0x2E, 0x2E, 0x31);
const KEYCAP_EDGE: Rgb = (0x5A, 0x5A, 0x5E);
const LIVE_CLASS: PCWSTR = w!("RelayTarget");
const STILL_CLASS: PCWSTR = w!("RelayTargets");
/// About 60 frames a second.
const FRAME_MS: u32 = 15;

// — live marks —

struct Live {
    tx: Sender<(Mark, f64)>,
    thread: u32,
}

static LIVE: OnceLock<Option<Live>> = OnceLock::new();

pub fn live(mark: &Mark, speed: f64) {
    let Some(l) = LIVE.get_or_init(start).as_ref() else { return };
    if l.tx.send((mark.clone(), speed)).is_ok() {
        unsafe {
            let _ = PostThreadMessageW(l.thread, WM_APP, WPARAM(0), LPARAM(0));
        }
    }
}

fn start() -> Option<Live> {
    let (tx, rx) = unbounded();
    let (ready_tx, ready_rx) = crossbeam_channel::bounded(1);
    std::thread::Builder::new()
        .name("relay-targets".into())
        .spawn(move || unsafe {
            // The queue exists once a message function has run: then it can take thread messages.
            let mut msg = MSG::default();
            let _ = PeekMessageW(&mut msg, None, 0, 0, PM_NOREMOVE);
            let _ = ready_tx.send(GetCurrentThreadId());
            run_live(rx);
        })
        .ok()?;
    Some(Live { tx, thread: ready_rx.recv().ok()? })
}

/// A live mark on the screen.
struct Shown {
    hwnd: HWND,
    surface: Surface,
    /// The window's top left, on the virtual desktop.
    at: (i32, i32),
    mark: Mark,
    scale: f32,
    born: Instant,
    life: f64,
    draw: f64,
    visible: bool,
}

impl Drop for Shown {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe fn run_live(rx: Receiver<(Mark, f64)>) {
    unsafe {
        register();
        let mut painter: Option<Painter> = None;
        let mut shown: Vec<Shown> = Vec::new();
        let mut timer = 0usize;
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            if msg.message != WM_APP && msg.message != WM_TIMER {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
                continue;
            }
            if painter.is_none() {
                painter = Painter::new().ok();
            }
            let Some(p) = &painter else {
                rx.try_iter().for_each(drop);
                continue;
            };
            for (mark, speed) in rx.try_iter() {
                if mark.is_caption() {
                    for s in shown.iter_mut().filter(|s| s.mark.is_caption()) {
                        s.life = s.life.min(s.born.elapsed().as_secs_f64() * 1000.0 + look::REPLACED_MS);
                    }
                }
                shown.extend(open(p, mark, speed));
            }
            let mut lost = false;
            shown.retain_mut(|s| {
                let age = s.born.elapsed().as_secs_f64() * 1000.0;
                if age >= s.life {
                    return false;
                }
                lost |= !frame(p, s, age);
                true
            });
            if lost {
                // The device was lost (a driver update, say): start again with a new one.
                painter = None;
            }
            if shown.is_empty() {
                if timer != 0 {
                    let _ = KillTimer(None, timer);
                    timer = 0;
                }
            } else if timer == 0 {
                timer = SetTimer(None, 0, FRAME_MS, None);
            }
        }
    }
}

/// A window for `mark`, sized to what it draws; `None` if it can't be made.
unsafe fn open(p: &Painter, mark: Mark, speed: f64) -> Option<Shown> {
    unsafe {
        let (area, k) = monitor_at(mark.x, mark.y);
        let draw = look::draw_ms(&mark, speed);
        let life = look::life(&mark, draw);
        let reach = ((look::RING_TO + look::LINE_W + look::HALO_W) * k + 2.0).ceil() as i32;
        let r = match &mark.kind {
            MarkKind::Click { .. } => Rect { x: mark.x - reach, y: mark.y - reach, w: 2 * reach, h: 2 * reach },
            MarkKind::Scroll { .. } => {
                let reach = reach.max((44.0 * k) as i32);
                Rect { x: mark.x - reach, y: mark.y - reach, w: 2 * reach, h: 2 * reach }
            }
            MarkKind::Path { points, .. } => {
                let pad = (16.0 * k) as i32;
                let (x0, x1) = points.iter().fold((i32::MAX, i32::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
                let (y0, y1) = points.iter().fold((i32::MAX, i32::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
                if points.is_empty() {
                    return None;
                }
                Rect { x: x0 - pad, y: y0 - pad, w: x1 - x0 + 2 * pad, h: y1 - y0 + 2 * pad }
            }
            MarkKind::Keys { .. } | MarkKind::Text { .. } => {
                let cap = look::caption(&mark.kind)?;
                let (w, h) = p.pill_size(&cap, k);
                let (w, h) = (w.ceil() as i32 + 2, h.ceil() as i32 + 2);
                let (x, y) = look::place_caption(mark.x, mark.y, w, h, area, k);
                Rect { x, y, w, h }
            }
        };
        let surface = Surface::new(r.w.max(1), r.h.max(1))?;
        let hwnd = layered_window(LIVE_CLASS, r, true)?;
        Some(Shown { hwnd, surface, at: (r.x, r.y), mark, scale: k, born: Instant::now(), life, draw, visible: false })
    }
}

/// Draws `s` at `age` and shows it; `false` when Direct2D needs a new device.
unsafe fn frame(p: &Painter, s: &mut Shown, age: f64) -> bool {
    unsafe {
        if p.begin(&s.surface).is_err() {
            return false;
        }
        draw(p, &s.mark, s.at, s.scale, age, s.draw);
        if p.end().is_err() {
            return false;
        }
        present(s.hwnd, &s.surface, s.at, look::fade(age, s.life, look::FADE_MS));
        if !s.visible {
            let _ = ShowWindow(s.hwnd, SW_SHOWNOACTIVATE);
            s.visible = true;
        }
        true
    }
}

/// Draws `mark` at `age` into a bitmap whose top left is `origin` on the screen.
unsafe fn draw(p: &Painter, mark: &Mark, origin: (i32, i32), k: f32, age: f64, draw_ms: f64) {
    unsafe {
        let (x, y) = ((mark.x - origin.0) as f32, (mark.y - origin.1) as f32);
        match &mark.kind {
            &MarkKind::Click { btn, count } => {
                for ring in look::ripples(age, count).into_iter().flatten() {
                    let r = ring.r * k;
                    let w = look::LINE_W * k;
                    if look::square(btn) {
                        p.round_rect_halo((x - r, y - r, 2.0 * r, 2.0 * r), r * 0.35, w, ring.alpha);
                    } else {
                        p.circle_halo((x, y), r, w, ring.alpha);
                    }
                }
                p.dot((x, y), k);
            }
            MarkKind::Path { points, drag } => {
                let local: Vec<(i32, i32)> = points.iter().map(|&(px, py)| (px - origin.0, py - origin.1)).collect();
                let pts = look::partial(&local, look::progress(age, draw_ms));
                if *drag {
                    p.polyline_halo(&pts, 3.0 * k, 1.0);
                    if let Some(&first) = pts.first() {
                        p.dot(first, k);
                    }
                    if age >= draw_ms {
                        p.arrow_head(&pts, k);
                    }
                } else {
                    p.polyline(&pts, 1.6 * k, look::RED, 0.4, true);
                }
            }
            &MarkKind::Scroll { delta, horizontal } => {
                p.dot((x, y), k);
                let dir = if horizontal == (delta > 0) { 1.0 } else { -1.0 };
                let slide = ((age / 120.0) % 1.0) as f32 * 6.0 * k * dir;
                for i in 0..3 {
                    let d = (12.0 + i as f32 * 7.0) * k * dir + slide;
                    let (a, b) = (6.0 * k, 5.0 * k * dir);
                    let alpha = 1.0 - i as f32 * 0.25;
                    let pts = if horizontal {
                        [(x + d, y - a), (x + d + b, y), (x + d, y + a)]
                    } else {
                        [(x - a, y + d), (x, y + d + b), (x + a, y + d)]
                    };
                    p.polyline_halo(&pts, 2.4 * k, alpha);
                }
            }
            MarkKind::Keys { .. } | MarkKind::Text { .. } => {
                if let Some(cap) = look::caption(&mark.kind) {
                    let shown = match (&cap, &mark.kind) {
                        // Typed text appears as it's typed, over the step's time.
                        (Caption::Text(_), MarkKind::Text { text }) => {
                            let n = (text.chars().count() as f64 * (age / draw_ms).min(1.0)).ceil() as usize;
                            look::caption(&MarkKind::Text { text: text.chars().take(n).collect() }).unwrap_or(cap)
                        }
                        _ => cap,
                    };
                    p.pill(&shown, (1.0, 1.0), k);
                }
            }
        }
    }
}

// — the still overlay —

pub fn show_targets(marks: &[Mark], caption: &str) {
    unsafe { still(marks, caption) }
}

unsafe fn still(marks: &[Mark], caption: &str) {
    unsafe {
        register();
        let vd = Rect {
            x: GetSystemMetrics(SM_XVIRTUALSCREEN),
            y: GetSystemMetrics(SM_YVIRTUALSCREEN),
            w: GetSystemMetrics(SM_CXVIRTUALSCREEN),
            h: GetSystemMetrics(SM_CYVIRTUALSCREEN),
        };
        let Ok(p) = Painter::new() else { return };
        let Some(surface) = Surface::new(vd.w, vd.h) else { return };
        let Some(hwnd) = layered_window(STILL_CLASS, vd, false) else { return };
        if paint_still(&p, &surface, vd, marks, caption).is_err() {
            let _ = DestroyWindow(hwnd);
            return;
        }
        present(hwnd, &surface, (vd.x, vd.y), 1.0);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(esc), Some(instance()), 0).ok();
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if let Some(h) = hook {
            let _ = UnhookWindowsHookEx(h);
        }
        let _ = DestroyWindow(hwnd);
    }
}

unsafe fn paint_still(p: &Painter, s: &Surface, vd: Rect, marks: &[Mark], caption: &str) -> windows::core::Result<()> {
    unsafe {
        let at = |x: i32, y: i32| ((x - vd.x) as f32, (y - vd.y) as f32);
        p.begin(s)?;
        // A faint wash over everything, so a click anywhere lands on the overlay.
        p.rt.Clear(Some(&color(look::INK, 0.12)));
        let path = route(marks);
        for w in path.windows(2) {
            let k = monitor_at(w[0].0, w[0].1).1;
            let pts = [at(w[0].0, w[0].1), at(w[1].0, w[1].1)];
            p.polyline(&pts, 3.5 * k, WHITE, 0.7, false);
            p.polyline(&pts, 1.4 * k, look::INK, 0.8, true);
        }
        for m in marks.iter().filter(|m| m.number.is_some()) {
            if let MarkKind::Path { points, drag: true } = &m.kind {
                let k = monitor_at(m.x, m.y).1;
                let pts: Vec<(f32, f32)> = points.iter().map(|&(x, y)| at(x, y)).collect();
                p.polyline_halo(&pts, 3.0 * k, 1.0);
                p.arrow_head(&pts, k);
            }
        }
        for spot in spots(marks) {
            let (area, k) = monitor_at(spot.x, spot.y);
            let (x, y) = at(spot.x, spot.y);
            p.dot((x, y), k * 0.8);
            let targets: Vec<&Mark> = spot.marks.iter().map(|&i| &marks[i]).filter(|m| !m.is_caption()).collect();
            let size = (22.0 * k) as i32;
            let (bx, by) = look::place_badges(spot.x, spot.y, targets.len(), size, area, k);
            for (i, m) in targets.iter().enumerate() {
                let (cx, cy) = at(bx + i as i32 * size + size / 2, by + size / 2);
                let right = matches!(m.kind, MarkKind::Click { btn: MouseBtn::Right, .. });
                p.badge((cx, cy), m.number.unwrap_or(0), if right { look::INK } else { look::RED }, k);
                if let MarkKind::Scroll { delta, horizontal: false } = m.kind {
                    let dir = if delta > 0 { -1.0 } else { 1.0 };
                    for j in 0..3 {
                        let d = (12.0 + j as f32 * 7.0) * k * dir;
                        let (a, b) = (6.0 * k, 5.0 * k * dir);
                        p.polyline_halo(
                            &[(x - a - 14.0 * k, y + d), (x - 14.0 * k, y + d + b), (x + a - 14.0 * k, y + d)],
                            2.2 * k,
                            1.0,
                        );
                    }
                }
            }
            let mut ty = y + 12.0 * k;
            for m in spot.marks.iter().map(|&i| &marks[i]).filter(|m| m.is_caption()) {
                let Some(cap) = look::caption(&m.kind) else { continue };
                let text = match cap {
                    Caption::Keys(keys) => keys.join(" + "),
                    Caption::Text(t) => t,
                };
                let tag = Caption::Text(format!("{}  {}", m.number.unwrap_or(0), text));
                let (w, h) = p.pill_size_px(&tag, 11.0 * k, k);
                p.pill_at(&tag, (x - w / 2.0, ty), 11.0 * k, k);
                ty += h + 3.0 * k;
            }
        }
        // The bar: on the monitor with the cursor, at its bottom.
        let mut cur = POINT::default();
        let _ = GetCursorPos(&mut cur);
        let (area, k) = monitor_at(cur.x, cur.y);
        let bar = Caption::Text(caption.to_string());
        let (w, h) = p.pill_size_px(&bar, 12.5 * k, k);
        let (cx, cy) = at(area.x + area.w / 2, area.y + area.h);
        p.pill_at(&bar, (cx - w / 2.0, cy - h - 28.0 * k), 12.5 * k, k);
        p.end()
    }
}

unsafe extern "system" fn esc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        if code >= 0 && (wp.0 as u32 == WM_KEYDOWN || wp.0 as u32 == WM_SYSKEYDOWN) {
            let k = &*(lp.0 as *const KBDLLHOOKSTRUCT);
            if k.vkCode == VK_ESCAPE.0 as u32 {
                PostQuitMessage(0);
                return LRESULT(1);
            }
        }
        CallNextHookEx(None, code, wp, lp)
    }
}

unsafe extern "system" fn live_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wp, lp) }
}

unsafe extern "system" fn still_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

// — windows and bitmaps —

fn instance() -> HINSTANCE {
    unsafe { GetModuleHandleW(None).map(|m| HINSTANCE(m.0)).unwrap_or_default() }
}

unsafe fn register() {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| unsafe {
        let cursor = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(live_proc),
            hInstance: instance(),
            lpszClassName: LIVE_CLASS,
            ..Default::default()
        });
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(still_proc),
            hInstance: instance(),
            hCursor: cursor,
            lpszClassName: STILL_CLASS,
            ..Default::default()
        });
    });
}

/// A layered window over everything that never takes the focus, left out of
/// screen captures; hidden until shown.
unsafe fn layered_window(class: PCWSTR, r: Rect, click_through: bool) -> Option<HWND> {
    unsafe {
        let mut ex = WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;
        if click_through {
            ex |= WS_EX_TRANSPARENT;
        }
        let hwnd = CreateWindowExW(ex, class, w!(""), WS_POPUP, r.x, r.y, r.w, r.h, None, None, Some(instance()), None)
            .ok()?;
        let _ = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
        Some(hwnd)
    }
}

unsafe fn present(hwnd: HWND, s: &Surface, at: (i32, i32), alpha: f32) {
    unsafe {
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: (alpha.clamp(0.0, 1.0) * 255.0) as u8,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let _ = UpdateLayeredWindow(
            hwnd,
            None,
            Some(&POINT { x: at.0, y: at.1 }),
            Some(&SIZE { cx: s.w, cy: s.h }),
            Some(s.dc),
            Some(&POINT::default()),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );
    }
}

/// The monitor at a point and its scaling (1.0 at 96 DPI).
unsafe fn monitor_at(x: i32, y: i32) -> (Rect, f32) {
    unsafe {
        let m = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(m, &mut mi);
        let (mut dx, mut dy) = (96u32, 96u32);
        let _ = GetDpiForMonitor(m, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
        let r = mi.rcMonitor;
        (Rect { x: r.left, y: r.top, w: r.right - r.left, h: r.bottom - r.top }, dx.max(48) as f32 / 96.0)
    }
}

/// A 32-bit premultiplied bitmap in a memory DC, for `UpdateLayeredWindow`.
struct Surface {
    dc: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    #[cfg_attr(not(test), allow(dead_code))]
    bits: *const u8,
    w: i32,
    h: i32,
}

impl Surface {
    unsafe fn new(w: i32, h: i32) -> Option<Surface> {
        unsafe {
            let dc = CreateCompatibleDC(None);
            if dc.is_invalid() {
                return None;
            }
            let bi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w,
                    biHeight: -h,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            let Ok(bmp) = CreateDIBSection(Some(dc), &bi, DIB_RGB_COLORS, &mut bits, None, 0) else {
                let _ = DeleteDC(dc);
                return None;
            };
            let old = SelectObject(dc, bmp.into());
            Some(Surface { dc, bmp, old, bits: bits as *const u8, w, h })
        }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            let _ = DeleteObject(self.bmp.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

// — drawing —

fn color((r, g, b): Rgb, a: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F { r: r as f32 / 255.0, g: g as f32 / 255.0, b: b as f32 / 255.0, a }
}

fn v((x, y): (f32, f32)) -> Vector2 {
    Vector2 { X: x, Y: y }
}

struct Painter {
    factory: ID2D1Factory,
    dw: IDWriteFactory,
    rt: ID2D1DCRenderTarget,
    brush: ID2D1SolidColorBrush,
    round: ID2D1StrokeStyle,
    dots: ID2D1StrokeStyle,
    formats: RefCell<HashMap<(u32, bool, bool), IDWriteTextFormat>>,
}

impl Painter {
    unsafe fn new() -> windows::core::Result<Painter> {
        unsafe {
            let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let props = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                // Device-independent pixels are pixels: the marks scale themselves.
                dpiX: 96.0,
                dpiY: 96.0,
                usage: D2D1_RENDER_TARGET_USAGE_NONE,
                minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
            };
            let rt = factory.CreateDCRenderTarget(&props)?;
            let brush = rt.CreateSolidColorBrush(&color(look::RED, 1.0), None)?;
            let stroke = |dash| D2D1_STROKE_STYLE_PROPERTIES {
                startCap: D2D1_CAP_STYLE_ROUND,
                endCap: D2D1_CAP_STYLE_ROUND,
                dashCap: D2D1_CAP_STYLE_ROUND,
                lineJoin: D2D1_LINE_JOIN_ROUND,
                miterLimit: 10.0,
                dashStyle: dash,
                dashOffset: 0.0,
            };
            let round = factory.CreateStrokeStyle(&stroke(D2D1_DASH_STYLE_SOLID), None)?;
            // In line widths: short dashes, round-capped into dots.
            let dots = factory.CreateStrokeStyle(&stroke(D2D1_DASH_STYLE_CUSTOM), Some(&[0.6, 2.6]))?;
            let dw: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            Ok(Painter { factory, dw, rt, brush, round, dots, formats: RefCell::new(HashMap::new()) })
        }
    }

    unsafe fn begin(&self, s: &Surface) -> windows::core::Result<()> {
        unsafe {
            self.rt.BindDC(s.dc, &RECT { left: 0, top: 0, right: s.w, bottom: s.h })?;
            self.rt.BeginDraw();
            self.rt.Clear(Some(&color(WHITE, 0.0)));
            Ok(())
        }
    }

    unsafe fn end(&self) -> windows::core::Result<()> {
        unsafe { self.rt.EndDraw(None, None) }
    }

    unsafe fn paint(&self, rgb: Rgb, alpha: f32) -> &ID2D1SolidColorBrush {
        unsafe {
            self.brush.SetColor(&color(rgb, alpha));
            &self.brush
        }
    }

    unsafe fn polyline(&self, pts: &[(f32, f32)], width: f32, rgb: Rgb, alpha: f32, dotted: bool) {
        unsafe {
            if pts.len() < 2 {
                return;
            }
            let Ok(geo) = self.factory.CreatePathGeometry() else { return };
            let Ok(sink) = geo.Open() else { return };
            sink.BeginFigure(v(pts[0]), D2D1_FIGURE_BEGIN_HOLLOW);
            let rest: Vec<Vector2> = pts[1..].iter().map(|&p| v(p)).collect();
            sink.AddLines(&rest);
            sink.EndFigure(D2D1_FIGURE_END_OPEN);
            if sink.Close().is_err() {
                return;
            }
            let style = if dotted { &self.dots } else { &self.round };
            self.rt.DrawGeometry(&geo, self.paint(rgb, alpha), width, style);
        }
    }

    /// A red line with a white edge.
    unsafe fn polyline_halo(&self, pts: &[(f32, f32)], width: f32, alpha: f32) {
        unsafe {
            self.polyline(pts, width + look::HALO_W, WHITE, 0.9 * alpha, false);
            self.polyline(pts, width, look::RED, alpha, false);
        }
    }

    unsafe fn circle_halo(&self, c: (f32, f32), r: f32, width: f32, alpha: f32) {
        unsafe {
            let e = D2D1_ELLIPSE { point: v(c), radiusX: r, radiusY: r };
            self.rt.DrawEllipse(&e, self.paint(WHITE, 0.9 * alpha), width + look::HALO_W, &self.round);
            self.rt.DrawEllipse(&e, self.paint(look::RED, alpha), width, &self.round);
        }
    }

    unsafe fn round_rect_halo(&self, (x, y, w, h): (f32, f32, f32, f32), radius: f32, width: f32, alpha: f32) {
        unsafe {
            let r = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F { left: x, top: y, right: x + w, bottom: y + h },
                radiusX: radius,
                radiusY: radius,
            };
            self.rt.DrawRoundedRectangle(&r, self.paint(WHITE, 0.9 * alpha), width + look::HALO_W, &self.round);
            self.rt.DrawRoundedRectangle(&r, self.paint(look::RED, alpha), width, &self.round);
        }
    }

    /// The dot on the exact pixel: red, ringed in white.
    unsafe fn dot(&self, c: (f32, f32), k: f32) {
        unsafe {
            let e = D2D1_ELLIPSE { point: v(c), radiusX: look::DOT_R * k, radiusY: look::DOT_R * k };
            self.rt.FillEllipse(&e, self.paint(look::RED, 1.0));
            self.rt.DrawEllipse(&e, self.paint(WHITE, 1.0), 1.6 * k, &self.round);
        }
    }

    /// An arrow head at the end of `pts`, pointing along its last stretch.
    unsafe fn arrow_head(&self, pts: &[(f32, f32)], k: f32) {
        unsafe {
            let Some(&tip) = pts.last() else { return };
            let Some(&from) = pts.iter().rev().find(|p| (p.0 - tip.0).hypot(p.1 - tip.1) > 4.0 * k) else { return };
            let a = (tip.1 - from.1).atan2(tip.0 - from.0);
            let l = 11.0 * k;
            let side = |d: f32| (tip.0 - l * (a + d).cos(), tip.1 - l * (a + d).sin());
            self.polyline_halo(&[side(-0.45), tip, side(0.45)], 3.0 * k, 1.0);
        }
    }

    /// A step number in a disc.
    unsafe fn badge(&self, c: (f32, f32), n: u32, fill: Rgb, k: f32) {
        unsafe {
            let r = 10.5 * k;
            let e = D2D1_ELLIPSE { point: v(c), radiusX: r, radiusY: r };
            self.rt.FillEllipse(&e, self.paint(fill, 1.0));
            self.rt.DrawEllipse(&e, self.paint(WHITE, 1.0), 2.0 * k, &self.round);
            self.text(&n.to_string(), (c.0 - r, c.1 - r, 2.0 * r, 2.0 * r), 11.5 * k, true, true, WHITE);
        }
    }

    unsafe fn format(&self, px: f32, bold: bool, center: bool) -> Option<IDWriteTextFormat> {
        unsafe {
            let key = ((px * 10.0) as u32, bold, center);
            if let Some(f) = self.formats.borrow().get(&key) {
                return Some(f.clone());
            }
            let weight = if bold { DWRITE_FONT_WEIGHT_SEMI_BOLD } else { DWRITE_FONT_WEIGHT_NORMAL };
            let f = self
                .dw
                .CreateTextFormat(
                    w!("Segoe UI"),
                    None,
                    weight,
                    DWRITE_FONT_STYLE_NORMAL,
                    DWRITE_FONT_STRETCH_NORMAL,
                    px,
                    w!(""),
                )
                .ok()?;
            let _ =
                f.SetTextAlignment(if center { DWRITE_TEXT_ALIGNMENT_CENTER } else { DWRITE_TEXT_ALIGNMENT_LEADING });
            let _ = f.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
            let _ = f.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);
            self.formats.borrow_mut().insert(key, f.clone());
            Some(f)
        }
    }

    unsafe fn text_width(&self, s: &str, px: f32, bold: bool) -> f32 {
        unsafe {
            let Some(f) = self.format(px, bold, false) else { return 0.0 };
            let wide: Vec<u16> = s.encode_utf16().collect();
            let Ok(layout) = self.dw.CreateTextLayout(&wide, &f, 100_000.0, 1000.0) else { return 0.0 };
            let mut m = DWRITE_TEXT_METRICS::default();
            let _ = layout.GetMetrics(&mut m);
            m.widthIncludingTrailingWhitespace
        }
    }

    unsafe fn text(&self, s: &str, (x, y, w, h): (f32, f32, f32, f32), px: f32, bold: bool, center: bool, rgb: Rgb) {
        unsafe {
            let Some(f) = self.format(px, bold, center) else { return };
            let wide: Vec<u16> = s.encode_utf16().collect();
            let r = D2D_RECT_F { left: x, top: y, right: x + w, bottom: y + h };
            self.rt.DrawText(
                &wide,
                &f,
                &r,
                self.paint(rgb, 1.0),
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );
        }
    }

    unsafe fn pill_size(&self, cap: &Caption, k: f32) -> (f32, f32) {
        unsafe { self.pill_size_px(cap, 12.0 * k, k) }
    }

    unsafe fn pill_size_px(&self, cap: &Caption, px: f32, k: f32) -> (f32, f32) {
        unsafe {
            let h = px + 14.0 * k;
            match cap {
                Caption::Keys(keys) => {
                    let caps: f32 = keys.iter().map(|key| self.text_width(key, px, true) + 16.0 * k).sum();
                    (caps + (keys.len().max(1) - 1) as f32 * 4.0 * k + 12.0 * k, h)
                }
                Caption::Text(t) => (self.text_width(t, px, false) + 20.0 * k, h),
            }
        }
    }

    /// A caption pill at the top left of the bitmap.
    unsafe fn pill(&self, cap: &Caption, at: (f32, f32), k: f32) {
        unsafe { self.pill_at(cap, at, 12.0 * k, k) }
    }

    unsafe fn pill_at(&self, cap: &Caption, (x, y): (f32, f32), px: f32, k: f32) {
        unsafe {
            let (w, h) = self.pill_size_px(cap, px, k);
            let r = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F { left: x, top: y, right: x + w, bottom: y + h },
                radiusX: 8.0 * k,
                radiusY: 8.0 * k,
            };
            self.rt.FillRoundedRectangle(&r, self.paint(look::INK, 0.88));
            self.rt.DrawRoundedRectangle(&r, self.paint(WHITE, 0.55), 1.0 * k, &self.round);
            match cap {
                Caption::Keys(keys) => {
                    let mut kx = x + 6.0 * k;
                    for key in keys {
                        let kw = self.text_width(key, px, true) + 16.0 * k;
                        let kr = D2D1_ROUNDED_RECT {
                            rect: D2D_RECT_F { left: kx, top: y + 4.0 * k, right: kx + kw, bottom: y + h - 4.0 * k },
                            radiusX: 4.0 * k,
                            radiusY: 4.0 * k,
                        };
                        self.rt.FillRoundedRectangle(&kr, self.paint(KEYCAP, 1.0));
                        self.rt.DrawRoundedRectangle(&kr, self.paint(KEYCAP_EDGE, 1.0), 1.0 * k, &self.round);
                        self.text(key, (kx, y, kw, h), px, true, true, WHITE);
                        kx += kw + 4.0 * k;
                    }
                }
                Caption::Text(t) => self.text(t, (x + 10.0 * k, y, w - 10.0 * k, h), px, false, false, WHITE),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Screen;
    use relay_core::model::MouseBtn;
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, IsWindowVisible};

    /// A mark is left out of what the screen reads, so a pixel check or a
    /// Find image step under it still sees the app.
    #[test]
    fn a_live_mark_is_not_in_what_the_screen_reads() {
        let screen = super::super::screen::WinScreen;
        let (x, y) = (200, 200);
        let Some(before) = screen.pixel(x, y) else { return }; // no desktop to read
        let mark = Mark {
            first: 0,
            number: Some(1),
            t: 0,
            end: 0,
            x,
            y,
            kind: MarkKind::Click { btn: MouseBtn::Left, count: 1 },
        };
        live(&mark, 1.0);
        std::thread::sleep(std::time::Duration::from_millis(250));
        let shown = unsafe { FindWindowW(LIVE_CLASS, None) }.ok();
        assert!(shown.is_some_and(|h| unsafe { IsWindowVisible(h) }.as_bool()), "the mark shows");
        assert_eq!(screen.pixel(x, y), Some(before));
    }

    /// Every live mark on light, dark and red, for a look at how they draw
    /// (screenshots can't show them: they're left out of captures).
    #[test]
    #[ignore = "writes target/targets-look.png"]
    fn draws_the_marks_for_a_look() {
        unsafe {
            let (w, h, row) = (900, 390, 130);
            let p = Painter::new().unwrap();
            let s = Surface::new(w, h).unwrap();
            p.begin(&s).unwrap();
            let mk = |x, y, kind| Mark { first: 0, number: Some(1), t: 0, end: 800, x, y, kind };
            let click = |btn, count| MarkKind::Click { btn, count };
            for band in 0..3 {
                let y = band * row + 60;
                draw(&p, &mk(40, y, click(MouseBtn::Left, 1)), (0, 0), 1.0, 200.0, 1.0);
                draw(&p, &mk(110, y, click(MouseBtn::Left, 2)), (0, 0), 1.0, 300.0, 1.0);
                draw(&p, &mk(180, y, click(MouseBtn::Right, 1)), (0, 0), 1.0, 200.0, 1.0);
                draw(&p, &mk(240, y - 20, MarkKind::Scroll { delta: -120, horizontal: false }), (0, 0), 1.0, 0.0, 1.0);
                let drag = vec![(280, y + 20), (320, y - 30), (380, y - 25), (420, y + 10)];
                draw(&p, &mk(280, y, MarkKind::Path { points: drag, drag: true }), (0, 0), 1.0, 900.0, 800.0);
                let mv = vec![(440, y + 30), (480, y - 10), (520, y - 30)];
                draw(&p, &mk(440, y, MarkKind::Path { points: mv, drag: false }), (0, 0), 1.0, 900.0, 800.0);
                p.pill(&Caption::Keys(vec!["Ctrl".into(), "Shift".into(), "S".into()]), (545.0, y as f32 - 14.0), 1.0);
                p.pill(&Caption::Text("“Quarterly report for …”".into()), (690.0, y as f32 - 14.0), 1.0);
                draw(&p, &mk(670, y + 40, click(MouseBtn::Left, 1)), (0, 0), 1.5, 150.0, 1.0);
            }
            p.end().unwrap();
            let bands: [Rgb; 3] = [(0xF7, 0xF7, 0xF5), (0x1F, 0x20, 0x23), look::RED];
            let bits = std::slice::from_raw_parts(s.bits, (w * h * 4) as usize);
            let mut px = Vec::with_capacity((w * h * 3) as usize);
            for (i, c) in bits.chunks(4).enumerate() {
                let bg = bands[(i as i32 / w / row).min(2) as usize];
                let a = c[3] as f32 / 255.0;
                for (src, b) in [(c[2], bg.0), (c[1], bg.1), (c[0], bg.2)] {
                    px.push((src as f32 + b as f32 * (1.0 - a)).round().min(255.0) as u8);
                }
            }
            let png = relay_core::image::Rgb8 { w: w as usize, h: h as usize, px }.encode_png();
            let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/targets-look.png");
            std::fs::write(&out, png).unwrap();
        }
    }
}
