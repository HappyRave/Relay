//! Where the player's window goes and how it's laid out, at any display scaling.

use relay_core::model::{Event, Macro, Rect};

/// The window's parts, in physical pixels: like Relay's player, without the
/// record key. Pause and Stop, then a screen with the state, the name and the
/// time on its first line, and the progress (or a notice) on its second.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    pub w: i32,
    pub h: i32,
    pub pause: Rect,
    pub stop: Rect,
    pub screen: Rect,
    /// The screen's first line, and its second.
    pub top: Rect,
    pub bottom: Rect,
    /// The keys' corners, and the screen's.
    pub key_radius: i32,
    pub screen_radius: i32,
    pub name_px: i32,
    pub small_px: i32,
    /// How thick lines are (1 px at 100 %).
    pub line: i32,
}

/// The progress's segments, as in Relay's player.
pub const LEDS: usize = 48;

/// The layout at `dpi` (96 is 100 % scaling).
pub fn layout(dpi: u32) -> Layout {
    let s = |v: i32| (v as f64 * dpi.max(48) as f64 / 96.0).round() as i32;
    let (w, h, pad, gap, key) = (s(400), s(64), s(8), s(6), s(48));
    let pause = Rect { x: pad, y: s(9), w: key, h: s(46) };
    let stop = Rect { x: pause.x + key + gap, ..pause };
    let screen = Rect { x: stop.x + key + gap, y: pad, w: w - (stop.x + key + gap) - pad, h: h - 2 * pad };
    let inner = Rect { x: screen.x + s(12), y: screen.y + s(6), w: screen.w - s(12) - s(10), h: screen.h - s(13) };
    Layout {
        w,
        h,
        pause,
        stop,
        screen,
        top: Rect { h: s(20), ..inner },
        bottom: Rect { y: inner.y + inner.h - s(12), h: s(12), ..inner },
        key_radius: s(11),
        screen_radius: s(10),
        name_px: s(13),
        small_px: s(12),
        line: s(1).max(1),
    }
}

/// Where the macro's clicks are along it, 0 to 1: the red segments of the progress.
pub fn click_marks(m: &Macro) -> Vec<f32> {
    let duration = relay_core::timeline::duration(&m.events).max(1) as f32;
    m.events
        .iter()
        .filter_map(|e| match e {
            Event::Button { t, down: true, .. } => Some(*t as f32 / duration),
            _ => None,
        })
        .collect()
}

/// Which of the [`LEDS`] segments are lit at `progress`, and which mark a click.
pub fn leds(progress: f32, marks: &[f32]) -> [(bool, bool); LEDS] {
    let last = (LEDS - 1) as f32;
    let lit = (progress.clamp(0.0, 1.0) * last).round() as usize;
    let mut out = [(false, false); LEDS];
    for (i, led) in out.iter_mut().enumerate() {
        led.0 = progress > 0.0 && i <= lit;
    }
    for m in marks {
        out[(m.clamp(0.0, 1.0) * last).round() as usize].1 = true;
    }
    out
}

/// Where the macro clicks, scrolls or checks a pixel, played `offset` away
/// from where it was recorded: places the window shouldn't cover.
pub fn busy_points(m: &Macro, offset: (i32, i32)) -> Vec<(i32, i32)> {
    let at = |x: &i32, y: &i32| Some((x + offset.0, y + offset.1));
    m.events
        .iter()
        .filter_map(|e| match e {
            Event::Button { x, y, .. } | Event::Wheel { x, y, .. } | Event::PixelWait { x, y, .. } => at(x, y),
            _ => None,
        })
        .collect()
}

/// A `w`×`h` window in a corner of `work` (the primary monitor's work
/// area), `margin` from its edges: bottom-right, else the first other corner
/// with none of `busy` under it. When every corner is busy, bottom-right and
/// click-through (`true`), so the macro's clicks reach the app beneath.
pub fn choose_place(work: Rect, w: i32, h: i32, margin: i32, busy: &[(i32, i32)]) -> (Rect, bool) {
    let (left, top) = (work.x + margin, work.y + margin);
    let (right, bottom) = (work.x + work.w - margin - w, work.y + work.h - margin - h);
    let corners = [(right, bottom), (left, bottom), (right, top), (left, top)];
    let rect = |(x, y): (i32, i32)| Rect { x, y, w, h };
    match corners.into_iter().map(rect).find(|r| !busy.iter().any(|&(x, y)| r.contains(x, y))) {
        Some(r) => (r, false),
        None => (rect(corners[0]), true),
    }
}

#[cfg(test)]
mod tests {
    use relay_core::model::{MouseBtn, RecordingMeta, Rgb};

    use super::*;

    const WORK: Rect = Rect { x: 0, y: 0, w: 1920, h: 1040 };

