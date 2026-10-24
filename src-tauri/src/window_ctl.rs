//! Relay's windows: the main window's placement, anchored at its
//! bottom-center, remembered across runs, zoomed by the text size and down on
//! small screens, resizable, never on top, and no focus stealing during
//! sessions; the player's anchor, and showing it without activating it (see
//! `presence` for when each shows).

use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Monitor, Runtime, WebviewWindow};
use ts_rs::TS;

use crate::storage::write_atomic;

/// The window's default size at 100% zoom, in CSS pixels (the user can resize it).
pub const EXPANDED: (f64, f64) = (944.0, 612.0);
/// The smallest window: the editor at its narrowest, its inspector sliding
/// over the steps list (`layout.ts`).
pub const MIN_EXPANDED: (f64, f64) = (760.0, 520.0);
/// Space kept between the window and the edges of the work area (CSS px).
const MARGIN: f64 = 16.0;
/// Default distance from the bottom of the work area, as in the design.
const BOTTOM_GAP: f64 = 24.0;
/// The most of the work area the editor takes at its default size.
const DEFAULT_SHARE: f64 = 0.8;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowPrefs {
    /// The player stands in for the window.
    pub player_mode: bool,
    /// Where the user put the window's bottom-center, in physical
    /// virtual-desktop pixels. Placing the window may keep it off an edge,
    /// but only a drag changes this.
    pub anchor: Option<(i32, i32)>,
    /// The editor's size, in CSS px, if the user resized it.
    pub size: Option<(f64, f64)>,
    /// Where the user put the editor's dividers.
    pub panes: Panes,
    /// Where the user put the player's bottom-center (physical px).
    pub player_anchor: Option<(i32, i32)>,
}

/// Where the user put the editor's divider: the inspector's width in CSS px
/// (`None`: the default). The UI keeps it inside the window.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct Panes {
    pub inspector_w: Option<f64>,
}

impl Panes {
    /// Sane values only (the UI's own limits depend on the window): a value
    /// that isn't a size is dropped, a huge one is capped.
    pub fn sanitized(self) -> Panes {
        Panes { inspector_w: self.inspector_w.filter(|v| v.is_finite() && *v > 0.0).map(|v| v.min(10_000.0)) }
    }
}

/// A window's handle (0 if there's none).
fn hwnd_of<R: Runtime>(app: &AppHandle<R>, label: &str) -> isize {
    #[cfg(windows)]
    return app.get_webview_window(label).and_then(|w| w.hwnd().ok()).map_or(0, |h| h.0 as isize);
    #[cfg(not(windows))]
    return {
        let _ = (app, label);
        0
    };
}

