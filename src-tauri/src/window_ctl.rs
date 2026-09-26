//! The floating widget's window: placement anchored at its bottom-center,
//! remembered across runs, zoomed down on small screens, square corners,
//! and no focus stealing during sessions.

use std::path::PathBuf;
use parking_lot::Mutex;
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
    /// The widget's bottom-center, in physical virtual-desktop pixels.
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
}

impl WindowState {
    pub fn open(dir: &std::path::Path) -> Self {
        let path = dir.join("window.json");
        let prefs: WindowPrefs =
            std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let size = if prefs.expanded { EXPANDED } else { COMPACT };
        WindowState { path, prefs: Mutex::new(prefs), size: Mutex::new(size), dirty: Mutex::new(None) }
    }

    pub fn prefs(&self) -> WindowPrefs {
        self.prefs.lock().clone()
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

fn contains(m: &Monitor, (x, y): (i32, i32)) -> bool {
    let r = m.work_area();
    x >= r.position.x && y > r.position.y && x < r.position.x + r.size.width as i32 && y <= r.position.y + r.size.height as i32
}

/// Sizes and positions the window for a widget of `css` size, keeping its
/// bottom-center at the saved anchor (or the default spot on the primary
/// monitor when the anchor is gone, e.g. after unplugging a monitor), inside
/// the monitor's work area. Returns the anchor actually used.
pub fn place(window: &WebviewWindow, state: &WindowState, css: (f64, f64)) -> Option<(i32, i32)> {
    let saved = state.prefs().anchor;
    let monitors = window.available_monitors().ok()?;
    let monitor = saved
        .and_then(|a| monitors.iter().find(|m| contains(m, a)).cloned())
        .or_else(|| window.primary_monitor().ok().flatten())
        .or_else(|| monitors.first().cloned())?;
    let anchor = saved.filter(|&a| contains(&monitor, a));

    let sf = monitor.scale_factor();
    let zoom = zoom_for(&monitor);
    let _ = window.set_zoom(zoom);
    let w = (css.0 * zoom * sf).round() as i32;
    let h = (css.1 * zoom * sf).round() as i32;

    let work = monitor.work_area();
    let (wx, wy) = (work.position.x, work.position.y);
    let (ww, wh) = (work.size.width as i32, work.size.height as i32);
    let (ax, ay) = anchor.unwrap_or((wx + ww / 2, wy + wh - (BOTTOM_GAP * sf) as i32));
    let margin = (MARGIN * sf) as i32;
    // Keep it inside the work area; center it if it can't fit with margins.
    let fit = |v: i32, lo: i32, hi: i32, center: i32| if hi < lo { center } else { v.clamp(lo, hi) };
    let x = fit(ax - w / 2, wx + margin, wx + ww - w - margin, wx + (ww - w) / 2);
    let y = fit(ay - h, wy, wy + wh - h, wy);

    set_client_rect(window, x, y, w, h);
    let used = (x + w / 2, y + h);
    state.prefs.lock().anchor = Some(used);
    *state.size.lock() = css;
    Some(used)
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
    let anchor = (pos.x + size.width as i32 / 2, pos.y + size.height as i32);
    let mut p = state.prefs.lock();
    if p.anchor != Some(anchor) {
        p.anchor = Some(anchor);
        *state.dirty.lock() = Some(Instant::now());
    }
}

/// Saves a moved position once dragging has paused for half a second.
pub fn spawn_autosave(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(250));
        let state = app.state::<WindowState>();
        let due = state.dirty.lock().is_some_and(|t| t.elapsed() >= Duration::from_millis(500));
        if due {
            *state.dirty.lock() = None;
            state.save();
        }
    });
}

/// Moves and sizes the window so its *client* area has this rect, in one call.
#[cfg(windows)]
fn set_client_rect(window: &WebviewWindow, x: i32, y: i32, w: i32, h: i32) {
    use windows::Win32::UI::WindowsAndMessaging::{SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos};
    // The outer frame includes invisible resize borders around the client area.
    let (Ok(op), Ok(os), Ok(ip), Ok(is)) = (window.outer_position(), window.outer_size(), window.inner_position(), window.inner_size())
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
    use windows::Win32::UI::WindowsAndMessaging::{GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_NOACTIVATE};
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
        DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND,
        DwmSetWindowAttribute,
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
    }
}
