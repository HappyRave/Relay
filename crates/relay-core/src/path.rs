//! Reshaping a cursor path (a MOVE step): smoothing out a wobbly move or
//! straightening it. Playback replays every recorded sample and doesn't
//! interpolate, so a reshaped path keeps its samples, their times and its two
//! ends: only where the samples in between lie changes. Each sample keeps its
//! share of the distance travelled, so the move keeps its speed curve (the
//! hand speeding up, then slowing down onto the target).

/// How far (in pixels) a point may be from the simplified path before it
/// counts as a corner worth keeping, when smoothing.
pub const SMOOTH_TOLERANCE_PX: f64 = 8.0;

/// A cursor position, in pixels.
pub type Point = (i32, i32);

type Pt = (f64, f64);

fn pt((x, y): (i32, i32)) -> Pt {
    (x as f64, y as f64)
}

/// Plain `sqrt` rather than `hypot`: it's exactly rounded everywhere, so the
/// UI tests' copy of this module gets the same pixels.
fn dist(a: Pt, b: Pt) -> f64 {
    let (dx, dy) = (a.0 - b.0, a.1 - b.1);
    (dx * dx + dy * dy).sqrt()
}

/// Ramer–Douglas–Peucker: the fewest points from `points` (always both ends)
/// such that none of the others is further than `eps` from the result.
pub fn simplify(points: &[(i32, i32)], eps: f64) -> Vec<(i32, i32)> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    let mut spans = vec![(0, points.len() - 1)];
    while let Some((a, b)) = spans.pop() {
        let (pa, pb) = (pt(points[a]), pt(points[b]));
        let far = (a + 1..b).map(|i| (i, off_segment(pt(points[i]), pa, pb))).max_by(|x, y| x.1.total_cmp(&y.1));
        if let Some((i, d)) = far
            && d > eps
        {
            keep[i] = true;
            spans.push((a, i));
            spans.push((i, b));
        }
    }
    points.iter().zip(keep).filter(|(_, k)| *k).map(|(p, _)| *p).collect()
}

/// How far `p` is from the segment `a`–`b`.
fn off_segment(p: Pt, a: Pt, b: Pt) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return dist(p, a);
    }
    let k = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0);
    dist(p, (a.0 + k * dx, a.1 + k * dy))
}

/// One pass of Chaikin's corner cutting, keeping both ends: each corner is
/// replaced by two points a quarter of the way along its sides.
fn chaikin(poly: &[Pt]) -> Vec<Pt> {
    if poly.len() < 3 {
        return poly.to_vec();
    }
    let mut out = vec![poly[0]];
    for w in poly.windows(2) {
        let (a, b) = (w[0], w[1]);
        out.push((0.75 * a.0 + 0.25 * b.0, 0.75 * a.1 + 0.25 * b.1));
        out.push((0.25 * a.0 + 0.75 * b.0, 0.25 * a.1 + 0.75 * b.1));
    }
    // The cuts next to the ends would move them: the ends stay put instead.
    out.remove(1);
    out.pop();
    out.push(poly[poly.len() - 1]);
    out
}

/// Places each of `points` on `target` at the same share of the length it
/// had along `points`, rounded to pixels. The ends land on `target`'s ends.
fn resample(points: &[(i32, i32)], target: &[Pt]) -> Vec<(i32, i32)> {
    let mut along = vec![0.0];
    for w in points.windows(2) {
        along.push(along.last().unwrap() + dist(pt(w[0]), pt(w[1])));
    }
    let total = *along.last().unwrap();
    let mut seg = vec![0.0];
    for w in target.windows(2) {
        seg.push(seg.last().unwrap() + dist(w[0], w[1]));
    }
    let target_len = *seg.last().unwrap();
    let last = points.len() - 1;
    let mut j = 0;
    along
        .iter()
        .enumerate()
        .map(|(i, &d)| {
            if i == 0 {
                return points[0];
            }
            if i == last {
                return points[last];
            }
            let want = if total == 0.0 { 0.0 } else { d / total * target_len };
            while j + 2 < seg.len() && seg[j + 1] < want {
                j += 1;
            }
            let (a, b) = (target[j], target[j + 1]);
            let span = seg[j + 1] - seg[j];
            let k = if span == 0.0 { 0.0 } else { ((want - seg[j]) / span).clamp(0.0, 1.0) };
            ((a.0 + k * (b.0 - a.0)).round() as i32, (a.1 + k * (b.1 - a.1)).round() as i32)
        })
        .collect()
}

/// `points` with the wobble taken out: simplified to its corners, which are
/// then rounded off. Smoothing again smooths further; a straight path stays.
pub fn smooth(points: &[(i32, i32)]) -> Vec<(i32, i32)> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let corners: Vec<Pt> = simplify(points, SMOOTH_TOLERANCE_PX).into_iter().map(pt).collect();
    resample(points, &chaikin(&corners))
}

