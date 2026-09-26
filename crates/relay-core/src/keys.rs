//! Keys are identified by their W3C `code` (the physical key: "KeyA",
//! "ControlLeft", "Enter"…), which is layout-independent and portable. The
//! Windows virtual-key and scan codes are kept alongside for faithful replay.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct KeyStroke {
    pub code: String,
    /// Windows virtual-key code; 0 when unknown (replay falls back to `code`/`ch`).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub vk: u16,
    /// Hardware scan code; 0 when unknown.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub scan: u16,
    /// Extended-key flag (right Ctrl/Alt, arrows, Insert/Delete, …).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ext: bool,
}

fn is_zero(v: &u16) -> bool {
    *v == 0
}

impl KeyStroke {
    /// A key known only by its code (tests, migrated files).
    pub fn code(code: impl Into<String>) -> Self {
        KeyStroke { code: code.into(), vk: 0, scan: 0, ext: false }
    }

    pub fn modifier(&self) -> Option<Modifier> {
        modifier(&self.code)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Modifier {
    Ctrl,
    Alt,
    Shift,
    Win,
}

impl Modifier {
    pub fn label(self) -> &'static str {
        match self {
            Modifier::Ctrl => "Ctrl",
            Modifier::Alt => "Alt",
            Modifier::Shift => "Shift",
            Modifier::Win => "Win",
        }
    }
}

pub fn modifier(code: &str) -> Option<Modifier> {
    match code {
        "ControlLeft" | "ControlRight" => Some(Modifier::Ctrl),
        "AltLeft" | "AltRight" => Some(Modifier::Alt),
        "ShiftLeft" | "ShiftRight" => Some(Modifier::Shift),
        "MetaLeft" | "MetaRight" => Some(Modifier::Win),
        _ => None,
    }
}

/// Right Alt is AltGr on many layouts: characters typed with it are text, not shortcuts.
pub fn is_alt_gr(code: &str) -> bool {
    code == "AltRight"
}

/// The label shown in the UI, e.g. "KeyA" → "A", "ArrowLeft" → "Left".
pub fn label(code: &str) -> String {
    if let Some(m) = modifier(code) {
        return m.label().into();
    }
    if let Some(c) = code.strip_prefix("Key")
        && c.len() == 1
    {
        return c.into();
    }
    if let Some(d) = code.strip_prefix("Digit") {
        return d.into();
    }
    if let Some(n) = code.strip_prefix("Numpad") {
        return format!("Num {n}");
    }
    if let Some(a) = code.strip_prefix("Arrow") {
        return a.into();
    }
    let punct = match code {
        "Escape" => "Esc",
        "Minus" => "-",
        "Equal" => "=",
        "BracketLeft" => "[",
        "BracketRight" => "]",
        "Semicolon" => ";",
        "Quote" => "'",
        "Backquote" => "`",
        "Backslash" => "\\",
        "Comma" => ",",
        "Period" => ".",
        "Slash" => "/",
        "PageUp" => "PgUp",
        "PageDown" => "PgDn",
        "Delete" => "Del",
        "Insert" => "Ins",
        other => other,
    };
    punct.into()
}

/// The label for a recorded key. Letters and digits come from the virtual
/// key, which follows the keyboard layout (the key labeled A on AZERTY is
/// physically "KeyQ" but has virtual key A); everything else from the code.
pub fn key_label(key: &KeyStroke) -> String {
    match key.vk {
        0x41..=0x5A | 0x30..=0x39 => (key.vk as u8 as char).to_string(),
        _ => label(&key.code),
    }
}

/// Inverse of [`label`] for the labels used by the prototype ("Ctrl + A").
pub fn code_for_label(label: &str) -> String {
    match label {
        "Ctrl" => "ControlLeft".into(),
        "Alt" => "AltLeft".into(),
        "Shift" => "ShiftLeft".into(),
        "Win" => "MetaLeft".into(),
        "Esc" => "Escape".into(),
        "PgUp" => "PageUp".into(),
        "PgDn" => "PageDown".into(),
        "Del" => "Delete".into(),
        "Ins" => "Insert".into(),
        "Left" | "Right" | "Up" | "Down" => format!("Arrow{label}"),
        l if l.starts_with("Num ") => format!("Numpad{}", &l[4..]),
        l if l.len() == 1 => {
            let c = l.chars().next().unwrap();
            if c.is_ascii_alphabetic() {
                format!("Key{}", c.to_ascii_uppercase())
            } else if c.is_ascii_digit() {
                format!("Digit{c}")
            } else {
                code_for_char(c).unwrap_or_else(|| l.into())
            }
        }
        other => other.into(),
    }
}

/// The US-layout key that types `c`, and whether Shift is needed.
pub fn key_for_char(c: char) -> Option<(String, bool)> {
    if c.is_ascii_lowercase() {
        return Some((format!("Key{}", c.to_ascii_uppercase()), false));
    }
    if c.is_ascii_uppercase() {
        return Some((format!("Key{c}"), true));
    }
    if c.is_ascii_digit() {
        return Some((format!("Digit{c}"), false));
    }
    let (code, shift) = match c {
        ' ' => ("Space", false),
        '-' => ("Minus", false),
        '_' => ("Minus", true),
        '=' => ("Equal", false),
        '+' => ("Equal", true),
        '.' => ("Period", false),
        ',' => ("Comma", false),
        '/' => ("Slash", false),
        ';' => ("Semicolon", false),
        '\'' => ("Quote", false),
        _ => return None,
    };
    Some((code.into(), shift))
}

fn code_for_char(c: char) -> Option<String> {
    key_for_char(c).map(|(code, _)| code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        assert_eq!(label("KeyA"), "A");
        assert_eq!(label("Digit7"), "7");
        assert_eq!(label("ControlRight"), "Ctrl");
        assert_eq!(label("MetaLeft"), "Win");
        assert_eq!(label("ArrowLeft"), "Left");
        assert_eq!(label("Escape"), "Esc");
        assert_eq!(label("Enter"), "Enter");
        assert_eq!(label("F2"), "F2");
        assert_eq!(label("Minus"), "-");
        assert_eq!(label("Numpad1"), "Num 1");
    }

    #[test]
    fn labels_round_trip_to_codes() {
        for l in [
            "Ctrl", "Alt", "Shift", "Win", "A", "7", "Enter", "Tab", "F2", "Esc", "Left", "-", "PgUp", "Del", "Ins",
            "Num 3", "Home", "Space",
        ] {
            assert_eq!(label(&code_for_label(l)), l, "{l}");
        }
    }

    #[test]
    fn recorded_labels_follow_the_layout() {
        let azerty_a = KeyStroke { code: "KeyQ".into(), vk: 0x41, scan: 0x10, ext: false };
        assert_eq!(key_label(&azerty_a), "A");
        assert_eq!(key_label(&KeyStroke::code("KeyQ")), "Q");
        assert_eq!(key_label(&KeyStroke { code: "Enter".into(), vk: 0x0D, scan: 0x1C, ext: false }), "Enter");
    }

    #[test]
    fn chars_to_keys() {
        assert_eq!(key_for_char('a'), Some(("KeyA".into(), false)));
        assert_eq!(key_for_char('A'), Some(("KeyA".into(), true)));
        assert_eq!(key_for_char('_'), Some(("Minus".into(), true)));
        assert_eq!(key_for_char('é'), None);
    }
}
