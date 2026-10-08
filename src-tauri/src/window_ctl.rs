//! The floating widget's window: placement anchored at its bottom-center,
//! remembered across runs, zoomed down on small screens, resizable as the
//! editor (not as the compact player), square corners, and no focus
//! stealing during sessions.

use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Monitor, Runtime, WebviewWindow};
use ts_rs::TS;

use crate::storage::write_atomic;

/// The widget's size at 100% zoom, in CSS pixels: the editor's by default
/// (the user can resize it), and the compact player's (fixed).
pub const EXPANDED: (f64, f64) = (944.0, 612.0);
pub const COMPACT: (f64, f64) = (604.0, 68.0);
/// The smallest editor: room for the transport, the four tabs, and the
/// preview and side panel at their narrowest (the UI's `layout.ts`).
pub const MIN_EXPANDED: (f64, f64) = (760.0, 520.0);
/// Space kept between the widget and the edges of the work area (CSS px).
const MARGIN: f64 = 16.0;
/// Default distance from the bottom of the work area, as in the design.
const BOTTOM_GAP: f64 = 24.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowPrefs {
    pub expanded: bool,
    /// Where the user put the widget's bottom-center, in physical
    /// virtual-desktop pixels. Placing the widget may keep it off an edge,
    /// but only a drag changes this.
    pub anchor: Option<(i32, i32)>,
    /// The editor's size, in CSS px, if the user resized it.
    pub size: Option<(f64, f64)>,
    /// Where the user put the editor's dividers.
    pub panes: Panes,
}

impl Default for WindowPrefs {
    fn default() -> Self {
        WindowPrefs { expanded: true, anchor: None, size: None, panes: Panes::default() }
    }
}

/// The editor's dividers, in CSS px: the preview's width, and the heights of
/// the button row and the timeline (`None`: the default). The UI keeps them
/// inside the window.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct Panes {
    pub preview_w: Option<f64>,
    pub transport_h: Option<f64>,
    pub timeline_h: Option<f64>,
}

impl Panes {
    /// Sane values only (the UI's own limits depend on the window): a value
    /// that isn't a size is dropped, a huge one is capped.
    pub fn sanitized(self) -> Panes {
        let ok = |v: Option<f64>| v.filter(|v| v.is_finite() && *v > 0.0).map(|v| v.min(10_000.0));
        Panes { preview_w: ok(self.preview_w), transport_h: ok(self.transport_h), timeline_h: ok(self.timeline_h) }
    }
}

/// The main window's handle (0 if there's none), to leave it out of screen
/// captures and recordings.
pub fn main_hwnd<R: Runtime>(app: &AppHandle<R>) -> isize {
    #[cfg(windows)]
    return app.get_webview_window("main").and_then(|w| w.hwnd().ok()).map_or(0, |h| h.0 as isize);
    #[cfg(not(windows))]
    return 0;
}

/// The editor's size: the one the user chose (never below [`MIN_EXPANDED`]), or the default.
pub fn expanded_size(prefs: &WindowPrefs) -> (f64, f64) {
    prefs.size.map_or(EXPANDED, |(w, h)| (w.max(MIN_EXPANDED.0), h.max(MIN_EXPANDED.1)))
}

/// The editor's size in CSS px once the user resized its client area to
/// `inner` physical px at this `zoom` and `scale` (rounded, never below the minimum).
pub fn resized_css(inner: (u32, u32), zoom: f64, scale: f64) -> (f64, f64) {
    let k = zoom * scale;
    ((inner.0 as f64 / k).round().max(MIN_EXPANDED.0), (inner.1 as f64 / k).round().max(MIN_EXPANDED.1))
}

/// A `css` size made to fit a `work` area (physical px, at `sf` and `zoom`)
/// with the margins, but never below [`MIN_EXPANDED`], which the zoom
/// always leaves room for.
pub fn fit_work(css: (f64, f64), work: Rect, sf: f64, zoom: f64) -> (f64, f64) {
    let k = sf * zoom;
    let max_w = (work.w as f64 / k - 2.0 * MARGIN).floor();
    let max_h = (work.h as f64 / k - MARGIN).floor();
    (css.0.min(max_w).max(MIN_EXPANDED.0), css.1.min(max_h).max(MIN_EXPANDED.1))
}

/// Window placement state, saved to `window.json` (Relay-only; the UI never writes it).
pub struct WindowState {
    path: PathBuf,
    prefs: Mutex<WindowPrefs>,
    /// The widget's current size in CSS px: the editor's (the user's), or
    /// the compact player's as the UI measured it.
    size: Mutex<(f64, f64)>,
    /// The zoom the window was last placed at.
    zoom: Mutex<f64>,
    /// When the anchor last changed and hasn't been saved yet.
    dirty: Mutex<Option<Instant>>,
    /// The bottom-center [`place`] last put the widget at, so the Moved
    /// events that raises aren't taken for a drag. Cleared by a drag.
    placed: Mutex<Option<(i32, i32)>>,
}

impl WindowState {
    pub fn open(dir: &std::path::Path) -> Self {
        let path = dir.join("window.json");
        let prefs: WindowPrefs =
            std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let size = if prefs.expanded { expanded_size(&prefs) } else { COMPACT };
        WindowState {
            path,
            prefs: Mutex::new(prefs),
            size: Mutex::new(size),
            zoom: Mutex::new(1.0),
            dirty: Mutex::new(None),
            placed: Mutex::new(None),
        }
    }

