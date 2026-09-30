//! Turns raw hook input into macro events. OS-independent: character
//! translation is injected, so the conversion is tested with synthetic input.

use relay_core::edit::normalize;
use relay_core::keys::{self, KeyStroke};
use relay_core::model::{Event, Ms};
use relay_core::view::MovePoint;

use crate::CharTranslator;
use crate::keymap;
use crate::types::{HeldKeys, RawInput, RawKind};

#[derive(Debug, Clone, Copy)]
pub struct RecorderConfig {
    pub capture_moves: bool,
    pub capture_keys: bool,
    /// Minimum time between recorded cursor samples.
    pub move_interval_ms: Ms,
}

impl Default for RecorderConfig {
    fn default() -> Self {
        RecorderConfig { capture_moves: true, capture_keys: true, move_interval_ms: 16 }
    }
}

pub struct Recorder {
    cfg: RecorderConfig,
    start: f64,
    events: Vec<Event>,
    last_move: Option<(Ms, i32, i32)>,
    /// The latest cursor sample skipped by the throttle, so the cursor's
    /// final position before an action (or the end) is still recorded.
    pending_move: Option<(Ms, i32, i32)>,
    held: HeldKeys,
    translator: Box<dyn CharTranslator>,
    first_press: Option<(i32, i32)>,
    /// Index of the first event not yet reported by [`Recorder::take_new_moves`].
    reported: usize,
    /// The first half of a character outside the BMP (an emoji) sent as
    /// Unicode, waiting for its second half.
    high_surrogate: Option<u16>,
}

/// What a finished recording produced.
pub struct Recording {
    pub events: Vec<Event>,
    /// Where the first mouse button went down (to find the anchor window).
    pub first_press: Option<(i32, i32)>,
    pub duration_ms: Ms,
}

impl Recorder {
    pub fn new(cfg: RecorderConfig, start: f64, translator: Box<dyn CharTranslator>) -> Self {
        Recorder {
            cfg,
            start,
            events: Vec::new(),
            last_move: None,
            pending_move: None,
            held: HeldKeys::new(),
            translator,
            first_press: None,
            reported: 0,
            high_surrogate: None,
        }
    }

