//! Looking for an image on the real screen: a 1:1 capture of the area,
//! searched by `relay_core::image`.

use std::collections::{HashMap, HashSet};

use relay_core::image::{self, FindOpts, Gray, Match};
use relay_core::model::Rect;
use relay_platform::{Screen, Snapshot};

/// The part of `a` inside `b`, if any.
fn intersect(a: Rect, b: Rect) -> Option<Rect> {
    let (x, y) = (a.x.max(b.x), a.y.max(b.y));
    let (r, bottom) = ((a.x + a.w).min(b.x + b.w), (a.y + a.h).min(b.y + b.h));
    (r > x && bottom > y).then_some(Rect { x, y, w: r - x, h: bottom - y })
}

/// A 1:1 picture of `area` (the whole desktop if `None`, clipped to it),
/// leaving out the window `exclude`, and where it is.
fn grab(screen: &dyn Screen, area: Option<Rect>, exclude: isize) -> Option<(Rect, Snapshot)> {
    let desktop = screen.virtual_desktop();
    let area = intersect(area.unwrap_or(desktop), desktop)?;
    Some((area, screen.capture(area, area.w as u32, exclude)?))
}

fn gray(snap: &Snapshot) -> Gray {
    Gray::from_rgb(snap.w as usize, snap.h as usize, &snap.rgb)
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
    let (area, snap) = grab(screen, area, exclude)?;
    let m = image::find(&gray(&snap), image, FindOpts { threshold })?;
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

/// Tells frames apart (FNV-1a over 8 bytes at a time: fast, and any change
/// counts).
fn fingerprint(bytes: &[u8]) -> u64 {
    let (words, rest) = bytes.as_chunks::<8>();
    let step = |h: u64, v: u64| (h ^ v).wrapping_mul(0x100_0000_01B3);
    let h = words.iter().fold(0xCBF2_9CE4_8422_2325, |h, w| step(h, u64::from_le_bytes(*w)));
    rest.iter().fold(h, |h, &b| step(h, b as u64))
}

/// One capture of an area, in a round.
struct Frame {
    snap: Snapshot,
    print: u64,
    gray: Option<Gray>,
}

/// What a search is: which image (by key), where, and how good a match.
type Search = (u64, Option<Rect>, u32);

/// Looks for the image triggers' images. Each area is captured once per
/// round, however many triggers look there, and an image isn't searched
/// again on a frame that hasn't changed since it last was.
#[derive(Default)]
pub struct Looker {
    frames: HashMap<Option<Rect>, Option<Frame>>,
    answers: HashMap<Search, (u64, bool)>,
    used: HashSet<Search>,
}

impl Looker {
    /// Starts a round: the screen is captured afresh.
    pub fn round(&mut self) {
        self.frames.clear();
        let used = std::mem::take(&mut self.used);
        self.answers.retain(|k, _| used.contains(k));
    }

    /// Whether `image` (told apart by `key`) is on screen in `area`, or
    /// `None` when the screen can't be read.
    pub fn look(
        &mut self,
        screen: &dyn Screen,
        image: &Gray,
        key: u64,
        area: Option<Rect>,
        threshold: f32,
        exclude: isize,
    ) -> Option<bool> {
        let frame = self
            .frames
            .entry(area)
            .or_insert_with(|| {
                grab(screen, area, exclude).map(|(_, snap)| Frame { print: fingerprint(&snap.rgb), snap, gray: None })
            })
            .as_mut()?;
        let search = (key, area, threshold.to_bits());
        self.used.insert(search);
        if let Some(&(print, found)) = self.answers.get(&search)
            && print == frame.print
        {
            return Some(found);
        }
        let g = frame.gray.get_or_insert_with(|| gray(&frame.snap));
        let found = image::find(g, image, FindOpts { threshold }).is_some_and(|m| m.score >= threshold);
        self.answers.insert(search, (frame.print, found));
        Some(found)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use relay_core::model::{MonitorInfo, Rgb};

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

    /// A 200×100 desktop at (-100, 0), light gray, with `patch` (a 16×16
    /// checkerboard) at (60, 40) when `shown`. Counts captures.
    struct FakeScreen {
        shown: Mutex<bool>,
        captures: Mutex<Vec<Rect>>,
    }

    fn patch() -> Gray {
        Gray { w: 16, h: 16, px: (0..256).map(|i| if (i % 16 / 4 + i / 64) % 2 == 0 { 20.0 } else { 220.0 }).collect() }
    }

    impl Screen for FakeScreen {
        fn monitors(&self) -> Vec<MonitorInfo> {
            Vec::new()
        }
        fn virtual_desktop(&self) -> Rect {
            Rect { x: -100, y: 0, w: 200, h: 100 }
        }
        fn cursor_pos(&self) -> (i32, i32) {
            (0, 0)
        }
        fn double_click(&self) -> (u32, u32) {
            (500, 4)
        }
        fn pixel(&self, _: i32, _: i32) -> Option<Rgb> {
            None
        }
        fn capture(&self, area: Rect, max_w: u32, exclude: isize) -> Option<Snapshot> {
            assert_eq!((max_w, exclude), (area.w as u32, 7), "1:1, without Relay's window");
            self.captures.lock().unwrap().push(area);
            let (p, shown) = (patch(), *self.shown.lock().unwrap());
            let mut rgb = Vec::new();
            for y in area.y..area.y + area.h {
                for x in area.x..area.x + area.w {
                    let (px, py) = (x - (-100 + 60), y - 40);
                    let inside = shown && (0..16).contains(&px) && (0..16).contains(&py);
                    let v = if inside { p.px[(py * 16 + px) as usize] as u8 } else { 200 };
                    rgb.extend([v, v, v]);
                }
            }
            Some(Snapshot { w: area.w as u32, h: area.h as u32, rgb })
        }
    }

    fn screen(shown: bool) -> FakeScreen {
        FakeScreen { shown: Mutex::new(shown), captures: Mutex::new(Vec::new()) }
    }

    #[test]
    fn finds_an_image_in_screen_pixels() {
        let s = screen(true);
        let m = find_on_screen(&s, &patch(), None, 0.85, 7).unwrap();
        assert_eq!((m.x, m.y, m.w, m.h), (-40, 40, 16, 16));
        let m = find_on_screen(&s, &patch(), Some(Rect { x: -50, y: 30, w: 400, h: 40 }), 0.85, 7).unwrap();
        assert_eq!((m.x, m.y), (-40, 40));
        assert_eq!(s.captures.lock().unwrap()[1], Rect { x: -50, y: 30, w: 150, h: 40 }, "clipped to the desktop");
        assert_eq!(find_on_screen(&screen(false), &patch(), None, 0.85, 7), None);
    }

    #[test]
    fn a_looker_captures_each_area_once_a_round_and_skips_unchanged_frames() {
        let s = screen(false);
        let mut looker = Looker::default();
        let left = Some(Rect { x: -100, y: 0, w: 100, h: 100 });
        looker.round();
        assert_eq!(looker.look(&s, &patch(), 1, None, 0.85, 7), Some(false));
        assert_eq!(looker.look(&s, &patch(), 2, None, 0.85, 7), Some(false), "another image, same capture");
        assert_eq!(looker.look(&s, &patch(), 1, left, 0.85, 7), Some(false));
        assert_eq!(s.captures.lock().unwrap().len(), 2, "one per area");

        // A new round captures again; the unchanged frame keeps its answer.
        looker.round();
        assert_eq!(looker.look(&s, &patch(), 1, None, 0.85, 7), Some(false));
        *s.shown.lock().unwrap() = true;
        looker.round();
        assert_eq!(looker.look(&s, &patch(), 1, None, 0.85, 7), Some(true), "the frame changed: searched again");
        assert_eq!(looker.look(&s, &patch(), 1, left, 0.85, 7), Some(true));
        assert_eq!(s.captures.lock().unwrap().len(), 5);
    }

    #[test]
    fn fingerprints_tell_any_change_apart() {
        let a = vec![7u8; 1003];
        let mut b = a.clone();
        b[1001] = 8;
        assert_ne!(fingerprint(&a), fingerprint(&b));
        assert_eq!(fingerprint(&a), fingerprint(&a.clone()));
    }
}
