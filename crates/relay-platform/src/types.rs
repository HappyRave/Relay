use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use relay_core::model::MouseBtn;

/// One low-level input event, as captured by the hook.
#[derive(Debug, Clone, PartialEq)]
pub struct RawInput {
    /// Monotonic milliseconds (see `Platform::now_ms`).
    pub time: f64,
    pub kind: RawKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RawKind {
    Move {
        x: i32,
        y: i32,
    },
    Button {
        x: i32,
        y: i32,
        btn: MouseBtn,
        down: bool,
    },
    Wheel {
        x: i32,
        y: i32,
        delta: i32,
        horizontal: bool,
    },
    Key {
        vk: u16,
        scan: u16,
        ext: bool,
        down: bool,
    },
    /// Esc was pressed during a session (and swallowed).
    Escape,
    /// Another key was pressed during playback with "stop on key press" (swallowed).
    StopKey,
}

/// What a hook session does, fixed for its lifetime.
#[derive(Debug, Clone)]
pub struct HookConfig {
    pub mode: HookMode,
    /// Ignore input injected by other programs (Relay's own is always ignored).
    pub ignore_injected: bool,
    /// Beats on every mouse event the hook sees while recording, including
    /// those it filters out, so a watchdog can tell a live hook from one
    /// Windows removed (a filtered event reports nothing otherwise).
    pub mouse_pulse: Option<Arc<MousePulse>>,
}

/// The time (see `Platform::now_ms`) of the latest mouse event a hook saw.
/// An atomic: the hook callback must not allocate or block.
#[derive(Debug, Default)]
pub struct MousePulse(AtomicU64);

impl MousePulse {
    pub fn beat(&self, time: f64) {
        self.0.store(time.to_bits(), Ordering::Relaxed);
    }

    /// The latest beat; 0 before any.
    pub fn last(&self) -> f64 {
        f64::from_bits(self.0.load(Ordering::Relaxed))
    }
}

#[derive(Debug, Clone)]
pub enum HookMode {
    /// Report input for a recording.
    Record {
        /// Relay's window: clicks and scrolls on it, and keys typed while it
        /// is in front, are UI, not macro input (0 when there is none).
        own_window: isize,
        /// Virtual keys passed through but not recorded (the control hotkeys).
        skip_vks: Vec<u16>,
        /// Swallow Esc and report [`RawKind::Escape`] instead of recording it.
        esc_stops: bool,
    },
    /// Playback: report Esc (swallowed) and, with `stop_on_key`, any other key
    /// but modifiers and `pass_vks` (swallowed too). Nothing is recorded.
    Watch { stop_on_key: bool, pass_vks: Vec<u16> },
}

/// Virtual keys currently held, for character translation.
pub type HeldKeys = BTreeSet<u16>;
