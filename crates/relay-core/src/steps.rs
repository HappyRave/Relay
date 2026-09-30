//! Groups raw events into the editor's steps (CLICK, DRAG, SCROLL, KEYS,
//! TYPE, WAIT, IF, MOVE). Every event belongs to at most one step; `items`
//! lists them so a step can be deleted as a unit. Cursor moves belong to the
//! press they happen during, or to a MOVE step: the path between two actions.

use std::collections::{BTreeSet, HashMap};

use serde::Serialize;
use ts_rs::TS;

use crate::keys::{self, Modifier};
use crate::model::{Event, MouseBtn, Ms, RecordingMeta, Rgb};

/// Characters typed less than this apart join the same TYPE step (the design's rule).
pub const TYPE_GAP_MS: Ms = 500;
/// Wheel notches less than this apart join the same SCROLL step.
pub const SCROLL_GAP_MS: Ms = 300;
/// A press that moves further than this becomes a drag.
pub const CLICK_SLOP_PX: i32 = 4;

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct TypedChar {
    pub t: Ms,
    pub ch: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum StepKind {
    Click {
        x: i32,
        y: i32,
        btn: MouseBtn,
        count: u8,
        label: String,
    },
    Drag {
        x: i32,
        y: i32,
        to_x: i32,
        to_y: i32,
        btn: MouseBtn,
        label: String,
    },
    Scroll {
        x: i32,
        y: i32,
        delta: i32,
        horizontal: bool,
    },
    Keys {
        combo: Vec<String>,
    },
    Type {
        text: String,
        chars: Vec<TypedChar>,
    },
    Wait {
        dur: Ms,
        label: String,
    },
    PixelWait {
        dur: Ms,
        x: i32,
        y: i32,
        color: Rgb,
        tolerance: u8,
        timeout_ms: Ms,
        label: String,
    },
    /// The cursor going from `x, y` (where it was before) to `to_x, to_y`,
    /// through `samples` recorded positions.
    Move {
        x: i32,
        y: i32,
        to_x: i32,
        to_y: i32,
        samples: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Step {
    pub t: Ms,
    pub end: Ms,
    /// Idle time before the step: since the previous steps ended (or the
    /// start), during which nothing happens. 0 when steps overlap.
    pub pause: Ms,
    /// Indices into the macro's events.
    pub items: Vec<u32>,
    #[serde(flatten)]
    #[ts(flatten)]
    pub kind: StepKind,
}

#[derive(Debug, Clone, Copy)]
pub struct GroupOptions {
    pub double_click_ms: Ms,
    /// The width of the double-click rectangle (`SM_CXDOUBLECLK`), which is
    /// centred on the first click: the second may be half of it away.
    pub double_click_px: u32,
}

impl GroupOptions {
    /// The system double-click settings.
    pub fn new(double_click_ms: Ms, double_click_px: u32) -> Self {
        GroupOptions { double_click_ms, double_click_px }
    }
}

impl Default for GroupOptions {
    /// The Windows defaults.
    fn default() -> Self {
        GroupOptions::new(500, 4)
    }
}

impl From<&RecordingMeta> for GroupOptions {
    fn from(r: &RecordingMeta) -> Self {
        GroupOptions::new(r.double_click_ms, r.double_click_px)
    }
}

struct PendingButton {
    step: usize,
    t: Ms,
    x: i32,
    y: i32,
    /// The cursor went further than the slop while the button was down.
    moved: bool,
}

impl PendingButton {
    fn beyond_slop(&self, x: i32, y: i32) -> bool {
        (x - self.x).abs().max((y - self.y).abs()) > CLICK_SLOP_PX
    }
}

struct LastClick {
    step: usize,
    btn: MouseBtn,
    t_down: Ms,
    x: i32,
    y: i32,
}

struct ModPress {
    code: String,
    modifier: Modifier,
    t: Ms,
    events: Vec<u32>,
    /// Steps started (a key, a click, a scroll) while this modifier was held.
    wrapped: Vec<usize>,
}

/// Records that `step` started while the `active` modifiers were held.
fn wrap(active: &mut [ModPress], step: usize) {
    for p in active {
        if p.wrapped.last() != Some(&step) {
            p.wrapped.push(step);
        }
    }
}

pub fn group_steps(events: &[Event], opts: GroupOptions) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new();
    let mut pending: HashMap<MouseBtn, PendingButton> = HashMap::new();
    let mut last_click: Option<LastClick> = None;
    let mut held_keys: HashMap<String, usize> = HashMap::new();
    let mut active_mods: Vec<ModPress> = Vec::new();
    let mut closed_mods: Vec<ModPress> = Vec::new();

    let push = |steps: &mut Vec<Step>, t: Ms, i: usize, kind: StepKind| {
        steps.push(Step { t, end: t, pause: 0, items: vec![i as u32], kind });
        steps.len() - 1
    };

    for (i, ev) in events.iter().enumerate() {
        let idx = i as u32;
        match ev {
            Event::Move { x, y, .. } => {
                for p in pending.values_mut() {
                    p.moved |= p.beyond_slop(*x, *y);
                }
                // A move while a button is down is part of that press (a drag's path).
                if let Some(step) = pending.values().map(|p| p.step).min() {
                    steps[step].items.push(idx);
                }
            }

            Event::Button { t, x, y, btn, down: true, label } => {
                let step = push(
                    &mut steps,
                    *t,
                    i,
                    StepKind::Click { x: *x, y: *y, btn: *btn, count: 1, label: label.clone() },
                );
                pending.insert(*btn, PendingButton { step, t: *t, x: *x, y: *y, moved: false });
                wrap(&mut active_mods, step);
            }

            Event::Button { t, x, y, btn, down: false, .. } => {
                let Some(p) = pending.remove(btn) else {
                    continue;
                };
                // A lasso that comes back to where it started is still a drag.
                if p.moved || p.beyond_slop(*x, *y) {
                    let s = &mut steps[p.step];
                    s.items.push(idx);
                    s.end = *t;
                    if let StepKind::Click { label, .. } = &s.kind {
                        s.kind = StepKind::Drag { x: p.x, y: p.y, to_x: *x, to_y: *y, btn: *btn, label: label.clone() };
                    }
                    last_click = None;
                    continue;
                }
                // A second click right after the first merges into a double click.
                let merge = last_click.as_ref().filter(|lc| {
                    lc.btn == *btn
                        && lc.step + 1 == p.step
                        && p.step + 1 == steps.len()
                        && p.t.saturating_sub(lc.t_down) <= opts.double_click_ms
                        && (p.x - lc.x).unsigned_abs().max((p.y - lc.y).unsigned_abs()) <= opts.double_click_px / 2
                });
                if let Some(lc) = merge {
                    let target = lc.step;
                    let second = steps.pop().expect("pending click step");
                    // Modifiers that wrapped the second click now wrap the merged one.
                    for m in active_mods.iter_mut().chain(closed_mods.iter_mut()) {
                        for w in &mut m.wrapped {
                            if *w == p.step {
                                *w = target;
                            }
                        }
                    }
                    let s = &mut steps[target];
                    s.items.extend(second.items);
                    s.items.push(idx);
                    s.end = *t;
                    if let StepKind::Click { count, .. } = &mut s.kind {
                        *count = count.saturating_add(1);
                    }
                    last_click = Some(LastClick { step: target, btn: *btn, t_down: p.t, x: p.x, y: p.y });
                } else {
                    let s = &mut steps[p.step];
                    s.items.push(idx);
                    s.end = *t;
                    last_click = Some(LastClick { step: p.step, btn: *btn, t_down: p.t, x: p.x, y: p.y });
                }
            }

            Event::Wheel { t, x, y, delta, horizontal } => {
                let last = steps.len().checked_sub(1);
                let merged = last.is_some_and(|l| {
                    let s = &mut steps[l];
                    match &mut s.kind {
                        StepKind::Scroll { delta: d, horizontal: h, .. }
                            if *h == *horizontal
                                && d.signum() == delta.signum()
                                && t.saturating_sub(s.end) < SCROLL_GAP_MS =>
                        {
                            *d = d.saturating_add(*delta);
                            s.items.push(idx);
                            s.end = *t;
                            true
                        }
                        _ => false,
                    }
                });
                if !merged {
                    let step = push(
                        &mut steps,
                        *t,
                        i,
                        StepKind::Scroll { x: *x, y: *y, delta: *delta, horizontal: *horizontal },
                    );
                    wrap(&mut active_mods, step);
                }
            }

            Event::Key { t, down: true, key, ch } => {
                if let Some(m) = key.modifier() {
                    match active_mods.iter_mut().find(|p| p.code == key.code) {
                        Some(p) => p.events.push(idx), // auto-repeat
                        None => active_mods.push(ModPress {
                            code: key.code.clone(),
                            modifier: m,
                            t: *t,
                            events: vec![idx],
                            wrapped: Vec::new(),
                        }),
                    }
                    continue;
                }
                let mods: BTreeSet<Modifier> = active_mods.iter().map(|p| p.modifier).collect();
                let alt_gr = keys::is_alt_gr(active_mods.iter().map(|p| p.code.as_str()));
                let printable = ch.as_deref().filter(|s| !s.is_empty() && !s.chars().any(char::is_control));
                let shortcut = mods.contains(&Modifier::Win)
                    || (!alt_gr && (mods.contains(&Modifier::Ctrl) || mods.contains(&Modifier::Alt)));

                // Auto-repeat of a held key extends its step.
                if let Some(&s) = held_keys.get(&key.code) {
                    let step = &mut steps[s];
                    step.items.push(idx);
                    step.end = *t;
                    if let (StepKind::Type { text, chars }, Some(c)) = (&mut step.kind, printable) {
                        text.push_str(c);
                        chars.push(TypedChar { t: *t, ch: c.into() });
                    }
                    continue;
                }

                let step = match printable {
                    Some(c) if !shortcut => {
                        let last = steps.len().checked_sub(1);
                        let joined = last.filter(|&l| match &steps[l].kind {
                            StepKind::Type { chars, .. } => {
                                chars.last().is_some_and(|lc| t.saturating_sub(lc.t) < TYPE_GAP_MS)
                            }
                            _ => false,
                        });
                        match joined {
                            Some(l) => {
                                let s = &mut steps[l];
                                if let StepKind::Type { text, chars } = &mut s.kind {
                                    text.push_str(c);
                                    chars.push(TypedChar { t: *t, ch: c.into() });
                                }
                                s.items.push(idx);
                                s.end = *t;
                                l
                            }
                            None => push(
                                &mut steps,
                                *t,
                                i,
                                StepKind::Type { text: c.into(), chars: vec![TypedChar { t: *t, ch: c.into() }] },
                            ),
                        }
                    }
                    _ => {
                        let mut combo: Vec<String> = mods.iter().map(|m| m.label().to_string()).collect();
                        combo.push(keys::key_label(key));
                        push(&mut steps, *t, i, StepKind::Keys { combo })
                    }
                };
                held_keys.insert(key.code.clone(), step);
                wrap(&mut active_mods, step);
            }

            Event::Key { t, down: false, key, .. } => {
                if key.modifier().is_some() {
                    if let Some(pos) = active_mods.iter().position(|p| p.code == key.code) {
                        let mut p = active_mods.remove(pos);
                        p.events.push(idx);
                        closed_mods.push(p);
                    }
                    continue;
                }
                if let Some(s) = held_keys.remove(&key.code) {
                    let step = &mut steps[s];
                    step.items.push(idx);
                    step.end = step.end.max(*t);
                }
            }

            Event::Wait { t, dur, label } => {
                let s = push(&mut steps, *t, i, StepKind::Wait { dur: *dur, label: label.clone() });
                steps[s].end = t.saturating_add(*dur);
            }

            Event::PixelWait { t, dur, x, y, color, tolerance, timeout_ms, label } => {
                let s = push(
                    &mut steps,
                    *t,
                    i,
                    StepKind::PixelWait {
                        dur: *dur,
                        x: *x,
                        y: *y,
                        color: *color,
                        tolerance: *tolerance,
                        timeout_ms: *timeout_ms,
                        label: label.clone(),
                    },
                );
                steps[s].end = t.saturating_add(*dur);
            }
        }
    }

    // Modifier presses belong to the one step they wrap (a shortcut, a
    // Shift-click), which then spans them; a lone tap (e.g. Win) is its own
    // KEYS step; presses shared by several steps belong to none.
    closed_mods.append(&mut active_mods);
    for mut p in closed_mods {
        p.wrapped.sort_unstable();
        p.wrapped.dedup();
        let end = p.events.last().map_or(p.t, |&e| events[e as usize].t());
        match p.wrapped.as_slice() {
            [s] => {
                let step = &mut steps[*s];
                step.items.extend(&p.events);
                step.t = step.t.min(p.t);
                step.end = step.end.max(end);
            }
            [] => {
                steps.push(Step {
                    t: p.t,
                    end,
                    pause: 0,
                    items: p.events,
                    kind: StepKind::Keys { combo: vec![p.modifier.label().into()] },
                });
            }
            _ => {}
        }
    }

    move_steps(events, &mut steps);
    for s in &mut steps {
        s.items.sort_unstable();
    }
    steps.sort_by_key(|s| s.t);
    let mut busy_until = 0;
    for s in &mut steps {
        s.pause = s.t.saturating_sub(busy_until);
        busy_until = busy_until.max(s.end);
    }
    steps
}

/// Adds a MOVE step for each run of cursor moves no press owns: the moves
/// between two other events. Moves between the presses of a double click
/// belong to it. Runs after the rest of the grouping, so MOVE steps never come
/// between a step and the one it merges with or the modifier that wraps it.
fn move_steps(events: &[Event], steps: &mut Vec<Step>) {
    let mut owned = vec![false; events.len()];
    for s in steps.iter() {
        for &i in &s.items {
            owned[i as usize] = true;
        }
    }
    for s in steps.iter_mut() {
        let StepKind::Click { count: 2.., .. } = s.kind else { continue };
        let presses = || s.items.iter().copied().filter(|&i| matches!(events[i as usize], Event::Button { .. }));
        let (Some(first), Some(last)) = (presses().min(), presses().max()) else { continue };
        for i in first..last {
            if !owned[i as usize] && matches!(events[i as usize], Event::Move { .. }) {
                owned[i as usize] = true;
                s.items.push(i);
            }
        }
    }

    let mut cursor: Option<(i32, i32)> = None;
    let mut run: Option<(i32, i32, Vec<u32>)> = None;
    let close = |run: &mut Option<(i32, i32, Vec<u32>)>, steps: &mut Vec<Step>| {
        let Some((x, y, items)) = run.take() else { return };
        let (first, last) = (&events[items[0] as usize], &events[*items.last().unwrap() as usize]);
        let (to_x, to_y) = last.pos().unwrap_or((x, y));
        steps.push(Step {
            t: first.t(),
            end: last.t(),
            pause: 0,
            kind: StepKind::Move { x, y, to_x, to_y, samples: items.len() as u32 },
            items,
        });
    };
    for (i, e) in events.iter().enumerate() {
        match e {
            Event::Move { x, y, .. } if !owned[i] => {
                let (fx, fy) = cursor.unwrap_or((*x, *y));
                run.get_or_insert_with(|| (fx, fy, Vec::new())).2.push(i as u32);
            }
            _ => close(&mut run, steps),
        }
        if let Event::Move { x, y, .. } | Event::Button { x, y, .. } | Event::Wheel { x, y, .. } = e {
            cursor = Some((*x, *y));
        }
    }
    close(&mut run, steps);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::KeyStroke;

    fn btn(t: Ms, x: i32, y: i32, down: bool) -> Event {
        Event::Button { t, x, y, btn: MouseBtn::Left, down, label: String::new() }
    }
    fn key(t: Ms, code: &str, down: bool, ch: Option<&str>) -> Event {
        Event::Key { t, down, key: KeyStroke::code(code), ch: ch.map(Into::into) }
    }
    fn tap(t: Ms, code: &str, ch: Option<&str>) -> [Event; 2] {
        [key(t, code, true, ch), key(t + 40, code, false, None)]
    }
    fn group(events: &[Event]) -> Vec<Step> {
        group_steps(events, GroupOptions::default())
    }

    #[test]
    fn click_and_drag() {
        let ev = [btn(0, 10, 10, true), btn(80, 11, 12, false), btn(1000, 10, 10, true), btn(1300, 200, 10, false)];
        let s = group(&ev);
        assert_eq!(s.len(), 2);
        assert!(matches!(s[0].kind, StepKind::Click { count: 1, .. }));
        assert_eq!(s[0].items, vec![0, 1]);
        assert!(matches!(s[1].kind, StepKind::Drag { to_x: 200, .. }));
        assert_eq!((s[1].t, s[1].end), (1000, 1300));
    }

    #[test]
    fn a_drag_that_returns_to_its_start_is_a_drag() {
        let mv = |t, x| Event::Move { t, x, y: 10 };
        let lasso = [btn(0, 10, 10, true), mv(100, 200), mv(200, 12), btn(300, 10, 10, false)];
        let s = group(&lasso);
        assert_eq!(s.len(), 1);
        assert!(matches!(s[0].kind, StepKind::Drag { x: 10, to_x: 10, .. }), "{:?}", s[0].kind);
        assert_eq!(s[0].items, vec![0, 1, 2, 3], "the path is the drag's");
        // A wobble within the slop is still a click.
        let wobble = [btn(0, 10, 10, true), mv(50, 14), btn(100, 10, 10, false)];
        assert!(matches!(group(&wobble)[0].kind, StepKind::Click { .. }));
    }

    #[test]
    fn double_click_merges_within_system_time() {
        let ev = [btn(0, 10, 10, true), btn(60, 10, 10, false), btn(200, 11, 10, true), btn(260, 11, 10, false)];
        let s = group(&ev);
        assert_eq!(s.len(), 1);
        assert!(matches!(s[0].kind, StepKind::Click { count: 2, .. }));
        assert_eq!(s[0].items, vec![0, 1, 2, 3]);

        let slow = [btn(0, 10, 10, true), btn(60, 10, 10, false), btn(900, 10, 10, true), btn(960, 10, 10, false)];
        assert_eq!(group(&slow).len(), 2);
    }

    #[test]
    fn double_click_distance_is_half_the_system_rectangle() {
        let two = |dx| {
            [btn(0, 10, 10, true), btn(60, 10, 10, false), btn(200, 10 + dx, 10, true), btn(260, 10 + dx, 10, false)]
        };
        let steps = |px, dx| group_steps(&two(dx), GroupOptions::new(500, px)).len();
        // The Windows default is a 4 px wide rectangle: 2 px either way.
        assert_eq!((steps(4, 2), steps(4, -2), steps(4, 3)), (1, 1, 2));
        assert_eq!((steps(5, 2), steps(5, 3)), (1, 2));
        // Not widened to the click slop: a 1 px rectangle needs the same pixel.
        assert_eq!((steps(1, 0), steps(1, 1)), (1, 2));
        assert_eq!((steps(20, 10), steps(20, 11)), (1, 2));
    }

    #[test]
    fn typing_merges_until_a_pause() {
        let mut ev = Vec::new();
        for (i, c) in ["h", "i"].iter().enumerate() {
            ev.extend(tap(i as u32 * 100, &format!("Key{}", c.to_uppercase()), Some(c)));
        }
        ev.extend(tap(1200, "KeyX", Some("x")));
        let s = group(&ev);
        assert_eq!(s.len(), 2);
        assert!(matches!(&s[0].kind, StepKind::Type { text, .. } if text == "hi"));
        assert_eq!(s[0].items, vec![0, 1, 2, 3]);
        assert!(matches!(&s[1].kind, StepKind::Type { text, .. } if text == "x"));
    }

    #[test]
    fn shortcuts_become_keys_steps_owning_their_modifiers() {
        let ev = [
            key(0, "ControlLeft", true, None),
            key(50, "KeyA", true, Some("\u{1}")),
            key(90, "KeyA", false, None),
            key(120, "ControlLeft", false, None),
        ];
        let s = group(&ev);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].kind, StepKind::Keys { combo: vec!["Ctrl".into(), "A".into()] });
        assert_eq!(s[0].items, vec![0, 1, 2, 3]);
        assert_eq!((s[0].t, s[0].end), (0, 120), "the step spans its modifier");
    }

    #[test]
    fn shift_click_is_one_click_owning_shift() {
        let b = |t, down| Event::Button { t, x: 5, y: 5, btn: MouseBtn::Left, down, label: String::new() };
        let ev = [key(0, "ShiftLeft", true, None), b(50, true), b(90, false), key(200, "ShiftLeft", false, None)];
        let s = group(&ev);
        assert_eq!(s.len(), 1, "{s:?}");
        assert!(matches!(s[0].kind, StepKind::Click { count: 1, .. }));
        assert_eq!((s[0].t, s[0].end, s[0].items.len()), (0, 200, 4));
    }

    #[test]
    fn a_modifier_around_a_double_click_follows_the_merge() {
        let b = |t, down| Event::Button { t, x: 5, y: 5, btn: MouseBtn::Left, down, label: String::new() };
        let ev = [
            key(0, "ControlLeft", true, None),
            b(50, true),
            b(90, false),
            b(200, true),
            b(240, false),
            key(300, "ControlLeft", false, None),
        ];
        let s = group(&ev);
        assert_eq!(s.len(), 1, "{s:?}");
        assert!(matches!(s[0].kind, StepKind::Click { count: 2, .. }));
        assert_eq!(s[0].items, vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn a_modifier_held_across_two_shortcuts_is_shared() {
        let ev = [
            key(0, "ControlLeft", true, None),
            key(50, "KeyC", true, None),
            key(90, "KeyC", false, None),
            key(150, "KeyV", true, None),
            key(190, "KeyV", false, None),
            key(220, "ControlLeft", false, None),
        ];
        let s = group(&ev);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].items, vec![1, 2]);
        assert_eq!(s[1].kind, StepKind::Keys { combo: vec!["Ctrl".into(), "V".into()] });
    }

    #[test]
    fn shift_and_alt_gr_characters_are_text() {
        let ev = [
            key(0, "ShiftLeft", true, None),
            key(20, "KeyH", true, Some("H")),
            key(60, "KeyH", false, None),
            key(70, "ShiftLeft", false, None),
            key(100, "ControlLeft", true, None),
            key(100, "AltRight", true, None),
            key(130, "Digit2", true, Some("@")),
            key(170, "Digit2", false, None),
            key(180, "AltRight", false, None),
            key(180, "ControlLeft", false, None),
        ];
        let s = group(&ev);
        assert_eq!(s.len(), 1, "{s:#?}");
        assert!(matches!(&s[0].kind, StepKind::Type { text, .. } if text == "H@"));
        assert_eq!(s[0].items.len(), 10);
    }

    #[test]
    fn right_alt_without_ctrl_is_alt() {
        // US layout: right Alt has no AltGr, and Right Alt + F opens a menu.
        let ev = [
            key(0, "AltRight", true, None),
            key(30, "KeyF", true, Some("f")),
            key(70, "KeyF", false, None),
            key(90, "AltRight", false, None),
        ];
        let s = group(&ev);
        assert_eq!(s.len(), 1, "{s:#?}");
        assert_eq!(s[0].kind, StepKind::Keys { combo: vec!["Alt".into(), "F".into()] });
        assert_eq!(s[0].items, vec![0, 1, 2, 3]);
    }

    #[test]
    fn non_printable_keys_and_lone_modifiers() {
        let mut ev = Vec::new();
        ev.extend(tap(0, "Enter", Some("\r")));
        ev.extend([key(100, "ShiftLeft", true, None)]);
        ev.extend(tap(120, "Tab", Some("\t")));
        ev.extend([key(200, "ShiftLeft", false, None)]);
        ev.extend(tap(500, "MetaLeft", None));
        let s = group(&ev);
        let combos: Vec<_> = s
            .iter()
            .map(|s| match &s.kind {
                StepKind::Keys { combo } => combo.join(" + "),
                k => panic!("{k:?}"),
            })
            .collect();
        assert_eq!(combos, ["Enter", "Shift + Tab", "Win"]);
    }

    #[test]
    fn held_key_repeat_extends_one_step() {
        let ev = [
            key(0, "KeyA", true, Some("a")),
            key(500, "KeyA", true, Some("a")),
            key(530, "KeyA", true, Some("a")),
            key(560, "KeyA", false, None),
        ];
        let s = group(&ev);
        assert_eq!(s.len(), 1);
        assert!(matches!(&s[0].kind, StepKind::Type { text, .. } if text == "aaa"));
    }

    #[test]
    fn wheel_notches_merge_by_direction() {
        let w = |t, delta| Event::Wheel { t, x: 0, y: 0, delta, horizontal: false };
        let s = group(&[w(0, -120), w(50, -120), w(100, 120)]);
        assert_eq!(s.len(), 2);
        assert!(matches!(s[0].kind, StepKind::Scroll { delta: -240, .. }));
    }

    #[test]
    fn gaps_merge_below_the_limit_not_at_it() {
        let w = |t| Event::Wheel { t, x: 0, y: 0, delta: -120, horizontal: false };
        assert_eq!(group(&[w(0), w(299)]).len(), 1);
        assert_eq!(group(&[w(0), w(300)]).len(), 2);
        let typed = |gap| [tap(0, "KeyA", Some("a")), tap(gap, "KeyB", Some("b"))].concat();
        assert_eq!(group(&typed(499)).len(), 1);
        assert_eq!(group(&typed(500)).len(), 2);
    }

    fn press(t: Ms, x: i32, b: MouseBtn, down: bool) -> Event {
        Event::Button { t, x, y: 10, btn: b, down, label: String::new() }
    }

    #[test]
    fn triple_and_right_double_clicks() {
        let clicks = |b, n: u32| {
            (0..n).flat_map(|i| [press(i * 150, 10, b, true), press(i * 150 + 50, 10, b, false)]).collect::<Vec<_>>()
        };
        let s = group(&clicks(MouseBtn::Left, 3));
        assert_eq!(s.len(), 1);
        assert!(matches!(s[0].kind, StepKind::Click { count: 3, btn: MouseBtn::Left, .. }));
        assert_eq!((s[0].items.len(), s[0].end), (6, 350));
        let s = group(&clicks(MouseBtn::Right, 2));
        assert_eq!(s.len(), 1);
        assert!(matches!(s[0].kind, StepKind::Click { count: 2, btn: MouseBtn::Right, .. }));
        // Different buttons don't merge.
        let mixed = [
            press(0, 10, MouseBtn::Left, true),
            press(50, 10, MouseBtn::Left, false),
            press(150, 10, MouseBtn::Middle, true),
            press(200, 10, MouseBtn::Middle, false),
        ];
        assert_eq!(group(&mixed).len(), 2);
    }

    #[test]
    fn modifier_drags_and_scrolls_own_their_modifier() {
        for m in ["ShiftLeft", "ControlLeft"] {
            let ev = [
                key(0, m, true, None),
                btn(50, 10, 10, true),
                Event::Move { t: 100, x: 100, y: 10 },
                btn(200, 100, 10, false),
                key(300, m, false, None),
            ];
            let s = group(&ev);
            assert_eq!(s.len(), 1, "{m}: {s:?}");
            assert!(matches!(s[0].kind, StepKind::Drag { x: 10, to_x: 100, .. }));
            assert_eq!((s[0].t, s[0].end, s[0].items.clone()), (0, 300, vec![0, 1, 2, 3, 4]));
        }
        let w = |t| Event::Wheel { t, x: 0, y: 0, delta: 120, horizontal: false };
        let zoom = [key(0, "ControlLeft", true, None), w(50), w(100), key(200, "ControlLeft", false, None)];
        let s = group(&zoom);
        assert_eq!(s.len(), 1);
        assert!(matches!(s[0].kind, StepKind::Scroll { delta: 240, .. }));
        assert_eq!(s[0].items, vec![0, 1, 2, 3]);
    }

    #[test]
    fn horizontal_and_vertical_scrolls_stay_apart() {
        let w = |t, horizontal| Event::Wheel { t, x: 0, y: 0, delta: 120, horizontal };
        let s = group(&[w(0, false), w(50, true), w(100, true), w(150, false)]);
        let kinds: Vec<_> = s
            .iter()
            .map(|s| match s.kind {
                StepKind::Scroll { delta, horizontal, .. } => (delta, horizontal),
                _ => panic!(),
            })
            .collect();
        assert_eq!(kinds, [(120, false), (240, true), (120, false)]);
    }

    #[test]
    fn a_key_held_across_a_click_is_its_own_step() {
        let ev = [
            key(0, "KeyA", true, Some("a")),
            btn(100, 5, 5, true),
            btn(150, 5, 5, false),
            key(600, "KeyA", false, None),
        ];
        let s = group(&ev);
        assert_eq!(s.len(), 2, "{s:?}");
        assert!(matches!(&s[0].kind, StepKind::Type { text, .. } if text == "a"));
        assert_eq!((s[0].t, s[0].end, s[0].items.clone()), (0, 600, vec![0, 3]));
        assert!(matches!(s[1].kind, StepKind::Click { count: 1, .. }));
        assert_eq!((s[1].items.clone(), s[1].pause), (vec![1, 2], 0), "no pause: the key is still down");
    }

    #[test]
    fn composed_characters_and_emoji_are_text() {
        let ev = [tap(0, "BracketLeft", Some("ê")), tap(100, "Unidentified", Some("😀")), tap(200, "KeyE", Some("^e"))]
            .concat();
        let s = group(&ev);
        assert_eq!(s.len(), 1);
        let StepKind::Type { text, chars } = &s[0].kind else { panic!() };
        assert_eq!(text, "ê😀^e");
        assert_eq!(chars.iter().map(|c| c.ch.as_str()).collect::<Vec<_>>(), ["ê", "😀", "^e"]);
        // A dead key types nothing by itself: it's a key, between two TYPE steps.
        let ev = [tap(0, "KeyA", Some("a")), tap(100, "BracketLeft", None), tap(200, "KeyE", Some("e"))].concat();
        let s = group(&ev);
        assert_eq!(s.len(), 3);
        assert_eq!(s[1].kind, StepKind::Keys { combo: vec!["[".into()] });
    }

    fn mv(t: Ms, x: i32) -> Event {
        Event::Move { t, x, y: 10 }
    }

    #[test]
    fn the_cursor_between_actions_is_a_move_step() {
        let ev = [mv(100, 0), mv(200, 50), btn(400, 50, 10, true), btn(450, 50, 10, false), mv(900, 80), mv(1000, 300)];
        let s = group(&ev);
        assert_eq!(s.len(), 3, "{s:#?}");
        // Before the first click: it starts where the cursor was first seen.
        assert_eq!(s[0].kind, StepKind::Move { x: 0, y: 10, to_x: 50, to_y: 10, samples: 2 });
        assert_eq!((s[0].t, s[0].end, s[0].pause, s[0].items.clone()), (100, 200, 100, vec![0, 1]));
        // The click's pause is the time the cursor stood still.
        assert_eq!(s[1].pause, 200);
        // After the last one (the trip to Relay's Stop button): it starts at the click.
        assert_eq!(s[2].kind, StepKind::Move { x: 50, y: 10, to_x: 300, to_y: 10, samples: 2 });
        assert_eq!((s[2].t, s[2].end, s[2].pause), (900, 1000, 450));
    }

    #[test]
    fn any_other_event_ends_a_move() {
        let ev =
            [mv(0, 0), mv(100, 10), key(150, "ShiftLeft", true, None), mv(200, 20), key(250, "ShiftLeft", false, None)];
        let kinds: Vec<_> =
            group(&ev).into_iter().map(|s| (s.items, matches!(s.kind, StepKind::Move { .. }))).collect();
        assert_eq!(kinds, [(vec![0, 1], true), (vec![2, 4], false), (vec![3], true)]);
        // A still cursor doesn't: a hesitation halfway is part of the move.
        let s = group(&[mv(0, 0), mv(100, 10), mv(3000, 20)]);
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn a_modifier_around_a_move_and_a_click_is_the_clicks() {
        let ev = [
            key(0, "ControlLeft", true, None),
            mv(50, 20),
            mv(100, 40),
            btn(150, 40, 10, true),
            btn(200, 40, 10, false),
            key(300, "ControlLeft", false, None),
        ];
        let s = group(&ev);
        assert_eq!(s.len(), 2, "{s:#?}");
        assert!(matches!(s[0].kind, StepKind::Click { .. }));
        assert_eq!(s[0].items, vec![0, 3, 4, 5], "Ctrl-click");
        assert_eq!((s[1].items.clone(), s[1].pause), (vec![1, 2], 0), "the move overlaps it");
    }

    #[test]
    fn a_shake_between_the_clicks_of_a_double_click_is_the_clicks() {
        let ev = [
            btn(0, 10, 10, true),
            btn(60, 10, 10, false),
            mv(100, 11),
            btn(200, 10, 10, true),
            btn(260, 10, 10, false),
        ];
        let s = group(&ev);
        assert_eq!(s.len(), 1, "{s:#?}");
        assert!(matches!(s[0].kind, StepKind::Click { count: 2, .. }));
        assert_eq!(s[0].items, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn waits_span_their_duration() {
        let s = group(&[Event::Wait { t: 100, dur: 700, label: "Dialog".into() }]);
        assert_eq!((s[0].t, s[0].end), (100, 800));
    }
}
