//! Where the player's window goes and how it's laid out, at any display scaling.

use relay_core::model::{Event, Macro, Rect};

/// The window's parts, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    pub w: i32,
    pub h: i32,
    pub border: i32,
    pub name: Rect,
    pub line: Rect,
    pub notice: Rect,
    pub bar: Rect,
    pub stop: Rect,
    pub name_px: i32,
    pub text_px: i32,
}

/// The layout at `dpi` (96 is 100 % scaling).
pub fn layout(dpi: u32) -> Layout {
    let s = |v: i32| (v as f64 * dpi.max(48) as f64 / 96.0).round() as i32;
    let (w, h, pad) = (s(320), s(104), s(12));
    let stop = Rect { x: w - pad - s(72), y: pad, w: s(72), h: s(28) };
    let text_w = stop.x - pad - s(8);
    Layout {
        w,
        h,
        border: s(2),
        name: Rect { x: pad, y: pad, w: text_w, h: s(18) },
        line: Rect { x: pad, y: pad + s(20), w: text_w, h: s(16) },
        notice: Rect { x: pad, y: pad + s(42), w: w - 2 * pad, h: s(28) },
        bar: Rect { x: s(2), y: h - s(2) - s(4), w: w - s(4), h: s(4) },
        stop,
        name_px: s(15),
        text_px: s(12),
    }
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
        assert_eq!((l.w, l.h, l.border, l.name_px, l.text_px), (320, 104, 2, 15, 12));
        assert_eq!(l.stop, Rect { x: 236, y: 12, w: 72, h: 28 });
        assert_eq!(l.bar, Rect { x: 2, y: 98, w: 316, h: 4 });
        let big = layout(144);
        assert_eq!((big.w, big.h, big.border, big.name_px), (480, 156, 3, 23));
        assert_eq!(layout(192).stop, Rect { x: 472, y: 24, w: 144, h: 56 });
        for dpi in [96, 120, 144, 192] {
            let l = layout(dpi);
            let inside = |r: Rect| r.x >= 0 && r.y >= 0 && r.x + r.w <= l.w && r.y + r.h <= l.h;
            assert!([l.name, l.line, l.notice, l.bar, l.stop].into_iter().all(inside), "{dpi}: {l:?}");
            assert!(l.name.x + l.name.w < l.stop.x && l.notice.y + l.notice.h <= l.bar.y, "{dpi}: {l:?}");
        }
    }

    #[test]
    fn it_goes_bottom_right_when_nothing_happens_there() {
        let (r, through) = choose_place(WORK, 320, 104, 16, &[(100, 100)]);
        assert_eq!((r, through), (Rect { x: 1584, y: 920, w: 320, h: 104 }, false));
        let second = Rect { x: -1920, y: 0, w: 1920, h: 1040 };
        assert_eq!(choose_place(second, 320, 104, 16, &[]).0, Rect { x: -336, y: 920, w: 320, h: 104 });
    }

    #[test]
    fn it_moves_away_from_the_macros_clicks() {
        let bottom_right = (1700, 1000);
        assert_eq!(choose_place(WORK, 320, 104, 16, &[bottom_right]).0, Rect { x: 16, y: 920, w: 320, h: 104 });
        let bottoms = [bottom_right, (100, 1000)];
        assert_eq!(choose_place(WORK, 320, 104, 16, &bottoms).0, Rect { x: 1584, y: 16, w: 320, h: 104 });
        let all_but_top_left = [bottom_right, (100, 1000), (1700, 50)];
        assert_eq!(choose_place(WORK, 320, 104, 16, &all_but_top_left).0, Rect { x: 16, y: 16, w: 320, h: 104 });
    }

    #[test]
    fn with_every_corner_busy_it_lets_clicks_through() {
        let all = [(1700, 1000), (100, 1000), (1700, 50), (100, 50)];
        let (r, through) = choose_place(WORK, 320, 104, 16, &all);
        assert_eq!((r, through), (Rect { x: 1584, y: 920, w: 320, h: 104 }, true));
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