    pub fn prefs(&self) -> WindowPrefs {
        self.prefs.lock().clone()
    }

    /// Where a widget of `css` size goes in `work` (see [`layout`]), noted as
    /// placed there. The user's anchor stays as it is.
    fn placement(&self, work: Rect, sf: f64, zoom: f64, css: (f64, f64), anchor: Option<(i32, i32)>) -> Rect {
        let r = layout(work, sf, zoom, css, anchor);
        *self.placed.lock() = Some(r.bottom_center());
        r
    }

    /// The widget's bottom-center is now `at`. Arriving where it was placed
    /// changes nothing; anywhere else, the user dragged it, and that's the new anchor.
    fn moved(&self, at: (i32, i32)) {
        let mut placed = self.placed.lock();
        if *placed == Some(at) {
            return;
        }
        *placed = None;
        let mut p = self.prefs.lock();
        if p.anchor != Some(at) {
            p.anchor = Some(at);
            *self.dirty.lock() = Some(Instant::now());
        }
    }

    /// The user resized the editor to `css`, its bottom-center now at `at`.
    fn resized(&self, css: (f64, f64), at: (i32, i32)) {
        *self.size.lock() = css;
        *self.placed.lock() = None;
        let mut p = self.prefs.lock();
        p.size = Some(css);
        p.anchor = Some(at);
        *self.dirty.lock() = Some(Instant::now());
    }

    /// Saves where the user put the dividers.
    pub fn set_panes(&self, panes: Panes) {
        self.prefs.lock().panes = panes.sanitized();
        self.save();
    }

    /// Back to the default size and dividers (the position stays).
    fn reset_layout(&self) {
        {
            let mut p = self.prefs.lock();
            p.size = None;
            p.panes = Panes::default();
        }
        self.save();
    }

    fn save(&self) {
        let body = serde_json::to_string_pretty(&*self.prefs.lock()).expect("prefs serialize");
        let _ = write_atomic(&self.path, &body);
    }
}

/// The zoom that fits the expanded widget on `monitor` (never above 100%).
pub fn zoom_for(monitor: &Monitor) -> f64 {
    let work = monitor.work_area().size;
    zoom_for_work_area(work.width, work.height, monitor.scale_factor())
}

/// The zoom for a work area of `w` × `h` physical pixels at `scale` (1.25 = 125%).
pub fn zoom_for_work_area(w: u32, h: u32, scale: f64) -> f64 {
    let (w, h) = (w as f64 / scale, h as f64 / scale);
    ((w - 2.0 * MARGIN) / EXPANDED.0).min((h - MARGIN) / EXPANDED.1).clamp(0.4, 1.0)
}

/// A rectangle in physical virtual-desktop pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    fn work_area(m: &Monitor) -> Rect {
        let r = m.work_area();
        Rect { x: r.position.x, y: r.position.y, w: r.size.width as i32, h: r.size.height as i32 }
    }

    /// The whole monitor, taskbar included.
    fn bounds(m: &Monitor) -> Rect {
        let (p, s) = (m.position(), m.size());
        Rect { x: p.x, y: p.y, w: s.width as i32, h: s.height as i32 }
    }

    /// Whether a bottom-center anchor at `(x, y)` sits in this rect: a
    /// window standing on the bottom edge belongs to it, one hanging from the
    /// top edge doesn't.
    fn holds_anchor(&self, (x, y): (i32, i32)) -> bool {
        x >= self.x && y > self.y && x < self.x + self.w && y <= self.y + self.h
    }

    fn bottom_center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h)
    }

    /// How far `(x, y)` is from this rect, squared (0 inside it).
    fn distance2(&self, (x, y): (i32, i32)) -> i64 {
        let gap = |v: i32, lo: i32, hi: i32| (lo - v).max(v - hi).max(0) as i64;
        gap(x, self.x, self.x + self.w - 1).pow(2) + gap(y, self.y + 1, self.y + self.h).pow(2)
    }
}

/// Which of the `monitors` (their full bounds) the widget goes on: the one
/// its `anchor` is on (the taskbar included, as the widget is then kept above
/// it), else the nearest one (its monitor was unplugged, or the anchor is in
/// a gap between monitors), else the `primary` one. `monitors` isn't empty.
pub fn choose_monitor(monitors: &[Rect], anchor: Option<(i32, i32)>, primary: usize) -> usize {
    let Some(a) = anchor else { return primary.min(monitors.len() - 1) };
    if let Some(i) = monitors.iter().position(|m| m.holds_anchor(a)) {
        return i;
    }
    // Equally near: the primary monitor, then the first.
    (0..monitors.len()).min_by_key(|&i| (monitors[i].distance2(a), i != primary)).expect("a monitor")
}

/// Where a widget of `css` size goes in the `work` area at `sf` scale and
/// `zoom`: its bottom-center at `anchor` (or the default spot, centered near
/// the bottom), kept inside the work area with margins, and centered when it
/// can't fit them.
pub fn layout(work: Rect, sf: f64, zoom: f64, css: (f64, f64), anchor: Option<(i32, i32)>) -> Rect {
    let w = (css.0 * zoom * sf).round() as i32;
    let h = (css.1 * zoom * sf).round() as i32;
    let (ax, ay) = anchor.unwrap_or((work.x + work.w / 2, work.y + work.h - (BOTTOM_GAP * sf) as i32));
    let margin = (MARGIN * sf) as i32;
    let fit = |v: i32, lo: i32, hi: i32, center: i32| if hi < lo { center } else { v.clamp(lo, hi) };
    let x = fit(ax - w / 2, work.x + margin, work.x + work.w - w - margin, work.x + (work.w - w) / 2);
    let y = fit(ay - h, work.y + margin, work.y + work.h - h - margin, work.y + (work.h - h) / 2);
    Rect { x, y, w, h }
}

