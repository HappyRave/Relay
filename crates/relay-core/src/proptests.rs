//! Property tests: arbitrary recordings and arbitrary edit sequences must keep
//! the event invariants, grouping must assign each event to at most one step,
//! each edit must do what it says, and humanized play times must keep steps whole.

use std::collections::HashSet;

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

use crate::edit::{EditError, EditOp, MAX_DUR, apply, check_invariants};
use crate::keys::KeyStroke;
use crate::model::{Event, Macro, MouseBtn, Ms, RecordingMeta, Rgb};
use crate::playback::plan_times;
use crate::steps::{GroupOptions, Step, StepKind, group_steps};
use crate::timeline::duration;

#[derive(Debug, Clone)]
enum Action {
    Click {
        x: i32,
        y: i32,
        btn: MouseBtn,
    },
    /// Two clicks on the same spot, quickly enough to be a double click.
    DoubleClick {
        x: i32,
        y: i32,
    },
    Drag {
        x: i32,
        y: i32,
    },
    /// A click or a drag with a modifier held around it (Shift-click, Ctrl-drag).
    ModClick {
        modifier: &'static str,
        x: i32,
        y: i32,
        drag: bool,
    },
    Tap {
        code: &'static str,
        ch: Option<&'static str>,
    },
    Combo {
        modifier: &'static str,
        code: &'static str,
    },
    /// A key pressed before a click and released after it.
    HeldAcrossClick {
        code: &'static str,
        ch: Option<&'static str>,
        x: i32,
        y: i32,
    },
    Wheel {
        delta: i32,
        horizontal: bool,
    },
    Move {
        x: i32,
        y: i32,
    },
    Wait {
        dur: u32,
    },
    /// Skips ahead to a huge time, so the arithmetic near `Ms::MAX` is exercised.
    Jump {
        to: Ms,
    },
}

fn action() -> impl Strategy<Value = Action> {
    let pos = (-1920..1920i32, -200..1080i32);
    let modifier = || prop::sample::select(vec!["ControlLeft", "AltLeft", "ShiftLeft", "MetaLeft"]);
    let typed = || {
        prop::sample::select(vec![
            ("KeyA", Some("a")),
            ("KeyB", Some("b")),
            ("Enter", Some("\r")),
            ("Tab", None),
            ("F2", None),
        ])
    };
    prop_oneof![
        3 => (pos.clone(), prop::sample::select(vec![MouseBtn::Left, MouseBtn::Right, MouseBtn::Middle]))
            .prop_map(|((x, y), btn)| Action::Click { x, y, btn }),
        1 => pos.clone().prop_map(|(x, y)| Action::DoubleClick { x, y }),
        1 => pos.clone().prop_map(|(x, y)| Action::Drag { x, y }),
        1 => (modifier(), pos.clone(), any::<bool>()).prop_map(|(modifier, (x, y), drag)| Action::ModClick {
            modifier,
            x,
            y,
            drag
        }),
        3 => typed().prop_map(|(code, ch)| Action::Tap { code, ch }),
        1 => (modifier(), prop::sample::select(vec!["KeyS", "KeyW", "Tab"]))
            .prop_map(|(modifier, code)| Action::Combo { modifier, code }),
        1 => (typed(), pos.clone()).prop_map(|((code, ch), (x, y))| Action::HeldAcrossClick { code, ch, x, y }),
        1 => (prop::sample::select(vec![-120, 120]), any::<bool>())
            .prop_map(|(delta, horizontal)| Action::Wheel { delta, horizontal }),
        2 => pos.prop_map(|(x, y)| Action::Move { x, y }),
        1 => (0..2000u32).prop_map(|dur| Action::Wait { dur }),
        1 => prop_oneof![Just(1 << 31), (Ms::MAX - 20_000)..Ms::MAX].prop_map(|to| Action::Jump { to }),
    ]
}

