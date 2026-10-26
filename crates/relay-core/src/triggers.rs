//! What can start a macro on its own (a hotkey, a weekly schedule, an app
//! launching, a pixel changing, an image appearing), plus the edge detectors
//! the polling triggers use: they're fed one sample per poll and report when
//! to fire.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::{ImagePng, Rect, Rgb};
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct ImageTrigger {
    pub enabled: bool,
    /// What to look for; none until one is set.
    pub image: Option<ImagePng>,
    /// The lowest match accepted, in percent.
    pub threshold: u8,
    /// Where to look; everywhere if `None`.
    pub area: Option<Rect>,
}

impl Default for ImageTrigger {
    fn default() -> Self {
        ImageTrigger { enabled: false, image: None, threshold: 85, area: None }
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
    pub image: ImageTrigger,
}

/// A trigger that's on, as the Library says what else runs a macro (the
/// hotkey has a column of its own). The image trigger's image isn't in it.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum TriggerOn {
    Schedule { schedule: WeeklySchedule },
    AppLaunch { exe: String },
    Pixel,
    Image,
}

impl MacroTriggers {
    /// The triggers besides the hotkey that would run the macro: on, and set
    /// up enough to fire.
    pub fn also_on(&self) -> Vec<TriggerOn> {
        let mut on = Vec::new();
        if self.schedule.enabled && self.schedule.schedule.days.iter().any(|d| *d) {
            on.push(TriggerOn::Schedule { schedule: self.schedule.schedule.clone() });
        }
        if self.app_launch.enabled && !self.app_launch.exe.is_empty() {
            on.push(TriggerOn::AppLaunch { exe: self.app_launch.exe.clone() });
        }
        if self.pixel.enabled {
            on.push(TriggerOn::Pixel);
        }
        if self.image.enabled && self.image.image.is_some() {
            on.push(TriggerOn::Image);
        }
        on
    }
}

/// Fires when a pixel starts matching the target color: two consecutive
/// matching samples after two consecutive non-matching ones. So a pixel that
/// stays red fires once, even if the cursor passes over it for one poll, and
/// a single-frame flicker doesn't fire. The image trigger uses it too, a
/// sample being whether the image is on screen.
#[derive(Debug, Default, Clone)]
pub struct PixelEdge {
    armed: bool,
    matches: u8,
    misses: u8,
}

impl PixelEdge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, matches: bool) -> bool {
        if !matches {
            self.matches = 0;
            self.misses = self.misses.saturating_add(1);
            self.armed |= self.misses >= 2;
            return false;
        }
        self.misses = 0;
        self.matches = self.matches.saturating_add(1);
        if self.armed && self.matches >= 2 {
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

    fn run(samples: &[u8]) -> Vec<bool> {
        let mut e = PixelEdge::new();
        samples.iter().map(|&m| e.update(m == 1)).collect()
    }

    #[test]
    fn the_library_hears_of_the_triggers_that_would_fire() {
        let mut t = MacroTriggers::default();
        assert_eq!(t.also_on(), vec![]);
        // On, but not set up enough to fire: no days, no program, no image.
        t.schedule =
            ScheduleTrigger { enabled: true, schedule: WeeklySchedule { days: [false; 7], ..Default::default() } };
        t.app_launch.enabled = true;
        t.image.enabled = true;
        assert_eq!(t.also_on(), vec![]);
        t.schedule.schedule = WeeklySchedule::default();
        t.app_launch.exe = "EXCEL.EXE".into();
        t.pixel.enabled = true;
        t.image.image = Some(ImagePng(vec![1, 2, 3]));
        t.hotkey = HotkeyTrigger { enabled: true, combo: "Ctrl + Alt + 1".into() };
        assert_eq!(
            t.also_on(),
            vec![
                TriggerOn::Schedule { schedule: WeeklySchedule::default() },
                TriggerOn::AppLaunch { exe: "EXCEL.EXE".into() },
                TriggerOn::Pixel,
                TriggerOn::Image,
            ]
        );
        t.pixel.enabled = false;
        assert_eq!(t.also_on().len(), 3);
    }

    #[test]
    fn pixel_fires_once_per_change() {
        // Matching from the start doesn't fire; a single matching sample doesn't either.
        let fired = run(&[1, 1, 0, 0, 1, 1, 1, 0, 0, 1, 0, 0, 1, 1]);
        let at: Vec<_> = fired.iter().enumerate().filter(|(_, f)| **f).map(|(i, _)| i).collect();
        assert_eq!(at, [5, 13]);
    }

    #[test]
    fn pixel_needs_two_misses_to_rearm() {
        // The cursor passes over a red pixel for one poll: no second fire.
        let fired = run(&[0, 0, 1, 1, 0, 1, 1, 0, 1, 1]);
        assert_eq!(fired.iter().filter(|f| **f).count(), 1, "{fired:?}");
        assert!(fired[3]);
        // At the start too: one miss doesn't arm it.
        assert!(!run(&[0, 1, 1]).contains(&true));
        assert_eq!(run(&[0, 0, 1, 1]), [false, false, false, true]);
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
