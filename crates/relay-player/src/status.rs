//! What the player's window says, and when it closes. Like Relay's own
//! player: a state, the macro's name, the time (or the data file's row), and
//! a second line with the progress, or a notice when there's one.

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

/// How the state's badge looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Red: something is happening (counting down, playing, waiting).
    Live,
    Plain,
    /// Green: it completed.
    Done,
    /// It ended on a failure.
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Status {
    pub name: String,
    pub duration: Ms,
    pub phase: Phase,
    /// Paused with the Pause key.
    pub paused: bool,
    /// It plays once per row of a data file: the time says which row.
    pub rows: bool,
    /// The latest thing worth knowing: a check that timed out, input
    /// Windows blocks, a window not found.
    pub notice: Option<String>,
}

impl Status {
    pub fn new(name: impl Into<String>, duration: Ms, countdown: bool, rows: bool) -> Status {
        let phase = if countdown {
            Phase::Countdown { left_ms: COUNTDOWN_MS }
        } else {
            Phase::Playing(Tick { t: 0.0, advancing: true, speed: 1.0, loop_idx: 0, loops: Some(1) })
        };
        Status { name: name.into(), duration, phase, paused: false, rows, notice: None }
    }

    pub fn end(&mut self, exit: Exit) {
        let progress = if exit == Exit::Completed { 1.0 } else { self.progress() };
        self.phase = Phase::Ended { exit, progress };
        self.paused = false;
        if exit == Exit::Killed && self.notice.is_none() {
            self.notice = Some("Stopped by the kill switch, Ctrl + Alt + End.".into());
        }
    }

    /// The badge: "PLAYING", "DONE ✓"…
    pub fn state(&self) -> (&'static str, Tone) {
        match &self.phase {
            Phase::Countdown { .. } => ("GET READY", Tone::Live),
            Phase::Playing(_) if self.paused => ("PAUSED", Tone::Plain),
            Phase::Playing(tick) if !tick.advancing => ("WAITING", Tone::Live),
            Phase::Playing(_) => ("PLAYING", Tone::Live),
            Phase::Ended { exit, .. } => match exit {
                Exit::Completed => ("DONE ✓", Tone::Done),
                Exit::Stopped | Exit::Killed => ("STOPPED", Tone::Plain),
                Exit::TimedOut => ("TIMED OUT", Tone::Failed),
                Exit::Locked => ("NOT PLAYED", Tone::Plain),
                Exit::Error | Exit::BadArgs => ("FAILED", Tone::Failed),
            },
        }
    }

