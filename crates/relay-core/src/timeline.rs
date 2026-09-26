//! How long a macro lasts, for the editor and the playback engine.

use crate::model::{Event, Ms};

/// Silence kept after the last event so the end of a macro is visible.
pub const TAIL_MS: Ms = 500;
/// The length of an empty macro, so its timeline still has a scale.
pub const MIN_DURATION_MS: Ms = 2000;

/// Total length of a macro: the end of its last event plus [`TAIL_MS`].
pub fn duration(events: &[Event]) -> Ms {
    events.iter().map(Event::end).max().map_or(MIN_DURATION_MS, |e| e.saturating_add(TAIL_MS))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_includes_waits_and_tail() {
        assert_eq!(duration(&[]), MIN_DURATION_MS);
        let ev = [Event::Move { t: 100, x: 0, y: 0 }, Event::Wait { t: 200, dur: 700, label: String::new() }];
        assert_eq!(duration(&ev), 1400);
    }
}