/// Sizes and positions the window for a widget of `css` size, keeping its
/// bottom-center at the saved anchor (or the default spot on the primary
/// monitor), inside the work area of the monitor [`choose_monitor`] picks.
/// The saved anchor stays where the user put it, so switching sizes near an
/// edge comes back to the same spot.
pub fn place(window: &WebviewWindow, state: &WindowState, css: (f64, f64)) {
    let prefs = state.prefs();
    let anchor = prefs.anchor;
    let Ok(monitors) = window.available_monitors() else { return };
    if monitors.is_empty() {
        return;
    }
    let bounds: Vec<Rect> = monitors.iter().map(Rect::bounds).collect();
    let primary = window
        .primary_monitor()
        .ok()
        .flatten()
        .and_then(|p| bounds.iter().position(|&b| b == Rect::bounds(&p)))
        .unwrap_or(0);
    let monitor = &monitors[choose_monitor(&bounds, anchor, primary)];

    let zoom = zoom_for(monitor);
    let (work, sf) = (Rect::work_area(monitor), monitor.scale_factor());
    let css = if prefs.expanded { fit_work(css, work, sf, zoom) } else { css };
    let _ = window.set_zoom(zoom);
    *state.zoom.lock() = zoom;
    set_resizing(window, prefs.expanded, zoom * sf);
    let r = state.placement(work, sf, zoom, css, anchor);
    set_client_rect(window, r.x, r.y, r.w, r.h);
    *state.size.lock() = css;
}

/// The editor can be resized (down to [`MIN_EXPANDED`] at `k` = zoom × scale), the compact player can't.
fn set_resizing(window: &WebviewWindow, expanded: bool, k: f64) {
    let min = expanded
        .then(|| tauri::PhysicalSize::new((MIN_EXPANDED.0 * k).round() as u32, (MIN_EXPANDED.1 * k).round() as u32));
    // The minimum goes first, so the compact player isn't held to the editor's.
    let _ = window.set_min_size(min.map(tauri::Size::Physical));
    let _ = window.set_resizable(expanded);
}

/// Called by the UI when it switches between the compact player and the
/// editor, and when the compact player's measured size (`css`) changes. The
/// editor's size is Relay's own (the user's, or the default), so `css` only
/// counts for the compact player.
pub fn fit(window: &WebviewWindow, state: &WindowState, css: (f64, f64), expanded: bool) {
    let (changed, css) = {
        let mut p = state.prefs.lock();
        let changed = p.expanded != expanded;
        p.expanded = expanded;
        (changed, fit_css(&p, css, expanded))
    };
    if *state.size.lock() != css || changed {
        place(window, state, css);
        state.save();
    }
}

/// The size [`fit`] gives the widget: the compact player's as the UI measured
/// it (`css`), or the editor's own (the UI sends 0 × 0 for it).
pub fn fit_css(prefs: &WindowPrefs, css: (f64, f64), expanded: bool) -> (f64, f64) {
    if expanded { expanded_size(prefs) } else { css }
}

/// *Reset layout*: the editor goes back to its default size and dividers.
pub fn reset_layout(window: &WebviewWindow, state: &WindowState) {
    state.reset_layout();
    if state.prefs().expanded {
        place(window, state, EXPANDED);
    }
}

/// Tracks the editor's size while the user resizes it; saved shortly after it
/// stops. Sizes Relay sets itself, and the move loop's own after a scale
/// change, aren't the user's.
pub fn on_resized(window: &WebviewWindow, state: &WindowState) {
    if !RESIZING.load(Ordering::SeqCst) || !state.prefs().expanded {
        return;
    }
    let (Ok(pos), Ok(size), Ok(sf)) = (window.inner_position(), window.inner_size(), window.scale_factor()) else {
        return;
    };
    let css = resized_css((size.width, size.height), *state.zoom.lock(), sf);
    state.resized(css, (pos.x + size.width as i32 / 2, pos.y + size.height as i32));
}

/// Where the window goes once the scale of the monitor it's on changed (it
/// was dragged onto another monitor, or the scaling setting changed): sized
/// for `sf` and `zoom`. During a drag, its top-left corner stays under the
/// cursor, where Windows put it; otherwise it's laid out at the `anchor`
/// inside `work`.
pub fn rescaled(
    client: Rect,
    work: Rect,
    sf: f64,
    zoom: f64,
    css: (f64, f64),
    anchor: Option<(i32, i32)>,
    dragging: bool,
) -> Rect {
    let r = layout(work, sf, zoom, css, anchor);
    if dragging { Rect { x: client.x, y: client.y, ..r } } else { r }
}

