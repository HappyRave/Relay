//! Native window tweaks for the floating widget.

use tauri::WebviewWindow;

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
