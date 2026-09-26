//! Property tests: arbitrary recordings and arbitrary edit sequences must keep
//! the event invariants, and grouping must assign each event to at most one step.

use std::collections::HashSet;

use proptest::prelude::*;

use crate::edit::{EditOp, apply, check_invariants};
use crate::keys::KeyStroke;
use crate::model::{Event, Macro, MouseBtn, RecordingMeta, Rgb};
use crate::steps::{GroupOptions, group_steps};
use crate::timeline::duration;

#[derive(Debug, Clone)]
enum Action {
    Click { x: i32, y: i32 },
    Drag { x: i32, y: i32 },
    Tap { code: &'static str, ch: Option<&'static str> },
    Combo { modifier: &'static str, code: &'static str },
    Wheel { delta: i32 },
    Move { x: i32, y: i32 },
    Wait { dur: u32 },
}

fn action() -> impl Strategy<Value = Action> {
    let pos = (0..1920i32, 0..1080i32);
    prop_oneof![
        pos.clone().prop_map(|(x, y)| Action::Click { x, y }),
        pos.clone().prop_map(|(x, y)| Action::Drag { x, y }),
        prop::sample::select(vec![
            ("KeyA", Some("a")),
            ("KeyB", Some("b")),
            ("Enter", Some("\r")),
            ("Tab", None),
            ("F2", None)
        ])
        .prop_map(|(code, ch)| Action::Tap { code, ch }),
        (
            prop::sample::select(vec!["ControlLeft", "AltLeft", "ShiftLeft", "MetaLeft"]),
            prop::sample::select(vec!["KeyS", "KeyW", "Tab"])
        )
            .prop_map(|(modifier, code)| Action::Combo { modifier, code }),
        prop::sample::select(vec![-120, 120]).prop_map(|delta| Action::Wheel { delta }),
        pos.prop_map(|(x, y)| Action::Move { x, y }),
        (50..2000u32).prop_map(|dur| Action::Wait { dur }),
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
    let btn = |t, x, y, down| Event::Button { t, x, y, btn: MouseBtn::Left, down, label: String::new() };
    for (a, gap) in actions {
        match a {
            Action::Click { x, y } => ev.extend([btn(t, *x, *y, true), btn(t + 60, *x, *y, false)]),
            Action::Drag { x, y } => ev.extend([
                btn(t, *x, *y, true),
                Event::Move { t: t + 100, x: x + 50, y: *y },
                btn(t + 200, x + 50, *y, false),
            ]),
            Action::Tap { code, ch } => ev.extend([key(t, code, true, *ch), key(t + 40, code, false, None)]),
            Action::Combo { modifier, code } => ev.extend([
                key(t, modifier, true, None),
                key(t + 20, code, true, None),
                key(t + 60, code, false, None),
                key(t + 80, modifier, false, None),
            ]),
            Action::Wheel { delta } => ev.push(Event::Wheel { t, x: 0, y: 0, delta: *delta, horizontal: false }),
            Action::Move { x, y } => ev.push(Event::Move { t, x: *x, y: *y }),
            Action::Wait { dur } => {
                // A cursor sample at the same time as the wait, as recordings have.
                ev.push(Event::Move { t, x: 1, y: 1 });
                ev.push(Event::Wait { t, dur: *dur, label: String::new() });
                t += dur;
            }
        }
        t += 250 + gap;
    }
    ev
}

fn edit(steps: usize, dur: u32) -> impl Strategy<Value = EditOp> {
    let idx = 0..(steps as u32 + 2);
    prop_oneof![
        idx.clone().prop_map(|index| EditOp::DeleteStep { index }),
        (0..dur + 500, 1..3000u32).prop_map(|(at, dur)| EditOp::InsertWait { at, dur, label: String::new() }),
        (0..dur + 500, 1..3000u32).prop_map(|(at, dur)| EditOp::InsertPixelWait {
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
        idx.clone().prop_map(|index| EditOp::SetLabel { index, label: "x".into() }),
        (idx.clone(), 0..3000u32).prop_map(|(index, dur)| EditOp::SetPause { index, dur }),
        (idx, 0..3u32).prop_map(|(index, dur)| EditOp::SetWaitDuration { index, dur }),
        (0..1500u32).prop_map(|max| EditOp::CapPauses { max }),
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
        for s in &steps {
            prop_assert!(s.t <= s.end);
            for &i in &s.items {
                prop_assert!((i as usize) < events.len());
                prop_assert!(!matches!(events[i as usize], Event::Move { .. }), "a move is a step item");
                prop_assert!(seen.insert(i), "event {} is in two steps", i);
            }
        }
        prop_assert!(steps.windows(2).all(|w| w[0].t <= w[1].t));
    }

    #[test]
    fn edits_preserve_the_invariants((events, ops) in macro_and_edits()) {
        let mut m = Macro::new("p", RecordingMeta::single_1080p(), events);
        for op in ops {
            let _ = apply(&mut m, op.clone());
            prop_assert_eq!(check_invariants(&m.events), Ok(()), "after {:?}", op);
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
}