/// Relay's windows, the main one and the player, to leave them out of
/// screen captures, image searches and recordings.
pub fn own_windows<R: Runtime>(app: &AppHandle<R>) -> relay_platform::OwnWindows {
    [hwnd_of(app, "main"), hwnd_of(app, "player")]
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
/// with the margins, and the `default` size to 80% of it. Never below
/// [`MIN_EXPANDED`], unless a big text size leaves no room for it.
pub fn fit_work(css: (f64, f64), work: Rect, sf: f64, zoom: f64, default: bool) -> (f64, f64) {
    let k = sf * zoom;
    let (w, h) = (work.w as f64 / k, work.h as f64 / k);
    let share = if default { DEFAULT_SHARE } else { 1.0 };
    let max_w = (w - 2.0 * MARGIN).min(w * share).floor();
    let max_h = (h - MARGIN).min(h * share).floor();
    (css.0.min(max_w).max(MIN_EXPANDED.0.min(max_w)), css.1.min(max_h).max(MIN_EXPANDED.1.min(max_h)))
}

/// Window placement state, saved to `window.json` (Relay-only; the UI never writes it).
pub struct WindowState {
    path: PathBuf,
    prefs: Mutex<WindowPrefs>,
    /// The window's current size in CSS px.
    size: Mutex<(f64, f64)>,
    /// The zoom the window was last placed at.
    zoom: Mutex<f64>,
    /// The text size's zoom (1.5 for 150%), on top of the screen's.
    text: Mutex<f64>,
    /// When the anchor last changed and hasn't been saved yet.
    dirty: Mutex<Option<Instant>>,
    /// The bottom-center [`place`] last put the window at, so the Moved
    /// events that raises aren't taken for a drag. Cleared by a drag.
    placed: Mutex<Option<(i32, i32)>>,
    /// The same for the player.
    player_placed: Mutex<Option<(i32, i32)>>,
}

impl WindowState {
    pub fn open(dir: &std::path::Path) -> Self {
        let path = dir.join("window.json");
        let prefs: WindowPrefs =
            std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let size = expanded_size(&prefs);
        WindowState {
            path,
            prefs: Mutex::new(prefs),
            size: Mutex::new(size),
            zoom: Mutex::new(1.0),
            text: Mutex::new(1.0),
            dirty: Mutex::new(None),
            placed: Mutex::new(None),
            player_placed: Mutex::new(None),
        }
    }

    pub fn prefs(&self) -> WindowPrefs {
        self.prefs.lock().clone()
    }

    /// Where a window of `css` size goes in `work` (see [`layout`]), noted as
    /// placed there. The user's anchor stays as it is.
    fn placement(&self, work: Rect, sf: f64, zoom: f64, css: (f64, f64), anchor: Option<(i32, i32)>) -> Rect {
        let r = layout(work, sf, zoom, css, anchor);
        *self.placed.lock() = Some(r.bottom_center());
        r
    }

    /// The window's bottom-center is now `at`. Arriving where it was placed
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

    /// The player was placed with its bottom-center `at`.
    pub fn player_placed(&self, at: (i32, i32)) {
        *self.player_placed.lock() = Some(at);
    }

    /// The player's bottom-center is now `at`: as for the window (see
    /// [`moved`](Self::moved)), anywhere but where it was placed is a drag
    /// and the new anchor. Returns whether it was a drag.
    pub fn player_moved(&self, at: (i32, i32)) -> bool {
        let mut placed = self.player_placed.lock();
        if *placed == Some(at) {
            return false;
        }
        *placed = None;
        let mut p = self.prefs.lock();
        if p.player_anchor != Some(at) {
            p.player_anchor = Some(at);
            *self.dirty.lock() = Some(Instant::now());
        }
        true
    }

    /// Player mode on or off, saved at once.
    pub fn set_player_mode(&self, on: bool) {
        self.prefs.lock().player_mode = on;
        self.save();
    }

    /// Saves where the user put the dividers.
    pub fn set_panes(&self, panes: Panes) {
        self.prefs.lock().panes = panes.sanitized();
        self.save();
    }

    fn save(&self) {
        let body = serde_json::to_string_pretty(&*self.prefs.lock()).expect("prefs serialize");
        let _ = write_atomic(&self.path, &body);
    }
}

/// The zoom on `monitor`: the one that fits the default window there (never
/// above 100%), times the text size's `text`.
pub fn zoom_for(monitor: &Monitor, text: f64) -> f64 {
    let work = monitor.work_area().size;
    zoom_for_work_area(work.width, work.height, monitor.scale_factor()) * text
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
    pub fn work_area(m: &Monitor) -> Rect {
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

    pub fn bottom_center(&self) -> (i32, i32) {
        (self.x + self.w / 2, self.y + self.h)
    }

    /// Whether this rect, grown by `margin` on every side, overlaps `r`.
    pub fn overlaps(&self, r: &Rect, margin: i32) -> bool {
        self.x - margin < r.x + r.w
            && r.x < self.x + self.w + margin
            && self.y - margin < r.y + r.h
            && r.y < self.y + self.h + margin
    }

    /// How far `(x, y)` is from this rect, squared (0 inside it).
    fn distance2(&self, (x, y): (i32, i32)) -> i64 {
        let gap = |v: i32, lo: i32, hi: i32| (lo - v).max(v - hi).max(0) as i64;
        gap(x, self.x, self.x + self.w - 1).pow(2) + gap(y, self.y + 1, self.y + self.h).pow(2)
    }
}

/// Which of the `monitors` (their full bounds) the window goes on: the one
/// its `anchor` is on (the taskbar included, as the window is then kept above
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

/// Where a window of `css` size goes in the `work` area at `sf` scale and
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

/// Sizes and positions the window for a window of `css` size, keeping its
/// bottom-center at the saved anchor (or the default spot on the primary
/// monitor), inside the work area of the monitor [`choose_monitor`] picks.
/// The saved anchor stays where the user put it, so switching sizes near an
/// edge comes back to the same spot.
pub fn place(window: &WebviewWindow, state: &WindowState, css: (f64, f64)) {
    let prefs = state.prefs();
    let anchor = prefs.anchor;
    let Some(monitor) = monitor_for(window, anchor) else { return };
    let zoom = zoom_for(&monitor, *state.text.lock());
    let (work, sf) = (Rect::work_area(&monitor), monitor.scale_factor());
    let css = fit_work(css, work, sf, zoom, prefs.size.is_none());
    let _ = window.set_zoom(zoom);
    *state.zoom.lock() = zoom;
    set_min_size(window, zoom * sf, work);
    let r = state.placement(work, sf, zoom, css, anchor);
    set_client_rect(window, r.x, r.y, r.w, r.h);
    *state.size.lock() = css;
}

/// The monitor a window anchored at `anchor` goes on (see [`choose_monitor`]).
pub fn monitor_for<R: Runtime>(window: &WebviewWindow<R>, anchor: Option<(i32, i32)>) -> Option<Monitor> {
    let monitors = window.available_monitors().ok().filter(|m| !m.is_empty())?;
    let bounds: Vec<Rect> = monitors.iter().map(Rect::bounds).collect();
    let primary = window
        .primary_monitor()
        .ok()
        .flatten()
        .and_then(|p| bounds.iter().position(|&b| b == Rect::bounds(&p)))
        .unwrap_or(0);
    let i = choose_monitor(&bounds, anchor, primary);
    monitors.into_iter().nth(i)
}

/// The window can be resized down to [`MIN_EXPANDED`] at `k` = zoom × scale,
/// or what the `work` area holds.
fn set_min_size(window: &WebviewWindow, k: f64, work: Rect) {
    let side = |min: f64, room: i32| (min * k).round().min(room as f64) as u32;
    let min = tauri::PhysicalSize::new(side(MIN_EXPANDED.0, work.w), side(MIN_EXPANDED.1, work.h));
    let _ = window.set_min_size(Some(tauri::Size::Physical(min)));
}

/// The text size changed: the window is zoomed and sized for it.
pub fn set_text_size(window: &WebviewWindow, state: &WindowState, text: crate::settings::TextSize) {
    let changed = std::mem::replace(&mut *state.text.lock(), text.zoom()) != text.zoom();
    if changed {
        place(window, state, expanded_size(&state.prefs()));
    }
}

/// Sets the text size's zoom before the window is first placed.
pub fn init_text_size(state: &WindowState, text: crate::settings::TextSize) {
    *state.text.lock() = text.zoom();
}

/// Tracks the editor's size while the user resizes it; saved shortly after it
/// stops. Sizes Relay sets itself, and the move loop's own after a scale
/// change, aren't the user's.
pub fn on_resized(window: &WebviewWindow, state: &WindowState) {
    if !RESIZING.load(Ordering::SeqCst) {
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
    let zoom = zoom_for(&monitor, *state.text.lock());
    let _ = window.set_zoom(zoom);
    *state.zoom.lock() = zoom;
    let dragging = in_move_loop(window);
    let (work, sf) = (Rect::work_area(&monitor), monitor.scale_factor());
    let prefs = state.prefs();
    set_min_size(window, zoom * sf, work);
    let css = *state.size.lock();
    let css = if dragging { css } else { fit_work(css, work, sf, zoom, prefs.size.is_none()) };
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

/// Once the window is dropped after a scale change, it stops holding its size,
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
pub fn in_move_loop(window: &WebviewWindow) -> bool {
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
pub fn in_move_loop(_window: &WebviewWindow) -> bool {
    false
}

/// Closing the window (its × or Alt+F4): puts Relay in the tray when *Close
/// to tray* is on. Returns whether it did; if not, the caller quits.
pub fn close_or_hide<R: Runtime>(app: &AppHandle<R>) -> bool {
    let to_tray = app.state::<Mutex<crate::settings::SettingsStore>>().lock().current.close_to_tray;
    if to_tray {
        crate::presence::set_tucked(app, true);
    }
    to_tray
}

/// Tracks the window's bottom-center while the user drags it; saved shortly after it stops.
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
pub fn set_client_rect<R: Runtime>(window: &WebviewWindow<R>, x: i32, y: i32, w: i32, h: i32) {
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
pub fn set_client_rect<R: Runtime>(window: &WebviewWindow<R>, x: i32, y: i32, w: i32, h: i32) {
    let _ = window.set_size(tauri::PhysicalSize::new(w as u32, h as u32));
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
}

/// The window's outer rect (its frame included).
pub fn outer_rect<R: Runtime>(window: &WebviewWindow<R>) -> Option<Rect> {
    let (p, s) = (window.outer_position().ok()?, window.outer_size().ok()?);
    Some(Rect { x: p.x, y: p.y, w: s.width as i32, h: s.height as i32 })
}

/// Shows the player without activating it: the keyboard stays with the app
/// being worked in.
#[cfg(windows)]
pub fn show_inactive<R: Runtime>(window: &WebviewWindow<R>) {
    use windows::Win32::UI::WindowsAndMessaging::{SW_SHOWNOACTIVATE, ShowWindow};
    if let Ok(hwnd) = window.hwnd() {
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
    }
}

/// Hides the player (shown by [`show_inactive`], so hidden the same way).
#[cfg(windows)]
pub fn hide_inactive<R: Runtime>(window: &WebviewWindow<R>) {
    use windows::Win32::UI::WindowsAndMessaging::{SW_HIDE, ShowWindow};
    if let Ok(hwnd) = window.hwnd() {
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
}

#[cfg(not(windows))]
pub fn show_inactive<R: Runtime>(window: &WebviewWindow<R>) {
    let _ = window.show();
}

#[cfg(not(windows))]
pub fn hide_inactive<R: Runtime>(window: &WebviewWindow<R>) {
    let _ = window.hide();
}

/// While a session runs, clicking the window must not take the keyboard away
/// from the app being recorded or played into. The player never takes it.
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

/// Rounds the window's corners on Windows 11, which also clips the frosted
/// glass to them, without the accent-colored border Windows may draw around
/// it. Windows 10 has no rounding: its corners stay square.
#[cfg(windows)]
pub fn apply_frame<R: Runtime>(window: &WebviewWindow<R>) {
    use std::ffi::c_void;
    use windows::Win32::Graphics::Dwm::{
        DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute,
    };

    let Ok(hwnd) = window.hwnd() else { return };
    // Both attributes exist on Windows 11 only; on Windows 10 the calls fail harmlessly.
    unsafe {
        let corner = DWMWCP_ROUND;
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
pub fn apply_frame<R: Runtime>(_window: &WebviewWindow<R>) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zoom_fits_small_screens_only() {
        // 1920x1080 at 100% (taskbar 48 px): the window fits as designed.
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
    /// A player-sized window.
    const SMALL: (f64, f64) = (604.0, 68.0);

    #[test]
    fn the_default_spot_is_bottom_center() {
        let r = layout(WORK, 1.0, 1.0, EXPANDED, None);
        assert_eq!(r, Rect { x: (1920 - 944) / 2, y: 1032 - 24 - 612, w: 944, h: 612 });
        // At 150%, sizes and the gap scale.
        let big = Rect { x: 0, y: 0, w: 2880, h: 1548 };
        let r = layout(big, 1.5, 1.0, SMALL, None);
        assert_eq!((r.w, r.h), (906, 102));
        assert_eq!(r.y + r.h, 1548 - 36);
    }

    #[test]
    fn the_anchor_is_the_bottom_center() {
        let r = layout(WORK, 1.0, 1.0, SMALL, Some((700, 500)));
        assert_eq!(r, Rect { x: 700 - 302, y: 500 - 68, w: 604, h: 68 });
        // Another size keeps the bottom-center where it was.
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
        assert_eq!(layout(big, 1.5, 1.0, SMALL, Some((1440, 5))).y, 24);
        // A second monitor to the left, at negative coordinates.
        let left = Rect { x: -1280, y: 0, w: 1280, h: 984 };
        let r = layout(left, 1.0, 1.0, SMALL, Some((-5, 500)));
        assert_eq!(r.x, -16 - 604);
    }

    #[test]
    fn a_widget_too_big_for_its_margins_is_centered() {
        let narrow = Rect { x: 100, y: 0, w: 960, h: 700 };
        let r = layout(narrow, 1.0, 1.0, EXPANDED, Some((150, 700)));
        assert_eq!(r.x, 100 + (960 - 944) / 2);
        // A short work area (the zoom leaves the default window 16 px shorter than it).
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
        // Back onto the 150% monitor, smaller.
        let primary = Rect { x: 0, y: 0, w: 2560, h: 1528 };
        let client = Rect { x: 1500, y: 700, w: 604, h: 68 };
        let r = rescaled(client, primary, 1.5, 1.0, SMALL, None, true);
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
        assert_eq!(layout(work, 1.0, 1.0, SMALL, Some((960, 1070))).y, 1032 - 16 - 68);
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
        // The small size fits here as placed; the big one is pushed left, off the right edge.
        state.moved((1500, 1000));
        // Placing, then the Moved event the placement raises.
        let switch = |css| {
            let r = state.placement(WORK, 1.0, 1.0, css, state.prefs().anchor);
            state.moved(r.bottom_center());
            r
        };
        let small = switch(SMALL);
        assert_eq!(small.bottom_center(), (1500, 1000));
        let big = switch(EXPANDED);
        assert_eq!(big.x, 1920 - 944 - 16);
        assert_eq!(state.prefs().anchor, Some((1500, 1000)), "clamping isn't a move");
        assert_eq!(switch(SMALL), small);
        assert_eq!(switch(EXPANDED), big);
    }

    #[test]
    fn a_drag_moves_the_anchor_even_back_to_where_it_was_placed() {
        let dir = tempfile::tempdir().unwrap();
        let state = WindowState::open(dir.path());
        let placed = state.placement(WORK, 1.0, 1.0, SMALL, None).bottom_center();
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
        assert!(!fresh.prefs().player_mode);
        assert_eq!(fresh.prefs().anchor, None);
        assert_eq!(*fresh.size.lock(), EXPANDED);

        let panes = Panes { inspector_w: Some(380.0) };
        *fresh.prefs.lock() = WindowPrefs {
            player_mode: true,
            anchor: Some((-300, 700)),
            size: Some((1200.0, 800.0)),
            panes,
            player_anchor: Some((900, 40)),
        };
        fresh.save();
        let again = WindowState::open(dir.path());
        assert!(again.prefs().player_mode);
        assert_eq!(again.prefs().anchor, Some((-300, 700)));
        assert_eq!(again.prefs().size, Some((1200.0, 800.0)));
        assert_eq!(again.prefs().panes, panes);
        assert_eq!(again.prefs().player_anchor, Some((900, 40)));
        assert_eq!(*again.size.lock(), (1200.0, 800.0), "opens at the size the user chose");
    }

    #[test]
    fn player_mode_is_saved_at_once() {
        let dir = tempfile::tempdir().unwrap();
        WindowState::open(dir.path()).set_player_mode(true);
        assert!(WindowState::open(dir.path()).prefs().player_mode);
    }

    #[test]
    fn dragging_the_player_moves_its_anchor_only() {
        let dir = tempfile::tempdir().unwrap();
        let state = WindowState::open(dir.path());
        state.player_placed((960, 1008));
        assert!(!state.player_moved((960, 1008)), "where it was placed");
        assert_eq!(state.prefs().player_anchor, None);
        assert!(state.player_moved((400, 300)));
        assert_eq!(state.prefs().player_anchor, Some((400, 300)));
        assert_eq!(state.prefs().anchor, None, "the window's stays");
        assert!(state.dirty.lock().is_some(), "saved once the drag pauses");
    }

    #[test]
    fn rects_overlap_with_a_margin() {
        let a = Rect { x: 0, y: 0, w: 100, h: 100 };
        assert!(a.overlaps(&Rect { x: 50, y: 50, w: 100, h: 100 }, 0));
        assert!(!a.overlaps(&Rect { x: 100, y: 0, w: 10, h: 10 }, 0), "touching isn't overlapping");
        assert!(a.overlaps(&Rect { x: 110, y: 0, w: 10, h: 10 }, 16));
        assert!(!a.overlaps(&Rect { x: 116, y: 0, w: 10, h: 10 }, 16));
        assert!(a.overlaps(&Rect { x: -20, y: -20, w: 10, h: 10 }, 16));
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
        assert_eq!(fit_work((1200.0, 800.0), WORK, 1.0, 1.0, false), (1200.0, 800.0));
        assert_eq!(fit_work((2400.0, 1400.0), WORK, 1.0, 1.0, false), (1920.0 - 32.0, 1032.0 - 16.0));
        // At 150%: 2880 × 1548 physical px hold 1920 × 1032 CSS px.
        let big = Rect { x: 0, y: 0, w: 2880, h: 1548 };
        assert_eq!(fit_work((2400.0, 1400.0), big, 1.5, 1.0, false), (1888.0, 1016.0));
        // A big text size on a small work area: as big as it holds, below the minimum.
        let small = Rect { x: 0, y: 0, w: 1366, h: 728 };
        assert_eq!(fit_work(EXPANDED, small, 1.0, 2.0, false), (683.0 - 32.0, 364.0 - 16.0));
    }

    #[test]
    fn the_default_size_takes_at_most_80_percent_of_the_work_area() {
        assert_eq!(fit_work(EXPANDED, WORK, 1.0, 1.0, true), EXPANDED);
        let laptop = Rect { x: 0, y: 0, w: 1366, h: 728 };
        assert_eq!(fit_work(EXPANDED, laptop, 1.0, 1.0, true), (944.0, 582.0));
        // A size the user chose may take more.
        assert_eq!(fit_work(EXPANDED, laptop, 1.0, 1.0, false), EXPANDED);
        // At 150% text the window grows, as far as 80% of the screen.
        assert_eq!(fit_work(EXPANDED, WORK, 1.0, 1.5, true), (944.0, 550.0));
    }

    #[test]
    fn a_resize_moves_the_anchor_and_is_saved() {
        let dir = tempfile::tempdir().unwrap();
        let state = WindowState::open(dir.path());
        state.resized((1200.0, 800.0), (900, 1000));
        assert_eq!(*state.size.lock(), (1200.0, 800.0));
        assert_eq!(state.prefs().size, Some((1200.0, 800.0)));
        assert_eq!(state.prefs().anchor, Some((900, 1000)));
        assert!(state.dirty.lock().is_some(), "saved once the resize pauses");
        state.set_panes(Panes { inspector_w: Some(420.0) });
        let saved = WindowState::open(dir.path()).prefs();
        assert_eq!(saved.panes, Panes { inspector_w: Some(420.0) }, "saved at once");
    }

    #[test]
    fn panes_that_arent_sizes_are_dropped() {
        for v in [f64::NAN, 0.0, -5.0] {
            assert_eq!(Panes { inspector_w: Some(v) }.sanitized(), Panes::default());
        }
        assert_eq!(Panes { inspector_w: Some(1e9) }.sanitized(), Panes { inspector_w: Some(10_000.0) });
        assert_eq!(Panes { inspector_w: Some(360.0) }.sanitized(), Panes { inspector_w: Some(360.0) });
    }

    #[test]
    fn broken_or_partial_prefs_fall_back_to_defaults() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("window.json"), "{ not json").unwrap();
        assert!(!WindowState::open(dir.path()).prefs().player_mode);
        // A file from before resizing: no size, no panes. An older Relay's compact player isn't player mode.
        std::fs::write(dir.path().join("window.json"), r#"{"anchor":[5,6],"expanded":false}"#).unwrap();
        let p = WindowState::open(dir.path()).prefs();
        assert!(!p.player_mode);
        assert_eq!(p.anchor, Some((5, 6)));
        assert_eq!((p.size, p.panes), (None, Panes::default()));
        std::fs::write(dir.path().join("window.json"), r#"{"panes":{"inspector_w":380}}"#).unwrap();
        assert_eq!(WindowState::open(dir.path()).prefs().panes, Panes { inspector_w: Some(380.0) });
        // An older Relay's dividers are gone.
        std::fs::write(dir.path().join("window.json"), r#"{"panes":{"preview_w":480}}"#).unwrap();
        assert_eq!(WindowState::open(dir.path()).prefs().panes, Panes::default());
    }
}
