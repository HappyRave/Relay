//! Picking a pixel from the screen, the parts that don't need the OS: where
//! the loupe and the help bar go, and what the picker's keys do. The Windows
//! backend draws them (`windows/picker.rs`).

use relay_core::model::{Rect, Rgb};

/// What was picked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Picked {
    pub x: i32,
    pub y: i32,
    pub color: Rgb,
    /// Picked from the frozen screen, not the live one.
    pub frozen: bool,
}

/// The loupe's diameter at 100% scaling (physical px times the monitor's scale).
pub const LOUPE: i32 = 132;
/// The tag under the loupe: the coordinates and the color.
pub const TAG_H: i32 = 22;
/// The pixels the loupe magnifies: 11 × 11, the picked one in the middle.
pub const SPAN: i32 = 11;
/// Between the cursor and the loupe.
const GAP: i32 = 24;
/// The help bar's distance from the work area's top (or bottom) edge.
pub const HELP_MARGIN: i32 = 14;

/// What a key does while picking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickKey {
    /// Moves the cursor by this many pixels.
    Move(i32, i32),
    Freeze,
    Pick,
    Cancel,
}

/// The picker's keys: the arrows move one pixel (ten with Shift), Space
/// freezes or goes back to the live screen, Enter picks, Esc cancels. Other
/// keys go on to the app under the picker.
pub fn pick_key(vk: u16, shift: bool) -> Option<PickKey> {
    let step = if shift { 10 } else { 1 };
    Some(match vk {
        0x25 => PickKey::Move(-step, 0),
        0x26 => PickKey::Move(0, -step),
        0x27 => PickKey::Move(step, 0),
        0x28 => PickKey::Move(0, step),
        0x20 => PickKey::Freeze,
        0x0D => PickKey::Pick,
        0x1B => PickKey::Cancel,
        _ => return None,
    })
}

/// Where the loupe (and its tag) goes for the cursor at `at` on a monitor
/// whose bounds are `monitor`, at `scale`: below and to the right of the
/// cursor, or on the other side where it would leave the monitor.
pub fn loupe_rect(at: (i32, i32), monitor: Rect, scale: f64) -> Rect {
    let px = |v: i32| (v as f64 * scale).round() as i32;
    let (w, h, gap) = (px(LOUPE), px(LOUPE + TAG_H + 6), px(GAP));
    let x = if at.0 + gap + w <= monitor.x + monitor.w { at.0 + gap } else { at.0 - gap - w };
    let y = if at.1 + gap + h <= monitor.y + monitor.h { at.1 + gap } else { at.1 - gap - h };
    Rect { x, y, w, h }
}

/// Where the help bar of `size` goes on the work area `work`: at the top,
/// centered, unless the cursor at `at` (or the loupe) is near it; then at the bottom.
pub fn help_rect(work: Rect, size: (i32, i32), at: (i32, i32), scale: f64) -> Rect {
    let px = |v: i32| (v as f64 * scale).round() as i32;
    let (w, h) = size;
    let x = work.x + (work.w - w) / 2;
    let top = work.y + px(HELP_MARGIN);
    let near_top = at.1 < top + h + px(LOUPE + TAG_H + GAP);
    let y = if near_top { work.y + work.h - h - px(HELP_MARGIN) } else { top };
    Rect { x, y, w, h }
}

/// "#1F7A45".
pub fn hex(c: Rgb) -> String {
    format!("#{:02X}{:02X}{:02X}", c.0, c.1, c.2)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FHD: Rect = Rect { x: 0, y: 0, w: 1920, h: 1080 };

    #[test]
    fn the_arrow_keys_nudge_space_freezes_enter_picks_esc_cancels() {
        assert_eq!(pick_key(0x25, false), Some(PickKey::Move(-1, 0)));
        assert_eq!(pick_key(0x28, true), Some(PickKey::Move(0, 10)));
        assert_eq!(pick_key(0x26, true), Some(PickKey::Move(0, -10)));
        assert_eq!(pick_key(0x27, false), Some(PickKey::Move(1, 0)));
        assert_eq!(pick_key(0x20, false), Some(PickKey::Freeze));
        assert_eq!(pick_key(0x0D, false), Some(PickKey::Pick));
        assert_eq!(pick_key(0x1B, false), Some(PickKey::Cancel));
        assert_eq!(pick_key(0x41, false), None, "other keys go to the app");
    }

    #[test]
    fn the_loupe_sits_beside_the_cursor_and_stays_on_its_monitor() {
        let r = loupe_rect((500, 400), FHD, 1.0);
        assert_eq!(r, Rect { x: 524, y: 424, w: 132, h: 160 });
        // Near the right and bottom edges: on the other side.
        let r = loupe_rect((1900, 1070), FHD, 1.0);
        assert_eq!((r.x, r.y), (1900 - 24 - 132, 1070 - 24 - 160));
        // At 150%, everything scales.
        let r = loupe_rect((500, 400), FHD, 1.5);
        assert_eq!(r, Rect { x: 536, y: 436, w: 198, h: 240 });
        // A monitor left of the primary one.
        let left = Rect { x: -1920, y: 0, w: 1920, h: 1080 };
        assert_eq!(loupe_rect((-10, 400), left, 1.0).x, -10 - 24 - 132);
    }

    #[test]
    fn the_help_bar_moves_to_the_bottom_when_the_cursor_comes_near_it() {
        let work = Rect { x: 0, y: 0, w: 1920, h: 1032 };
        let top = help_rect(work, (600, 40), (960, 600), 1.0);
        assert_eq!(top, Rect { x: 660, y: 14, w: 600, h: 40 });
        let bottom = help_rect(work, (600, 40), (960, 100), 1.0);
        assert_eq!(bottom.y, 1032 - 40 - 14);
        let second = Rect { x: 1920, y: 0, w: 2560, h: 1400 };
        assert_eq!(help_rect(second, (600, 40), (2500, 900), 1.0).x, 1920 + (2560 - 600) / 2);
    }

    #[test]
    fn colors_read_as_hex() {
        assert_eq!(hex(Rgb(0x1F, 0x7A, 0x45)), "#1F7A45");
        assert_eq!(hex(Rgb(0, 0, 0)), "#000000");
    }
}