    #[test]
    fn the_layout_scales_with_the_display() {
        let l = layout(96);
        assert_eq!((l.w, l.h, l.name_px, l.small_px), (400, 64, 13, 12));
        assert_eq!(l.pause, Rect { x: 8, y: 9, w: 48, h: 46 });
        assert_eq!(l.stop, Rect { x: 62, y: 9, w: 48, h: 46 });
        assert_eq!(l.screen, Rect { x: 116, y: 8, w: 276, h: 48 });
        let big = layout(144);
        assert_eq!((big.w, big.h, big.name_px), (600, 96, 20));
        assert_eq!(layout(192).stop, Rect { x: 124, y: 18, w: 96, h: 92 });
        for dpi in [96, 120, 144, 192] {
            let l = layout(dpi);
            let inside =
                |r: Rect, o: Rect| r.x >= o.x && r.y >= o.y && r.x + r.w <= o.x + o.w && r.y + r.h <= o.y + o.h;
            let window = Rect { x: 0, y: 0, w: l.w, h: l.h };
            assert!([l.pause, l.stop, l.screen].into_iter().all(|r| inside(r, window)), "{dpi}: {l:?}");
            assert!(inside(l.top, l.screen) && inside(l.bottom, l.screen), "{dpi}: {l:?}");
            assert!(l.top.y + l.top.h <= l.bottom.y && l.stop.x + l.stop.w < l.screen.x, "{dpi}: {l:?}");
        }
    }

    #[test]
    fn the_progress_lights_segments_and_marks_the_clicks() {
        let none = leds(0.0, &[]);
        assert!(none.iter().all(|&(on, mark)| !on && !mark));
        let half = leds(0.5, &[0.0, 1.0]);
        assert_eq!(half.iter().filter(|l| l.0).count(), 25);
        assert!(half[0].1 && half[LEDS - 1].1 && !half[10].1);
        assert!(leds(1.0, &[]).iter().all(|l| l.0));
        let m = Macro::new(
            "m",
            RecordingMeta::single_1080p(),
            vec![
                Event::Button { t: 0, x: 1, y: 1, btn: MouseBtn::Left, down: true, label: String::new() },
                Event::Button { t: 10, x: 1, y: 1, btn: MouseBtn::Left, down: false, label: String::new() },
                Event::Button { t: 1000, x: 1, y: 1, btn: MouseBtn::Left, down: true, label: String::new() },
                Event::Button { t: 1010, x: 1, y: 1, btn: MouseBtn::Left, down: false, label: String::new() },
            ],
        );
        let marks = click_marks(&m);
        assert_eq!(marks.len(), 2);
        assert_eq!(marks[0], 0.0);
        let duration = relay_core::timeline::duration(&m.events) as f32;
        assert!((marks[1] - 1000.0 / duration).abs() < 1e-6);
    }

    #[test]
    fn it_goes_bottom_right_when_nothing_happens_there() {
        let (r, through) = choose_place(WORK, 400, 64, 16, &[(100, 100)]);
        assert_eq!((r, through), (Rect { x: 1504, y: 960, w: 400, h: 64 }, false));
        let second = Rect { x: -1920, y: 0, w: 1920, h: 1040 };
        assert_eq!(choose_place(second, 400, 64, 16, &[]).0, Rect { x: -416, y: 960, w: 400, h: 64 });
    }

    #[test]
    fn it_moves_away_from_the_macros_clicks() {
        let bottom_right = (1700, 1000);
        assert_eq!(choose_place(WORK, 400, 64, 16, &[bottom_right]).0, Rect { x: 16, y: 960, w: 400, h: 64 });
        let bottoms = [bottom_right, (100, 1000)];
        assert_eq!(choose_place(WORK, 400, 64, 16, &bottoms).0, Rect { x: 1504, y: 16, w: 400, h: 64 });
        let all_but_top_left = [bottom_right, (100, 1000), (1700, 50)];
        assert_eq!(choose_place(WORK, 400, 64, 16, &all_but_top_left).0, Rect { x: 16, y: 16, w: 400, h: 64 });
    }

    #[test]
    fn with_every_corner_busy_it_lets_clicks_through() {
        let all = [(1700, 1000), (100, 1000), (1700, 50), (100, 50)];
        let (r, through) = choose_place(WORK, 400, 64, 16, &all);
        assert_eq!((r, through), (Rect { x: 1504, y: 960, w: 400, h: 64 }, true));
    }

    #[test]
    fn clicks_scrolls_and_pixel_checks_are_busy_points() {
        let events = vec![
            Event::Move { t: 0, x: 5, y: 5 },
            Event::Button { t: 10, x: 1700, y: 1000, btn: MouseBtn::Left, down: true, label: String::new() },
            Event::Button { t: 20, x: 1700, y: 1000, btn: MouseBtn::Left, down: false, label: String::new() },
            Event::Wheel { t: 30, x: 40, y: 50, delta: 120, horizontal: false },
            Event::PixelWait {
                t: 40,
                dur: 800,
                x: 7,
                y: 8,
                color: Rgb(0, 0, 0),
                tolerance: 8,
                timeout_ms: 5000,
                label: String::new(),
            },
        ];
        let m = Macro::new("m", RecordingMeta::single_1080p(), events);
        assert_eq!(busy_points(&m, (0, 0)), [(1700, 1000), (1700, 1000), (40, 50), (7, 8)]);
        assert_eq!(busy_points(&m, (10, -10))[2], (50, 40), "moved with the anchor window");
    }
}
