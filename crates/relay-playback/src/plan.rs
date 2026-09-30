//! Getting a macro ready to play: its [`PlayPlan`], where "Window"
//! coordinates put it now, and whether it clicks inside a window of ours.

use relay_core::model::{CoordMode, Event, Macro, Ms, Rect, WindowInfo};
use relay_core::steps::group_steps;
use relay_core::timeline;

use crate::engine::PlayPlan;

impl PlayPlan {
    /// Plays `m` with its saved playback options, from `from`.
    pub fn for_macro(m: Macro, from: Ms, seed: u64, offset: (i32, i32), own_window: isize) -> PlayPlan {
        PlayPlan {
            duration: timeline::duration(&m.events),
            repeat: m.playback.repeat,
            speed: m.playback.speed as f64,
            jitter_ms: if m.playback.humanize { m.playback.jitter_ms } else { 0 },
            seed,
            offset,
            from,
            own_window,
            steps: group_steps(&m.events, (&m.recording).into()),
            events: m.events,
        }
    }
}

/// For "Window" coordinates: how far the anchor window moved since recording,
/// found with `find_window(exe, class)`. When it can't be used, plays at
/// screen coordinates and says why.
pub fn window_offset(
    m: &Macro,
    find_window: impl FnOnce(&str, &str) -> Option<WindowInfo>,
) -> ((i32, i32), Option<String>) {
    if m.playback.coord_mode != CoordMode::Window {
        return ((0, 0), None);
    }
    let Some(anchor) = &m.recording.anchor_window else {
        return ((0, 0), Some("This macro has no anchor window; playing at screen coordinates.".into()));
    };
    match find_window(&anchor.exe, &anchor.class) {
        Some(now) => ((now.rect.x - anchor.rect.x, now.rect.y - anchor.rect.y), None),
        None => ((0, 0), Some(format!("Couldn't find {}; playing at screen coordinates.", anchor.exe))),
    }
}

/// Whether the macro clicks or scrolls inside `r` when played `offset` away
/// from where it was recorded.
pub fn clicks_inside(m: &Macro, r: Rect, offset: (i32, i32)) -> bool {
    m.events.iter().any(|e| {
        matches!(e, Event::Button { x, y, .. } | Event::Wheel { x, y, .. } if r.contains(x + offset.0, y + offset.1))
    })
}

#[cfg(test)]
mod tests {
    use relay_core::model::{MouseBtn, RecordingMeta, Repeat};

    use super::*;

    fn clicking_at(x: i32, y: i32) -> Macro {
        let mut events = vec![Event::Move { t: 0, x: 500, y: 500 }];
        events.push(Event::Button { t: 10, x, y, btn: MouseBtn::Left, down: true, label: String::new() });
        events.push(Event::Button { t: 20, x, y, btn: MouseBtn::Left, down: false, label: String::new() });
        Macro::new("m", RecordingMeta::single_1080p(), events)
    }

    fn window(exe: &str, x: i32, y: i32) -> WindowInfo {
        WindowInfo { exe: exe.into(), class: "Main".into(), title: String::new(), rect: Rect { x, y, w: 800, h: 600 } }
    }

    #[test]
    fn a_plan_uses_the_saved_options() {
        let mut m = clicking_at(900, 700);
        m.playback.repeat = Repeat::Count(3);
        m.playback.speed = 2.0;
        m.playback.jitter_ms = 40;
        m.playback.humanize = false;
        let plan = PlayPlan::for_macro(m.clone(), 10, 7, (5, 6), 42);
        assert_eq!(
            (plan.repeat, plan.speed, plan.jitter_ms, plan.seed, plan.offset, plan.from, plan.own_window),
            (Repeat::Count(3), 2.0, 0, 7, (5, 6), 10, 42),
            "no jitter without Humanize"
        );
        assert_eq!(plan.duration, timeline::duration(&m.events));
        assert_eq!(plan.steps, group_steps(&m.events, (&m.recording).into()));
        assert_eq!(plan.events, m.events);

        m.playback.humanize = true;
        assert_eq!(PlayPlan::for_macro(m, 0, 0, (0, 0), 0).jitter_ms, 40);
    }

    #[test]
    fn screen_coordinates_never_move() {
        let m = clicking_at(900, 700);
        assert_eq!(window_offset(&m, |_, _| panic!("not looked up")), ((0, 0), None));
    }

    #[test]
    fn window_coordinates_follow_the_anchor_window() {
        let mut m = clicking_at(900, 700);
        m.playback.coord_mode = CoordMode::Window;
        assert_eq!(
            window_offset(&m, |_, _| None),
            ((0, 0), Some("This macro has no anchor window; playing at screen coordinates.".into()))
        );

        m.recording.anchor_window = Some(window("EXCEL.EXE", 100, 50));
        let moved = window_offset(&m, |exe, class| {
            assert_eq!((exe, class), ("EXCEL.EXE", "Main"));
            Some(window("EXCEL.EXE", 600, 450))
        });
        assert_eq!(moved, ((500, 400), None));
        assert_eq!(
            window_offset(&m, |_, _| None),
            ((0, 0), Some("Couldn't find EXCEL.EXE; playing at screen coordinates.".into()))
        );
    }

    #[test]
    fn clicks_under_a_window_are_found() {
        let widget = Rect { x: 488, y: 444, w: 944, h: 612 };
        assert!(clicks_inside(&clicking_at(900, 700), widget, (0, 0)));
        assert!(!clicks_inside(&clicking_at(100, 100), widget, (0, 0)));
        // Moves don't count: only a click needs to reach the app beneath.
        assert!(!clicks_inside(&clicking_at(100, 100), Rect { x: 490, y: 490, w: 20, h: 20 }, (0, 0)));
    }

    #[test]
    fn the_window_offset_moves_the_clicks() {
        let widget = Rect { x: 488, y: 444, w: 944, h: 612 };
        // Recorded at (100, 100); the anchor window moved by (500, 400).
        assert!(clicks_inside(&clicking_at(100, 100), widget, (500, 400)));
        assert!(!clicks_inside(&clicking_at(900, 700), widget, (-800, 0)));
    }

    #[test]
    fn scrolling_under_a_window_counts_too() {
        let widget = Rect { x: 488, y: 444, w: 944, h: 612 };
        let scrolling = |x, y| {
            let events = vec![Event::Wheel { t: 10, x, y, delta: -120, horizontal: false }];
            Macro::new("m", RecordingMeta::single_1080p(), events)
        };
        assert!(clicks_inside(&scrolling(900, 700), widget, (0, 0)));
        assert!(!clicks_inside(&scrolling(100, 100), widget, (0, 0)));
        assert!(clicks_inside(&scrolling(100, 100), widget, (500, 400)), "moved with the window");
    }
}
