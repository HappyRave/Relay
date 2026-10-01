//! The `.rly` file format: a versioned JSON envelope around [`Macro`].
//!
//! Loading parses into a JSON value first, runs the migration chain up to
//! [`VERSION`], then deserializes. Files from a newer Relay are rejected.
//!
//! v2 only adds the `find_image` event and v3 the `text` event, so a macro
//! is written with the oldest version that has its events, which older
//! Relays read.
//!
//! An exported program is the player's exe with the `.rly` appended, then a
//! trailer: its length (u64 LE), [`BUNDLE_VERSION`] (u32 LE) and
//! [`BUNDLE_MAGIC`]. Windows ignores data after the image it loads.

use chrono::Utc;
use serde::Serialize;
use serde_json::{Map, Value, json};
use thiserror::Error;
use uuid::Uuid;

use crate::edit::normalize;
use crate::keys::{code_for_label, key_for_char, split_combo};
use crate::model::{Event, Macro, PlaybackOptions, RecordingMeta};
use crate::steps::{Step, group_steps};

pub const FORMAT: &str = "relay-macro";
pub const VERSION: u64 = 3;
/// The last 8 bytes of an exported program.
pub const BUNDLE_MAGIC: &[u8; 8] = b"RELAYRLY";
pub const BUNDLE_VERSION: u32 = 1;
const TRAILER: usize = 8 + 4 + 8;

#[derive(Debug, Error)]
pub enum FormatError {
    #[error("not a Relay macro")]
    NotRelay,
    #[error("not a program exported by Relay")]
    NotRelayProgram,
    #[error("this macro was saved by a newer Relay (format version {0})")]
    TooNew(u64),
    #[error("invalid macro file: {0}")]
    Invalid(String),
}

#[derive(Serialize)]
struct Envelope<'a> {
    format: &'static str,
    version: u64,
    #[serde(flatten)]
    body: &'a Macro,
    /// Derived, for developers reading an export; ignored when importing.
    #[serde(skip_serializing_if = "Option::is_none")]
    steps: Option<Vec<Step>>,
}

impl<'a> Envelope<'a> {
    fn new(body: &'a Macro, steps: Option<Vec<Step>>) -> Self {
        let has = |f: fn(&Event) -> bool| body.events.iter().any(f);
        let version = if has(|e| matches!(e, Event::Text { .. })) {
            3
        } else if has(|e| matches!(e, Event::FindImage { .. })) {
            2
        } else {
            1
        };
        Envelope { format: FORMAT, version, body, steps }
    }
}

/// Serializes a macro as a compact `.rly` document.
pub fn to_rly(m: &Macro) -> String {
    serde_json::to_string(&Envelope::new(m, None)).expect("macro serializes")
}

/// The pretty-printed JSON export, including the derived steps.
pub fn to_export_json(m: &Macro) -> String {
    let steps = group_steps(&m.events, (&m.recording).into());
    serde_json::to_string_pretty(&Envelope::new(m, Some(steps))).expect("macro serializes")
}

/// Parses a `.rly` (or exported `.json`) document of any supported version.
pub fn from_rly(s: &str) -> Result<Macro, FormatError> {
    let invalid = |e: serde_json::Error| FormatError::Invalid(e.to_string());
    // Not even JSON: say so plainly rather than quoting the parser, unless
    // it's a damaged macro (a truncated file still names its format).
    let mut v: Value = serde_json::from_str(s)
        .map_err(|e| if s.contains(&format!("\"{FORMAT}\"")) { invalid(e) } else { FormatError::NotRelay })?;
    let obj = v.as_object_mut().ok_or(FormatError::NotRelay)?;
    if obj.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err(FormatError::NotRelay);
    }
    let mut version =
        obj.get("version").and_then(Value::as_u64).ok_or_else(|| FormatError::Invalid("no format version".into()))?;
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
    let mut m: Macro = serde_json::from_value(v).map_err(invalid)?;
    normalize(&mut m.events);
    Ok(m)
}

