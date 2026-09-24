//! Placeholder backend for non-Windows targets: the workspace builds and the
//! OS-independent parts test everywhere, but capture is unsupported.

use std::sync::Arc;
use std::time::Instant;

use crossbeam_channel::Sender;
use relay_core::model::{MonitorInfo, Rect, WindowInfo};

use crate::{CharTranslator, HeldKeys, HookConfig, HookSession, InputHook, Platform, PlatformError, RawInput, Result, Screen, WindowQuery};

struct Stub;

impl InputHook for Stub {
    fn start(&self, _: HookConfig, _: Sender<RawInput>) -> Result<Box<dyn HookSession>> {
        Err(PlatformError::Unsupported)
    }
}

impl Screen for Stub {
    fn monitors(&self) -> Vec<MonitorInfo> {
        Vec::new()
    }
    fn virtual_desktop(&self) -> Rect {
        Rect { x: 0, y: 0, w: 1920, h: 1080 }
    }
    fn cursor_pos(&self) -> (i32, i32) {
        (0, 0)
    }
    fn double_click(&self) -> (u32, u32) {
        (500, 4)
    }
}

impl WindowQuery for Stub {
    fn root_window_at(&self, _: i32, _: i32) -> Option<WindowInfo> {
        None
    }
}

impl CharTranslator for Stub {
    fn translate(&mut self, _: u16, _: u16, _: &HeldKeys) -> Option<String> {
        None
    }
}

fn now_ms() -> f64 {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f64() * 1000.0
}

pub fn platform() -> Platform {
    Platform {
        hook: Box::new(Stub),
        screen: Arc::new(Stub),
        windows: Arc::new(Stub),
        translator: || Box::new(Stub),
        now_ms,
    }
}
