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

/// Splits a combo like "Ctrl + A" into its labels. The key can be `+` itself
/// ("Ctrl + +"). `None` when a part is missing ("Ctrl +", "").
pub fn split_combo(combo: &str) -> Option<Vec<&str>> {
    let mut parts: Vec<&str> = combo.split('+').map(str::trim).collect();
    // "Ctrl + +" splits into "Ctrl", "", "": the last two are the + key.
    if parts.len() >= 2 && parts[parts.len() - 2..] == ["", ""] {
        parts.truncate(parts.len() - 2);
        parts.push("+");
    }
    (!parts.iter().any(|p| p.is_empty())).then_some(parts)
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
        '[' => ("BracketLeft", false),
        '{' => ("BracketLeft", true),
        ']' => ("BracketRight", false),
        '}' => ("BracketRight", true),
        '\\' => ("Backslash", false),
        '|' => ("Backslash", true),
        ';' => ("Semicolon", false),
        ':' => ("Semicolon", true),
        '\'' => ("Quote", false),
        '"' => ("Quote", true),
        '`' => ("Backquote", false),
        '~' => ("Backquote", true),
        ',' => ("Comma", false),
        '<' => ("Comma", true),
        '.' => ("Period", false),
        '>' => ("Period", true),
        '/' => ("Slash", false),
        '?' => ("Slash", true),
        '!' => ("Digit1", true),
        '@' => ("Digit2", true),
        '#' => ("Digit3", true),
        '$' => ("Digit4", true),
        '%' => ("Digit5", true),
        '^' => ("Digit6", true),
        '&' => ("Digit7", true),
        '*' => ("Digit8", true),
        '(' => ("Digit9", true),
        ')' => ("Digit0", true),
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
        let cases = [
            ("Ctrl", "ControlLeft"),
            ("Alt", "AltLeft"),
            ("Shift", "ShiftLeft"),
            ("Win", "MetaLeft"),
            ("A", "KeyA"),
            ("7", "Digit7"),
            ("Enter", "Enter"),
            ("Tab", "Tab"),
            ("F2", "F2"),
            ("Esc", "Escape"),
            ("Left", "ArrowLeft"),
            ("PgUp", "PageUp"),
            ("Del", "Delete"),
            ("Ins", "Insert"),
            ("Num 3", "Numpad3"),
            ("Home", "Home"),
            ("Space", "Space"),
            ("-", "Minus"),
            ("=", "Equal"),
            ("[", "BracketLeft"),
            ("]", "BracketRight"),
            ("\\", "Backslash"),
            ("`", "Backquote"),
            (";", "Semicolon"),
            ("'", "Quote"),
            (",", "Comma"),
            (".", "Period"),
            ("/", "Slash"),
        ];
        for (l, code) in cases {
            assert_eq!(code_for_label(l), code, "{l}");
            assert_eq!(label(code), l, "{code}");
        }
        // Shifted symbols name the key they're on (a "+" hotkey is the = key).
        assert_eq!(code_for_label("+"), "Equal");
        assert_eq!(code_for_label("?"), "Slash");
    }

    #[test]
    fn combos_split_into_labels() {
        assert_eq!(split_combo("Ctrl + Shift + S"), Some(vec!["Ctrl", "Shift", "S"]));
        assert_eq!(split_combo("F9"), Some(vec!["F9"]));
        assert_eq!(split_combo("Ctrl + +"), Some(vec!["Ctrl", "+"]));
        assert_eq!(split_combo("Ctrl+Alt++"), Some(vec!["Ctrl", "Alt", "+"]));
        assert_eq!(split_combo("+"), Some(vec!["+"]));
        for bad in ["", "Ctrl +", "+ A", "Ctrl + + A", "Ctrl + + +"] {
            assert_eq!(split_combo(bad), None, "{bad:?}");
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
        assert_eq!(key_for_char('['), Some(("BracketLeft".into(), false)));
        assert_eq!(key_for_char('~'), Some(("Backquote".into(), true)));
        assert_eq!(key_for_char('@'), Some(("Digit2".into(), true)));
        assert_eq!(key_for_char(')'), Some(("Digit0".into(), true)));
        assert_eq!(key_for_char('"'), Some(("Quote".into(), true)));
        assert_eq!(key_for_char('é'), None);
        // Every printable US character has a key.
        for c in (' '..='~').filter(|c| !c.is_ascii_alphanumeric()) {
            assert!(key_for_char(c).is_some(), "{c:?}");
        }
    }
}
