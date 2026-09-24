//! Turns raw hook input into macro events. OS-independent: character
//! translation is injected, so the conversion is tested with synthetic input.

use relay_core::edit::normalize;
use relay_core::keys::{self, KeyStroke};
use relay_core::model::{Event, Ms};
use relay_core::view::MovePoint;

use crate::keymap;
use crate::types::{HeldKeys, RawInput, RawKind};
use crate::CharTranslator;

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
    held: HeldKeys,
    translator: Box<dyn CharTranslator>,
    first_press: Option<(i32, i32)>,
    /// Index of the first event not yet reported by [`Recorder::take_new_moves`].
    reported: usize,
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
            held: HeldKeys::new(),
            translator,
            first_press: None,
            reported: 0,
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
        match raw.kind {
            RawKind::Move { x, y } => {
                if !self.cfg.capture_moves {
                    return;
                }
                if let Some((lt, lx, ly)) = self.last_move
                    && ((lx, ly) == (x, y) || t.saturating_sub(lt) < self.cfg.move_interval_ms)
                {
                    return;
                }
                self.last_move = Some((t, x, y));
                self.events.push(Event::Move { t, x, y });
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
                if !self.cfg.capture_keys || (vk == VK_END && self.ctrl_alt_held()) {
                    return; // Ctrl + Alt + End is the kill switch, not macro input.
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
            RawKind::Escape => {}
        }
    }

    /// Cursor samples recorded since the last call, for live progress.
    pub fn take_new_moves(&mut self) -> Vec<MovePoint> {
        let new = relay_core::view::cursor_path(&self.events[self.reported..]);
        self.reported = self.events.len();
        new
    }

    fn ctrl_alt_held(&self) -> bool {
        let any = |ks: [u16; 3]| ks.iter().any(|k| self.held.contains(k));
        any([0x11, 0xA2, 0xA3]) && any([0x12, 0xA4, 0xA5])
    }

    pub fn finish(mut self, now: f64) -> Recording {
        let duration_ms = self.elapsed(now);
        trim_trailing_modifiers(&mut self.events);
        normalize(&mut self.events);
        Recording { events: self.events, first_press: self.first_press, duration_ms }
    }
}

const VK_END: u16 = 0x23;

/// Drops modifier presses after the last real action: they belong to the
/// hotkey that stopped the recording (Ctrl + Alt of the kill switch).
fn trim_trailing_modifiers(events: &mut Vec<Event>) {
    let is_modifier = |e: &Event| matches!(e, Event::Key { key, .. } if keys::modifier(&key.code).is_some());
    let last_action = events
        .iter()
        .rposition(|e| !matches!(e, Event::Move { .. }) && !is_modifier(e))
        .map_or(0, |i| i + 1);
    let mut i = 0;
    events.retain(|e| {
        i += 1;
        i <= last_action || !is_modifier(e)
    });
}

/// True when a recording has something worth keeping (not just a stray twitch).
pub fn is_meaningful(events: &[Event]) -> bool {
    let actions = events.iter().filter(|e| !matches!(e, Event::Move { .. })).count();
    actions > 0 || events.len() >= 5
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
                k => format!("{k:?}"),
            })
            .collect();
        assert_eq!(steps, ["CLICK", "TYPE hello", "KEYS Ctrl + S"]);
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
        assert_eq!(r.events().len(), 1);
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
        assert!(!is_meaningful(&[Event::Move { t: 0, x: 0, y: 0 }]));
        assert!(is_meaningful(&[Event::Wheel { t: 0, x: 0, y: 0, delta: 120, horizontal: false }]));
    }
}
