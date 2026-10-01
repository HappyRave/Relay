//! What the player's window says, and when it closes.

use relay_core::model::Ms;
use relay_playback::Tick;

use crate::exit::Exit;

pub const COUNTDOWN_MS: f64 = 3000.0;

#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    Countdown {
        left_ms: f64,
    },
    Playing(Tick),
    /// `progress` is where the bar stood when it ended.
    Ended {
        exit: Exit,
        progress: f32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Status {
    pub name: String,
    pub duration: Ms,
    pub phase: Phase,
    /// The latest thing worth knowing: a check that timed out, input
    /// Windows blocks, a window not found.
    pub notice: Option<String>,
}

impl Status {
    pub fn new(name: impl Into<String>, duration: Ms, countdown: bool) -> Status {
        let phase = if countdown {
            Phase::Countdown { left_ms: COUNTDOWN_MS }
        } else {
            Phase::Playing(Tick { t: 0.0, advancing: true, speed: 1.0, loop_idx: 0, loops: Some(1) })
        };
        Status { name: name.into(), duration, phase, notice: None }
    }

    pub fn end(&mut self, exit: Exit) {
        let progress = if exit == Exit::Completed { 1.0 } else { self.progress() };
        self.phase = Phase::Ended { exit, progress };
    }

    /// "Starting in 3", "0:12 / 0:45 · loop 2 of 3", "Done"…
    pub fn line(&self) -> String {
        match &self.phase {
            Phase::Countdown { left_ms } => format!("Starting in {}", (left_ms / 1000.0).ceil().max(1.0)),
            Phase::Playing(tick) => {
                let mut s = format!("{} / {}", clock(tick.t), clock(self.duration as f64));
                match tick.loops {
                    Some(1) => {}
                    Some(n) => s += &format!(" · loop {} of {n}", tick.loop_idx + 1),
                    None => s += &format!(" · loop {} of ∞", tick.loop_idx + 1),
                }
                if tick.advancing { s } else { format!("Waiting · {s}") }
            }
            Phase::Ended { exit, .. } => match exit {
                Exit::Completed => "Done",
                Exit::Stopped => "Stopped",
                Exit::Killed => "Stopped by the kill switch",
                Exit::TimedOut => "Timed out",
                Exit::Locked => "Not played",
                Exit::Error | Exit::BadArgs => "Playback failed",
            }
            .into(),
        }
    }

    /// How far the bar is filled, 0 to 1: the time in the current loop.
    pub fn progress(&self) -> f32 {
        match &self.phase {
            Phase::Countdown { .. } => 0.0,
            Phase::Playing(tick) if self.duration == 0 => {
                if tick.advancing {
                    0.0
                } else {
                    1.0
                }
            }
            Phase::Playing(tick) => (tick.t / self.duration as f64).clamp(0.0, 1.0) as f32,
            Phase::Ended { progress, .. } => *progress,
        }
    }

    /// Whether the window closes by itself: after a normal end, not after
    /// something the user should read.
    pub fn closes_on_its_own(&self) -> bool {
        matches!(self.phase, Phase::Ended { exit: Exit::Completed | Exit::Stopped | Exit::Killed, .. })
    }
}

/// "0:07", "12:34", "1:02:03".
fn clock(ms: f64) -> String {
    let s = (ms.max(0.0) / 1000.0).floor() as u64;
    let (h, m, s) = (s / 3600, s / 60 % 60, s % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playing(t: f64, loop_idx: u32, loops: Option<u32>, advancing: bool) -> Status {
        let mut s = Status::new("Export invoice", 45_000, true);
        s.phase = Phase::Playing(Tick { t, advancing, speed: 1.0, loop_idx, loops });
        s
    }

    #[test]
    fn the_countdown_counts_whole_seconds() {
        let mut s = Status::new("Export invoice", 45_000, true);
        assert_eq!(s.line(), "Starting in 3");
        s.phase = Phase::Countdown { left_ms: 2001.0 };
        assert_eq!(s.line(), "Starting in 3");
        s.phase = Phase::Countdown { left_ms: 2000.0 };
        assert_eq!(s.line(), "Starting in 2");
        s.phase = Phase::Countdown { left_ms: 0.0 };
        assert_eq!(s.line(), "Starting in 1");
        assert_eq!(s.progress(), 0.0);
        assert!(!s.closes_on_its_own());
    }

    #[test]
    fn without_a_countdown_it_starts_playing() {
        let s = Status::new("m", 1000, false);
        assert_eq!(s.line(), "0:00 / 0:01");
    }

    #[test]
    fn playing_shows_the_time_and_the_loop() {
        assert_eq!(playing(12_400.0, 0, Some(1), true).line(), "0:12 / 0:45");
        assert_eq!(playing(12_400.0, 1, Some(3), true).line(), "0:12 / 0:45 · loop 2 of 3");
        assert_eq!(playing(0.0, 4, None, true).line(), "0:00 / 0:45 · loop 5 of ∞");
        assert_eq!(playing(3000.0, 0, Some(1), false).line(), "Waiting · 0:03 / 0:45", "a check");
        let mut long = playing(3_723_000.0, 0, Some(1), true);
        long.duration = 7_200_000;
        assert_eq!(long.line(), "1:02:03 / 2:00:00");
        assert_eq!(playing(22_500.0, 0, Some(1), true).progress(), 0.5);
        assert_eq!(playing(99_000.0, 0, Some(1), true).progress(), 1.0);
    }

    #[test]
    fn an_empty_macro_fills_the_bar_when_done() {
        let mut s = Status::new("m", 0, false);
        assert_eq!(s.progress(), 0.0);
        s.phase = Phase::Playing(Tick { t: 0.0, advancing: false, speed: 1.0, loop_idx: 0, loops: Some(1) });
        assert_eq!(s.progress(), 1.0);
    }

    #[test]
    fn the_end_says_how_it_ended_and_whether_it_closes() {
        let ended = |exit| {
            let mut s = playing(9000.0, 0, Some(1), true);
            s.end(exit);
            (s.line(), s.progress(), s.closes_on_its_own())
        };
        assert_eq!(ended(Exit::Completed), ("Done".into(), 1.0, true));
        assert_eq!(ended(Exit::Stopped), ("Stopped".into(), 0.2, true));
        assert_eq!(ended(Exit::Killed), ("Stopped by the kill switch".into(), 0.2, true));
        assert_eq!(ended(Exit::TimedOut), ("Timed out".into(), 0.2, false));
        assert_eq!(ended(Exit::Error), ("Playback failed".into(), 0.2, false));
        assert_eq!(ended(Exit::Locked), ("Not played".into(), 0.2, false));
    }
}
