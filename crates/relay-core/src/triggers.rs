//! Edge detectors for the polling triggers. They are fed one sample per poll
//! and report the moment the trigger should fire.

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