/// Turns actions into a balanced, sorted event list with gaps between them.
fn record(actions: &[(Action, u32)]) -> Vec<Event> {
    let mut ev = Vec::new();
    let mut t = 0u32;
    let key = |t, code: &str, down, ch: Option<&str>| Event::Key {
        t,
        down,
        key: KeyStroke::code(code),
        ch: ch.map(Into::into),
    };
    let btn = |t, x, y, btn, down| Event::Button { t, x, y, btn, down, label: String::new() };
    let left = MouseBtn::Left;
    for (a, gap) in actions {
        let at = |d: u32| t.saturating_add(d);
        match a {
            Action::Click { x, y, btn: b } => ev.extend([btn(t, *x, *y, *b, true), btn(at(60), *x, *y, *b, false)]),
            Action::DoubleClick { x, y } => ev.extend([
                btn(t, *x, *y, left, true),
                btn(at(60), *x, *y, left, false),
                btn(at(150), *x, *y, left, true),
                btn(at(210), *x, *y, left, false),
            ]),
            Action::Drag { x, y } => ev.extend([
                btn(t, *x, *y, left, true),
                Event::Move { t: at(100), x: x + 50, y: *y },
                btn(at(200), x + 50, *y, left, false),
            ]),
            Action::ModClick { modifier, x, y, drag } => {
                let to = if *drag { x + 80 } else { *x };
                ev.extend([
                    key(t, modifier, true, None),
                    btn(at(30), *x, *y, left, true),
                    btn(at(120), to, *y, left, false),
                    key(at(160), modifier, false, None),
                ]);
            }
            Action::Tap { code, ch } => ev.extend([key(t, code, true, *ch), key(at(40), code, false, None)]),
            Action::Combo { modifier, code } => ev.extend([
                key(t, modifier, true, None),
                key(at(20), code, true, None),
                key(at(60), code, false, None),
                key(at(80), modifier, false, None),
            ]),
            Action::HeldAcrossClick { code, ch, x, y } => ev.extend([
                key(t, code, true, *ch),
                btn(at(50), *x, *y, left, true),
                btn(at(110), *x, *y, left, false),
                key(at(200), code, false, None),
            ]),
            Action::Wheel { delta, horizontal } => {
                ev.push(Event::Wheel { t, x: 0, y: 0, delta: *delta, horizontal: *horizontal })
            }
            Action::Move { x, y } => ev.push(Event::Move { t, x: *x, y: *y }),
            Action::Wait { dur } => {
                // A cursor sample at the same time as the wait, as recordings have.
                ev.push(Event::Move { t, x: 1, y: 1 });
                ev.push(Event::Wait { t, dur: *dur, label: String::new() });
                t = at(*dur);
            }
            Action::Jump { to } => t = t.max(*to),
        }
        t = t.saturating_add(250 + gap);
    }
    ev
}

fn edit(steps: usize, dur: u32) -> impl Strategy<Value = EditOp> {
    let idx = 0..(steps as u32 + 2);
    let at = 0..dur.saturating_add(500);
    prop_oneof![
        idx.clone().prop_map(|index| EditOp::DeleteStep { index }),
        (at.clone(), 0..3000u32).prop_map(|(at, dur)| EditOp::InsertWait { at, dur, label: String::new() }),
        (at, 1..3000u32).prop_map(|(at, dur)| EditOp::InsertPixelWait {
            at,
            dur,
            x: 1,
            y: 2,
            color: Rgb(1, 2, 3),
            tolerance: 8,
            timeout_ms: 5000,
            label: String::new()
        }),
        (idx.clone(), 0..3000u32).prop_map(|(index, dur)| EditOp::SetWaitDuration { index, dur }),
        idx.clone().prop_map(|index| EditOp::UpdatePixelWait {
            index,
            x: -4,
            y: 5,
            color: Rgb(9, 9, 9),
            tolerance: 0,
            timeout_ms: 100
        }),
        idx.clone().prop_map(|index| EditOp::SetLabel { index, label: "x".into() }),
        (idx.clone(), 0..3000u32).prop_map(|(index, dur)| EditOp::SetPause { index, dur }),
        (idx, 0..3u32).prop_map(|(index, dur)| EditOp::SetWaitDuration { index, dur }),
        (0..1500u32).prop_map(|max| EditOp::CapPauses { max }),
        Just(EditOp::Rename { name: "renamed".into() }),
    ]
}

fn macro_and_edits() -> impl Strategy<Value = (Vec<Event>, Vec<EditOp>)> {
    prop::collection::vec((action(), 0..600u32), 0..25).prop_flat_map(|actions| {
        let events = record(&actions);
        let steps = group_steps(&events, GroupOptions::default()).len();
        let ops = prop::collection::vec(edit(steps, duration(&events)), 0..12);
        (Just(events), ops)
    })
}

