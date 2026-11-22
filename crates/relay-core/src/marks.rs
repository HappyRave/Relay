//! What *Show targets* draws: a mark for each step that acts on the screen
//! (a click, a drag, a mouse move, a scroll, keys, typed text), where it
//! lands with the macro's *Window* offset applied. Live marks show one by one
//! as their step plays; the still overlay shows every target at once,
//! grouping the steps that land on the same spot.

use crate::model::{Event, MouseBtn, Ms};
use crate::steps::{Step, StepKind, step_number};

#[derive(Debug, Clone, PartialEq)]
pub enum MarkKind {
    Click {
        btn: MouseBtn,
        count: u8,
    },
    /// A drag, or a mouse move (`drag: false`), through `points`.
    Path {
        points: Vec<(i32, i32)>,
        drag: bool,
    },
    Scroll {
        delta: i32,
        horizontal: bool,
    },
    Keys {
        combo: Vec<String>,
    },
    /// Typed text, or a Text step's template.
    Text {
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mark {
    /// The step's first event: the mark shows when it plays.
    pub first: usize,
    /// The step's number; `None` for a mouse move.
    pub number: Option<u32>,
    pub t: Ms,
    pub end: Ms,
    /// Where it lands; for keys and text, where the cursor is then.
    pub x: i32,
    pub y: i32,
    pub kind: MarkKind,
}

impl Mark {
    /// Keys and text: drawn as a caption by the cursor, not as a target.
    pub fn is_caption(&self) -> bool {
        matches!(self.kind, MarkKind::Keys { .. } | MarkKind::Text { .. })
    }
}

/// The marks of `steps`, ordered by their first event, shifted by `offset`.
pub fn marks(events: &[Event], steps: &[Step], offset: (i32, i32)) -> Vec<Mark> {
    let (dx, dy) = offset;
    let cursor = cursor_before(events);
    let mut out: Vec<Mark> = steps
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            let first = *s.items.iter().min()? as usize;
            let number = (!matches!(s.kind, StepKind::Move { .. })).then(|| step_number(steps, i));
            let ((x, y), kind) = match &s.kind {
                &StepKind::Click { x, y, btn, count, .. } => ((x, y), MarkKind::Click { btn, count }),
                StepKind::Drag { x, y, .. } | StepKind::Move { x, y, .. } => {
                    let drag = matches!(s.kind, StepKind::Drag { .. });
                    let mut points = if drag { vec![] } else { vec![(x + dx, y + dy)] };
                    points.extend(s.items.iter().filter_map(|&e| match events[e as usize] {
                        Event::Move { x, y, .. } | Event::Button { x, y, .. } => Some((x + dx, y + dy)),
                        _ => None,
                    }));
                    points.dedup();
                    ((*x, *y), MarkKind::Path { points, drag })
                }
                &StepKind::Scroll { x, y, delta, horizontal } => ((x, y), MarkKind::Scroll { delta, horizontal }),
                StepKind::Keys { combo } => (cursor[first], MarkKind::Keys { combo: combo.clone() }),
                StepKind::Type { text, .. } | StepKind::Text { text, .. } => {
                    (cursor[first], MarkKind::Text { text: text.clone() })
                }
                StepKind::Wait { .. } | StepKind::PixelWait { .. } | StepKind::FindImage { .. } => return None,
            };
            Some(Mark { first, number, t: s.t, end: s.end, x: x + dx, y: y + dy, kind })
        })
        .collect();
    out.sort_by_key(|m| m.first);
    out
}

/// Where the cursor is just before each event: the last position an event
/// recorded, (0, 0) before any.
fn cursor_before(events: &[Event]) -> Vec<(i32, i32)> {
    let mut at = (0, 0);
    events
        .iter()
        .map(|e| {
            let before = at;
            if let Event::Move { x, y, .. } | Event::Button { x, y, .. } | Event::Wheel { x, y, .. } = *e {
                at = (x, y);
            }
            before
        })
        .collect()
}

