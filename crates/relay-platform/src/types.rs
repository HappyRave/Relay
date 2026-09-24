use std::collections::BTreeSet;

use relay_core::model::{MouseBtn, Rect};

/// One low-level input event, as captured by the hook.
#[derive(Debug, Clone, PartialEq)]
pub struct RawInput {
    /// Monotonic milliseconds (see `Platform::now_ms`).
    pub time: f64,
    pub kind: RawKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RawKind {
    Move { x: i32, y: i32 },
    Button { x: i32, y: i32, btn: MouseBtn, down: bool },
    Wheel { x: i32, y: i32, delta: i32, horizontal: bool },
    Key { vk: u16, scan: u16, ext: bool, down: bool },
    /// Esc was pressed during a session (and swallowed).
    Escape,
    /// Another key was pressed during playback with "stop on key press" (swallowed).
    StopKey,
}

/// What the hook filters. Swapped atomically while the hook runs.
#[derive(Debug, Clone, Default)]
pub struct HookConfig {
    /// Relay's own window: clicks and wheel events inside it are not recorded.
    pub own_rect: Option<Rect>,
    /// Relay's window handle: keys typed while it is focused are not recorded.
    pub own_window: isize,
    /// Swallow Esc and report [`RawKind::Escape`] instead.
    pub swallow_escape: bool,
    /// Ignore input injected by other programs (and always Relay's own).
    pub ignore_injected: bool,
    /// Virtual keys passed through but not recorded (the control hotkeys).
    pub drop_vks: Vec<u16>,
    /// Report input for recording. Off during playback, where the hook only
    /// watches for Esc and, with `stop_on_key`, any other key.
    pub record: bool,
    /// During playback: stop on any physical key press (modifiers excepted,
    /// so hotkeys like Ctrl + Alt + End still reach their handler).
    pub stop_on_key: bool,
}

/// Virtual keys currently held, for character translation.
pub type HeldKeys = BTreeSet<u16>;
