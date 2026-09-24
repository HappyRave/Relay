//! Groups raw events into the editor's steps (CLICK, DRAG, SCROLL, KEYS,
//! TYPE, WAIT, IF). Every non-move event belongs to at most one step; `items`
//! lists them so a step can be deleted as a unit.

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
    Click { x: i32, y: i32, btn: MouseBtn, count: u8, label: String },
    Drag { x: i32, y: i32, to_x: i32, to_y: i32, btn: MouseBtn, label: String },
    Scroll { x: i32, y: i32, delta: i32, horizontal: bool },
    Keys { combo: Vec<String> },
    Type { text: String, chars: Vec<TypedChar> },
    Wait { dur: Ms, label: String },
    PixelWait { dur: Ms, x: i32, y: i32, color: Rgb, tolerance: u8, timeout_ms: Ms, label: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct Step {
    pub t: Ms,
    pub end: Ms,
    /// Idle time before the step: since the previous steps ended (or the
    /// start), during which only the cursor moves. 0 when steps overlap.
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
    pub double_click_px: u32,
}

impl Default for GroupOptions {
    fn default() -> Self {
        GroupOptions { double_click_ms: 500, double_click_px: 4 }
    }
}

impl From<&RecordingMeta> for GroupOptions {
    fn from(r: &RecordingMeta) -> Self {
        GroupOptions { double_click_ms: r.double_click_ms, double_click_px: r.double_click_px.max(4) }
    }
}

struct PendingButton {
    step: usize,
    t: Ms,
    x: i32,
    y: i32,
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
    /// Steps whose keys were pressed while this modifier was held.
    wrapped: Vec<usize>,
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
            Event::Move { .. } => {}

            Event::Button { t, x, y, btn, down: true, label } => {
                let step = push(
                    &mut steps,
                    *t,
                    i,
                    StepKind::Click { x: *x, y: *y, btn: *btn, count: 1, label: label.clone() },
                );
                pending.insert(*btn, PendingButton { step, t: *t, x: *x, y: *y });
            }

            Event::Button { t, x, y, btn, down: false, .. } => {
                let Some(p) = pending.remove(btn) else { continue };
                let moved = (x - p.x).abs().max((y - p.y).abs()) > CLICK_SLOP_PX;
                if moved {
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
                        && (p.x - lc.x).unsigned_abs().max((p.y - lc.y).unsigned_abs()) <= opts.double_click_px
                });
                if let Some(lc) = merge {
                    let target = lc.step;
                    let second = steps.pop().expect("pending click step");
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
                                && t.saturating_sub(s.end) <= SCROLL_GAP_MS =>
                        {
                            *d += delta;
                            s.items.push(idx);
                            s.end = *t;
                            true
                        }
                        _ => false,
                    }
                });
                if !merged {
                    push(&mut steps, *t, i, StepKind::Scroll { x: *x, y: *y, delta: *delta, horizontal: *horizontal });
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
                let alt_gr = active_mods.iter().any(|p| keys::is_alt_gr(&p.code));
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
                for p in &mut active_mods {
                    p.wrapped.push(step);
                }
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
                steps[s].end = t + dur;
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
                steps[s].end = t + dur;
            }
        }
    }

    // Modifier presses belong to the one step they wrap; a lone tap (e.g. Win)
    // is its own KEYS step; presses shared by several steps belong to none.
    closed_mods.append(&mut active_mods);
    for mut p in closed_mods {
        p.wrapped.dedup();
        match p.wrapped.as_slice() {
            [s] => steps[*s].items.extend(&p.events),
            [] => {
                let end = p.events.last().map_or(p.t, |&e| events[e as usize].t());
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
    fn waits_span_their_duration() {
        let s = group(&[Event::Wait { t: 100, dur: 700, label: "Dialog".into() }]);
        assert_eq!((s[0].t, s[0].end), (100, 800));
    }
}
