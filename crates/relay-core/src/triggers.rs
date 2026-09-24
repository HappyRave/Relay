//! What can start a macro on its own (a hotkey, a weekly schedule, an app
//! launching, a pixel changing), plus the edge detectors the polling triggers
//! use: they're fed one sample per poll and report when to fire.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::Rgb;
use crate::schedule::WeeklySchedule;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct HotkeyTrigger {
    pub enabled: bool,
    /// e.g. "Ctrl + Alt + 1"; empty when none is set.
    pub combo: String,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct ScheduleTrigger {
    pub enabled: bool,
    pub schedule: WeeklySchedule,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct AppLaunchTrigger {
    pub enabled: bool,
    /// Executable file name, matched case-insensitively ("EXCEL.EXE").
    pub exe: String,
    /// Wait after the app starts, so its window is ready.
    pub delay_ms: u32,
}

impl Default for AppLaunchTrigger {
    fn default() -> Self {
        AppLaunchTrigger { enabled: false, exe: String::new(), delay_ms: 2000 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct PixelTrigger {
    pub enabled: bool,
    pub x: i32,
    pub y: i32,
    pub color: Rgb,
    pub tolerance: u8,
}

impl Default for PixelTrigger {
    fn default() -> Self {
        PixelTrigger { enabled: false, x: 0, y: 0, color: Rgb(0xEC, 0x30, 0x13), tolerance: 8 }
    }
}

/// All of a macro's triggers. Machine-local: kept in library.json, not in the `.rly`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct MacroTriggers {
    pub hotkey: HotkeyTrigger,
    pub schedule: ScheduleTrigger,
    pub app_launch: AppLaunchTrigger,
    pub pixel: PixelTrigger,
}

/// Fires when a pixel starts matching the target color: two consecutive
/// matching samples after at least one non-matching one. Re-arms only once the
/// pixel stops matching again, so a pixel that stays red fires once.
#[derive(Debug, Default, Clone)]
pub struct PixelEdge {
    armed: bool,
    streak: u8,
}

impl PixelEdge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, matches: bool) -> bool {
        if !matches {
            self.armed = true;
            self.streak = 0;
            return false;
        }
        self.streak = self.streak.saturating_add(1);
        if self.armed && self.streak >= 2 {
            self.armed = false;
            return true;
        }
        false
    }
}

/// Fires when a process appears. The first sample is the baseline, so an app
/// that is already running when Relay starts does not fire.
#[derive(Debug, Default, Clone)]
pub struct ProcessLaunchEdge {
    present: Option<bool>,
}

impl ProcessLaunchEdge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, present: bool) -> bool {
        let fired = self.present == Some(false) && present;
        self.present = Some(present);
        fired
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixel_fires_once_per_change() {
        let mut e = PixelEdge::new();
        let fired: Vec<bool> = [true, true, false, true, true, true, false, true, false, true, true]
            .into_iter()
            .map(|m| e.update(m))
            .collect();
        // Matching from the start doesn't fire; a single matching sample doesn't either.
        assert_eq!(fired, [false, false, false, false, true, false, false, false, false, false, true]);
    }

    #[test]
    fn triggers_default_to_off_and_tolerate_partial_json() {
        let t: MacroTriggers = serde_json::from_str(r#"{"hotkey":{"enabled":true,"combo":"Ctrl + Alt + 1"}}"#).unwrap();
        assert!(t.hotkey.enabled && !t.schedule.enabled && !t.app_launch.enabled && !t.pixel.enabled);
        assert_eq!(t.app_launch.delay_ms, 2000);
        assert_eq!(t.schedule.schedule.days, [true, true, true, true, true, false, false]);
    }

    #[test]
    fn process_fires_on_launch_not_on_baseline() {
        let mut e = ProcessLaunchEdge::new();
        assert!(!e.update(true));
        assert!(!e.update(false));
        assert!(e.update(true));
        assert!(!e.update(true));
        assert!(!e.update(false));
        assert!(e.update(true));
    }
}