/// What `apply` must answer for `op` on a macro with these steps.
fn expected(op: &EditOp, steps: &[Step]) -> Result<(), EditError> {
    let kind = |index: u32| steps.get(index as usize).map(|s| &s.kind).ok_or(EditError::NoSuchStep(index));
    let only = |index: u32, ok: fn(&StepKind) -> bool| {
        if ok(kind(index)?) { Ok(()) } else { Err(EditError::WrongKind(index)) }
    };
    match *op {
        EditOp::Rename { .. }
        | EditOp::InsertWait { .. }
        | EditOp::InsertPixelWait { .. }
        | EditOp::CapPauses { .. } => Ok(()),
        EditOp::DeleteStep { index } | EditOp::SetPause { index, .. } => kind(index).map(|_| ()),
        EditOp::SetWaitDuration { index, .. } => {
            only(index, |k| matches!(k, StepKind::Wait { .. } | StepKind::PixelWait { .. }))
        }
        EditOp::UpdatePixelWait { index, .. } => only(index, |k| matches!(k, StepKind::PixelWait { .. })),
        EditOp::SetLabel { index, .. } => only(index, |k| {
            matches!(
                k,
                StepKind::Click { .. } | StepKind::Drag { .. } | StepKind::Wait { .. } | StepKind::PixelWait { .. }
            )
        }),
    }
}

/// A step's kind without the times of its characters, which an insertion before them moves.
fn untimed(kind: &StepKind) -> StepKind {
    match kind {
        StepKind::Type { text, .. } => StepKind::Type { text: text.clone(), chars: Vec::new() },
        k => k.clone(),
    }
}

/// Whether adding `dur` to every event could reach `Ms::MAX`, where times saturate and bunch up.
fn saturates(events: &[Event], dur: Ms) -> bool {
    events.iter().map(Event::end).max().unwrap_or(0) as u64 + dur as u64 >= Ms::MAX as u64
}

/// The properties of one successful edit, from the events and steps before it.
fn check_op(op: &EditOp, before: &[Event], steps: &[Step], after: &[Event]) -> Result<(), TestCaseError> {
    let after_steps = group_steps(after, GroupOptions::default());
    match op {
        EditOp::DeleteStep { index } => {
            let step = &steps[*index as usize];
            if matches!(step.kind, StepKind::Wait { .. } | StepKind::PixelWait { .. }) {
                return Ok(());
            }
            // Exactly its events go; the others keep their times.
            let kept: Vec<Event> = before
                .iter()
                .enumerate()
                .filter(|(i, _)| !step.items.contains(&(*i as u32)))
                .map(|(_, e)| e.clone())
                .collect();
            prop_assert_eq!(after, &kept[..]);
            // One step less, or two when its neighbours now merge (two clicks into a double click).
            let n = after_steps.len();
            prop_assert!(n + 1 == steps.len() || n + 2 == steps.len(), "{} steps, then {}", steps.len(), n);
        }
        EditOp::InsertWait { dur, label, .. } | EditOp::InsertPixelWait { dur, label, .. } => {
            if saturates(before, *dur) {
                return Ok(());
            }
            let p = after.iter().position(
                |e| matches!(e, Event::Wait { label: l, .. } | Event::PixelWait { label: l, .. } if l == label),
            );
            let p = p.expect("the inserted wait") as u32;
            let new = after_steps.iter().position(|s| s.items == [p]).expect("the inserted wait's step");
            let dur = (*dur).min(MAX_DUR);
            let (StepKind::Wait { dur: d, .. } | StepKind::PixelWait { dur: d, .. }) = after_steps[new].kind else {
                return Err(TestCaseError::fail(format!("{:?}", after_steps[new])));
            };
            prop_assert_eq!(d, dur);
            // Every other step keeps its kind and its events.
            let others: Vec<_> = after_steps.iter().enumerate().filter(|(i, _)| *i != new).map(|(_, s)| s).collect();
            prop_assert_eq!(others.len(), steps.len());
            for (a, b) in others.into_iter().zip(steps) {
                prop_assert_eq!(untimed(&a.kind), untimed(&b.kind));
                let moved: Vec<u32> = b.items.iter().map(|&i| if i >= p { i + 1 } else { i }).collect();
                prop_assert_eq!(&a.items, &moved);
            }
        }
        EditOp::SetPause { index, dur } => {
            let i = *index as usize;
            // Only a real pause can be retimed: a step that overlaps the one before has none.
            if steps[i].pause > 0 && !saturates(before, *dur) && after_steps.len() == steps.len() {
                prop_assert_eq!(after_steps[i].pause, (*dur).min(MAX_DUR));
            }
        }
        EditOp::CapPauses { max } => {
            for s in &after_steps {
                prop_assert!(s.pause <= *max, "{:?}", s);
            }
        }
        _ => {}
    }
    Ok(())
}

