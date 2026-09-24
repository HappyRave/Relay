//! Time math shared by the editor and the playback engine.

use crate::model::{Event, Ms};
use crate::steps::Step;

/// Silence kept after the last event so the end of a macro is visible.
pub const TAIL_MS: Ms = 500;
/// The shortest timeline shown, e.g. for an empty macro.
pub const MIN_DURATION_MS: Ms = 2000;

/// Total length of a macro: the end of its last event plus [`TAIL_MS`].
pub fn duration(events: &[Event]) -> Ms {
    events.iter().map(Event::end).max().map_or(MIN_DURATION_MS, |e| e + TAIL_MS)
}

/// Index of the first event at or after `t` (where playback resumes after a seek).
pub fn event_index_at(events: &[Event], t: Ms) -> usize {
    events.partition_point(|e| e.t() < t)
}

/// Index of the last step that started at or before `t`.
pub fn step_at(steps: &[Step], t: Ms) -> Option<usize> {
    steps.partition_point(|s| s.t <= t).checked_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::steps::{GroupOptions, group_steps};

    #[test]
    fn duration_includes_waits_and_tail() {
        assert_eq!(duration(&[]), MIN_DURATION_MS);
        let ev = [Event::Move { t: 100, x: 0, y: 0 }, Event::Wait { t: 200, dur: 700, label: String::new() }];
        assert_eq!(duration(&ev), 1400);
    }

    #[test]
    fn lookups() {
        let ev = [
            Event::Wait { t: 0, dur: 100, label: String::new() },
            Event::Wait { t: 100, dur: 100, label: String::new() },
        ];
        assert_eq!(event_index_at(&ev, 50), 1);
        let steps = group_steps(&ev, GroupOptions::default());
        assert_eq!(step_at(&steps, 50), Some(0));
        assert_eq!(step_at(&steps, 100), Some(1));
    }
}
