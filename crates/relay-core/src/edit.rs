//! Edit operations on a macro's events, plus the normalization that keeps
//! every press balanced.

use std::collections::HashSet;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::keys::KeyStroke;
use crate::model::{Event, Macro, MouseBtn, Ms, Rgb};
use crate::steps::{Step, StepKind, group_steps};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "op", rename_all = "snake_case")]
#[ts(export)]
pub enum EditOp {
    Rename {
        name: String,
    },
    DeleteStep {
        index: u32,
    },
    InsertWait {
        at: Ms,
        dur: Ms,
        label: String,
    },
    InsertPixelWait {
        at: Ms,
        dur: Ms,
        x: i32,
        y: i32,
        color: Rgb,
        tolerance: u8,
        timeout_ms: Ms,
        label: String,
    },
    SetWaitDuration {
        index: u32,
        dur: Ms,
    },
    UpdatePixelWait {
        index: u32,
        x: i32,
        y: i32,
        color: Rgb,
        tolerance: u8,
        timeout_ms: Ms,
    },
    SetLabel {
        index: u32,
        label: String,
    },
    /// Sets the idle time before a step (see [`Step::pause`]). Cursor moves in
    /// the pause are retimed to fit, and everything after moves with the step.
    SetPause {
        index: u32,
        dur: Ms,
    },
    /// Shortens every pause longer than `max` to `max`.
    CapPauses {
        max: Ms,
    },
}

#[derive(Debug, Error, PartialEq)]
pub enum EditError {
    #[error("there is no step {0}")]
    NoSuchStep(u32),
    #[error("step {0} can't be edited this way")]
    WrongKind(u32),
}

pub fn apply(m: &mut Macro, op: EditOp) -> Result<(), EditError> {
    // Every edit but a rename addresses or respects steps.
    let steps = if matches!(op, EditOp::Rename { .. }) { Vec::new() } else { group_steps(&m.events, (&m.recording).into()) };
    let get = |index: u32| steps.get(index as usize).ok_or(EditError::NoSuchStep(index));
    match op {
        EditOp::Rename { name } => m.name = name,

        EditOp::DeleteStep { index } => {
            let step = get(index)?;
            let remove: HashSet<u32> = step.items.iter().copied().collect();
            let mut i = 0u32;
            m.events.retain(|_| {
                let keep = !remove.contains(&i);
                i += 1;
                keep
            });
            // Deleting a wait closes the gap it left.
            if let StepKind::Wait { dur, .. } | StepKind::PixelWait { dur, .. } = step.kind {
                shift_from(&mut m.events, step.end, -(dur as i64));
            }
            normalize(&mut m.events);
        }

        EditOp::InsertWait { at, dur, label } => {
            let dur = dur.min(MAX_DUR);
            insert_timed(&mut m.events, &steps, at, dur, |t| Event::Wait { t, dur, label });
        }

        EditOp::InsertPixelWait { at, dur, x, y, color, tolerance, timeout_ms, label } => {
            let dur = dur.min(MAX_DUR);
            insert_timed(&mut m.events, &steps, at, dur, |t| Event::PixelWait {
                t,
                dur,
                x,
                y,
                color,
                tolerance,
                timeout_ms,
                label,
            });
        }

        EditOp::SetWaitDuration { index, dur: new } => {
            let step = get(index)?;
            let item = step.items[0] as usize;
            let (StepKind::Wait { dur: old, .. } | StepKind::PixelWait { dur: old, .. }) = step.kind else {
                return Err(EditError::WrongKind(index));
            };
            let new = new.min(MAX_DUR);
            // Everything after the wait in the list moves: nothing happens
            // during a wait, so those are exactly the events after it in time.
            let delta = new as i64 - old as i64;
            for e in &mut m.events[item + 1..] {
                let t = e.t_mut();
                *t = (*t as i64 + delta).clamp(0, Ms::MAX as i64) as Ms;
            }
            if let Event::Wait { dur, .. } | Event::PixelWait { dur, .. } = &mut m.events[item] {
                *dur = new;
            }
        }

        EditOp::UpdatePixelWait { index, x: nx, y: ny, color: nc, tolerance: nt, timeout_ms: no } => {
            let step = get(index)?;
            match &mut m.events[step.items[0] as usize] {
                Event::PixelWait { x, y, color, tolerance, timeout_ms, .. } => {
                    (*x, *y, *color, *tolerance, *timeout_ms) = (nx, ny, nc, nt, no);
                }
                _ => return Err(EditError::WrongKind(index)),
            }
        }

        EditOp::SetLabel { index, label: new } => {
            let step = get(index)?;
            let first = step.items[0] as usize;
            match &mut m.events[first] {
                Event::Button { label, down: true, .. }
                | Event::Wait { label, .. }
                | Event::PixelWait { label, .. } => {
                    *label = new;
                }
                _ => return Err(EditError::WrongKind(index)),
            }
        }

        EditOp::SetPause { index, dur } => {
            let step = get(index)?;
            retime_pauses(&mut m.events, &[(step.t - step.pause, step.t, dur.min(MAX_DUR))]);
        }

        EditOp::CapPauses { max } => {
            let long: Vec<_> = steps.iter().filter(|s| s.pause > max).map(|s| (s.t - s.pause, s.t, max)).collect();
            retime_pauses(&mut m.events, &long);
        }
    }
    m.modified_at = Utc::now();
    Ok(())
}

