//! The `.rly` file format: a versioned JSON envelope around [`Macro`].
//!
//! Loading parses into a JSON value first, runs the migration chain up to
//! [`VERSION`], then deserializes. Files from a newer Relay are rejected.

use chrono::Utc;
use serde::Serialize;
use serde_json::{Map, Value, json};
use thiserror::Error;
use uuid::Uuid;

use crate::edit::normalize;
use crate::keys::{code_for_label, key_for_char};
use crate::model::{Macro, PlaybackOptions, RecordingMeta};
use crate::steps::{Step, group_steps};

pub const FORMAT: &str = "relay-macro";
pub const VERSION: u64 = 1;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("not a Relay macro")]
    NotRelay,
    #[error("this macro was saved by a newer Relay (format version {0})")]
    TooNew(u64),
    #[error("invalid macro file: {0}")]
    Invalid(String),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Serialize)]
struct Envelope<'a> {
    format: &'static str,
    version: u64,
    #[serde(flatten)]
    body: &'a Macro,
}

#[derive(Serialize)]
struct Export<'a> {
    format: &'static str,
    version: u64,
    #[serde(flatten)]
    body: &'a Macro,
    /// Derived, for developers reading the file; ignored when importing.
    steps: Vec<Step>,
}

/// Serializes a macro as a compact `.rly` document.
pub fn to_rly(m: &Macro) -> String {
    serde_json::to_string(&Envelope { format: FORMAT, version: VERSION, body: m }).expect("macro serializes")
}

/// The pretty-printed JSON export, including the derived steps.
pub fn to_export_json(m: &Macro) -> String {
    let steps = group_steps(&m.events, (&m.recording).into());
    serde_json::to_string_pretty(&Export { format: FORMAT, version: VERSION, body: m, steps }).expect("macro serializes")
}

/// Parses a `.rly` (or exported `.json`) document of any supported version.
pub fn from_rly(s: &str) -> Result<Macro, FormatError> {
    let mut v: Value = serde_json::from_str(s)?;
    let obj = v.as_object_mut().ok_or(FormatError::NotRelay)?;
    if obj.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err(FormatError::NotRelay);
    }
    let mut version = obj.get("version").and_then(Value::as_u64).ok_or(FormatError::NotRelay)?;
    if version > VERSION {
        return Err(FormatError::TooNew(version));
    }
    while version < VERSION {
        migrate(version, obj)?;
        version += 1;
    }
    obj.remove("format");
    obj.remove("version");
    obj.remove("steps");
    let mut m: Macro = serde_json::from_value(v)?;
    normalize(&mut m.events);
    Ok(m)
}

fn migrate(from: u64, obj: &mut Map<String, Value>) -> Result<(), FormatError> {
    match from {
        0 => migrate_v0(obj),
        _ => Err(FormatError::Invalid(format!("no migration from version {from}"))),
    }
}

