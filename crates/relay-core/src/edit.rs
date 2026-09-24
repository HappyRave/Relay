//! Edit operations on a macro's events, plus the normalization that keeps
//! every press balanced.

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

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
    Rename { name: String },
    DeleteStep { index: u32 },
    InsertWait { at: Ms, dur: Ms, label: String },
    InsertPixelWait { at: Ms, dur: Ms, x: i32, y: i32, color: Rgb, tolerance: u8, timeout_ms: Ms, label: String },
    SetWaitDuration { index: u32, dur: Ms },
    UpdatePixelWait { index: u32, x: i32, y: i32, color: Rgb, tolerance: u8, timeout_ms: Ms },
    SetLabel { index: u32, label: String },
    /// Sets the idle time before a step (see [`Step::pause`]). Cursor moves in
    /// the pause are retimed to fit, and everything after moves with the step.
    SetPause { index: u32, dur: Ms },
    /// Shortens every pause longer than `max` to `max`.
    CapPauses { max: Ms },
}

#[derive(Debug, Error, PartialEq)]
pub enum EditError {
    #[error("there is no step {0}")]
    NoSuchStep(u32),
    #[error("step {0} can't be edited this way")]
    WrongKind(u32),
}

pub fn apply(m: &mut Macro, op: EditOp) -> Result<(), EditError> {
    let steps = group_steps(&m.events, (&m.recording).into());
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
            insert_timed(&mut m.events, &steps, at, dur, |t| Event::Wait { t, dur, label });
        }

        EditOp::InsertPixelWait { at, dur, x, y, color, tolerance, timeout_ms, label } => {
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
            let start = m.events[item].t();
            shift_from(&mut m.events, step.end, new as i64 - old as i64);
            // A zero-length wait sits at `step.end`; keep it where it was.
            *m.events[item].t_mut() = start;
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
                Event::Button { label, down: true, .. } | Event::Wait { label, .. } | Event::PixelWait { label, .. } => {
                    *label = new;
                }
                _ => return Err(EditError::WrongKind(index)),
            }
        }

        EditOp::SetPause { index, dur } => {
            let step = get(index)?;
            retime_pause(&mut m.events, step.t - step.pause, step.t, dur);
        }

        EditOp::CapPauses { max } => {
            // Last to first: retiming a pause only moves what comes after it,
            // so the earlier pauses stay where they were computed.
            for s in steps.iter().rev().filter(|s| s.pause > max) {
                retime_pause(&mut m.events, s.t - s.pause, s.t, max);
            }
        }
    }
    m.modified_at = Utc::now();
    Ok(())
}

/// Turns the pause `from..to` into one of `dur` ms: events inside it (cursor
/// moves, a shared modifier's release) are scaled to fit, events at or after
/// `to` shift by the difference. Keeps the events in order.
fn retime_pause(events: &mut [Event], from: Ms, to: Ms, dur: Ms) {
    let old = to - from;
    if old == dur {
        return;
    }
    for e in events.iter_mut() {
        let t = e.t_mut();
        if *t >= to {
            *t = *t - old + dur;
        } else if *t > from {
            *t = from + ((*t - from) as u64 * dur as u64 / old as u64) as Ms;
        }
    }
}

/// Moves `at` just past the step it falls on, from its start to its end (a
/// press, a drag, a wait), so a step's own release is never pushed behind the
/// inserted wait. Clicking a step puts the playhead on its start, so an
/// insertion there lands after that step.
fn snap_insertion(steps: &[Step], mut at: Ms) -> Ms {
    while let Some(s) = steps.iter().find(|s| s.t <= at && at <= s.end) {
        at = s.end + 1;
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
            *t = (*t as i64 + delta).max(0) as Ms;
        }
    }
}

#[derive(Hash, PartialEq, Eq, Clone)]
enum Press {
    Key(String),
    Button(MouseBtn),
}

/// Sorts events by time, drops releases whose press is missing (it happened
/// before recording started) and releases anything still held at the end.
pub fn normalize(events: &mut Vec<Event>) {
    events.sort_by_key(Event::t);
    let mut held: HashMap<Press, Event> = HashMap::new();
    let mut order: Vec<Press> = Vec::new();
    let mut cursor = (0, 0);
    events.retain(|e| {
        if let Some(p) = e.pos() {
            cursor = p;
        }
        let (press, down) = match e {
            Event::Key { key, down, .. } => (Press::Key(key.code.clone()), *down),
            Event::Button { btn, down, .. } => (Press::Button(*btn), *down),
            _ => return true,
        };
        if down {
            match held.entry(press) {
                // Keys auto-repeat; a second button down without an up is dropped.
                Entry::Occupied(o) => matches!(o.key(), Press::Key(_)),
                Entry::Vacant(v) => {
                    order.push(v.key().clone());
                    v.insert(e.clone());
                    true
                }
            }
        } else {
            order.retain(|p| p != &press);
            held.remove(&press).is_some()
        }
    });
    let end = events.last().map_or(0, Event::t);
    for press in order.into_iter().rev() {
        match held.remove(&press) {
            Some(Event::Key { key, .. }) => events.push(release_key(end, key)),
            Some(Event::Button { btn, .. }) => events.push(Event::Button {
                t: end,
                x: cursor.0,
                y: cursor.1,
                btn,
                down: false,
                label: String::new(),
            }),
            _ => {}
        }
    }
}

fn release_key(t: Ms, key: KeyStroke) -> Event {
    Event::Key { t, down: false, key, ch: None }
}

/// Checks the invariants edits must preserve; used by tests.
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
        let mut m = mac(vec![key(0, "ControlLeft", true), key(10, "KeyS", true), key(20, "KeyS", false), key(30, "ControlLeft", false)]);
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
    fn labels_live_on_the_press() {
        let mut m = mac(click(0).to_vec());
        apply(&mut m, EditOp::SetLabel { index: 0, label: "Save".into() }).unwrap();
        assert!(matches!(&m.events[0], Event::Button { label, .. } if label == "Save"));
    }
}