/// Re-sizes the window for the monitor it's on after a scale change. Call it
/// once the window system has applied its own rect for the new scale (not
/// from inside the `ScaleFactorChanged` event, which would be overwritten),
/// and on the monitor Windows picked for the new scale, which isn't always
/// the one the anchor is on.
pub fn rescale(window: &WebviewWindow, state: &WindowState) {
    let (Ok(Some(monitor)), Ok(pos), Ok(size)) =
        (window.current_monitor(), window.inner_position(), window.inner_size())
    else {
        return;
    };
    let client = Rect { x: pos.x, y: pos.y, w: size.width as i32, h: size.height as i32 };
    let zoom = zoom_for(&monitor);
    let _ = window.set_zoom(zoom);
    *state.zoom.lock() = zoom;
    let dragging = in_move_loop(window);
    let (work, sf) = (Rect::work_area(&monitor), monitor.scale_factor());
    let expanded = state.prefs().expanded;
    set_resizing(window, expanded, zoom * sf);
    let css = *state.size.lock();
    let css = if expanded && !dragging { fit_work(css, work, sf, zoom) } else { css };
    *state.size.lock() = css;
    let r = rescaled(client, work, sf, zoom, css, state.prefs().anchor, dragging);
    if r != client {
        if !dragging {
            *state.placed.lock() = Some(r.bottom_center());
        }
        set_client_rect(window, r.x, r.y, r.w, r.h);
    }
    if dragging {
        // A resize keeps following the mouse; only a move holds its size.
        if !RESIZING.load(Ordering::SeqCst)
            && let Ok(outer) = window.outer_size()
        {
            hold_size(outer.width, outer.height);
        }
        rescale_after_drag(window);
    }
}

/// Whether the user is resizing the window (from `WM_SIZING` to the end of the loop).
static RESIZING: AtomicBool = AtomicBool::new(false);

/// The outer size the window keeps for the rest of a drag (width in the high
/// half, height in the low one), or 0. Windows' move loop keeps the size it
/// suggested for a new scale (often a pixel or two off) and re-applies it on
/// every mouse move, which would undo [`rescale`]'s; [`watch_move_size`]
/// substitutes this one.
static HELD_SIZE: AtomicU64 = AtomicU64::new(0);

fn hold_size(w: u32, h: u32) {
    HELD_SIZE.store((w as u64) << 32 | h as u64, Ordering::SeqCst);
}

/// Once the widget is dropped after a scale change, it stops holding its size,
/// is sized again and is kept inside the work area of the monitor it's on.
fn rescale_after_drag(window: &WebviewWindow) {
    static WATCHING: AtomicBool = AtomicBool::new(false);
    if WATCHING.swap(true, Ordering::SeqCst) {
        return;
    }
    let window = window.clone();
    std::thread::spawn(move || {
        while in_move_loop(&window) {
            std::thread::sleep(Duration::from_millis(50));
        }
        WATCHING.store(false, Ordering::SeqCst);
        HELD_SIZE.store(0, Ordering::SeqCst);
        let w = window.clone();
        let _ = window.run_on_main_thread(move || rescale(&w, &w.app_handle().state::<WindowState>()));
    });
}

/// Whether the user is dragging the window (Windows' move loop is running).
#[cfg(windows)]
fn in_move_loop(window: &WebviewWindow) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        GUI_INMOVESIZE, GUITHREADINFO, GetGUIThreadInfo, GetWindowThreadProcessId,
    };
    let Ok(hwnd) = window.hwnd() else { return false };
    let mut info = GUITHREADINFO { cbSize: size_of::<GUITHREADINFO>() as u32, ..Default::default() };
    unsafe {
        let thread = GetWindowThreadProcessId(hwnd, None);
        GetGUIThreadInfo(thread, &mut info).is_ok() && info.flags.contains(GUI_INMOVESIZE) && info.hwndMoveSize == hwnd
    }
}

/// Watches Windows' move and size loop:
/// - a move keeps the size [`rescale`] gave the window after a scale change
///   mid-drag (see [`HELD_SIZE`]); a new scale change lets the window
///   system's own rect for it through, until `rescale` holds the next size;
/// - a resize by the user ([`RESIZING`], for [`on_resized`]) is told apart
///   from the ones Relay and the move loop make, and never holds a size.
#[cfg(windows)]
pub fn watch_move_size(window: &WebviewWindow) {
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
    use windows::Win32::UI::WindowsAndMessaging::{
        SWP_NOSIZE, WINDOWPOS, WM_DPICHANGED, WM_EXITSIZEMOVE, WM_SIZING, WM_WINDOWPOSCHANGING,
    };

    unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM, _id: usize, _data: usize) -> LRESULT {
        match msg {
            WM_SIZING => {
                RESIZING.store(true, Ordering::SeqCst);
                HELD_SIZE.store(0, Ordering::SeqCst);
            }
            WM_EXITSIZEMOVE => RESIZING.store(false, Ordering::SeqCst),
            WM_DPICHANGED => HELD_SIZE.store(0, Ordering::SeqCst),
            WM_WINDOWPOSCHANGING => {
                let held = HELD_SIZE.load(Ordering::SeqCst);
                // SAFETY: WM_WINDOWPOSCHANGING's lParam points to the WINDOWPOS being applied.
                let pos = unsafe { &mut *(lp.0 as *mut WINDOWPOS) };
                if held != 0 && !pos.flags.contains(SWP_NOSIZE) {
                    pos.cx = (held >> 32) as i32;
                    pos.cy = (held & 0xffff_ffff) as i32;
                }
            }
            _ => {}
        }
        unsafe { DefSubclassProc(hwnd, msg, wp, lp) }
    }

    let Ok(hwnd) = window.hwnd() else { return };
    unsafe {
        let _ = SetWindowSubclass(hwnd, Some(proc), 1, 0);
    }
}

