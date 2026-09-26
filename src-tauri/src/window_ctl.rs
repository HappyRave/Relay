//! The floating widget's window: placement anchored at its bottom-center,
//! remembered across runs, zoomed down on small screens, square corners,
//! and no focus stealing during sessions.

use parking_lot::Mutex;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Monitor, WebviewWindow};

use crate::storage::write_atomic;

/// The widget's size at 100% zoom, in CSS pixels (expanded and compact).
pub const EXPANDED: (f64, f64) = (944.0, 612.0);
pub const COMPACT: (f64, f64) = (604.0, 68.0);
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
}

impl Default for WindowPrefs {
    fn default() -> Self {
        WindowPrefs { expanded: true, anchor: None }
    }
}

/// Window placement state, saved to `window.json` (Relay-only; the UI never writes it).
pub struct WindowState {
    path: PathBuf,
    prefs: Mutex<WindowPrefs>,
    /// The widget's current size in CSS px, as last measured by the UI.
    size: Mutex<(f64, f64)>,
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
        let size = if prefs.expanded { EXPANDED } else { COMPACT };
        WindowState {
            path,
            prefs: Mutex::new(prefs),
            size: Mutex::new(size),
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
    let y = fit(ay - h, work.y, work.y + work.h - h, work.y);
    Rect { x, y, w, h }
}

/// Sizes and positions the window for a widget of `css` size, keeping its
/// bottom-center at the saved anchor (or the default spot on the primary
/// monitor), inside the work area of the monitor [`choose_monitor`] picks.
/// The saved anchor stays where the user put it, so switching sizes near an
/// edge comes back to the same spot.
pub fn place(window: &WebviewWindow, state: &WindowState, css: (f64, f64)) {
    let anchor = state.prefs().anchor;
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
    let _ = window.set_zoom(zoom);
    let r = state.placement(Rect::work_area(monitor), monitor.scale_factor(), zoom, css, anchor);
    set_client_rect(window, r.x, r.y, r.w, r.h);
    *state.size.lock() = css;
}

/// Called by the UI when the widget's size changes (switching compact/expanded).
pub fn fit(window: &WebviewWindow, state: &WindowState, css: (f64, f64), expanded: bool) {
    let changed = {
        let mut p = state.prefs.lock();
        let changed = p.expanded != expanded;
        p.expanded = expanded;
        changed
    };
    if *state.size.lock() != css || changed {
        place(window, state, css);
        state.save();
    }
}

/// Re-places the window at its current size (monitor or scaling changes).
pub fn replace(window: &WebviewWindow, state: &WindowState) {
    let css = *state.size.lock();
    place(window, state, css);
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
pub fn apply_on_top(window: &WebviewWindow, setting: crate::settings::KeepOnTop, session: bool) {
    let _ = window.set_always_on_top(setting.on_top(session));
}

/// While a session runs, clicking the widget must not take the keyboard away
/// from the app being recorded or played into.
#[cfg(windows)]
pub fn set_no_activate(window: &WebviewWindow, on: bool) {
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
pub fn set_no_activate(_window: &WebviewWindow, _on: bool) {}

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
        assert_eq!(layout(WORK, 1.0, 1.0, EXPANDED, Some((960, 100))).y, 0);
        // Below the bottom (a taller taskbar now): pulled up.
        assert_eq!(layout(WORK, 1.0, 1.0, EXPANDED, Some((960, 2000))).y, 1032 - 612);
        // A second monitor to the left, at negative coordinates.
        let left = Rect { x: -1280, y: 0, w: 1280, h: 984 };
        let r = layout(left, 1.0, 1.0, COMPACT, Some((-5, 500)));
        assert_eq!(r.x, -16 - 604);
    }

    #[test]
    fn a_widget_too_wide_for_its_margins_is_centered() {
        let narrow = Rect { x: 100, y: 0, w: 960, h: 700 };
        let r = layout(narrow, 1.0, 1.0, EXPANDED, Some((150, 700)));
        assert_eq!(r.x, 100 + (960 - 944) / 2);
    }

    #[test]
    fn zoom_shrinks_the_window() {
        let r = layout(WORK, 1.25, 0.8, EXPANDED, None);
        assert_eq!((r.w, r.h), (944, 612));
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
        let r = layout(work, 1.0, 1.0, COMPACT, Some((960, 1070)));
        assert!(r.y + r.h <= 1032, "{r:?}");
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

        *fresh.prefs.lock() = WindowPrefs { expanded: false, anchor: Some((-300, 700)) };
        fresh.save();
        let again = WindowState::open(dir.path());
        assert!(!again.prefs().expanded);
        assert_eq!(again.prefs().anchor, Some((-300, 700)));
        assert_eq!(*again.size.lock(), COMPACT, "opens at the size it was left at");
    }

    #[test]
    fn broken_or_partial_prefs_fall_back_to_defaults() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("window.json"), "{ not json").unwrap();
        assert!(WindowState::open(dir.path()).prefs().expanded);
        std::fs::write(dir.path().join("window.json"), r#"{"anchor":[5,6]}"#).unwrap();
        let p = WindowState::open(dir.path()).prefs();
        assert!(p.expanded);
        assert_eq!(p.anchor, Some((5, 6)));
    }
}