/// The longest wait or pause an edit may set (a day), so times stay far
/// from overflowing.
pub const MAX_DUR: Ms = 24 * 60 * 60 * 1000;

/// Gives each pause `from..to` (sorted, not overlapping) a new length `dur`,
/// in one pass: events inside a pause (cursor moves, a shared modifier's
/// release) are scaled to fit, events after it shift by the difference.
/// Monotone, so the events stay in order.
fn retime_pauses(events: &mut [Event], pauses: &[(Ms, Ms, Ms)]) {
    let mut shift: i64 = 0; // what the pauses already passed added or removed
    let mut k = 0;
    for e in events.iter_mut() {
        let t = e.t_mut();
        let old = *t;
        while k < pauses.len() && old >= pauses[k].1 {
            let (from, to, dur) = pauses[k];
            shift += dur as i64 - (to - from) as i64;
            k += 1;
        }
        let new = match pauses.get(k) {
            Some(&(from, to, dur)) if old > from => {
                from as i64 + shift + ((old - from) as u64 * dur as u64 / (to - from) as u64) as i64
            }
            _ => old as i64 + shift,
        };
        *t = new.clamp(0, Ms::MAX as i64) as Ms;
    }
}

/// Moves `at` just past the step it falls on, from its start to its end (a
/// press, a drag, a wait), so a step's own release is never pushed behind the
/// inserted wait. Clicking a step puts the playhead on its start, so an
/// insertion there lands after that step.
fn snap_insertion(steps: &[Step], mut at: Ms) -> Ms {
    while let Some(s) = steps.iter().find(|s| s.t <= at && at <= s.end) {
        at = s.end.saturating_add(1);
    }
    at
}

fn insert_timed(events: &mut Vec<Event>, steps: &[Step], at: Ms, dur: Ms, make: impl FnOnce(Ms) -> Event) {
    let at = snap_insertion(steps, at);
    shift_from(events, at, dur as i64);
    let pos = events.partition_point(|e| e.t() < at);
    events.insert(pos, make(at));
}

/// Adds `delta` to the time of every event at or after `from`.
fn shift_from(events: &mut [Event], from: Ms, delta: i64) {
    for e in events.iter_mut() {
        let t = e.t_mut();
        if *t >= from {
            *t = (*t as i64 + delta).clamp(0, Ms::MAX as i64) as Ms;
        }
    }
}

/// What a key or button event presses: the physical key, or the button.
#[derive(PartialEq, Clone)]
enum Press {
    Key(KeyStroke),
    Button(MouseBtn),
}

impl Press {
    fn of(e: &Event) -> Option<(Press, bool)> {
        match e {
            Event::Key { key, down, .. } => Some((Press::Key(key.clone()), *down)),
            Event::Button { btn, down, .. } => Some((Press::Button(*btn), *down)),
            _ => None,
        }
    }

    fn same(&self, other: &Press) -> bool {
        match (self, other) {
            (Press::Key(a), Press::Key(b)) => a.code == b.code,
            (Press::Button(a), Press::Button(b)) => a == b,
            _ => false,
        }
    }
}