#[cfg(not(windows))]
pub fn watch_move_size(_window: &WebviewWindow) {}

#[cfg(not(windows))]
fn in_move_loop(_window: &WebviewWindow) -> bool {
    false
}

/// Closing the widget (its × or Alt+F4): hides it to the tray when *Close to
/// tray* is on. Returns whether it hid it; if not, the caller quits.
pub fn close_or_hide(window: &WebviewWindow) -> bool {
    let to_tray = window.app_handle().state::<Mutex<crate::settings::SettingsStore>>().lock().current.close_to_tray;
    if to_tray {
        let _ = window.hide();
    }
    to_tray
}

/// Tracks the widget's bottom-center while the user drags it; saved shortly after it stops.
pub fn on_moved(window: &WebviewWindow, state: &WindowState) {
    let (Ok(pos), Ok(size)) = (window.inner_position(), window.inner_size()) else { return };
    state.moved((pos.x + size.width as i32 / 2, pos.y + size.height as i32));
}

/// Saves a moved position once dragging has paused for half a second.
pub fn spawn_autosave(app: AppHandle) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(250));
            let state = app.state::<WindowState>();
            let due = state.dirty.lock().is_some_and(|t| t.elapsed() >= Duration::from_millis(500));
            if due {
                *state.dirty.lock() = None;
                state.save();
            }
        }
    });
}

/// Moves and sizes the window so its *client* area has this rect, in one call.
#[cfg(windows)]
fn set_client_rect(window: &WebviewWindow, x: i32, y: i32, w: i32, h: i32) {
    use windows::Win32::UI::WindowsAndMessaging::{SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos};
    // The outer frame includes invisible resize borders around the client area.
    let (Ok(op), Ok(os), Ok(ip), Ok(is)) =
        (window.outer_position(), window.outer_size(), window.inner_position(), window.inner_size())
    else {
        return;
    };
    let (left, top) = (ip.x - op.x, ip.y - op.y);
    let (extra_w, extra_h) = (os.width as i32 - is.width as i32, os.height as i32 - is.height as i32);
    let Ok(hwnd) = window.hwnd() else { return };
    unsafe {
        let _ = SetWindowPos(hwnd, None, x - left, y - top, w + extra_w, h + extra_h, SWP_NOZORDER | SWP_NOACTIVATE);
    }
}

#[cfg(not(windows))]
fn set_client_rect(window: &WebviewWindow, x: i32, y: i32, w: i32, h: i32) {
    let _ = window.set_size(tauri::PhysicalSize::new(w as u32, h as u32));
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
}

/// Applies the *Keep on top* setting; `session` is whether a recording or
/// playback (or its countdown) is running.
pub fn apply_on_top<R: Runtime>(window: &WebviewWindow<R>, setting: crate::settings::KeepOnTop, session: bool) {
    let _ = window.set_always_on_top(setting.on_top(session));
}

/// While a session runs, clicking the widget must not take the keyboard away
/// from the app being recorded or played into.
#[cfg(windows)]
pub fn set_no_activate<R: Runtime>(window: &WebviewWindow<R>, on: bool) {
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_NOACTIVATE,
    };
    let Ok(hwnd) = window.hwnd() else { return };
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let flag = WS_EX_NOACTIVATE.0 as isize;
        let next = if on { style | flag } else { style & !flag };
        if next != style {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next);
        }
    }
}

#[cfg(not(windows))]
pub fn set_no_activate<R: Runtime>(_window: &WebviewWindow<R>, _on: bool) {}

/// Makes the window match the design system: square corners and no
/// accent-colored DWM border (Windows 11 rounds and outlines top-level
/// windows by default). The widget draws its own 2px ink border in CSS.
#[cfg(windows)]
pub fn apply_modernist_frame(window: &WebviewWindow) {
    use std::ffi::c_void;
    use windows::Win32::Graphics::Dwm::{
        DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DwmSetWindowAttribute,
    };

    let Ok(hwnd) = window.hwnd() else { return };
    // Both attributes exist on Windows 11 only; on Windows 10 the calls fail
    // harmlessly and the (already square) default frame is kept.
    unsafe {
        let corner = DWMWCP_DONOTROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            &corner as *const _ as *const c_void,
            size_of_val(&corner) as u32,
        );
        let color = DWMWA_COLOR_NONE;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_BORDER_COLOR,
            &color as *const _ as *const c_void,
            size_of_val(&color) as u32,
        );
    }
}

