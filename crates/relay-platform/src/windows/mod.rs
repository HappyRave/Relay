//! The Windows backend. Relay is per-monitor DPI aware (v2), so every
//! coordinate here is a physical pixel on the virtual desktop.

mod hook;
mod screen;
mod text;
mod window;

use std::sync::{Arc, OnceLock};

use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

use crate::Platform;

/// Milliseconds from QueryPerformanceCounter: sub-microsecond resolution,
/// unlike the ~15.6 ms `time` field in hook structs.
pub fn now_ms() -> f64 {
    static FREQ: OnceLock<f64> = OnceLock::new();
    let freq = *FREQ.get_or_init(|| {
        let mut f = 0i64;
        unsafe {
            let _ = QueryPerformanceFrequency(&mut f);
        }
        f.max(1) as f64
    });
    let mut c = 0i64;
    unsafe {
        let _ = QueryPerformanceCounter(&mut c);
    }
    c as f64 * 1000.0 / freq
}

pub fn platform() -> Platform {
    Platform {
        hook: Box::new(hook::LowLevelHook),
        screen: Arc::new(screen::WinScreen),
        windows: Arc::new(window::WinWindows),
        translator: || Box::new(text::ToUnicodeTranslator),
        now_ms,
    }
}
