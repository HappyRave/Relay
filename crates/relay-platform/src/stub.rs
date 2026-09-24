//! Placeholder backend for non-Windows targets: the workspace builds and the
//! OS-independent parts test everywhere, but capture is unsupported.

use std::sync::Arc;
use std::time::Instant;

use crossbeam_channel::Sender;
use relay_core::model::{MonitorInfo, Rect, Rgb, WindowInfo};

use relay_core::keys::KeyStroke;
use relay_core::model::MouseBtn;

use crate::{
    CharTranslator, HeldKeys, HookConfig, HookSession, InputHook, Injector, Platform, PlatformError, RawInput, Result,
    Screen, Timer, WindowQuery, WindowRef,
};

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
    fn pixel(&self, _: i32, _: i32) -> Option<Rgb> {
        None
    }
}

impl WindowQuery for Stub {
    fn root_window_at(&self, _: i32, _: i32) -> Option<WindowInfo> {
        None
    }
    fn foreground(&self) -> Option<WindowRef> {
        None
    }
    fn restore_previous(&self, _: isize) -> Option<WindowRef> {
        None
    }
    fn find_window(&self, _: &str, _: &str) -> Option<WindowInfo> {
        None
    }
    fn is_elevated(&self, _: u32) -> bool {
        false
    }
    fn self_elevated(&self) -> bool {
        false
    }
    fn input_desktop_available(&self) -> bool {
        true
    }
}

impl Injector for Stub {
    fn move_to(&mut self, _: i32, _: i32) -> Result<()> {
        Err(PlatformError::Unsupported)
    }
    fn button(&mut self, _: MouseBtn, _: bool) -> Result<()> {
        Err(PlatformError::Unsupported)
    }
    fn wheel(&mut self, _: i32, _: bool) -> Result<()> {
        Err(PlatformError::Unsupported)
    }
    fn key(&mut self, _: &KeyStroke, _: bool, _: Option<&str>) -> Result<()> {
        Err(PlatformError::Unsupported)
    }
}

/// A plain sleeping timer.
struct SleepTimer(Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>);

impl Timer for SleepTimer {
    fn now_ms(&self) -> f64 {
        now_ms()
    }
    fn wait_until(&mut self, deadline: f64) -> bool {
        let (lock, cv) = &*self.0;
        let mut woken = lock.lock().unwrap();
        loop {
            if *woken {
                *woken = false;
                return true;
            }
            let left = deadline - now_ms();
            if left <= 0.0 {
                return false;
            }
            woken = cv.wait_timeout(woken, std::time::Duration::from_secs_f64(left / 1000.0)).unwrap().0;
        }
    }
    fn waker(&self) -> Arc<dyn Fn() + Send + Sync> {
        let pair = self.0.clone();
        Arc::new(move || {
            *pair.0.lock().unwrap() = true;
            pair.1.notify_all();
        })
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
        injector: || Box::new(Stub),
        timer: || Box::new(SleepTimer(Arc::default())),
        now_ms,
    }
}