    pub fn elapsed(&self, now: f64) -> Ms {
        (now - self.start).max(0.0).round() as Ms
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn push(&mut self, raw: RawInput) {
        let t = self.elapsed(raw.time);
        if !matches!(raw.kind, RawKind::Move { .. }) {
            self.flush_move();
        }
        match raw.kind {
            RawKind::Move { x, y } => {
                if !self.cfg.capture_moves {
                    return;
                }
                match self.last_move {
                    Some((_, lx, ly)) if (lx, ly) == (x, y) => self.pending_move = None,
                    Some((lt, ..)) if t.saturating_sub(lt) < self.cfg.move_interval_ms => {
                        self.pending_move = Some((t, x, y));
                    }
                    _ => self.record_move(t, x, y),
                }
            }
            RawKind::Button { x, y, btn, down } => {
                if down && self.first_press.is_none() {
                    self.first_press = Some((x, y));
                }
                self.events.push(Event::Button { t, x, y, btn, down, label: String::new() });
            }
            RawKind::Wheel { x, y, delta, horizontal } => {
                self.events.push(Event::Wheel { t, x, y, delta, horizontal });
            }
            RawKind::Key { vk, scan, ext, down } => {
                // Ctrl + Alt + End is the kill switch, not macro input. (The
                // Windows hook already lets it through unreported; this keeps
                // the recorder right on its own.)
                if !self.cfg.capture_keys || (vk == keymap::VK_END && self.ctrl_alt_held()) {
                    return;
                }
                if vk == keymap::VK_PACKET {
                    // Unicode text sent by another program (SendInput with
                    // KEYEVENTF_UNICODE): `scan` is a UTF-16 unit, not a key,
                    // so it's kept as text and replayed as text. A character
                    // outside the BMP comes as two units: it's recorded once,
                    // with the second.
                    if (0xD800..0xDC00).contains(&scan) {
                        if down {
                            self.high_surrogate = Some(scan);
                        }
                        return;
                    }
                    let ch = down.then(|| {
                        let units: Vec<u16> = self.high_surrogate.take().into_iter().chain([scan]).collect();
                        String::from_utf16_lossy(&units)
                    });
                    let key = KeyStroke::code("Unidentified");
                    self.events.push(Event::Key { t, down, key, ch });
                    return;
                }
                let ch = if down { self.translator.translate(vk, scan, &self.held) } else { None };
                if down {
                    self.held.insert(vk);
                } else {
                    self.held.remove(&vk);
                }
                let key = KeyStroke { code: keymap::code(scan, ext, vk), vk, scan, ext };
                self.events.push(Event::Key { t, down, key, ch });
            }
            RawKind::Escape | RawKind::StopKey => {}
        }
    }

    /// Cursor samples recorded since the last call, for live progress.
    pub fn take_new_moves(&mut self) -> Vec<MovePoint> {
        let new = relay_core::view::cursor_path(&self.events[self.reported..]);
        self.reported = self.events.len();
        new
    }

    fn record_move(&mut self, t: Ms, x: i32, y: i32) {
        self.last_move = Some((t, x, y));
        self.pending_move = None;
        self.events.push(Event::Move { t, x, y });
    }

    fn flush_move(&mut self) {
        if let Some((t, x, y)) = self.pending_move.take() {
            self.record_move(t, x, y);
        }
    }

    fn ctrl_alt_held(&self) -> bool {
        self.held.iter().any(|&k| keymap::is_ctrl_vk(k)) && self.held.iter().any(|&k| keymap::is_alt_vk(k))
    }

    pub fn finish(mut self, now: f64) -> Recording {
        let duration_ms = self.elapsed(now);
        self.flush_move();
        trim_trailing_modifiers(&mut self.events);
        normalize(&mut self.events);
        Recording { events: self.events, first_press: self.first_press, duration_ms }
    }
}

/// Drops modifiers pressed after the last real action and still held at
/// the end: they belong to the hotkey that stopped the recording (Ctrl + Alt
/// of the kill switch). A modifier tapped at the end (Win) is kept, and so is
/// the release of a modifier pressed earlier.
fn trim_trailing_modifiers(events: &mut Vec<Event>) {
    let modifier = |e: &Event| match e {
        Event::Key { key, down, .. } if keys::modifier(&key.code).is_some() => Some((key.code.clone(), *down)),
        _ => None,
    };
    let last_action =
        events.iter().rposition(|e| !matches!(e, Event::Move { .. }) && modifier(e).is_none()).map_or(0, |i| i + 1);
    // From the end: a press (or its auto-repeat) with no release after it is still held.
    let mut released: Vec<String> = Vec::new();
    let mut held = vec![false; events.len()];
    for (i, e) in events.iter().enumerate().skip(last_action).rev() {
        match modifier(e) {
            Some((code, false)) => released.push(code),
            Some((code, true)) => held[i] = !released.contains(&code),
            None => {}
        }
    }
    let mut i = 0;
    events.retain(|_| {
        i += 1;
        !held[i - 1]
    });
}

/// True when a recording has something worth keeping: a click, a key or a
/// scroll. Cursor movement alone never counts.
pub fn is_meaningful(events: &[Event]) -> bool {
    events.iter().any(|e| matches!(e, Event::Button { .. } | Event::Key { .. } | Event::Wheel { .. }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use relay_core::model::MouseBtn;
    use relay_core::steps::{GroupOptions, StepKind, group_steps};

    /// A US-layout translator for letters, driven by the held Shift/Ctrl keys.
    struct Us;
    impl CharTranslator for Us {
        fn translate(&mut self, vk: u16, _scan: u16, held: &HeldKeys) -> Option<String> {
            let ctrl = held.contains(&0xA2) || held.contains(&0xA3);
            let shift = held.contains(&0xA0) || held.contains(&0xA1);
            match vk {
                0x41..=0x5A if ctrl => Some(char::from(vk as u8 - 0x40).to_string()),
                0x41..=0x5A if shift => Some((vk as u8 as char).to_string()),
                0x41..=0x5A => Some((vk as u8 as char).to_ascii_lowercase().to_string()),
                0x0D => Some("\r".into()),
                _ => None,
            }
        }
    }

    fn raw(time: f64, kind: RawKind) -> RawInput {
        RawInput { time, kind }
    }
    fn key(time: f64, vk: u16, scan: u16, down: bool) -> RawInput {
        raw(time, RawKind::Key { vk, scan, ext: false, down })
    }
    fn tap(time: f64, vk: u16, scan: u16) -> [RawInput; 2] {
        [key(time, vk, scan, true), key(time + 40.0, vk, scan, false)]
    }

    #[test]
    fn records_click_hello_and_ctrl_s() {
        let start = 10_000.0;
        let mut r = Recorder::new(RecorderConfig::default(), start, Box::new(Us));
        r.push(raw(start + 0.0, RawKind::Move { x: 100, y: 100 }));
        r.push(raw(start + 5.0, RawKind::Move { x: 101, y: 100 })); // too soon: coalesced
        r.push(raw(start + 20.0, RawKind::Move { x: 200, y: 150 }));
        r.push(raw(start + 100.0, RawKind::Button { x: 200, y: 150, btn: MouseBtn::Left, down: true }));
        r.push(raw(start + 160.0, RawKind::Button { x: 200, y: 150, btn: MouseBtn::Left, down: false }));
        let letters = [(0x48, 0x23), (0x45, 0x12), (0x4C, 0x26), (0x4C, 0x26), (0x4F, 0x18)];
        for (i, (vk, scan)) in letters.into_iter().enumerate() {
            for e in tap(start + 400.0 + i as f64 * 90.0, vk, scan) {
                r.push(e);
            }
        }
        r.push(key(start + 1200.0, 0xA2, 0x1D, true));
        for e in tap(start + 1220.0, 0x53, 0x1F) {
            r.push(e);
        }
        r.push(key(start + 1300.0, 0xA2, 0x1D, false));
        let rec = r.finish(start + 1500.0);

        assert_eq!(rec.first_press, Some((200, 150)));
        assert_eq!(rec.duration_ms, 1500);
        let moves = rec.events.iter().filter(|e| matches!(e, Event::Move { .. })).count();
        assert_eq!(moves, 2);
        let steps: Vec<String> = group_steps(&rec.events, GroupOptions::default())
            .into_iter()
            .map(|s| match s.kind {
                StepKind::Click { .. } => "CLICK".to_string(),
                StepKind::Type { text, .. } => format!("TYPE {text}"),
                StepKind::Keys { combo } => format!("KEYS {}", combo.join(" + ")),
                StepKind::Move { to_x, to_y, samples, .. } => format!("MOVE {to_x},{to_y} ×{samples}"),
                k => format!("{k:?}"),
            })
            .collect();
        assert_eq!(steps, ["MOVE 200,150 ×2", "CLICK", "TYPE hello", "KEYS Ctrl + S"]);
    }

    #[test]
    fn keys_use_physical_codes_and_can_be_skipped() {
        let mut r = Recorder::new(RecorderConfig::default(), 0.0, Box::new(Us));
        // The key labeled A on AZERTY: virtual key A, physical position Q.
        r.push(key(0.0, 0x41, 0x10, true));
        let Event::Key { key: stroke, ch, .. } = &r.events()[0] else { panic!() };
        assert_eq!((stroke.code.as_str(), stroke.vk, ch.as_deref()), ("KeyQ", 0x41, Some("a")));

        let cfg = RecorderConfig { capture_keys: false, capture_moves: false, ..Default::default() };
        let mut r = Recorder::new(cfg, 0.0, Box::new(Us));
        r.push(key(0.0, 0x41, 0x1E, true));
        r.push(raw(10.0, RawKind::Move { x: 1, y: 1 }));
        r.push(raw(20.0, RawKind::Button { x: 1, y: 1, btn: MouseBtn::Left, down: true }));
        assert_eq!(
            r.events(),
            [Event::Button { t: 20, x: 1, y: 1, btn: MouseBtn::Left, down: true, label: String::new() }],
            "only the click"
        );
    }

    #[test]
    fn without_the_path_clicks_keep_their_positions() {
        let cfg = RecorderConfig { capture_moves: false, ..Default::default() };
        let mut r = Recorder::new(cfg, 0.0, Box::new(Us));
        r.push(raw(0.0, RawKind::Move { x: 5, y: 5 }));
        r.push(raw(100.0, RawKind::Button { x: 300, y: 400, btn: MouseBtn::Right, down: true }));
        r.push(raw(150.0, RawKind::Move { x: 310, y: 400 }));
        r.push(raw(200.0, RawKind::Button { x: 320, y: 400, btn: MouseBtn::Right, down: false }));
        let rec = r.finish(300.0);
        assert!(rec.events.iter().all(|e| !matches!(e, Event::Move { .. })));
        assert_eq!(rec.events.iter().filter_map(Event::pos).collect::<Vec<_>>(), [(300, 400), (320, 400)]);
        assert_eq!(rec.first_press, Some((300, 400)));
    }

    #[test]
    fn wheels_both_ways_on_any_monitor() {
        let mut r = Recorder::new(RecorderConfig::default(), 0.0, Box::new(Us));
        r.push(raw(0.0, RawKind::Wheel { x: -1500, y: -200, delta: -120, horizontal: false }));
        r.push(raw(40.0, RawKind::Wheel { x: -1500, y: -200, delta: 240, horizontal: true }));
        r.push(raw(90.0, RawKind::Button { x: -1500, y: -200, btn: MouseBtn::Middle, down: true }));
        r.push(raw(120.0, RawKind::Button { x: -1500, y: -200, btn: MouseBtn::Middle, down: false }));
        let rec = r.finish(200.0);
        assert_eq!(
            rec.events[..2],
            [
                Event::Wheel { t: 0, x: -1500, y: -200, delta: -120, horizontal: false },
                Event::Wheel { t: 40, x: -1500, y: -200, delta: 240, horizontal: true },
            ]
        );
        assert_eq!(rec.first_press, Some((-1500, -200)), "a monitor left of and above the primary");
        let steps = group_steps(&rec.events, GroupOptions::default());
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn input_from_before_the_start_is_at_zero() {
        let mut r = Recorder::new(RecorderConfig::default(), 1000.0, Box::new(Us));
        r.push(raw(990.0, RawKind::Button { x: 1, y: 1, btn: MouseBtn::Left, down: true }));
        r.push(raw(1040.0, RawKind::Button { x: 1, y: 1, btn: MouseBtn::Left, down: false }));
        assert_eq!(r.events().iter().map(Event::t).collect::<Vec<_>>(), [0, 40]);
        assert_eq!(r.elapsed(900.0), 0);
    }

    #[test]
    fn esc_and_stop_keys_are_not_input() {
        let mut r = Recorder::new(RecorderConfig::default(), 0.0, Box::new(Us));
        r.push(raw(0.0, RawKind::Move { x: 1, y: 1 }));
        r.push(raw(5.0, RawKind::Move { x: 2, y: 2 })); // throttled
        r.push(raw(10.0, RawKind::Escape));
        r.push(raw(20.0, RawKind::StopKey));
        assert!(r.events().iter().all(|e| matches!(e, Event::Move { .. })), "{:?}", r.events());
        // (They still flush the cursor's last position, as any non-move input does.)
        assert_eq!(r.events().len(), 2);
    }

    #[test]
    fn the_kill_switch_with_right_ctrl_is_not_recorded() {
        let mut r = Recorder::new(RecorderConfig::default(), 0.0, Box::new(Us));
        for e in tap(0.0, 0x41, 0x1E) {
            r.push(e);
        }
        r.push(raw(500.0, RawKind::Key { vk: 0xA3, scan: 0x1D, ext: true, down: true }));
        r.push(raw(510.0, RawKind::Key { vk: 0xA5, scan: 0x38, ext: true, down: true }));
        r.push(raw(520.0, RawKind::Key { vk: 0x23, scan: 0x4F, ext: true, down: true }));
        let rec = r.finish(600.0);
        assert_eq!(rec.events.len(), 2, "{:?}", rec.events);
        assert!(rec.events.iter().all(|e| matches!(e, Event::Key { key, .. } if key.code == "KeyA")));
    }

    #[test]
    fn the_kill_switch_is_not_recorded() {
        let mut r = Recorder::new(RecorderConfig::default(), 0.0, Box::new(Us));
        for e in tap(0.0, 0x41, 0x1E) {
            r.push(e);
        }
        r.push(key(500.0, 0xA2, 0x1D, true));
        r.push(key(510.0, 0xA4, 0x38, true));
        r.push(key(520.0, 0x23, 0x4F, true));
        let rec = r.finish(600.0);
        assert_eq!(rec.events.len(), 2, "{:?}", rec.events);
        let steps = group_steps(&rec.events, GroupOptions::default());
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn a_modifier_tapped_at_the_end_is_kept() {
        let mut r = Recorder::new(RecorderConfig::default(), 0.0, Box::new(Us));
        for e in tap(0.0, 0x41, 0x1E) {
            r.push(e);
        }
        // Win, to open the Start menu, then F9 (not recorded) stops.
        r.push(raw(500.0, RawKind::Key { vk: 0x5B, scan: 0x5B, ext: true, down: true }));
        r.push(raw(560.0, RawKind::Key { vk: 0x5B, scan: 0x5B, ext: true, down: false }));
        let rec = r.finish(900.0);
        let steps = group_steps(&rec.events, GroupOptions::default());
        assert_eq!(steps.len(), 2, "{:?}", rec.events);
        assert_eq!(steps[1].kind, StepKind::Keys { combo: vec!["Win".into()] });
    }

    #[test]
    fn a_modifier_held_through_the_last_action_keeps_its_release() {
        let mut r = Recorder::new(RecorderConfig::default(), 0.0, Box::new(Us));
        r.push(key(0.0, 0xA2, 0x1D, true));
        r.push(raw(50.0, RawKind::Button { x: 1, y: 1, btn: MouseBtn::Left, down: true }));
        r.push(raw(90.0, RawKind::Button { x: 1, y: 1, btn: MouseBtn::Left, down: false }));
        r.push(key(150.0, 0xA2, 0x1D, false));
        r.push(raw(900.0, RawKind::Move { x: 500, y: 500 }));
        let rec = r.finish(1000.0);
        let ups: Vec<_> =
            rec.events.iter().filter(|e| matches!(e, Event::Key { down: false, .. })).map(|e| e.t()).collect();
        assert_eq!(ups, [150], "Ctrl is released where it was, not at the end");
    }

    #[test]
    fn the_last_cursor_position_before_an_action_is_kept() {
        let mut r = Recorder::new(RecorderConfig::default(), 0.0, Box::new(Us));
        r.push(raw(0.0, RawKind::Move { x: 1, y: 1 }));
        r.push(raw(5.0, RawKind::Move { x: 9, y: 9 })); // throttled…
        for e in tap(300.0, 0x41, 0x1E) {
            r.push(e);
        }
        let moves: Vec<_> = r.events().iter().filter_map(|e| e.pos()).collect();
        assert_eq!(moves, [(1, 1), (9, 9)], "…but kept before the key");
    }

    #[test]
    fn unicode_from_other_programs_is_recorded_as_text() {
        let mut r = Recorder::new(RecorderConfig::default(), 0.0, Box::new(Us));
        r.push(key(0.0, 0xE7, 'é' as u16, true));
        r.push(key(10.0, 0xE7, 'é' as u16, false));
        let Event::Key { key: k, ch, .. } = &r.events()[0] else { panic!() };
        assert_eq!((k.scan, k.vk, ch.as_deref()), (0, 0, Some("é")));
    }

    #[test]
    fn unicode_outside_the_bmp_is_one_character() {
        let mut units = [0u16; 2];
        '😀'.encode_utf16(&mut units);
        let [high, low] = units;
        let mut r = Recorder::new(RecorderConfig::default(), 0.0, Box::new(Us));
        // Senders release each unit before the next, or press both first.
        for order in [
            [(high, true), (high, false), (low, true), (low, false)],
            [(high, true), (low, true), (high, false), (low, false)],
        ] {
            for (unit, down) in order {
                r.push(key(0.0, 0xE7, unit, down));
            }
        }
        let keys: Vec<_> = r
            .events()
            .iter()
            .map(|e| match e {
                Event::Key { down, ch, .. } => (*down, ch.as_deref()),
                e => panic!("{e:?}"),
            })
            .collect();
        assert_eq!(keys, [(true, Some("😀")), (false, None), (true, Some("😀")), (false, None)]);
        // A lone half is still recorded, as the replacement character.
        let mut r = Recorder::new(RecorderConfig::default(), 0.0, Box::new(Us));
        r.push(key(0.0, 0xE7, high, true));
        r.push(key(10.0, 0xE7, 'a' as u16, true));
        let Event::Key { ch, .. } = &r.events()[0] else { panic!() };
        assert_eq!(ch.as_deref(), Some("\u{FFFD}a"));
    }

    #[test]
    fn new_moves_are_reported_once() {
        let mut r = Recorder::new(RecorderConfig::default(), 0.0, Box::new(Us));
        r.push(raw(0.0, RawKind::Move { x: 1, y: 1 }));
        r.push(raw(20.0, RawKind::Move { x: 2, y: 2 }));
        assert_eq!(r.take_new_moves().len(), 2);
        r.push(raw(40.0, RawKind::Move { x: 3, y: 3 }));
        assert_eq!(r.take_new_moves(), vec![MovePoint { t: 40, x: 3, y: 3 }]);
    }

    #[test]
    fn meaningful_recordings() {
        let moves = |n: u32| (0..n).map(|i| Event::Move { t: i * 16, x: i as i32, y: 0 }).collect::<Vec<_>>();
        for n in [0, 1, 4, 5, 100] {
            assert!(!is_meaningful(&moves(n)), "{n} moves");
        }
        let with = |e: Event| [moves(3), vec![e]].concat();
        assert!(is_meaningful(&with(Event::Wheel { t: 50, x: 0, y: 0, delta: 120, horizontal: false })));
        assert!(is_meaningful(&with(Event::Button {
            t: 50,
            x: 0,
            y: 0,
            btn: MouseBtn::Left,
            down: true,
            label: String::new()
        })));
        assert!(is_meaningful(&with(Event::Key { t: 50, down: true, key: KeyStroke::code("KeyA"), ch: None })));
    }
}