#[cfg(not(windows))]
pub fn apply_modernist_frame(_window: &WebviewWindow) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_fits_small_screens_only() {
        // 1920x1080 at 100% (taskbar 48 px): the widget fits as designed.
        assert_eq!(zoom_for_work_area(1920, 1032, 1.0), 1.0);
        // 1366x768 at 125%: 1093x590 logical, so it shrinks to fit the height.
        let z = zoom_for_work_area(1366, 720, 1.25);
        assert!((z - (576.0 - 16.0) / 612.0).abs() < 1e-9, "{z}");
        assert!(612.0 * z <= 576.0 - 16.0 + 1e-9);
        // Never below 40%.
        assert_eq!(zoom_for_work_area(400, 300, 1.0), 0.4);
        // Width-bound: a tall, narrow portrait monitor.
        let z = zoom_for_work_area(800, 1400, 1.0);
        assert!((z - (800.0 - 32.0) / 944.0).abs() < 1e-9, "{z}");
    }

    const WORK: Rect = Rect { x: 0, y: 0, w: 1920, h: 1032 };

    #[test]
    fn the_default_spot_is_bottom_center() {
        let r = layout(WORK, 1.0, 1.0, EXPANDED, None);
        assert_eq!(r, Rect { x: (1920 - 944) / 2, y: 1032 - 24 - 612, w: 944, h: 612 });
        // At 150%, sizes and the gap scale.
        let big = Rect { x: 0, y: 0, w: 2880, h: 1548 };
        let r = layout(big, 1.5, 1.0, COMPACT, None);
        assert_eq!((r.w, r.h), (906, 102));
        assert_eq!(r.y + r.h, 1548 - 36);
    }

    #[test]
    fn the_anchor_is_the_bottom_center() {
        let r = layout(WORK, 1.0, 1.0, COMPACT, Some((700, 500)));
        assert_eq!(r, Rect { x: 700 - 302, y: 500 - 68, w: 604, h: 68 });
        // Switching to expanded keeps the bottom-center where it was.
        let r = layout(WORK, 1.0, 1.0, EXPANDED, Some((700, 900)));
        assert_eq!((r.x + r.w / 2, r.y + r.h), (700, 900));
    }

    #[test]
    fn the_widget_stays_inside_the_work_area() {
        // Anchored near the left edge: pushed right to the margin.
        assert_eq!(layout(WORK, 1.0, 1.0, EXPANDED, Some((10, 900))).x, 16);
        // Near the right edge.
        assert_eq!(layout(WORK, 1.0, 1.0, EXPANDED, Some((1915, 900))).x, 1920 - 944 - 16);
        // Near the top: expanding downward instead of off the screen.
        assert_eq!(layout(WORK, 1.0, 1.0, EXPANDED, Some((960, 100))).y, 16);
        // Below the bottom (a taller taskbar now): pulled up.
        assert_eq!(layout(WORK, 1.0, 1.0, EXPANDED, Some((960, 2000))).y, 1032 - 612 - 16);
        // The margins scale too.
        let big = Rect { x: 0, y: 0, w: 2880, h: 1548 };
        assert_eq!(layout(big, 1.5, 1.0, COMPACT, Some((1440, 5))).y, 24);
        // A second monitor to the left, at negative coordinates.
        let left = Rect { x: -1280, y: 0, w: 1280, h: 984 };
        let r = layout(left, 1.0, 1.0, COMPACT, Some((-5, 500)));
        assert_eq!(r.x, -16 - 604);
    }

    #[test]
    fn a_widget_too_big_for_its_margins_is_centered() {
        let narrow = Rect { x: 100, y: 0, w: 960, h: 700 };
        let r = layout(narrow, 1.0, 1.0, EXPANDED, Some((150, 700)));
        assert_eq!(r.x, 100 + (960 - 944) / 2);
        // A short work area (the zoom leaves the expanded widget 16 px shorter than it).
        let short = Rect { x: 0, y: 40, w: 1920, h: 628 };
        for anchor in [(960, 668), (960, 100)] {
            assert_eq!(layout(short, 1.0, 1.0, EXPANDED, Some(anchor)).y, 40 + 8);
        }
    }

    #[test]
    fn zoom_shrinks_the_window() {
        let r = layout(WORK, 1.25, 0.8, EXPANDED, None);
        assert_eq!((r.w, r.h), (944, 612));
    }

    #[test]
    fn a_scale_change_sizes_the_widget_for_the_monitor_it_is_on() {
        // Dragged from a 150% monitor onto a Full HD one lower down: Windows
        // put it at (2600, 900) with its own idea of the size.
        let fhd = Rect { x: 2560, y: 87, w: 1920, h: 1032 };
        let client = Rect { x: 2600, y: 900, w: 1416, h: 918 };
        let anchor = Some((2600 + 708, 900 + 918));
        let r = rescaled(client, fhd, 1.0, 1.0, EXPANDED, anchor, true);
        assert_eq!(r, Rect { x: 2600, y: 900, w: 944, h: 612 }, "the top-left stays under the cursor");
        // Not dragging (the scaling setting changed): kept inside the work area.
        let r = rescaled(client, fhd, 1.0, 1.0, EXPANDED, anchor, false);
        assert_eq!((r.w, r.h), (944, 612));
        assert_eq!(r.y + r.h, 87 + 1032 - 16);
        // Back onto the 150% monitor, compact.
        let primary = Rect { x: 0, y: 0, w: 2560, h: 1528 };
        let client = Rect { x: 1500, y: 700, w: 604, h: 68 };
        let r = rescaled(client, primary, 1.5, 1.0, COMPACT, None, true);
        assert_eq!(r, Rect { x: 1500, y: 700, w: 906, h: 102 });
    }

    #[test]
    fn an_anchor_on_the_bottom_edge_belongs_to_the_monitor_above_it() {
        let top = Rect { x: 0, y: 0, w: 1920, h: 1080 };
        let below = Rect { x: 0, y: 1080, w: 1920, h: 1080 };
        assert!(top.holds_anchor((960, 1080)));
        assert!(!below.holds_anchor((960, 1080)));
        assert!(below.holds_anchor((960, 1081)));
        assert!(!top.holds_anchor((1920, 500)), "the right edge is the next monitor's");
        assert!(top.holds_anchor((0, 500)));
        assert!(!top.holds_anchor((-1, 500)));
    }

    #[test]
    fn the_monitor_is_the_one_the_anchor_is_on_or_the_nearest() {
        // The primary monitor, a smaller one to its right, top-aligned, and one to its left.
        let primary = Rect { x: 0, y: 0, w: 1920, h: 1080 };
        let right = Rect { x: 1920, y: 0, w: 1280, h: 720 };
        let left = Rect { x: -1280, y: 0, w: 1280, h: 1024 };
        let monitors = [left, primary, right];
        assert_eq!(choose_monitor(&monitors, None, 1), 1, "the default spot is on the primary monitor");
        assert_eq!(choose_monitor(&monitors, Some((2500, 700)), 1), 2);
        // Over the taskbar: still that monitor, and pulled up into its work area.
        assert_eq!(choose_monitor(&monitors, Some((2500, 719)), 1), 2);
        let work = Rect { h: 1032, ..primary };
        assert_eq!(choose_monitor(&monitors, Some((960, 1070)), 1), 1);
        assert_eq!(layout(work, 1.0, 1.0, COMPACT, Some((960, 1070))).y, 1032 - 16 - 68);
        // Negative coordinates.
        assert_eq!(choose_monitor(&monitors, Some((-5, 500)), 1), 0);
        assert_eq!(choose_monitor(&monitors, Some((-1280, 1024)), 1), 0);
        // In the gap below the smaller monitor: the nearest one.
        assert_eq!(choose_monitor(&monitors, Some((2500, 900)), 1), 2);
        assert_eq!(choose_monitor(&monitors, Some((1990, 1060)), 1), 1);
        // On a monitor that's gone (it was to the right of the smaller one, or above them all).
        assert_eq!(choose_monitor(&monitors, Some((4000, 500)), 1), 2);
        assert_eq!(choose_monitor(&monitors, Some((960, -300)), 1), 1);
        assert_eq!(choose_monitor(&[left, primary], Some((2500, 700)), 1), 1);
        // Equally near two monitors: the primary one.
        let (a, b) = (Rect { x: 0, y: 0, w: 100, h: 100 }, Rect { x: 199, y: 0, w: 100, h: 100 });
        assert_eq!(choose_monitor(&[a, b], Some((149, 50)), 1), 1);
        assert_eq!(choose_monitor(&[a, b], Some((149, 50)), 0), 0);
    }

    #[test]
    fn switching_sizes_near_an_edge_comes_back_to_the_same_spot() {
        let dir = tempfile::tempdir().unwrap();
        let state = WindowState::open(dir.path());
        // Compact fits here as placed; expanded is pushed left, off the right edge.
        state.moved((1500, 1000));
        // Placing, then the Moved event the placement raises.
        let switch = |css| {
            let r = state.placement(WORK, 1.0, 1.0, css, state.prefs().anchor);
            state.moved(r.bottom_center());
            r
        };
        let compact = switch(COMPACT);
        assert_eq!(compact.bottom_center(), (1500, 1000));
        let expanded = switch(EXPANDED);
        assert_eq!(expanded.x, 1920 - 944 - 16);
        assert_eq!(state.prefs().anchor, Some((1500, 1000)), "clamping isn't a move");
        assert_eq!(switch(COMPACT), compact);
        assert_eq!(switch(EXPANDED), expanded);
    }

    #[test]
    fn a_drag_moves_the_anchor_even_back_to_where_it_was_placed() {
        let dir = tempfile::tempdir().unwrap();
        let state = WindowState::open(dir.path());
        let placed = state.placement(WORK, 1.0, 1.0, COMPACT, None).bottom_center();
        state.moved(placed);
        assert_eq!(state.prefs().anchor, None, "the default spot follows the monitor");
        assert!(state.dirty.lock().is_none());
        state.moved((700, 500));
        assert_eq!(state.prefs().anchor, Some((700, 500)));
        assert!(state.dirty.lock().is_some(), "saved once the drag pauses");
        state.moved(placed);
        assert_eq!(state.prefs().anchor, Some(placed));
    }

    #[test]
    fn prefs_survive_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let fresh = WindowState::open(dir.path());
        assert!(fresh.prefs().expanded);
        assert_eq!(fresh.prefs().anchor, None);
        assert_eq!(*fresh.size.lock(), EXPANDED);

        let panes = Panes { preview_w: Some(520.0), transport_h: Some(110.0), timeline_h: Some(180.0) };
        *fresh.prefs.lock() =
            WindowPrefs { expanded: false, anchor: Some((-300, 700)), size: Some((1200.0, 800.0)), panes };
        fresh.save();
        let again = WindowState::open(dir.path());
        assert!(!again.prefs().expanded);
        assert_eq!(again.prefs().anchor, Some((-300, 700)));
        assert_eq!(again.prefs().size, Some((1200.0, 800.0)));
        assert_eq!(again.prefs().panes, panes);
        assert_eq!(*again.size.lock(), COMPACT, "opens at the size it was left at");
        // Expanded, it opens at the editor's size the user chose.
        again.prefs.lock().expanded = true;
        again.save();
        assert_eq!(*WindowState::open(dir.path()).size.lock(), (1200.0, 800.0));
    }

    #[test]
    fn the_editor_opens_at_the_users_size_never_below_the_minimum() {
        let mut p = WindowPrefs::default();
        assert_eq!(expanded_size(&p), EXPANDED, "the default");
        p.size = Some((1400.0, 900.0));
        assert_eq!(expanded_size(&p), (1400.0, 900.0));
        p.size = Some((300.0, 900.0));
        assert_eq!(expanded_size(&p), (MIN_EXPANDED.0, 900.0), "a size from a hand-edited file");
    }

    #[test]
    fn the_ui_sizes_the_compact_player_and_never_the_editor() {
        let mut p = WindowPrefs::default();
        assert_eq!(fit_css(&p, COMPACT, false), COMPACT);
        assert_eq!(fit_css(&p, (0.0, 0.0), true), EXPANDED, "the editor's 0 × 0 is ignored");
        p.size = Some((1200.0, 800.0));
        assert_eq!(fit_css(&p, (0.0, 0.0), true), (1200.0, 800.0));
        assert_eq!(
            fit_css(&p, (604.0, 68.0), false),
            (604.0, 68.0),
            "the user's editor size isn't the compact player's"
        );
    }

    #[test]
    fn a_resize_by_the_user_is_measured_in_css_px() {
        assert_eq!(resized_css((1200, 800), 1.0, 1.0), (1200.0, 800.0));
        // At 150%, 1800 × 1200 physical px is 1200 × 800 CSS px.
        assert_eq!(resized_css((1800, 1200), 1.0, 1.5), (1200.0, 800.0));
        assert_eq!(resized_css((1801, 1199), 1.0, 1.5), (1201.0, 799.0), "rounded");
        // Zoomed down to 80% on a small screen.
        assert_eq!(resized_css((800, 480), 0.8, 1.0), (1000.0, 600.0));
        assert_eq!(resized_css((100, 100), 1.0, 1.0), MIN_EXPANDED, "never below the minimum");
    }

    #[test]
    fn a_big_editor_is_made_to_fit_the_work_area() {
        assert_eq!(fit_work((1200.0, 800.0), WORK, 1.0, 1.0), (1200.0, 800.0));
        assert_eq!(fit_work((2400.0, 1400.0), WORK, 1.0, 1.0), (1920.0 - 32.0, 1032.0 - 16.0));
        // At 150%: 2880 × 1548 physical px hold 1920 × 1032 CSS px.
        let big = Rect { x: 0, y: 0, w: 2880, h: 1548 };
        assert_eq!(fit_work((2400.0, 1400.0), big, 1.5, 1.0), (1888.0, 1016.0));
        // A work area smaller than the minimum (the zoom shrinks it to fit): the minimum.
        let tiny = Rect { x: 0, y: 0, w: 700, h: 500 };
        assert_eq!(fit_work(EXPANDED, tiny, 1.0, 1.0), MIN_EXPANDED);
    }

    #[test]
    fn a_resize_moves_the_anchor_and_is_saved_and_a_reset_forgets_it() {
        let dir = tempfile::tempdir().unwrap();
        let state = WindowState::open(dir.path());
        state.resized((1200.0, 800.0), (900, 1000));
        assert_eq!(*state.size.lock(), (1200.0, 800.0));
        assert_eq!(state.prefs().size, Some((1200.0, 800.0)));
        assert_eq!(state.prefs().anchor, Some((900, 1000)));
        assert!(state.dirty.lock().is_some(), "saved once the resize pauses");
        state.set_panes(Panes { preview_w: Some(700.0), ..Panes::default() });
        let saved = WindowState::open(dir.path()).prefs();
        assert_eq!(saved.panes, Panes { preview_w: Some(700.0), ..Panes::default() }, "saved at once");

        state.reset_layout();
        let reset = WindowState::open(dir.path()).prefs();
        assert_eq!((reset.size, reset.panes), (None, Panes::default()));
        assert_eq!(reset.anchor, Some((900, 1000)), "the position stays");
    }

    #[test]
    fn panes_that_arent_sizes_are_dropped() {
        let p = Panes { preview_w: Some(f64::NAN), transport_h: Some(0.0), timeline_h: Some(-5.0) }.sanitized();
        assert_eq!(p, Panes::default());
        let p = Panes { preview_w: Some(1e9), transport_h: Some(100.0), timeline_h: Some(120.0) }.sanitized();
        assert_eq!(p, Panes { preview_w: Some(10_000.0), transport_h: Some(100.0), timeline_h: Some(120.0) });
    }

    #[test]
    fn broken_or_partial_prefs_fall_back_to_defaults() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("window.json"), "{ not json").unwrap();
        assert!(WindowState::open(dir.path()).prefs().expanded);
        // A file from before resizing: no size, no panes.
        std::fs::write(dir.path().join("window.json"), r#"{"anchor":[5,6]}"#).unwrap();
        let p = WindowState::open(dir.path()).prefs();
        assert!(p.expanded);
        assert_eq!(p.anchor, Some((5, 6)));
        assert_eq!((p.size, p.panes), (None, Panes::default()));
        std::fs::write(dir.path().join("window.json"), r#"{"panes":{"preview_w":480}}"#).unwrap();
        assert_eq!(WindowState::open(dir.path()).prefs().panes, Panes { preview_w: Some(480.0), ..Panes::default() });
    }
}