/// An exported program: the player `stub` playing `m`.
pub fn bundle(stub: &[u8], m: &Macro) -> Vec<u8> {
    let rly = to_rly(m);
    let mut out = Vec::with_capacity(stub.len() + rly.len() + TRAILER);
    out.extend_from_slice(stub);
    out.extend_from_slice(rly.as_bytes());
    out.extend_from_slice(&(rly.len() as u64).to_le_bytes());
    out.extend_from_slice(&BUNDLE_VERSION.to_le_bytes());
    out.extend_from_slice(BUNDLE_MAGIC);
    out
}

/// The `.rly` inside an exported program.
pub fn unbundle(file: &[u8]) -> Result<&str, FormatError> {
    let Some(body) = file.len().checked_sub(TRAILER) else { return Err(FormatError::NotRelayProgram) };
    let (rest, trailer) = file.split_at(body);
    if &trailer[12..] != BUNDLE_MAGIC {
        return Err(FormatError::NotRelayProgram);
    }
    let version = u32::from_le_bytes(trailer[8..12].try_into().expect("4 bytes"));
    if version > BUNDLE_VERSION {
        return Err(FormatError::TooNew(version.into()));
    }
    let len = u64::from_le_bytes(trailer[..8].try_into().expect("8 bytes"));
    let start = usize::try_from(len)
        .ok()
        .and_then(|len| rest.len().checked_sub(len))
        .ok_or_else(|| FormatError::Invalid("the program's macro is cut short".into()))?;
    std::str::from_utf8(&rest[start..]).map_err(|e| FormatError::Invalid(e.to_string()))
}

/// Reads a `.rly`, a JSON export or an exported program (an exe starts with "MZ").
pub fn from_file(bytes: &[u8]) -> Result<Macro, FormatError> {
    if bytes.starts_with(b"MZ") {
        return from_rly(unbundle(bytes)?);
    }
    from_rly(std::str::from_utf8(bytes).map_err(|_| FormatError::NotRelay)?)
}

fn migrate(from: u64, obj: &mut Map<String, Value>) -> Result<(), FormatError> {
    match from {
        0 => migrate_v0(obj),
        // v2 and v3 each added an event type: older files are newer ones as they are.
        1 | 2 => Ok(()),
        _ => Err(FormatError::Invalid(format!("no migration from version {from}"))),
    }
}

