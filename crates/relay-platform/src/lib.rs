//! Relay platform layer: traits for input capture, screen and window queries,
//! with a Windows backend and a stub for other targets. The recorder and the
//! key map are OS-independent so they can be tested anywhere.

pub mod keymap;
pub mod recorder;
pub mod types;

#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
mod stub;

use std::sync::Arc;

use crossbeam_channel::Sender;
use relay_core::model::{MonitorInfo, Rect, WindowInfo};

pub use types::{HeldKeys, HookConfig, RawInput, RawKind};

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("not supported on this platform")]
    Unsupported,
    #[error("{0}")]
    Os(String),
}

pub type Result<T> = std::result::Result<T, PlatformError>;

/// Tags every event Relay injects, so its own hook can ignore them.
pub const RELAY_MAGIC: usize = 0x5245_4C59;

/// Global low-level input capture.
pub trait InputHook: Send + Sync {
    /// Installs the hooks on a dedicated thread; raw input goes to `tx`.
    fn start(&self, cfg: HookConfig, tx: Sender<RawInput>) -> Result<Box<dyn HookSession>>;
}

/// A running hook; dropping it without `stop` leaks the thread until exit.
pub trait HookSession: Send {
    fn update(&self, cfg: HookConfig);
    fn stop(self: Box<Self>);
}

pub trait Screen: Send + Sync {
    fn monitors(&self) -> Vec<MonitorInfo>;
    fn virtual_desktop(&self) -> Rect;
    fn cursor_pos(&self) -> (i32, i32);
    /// System double-click time (ms) and distance (px).
    fn double_click(&self) -> (u32, u32);
}

pub trait WindowQuery: Send + Sync {
    /// The top-level window under a screen point.
    fn root_window_at(&self, x: i32, y: i32) -> Option<WindowInfo>;
}

/// Turns a key press into the character it types, for display.
pub trait CharTranslator: Send {
    fn translate(&mut self, vk: u16, scan: u16, held: &HeldKeys) -> Option<String>;
}

pub struct Platform {
    pub hook: Box<dyn InputHook>,
    pub screen: Arc<dyn Screen>,
    pub windows: Arc<dyn WindowQuery>,
    pub translator: fn() -> Box<dyn CharTranslator>,
    /// Monotonic milliseconds, the time base of [`RawInput::time`].
    pub now_ms: fn() -> f64,
}

/// The backend for the current OS.
pub fn platform() -> Platform {
    #[cfg(windows)]
    return windows::platform();
    #[cfg(not(windows))]
    return stub::platform();
}