/// v0 is the M0 prototype's export: high-level events (`click`, `key` combos
/// like "Ctrl + A", `char`, `wait`, `cond`) and no recording metadata. v1
/// stores presses and releases, so each high-level event is expanded.
fn migrate_v0(obj: &mut Map<String, Value>) -> Result<(), FormatError> {
    let bad = |what: &str| FormatError::Invalid(format!("v0 event without {what}"));
    let events = obj.remove("events").and_then(|e| e.as_array().cloned()).ok_or_else(|| bad("list"))?;
    let mut out = Vec::new();
    for e in &events {
        let t = e["t"].as_f64().ok_or_else(|| bad("t"))?.round() as u64;
        let num = |k: &str| e[k].as_f64().map(|v| v.round() as i64).ok_or_else(|| bad(k));
        let text = |k: &str| e[k].as_str().unwrap_or_default().to_string();
        let key_ev = |t: u64, code: &str, down: bool, ch: Option<String>| {
            let mut k = json!({ "type": "key", "t": t, "down": down, "key": { "code": code } });
            if let Some(ch) = ch {
                k["ch"] = json!(ch);
            }
            k
        };
        match e["type"].as_str().unwrap_or_default() {
            "move" => out.push(json!({ "type": "move", "t": t, "x": num("x")?, "y": num("y")? })),
            "click" => {
                let (x, y) = (num("x")?, num("y")?);
                let btn = match e["btn"].as_str() {
                    Some("Right") => "Right",
                    Some("Middle") => "Middle",
                    _ => "Left",
                };
                let count = e["count"].as_u64().unwrap_or(1).max(1);
                for i in 0..count {
                    let down = t + i * 120;
                    let mut d = json!({ "type": "button", "t": down, "x": x, "y": y, "btn": btn, "down": true });
                    if i == 0 && !text("label").is_empty() {
                        d["label"] = json!(text("label"));
                    }
                    out.push(d);
                    out.push(json!({ "type": "button", "t": down + 60, "x": x, "y": y, "btn": btn, "down": false }));
                }
            }
            "key" => {
                let combo = text("key");
                let parts: Vec<String> = combo.split(" + ").map(code_for_label).collect();
                let (main, mods) = parts.split_last().ok_or_else(|| bad("key"))?;
                for m in mods {
                    out.push(key_ev(t, m, true, None));
                }
                out.push(key_ev(t + 20, main, true, None));
                out.push(key_ev(t + 60, main, false, None));
                for m in mods.iter().rev() {
                    out.push(key_ev(t + 80, m, false, None));
                }
            }
            "char" => {
                let s = text("key");
                let c = s.chars().next().ok_or_else(|| bad("key"))?;
                let (code, shift) = key_for_char(c).unwrap_or_else(|| ("Unidentified".into(), false));
                if shift {
                    out.push(key_ev(t, "ShiftLeft", true, None));
                }
                out.push(key_ev(t, &code, true, Some(s.clone())));
                out.push(key_ev(t + 40, &code, false, None));
                if shift {
                    out.push(key_ev(t + 40, "ShiftLeft", false, None));
                }
            }
            "wait" => out.push(json!({ "type": "wait", "t": t, "dur": num("dur")?, "label": text("label") })),
            "cond" => out.push(json!({
                "type": "pixel_wait", "t": t, "dur": num("dur")?, "x": num("x")?, "y": num("y")?,
                "color": text("color").to_uppercase(), "tolerance": 8, "timeout_ms": 5000, "label": text("label"),
            })),
            other => return Err(FormatError::Invalid(format!("unknown v0 event type {other:?}"))),
        }
    }
    let now = Utc::now();
    obj.insert("events".into(), Value::Array(out));
    obj.entry("id").or_insert_with(|| json!(Uuid::new_v4()));
    obj.entry("name").or_insert_with(|| json!("Imported macro"));
    obj.entry("created_at").or_insert_with(|| json!(now));
    obj.entry("modified_at").or_insert_with(|| json!(now));
    obj.entry("recording").or_insert_with(|| json!(RecordingMeta::single_1080p()));
    obj.entry("playback").or_insert_with(|| json!(PlaybackOptions::default()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::check_invariants;
    use crate::keys::KeyStroke;
    use crate::model::{Event, MouseBtn, Rgb};
    use chrono::TimeZone;

    fn fixed_macro() -> Macro {
        let at = Utc.with_ymd_and_hms(2026, 9, 24, 9, 0, 0).unwrap();
        Macro {
            id: Uuid::from_u128(0x5245_4c59),
            name: "Save the file".into(),
            created_at: at,
            modified_at: at,
            recording: RecordingMeta::single_1080p(),
            playback: PlaybackOptions::default(),
            events: vec![
                Event::Move { t: 0, x: 960, y: 540 },
                Event::Button { t: 120, x: 960, y: 540, btn: MouseBtn::Left, down: true, label: "Save".into() },
                Event::Button { t: 180, x: 960, y: 540, btn: MouseBtn::Left, down: false, label: String::new() },
                Event::Key { t: 400, down: true, key: KeyStroke { code: "KeyS".into(), vk: 0x53, scan: 0x1F, ext: false }, ch: Some("s".into()) },
                Event::Key { t: 450, down: false, key: KeyStroke { code: "KeyS".into(), vk: 0x53, scan: 0x1F, ext: false }, ch: None },
                Event::PixelWait { t: 600, dur: 900, x: 10, y: 20, color: Rgb(0xEC, 0x30, 0x13), tolerance: 8, timeout_ms: 5000, label: String::new() },
            ],
        }
    }

    #[test]
    fn rly_snapshot() {
        insta::assert_snapshot!(to_rly(&fixed_macro()));
    }

    #[test]
    fn round_trips() {
        let m = fixed_macro();
        assert_eq!(from_rly(&to_rly(&m)).unwrap(), m);
        // The developer export (with derived steps) imports too.
        assert_eq!(from_rly(&to_export_json(&m)).unwrap(), m);
    }

    #[test]
    fn rejects_foreign_and_newer_files() {
        assert!(matches!(from_rly("{}"), Err(FormatError::NotRelay)));
        assert!(matches!(from_rly(r#"{"format":"relay-macro","version":99}"#), Err(FormatError::TooNew(99))));
        assert!(matches!(from_rly("[1]"), Err(FormatError::NotRelay)));
    }

    #[test]
    fn migrates_the_m0_export() {
        let v0 = r##"{"format":"relay-macro","version":0,"name":"Old","events":[
            {"id":1,"t":0,"type":"move","x":100,"y":200},
            {"id":2,"t":10,"type":"click","x":100,"y":200,"btn":"Left","count":2,"label":"Field"},
            {"id":3,"t":400,"type":"key","key":"Ctrl + A"},
            {"id":4,"t":700,"type":"char","key":"H"},
            {"id":5,"t":785,"type":"char","key":"i"},
            {"id":6,"t":1100,"type":"wait","dur":700,"label":"Dialog opens"},
            {"id":7,"t":1800,"type":"cond","dur":900,"label":"Grey","x":5,"y":6,"color":"#9b9797"}
        ]}"##;
        let m = from_rly(v0).unwrap();
        assert_eq!(m.name, "Old");
        check_invariants(&m.events).unwrap();
        insta::assert_json_snapshot!(m.events);
        let kinds: Vec<_> = group_steps(&m.events, (&m.recording).into())
            .iter()
            .map(|s| serde_json::to_value(s).unwrap()["kind"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(kinds, ["click", "keys", "type", "wait", "pixel_wait"]);
    }
}
