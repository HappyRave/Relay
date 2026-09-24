//! Relay platform layer: traits for input capture, screen and window queries,
//! with a Windows backend and a stub for other targets. The recorder and the
//! key map are OS-independent so they can be tested anywhere.

pub mod keymap;
pub mod processes;
pub mod recorder;
pub mod types;

#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
mod stub;

use std::sync::Arc;

use crossbeam_channel::Sender;
use relay_core::keys::KeyStroke;
use relay_core::model::{MonitorInfo, MouseBtn, Rect, Rgb, WindowInfo};

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
    /// The color of one screen pixel, or `None` if it can't be read.
    fn pixel(&self, x: i32, y: i32) -> Option<Rgb>;
}

/// A top-level window, by handle and owning process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowRef {
    pub hwnd: isize,
    pub pid: u32,
}

pub trait WindowQuery: Send + Sync {
    /// The top-level window under a screen point.
    fn root_window_at(&self, x: i32, y: i32) -> Option<WindowInfo>;
    fn foreground(&self) -> Option<WindowRef>;
    /// Gives the keyboard back to the app the user was in before Relay: the
    /// last foreground window of another process, or failing that the first
    /// real window below `own` in z-order. Returns the window activated.
    fn restore_previous(&self, own: isize) -> Option<WindowRef>;
    /// The topmost visible window of `exe` with window class `class`.
    fn find_window(&self, exe: &str, class: &str) -> Option<WindowInfo>;
    /// Whether a process runs elevated (as administrator).
    fn is_elevated(&self, pid: u32) -> bool;
    fn self_elevated(&self) -> bool;
    /// False on the lock screen or a UAC prompt, where no input can be sent.
    fn input_desktop_available(&self) -> bool;
}

/// Synthesizes input. Every event carries [`RELAY_MAGIC`] so Relay's own hook
/// can tell its playback apart from the user.
pub trait Injector: Send {
    /// Moves the cursor to a virtual-desktop pixel.
    fn move_to(&mut self, x: i32, y: i32) -> Result<()>;
    fn button(&mut self, btn: MouseBtn, down: bool) -> Result<()>;
    fn wheel(&mut self, delta: i32, horizontal: bool) -> Result<()>;
    /// Presses or releases a key: by scan code when known, else by virtual
    /// key, else types `ch` as Unicode.
    fn key(&mut self, key: &KeyStroke, down: bool, ch: Option<&str>) -> Result<()>;
}

/// Sleeps with sub-millisecond precision and can be woken early.
pub trait Timer: Send {
    fn now_ms(&self) -> f64;
    /// Waits until `deadline` (in [`Timer::now_ms`] time). Returns true when
    /// woken early by the waker.
    fn wait_until(&mut self, deadline: f64) -> bool;
    /// Wakes a pending or the next [`Timer::wait_until`].
    fn waker(&self) -> Arc<dyn Fn() + Send + Sync>;
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
    pub injector: fn() -> Box<dyn Injector>,
    /// A timer for the playback thread; also raises that thread's priority and
    /// opts the process out of timer throttling.
    pub timer: fn() -> Box<dyn Timer>,
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
