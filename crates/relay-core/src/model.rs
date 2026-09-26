//! The macro model: raw input events plus recording metadata and playback options.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use ts_rs::TS;
use uuid::Uuid;

use crate::keys::KeyStroke;

/// Milliseconds from the start of the recording.
pub type Ms = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum MouseBtn {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

/// An sRGB color, serialized as `"#RRGGBB"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TS)]
#[ts(export, type = "string")]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// True when every channel differs by at most `tolerance`.
    pub fn within(self, other: Rgb, tolerance: u8) -> bool {
        let d = |a: u8, b: u8| a.abs_diff(b) <= tolerance;
        d(self.0, other.0) && d(self.1, other.1) && d(self.2, other.2)
    }

    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }

    pub fn parse(s: &str) -> Option<Rgb> {
        let h = s.strip_prefix('#')?;
        // Not just the length: from_str_radix would also accept "+1".
        if h.len() != 6 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        let c = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
        Some(Rgb(c(0)?, c(2)?, c(4)?))
    }
}

impl Serialize for Rgb {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Rgb::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("invalid color {s:?}")))
    }
}

/// A raw recorded (or inserted) event. Coordinates are physical pixels on the
/// virtual desktop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Move {
        t: Ms,
        x: i32,
        y: i32,
    },
    Button {
        t: Ms,
        x: i32,
        y: i32,
        btn: MouseBtn,
        down: bool,
        /// User-editable click label, kept on the down event.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        label: String,
    },
    Wheel {
        t: Ms,
        x: i32,
        y: i32,
        delta: i32,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        horizontal: bool,
    },
    Key {
        t: Ms,
        down: bool,
        key: KeyStroke,
        /// The character this key produced, for display and TYPE grouping.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ch: Option<String>,
    },
    Wait {
        t: Ms,
        dur: Ms,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        label: String,
    },
    PixelWait {
        t: Ms,
        dur: Ms,
        x: i32,
        y: i32,
        color: Rgb,
        tolerance: u8,
        timeout_ms: Ms,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        label: String,
    },
}

impl Event {
    pub fn t(&self) -> Ms {
        match self {
            Event::Move { t, .. }
            | Event::Button { t, .. }
            | Event::Wheel { t, .. }
            | Event::Key { t, .. }
            | Event::Wait { t, .. }
            | Event::PixelWait { t, .. } => *t,
        }
    }

    pub fn t_mut(&mut self) -> &mut Ms {
        match self {
            Event::Move { t, .. }
            | Event::Button { t, .. }
            | Event::Wheel { t, .. }
            | Event::Key { t, .. }
            | Event::Wait { t, .. }
            | Event::PixelWait { t, .. } => t,
        }
    }

    /// When the event is over: `t` for instant events, `t + dur` for waits.
    pub fn end(&self) -> Ms {
        match self {
            Event::Wait { t, dur, .. } | Event::PixelWait { t, dur, .. } => t.saturating_add(*dur),
            e => e.t(),
        }
    }

    pub fn pos(&self) -> Option<(i32, i32)> {
        match self {
            Event::Move { x, y, .. }
            | Event::Button { x, y, .. }
            | Event::Wheel { x, y, .. }
            | Event::PixelWait { x, y, .. } => Some((*x, *y)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct MonitorInfo {
    pub name: String,
    pub rect: Rect,
    pub work: Rect,
    pub dpi: u32,
    pub primary: bool,
}

/// The window the recording is anchored to (for "Window" coordinates).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WindowInfo {
    pub exe: String,
    pub class: String,
    pub title: String,
    pub rect: Rect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RecordingMeta {
    pub os: String,
    pub virtual_desktop: Rect,
    pub monitors: Vec<MonitorInfo>,
    /// System double-click time and distance when recorded.
    pub double_click_ms: Ms,
    pub double_click_px: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_window: Option<WindowInfo>,
}

impl RecordingMeta {
    /// A single 1920×1080 monitor, used for samples and migrated files.
    pub fn single_1080p() -> Self {
        let rect = Rect { x: 0, y: 0, w: 1920, h: 1080 };
        RecordingMeta {
            os: "windows".into(),
            virtual_desktop: rect,
            monitors: vec![MonitorInfo {
                name: "\\\\.\\DISPLAY1".into(),
                rect,
                work: Rect { h: 1032, ..rect },
                dpi: 96,
                primary: true,
            }],
            double_click_ms: 500,
            double_click_px: 4,
            anchor_window: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum CoordMode {
    Screen,
    Window,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Repeat {
    Count(u32),
    Forever,
}

impl Repeat {
    /// How many times the macro plays (at least once), or `None` for forever.
    pub fn loops(self) -> Option<u32> {
        match self {
            Repeat::Count(n) => Some(n.max(1)),
            Repeat::Forever => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PlaybackOptions {
    pub speed: f32,
    pub repeat: Repeat,
    pub humanize: bool,
    pub jitter_ms: u32,
    pub stop_on_key: bool,
    pub coord_mode: CoordMode,
}

impl Default for PlaybackOptions {
    fn default() -> Self {
        PlaybackOptions {
            speed: 1.0,
            repeat: Repeat::Count(1),
            humanize: true,
            jitter_ms: 40,
            stop_on_key: true,
            coord_mode: CoordMode::Screen,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Macro {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub recording: RecordingMeta,
    pub playback: PlaybackOptions,
    pub events: Vec<Event>,
}

impl Macro {
    pub fn new(name: impl Into<String>, recording: RecordingMeta, events: Vec<Event>) -> Self {
        let now = Utc::now();
        Macro {
            id: Uuid::new_v4(),
            name: name.into(),
            created_at: now,
            modified_at: now,
            recording,
            playback: PlaybackOptions::default(),
            events,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_round_trips_and_compares() {
        let c = Rgb::parse("#EC3013").unwrap();
        assert_eq!(c, Rgb(0xEC, 0x30, 0x13));
        assert_eq!(serde_json::to_string(&c).unwrap(), "\"#EC3013\"");
        assert!(c.within(Rgb(0xE0, 0x38, 0x13), 12));
        assert!(!c.within(Rgb(0xE0, 0x38, 0x13), 8));
        assert!(Rgb::parse("EC3013").is_none());
        assert!(Rgb::parse("#EC30").is_none());
        assert!(Rgb::parse("#+1+2+3").is_none());
    }

    #[test]
    fn repeat_serializes_compactly() {
        assert_eq!(serde_json::to_string(&Repeat::Count(3)).unwrap(), r#"{"count":3}"#);
        assert_eq!(serde_json::to_string(&Repeat::Forever).unwrap(), r#""forever""#);
    }
}