    /// At the right of the screen: the countdown, the row of a data file, or
    /// the time ("00:12 / 00:45", with "2/3 · " in front for a repeat).
    pub fn right(&self) -> String {
        match &self.phase {
            Phase::Countdown { left_ms } => format!("{}", (left_ms / 1000.0).ceil().max(1.0)),
            Phase::Playing(tick) if self.rows => {
                format!("Row {} / {}", tick.loop_idx + 1, tick.loops.map_or("∞".into(), |n| n.to_string()))
            }
            Phase::Playing(tick) => {
                let time = format!("{} / {}", clock(tick.t), clock(self.duration as f64));
                match tick.loops {
                    Some(1) => time,
                    Some(n) => format!("{}/{n} · {time}", tick.loop_idx + 1),
                    None => format!("{}/∞ · {time}", tick.loop_idx + 1),
                }
            }
            Phase::Ended { progress, .. } => {
                format!("{} / {}", clock(*progress as f64 * self.duration as f64), clock(self.duration as f64))
            }
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

    /// Whether Pause (or Play, while paused) can be pressed.
    pub fn can_pause(&self) -> bool {
        matches!(self.phase, Phase::Playing(_))
    }

    /// Whether the window closes by itself: after a normal end, not after
    /// something the user should read.
    pub fn closes_on_its_own(&self) -> bool {
        matches!(self.phase, Phase::Ended { exit: Exit::Completed | Exit::Stopped | Exit::Killed, .. })
    }
}

/// "00:07", "12:34", "1:02:03".
fn clock(ms: f64) -> String {
    let s = (ms.max(0.0) / 1000.0).floor() as u64;
    let (h, m, s) = (s / 3600, s / 60 % 60, s % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m:02}:{s:02}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playing(t: f64, loop_idx: u32, loops: Option<u32>, advancing: bool) -> Status {
        let mut s = Status::new("Export invoice", 45_000, true, false);
        s.phase = Phase::Playing(Tick { t, advancing, speed: 1.0, loop_idx, loops });
        s
    }

    #[test]
    fn the_countdown_counts_whole_seconds() {
        let mut s = Status::new("Export invoice", 45_000, true, false);
        assert_eq!((s.state(), s.right()), (("GET READY", Tone::Live), "3".into()));
        s.phase = Phase::Countdown { left_ms: 2001.0 };
        assert_eq!(s.right(), "3");
        s.phase = Phase::Countdown { left_ms: 2000.0 };
        assert_eq!(s.right(), "2");
        s.phase = Phase::Countdown { left_ms: 0.0 };
        assert_eq!(s.right(), "1");
        assert_eq!(s.progress(), 0.0);
        assert!(!s.closes_on_its_own() && !s.can_pause());
    }

    #[test]
    fn without_a_countdown_it_starts_playing() {
        let s = Status::new("m", 1000, false, false);
        assert_eq!((s.state(), s.right()), (("PLAYING", Tone::Live), "00:00 / 00:01".into()));
    }

    #[test]
    fn playing_shows_the_time_and_the_loop() {
        assert_eq!(playing(12_400.0, 0, Some(1), true).right(), "00:12 / 00:45");
        assert_eq!(playing(12_400.0, 1, Some(3), true).right(), "2/3 · 00:12 / 00:45");
        assert_eq!(playing(0.0, 4, None, true).right(), "5/∞ · 00:00 / 00:45");
        assert_eq!(playing(3000.0, 0, Some(1), false).state(), ("WAITING", Tone::Live), "a check");
        let mut long = playing(3_723_000.0, 0, Some(1), true);
        long.duration = 7_200_000;
        assert_eq!(long.right(), "1:02:03 / 2:00:00");
        assert_eq!(playing(22_500.0, 0, Some(1), true).progress(), 0.5);
        assert_eq!(playing(99_000.0, 0, Some(1), true).progress(), 1.0);
    }

    #[test]
    fn a_data_file_run_says_which_row() {
        let mut s = playing(1000.0, 36, Some(200), true);
        s.rows = true;
        assert_eq!(s.right(), "Row 37 / 200");
    }

    #[test]
    fn paused_says_so_and_can_play_again() {
        let mut s = playing(1000.0, 0, Some(1), false);
        s.paused = true;
        assert_eq!(s.state(), ("PAUSED", Tone::Plain));
        assert!(s.can_pause());
    }

    #[test]
    fn an_empty_macro_fills_the_bar_when_done() {
        let mut s = Status::new("m", 0, false, false);
        assert_eq!(s.progress(), 0.0);
        s.phase = Phase::Playing(Tick { t: 0.0, advancing: false, speed: 1.0, loop_idx: 0, loops: Some(1) });
        assert_eq!(s.progress(), 1.0);
    }

    #[test]
    fn the_end_says_how_it_ended_and_whether_it_closes() {
        let ended = |exit| {
            let mut s = playing(9000.0, 0, Some(1), true);
            s.end(exit);
            (s.state(), s.progress(), s.closes_on_its_own())
        };
        assert_eq!(ended(Exit::Completed), (("DONE ✓", Tone::Done), 1.0, true));
        assert_eq!(ended(Exit::Stopped), (("STOPPED", Tone::Plain), 0.2, true));
        assert_eq!(ended(Exit::Killed), (("STOPPED", Tone::Plain), 0.2, true));
        assert_eq!(ended(Exit::TimedOut), (("TIMED OUT", Tone::Failed), 0.2, false));
        assert_eq!(ended(Exit::Error), (("FAILED", Tone::Failed), 0.2, false));
        assert_eq!(ended(Exit::Locked), (("NOT PLAYED", Tone::Plain), 0.2, false));
        let mut s = playing(9000.0, 0, Some(1), true);
        s.paused = true;
        s.end(Exit::Killed);
        assert_eq!(s.notice.as_deref(), Some("Stopped by the kill switch, Ctrl + Alt + End."));
        assert!(!s.paused && !s.can_pause());
        assert_eq!(s.right(), "00:09 / 00:45");
    }
}
