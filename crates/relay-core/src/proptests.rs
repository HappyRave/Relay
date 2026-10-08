//! Property tests: arbitrary recordings and arbitrary edit sequences must keep
//! the event invariants, grouping must assign each event to at most one step,
//! each edit must do what it says, and humanized play times must keep steps whole.

use std::collections::HashSet;

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;

use crate::edit::{EditError, EditOp, MAX_DUR, apply, check_invariants};
use crate::keys::KeyStroke;
use crate::model::{Event, ImagePng, Macro, MouseBtn, Ms, RecordingMeta, Rgb};
use crate::playback::plan_times;
use crate::splice::Splice;
use crate::steps::{GroupOptions, Step, StepKind, group_steps};
use crate::text;
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
    /// The cursor going through a few points, a sample every 16 ms.
    Move {
        path: Vec<(i32, i32)>,
    },
    Wait {
        dur: u32,
    },
    /// A Text step, as Make editable or + Type text leave one.
    Text {
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
        2 => prop::collection::vec(pos, 1..8).prop_map(|path| Action::Move { path }),
        1 => (0..2000u32).prop_map(|dur| Action::Wait { dur }),
        1 => (0..2000u32).prop_map(|dur| Action::Text { dur }),
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
                // The hand shakes a pixel between the two clicks.
                Event::Move { t: at(100), x: x + 1, y: *y },
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
            Action::Move { path } => {
                ev.extend(path.iter().enumerate().map(|(i, &(x, y))| Event::Move { t: at(i as u32 * 16), x, y }));
                t = at(path.len() as u32 * 16);
            }
            Action::Wait { dur } => {
                // A cursor sample at the same time as the wait, as recordings have.
                ev.push(Event::Move { t, x: 1, y: 1 });
                ev.push(Event::Wait { t, dur: *dur, label: String::new() });
                t = at(*dur);
            }
            Action::Text { dur } => {
                ev.push(Event::Text { t, dur: *dur, text: "No. {n}".into() });
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
        (at.clone(), 1..3000u32).prop_map(|(at, dur)| EditOp::InsertFindImage {
            at,
            dur,
            image: ImagePng(b"\x89PNG".to_vec()),
            click_x: 3,
            click_y: 4,
            btn: MouseBtn::Left,
            threshold: 85,
            timeout_ms: 5000,
            area: None,
            label: String::new()
        }),
        idx.clone().prop_map(|index| EditOp::UpdateFindImage {
            index,
            image: ImagePng(b"\x89PNG".to_vec()),
            click_x: -1,
            click_y: 0,
            btn: MouseBtn::Right,
            threshold: 70,
            timeout_ms: 100,
            area: None
        }),
        (at.clone(), 1..3000u32).prop_map(|(at, dur)| EditOp::InsertPixelWait {
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
        (idx.clone(), 0..3u32).prop_map(|(index, dur)| EditOp::SetWaitDuration { index, dur }),
        (0..1500u32).prop_map(|max| EditOp::CapPauses { max }),
        (idx.clone(), 0..3000u32).prop_map(|(index, dur)| EditOp::SetMoveDuration { index, dur }),
        idx.clone().prop_map(|index| EditOp::SmoothMove { index }),
        idx.clone().prop_map(|index| EditOp::StraightenMove { index }),
        (at.clone(), prop::sample::select(vec!["", "{date} {{x}}", "bad {"]))
            .prop_map(|(at, text)| EditOp::InsertText { at, text: text.into() }),
        (idx.clone(), prop::sample::select(vec!["short", "a much longer text {time} than before", "}"]))
            .prop_map(|(index, text)| EditOp::UpdateText { index, text: text.into() }),
        idx.prop_map(|index| EditOp::MakeEditable { index }),
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
    let template = |t: &str| text::validate(t).map_err(EditError::Text);
    match op {
        EditOp::InsertText { text, .. } => template(text),
        EditOp::UpdateText { index, text } => {
            only(*index, |k| matches!(k, StepKind::Text { .. }))?;
            template(text)
        }
        EditOp::MakeEditable { index } => only(*index, |k| matches!(k, StepKind::Type { .. })),
        EditOp::Rename { .. }
        | EditOp::InsertWait { .. }
        | EditOp::InsertPixelWait { .. }
        | EditOp::InsertFindImage { .. }
        | EditOp::CapPauses { .. } => Ok(()),
        EditOp::DeleteStep { index } | EditOp::SetPause { index, .. } => kind(*index).map(|_| ()),
        EditOp::SetWaitDuration { index, .. } => only(*index, |k| {
            matches!(
                k,
                StepKind::Wait { .. } | StepKind::PixelWait { .. } | StepKind::FindImage { .. } | StepKind::Text { .. }
            )
        }),
        EditOp::UpdatePixelWait { index, .. } => only(*index, |k| matches!(k, StepKind::PixelWait { .. })),
        EditOp::UpdateFindImage { index, .. } => only(*index, |k| matches!(k, StepKind::FindImage { .. })),
        EditOp::SetMoveDuration { index, .. } | EditOp::SmoothMove { index } | EditOp::StraightenMove { index } => {
            only(*index, |k| matches!(k, StepKind::Move { .. }))
        }
        EditOp::SetLabel { index, .. } => only(*index, |k| {
            matches!(
                k,
                StepKind::Click { .. }
                    | StepKind::Drag { .. }
                    | StepKind::Wait { .. }
                    | StepKind::PixelWait { .. }
                    | StepKind::FindImage { .. }
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
            if matches!(
                step.kind,
                StepKind::Wait { .. } | StepKind::PixelWait { .. } | StepKind::FindImage { .. } | StepKind::Text { .. }
            ) {
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
            // At least one step less: more when its neighbours now merge (two
            // clicks into a double click, the moves on either side into one).
            prop_assert!(after_steps.len() < steps.len(), "{} steps, then {}", steps.len(), after_steps.len());
        }
        EditOp::InsertWait { dur, label, .. }
        | EditOp::InsertPixelWait { dur, label, .. }
        | EditOp::InsertFindImage { dur, label, .. } => {
            if saturates(before, *dur) {
                return Ok(());
            }
            let p = after.iter().position(|e| {
                matches!(
                    e,
                    Event::Wait { label: l, .. } | Event::PixelWait { label: l, .. } | Event::FindImage { label: l, .. }
                        if l == label
                )
            });
            let p = p.expect("the inserted wait") as u32;
            let new = after_steps.iter().position(|s| s.items == [p]).expect("the inserted wait's step");
            let dur = (*dur).min(MAX_DUR);
            let (StepKind::Wait { dur: d, .. }
            | StepKind::PixelWait { dur: d, .. }
            | StepKind::FindImage { dur: d, .. }) = after_steps[new].kind
            else {
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
        EditOp::InsertText { text, .. } => {
            let dur = text::typing_ms(text);
            if saturates(before, dur) {
                return Ok(());
            }
            let p = after.iter().position(|e| matches!(e, Event::Text { text: t, .. } if t == text));
            let p = p.expect("the inserted text") as u32;
            let new = after_steps.iter().find(|s| s.items == [p]).expect("the inserted text's step");
            prop_assert_eq!(&new.kind, &StepKind::Text { dur, text: text.clone() });
            prop_assert_eq!(after_steps.len(), steps.len() + 1);
        }
        EditOp::UpdateText { index, text } => {
            let step = &steps[*index as usize];
            let StepKind::Text { dur, .. } = step.kind else { unreachable!() };
            let i = step.items[0] as usize;
            let want = dur.max(text::typing_ms(text));
            prop_assert_eq!(&after[i], &Event::Text { t: before[i].t(), dur: want, text: text.clone() });
        }
        EditOp::MakeEditable { index } => {
            let step = &steps[*index as usize];
            let StepKind::Type { text: typed, .. } = &step.kind else { unreachable!() };
            if saturates(before, step.end - step.t) {
                return Ok(());
            }
            // One Text step in its place, typing the same, from when it started.
            let made = after_steps.iter().find(|s| s.t == step.t && matches!(s.kind, StepKind::Text { .. }));
            let made = made.expect("the Text step");
            let StepKind::Text { dur, text: template } = &made.kind else { unreachable!() };
            prop_assert_eq!(&text::fill(template, 1, chrono::NaiveDateTime::default(), || None, |_| None), typed);
            prop_assert!(*dur >= step.end - step.t);
            let typing = after_steps.iter().filter(|s| matches!(s.kind, StepKind::Type { .. })).count();
            let typing_before = steps.iter().filter(|s| matches!(s.kind, StepKind::Type { .. })).count();
            prop_assert!(typing < typing_before);
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
        EditOp::SetMoveDuration { index, dur } => {
            let step = &steps[*index as usize];
            let (first, last) = (step.items[0] as usize, *step.items.last().unwrap() as usize);
            // The move starts when it did and lasts `dur`; the rest keep their order.
            prop_assert_eq!(after.len(), before.len());
            prop_assert_eq!(after[first].t(), before[first].t());
            if step.end > step.t && !saturates(before, *dur) {
                prop_assert_eq!(after[last].t(), step.t + (*dur).min(MAX_DUR));
            }
            // Events that saturated at `Ms::MAX` are normalized after the edit: moved
            // back, they may land inside a wait and go to its end.
            let delta = after[last].t() as i64 - before[last].t() as i64;
            for i in last + 1..before.len() {
                if before[i].t() > step.end && !saturates(before, 0) {
                    prop_assert_eq!(after[i].t() as i64, (before[i].t() as i64 + delta).min(Ms::MAX as i64));
                }
            }
        }
        EditOp::SmoothMove { index } | EditOp::StraightenMove { index } => {
            // Only the samples in between move; nothing changes time.
            let step = &steps[*index as usize];
            let last = *step.items.last().unwrap();
            for (i, (a, b)) in before.iter().zip(after).enumerate() {
                let i = i as u32;
                if step.items.contains(&i) && i != last {
                    prop_assert!(matches!(b, Event::Move { .. }) && a.t() == b.t(), "{:?} → {:?}", a, b);
                } else {
                    prop_assert_eq!(a, b);
                }
            }
            prop_assert_eq!(after_steps.len(), steps.len());
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
                prop_assert!(s.t <= e.t() && e.end() <= s.end, "event {} is outside its step {:?}", i, s);
                prop_assert!(seen.insert(i), "event {} is in two steps", i);
            }
            // The idle time since every earlier step ended.
            prop_assert_eq!(s.pause, s.t.saturating_sub(busy_until));
            busy_until = busy_until.max(s.end);
        }
        prop_assert!(steps.windows(2).all(|w| w[0].t <= w[1].t));
        // Every cursor move belongs to a step: a press's, or a MOVE.
        for (i, e) in events.iter().enumerate() {
            prop_assert!(!matches!(e, Event::Move { .. }) || seen.contains(&(i as u32)), "move {} is in no step", i);
        }
        for s in &steps {
            if let StepKind::Move { samples, .. } = s.kind {
                let only_moves = s.items.iter().all(|&i| matches!(events[i as usize], Event::Move { .. }));
                prop_assert!(only_moves, "{:?}", s);
                prop_assert_eq!(samples as usize, s.items.len());
            }
        }
    }

    #[test]
    fn edits_do_what_they_say_and_preserve_the_invariants((events, ops) in macro_and_edits()) {
        let mut m = Macro::new("p", RecordingMeta::single_1080p(), events);
        for (n, mut op) in ops.into_iter().enumerate() {
            // A label no other wait has, to find the one inserted.
            if let EditOp::InsertWait { label, .. }
            | EditOp::InsertPixelWait { label, .. }
            | EditOp::InsertFindImage { label, .. } = &mut op
            {
                *label = format!("inserted {n}");
            }
            if let EditOp::InsertText { text, .. } = &mut op
                && text != "bad {"
            {
                *text = format!("inserted {n} {text}");
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
    fn every_edit_undoes_and_redoes_from_its_splice((events, ops) in macro_and_edits()) {
        let mut m = Macro::new("p", RecordingMeta::single_1080p(), events);
        let mut history = Vec::new();
        for op in ops {
            let before = m.events.clone();
            if apply(&mut m, op).is_ok() {
                history.push((Splice::between(&before, &m.events), before, m.events.clone()));
            }
        }
        // Undo all of them, newest first, then redo them all.
        let mut redo = Vec::new();
        for (splice, before, after) in history.into_iter().rev() {
            prop_assert_eq!(&m.events, &after);
            redo.push((splice.revert(&mut m.events), after));
            prop_assert_eq!(&m.events, &before);
        }
        for (splice, after) in redo.into_iter().rev() {
            splice.revert(&mut m.events);
            prop_assert_eq!(&m.events, &after);
        }
    }

    #[test]
    fn deleting_every_step_leaves_nothing((events, _) in macro_and_edits()) {
        let mut m = Macro::new("p", RecordingMeta::single_1080p(), events);
        while !group_steps(&m.events, (&m.recording).into()).is_empty() {
            apply(&mut m, EditOp::DeleteStep { index: 0 }).unwrap();
        }
        prop_assert!(m.events.is_empty(), "{:?}", m.events);
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
        // The cursor path between steps follows them; the rest move as a whole.
        for s in steps.iter().filter(|s| !matches!(s.kind, StepKind::Move { .. })) {
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