/// `points` on the straight line from the first to the last.
pub fn straighten(points: &[(i32, i32)]) -> Vec<(i32, i32)> {
    if points.len() < 3 {
        return points.to_vec();
    }
    resample(points, &[pt(points[0]), pt(points[points.len() - 1])])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A wobbly move from (0, 0) to (300, 0), with a bump in the middle.
    fn wobbly() -> Vec<(i32, i32)> {
        (0..=30)
            .map(|i: i32| (i * 10, if i % 2 == 0 { 3 } else { -3 } + if (12..=18).contains(&i) { 40 } else { 0 }))
            .collect()
    }

    fn ends(p: &[(i32, i32)]) -> ((i32, i32), (i32, i32)) {
        (p[0], p[p.len() - 1])
    }

    #[test]
    fn simplify_keeps_the_corners_only() {
        let l = [(0, 0), (10, 1), (20, 0), (30, 30), (40, 60), (50, 61), (60, 60)];
        assert_eq!(simplify(&l, 2.0), vec![(0, 0), (20, 0), (40, 60), (60, 60)]);
        assert_eq!(simplify(&l, 100.0), vec![(0, 0), (60, 60)]);
        assert_eq!(simplify(&[(0, 0), (5, 5)], 1.0), vec![(0, 0), (5, 5)]);
    }

    #[test]
    fn reshaping_keeps_the_ends_and_the_count() {
        let w = wobbly();
        for out in [smooth(&w), straighten(&w)] {
            assert_eq!(out.len(), w.len());
            assert_eq!(ends(&out), ends(&w));
        }
    }

    #[test]
    fn straighten_puts_every_sample_on_the_line_in_order() {
        let s = straighten(&wobbly());
        assert!(s.iter().all(|&(_, y)| y == 3), "{s:?}");
        assert!(s.windows(2).all(|w| w[0].0 <= w[1].0), "in order: {s:?}");
    }

    #[test]
    fn straighten_keeps_the_speed_curve() {
        // Slow, then fast: the samples bunch up at the start.
        let p = [(0, 0), (1, 5), (2, 0), (50, 5), (100, 0)];
        let s = straighten(&p);
        assert!(s[1].0 < 10 && s[2].0 < 10 && s[3].0 > 30, "{s:?}");
    }

    #[test]
    fn smooth_takes_the_wobble_out_but_keeps_the_bump() {
        let s = smooth(&wobbly());
        let wobble = |p: &[(i32, i32)]| p.windows(3).filter(|w| (w[1].1 - w[0].1) * (w[2].1 - w[1].1) < 0).count();
        assert!(wobble(&s) <= 2, "no more zigzag: {s:?}");
        assert!(s.iter().any(|&(_, y)| y > 25), "the bump stays: {s:?}");
    }

    #[test]
    fn a_straight_path_stays_straight_and_repeats_settle() {
        let line: Vec<_> = (0..=10).map(|i| (i * 10, i * 5)).collect();
        assert_eq!(smooth(&line), line);
        assert_eq!(straighten(&line), line);
        let mut p = wobbly();
        for _ in 0..50 {
            p = smooth(&p);
        }
        assert_eq!(smooth(&p), p, "smoothing converges");
    }

    #[test]
    fn a_move_that_goes_nowhere_is_left_alone() {
        let still = [(5, 5), (5, 5), (5, 5)];
        assert_eq!(smooth(&still), still);
        assert_eq!(straighten(&still), still);
        let there_and_back = [(0, 0), (50, 0), (0, 0)];
        assert_eq!(straighten(&there_and_back), [(0, 0), (0, 0), (0, 0)]);
    }

    /// Writes Rust's answers for a few paths, which the UI tests' fake core
    /// (`src/test/fake-edit.ts`) must reproduce, like ts-rs does for the bindings.
    #[test]
    fn export_path_cases() {
        let spiral: Vec<_> = (0..40)
            .map(|i: i32| {
                let a = i as f64 * 0.3;
                ((a.cos() * i as f64 * 4.0).round() as i32 + 500, (a.sin() * i as f64 * 4.0).round() as i32 + 300)
            })
            .collect();
        let inputs = [
            wobbly(),
            spiral,
            vec![(0, 0), (1, 5), (2, 0), (50, 5), (100, 0)],
            vec![(0, 0), (50, 0), (0, 0)],
            vec![(7, 7), (7, 7), (7, 7), (8, 7)],
            vec![(3, 4), (9, 12)],
        ];
        let cases: Vec<_> = inputs
            .iter()
            .map(|p| serde_json::json!({ "points": p, "smooth": smooth(p), "straighten": straighten(p) }))
            .collect();
        let body = serde_json::to_string(&cases).unwrap()
            + "
";
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src/test/path-cases.json");
        if std::fs::read_to_string(&path).ok().as_deref() != Some(body.as_str()) {
            std::fs::write(&path, body).unwrap();
        }
    }
}