/// A spot on the still overlay: the numbered marks that land on one point,
/// in step order.
#[derive(Debug, Clone, PartialEq)]
pub struct Spot {
    pub x: i32,
    pub y: i32,
    /// Indices into the marks.
    pub marks: Vec<usize>,
}

/// The still overlay's spots: numbered marks on the same point share one,
/// in the order the first of them plays. Moves have no spot.
pub fn spots(marks: &[Mark]) -> Vec<Spot> {
    let mut by_number: Vec<usize> = (0..marks.len()).filter(|&i| marks[i].number.is_some()).collect();
    by_number.sort_by_key(|&i| marks[i].number);
    let mut out: Vec<Spot> = Vec::new();
    for i in by_number {
        let (x, y) = (marks[i].x, marks[i].y);
        match out.iter_mut().find(|s| (s.x, s.y) == (x, y)) {
            Some(s) => s.marks.push(i),
            None => out.push(Spot { x, y, marks: vec![i] }),
        }
    }
    out
}

/// The still overlay's route: the points the numbered marks visit in step
/// order (a drag adds where it ends), without repeating a point.
pub fn route(marks: &[Mark]) -> Vec<(i32, i32)> {
    let mut numbered: Vec<&Mark> = marks.iter().filter(|m| m.number.is_some()).collect();
    numbered.sort_by_key(|m| m.number);
    let mut out: Vec<(i32, i32)> = Vec::new();
    for m in numbered {
        let mut visit = |p: (i32, i32)| {
            if out.last() != Some(&p) {
                out.push(p);
            }
        };
        visit((m.x, m.y));
        if let MarkKind::Path { points, .. } = &m.kind
            && let Some(&end) = points.last()
        {
            visit(end);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::KeyStroke;
    use crate::steps::{GroupOptions, group_steps};

    fn btn(t: Ms, x: i32, y: i32, b: MouseBtn, down: bool) -> Event {
        Event::Button { t, x, y, btn: b, down, label: String::new() }
    }
    fn click(t: Ms, x: i32, y: i32) -> [Event; 2] {
        [btn(t, x, y, MouseBtn::Left, true), btn(t + 60, x, y, MouseBtn::Left, false)]
    }
    fn key(t: Ms, code: &str, down: bool, ch: Option<&str>) -> Event {
        Event::Key { t, down, key: KeyStroke::code(code), ch: ch.map(Into::into) }
    }
    fn mv(t: Ms, x: i32, y: i32) -> Event {
        Event::Move { t, x, y }
    }
    fn of(events: &[Event], offset: (i32, i32)) -> Vec<Mark> {
        marks(events, &group_steps(events, GroupOptions::default()), offset)
    }

    /// Move to (100, 50), click, type "hi", Ctrl+S, drag (10, 10) → (200, 10), scroll, right click.
    fn scene() -> Vec<Event> {
        let mut e = vec![mv(0, 40, 40), mv(100, 100, 50)];
        e.extend(click(200, 100, 50));
        e.extend([key(1000, "KeyH", true, Some("h")), key(1040, "KeyH", false, None)]);
        e.extend([key(1100, "KeyI", true, Some("i")), key(1140, "KeyI", false, None)]);
        e.extend([key(2000, "ControlLeft", true, None), key(2050, "KeyS", true, None)]);
        e.extend([key(2100, "KeyS", false, None), key(2150, "ControlLeft", false, None)]);
        e.extend([mv(2800, 10, 10), btn(3000, 10, 10, MouseBtn::Left, true), mv(3100, 100, 12)]);
        e.push(btn(3200, 200, 10, MouseBtn::Left, false));
        e.push(Event::Wheel { t: 4000, x: 200, y: 10, delta: -120, horizontal: false });
        e.extend([btn(5000, 200, 10, MouseBtn::Right, true), btn(5060, 200, 10, MouseBtn::Right, false)]);
        e
    }

    #[test]
    fn every_kind_of_step_gets_a_mark_in_play_order() {
        let m = of(&scene(), (0, 0));
        let kinds: Vec<_> = m
            .iter()
            .map(|m| match &m.kind {
                MarkKind::Click { btn, .. } => format!("{:?} click {},{}", btn, m.x, m.y),
                MarkKind::Path { drag, points } => format!("{} {:?}", if *drag { "drag" } else { "move" }, points),
                MarkKind::Scroll { delta, .. } => format!("scroll {delta} {},{}", m.x, m.y),
                MarkKind::Keys { combo } => format!("keys {} {},{}", combo.join("+"), m.x, m.y),
                MarkKind::Text { text } => format!("text {text} {},{}", m.x, m.y),
            })
            .collect();
        assert_eq!(
            kinds,
            [
                "move [(40, 40), (100, 50)]",
                "Left click 100,50",
                "text hi 100,50",
                "keys Ctrl+S 100,50",
                "move [(100, 50), (10, 10)]",
                "drag [(10, 10), (100, 12), (200, 10)]",
                "scroll -120 200,10",
                "Right click 200,10",
            ]
        );
        let numbers: Vec<_> = m.iter().map(|m| m.number).collect();
        assert_eq!(
            numbers,
            [None, Some(1), Some(2), Some(3), None, Some(4), Some(5), Some(6)],
            "moves aren't numbered"
        );
    }

    #[test]
    fn marks_show_when_their_first_event_plays() {
        let e = scene();
        let m = of(&e, (0, 0));
        assert!(m.windows(2).all(|w| w[0].first < w[1].first));
        let drag = m.iter().find(|m| matches!(m.kind, MarkKind::Path { drag: true, .. })).unwrap();
        assert!(matches!(e[drag.first], Event::Button { down: true, .. }));
    }

    #[test]
    fn the_window_offset_moves_every_mark() {
        let m = of(&scene(), (30, -5));
        let click = m.iter().find(|m| m.number == Some(1)).unwrap();
        assert_eq!((click.x, click.y), (130, 45));
        let keys = m.iter().find(|m| matches!(m.kind, MarkKind::Keys { .. })).unwrap();
        assert_eq!((keys.x, keys.y), (130, 45), "keys show by the cursor, which moved too");
        let MarkKind::Path { points, .. } = &m.iter().find(|m| m.number == Some(4)).unwrap().kind else { panic!() };
        assert_eq!(points, &[(40, 5), (130, 7), (230, 5)]);
    }

    #[test]
    fn waits_and_checks_have_no_mark() {
        let mut e = click(0, 5, 5).to_vec();
        e.push(Event::Wait { t: 500, dur: 300, label: String::new() });
        assert_eq!(of(&e, (0, 0)).len(), 1);
    }

    #[test]
    fn keys_before_any_mouse_event_show_at_the_origin() {
        let e = [key(0, "Enter", true, None), key(40, "Enter", false, None)];
        let m = of(&e, (0, 0));
        assert_eq!((m[0].x, m[0].y), (0, 0));
    }

    #[test]
    fn steps_on_the_same_spot_share_one_and_moves_have_none() {
        let m = of(&scene(), (0, 0));
        let s = spots(&m);
        let numbers = |s: &Spot| s.marks.iter().map(|&i| m[i].number.unwrap()).collect::<Vec<_>>();
        assert_eq!(s.len(), 3);
        assert_eq!(((s[0].x, s[0].y), numbers(&s[0])), ((100, 50), vec![1, 2, 3]));
        assert_eq!(((s[1].x, s[1].y), numbers(&s[1])), ((10, 10), vec![4]));
        assert_eq!(((s[2].x, s[2].y), numbers(&s[2])), ((200, 10), vec![5, 6]));
    }

    #[test]
    fn the_route_visits_each_target_in_step_order() {
        let m = of(&scene(), (0, 0));
        assert_eq!(route(&m), [(100, 50), (10, 10), (200, 10)], "the drag ends where the scroll is");
        assert!(route(&[]).is_empty());
    }
}