/// Sorts events by time, drops releases whose press is missing (it happened
/// before recording started) and releases anything still held at the end.
pub fn normalize(events: &mut Vec<Event>) {
    events.sort_by_key(Event::t);
    // Presses still held, oldest first (only a handful at any time).
    let mut held: Vec<Press> = Vec::new();
    let mut cursor = (0, 0);
    events.retain(|e| {
        if let Some(p) = e.pos() {
            cursor = p;
        }
        let Some((press, down)) = Press::of(e) else {
            return true;
        };
        let at = held.iter().position(|h| h.same(&press));
        match (down, at) {
            // Keys auto-repeat; a second button down without an up is dropped.
            (true, Some(_)) => matches!(press, Press::Key(_)),
            (true, None) => {
                held.push(press);
                true
            }
            (false, Some(i)) => {
                held.remove(i);
                true
            }
            (false, None) => false,
        }
    });
    let t = events.last().map_or(0, Event::t);
    for press in held.into_iter().rev() {
        events.push(match press {
            Press::Key(key) => Event::Key { t, down: false, key, ch: None },
            Press::Button(btn) => Event::Button { t, x: cursor.0, y: cursor.1, btn, down: false, label: String::new() },
        });
    }
}

/// Checks the invariants edits must preserve (for tests).
#[cfg(test)]
pub fn check_invariants(events: &[Event]) -> Result<(), String> {
    if !events.windows(2).all(|w| w[0].t() <= w[1].t()) {
        return Err("events are not sorted".into());
    }
    let mut normalized = events.to_vec();
    normalize(&mut normalized);
    if normalized != events {
        return Err("presses are not balanced".into());
    }
    for w in events.iter().filter(|e| matches!(e, Event::Wait { .. } | Event::PixelWait { .. })) {
        let (start, end) = (w.t(), w.end());
        if let Some(e) = events.iter().find(|e| e.t() > start && e.t() < end) {
            return Err(format!("{e:?} happens during the wait at {start}..{end}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RecordingMeta;
    use crate::steps::group_steps;

    fn key(t: Ms, code: &str, down: bool) -> Event {
        Event::Key { t, down, key: KeyStroke::code(code), ch: None }
    }
    fn click(t: Ms) -> [Event; 2] {
        let b = |t, down| Event::Button { t, x: 5, y: 5, btn: MouseBtn::Left, down, label: String::new() };
        [b(t, true), b(t + 80, false)]
    }
    fn mac(events: Vec<Event>) -> Macro {
        Macro::new("t", RecordingMeta::single_1080p(), events)
    }

    #[test]
    fn normalize_drops_orphan_releases_and_closes_held_keys() {
        let mut ev = vec![key(0, "KeyA", false), key(10, "KeyB", true), Event::Move { t: 20, x: 1, y: 1 }];
        normalize(&mut ev);
        assert_eq!(ev, vec![key(10, "KeyB", true), Event::Move { t: 20, x: 1, y: 1 }, key(20, "KeyB", false)]);
        assert!(check_invariants(&ev).is_ok());
    }

    #[test]
    fn insert_wait_shifts_later_events_and_snaps_out_of_a_press() {
        let mut m = mac([click(0), click(1000)].concat());
        apply(&mut m, EditOp::InsertWait { at: 40, dur: 500, label: "x".into() }).unwrap();
        let ts: Vec<_> = m.events.iter().map(Event::t).collect();
        assert_eq!(ts, [0, 80, 81, 1500, 1580]);
        assert!(matches!(m.events[2], Event::Wait { t: 81, dur: 500, .. }));
        check_invariants(&m.events).unwrap();
    }

    #[test]
    fn deleting_a_wait_closes_the_gap() {
        let mut m = mac([click(0), click(1000)].concat());
        apply(&mut m, EditOp::InsertWait { at: 500, dur: 700, label: String::new() }).unwrap();
        apply(&mut m, EditOp::DeleteStep { index: 1 }).unwrap();
        assert_eq!(m.events, mac([click(0), click(1000)].concat()).events);
    }

    #[test]
    fn deleting_a_shortcut_removes_its_modifiers() {
        let mut m = mac(vec![
            key(0, "ControlLeft", true),
            key(10, "KeyS", true),
            key(20, "KeyS", false),
            key(30, "ControlLeft", false),
        ]);
        apply(&mut m, EditOp::DeleteStep { index: 0 }).unwrap();
        assert!(m.events.is_empty());
    }

    #[test]
    fn set_wait_duration_moves_what_follows() {
        let mut m = mac(click(1000).to_vec());
        apply(&mut m, EditOp::InsertWait { at: 0, dur: 500, label: String::new() }).unwrap();
        apply(&mut m, EditOp::SetWaitDuration { index: 0, dur: 200 }).unwrap();
        assert_eq!(m.events[1].t(), 1200);
        assert_eq!(apply(&mut m, EditOp::SetWaitDuration { index: 1, dur: 5 }), Err(EditError::WrongKind(1)));
        assert_eq!(apply(&mut m, EditOp::DeleteStep { index: 9 }), Err(EditError::NoSuchStep(9)));
    }

    #[test]
    fn inserting_at_a_step_start_goes_after_the_step() {
        // Clicking a step puts the playhead on its start: the wait follows it.
        let mut m = mac([click(0), click(1000)].concat());
        apply(&mut m, EditOp::InsertWait { at: 1000, dur: 500, label: String::new() }).unwrap();
        let ts: Vec<_> = m.events.iter().map(Event::t).collect();
        assert_eq!(ts, [0, 80, 1000, 1080, 1081]);
        assert!(matches!(m.events[4], Event::Wait { t: 1081, .. }));
        check_invariants(&m.events).unwrap();
    }

    #[test]
    fn set_pause_retimes_the_path_and_moves_what_follows() {
        let mv = |t| Event::Move { t, x: t as i32, y: 0 };
        let mut m = mac([click(0).to_vec(), vec![mv(500), mv(1500)], click(2080).to_vec()].concat());
        // The second click comes after a 2000 ms pause (80..2080).
        let steps = group_steps(&m.events, (&m.recording).into());
        assert_eq!((steps[0].pause, steps[1].pause), (0, 2000));
        // (Not under the double-click time: the two clicks would merge.)
        apply(&mut m, EditOp::SetPause { index: 1, dur: 600 }).unwrap();
        let ts: Vec<_> = m.events.iter().map(Event::t).collect();
        assert_eq!(ts, [0, 80, 206, 506, 680, 760]);
        // The path keeps its shape: the moves keep their positions.
        assert!(matches!(m.events[3], Event::Move { x: 1500, .. }));
        check_invariants(&m.events).unwrap();
        // And back.
        apply(&mut m, EditOp::SetPause { index: 1, dur: 2000 }).unwrap();
        assert_eq!(m.events.last().unwrap().t(), 2160);
    }

    #[test]
    fn cap_pauses_shortens_only_long_pauses() {
        let mut m = mac([click(0), click(3080), click(3700), click(9000)].concat());
        apply(&mut m, EditOp::CapPauses { max: 1000 }).unwrap();
        let pauses: Vec<_> = group_steps(&m.events, (&m.recording).into()).iter().map(|s| s.pause).collect();
        assert_eq!(pauses, [0, 1000, 540, 1000]);
        check_invariants(&m.events).unwrap();
    }

    #[test]
    fn inserting_inside_a_shortcut_goes_after_its_modifier() {
        let mut m = mac(vec![
            key(0, "ControlLeft", true),
            key(50, "KeyA", true),
            key(90, "KeyA", false),
            key(300, "ControlLeft", false),
        ]);
        apply(&mut m, EditOp::InsertWait { at: 50, dur: 1000, label: String::new() }).unwrap();
        assert!(matches!(m.events[4], Event::Wait { t: 301, .. }), "{:?}", m.events);
        assert_eq!(m.events[3].t(), 300, "Ctrl isn't held through the wait");
        check_invariants(&m.events).unwrap();
    }

    #[test]
    fn a_zero_length_wait_keeps_same_time_events_in_order() {
        let mut m = mac([click(0).to_vec(), vec![Event::Move { t: 500, x: 1, y: 1 }], click(1000).to_vec()].concat());
        apply(&mut m, EditOp::InsertWait { at: 500, dur: 0, label: String::new() }).unwrap();
        let wait = group_steps(&m.events, (&m.recording).into())
            .iter()
            .position(|s| matches!(s.kind, StepKind::Wait { .. }))
            .unwrap();
        apply(&mut m, EditOp::SetWaitDuration { index: wait as u32, dur: 300 }).unwrap();
        check_invariants(&m.events).unwrap();
        assert_eq!(m.events.last().unwrap().t(), 1380);
    }

    #[test]
    fn huge_times_and_durations_saturate_instead_of_panicking() {
        let mut m = mac(vec![Event::Wait { t: Ms::MAX - 10, dur: 1000, label: String::new() }]);
        let _ = crate::view::MacroView::of(&m);
        apply(&mut m, EditOp::SetPause { index: 0, dur: Ms::MAX }).unwrap();
        apply(&mut m, EditOp::SetWaitDuration { index: 0, dur: Ms::MAX }).unwrap();
        assert!(matches!(m.events[0], Event::Wait { dur: MAX_DUR, .. }));
    }

    #[test]
    fn labels_live_on_the_press() {
        let mut m = mac(click(0).to_vec());
        apply(&mut m, EditOp::SetLabel { index: 0, label: "Save".into() }).unwrap();
        assert!(matches!(&m.events[0], Event::Button { label, .. } if label == "Save"));
    }
}
