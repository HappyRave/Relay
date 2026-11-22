//! How *Show targets* looks (design round T1: live marks are "T1 Ripple",
//! the still overlay is "O1 Route"): sizes at 100% scaling, timings, and
//! where captions and number badges go. The OS backend draws it.

use relay_core::marks::{Mark, MarkKind};
use relay_core::model::{MouseBtn, Rect};

/// Relay's red, as on the player.
pub const RED: (u8, u8, u8) = (0xE0, 0x30, 0x0F);
pub const INK: (u8, u8, u8) = (0x14, 0x14, 0x15);

/// The dot on the exact pixel.
pub const DOT_R: f32 = 4.2;
/// A ripple grows from this radius to `RING_TO` over `RIPPLE_MS`.
pub const RING_FROM: f32 = 6.0;
pub const RING_TO: f32 = 26.0;
pub const RIPPLE_MS: f64 = 650.0;
/// The second ripple of a double click starts this much later.
pub const DOUBLE_GAP_MS: f64 = 170.0;
pub const LINE_W: f32 = 2.5;
/// The white edge around red lines, so they show on red too.
pub const HALO_W: f32 = 2.5;

pub const CLICK_MS: f64 = 1000.0;
pub const SCROLL_MS: f64 = 1100.0;
pub const CAPTION_MS: f64 = 1100.0;
/// How long a drag stays after it has drawn, and a move's trail.
pub const DRAG_HOLD_MS: f64 = 900.0;
pub const MOVE_HOLD_MS: f64 = 600.0;
pub const FADE_MS: f64 = 350.0;
/// How fast a caption gives way to the next one.
pub const REPLACED_MS: f64 = 150.0;
/// Longer typed text is cut, with an ellipsis.
pub const CAPTION_CHARS: usize = 22;

/// Opacity at `age` of a mark that lives `life` ms and fades over its last `out`.
pub fn fade(age: f64, life: f64, out: f64) -> f32 {
    if age < 0.0 || age >= life {
        0.0
    } else if age > life - out {
        ((life - age) / out) as f32
    } else {
        1.0
    }
}

