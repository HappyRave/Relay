//! Looking for an image on the real screen: a 1:1 capture of the area,
//! searched by `relay_core::image`.

use relay_core::image::{self, FindOpts, Gray, Match};
use relay_core::model::Rect;
use relay_platform::Screen;

/// The part of `a` inside `b`, if any.
fn intersect(a: Rect, b: Rect) -> Option<Rect> {
    let (x, y) = (a.x.max(b.x), a.y.max(b.y));
    let (r, bottom) = ((a.x + a.w).min(b.x + b.w), (a.y + a.h).min(b.y + b.h));
    (r > x && bottom > y).then_some(Rect { x, y, w: r - x, h: bottom - y })
}

/// The best match for `image` in `area` (the whole desktop if `None`),
/// leaving out the window `exclude`, in screen pixels. Also returns a match
/// below `threshold`, so callers can say how close it came.
pub fn best_on_screen(
    screen: &dyn Screen,
    image: &Gray,
    area: Option<Rect>,
    threshold: f32,
    exclude: isize,
) -> Option<Match> {
    let desktop = screen.virtual_desktop();
    let area = intersect(area.unwrap_or(desktop), desktop)?;
    let snap = screen.capture(area, area.w as u32, exclude)?;
    let gray = Gray::from_rgb(snap.w as usize, snap.h as usize, &snap.rgb);
    let m = image::find(&gray, image, FindOpts { threshold })?;
    Some(Match { x: m.x + area.x, y: m.y + area.y, ..m })
}

/// Where `image` is on screen, if it matches at least `threshold`.
pub fn find_on_screen(
    screen: &dyn Screen,
    image: &Gray,
    area: Option<Rect>,
    threshold: f32,
    exclude: isize,
) -> Option<Match> {
    best_on_screen(screen, image, area, threshold, exclude).filter(|m| m.score >= threshold)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn areas_are_clipped_to_the_desktop() {
        let desk = Rect { x: -1920, y: 0, w: 3840, h: 1080 };
        assert_eq!(
            intersect(Rect { x: -2000, y: -50, w: 500, h: 100 }, desk),
            Some(Rect { x: -1920, y: 0, w: 420, h: 50 })
        );
        assert_eq!(intersect(Rect { x: 1920, y: 0, w: 10, h: 10 }, desk), None);
        assert_eq!(intersect(desk, desk), Some(desk));
    }
}