/// v0 is the M0 prototype's export: high-level events (`click`, `key` combos
/// like "Ctrl + A", `char`, `wait`, `cond`) and no recording metadata. v1
/// stores presses and releases, so each high-level event is expanded.
fn migrate_v0(obj: &mut Map<String, Value>) -> Result<(), FormatError> {
    let bad = |what: &str| FormatError::Invalid(format!("v0 event without {what}"));
    let Some(Value::Array(events)) = obj.remove("events") else {
        return Err(bad("list"));
    };
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
                    let label = text("label");
                    if i == 0 && !label.is_empty() {
                        d["label"] = json!(label);
                    }
                    out.push(d);
                    out.push(json!({ "type": "button", "t": down + 60, "x": x, "y": y, "btn": btn, "down": false }));
                }
            }
            "key" => {
                let combo = text("key");
                let parts = split_combo(&combo).ok_or_else(|| bad("key"))?;
                let parts: Vec<String> = parts.into_iter().map(code_for_label).collect();
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
    use crate::model::{CoordMode, ImagePng, MouseBtn, Rect, Repeat, Rgb, WindowInfo};
    use crate::steps::StepKind;
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
                Event::Key {
                    t: 400,
                    down: true,
                    key: KeyStroke { code: "KeyS".into(), vk: 0x53, scan: 0x1F, ext: false },
                    ch: Some("s".into()),
                },
                Event::Key {
                    t: 450,
                    down: false,
                    key: KeyStroke { code: "KeyS".into(), vk: 0x53, scan: 0x1F, ext: false },
                    ch: None,
                },
                Event::PixelWait {
                    t: 600,
                    dur: 900,
                    x: 10,
                    y: 20,
                    color: Rgb(0xEC, 0x30, 0x13),
                    tolerance: 8,
                    timeout_ms: 5000,
                    label: String::new(),
                },
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
    fn every_event_and_option_round_trips() {
        let mut m = fixed_macro();
        m.recording.virtual_desktop = Rect { x: -1920, y: -200, w: 3840, h: 1280 };
        m.recording.anchor_window = Some(WindowInfo {
            exe: "EXCEL.EXE".into(),
            class: "XLMAIN".into(),
            title: "Book1 – Excel".into(),
            rect: Rect { x: -1900, y: -180, w: 1200, h: 800 },
        });
        m.playback = PlaybackOptions {
            speed: 2.5,
            repeat: Repeat::Forever,
            humanize: false,
            jitter_ms: 0,
            stop_on_key: false,
            coord_mode: CoordMode::Window,
        };
        let right = KeyStroke { code: "ControlRight".into(), vk: 0xA3, scan: 0x1D, ext: true };
        m.events = vec![
            Event::Move { t: 0, x: -1500, y: -100 },
            Event::Button { t: 10, x: -1500, y: -100, btn: MouseBtn::X2, down: true, label: "Back".into() },
            Event::Button { t: 20, x: -1500, y: -100, btn: MouseBtn::X2, down: false, label: String::new() },
            Event::Wheel { t: 30, x: -1500, y: -100, delta: -240, horizontal: true },
            Event::Wheel { t: 40, x: -1500, y: -100, delta: 120, horizontal: false },
            Event::Key { t: 50, down: true, key: right.clone(), ch: None },
            Event::Key { t: 60, down: true, key: KeyStroke::code("Unidentified"), ch: Some("😀é".into()) },
            Event::Key { t: 70, down: false, key: KeyStroke::code("Unidentified"), ch: None },
            Event::Key { t: 80, down: false, key: right, ch: None },
            Event::Wait { t: 100, dur: 700, label: "Dialog".into() },
            Event::PixelWait {
                t: 800,
                dur: 0,
                x: -5,
                y: -6,
                color: Rgb(0, 0xFF, 0x7F),
                tolerance: 255,
                timeout_ms: 0,
                label: "Ready".into(),
            },
        ];
        check_invariants(&m.events).unwrap();
        let text = to_rly(&m);
        assert!(text.contains(r#""ext":true"#) && text.contains(r#""horizontal":true"#), "{text}");
        assert!(text.contains(r#""repeat":"forever""#) && text.contains(r#""coord_mode":"window""#), "{text}");
        assert_eq!(from_rly(&text).unwrap(), m);
        assert_eq!(from_rly(&to_export_json(&m)).unwrap(), m);
        m.playback.repeat = Repeat::Count(3);
        assert_eq!(from_rly(&to_rly(&m)).unwrap(), m);
    }

    #[test]
    fn a_macro_that_finds_an_image_is_v2_and_round_trips() {
        let mut m = fixed_macro();
        assert!(to_rly(&m).contains(r#""version":1,"#), "no image: still v1");
        let png = crate::image::Rgb8 { w: 8, h: 8, px: (0..192).map(|i| (i * 37 % 256) as u8).collect() }.encode_png();
        m.events.push(Event::FindImage {
            t: 1500,
            dur: 800,
            image: ImagePng(png),
            click_x: 4,
            click_y: -2,
            btn: MouseBtn::Right,
            threshold: 85,
            timeout_ms: 5000,
            area: Some(Rect { x: -1920, y: 0, w: 1920, h: 1080 }),
            label: "OK".into(),
        });
        let text = to_rly(&m);
        assert!(text.contains(r#""version":2,"#) && text.contains(r#""image":"iVBORw0KGgo"#), "{text}");
        assert_eq!(from_rly(&text).unwrap(), m);
        assert_eq!(from_rly(&to_export_json(&m)).unwrap(), m);
        let e = from_rly(&text.replace(r#""image":"iVBOR"#, r#""image":"AAAA"#)).unwrap_err().to_string();
        assert!(e.contains("invalid image"), "{e}");
        let old = text.replace(r#""version":2,"#, r#""version":1,"#);
        assert_eq!(from_rly(&old).unwrap(), m, "no migration needed");
    }

    #[test]
    fn a_macro_that_types_text_is_v3_and_round_trips() {
        let mut m = fixed_macro();
        m.events.push(Event::Text { t: 1500, dur: 400, text: "Invoice {date} {{draft}} #{n}\n".into() });
        let text = to_rly(&m);
        assert!(text.contains(r#""version":3,"#), "{text}");
        assert!(
            text.contains(r#"{"type":"text","t":1500,"dur":400,"text":"Invoice {date} {{draft}} #{n}\n"}"#),
            "{text}"
        );
        assert_eq!(from_rly(&text).unwrap(), m);
        assert_eq!(from_rly(&to_export_json(&m)).unwrap(), m);
        let png = crate::image::Rgb8 { w: 4, h: 4, px: vec![200; 48] }.encode_png();
        m.events.push(Event::FindImage {
            t: 2000,
            dur: 800,
            image: ImagePng(png),
            click_x: 2,
            click_y: 2,
            btn: MouseBtn::Left,
            threshold: 85,
            timeout_ms: 5000,
            area: None,
            label: String::new(),
        });
        assert!(to_rly(&m).contains(r#""version":3,"#), "the newest event decides");
        for old in [1, 2] {
            let older = text.replace(r#""version":3,"#, &format!(r#""version":{old},"#));
            assert!(from_rly(&older).is_ok(), "no migration needed from v{old}");
        }
        let exe = bundle(STUB, &m);
        assert_eq!(from_file(&exe).unwrap(), m);
    }

    /// Something shaped like the player: an MZ header and some bytes.
    const STUB: &[u8] = b"MZ\x90\0the player\0\0";

    #[test]
    fn a_program_carries_its_macro() {
        let mut m = fixed_macro();
        m.name = "Relevé mensuel ✓".into();
        let exe = bundle(STUB, &m);
        assert!(exe.starts_with(STUB) && exe.ends_with(BUNDLE_MAGIC));
        assert_eq!(unbundle(&exe).unwrap(), to_rly(&m));
        assert_eq!(from_file(&exe).unwrap(), m);
    }

    #[test]
    fn a_program_that_finds_an_image_carries_a_v2_macro() {
        let mut m = fixed_macro();
        let png = crate::image::Rgb8 { w: 4, h: 4, px: vec![200; 48] }.encode_png();
        m.events.push(Event::FindImage {
            t: 1500,
            dur: 800,
            image: ImagePng(png),
            click_x: 2,
            click_y: 2,
            btn: MouseBtn::Left,
            threshold: 85,
            timeout_ms: 5000,
            area: None,
            label: String::new(),
        });
        let exe = bundle(STUB, &m);
        assert!(unbundle(&exe).unwrap().contains(r#""version":2,"#));
        assert_eq!(from_file(&exe).unwrap(), m);
    }

    #[test]
    fn plain_macro_files_read_as_before() {
        let m = fixed_macro();
        assert_eq!(from_file(to_rly(&m).as_bytes()).unwrap(), m);
        assert_eq!(from_file(to_export_json(&m).as_bytes()).unwrap(), m);
        assert!(matches!(from_file(&[0xFF, 0xFE, 0x00]), Err(FormatError::NotRelay)), "not UTF-8");
    }

    #[test]
    fn other_programs_are_refused() {
        let program = |tail: &[u8]| [STUB, tail].concat();
        assert_eq!(from_file(b"MZ").unwrap_err().to_string(), "not a program exported by Relay", "too short");
        assert!(matches!(from_file(&program(&[0; 64])), Err(FormatError::NotRelayProgram)), "no trailer");
        let exe = bundle(STUB, &fixed_macro());
        let mut other = exe.clone();
        *other.last_mut().unwrap() = b'X';
        assert!(matches!(from_file(&other), Err(FormatError::NotRelayProgram)), "another magic");
    }

    #[test]
    fn a_damaged_or_newer_program_says_so() {
        let exe = bundle(STUB, &fixed_macro());
        let n = exe.len();
        let mut newer = exe.clone();
        newer[n - 12..n - 8].copy_from_slice(&2u32.to_le_bytes());
        assert!(matches!(unbundle(&newer), Err(FormatError::TooNew(2))));

        let mut too_long = exe.clone();
        too_long[n - 20..n - 12].copy_from_slice(&(n as u64).to_le_bytes());
        assert!(matches!(unbundle(&too_long), Err(FormatError::Invalid(_))), "longer than the file");
        let mut huge = exe.clone();
        huge[n - 20..n - 12].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(matches!(unbundle(&huge), Err(FormatError::Invalid(_))));

        // A byte of the macro lost: its JSON no longer parses.
        let mut cut = exe[..STUB.len()].to_vec();
        cut.extend_from_slice(&exe[STUB.len() + 1..]);
        let e = from_file(&cut).unwrap_err();
        assert!(matches!(e, FormatError::Invalid(_) | FormatError::NotRelay), "{e:?}");
    }

    #[test]
    fn loading_sorts_and_balances_the_events() {
        let key = |t, down| Event::Key { t, down, key: KeyStroke::code("KeyA"), ch: None };
        let btn = |t, down| Event::Button { t, x: 1, y: 1, btn: MouseBtn::Left, down, label: String::new() };
        let mut m = fixed_macro();
        // Out of order, a release without its press, and a press never released.
        m.events =
            vec![Event::Move { t: 300, x: 1, y: 1 }, key(100, false), btn(200, true), Event::Move { t: 0, x: 0, y: 0 }];
        let loaded = from_rly(&to_rly(&m)).unwrap();
        assert_eq!(
            loaded.events,
            [Event::Move { t: 0, x: 0, y: 0 }, btn(200, true), Event::Move { t: 300, x: 1, y: 1 }, btn(300, false)]
        );
        check_invariants(&loaded.events).unwrap();
    }

    #[test]
    fn broken_fields_are_invalid() {
        let good = to_rly(&fixed_macro());
        let cases = [
            (good.replace("#EC3013", "#EC30"), "invalid color"),
            (good.replace("#EC3013", "red"), "invalid color"),
            (good.replace(r#""dur":900"#, r#""dur":-900"#), "invalid value"),
            (good.replace(r#""btn":"Left","#, ""), "missing field `btn`"),
            (good.replace(r#""recording""#, r#""recorded""#), "missing field `recording`"),
        ];
        for (s, want) in cases {
            let e = from_rly(&s).unwrap_err().to_string();
            assert!(e.starts_with("invalid macro file: ") && e.contains(want), "{e}");
        }
    }

    #[test]
    fn broken_v0_files_are_invalid() {
        let v0 = |events: &str| format!(r#"{{"format":"relay-macro","version":0,"events":[{events}]}}"#);
        let cases = [
            (v0(r#"{"t":0,"type":"teleport"}"#), r#"unknown v0 event type "teleport""#),
            (v0(r#"{"type":"move","x":1,"y":1}"#), "v0 event without t"),
            (v0(r#"{"t":0,"type":"click","x":1}"#), "v0 event without y"),
            (v0(r#"{"t":0,"type":"char"}"#), "v0 event without key"),
            (v0(r#"{"t":0,"type":"key","key":""}"#), "v0 event without key"),
            (v0(r#"{"t":0,"type":"cond","dur":5,"x":1,"y":1,"color":"grey"}"#), "invalid color"),
            (r#"{"format":"relay-macro","version":0}"#.into(), "v0 event without list"),
        ];
        for (s, want) in cases {
            let e = from_rly(&s).unwrap_err().to_string();
            assert!(e.starts_with("invalid macro file: ") && e.contains(want), "{e} for {s}");
        }
        // A negative duration is refused, not wrapped.
        let e = from_rly(&v0(r#"{"t":0,"type":"wait","dur":-5}"#)).unwrap_err();
        assert!(matches!(e, FormatError::Invalid(_)), "{e:?}");
    }

    #[test]
    fn rejects_foreign_and_newer_files() {
        assert!(matches!(from_rly("{}"), Err(FormatError::NotRelay)));
        assert!(matches!(from_rly(r#"{"format":"relay-macro","version":99}"#), Err(FormatError::TooNew(99))));
        assert!(matches!(from_rly("[1]"), Err(FormatError::NotRelay)));
        assert_eq!(from_rly("not json").unwrap_err().to_string(), "not a Relay macro");
        assert!(matches!(from_rly(r#"{"format":"other","version":1}"#), Err(FormatError::NotRelay)));
    }

    #[test]
    fn a_damaged_macro_is_invalid_not_foreign() {
        let good = to_rly(&fixed_macro());
        let damaged = [
            good.replace(r#""type":"move""#, r#""type":"teleport""#), // unknown variant
            good.replace(r#""x":960"#, r#""x":"left""#),              // a field of the wrong type
            good.replace(r#""name":"Save the file","#, ""),           // a missing field
            good[..good.len() / 2].to_string(),                       // truncated
            good.replace(r#""version":1,"#, ""),                      // no version
        ];
        for s in damaged {
            let e = from_rly(&s).unwrap_err();
            assert!(matches!(e, FormatError::Invalid(_)), "{e:?} for {s}");
            assert!(e.to_string().starts_with("invalid macro file: "), "{e}");
        }
    }

    #[test]
    fn loading_moves_events_out_of_waits() {
        let mut m = fixed_macro();
        // A hand-edited file: a key typed during the pixel check (600..1500).
        m.events.push(Event::Key { t: 700, down: true, key: KeyStroke::code("KeyA"), ch: Some("a".into()) });
        m.events.push(Event::Key { t: 750, down: false, key: KeyStroke::code("KeyA"), ch: None });
        let loaded = from_rly(&to_rly(&m)).unwrap();
        check_invariants(&loaded.events).unwrap();
        assert_eq!(loaded.events.iter().map(Event::t).collect::<Vec<_>>(), [0, 120, 180, 400, 450, 600, 1500, 1500]);
    }

    #[test]
    fn a_v0_combo_can_use_the_plus_key() {
        let v0 = r#"{"format":"relay-macro","version":0,"events":[{"t":0,"type":"key","key":"Ctrl + +"}]}"#;
        let m = from_rly(v0).unwrap();
        let keys: Vec<_> = m
            .events
            .iter()
            .map(|e| match e {
                Event::Key { key, down, .. } => (key.code.as_str(), *down),
                e => panic!("{e:?}"),
            })
            .collect();
        assert_eq!(keys, [("ControlLeft", true), ("Equal", true), ("Equal", false), ("ControlLeft", false)]);
        let bad = r#"{"format":"relay-macro","version":0,"events":[{"t":0,"type":"key","key":"Ctrl +"}]}"#;
        assert!(matches!(from_rly(bad), Err(FormatError::Invalid(_))));
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
        assert_eq!(kinds, ["move", "click", "keys", "type", "wait", "pixel_wait"]);

        // The details the snapshot shows, spelled out.
        let all = group_steps(&m.events, (&m.recording).into());
        assert!(matches!(all[0].kind, StepKind::Move { to_x: 100, to_y: 200, samples: 1, .. }));
        let steps = &all[1..];
        let StepKind::Click { x: 100, y: 200, btn: MouseBtn::Left, count: 2, label } = &steps[0].kind else {
            panic!("{:?}", steps[0].kind)
        };
        assert_eq!(label, "Field");
        let presses: Vec<_> =
            m.events.iter().filter(|e| matches!(e, Event::Button { down: true, .. })).map(Event::t).collect();
        assert_eq!(presses, [10, 130], "two presses 120 ms apart");
        assert_eq!(steps[1].kind, StepKind::Keys { combo: vec!["Ctrl".into(), "A".into()] });
        assert!(matches!(&steps[2].kind, StepKind::Type { text, .. } if text == "Hi"));
        assert!(matches!(&steps[3].kind, StepKind::Wait { dur: 700, label } if label == "Dialog opens"));
        let StepKind::PixelWait { dur: 900, x: 5, y: 6, color, tolerance: 8, timeout_ms: 5000, label } = &steps[4].kind
        else {
            panic!("{:?}", steps[4].kind)
        };
        assert_eq!((color.to_hex().as_str(), label.as_str()), ("#9B9797", "Grey"));
        // The H is typed with Shift.
        assert!(m.events.iter().any(|e| matches!(e, Event::Key { key, down: true, .. } if key.code == "ShiftLeft")));
        assert_eq!(m.recording, RecordingMeta::single_1080p());
        assert_eq!(m.playback, PlaybackOptions::default());
    }
}