pub fn ease(u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    if u < 0.5 { 2.0 * u * u } else { 1.0 - (-2.0 * u + 2.0).powi(2) / 2.0 }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ring {
    pub r: f32,
    pub alpha: f32,
}

/// The rings of a click `age` ms in: one, or two for a double click.
pub fn ripples(age: f64, count: u8) -> [Option<Ring>; 2] {
    let ring = |a: f64| {
        (0.0..RIPPLE_MS * 1.15).contains(&a).then(|| Ring {
            r: RING_FROM + (RING_TO - RING_FROM) * ease(a / RIPPLE_MS) as f32,
            alpha: (1.0 - a / (RIPPLE_MS * 1.15)) as f32,
        })
    };
    [ring(age), if count >= 2 { ring(age - DOUBLE_GAP_MS) } else { None }]
}

/// Whether a click ripples as a rounded square (right clicks) or a circle.
pub fn square(btn: MouseBtn) -> bool {
    btn == MouseBtn::Right
}

/// How long `mark` shows, its path drawing over `draw_ms`.
pub fn life(mark: &Mark, draw_ms: f64) -> f64 {
    match mark.kind {
        MarkKind::Click { .. } => CLICK_MS,
        MarkKind::Scroll { .. } => SCROLL_MS,
        MarkKind::Keys { .. } | MarkKind::Text { .. } => CAPTION_MS,
        MarkKind::Path { drag: true, .. } => draw_ms + DRAG_HOLD_MS,
        MarkKind::Path { drag: false, .. } => draw_ms + MOVE_HOLD_MS,
    }
}

/// How long a path takes to draw: its time in the macro, at the playback's speed.
pub fn draw_ms(mark: &Mark, speed: f64) -> f64 {
    (mark.end.saturating_sub(mark.t) as f64 / speed.max(0.01)).max(1.0)
}

/// How much of a path shows `age` ms in.
pub fn progress(age: f64, draw_ms: f64) -> f64 {
    ease(age / draw_ms)
}

/// The points of `points` up to `p` (0 to 1) of its length; the last one may
/// fall between two.
pub fn partial(points: &[(i32, i32)], p: f64) -> Vec<(f32, f32)> {
    let seg = |a: (i32, i32), b: (i32, i32)| (((b.0 - a.0) as f64).powi(2) + ((b.1 - a.1) as f64).powi(2)).sqrt();
    let total: f64 = points.windows(2).map(|w| seg(w[0], w[1])).sum();
    let mut left = total * p.clamp(0.0, 1.0);
    let mut out: Vec<(f32, f32)> = points.first().map(|&(x, y)| (x as f32, y as f32)).into_iter().collect();
    for w in points.windows(2) {
        let d = seg(w[0], w[1]);
        if left >= d {
            out.push((w[1].0 as f32, w[1].1 as f32));
            left -= d;
        } else {
            let u = if d > 0.0 { left / d } else { 0.0 };
            out.push((
                (w[0].0 as f64 + (w[1].0 - w[0].0) as f64 * u) as f32,
                (w[0].1 as f64 + (w[1].1 - w[0].1) as f64 * u) as f32,
            ));
            break;
        }
    }
    out
}

/// What a caption says: keycaps, or quoted text.
#[derive(Debug, Clone, PartialEq)]
pub enum Caption {
    Keys(Vec<String>),
    Text(String),
}

pub fn caption(kind: &MarkKind) -> Option<Caption> {
    match kind {
        MarkKind::Keys { combo } => Some(Caption::Keys(combo.clone())),
        MarkKind::Text { text } => Some(Caption::Text(format!("“{}”", cut(text, CAPTION_CHARS)))),
        _ => None,
    }
}

/// `text` on one line, at most `n` characters, with an ellipsis when cut.
pub fn cut(text: &str, n: usize) -> String {
    let line: String = text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    if line.chars().count() <= n { line } else { line.chars().take(n.saturating_sub(1)).chain(['…']).collect() }
}

/// Where a `w` × `h` caption goes for the cursor at `x, y`: below and to the
/// right, flipped to the left or above where it would leave `area`.
pub fn place_caption(x: i32, y: i32, w: i32, h: i32, area: Rect, scale: f32) -> (i32, i32) {
    let gap = (18.0 * scale) as i32;
    let mut px = x + gap;
    let mut py = y + (16.0 * scale) as i32;
    if px + w > area.x + area.w {
        px = x - gap - w;
    }
    if py + h > area.y + area.h {
        py = y - gap - h;
    }
    (px.max(area.x), py.max(area.y))
}

/// Where the row of `count` number badges of a spot starts (its top left,
/// each badge `size` wide): above and to the right of the spot, flipped to
/// the left near the right edge and below near the top.
pub fn place_badges(x: i32, y: i32, count: usize, size: i32, area: Rect, scale: f32) -> (i32, i32) {
    let gap = (8.0 * scale) as i32;
    let w = count as i32 * size;
    let mut bx = x + gap;
    let mut by = y - gap - size;
    if bx + w > area.x + area.w {
        bx = x - gap - w;
    }
    if by < area.y {
        by = y + gap;
    }
    (bx, by)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mark(kind: MarkKind, t: u32, end: u32) -> Mark {
        Mark { first: 0, number: Some(1), t, end, x: 0, y: 0, kind }
    }
    const SCREEN: Rect = Rect { x: 0, y: 0, w: 1920, h: 1080 };

    #[test]
    fn a_mark_fades_over_the_end_of_its_life() {
        assert_eq!(fade(-1.0, 1000.0, 350.0), 0.0);
        assert_eq!(fade(0.0, 1000.0, 350.0), 1.0);
        assert_eq!(fade(650.0, 1000.0, 350.0), 1.0);
        assert!((fade(825.0, 1000.0, 350.0) - 0.5).abs() < 1e-6);
        assert_eq!(fade(1000.0, 1000.0, 350.0), 0.0);
    }

    #[test]
    fn a_click_ripples_out_from_the_dot_and_a_double_click_twice() {
        let [a, b] = ripples(0.0, 1);
        assert_eq!(a, Some(Ring { r: RING_FROM, alpha: 1.0 }));
        assert_eq!(b, None);
        assert_eq!(ripples(RIPPLE_MS, 1)[0].unwrap().r, RING_TO);
        assert_eq!(ripples(RIPPLE_MS * 1.2, 1), [None, None], "gone before the dot fades");
        let [a, b] = ripples(100.0, 2);
        assert!(a.is_some());
        assert_eq!(b, None, "the second starts {DOUBLE_GAP_MS} ms later");
        let [a, b] = ripples(DOUBLE_GAP_MS + 1.0, 2);
        assert!(b.unwrap().r < a.unwrap().r);
        assert!(square(MouseBtn::Right) && !square(MouseBtn::Left));
    }

    #[test]
    fn paths_draw_at_the_playback_speed_then_hold() {
        let drag = mark(MarkKind::Path { points: vec![], drag: true }, 1000, 1800);
        assert_eq!(draw_ms(&drag, 1.0), 800.0);
        assert_eq!(draw_ms(&drag, 4.0), 200.0);
        assert_eq!(life(&drag, 200.0), 200.0 + DRAG_HOLD_MS);
        let mv = mark(MarkKind::Path { points: vec![], drag: false }, 0, 0);
        assert_eq!(draw_ms(&mv, 1.0), 1.0, "never zero");
        assert_eq!(life(&mv, 1.0), 1.0 + MOVE_HOLD_MS);
        assert_eq!(life(&mark(MarkKind::Click { btn: MouseBtn::Left, count: 1 }, 0, 0), 9.0), CLICK_MS);
        assert_eq!(progress(400.0, 800.0), 0.5);
        assert_eq!(progress(900.0, 800.0), 1.0);
    }

    #[test]
    fn a_partial_path_stops_part_way_along_it() {
        let pts = [(0, 0), (10, 0), (10, 30)];
        assert_eq!(partial(&pts, 0.0), [(0.0, 0.0), (0.0, 0.0)]);
        assert_eq!(partial(&pts, 0.5), [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
        assert_eq!(partial(&pts, 1.0), [(0.0, 0.0), (10.0, 0.0), (10.0, 30.0)]);
        assert!(partial(&[], 0.5).is_empty());
        assert_eq!(partial(&[(3, 4)], 1.0), [(3.0, 4.0)]);
    }

    #[test]
    fn captions_show_keycaps_or_quoted_text_cut_to_one_line() {
        assert_eq!(
            caption(&MarkKind::Keys { combo: vec!["Ctrl".into(), "S".into()] }),
            Some(Caption::Keys(vec!["Ctrl".into(), "S".into()]))
        );
        assert_eq!(caption(&MarkKind::Text { text: "hi".into() }), Some(Caption::Text("“hi”".into())));
        let long = caption(&MarkKind::Text { text: "Quarterly report for Northwind".into() });
        assert_eq!(long, Some(Caption::Text("“Quarterly report for …”".into())));
        assert_eq!(cut("a\nb", 9), "a b");
        assert_eq!(cut("ééééé", 3), "éé…", "characters, not bytes");
        assert_eq!(caption(&MarkKind::Scroll { delta: 1, horizontal: false }), None);
    }

    #[test]
    fn a_caption_goes_below_right_and_flips_at_the_edges() {
        assert_eq!(place_caption(100, 100, 80, 26, SCREEN, 1.0), (118, 116));
        assert_eq!(place_caption(1900, 100, 80, 26, SCREEN, 1.0), (1802, 116), "flipped left");
        assert_eq!(place_caption(100, 1070, 80, 26, SCREEN, 1.0), (118, 1026), "flipped up");
        assert_eq!(place_caption(100, 100, 80, 26, SCREEN, 2.0), (136, 132), "scaled");
        let second = Rect { x: 1920, y: 0, w: 1920, h: 1080 };
        assert_eq!(place_caption(1925, 100, 80, 26, second, 1.0), (1943, 116), "on the monitor it's on");
    }

    #[test]
    fn badges_go_above_right_and_flip_at_the_edges() {
        assert_eq!(place_badges(100, 100, 2, 22, SCREEN, 1.0), (108, 70));
        assert_eq!(place_badges(1910, 100, 2, 22, SCREEN, 1.0), (1858, 70), "flipped left");
        assert_eq!(place_badges(100, 10, 1, 22, SCREEN, 1.0), (108, 18), "below near the top");
    }
}