proptest! {
    #[test]
    fn recordings_satisfy_the_invariants(actions in prop::collection::vec((action(), 0..600u32), 0..40)) {
        let events = record(&actions);
        prop_assert_eq!(check_invariants(&events), Ok(()));
    }

    #[test]
    fn steps_own_disjoint_valid_events(actions in prop::collection::vec((action(), 0..600u32), 0..40)) {
        let events = record(&actions);
        let steps = group_steps(&events, GroupOptions::default());
        let mut seen = HashSet::new();
        let mut busy_until = 0;
        for s in &steps {
            prop_assert!(s.t <= s.end);
            for &i in &s.items {
                prop_assert!((i as usize) < events.len());
                let e = &events[i as usize];
                prop_assert!(!matches!(e, Event::Move { .. }), "a move is a step item");
                prop_assert!(s.t <= e.t() && e.end() <= s.end, "event {} is outside its step {:?}", i, s);
                prop_assert!(seen.insert(i), "event {} is in two steps", i);
            }
            // The idle time since every earlier step ended.
            prop_assert_eq!(s.pause, s.t.saturating_sub(busy_until));
            busy_until = busy_until.max(s.end);
        }
        prop_assert!(steps.windows(2).all(|w| w[0].t <= w[1].t));
    }

    #[test]
    fn edits_do_what_they_say_and_preserve_the_invariants((events, ops) in macro_and_edits()) {
        let mut m = Macro::new("p", RecordingMeta::single_1080p(), events);
        for (n, mut op) in ops.into_iter().enumerate() {
            // A label no other wait has, to find the one inserted.
            if let EditOp::InsertWait { label, .. } | EditOp::InsertPixelWait { label, .. } = &mut op {
                *label = format!("inserted {n}");
            }
            let before = m.events.clone();
            let steps = group_steps(&before, (&m.recording).into());
            let result = apply(&mut m, op.clone());
            prop_assert_eq!(&result, &expected(&op, &steps), "{:?}", op);
            prop_assert_eq!(check_invariants(&m.events), Ok(()), "after {:?}", op);
            if result.is_ok() {
                check_op(&op, &before, &steps, &m.events)?;
            } else {
                prop_assert_eq!(&m.events, &before, "a failed edit changes nothing");
            }
        }
    }

    #[test]
    fn deleting_every_step_leaves_only_moves((events, _) in macro_and_edits()) {
        let mut m = Macro::new("p", RecordingMeta::single_1080p(), events);
        while !group_steps(&m.events, (&m.recording).into()).is_empty() {
            apply(&mut m, EditOp::DeleteStep { index: 0 }).unwrap();
        }
        prop_assert!(m.events.iter().all(|e| matches!(e, Event::Move { .. })), "{:?}", m.events);
    }

    #[test]
    fn humanized_times_keep_steps_whole(
        actions in prop::collection::vec((action(), 0..600u32), 0..40),
        jitter in 0..200u32,
        seed in any::<u64>(),
    ) {
        let events = record(&actions);
        let steps = group_steps(&events, GroupOptions::default());
        let plan = plan_times(&events, &steps, jitter, seed);
        prop_assert_eq!(plan.len(), events.len());
        prop_assert!(plan.windows(2).all(|w| w[0] <= w[1]), "not ordered");
        prop_assert!(plan.iter().all(|&p| p >= 0.0 && p.is_finite()));
        for s in &steps {
            let first = s.items[0] as usize;
            let d = plan[first] - events[first].t() as f64;
            for &i in &s.items {
                let i = i as usize;
                // Relative to the magnitude: times near Ms::MAX are large floats.
                let err = (plan[i] - events[i].t() as f64 - d).abs();
                let tolerance = 1e-6 * (1.0 + events[i].t() as f64 / 1e6);
                prop_assert!(err <= tolerance, "event {} of {:?} is off by {}", i, s, err);
            }
        }
    }
}
