//! Edit operations on a macro's events, plus the normalization that keeps
//! every press balanced.

use std::collections::HashSet;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;

use crate::image::MIN_THRESHOLD;
use crate::keys::KeyStroke;
use crate::model::{Event, ImagePng, Macro, MouseBtn, Ms, Rect, Rgb};
use crate::path;
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
    InsertFindImage {
        at: Ms,
        dur: Ms,
        image: ImagePng,
        click_x: i32,
        click_y: i32,
        btn: MouseBtn,
        threshold: u8,
        timeout_ms: Ms,
        area: Option<Rect>,
        label: String,
    },
    UpdateFindImage {
        index: u32,
        image: ImagePng,
        click_x: i32,
        click_y: i32,
        btn: MouseBtn,
        threshold: u8,
        timeout_ms: Ms,
        area: Option<Rect>,
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
    /// Sets how long a MOVE step takes, from its first sample to its last:
    /// the samples are retimed to fit, and everything after moves with it.
    SetMoveDuration {
        index: u32,
        dur: Ms,
    },
    /// Takes the wobble out of a MOVE step's path ([`path::smooth`]).
    SmoothMove {
        index: u32,
    },
    /// Makes a MOVE step's path a straight line ([`path::straighten`]).
    StraightenMove {
        index: u32,
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
    let steps =
        if matches!(op, EditOp::Rename { .. }) { Vec::new() } else { group_steps(&m.events, (&m.recording).into()) };
    let get = |index: u32| steps.get(index as usize).ok_or(EditError::NoSuchStep(index));
    // Events that saturated at `Ms::MAX` have lost how far past it they were,
    // so moving them back can land them inside a wait that also ends there.
    let saturated = m.events.iter().any(|e| e.end() == Ms::MAX);
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
            if let StepKind::Wait { dur, .. } | StepKind::PixelWait { dur, .. } | StepKind::FindImage { dur, .. } =
                step.kind
            {
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

        EditOp::InsertFindImage { at, dur, image, click_x, click_y, btn, threshold, timeout_ms, area, label } => {
            let (dur, threshold) = (dur.min(MAX_DUR), threshold.clamp(MIN_THRESHOLD, 100));
            insert_timed(&mut m.events, &steps, at, dur, |t| Event::FindImage {
                t,
                dur,
                image,
                click_x,
                click_y,
                btn,
                threshold,
                timeout_ms,
                area,
                label,
            });
        }

        EditOp::SetWaitDuration { index, dur: new } => {
            let step = get(index)?;
            let item = step.items[0] as usize;
            let (StepKind::Wait { dur: old, .. }
            | StepKind::PixelWait { dur: old, .. }
            | StepKind::FindImage { dur: old, .. }) = step.kind
            else {
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
            if let Event::Wait { dur, .. } | Event::PixelWait { dur, .. } | Event::FindImage { dur, .. } =
                &mut m.events[item]
            {
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

        EditOp::UpdateFindImage {
            index,
            image: ni,
            click_x: ncx,
            click_y: ncy,
            btn: nb,
            threshold: nt,
            timeout_ms: no,
            area: na,
        } => {
            let step = get(index)?;
            match &mut m.events[step.items[0] as usize] {
                Event::FindImage { image, click_x, click_y, btn, threshold, timeout_ms, area, .. } => {
                    (*image, *click_x, *click_y, *btn) = (ni, ncx, ncy, nb);
                    (*threshold, *timeout_ms, *area) = (nt.clamp(MIN_THRESHOLD, 100), no, na);
                }
                _ => return Err(EditError::WrongKind(index)),
            }
        }

        EditOp::SetLabel { index, label: new } => {
            let step = get(index)?;
            // The first press, not the first item: a Shift-click starts with Shift.
            let target = step.items.iter().map(|&i| i as usize).find(|&i| {
                matches!(
                    m.events[i],
                    Event::Button { down: true, .. }
                        | Event::Wait { .. }
                        | Event::PixelWait { .. }
                        | Event::FindImage { .. }
                )
            });
            match target.map(|i| &mut m.events[i]) {
                Some(
                    Event::Button { label, .. }
                    | Event::Wait { label, .. }
                    | Event::PixelWait { label, .. }
                    | Event::FindImage { label, .. },
                ) => *label = new,
                _ => return Err(EditError::WrongKind(index)),
            }
        }

        EditOp::SetPause { index, dur } => {
            let step = get(index)?;
            if step.pause > 0 {
                retime(&mut m.events, &[(step.t - step.pause, step.t, dur.min(MAX_DUR))]);
            } else {
                // No gap to stretch: the step and what follows it in the list
                // move, not what ends at that same moment (a move's last sample).
                let first = step.items[0] as usize;
                for e in &mut m.events[first..] {
                    let t = e.t_mut();
                    *t = t.saturating_add(dur.min(MAX_DUR));
                }
            }
        }

        EditOp::CapPauses { max } => {
            let long: Vec<_> = steps.iter().filter(|s| s.pause > max).map(|s| (s.t - s.pause, s.t, max)).collect();
            retime(&mut m.events, &long);
        }

        EditOp::SetMoveDuration { index, dur } => {
            let step = get(index)?;
            let StepKind::Move { .. } = step.kind else {
                return Err(EditError::WrongKind(index));
            };
            // A single sample is a jump: it has no length to set.
            if step.end > step.t {
                retime(&mut m.events, &[(step.t, step.end, dur.min(MAX_DUR))]);
            }
        }

        EditOp::SmoothMove { index } => reshape(m, get(index)?, index, path::smooth)?,

        EditOp::StraightenMove { index } => reshape(m, get(index)?, index, path::straighten)?,
    }
    if saturated {
        normalize(&mut m.events);
    }
    m.modified_at = Utc::now();
    Ok(())
}

/// The longest wait or pause an edit may set (a day), so times stay far
/// from overflowing.
pub const MAX_DUR: Ms = 24 * 60 * 60 * 1000;

/// Moves a MOVE step's samples onto the path `f` makes of it. The path
/// starts where the cursor was before the move, which stays put, as does the
/// last sample; the samples keep their times.
fn reshape(m: &mut Macro, step: &Step, index: u32, f: fn(&[path::Point]) -> Vec<path::Point>) -> Result<(), EditError> {
    let StepKind::Move { x, y, .. } = step.kind else {
        return Err(EditError::WrongKind(index));
    };
    let mut points = vec![(x, y)];
    points.extend(step.items.iter().filter_map(|&i| m.events[i as usize].pos()));
    for (&i, &(nx, ny)) in step.items.iter().zip(&f(&points)[1..]) {
        if let Event::Move { x, y, .. } = &mut m.events[i as usize] {
            (*x, *y) = (nx, ny);
        }
    }
    Ok(())
}

/// Gives each span `from..to` (sorted, not overlapping: pauses, or a move
/// from its first sample to its last) a new length `dur`, in one pass: events
/// inside a span (cursor moves, a shared modifier's release) are scaled to
/// fit, events after it shift by the difference. An event at `from` stays.
/// Monotone, so the events stay in order.
fn retime(events: &mut [Event], pauses: &[(Ms, Ms, Ms)]) {
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
/// insertion there lands after that step. A step ending at `Ms::MAX` can't
/// be passed: the insertion stops there.
fn snap_insertion(steps: &[Step], mut at: Ms) -> Ms {
    while at < Ms::MAX
        && let Some(s) = steps.iter().find(|s| s.t <= at && at <= s.end)
    {
        at = s.end.saturating_add(1);
    }
    at
}

fn insert_timed(events: &mut Vec<Event>, steps: &[Step], at: Ms, dur: Ms, make: impl FnOnce(Ms) -> Event) {
    let at = snap_insertion(steps, at);
    shift_from(events, at, dur as i64);
    // Nothing moves past `Ms::MAX`, so a wait there goes after what's already there.
    let pos = if at == Ms::MAX { events.len() } else { events.partition_point(|e| e.t() < at) };
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

/// Sorts events by time, moves anything that comes after a wait but before
/// its end to its end, drops releases whose press is missing (it happened before recording
/// started) and releases anything still held at the end.
pub fn normalize(events: &mut Vec<Event>) {
    events.sort_by_key(Event::t);
    // What comes after a wait happens once it's over (an event at the same
    // time is listed before it). Sorted, so pushing events to the end of the
    // wait before them keeps the order.
    let mut wait_end: Option<Ms> = None;
    for e in events.iter_mut() {
        if let Some(end) = wait_end
            && e.t() < end
        {
            *e.t_mut() = end;
        }
        if matches!(e, Event::Wait { .. } | Event::PixelWait { .. } | Event::FindImage { .. }) {
            wait_end = Some(e.end());
        }
    }
    // Presses still held, oldest first (only a handful at any time).
    let mut held: Vec<Press> = Vec::new();
    let mut cursor = (0, 0);
    events.retain(|e| {
        // A pixel check's position isn't the cursor's.
        if let Event::Move { x, y, .. } | Event::Button { x, y, .. } | Event::Wheel { x, y, .. } = e {
            cursor = (*x, *y);
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
    // Sorted, so an event during a wait is one listed after it, before its end.
    let mut wait: Option<(Ms, Ms)> = None;
    for e in events {
        if let Some((start, end)) = wait
            && e.t() < end
        {
            return Err(format!("{e:?} happens during the wait at {start}..{end}"));
        }
        if matches!(e, Event::Wait { .. } | Event::PixelWait { .. } | Event::FindImage { .. }) {
            wait = Some((e.t(), e.end()));
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
    fn normalize_moves_what_happens_during_a_wait_to_its_end() {
        let wait = |t, dur| Event::Wait { t, dur, label: String::new() };
        let pixel = Event::PixelWait {
            t: 2000,
            dur: 500,
            x: 1,
            y: 1,
            color: Rgb(1, 2, 3),
            tolerance: 8,
            timeout_ms: 5000,
            label: String::new(),
        };
        let mut ev = vec![
            Event::Move { t: 100, x: 1, y: 1 }, // listed before the wait: before it
            wait(100, 1000),
            wait(100, 10), // listed after: once the first is over
            Event::Move { t: 100, x: 1, y: 1 },
            key(400, "KeyA", true),
            key(450, "KeyA", false),
            wait(900, 300), // inside the first: pushed out, and it then ends later
            key(1150, "KeyB", true),
            key(1500, "KeyB", false),
            pixel,
            Event::Move { t: 2200, x: 2, y: 2 },
            Event::Move { t: 2500, x: 3, y: 3 },
        ];
        normalize(&mut ev);
        let ts: Vec<_> = ev.iter().map(Event::t).collect();
        assert_eq!(ts, [100, 100, 1100, 1110, 1110, 1110, 1110, 1410, 1500, 2000, 2500, 2500]);
        check_invariants(&ev).unwrap();
    }

    #[test]
    fn a_release_normalize_adds_is_where_the_cursor_was_not_on_a_pixel_check() {
        let pixel = Event::PixelWait {
            t: 100,
            dur: 0,
            x: 900,
            y: 900,
            color: Rgb(1, 2, 3),
            tolerance: 8,
            timeout_ms: 5000,
            label: String::new(),
        };
        let b = Event::Button { t: 0, x: 5, y: 5, btn: MouseBtn::Left, down: true, label: String::new() };
        let mut ev = vec![b, pixel];
        normalize(&mut ev);
        assert!(matches!(ev[2], Event::Button { x: 5, y: 5, down: false, .. }), "{ev:?}");
        let steps = group_steps(&ev, (&RecordingMeta::single_1080p()).into());
        assert!(matches!(steps[0].kind, StepKind::Click { .. }), "still a click, not a drag");
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
    fn pauses_are_idle_time_and_set_pause_moves_what_follows() {
        let mv = |t| Event::Move { t, x: t as i32, y: 0 };
        let mut m = mac([click(0).to_vec(), vec![mv(500), mv(1000), mv(1500)], click(2080).to_vec()].concat());
        let steps = group_steps(&m.events, (&m.recording).into());
        assert!(matches!(steps[1].kind, StepKind::Move { x: 5, y: 5, to_x: 1500, to_y: 0, samples: 3 }));
        // Idle before the move (80..500) and after it (1500..2080); moving isn't a pause.
        assert_eq!(steps.iter().map(|s| s.pause).collect::<Vec<_>>(), [0, 420, 580]);
        apply(&mut m, EditOp::SetPause { index: 2, dur: 100 }).unwrap();
        assert_eq!(m.events.iter().map(Event::t).collect::<Vec<_>>(), [0, 80, 500, 1000, 1500, 1600, 1680]);
        apply(&mut m, EditOp::SetPause { index: 1, dur: 0 }).unwrap();
        assert_eq!(m.events.iter().map(Event::t).collect::<Vec<_>>(), [0, 80, 80, 580, 1080, 1180, 1260]);
        check_invariants(&m.events).unwrap();
    }

    #[test]
    fn a_pause_added_where_there_was_none_leaves_the_move_before_alone() {
        // The last sample and the click at the same moment, as recordings have.
        let mv = |t| Event::Move { t, x: 5, y: t as i32 };
        let mut m = mac([vec![mv(0), mv(400)], click(400).to_vec()].concat());
        assert_eq!(steps(&m)[1].pause, 0);
        apply(&mut m, EditOp::SetPause { index: 1, dur: 300 }).unwrap();
        assert_eq!(m.events.iter().map(Event::t).collect::<Vec<_>>(), [0, 400, 700, 780]);
        assert_eq!(steps(&m)[1].pause, 300);
        check_invariants(&m.events).unwrap();
    }

    #[test]
    fn set_move_duration_retimes_the_samples_and_moves_what_follows() {
        let mv = |t| Event::Move { t, x: t as i32, y: 0 };
        let mut m = mac([click(0).to_vec(), vec![mv(500), mv(1000), mv(1500)], click(2080).to_vec()].concat());
        apply(&mut m, EditOp::SetMoveDuration { index: 1, dur: 500 }).unwrap();
        assert_eq!(m.events.iter().map(Event::t).collect::<Vec<_>>(), [0, 80, 500, 750, 1000, 1580, 1660]);
        // The path keeps its shape.
        assert!(matches!(m.events[3], Event::Move { x: 1000, .. }));
        apply(&mut m, EditOp::SetMoveDuration { index: 1, dur: 2000 }).unwrap();
        assert_eq!(m.events.iter().map(Event::t).collect::<Vec<_>>(), [0, 80, 500, 1500, 2500, 3080, 3160]);
        apply(&mut m, EditOp::SetMoveDuration { index: 1, dur: 0 }).unwrap();
        assert_eq!(m.events.iter().map(Event::t).collect::<Vec<_>>(), [0, 80, 500, 500, 500, 1080, 1160]);
        check_invariants(&m.events).unwrap();
        assert_eq!(steps(&m).len(), 3, "still one move");
        assert_eq!(apply(&mut m, EditOp::SetMoveDuration { index: 0, dur: 5 }), Err(EditError::WrongKind(0)));
        assert_eq!(apply(&mut m, EditOp::SetMoveDuration { index: 3, dur: 5 }), Err(EditError::NoSuchStep(3)));
        // A single sample is a jump: nothing to retime.
        let mut jump = mac([click(0).to_vec(), vec![mv(500)], click(1000).to_vec()].concat());
        let before = jump.events.clone();
        apply(&mut jump, EditOp::SetMoveDuration { index: 1, dur: 300 }).unwrap();
        assert_eq!(jump.events, before);
    }

    #[test]
    fn deleting_the_last_move_shortens_the_macro() {
        // The trip to Relay's Stop button, after the last click.
        let mv = |t, x| Event::Move { t, x, y: 0 };
        let mut m = mac([click(0).to_vec(), vec![mv(500, 100), mv(900, 400), mv(1300, 900)]].concat());
        assert_eq!(crate::timeline::duration(&m.events), 1300 + crate::timeline::TAIL_MS);
        let s = steps(&m);
        assert!(matches!(s[1].kind, StepKind::Move { to_x: 900, samples: 3, .. }));
        apply(&mut m, EditOp::DeleteStep { index: 1 }).unwrap();
        assert_eq!(m.events.len(), 2);
        assert_eq!(crate::timeline::duration(&m.events), 80 + crate::timeline::TAIL_MS);
    }

    #[test]
    fn smoothing_and_straightening_move_the_samples_in_between() {
        // From the click at (5, 5) to (205, 5), zigzagging 4 px either side.
        let zigzag = |i: u32| {
            let y = if i == 20 {
                5
            } else if i.is_multiple_of(2) {
                9
            } else {
                1
            };
            Event::Move { t: 100 + i * 16, x: 5 + i as i32 * 10, y }
        };
        let fresh = || mac([click(0).to_vec(), (1..=20).map(zigzag).collect(), click(1000).to_vec()].concat());
        let ys = |m: &Macro| m.events[2..22].iter().map(|e| e.pos().unwrap().1).collect::<Vec<_>>();

        let mut m = fresh();
        let before = m.events.clone();
        apply(&mut m, EditOp::StraightenMove { index: 1 }).unwrap();
        assert_eq!(ys(&m), [5; 20]);
        assert!(m.events.iter().zip(&before).all(|(a, b)| a.t() == b.t()), "the times stay");
        assert_eq!(m.events[21], before[21], "the end stays");

        let mut m = fresh();
        apply(&mut m, EditOp::SmoothMove { index: 1 }).unwrap();
        assert!(ys(&m).iter().all(|y| (4..=6).contains(y)), "the zigzag is gone: {:?}", ys(&m));
        assert_eq!(m.events[21], before[21], "the end stays");

        assert_eq!(apply(&mut m, EditOp::SmoothMove { index: 0 }), Err(EditError::WrongKind(0)));
        assert_eq!(apply(&mut m, EditOp::StraightenMove { index: 2 }), Err(EditError::WrongKind(2)));
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

        // Inserting inside a step that ends at the very last millisecond stops there.
        let b = |t, down| Event::Button { t, x: 5, y: 5, btn: MouseBtn::Left, down, label: String::new() };
        let mut m = mac(vec![b(Ms::MAX - 50, true), b(Ms::MAX, false)]);
        apply(&mut m, EditOp::InsertWait { at: Ms::MAX - 20, dur: 500, label: String::new() }).unwrap();
        let pixel = EditOp::InsertPixelWait {
            at: Ms::MAX - 50,
            dur: Ms::MAX,
            x: 1,
            y: 2,
            color: Rgb(1, 2, 3),
            tolerance: 8,
            timeout_ms: 5000,
            label: String::new(),
        };
        apply(&mut m, pixel).unwrap();
        assert!(matches!(m.events[1], Event::Button { t: Ms::MAX, down: false, .. }), "{:?}", m.events);
        assert!(matches!(m.events[2], Event::Wait { t: Ms::MAX, dur: 500, .. }));
        assert!(matches!(m.events[3], Event::PixelWait { t: Ms::MAX, dur: MAX_DUR, .. }));
        check_invariants(&m.events).unwrap();

        // A wait whose end saturated, with a key after it (saturated too):
        // shortening an earlier wait moves both back, and the key stays after the wait.
        let mut m = mac(vec![
            Event::Wait { t: 0, dur: 500, label: String::new() },
            Event::Wait { t: Ms::MAX - 100, dur: 1000, label: String::new() },
            key(Ms::MAX, "KeyA", true),
            key(Ms::MAX, "KeyA", false),
        ]);
        check_invariants(&m.events).unwrap();
        apply(&mut m, EditOp::SetWaitDuration { index: 0, dur: 300 }).unwrap();
        check_invariants(&m.events).unwrap();
        assert_eq!((m.events[1].t(), m.events[2].t()), (Ms::MAX - 300, Ms::MAX), "{:?}", m.events);
        let _ = crate::view::MacroView::of(&m);
    }

    fn pixel_at(at: Ms, dur: Ms) -> EditOp {
        EditOp::InsertPixelWait {
            at,
            dur,
            x: 10,
            y: 20,
            color: Rgb(1, 2, 3),
            tolerance: 8,
            timeout_ms: 5000,
            label: String::new(),
        }
    }
    fn steps(m: &Macro) -> Vec<Step> {
        group_steps(&m.events, (&m.recording).into())
    }

    #[test]
    fn a_pixel_check_is_inserted_like_a_wait_and_deleting_it_closes_the_gap() {
        let original = mac([click(0), click(1000)].concat());
        let mut m = original.clone();
        apply(&mut m, pixel_at(500, 700)).unwrap();
        let ts: Vec<_> = m.events.iter().map(Event::t).collect();
        assert_eq!(ts, [0, 80, 500, 1700, 1780]);
        assert!(matches!(m.events[2], Event::PixelWait { x: 10, y: 20, dur: 700, timeout_ms: 5000, .. }));
        check_invariants(&m.events).unwrap();
        apply(&mut m, EditOp::DeleteStep { index: 1 }).unwrap();
        assert_eq!(m.events, original.events);
    }

    #[test]
    fn updating_a_pixel_check_keeps_its_time_and_duration() {
        let mut m = mac([click(0), click(1000)].concat());
        apply(&mut m, pixel_at(500, 700)).unwrap();
        let update =
            |index| EditOp::UpdatePixelWait { index, x: -3, y: 4, color: Rgb(9, 8, 7), tolerance: 0, timeout_ms: 100 };
        apply(&mut m, update(1)).unwrap();
        assert_eq!(
            m.events[2],
            Event::PixelWait {
                t: 500,
                dur: 700,
                x: -3,
                y: 4,
                color: Rgb(9, 8, 7),
                tolerance: 0,
                timeout_ms: 100,
                label: String::new()
            }
        );
        assert_eq!(apply(&mut m, update(0)), Err(EditError::WrongKind(0)));
        assert_eq!(apply(&mut m, update(3)), Err(EditError::NoSuchStep(3)));
    }

    fn find_at(at: Ms, dur: Ms) -> EditOp {
        EditOp::InsertFindImage {
            at,
            dur,
            image: ImagePng(b"\x89PNG1".to_vec()),
            click_x: 50,
            click_y: 20,
            btn: MouseBtn::Left,
            threshold: 85,
            timeout_ms: 5000,
            area: None,
            label: String::new(),
        }
    }

    #[test]
    fn a_find_image_step_is_a_wait_that_can_be_retimed_updated_labeled_and_deleted() {
        let original = mac([click(0), click(1000)].concat());
        let mut m = original.clone();
        apply(&mut m, find_at(500, 700)).unwrap();
        assert_eq!(m.events.iter().map(Event::t).collect::<Vec<_>>(), [0, 80, 500, 1700, 1780]);
        check_invariants(&m.events).unwrap();
        let s = steps(&m);
        assert!(matches!(s[1].kind, StepKind::FindImage { dur: 700, click_x: 50, threshold: 85, .. }));
        assert_eq!((s[1].t, s[1].end), (500, 1200));

        apply(&mut m, EditOp::SetWaitDuration { index: 1, dur: 200 }).unwrap();
        assert_eq!(m.events[3].t(), 1200);
        apply(&mut m, EditOp::SetLabel { index: 1, label: "OK button".into() }).unwrap();
        let area = Some(Rect { x: -1920, y: 0, w: 800, h: 600 });
        let update = |index, threshold| EditOp::UpdateFindImage {
            index,
            image: ImagePng(b"\x89PNG2".to_vec()),
            click_x: -4,
            click_y: 3,
            btn: MouseBtn::Right,
            threshold,
            timeout_ms: 9000,
            area,
        };
        apply(&mut m, update(1, 99)).unwrap();
        assert_eq!(
            m.events[2],
            Event::FindImage {
                t: 500,
                dur: 200,
                image: ImagePng(b"\x89PNG2".to_vec()),
                click_x: -4,
                click_y: 3,
                btn: MouseBtn::Right,
                threshold: 99,
                timeout_ms: 9000,
                area,
                label: "OK button".into(),
            }
        );
        apply(&mut m, update(1, 5)).unwrap();
        assert!(matches!(m.events[2], Event::FindImage { threshold: MIN_THRESHOLD, .. }));
        assert_eq!(apply(&mut m, update(0, 90)), Err(EditError::WrongKind(0)));

        apply(&mut m, EditOp::DeleteStep { index: 1 }).unwrap();
        assert_eq!(m.events.iter().map(Event::t).collect::<Vec<_>>(), [0, 80, 1000, 1080]);
    }

    #[test]
    fn nothing_happens_during_a_find_image_step() {
        let mut ev = vec![
            Event::FindImage {
                t: 100,
                dur: 500,
                image: ImagePng(b"\x89PNG".to_vec()),
                click_x: 0,
                click_y: 0,
                btn: MouseBtn::Left,
                threshold: 85,
                timeout_ms: 5000,
                area: None,
                label: String::new(),
            },
            Event::Move { t: 300, x: 1, y: 1 },
        ];
        normalize(&mut ev);
        assert_eq!(ev[1].t(), 600);
        check_invariants(&ev).unwrap();
    }

    #[test]
    fn waits_and_pixel_checks_take_labels_but_typing_and_shortcuts_dont() {
        let typed = vec![
            Event::Key { t: 2000, down: true, key: KeyStroke::code("KeyA"), ch: Some("a".into()) },
            key(2040, "KeyA", false),
        ];
        let mut m = mac([click(0).to_vec(), typed].concat());
        apply(&mut m, EditOp::InsertWait { at: 500, dur: 300, label: String::new() }).unwrap();
        apply(&mut m, pixel_at(1000, 300)).unwrap();
        apply(&mut m, EditOp::SetLabel { index: 1, label: "Dialog".into() }).unwrap();
        apply(&mut m, EditOp::SetLabel { index: 2, label: "Red".into() }).unwrap();
        let s = steps(&m);
        assert!(matches!(&s[1].kind, StepKind::Wait { label, .. } if label == "Dialog"));
        assert!(matches!(&s[2].kind, StepKind::PixelWait { label, .. } if label == "Red"));
        assert!(matches!(s[3].kind, StepKind::Type { .. }));
        assert_eq!(apply(&mut m, EditOp::SetLabel { index: 3, label: "x".into() }), Err(EditError::WrongKind(3)));
        let mut m = mac(vec![
            key(0, "ControlLeft", true),
            key(10, "KeyS", true),
            key(20, "KeyS", false),
            key(30, "ControlLeft", false),
        ]);
        assert_eq!(apply(&mut m, EditOp::SetLabel { index: 0, label: "x".into() }), Err(EditError::WrongKind(0)));
    }

    #[test]
    fn the_first_steps_pause_counts_from_the_start() {
        let mv = |t| Event::Move { t, x: t as i32, y: 0 };
        let mut m = mac([vec![mv(300), mv(500)], click(1000).to_vec()].concat());
        assert_eq!(steps(&m)[0].pause, 300);
        apply(&mut m, EditOp::SetPause { index: 0, dur: 100 }).unwrap();
        assert_eq!(m.events.iter().map(Event::t).collect::<Vec<_>>(), [100, 300, 800, 880]);
        apply(&mut m, EditOp::SetPause { index: 0, dur: 0 }).unwrap();
        assert_eq!(m.events.iter().map(Event::t).collect::<Vec<_>>(), [0, 200, 700, 780]);
        check_invariants(&m.events).unwrap();
        // A pause of 0 between two steps.
        let mut m = mac([click(0), click(3000)].concat());
        apply(&mut m, EditOp::SetPause { index: 1, dur: 0 }).unwrap();
        assert_eq!(m.events.iter().map(Event::t).collect::<Vec<_>>(), [0, 80, 80, 160]);
    }

    #[test]
    fn capping_pauses_can_join_typed_text() {
        // Intended: with the pause gone the characters were typed without a
        // break, so they're one TYPE step. The keystrokes replayed are the same.
        let typed = |t, code: &str, c: &str| {
            [Event::Key { t, down: true, key: KeyStroke::code(code), ch: Some(c.into()) }, key(t + 40, code, false)]
        };
        let mut m = mac([typed(0, "KeyA", "a"), typed(3000, "KeyB", "b")].concat());
        assert_eq!(steps(&m).len(), 2);
        apply(&mut m, EditOp::CapPauses { max: 100 }).unwrap();
        let s = steps(&m);
        assert_eq!(s.len(), 1);
        assert!(matches!(&s[0].kind, StepKind::Type { text, .. } if text == "ab"));
        check_invariants(&m.events).unwrap();
    }

    #[test]
    fn deleting_one_of_two_shortcuts_keeps_the_shared_ctrl() {
        let mut m = mac(vec![
            key(0, "ControlLeft", true),
            key(50, "KeyC", true),
            key(90, "KeyC", false),
            key(150, "KeyV", true),
            key(190, "KeyV", false),
            key(220, "ControlLeft", false),
        ]);
        apply(&mut m, EditOp::DeleteStep { index: 0 }).unwrap();
        assert_eq!(m.events.len(), 4);
        let s = steps(&m);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].kind, StepKind::Keys { combo: vec!["Ctrl".into(), "V".into()] });
        assert_eq!(s[0].items, vec![0, 1, 2, 3], "and the Ctrl is now V's alone");
    }

    #[test]
    fn labels_live_on_the_press() {
        let mut m = mac(click(0).to_vec());
        apply(&mut m, EditOp::SetLabel { index: 0, label: "Save".into() }).unwrap();
        assert!(matches!(&m.events[0], Event::Button { label, .. } if label == "Save"));
    }

    #[test]
    fn a_click_owning_its_modifier_is_labeled_on_the_press() {
        let b = |t, x, down| Event::Button { t, x, y: 5, btn: MouseBtn::Left, down, label: String::new() };
        let shift_click =
            vec![key(0, "ShiftLeft", true), b(50, 5, true), b(90, 5, false), key(200, "ShiftLeft", false)];
        let ctrl_drag =
            vec![key(0, "ControlLeft", true), b(50, 5, true), b(400, 300, false), key(500, "ControlLeft", false)];
        for events in [shift_click, ctrl_drag] {
            let mut m = mac(events);
            apply(&mut m, EditOp::SetLabel { index: 0, label: "Pick".into() }).unwrap();
            assert!(
                matches!(&m.events[1], Event::Button { label, down: true, .. } if label == "Pick"),
                "{:?}",
                m.events
            );
            let kind = &group_steps(&m.events, (&m.recording).into())[0].kind;
            assert!(matches!(kind, StepKind::Click { label, .. } | StepKind::Drag { label, .. } if label == "Pick"));
        }
    }
}
