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

/// A picture of part of the screen: `w` × `h` pixels, rows top to bottom,
/// 3 bytes (R, G, B) per pixel.
#[derive(Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub w: u32,
    pub h: u32,
    pub rgb: Vec<u8>,
}

impl std::fmt::Debug for Snapshot {
    // The pixels are someone's screen: never in logs or panic messages.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Snapshot({}×{})", self.w, self.h)
    }
}

/// The size of a snapshot of an area `w` × `h` physical pixels wide, at most
/// `max_w` wide: scaled down evenly, never up, and at least 1 × 1.
pub fn snapshot_size(w: i32, h: i32, max_w: u32) -> (u32, u32) {
    let (w, h) = (w.max(1) as f64, h.max(1) as f64);
    let scale = (max_w as f64 / w).min(1.0);
    (((w * scale).round() as u32).max(1), ((h * scale).round() as u32).max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_scale_down_to_fit_but_never_up() {
        assert_eq!(snapshot_size(1920, 1080, 3200), (1920, 1080));
        // Three monitors, 2560 + 1920 + 1920 wide: half size.
        assert_eq!(snapshot_size(6400, 1600, 3200), (3200, 800));
        assert_eq!(snapshot_size(3840, 2160, 3200), (3200, 1800));
        assert_eq!(snapshot_size(0, -5, 3200), (1, 1));
    }

    #[test]
    fn a_snapshot_never_shows_its_pixels_in_debug_output() {
        let s = Snapshot { w: 2, h: 1, rgb: vec![1, 2, 3, 4, 5, 6] };
        assert_eq!(format!("{s:?}"), "Snapshot(2×1)");
    }
}
