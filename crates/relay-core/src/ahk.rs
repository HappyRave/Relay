//! AutoHotkey v2 export: a macro as a readable `.ahk` script, one commented
//! block per step. The script plays it on its own: screen coordinates, the
//! steps' timing (start to start, so a step's own length doesn't add up),
//! Speed, Repeat and Humanize as variables at the top, Esc (with Stop on key
//! press) and Ctrl+Alt+End to stop. A data file's rows and Find image
//! pictures are embedded.

use std::collections::BTreeSet;
use std::fmt::Write;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

use crate::data::DataTable;
use crate::keys::{self, Modifier};
use crate::model::{Event, Macro, MouseBtn, Ms, Repeat};
use crate::steps::{Step, StepKind, group_steps};
use crate::text::{self, Part};
use crate::timeline;

/// Base64 lines in a continuation section.
const B64_LINE: usize = 120;

/// The script that plays `m`, once per row of `data` if it has a data file.
pub fn script(m: &Macro, data: Option<&DataTable>) -> String {
    let steps = group_steps(&m.events, (&m.recording).into());
    let mut out = String::new();
    let o = &mut out;
    let has = |f: fn(&StepKind) -> bool| steps.iter().any(|s| f(&s.kind));
    let images = steps.iter().filter(|s| matches!(s.kind, StepKind::FindImage { .. })).count();

    let _ = writeln!(o, "; {}, exported from Relay.", m.name.replace('\n', " "));
    let _ = writeln!(o, "; It plays at once, after the countdown: click into the app it works in.");
    let stop = if m.playback.stop_on_key { "Esc or Ctrl+Alt+End" } else { "Ctrl+Alt+End" };
    let _ = writeln!(o, "; {stop} stops it.");
    o.push_str(
        "#Requires AutoHotkey v2.0\n\
         #SingleInstance Force\n\
         ; Relay's coordinates are physical pixels, on every monitor.\n\
         DllCall(\"SetThreadDpiAwarenessContext\", \"Ptr\", -4, \"Ptr\")\n\
         CoordMode \"Mouse\", \"Screen\"\n\
         CoordMode \"Pixel\", \"Screen\"\n\
         SetDefaultMouseSpeed 0\n\n",
    );

    let _ = writeln!(o, "Speed := {}  ; 2 plays twice as fast", number(m.playback.speed));
    match data {
        Some(d) => {
            let _ = writeln!(o, "Rows := [");
            for row in &d.rows {
                let pairs: Vec<String> = d
                    .columns
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| !c.is_empty())
                    .map(|(i, c)| format!("{}, {}", quote(c), quote(row.get(i).map_or("", String::as_str))))
                    .collect();
                let _ = writeln!(o, "    Row({}),", pairs.join(", "));
            }
            let _ = writeln!(o, "]");
            let _ = writeln!(o, "Repeat := Rows.Length  ; once per row of the data file");
        }
        None => {
            let n = match m.playback.repeat {
                Repeat::Count(n) => n.max(1),
                Repeat::Forever => 0,
            };
            let _ = writeln!(o, "Repeat := {n}  ; 0 loops until stopped");
        }
    }
    let jitter = if m.playback.humanize { m.playback.jitter_ms } else { 0 };
    let _ = writeln!(o, "Jitter := {jitter}  ; Humanize: each pause ± this many ms (0: exact)");
    let _ = writeln!(o, "Countdown := 3  ; seconds to click into the app first\n");

    let _ = writeln!(o, "^!End::ExitApp");
    if m.playback.stop_on_key {
        let _ = writeln!(o, "Esc::ExitApp");
    }
    // Nothing stays held down, however it ends.
    let _ = writeln!(o, "OnExit((*) => Send(\"{{Ctrl up}}{{Alt up}}{{Shift up}}{{LWin up}}\"))\n");

    for (i, s) in steps.iter().filter(|s| matches!(s.kind, StepKind::FindImage { .. })).enumerate() {
        if let StepKind::FindImage { image, .. } = &s.kind {
            let b64 = BASE64.encode(&image.0);
            let _ = writeln!(o, "Image{} := SaveImage(\"", i + 1);
            let _ = writeln!(o, "(Join");
            for line in b64.as_bytes().chunks(B64_LINE) {
                let _ = writeln!(o, "{}", std::str::from_utf8(line).expect("base64 is ASCII"));
            }
            let _ = writeln!(o, ")\", {})\n", i + 1);
        }
    }

    o.push_str(
        "Loop Countdown {\n    \
             ToolTip \"Playing in \" (Countdown - A_Index + 1) \"…\"\n    \
             Sleep 1000\n\
         }\n\
         ToolTip\n\
         Loop {\n    \
             if (Repeat && A_Index > Repeat)\n        \
                 break\n    \
             Play(A_Index)\n\
         }\n\
         ExitApp\n\n",
    );

    let _ = writeln!(o, "Play(N) {{");
    let mut image = 0;
    let mut prev = 0;
    for (i, s) in steps.iter().enumerate() {
        let n = i + 1;
        let _ = writeln!(o, "    ; {n}. {}", describe(&s.kind));
        let wait = s.t.saturating_sub(prev);
        prev = s.t;
        if matches!(s.kind, StepKind::Move { .. }) {
            let _ = writeln!(o, "    Pause({wait}, false)");
        } else {
            let _ = writeln!(o, "    Pause({wait})");
        }
        if matches!(s.kind, StepKind::FindImage { .. }) {
            image += 1;
        }
        step(o, &m.events, s, n, image);
    }
    let _ = writeln!(o, "    Pause({})", timeline::duration(&m.events).saturating_sub(prev));
    let _ = writeln!(o, "}}\n");

    helpers(o, has(|k| matches!(k, StepKind::PixelWait { .. })), images > 0, data.is_some());
    out
}

fn step(o: &mut String, events: &[Event], s: &Step, n: usize, image: usize) {
    let mods = || held_modifiers(events, s);
    match &s.kind {
        StepKind::Move { to_x, to_y, .. } => {
            let _ = writeln!(o, "    MouseMove {to_x}, {to_y}");
        }
        StepKind::Click { x, y, btn, count, .. } => {
            let (down, up) = mods();
            let click = match (btn, count) {
                (MouseBtn::Left, 1) => format!("{x} {y}"),
                (b, 1) => format!("{x} {y} {}", button(*b)),
                (b, c) => format!("{x} {y} {} {c}", button(*b)),
            };
            if down.is_empty() {
                let _ = writeln!(o, "    Click \"{click}\"");
            } else {
                let _ = writeln!(o, "    Send \"{down}{{Click {click}}}{up}\"");
            }
        }
        StepKind::Drag { x, y, to_x, to_y, btn, .. } => {
            let (down, up) = mods();
            if !down.is_empty() {
                let _ = writeln!(o, "    Send \"{down}\"");
            }
            let _ = writeln!(o, "    MouseClickDrag \"{}\", {x}, {y}, {to_x}, {to_y}, 5", button(*btn));
            if !up.is_empty() {
                let _ = writeln!(o, "    Send \"{up}\"");
            }
        }
        StepKind::Scroll { x, y, delta, horizontal } => {
            let (down, up) = mods();
            let dir = match (horizontal, *delta >= 0) {
                (false, true) => "WheelUp",
                (false, false) => "WheelDown",
                (true, true) => "WheelRight",
                (true, false) => "WheelLeft",
            };
            let notches = ((delta.unsigned_abs() as f64) / 120.0).round().max(1.0);
            let _ = writeln!(o, "    Send \"{down}{{Click {x} {y} {dir} {notches}}}{up}\"");
        }
        StepKind::Keys { combo } => {
            let _ = writeln!(o, "    Send {}", quote(&send_combo(combo)));
        }
        StepKind::Type { text, .. } => {
            let _ = writeln!(o, "    SendText {}", quote(text));
        }
        StepKind::Text { text, .. } => match template(text) {
            Some(expr) => {
                let _ = writeln!(o, "    SendText {expr}");
            }
            None => {
                let _ = writeln!(o, "    ; (types nothing)");
            }
        },
        StepKind::Wait { .. } => {}
        StepKind::PixelWait { x, y, color, tolerance, timeout_ms, .. } => {
            let hex = format!("0x{:02X}{:02X}{:02X}", color.0, color.1, color.2);
            let _ = writeln!(o, "    WaitPixel({x}, {y}, {hex}, {tolerance}, {timeout_ms}, {n})");
        }
        StepKind::FindImage { click_x, click_y, btn, threshold, timeout_ms, area, .. } => {
            let area = match area {
                Some(r) => format!("[{}, {}, {}, {}]", r.x, r.y, r.x + r.w - 1, r.y + r.h - 1),
                None => "Screen()".into(),
            };
            let _ = writeln!(
                o,
                "    FindImage(Image{image}, {}, {area}, {click_x}, {click_y}, \"{}\", {timeout_ms}, {n})",
                variation(*threshold),
                button(*btn)
            );
        }
    }
}

/// The modifiers held during a pointer step (a Shift-click, or Ctrl held
/// over several clicks), as the `{… down}` and `{… up}` keys around it.
fn held_modifiers(events: &[Event], s: &Step) -> (String, String) {
    let at = s
        .items
        .iter()
        .map(|&i| &events[i as usize])
        .find(|e| !matches!(e, Event::Key { key, .. } if key.modifier().is_some()))
        .map_or(s.t, Event::t);
    let mut held = BTreeSet::new();
    for e in events.iter().take_while(|e| e.t() <= at) {
        if let Event::Key { down, key, .. } = e
            && let Some(m) = key.modifier()
        {
            if *down {
                held.insert(m);
            } else {
                held.remove(&m);
            }
        }
    }
    let name = |m: &Modifier| match m {
        Modifier::Ctrl => "Ctrl",
        Modifier::Alt => "Alt",
        Modifier::Shift => "Shift",
        Modifier::Win => "LWin",
    };
    let down = held.iter().map(|m| format!("{{{} down}}", name(m))).collect();
    let up = held.iter().rev().map(|m| format!("{{{} up}}", name(m))).collect();
    (down, up)
}

/// A KEYS step's combo ("Ctrl", "S") as `Send` keys ("^s").
fn send_combo(combo: &[String]) -> String {
    let prefix = |label: &str| match label {
        "Ctrl" => Some("^"),
        "Alt" => Some("!"),
        "Shift" => Some("+"),
        "Win" => Some("#"),
        _ => None,
    };
    let (mods, key) = match combo.split_last() {
        Some((key, mods)) if prefix(key).is_none() => (mods, key.as_str()),
        // A modifier on its own (a tap of Win).
        _ => {
            let names: Vec<String> =
                combo.iter().map(|m| format!("{{{}}}", if m == "Win" { "LWin" } else { m })).collect();
            return names.concat();
        }
    };
    let mut out: String = mods.iter().filter_map(|m| prefix(m)).collect();
    out.push_str(&send_key(key));
    out
}

/// One key's label as `Send` writes it: a letter or digit as it is, a
/// symbol Send treats specially in braces, a named key in braces.
fn send_key(label: &str) -> String {
    let mut chars = label.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        return match c {
            '^' | '!' | '+' | '#' | '{' | '}' => format!("{{{c}}}"),
            c => c.to_lowercase().to_string(),
        };
    }
    let name = match label {
        "Esc" => "Esc".to_string(),
        "PgUp" | "PgDn" | "Del" | "Ins" => label.to_string(),
        "ContextMenu" => "AppsKey".to_string(),
        "Backspace" | "Enter" | "Tab" | "Space" | "Home" | "End" | "Left" | "Right" | "Up" | "Down" => {
            label.to_string()
        }
        l if l.starts_with("Num ") => format!("Numpad{}", &l[4..]),
        l => keys::label(l),
    };
    format!("{{{name}}}")
}

/// A Text step's template as an AutoHotkey expression; `None` if it types nothing.
fn template(t: &str) -> Option<String> {
    let parts = match text::parse(t) {
        Ok(parts) => parts,
        // A hand-edited file with a mistake: typed as it is, as Relay does.
        Err(_) => vec![Part::Literal(t.to_string())],
    };
    let exprs: Vec<String> = parts
        .iter()
        .map(|p| match p {
            Part::Literal(s) => quote(s),
            Part::Date => "FormatTime(, \"yyyy-MM-dd\")".into(),
            Part::Time => "FormatTime(, \"HH:mm:ss\")".into(),
            Part::Clipboard => "A_Clipboard".into(),
            Part::Repeat => "N".into(),
            Part::Column(name) => format!("Col(N, {})", quote(name)),
        })
        .collect();
    (!exprs.is_empty()).then(|| exprs.join(" . "))
}

/// A quoted AutoHotkey string.
fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '`' => out.push_str("``"),
            '"' => out.push_str("`\""),
            '\n' => out.push_str("`n"),
            '\r' => out.push_str("`r"),
            '\t' => out.push_str("`t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn button(b: MouseBtn) -> &'static str {
    match b {
        MouseBtn::Left => "Left",
        MouseBtn::Right => "Right",
        MouseBtn::Middle => "Middle",
        MouseBtn::X1 => "X1",
        MouseBtn::X2 => "X2",
    }
}

/// ImageSearch's color variation for a Find image threshold: it compares
/// pixels, not shapes, so 100 % is exact and every point below allows a
/// little more (85 % → 30).
fn variation(threshold: u8) -> u32 {
    (100 - u32::from(threshold.min(100))) * 2
}

fn number(v: f32) -> String {
    let s = format!("{v}");
    s.strip_suffix(".0").map_or(s.clone(), str::to_string)
}

fn describe(kind: &StepKind) -> String {
    let named = |what: &str, label: &str| match label.is_empty() {
        true => what.to_string(),
        false => format!("{what} · {}", label.replace('\n', " ")),
    };
    match kind {
        StepKind::Move { .. } => "Move".into(),
        StepKind::Click { count: 2, label, .. } => named("Double-click", label),
        StepKind::Click { btn: MouseBtn::Right, label, .. } => named("Right-click", label),
        StepKind::Click { label, .. } => named("Click", label),
        StepKind::Drag { label, .. } => named("Drag", label),
        StepKind::Scroll { .. } => "Scroll".into(),
        StepKind::Keys { combo } => format!("Keys · {}", combo.join(" + ")),
        StepKind::Type { .. } => "Type".into(),
        StepKind::Text { .. } => "Text".into(),
        StepKind::Wait { dur, label } => named(&format!("Wait {}", seconds(*dur)), label),
        StepKind::PixelWait { label, .. } => named("Pixel check", label),
        StepKind::FindImage { label, .. } => named("Find image", label),
    }
}

fn seconds(ms: Ms) -> String {
    format!("{} s", number(ms as f32 / 1000.0))
}

fn helpers(o: &mut String, pixel: bool, image: bool, data: bool) {
    o.push_str(
        "Pause(ms, humanize := true) {\n    \
             if (humanize && Jitter)\n        \
                 ms += Random(-Jitter, Jitter)\n    \
             if (ms > 0)\n        \
                 Sleep Round(ms / Speed)\n\
         }\n\n\
         Stop(message, code) {\n    \
             MsgBox message, \"Relay macro\", \"Icon!\"\n    \
             ExitApp code\n\
         }\n",
    );
    if pixel {
        o.push_str(
            "\nWaitPixel(x, y, color, tolerance, timeout, step) {\n    \
                 start := A_TickCount\n    \
                 Loop {\n        \
                     c := PixelGetColor(x, y)\n        \
                     if (Abs((c >> 16) - (color >> 16)) <= tolerance\n            \
                         && Abs(((c >> 8) & 0xFF) - ((color >> 8) & 0xFF)) <= tolerance\n            \
                         && Abs((c & 0xFF) - (color & 0xFF)) <= tolerance)\n            \
                         return\n        \
                     if (A_TickCount - start > timeout)\n            \
                         Stop(\"Pixel check timed out at step \" step \"; playback stopped.\", 5)\n        \
                     Sleep 30\n    \
                 }\n\
             }\n",
        );
    }
    if image {
        o.push_str(
            "\n; The whole desktop, every monitor.\n\
             Screen() {\n    \
                 x := SysGet(76), y := SysGet(77)\n    \
                 return [x, y, x + SysGet(78) - 1, y + SysGet(79) - 1]\n\
             }\n\n\
             FindImage(file, variation, area, clickX, clickY, button, timeout, step) {\n    \
                 start := A_TickCount\n    \
                 Loop {\n        \
                     if ImageSearch(&x, &y, area[1], area[2], area[3], area[4], \"*\" variation \" \" file) {\n            \
                         Click x + clickX, y + clickY, button\n            \
                         return\n        \
                     }\n        \
                     if (A_TickCount - start > timeout)\n            \
                         Stop(\"Image not found at step \" step \"; playback stopped.\", 5)\n        \
                     Sleep 250\n    \
                 }\n\
             }\n\n\
             ; Writes a picture embedded as base64 to a temporary file, for ImageSearch.\n\
             SaveImage(b64, n) {\n    \
                 size := 0\n    \
                 DllCall(\"Crypt32\\CryptStringToBinary\", \"Str\", b64, \"UInt\", 0, \"UInt\", 1, \"Ptr\", 0, \"UInt*\", &size, \"Ptr\", 0, \"Ptr\", 0)\n    \
                 buf := Buffer(size)\n    \
                 DllCall(\"Crypt32\\CryptStringToBinary\", \"Str\", b64, \"UInt\", 0, \"UInt\", 1, \"Ptr\", buf, \"UInt*\", &size, \"Ptr\", 0, \"Ptr\", 0)\n    \
                 path := A_Temp \"\\relay-image-\" ProcessExist() \"-\" n \".png\"\n    \
                 f := FileOpen(path, \"w\")\n    \
                 f.RawWrite(buf, size)\n    \
                 f.Close()\n    \
                 return path\n\
             }\n",
        );
    }
    if data {
        o.push_str(
            "\n; A row of the data file: its columns, found whatever their case.\n\
             Row(pairs*) {\n    \
                 row := Map()\n    \
                 row.CaseSense := false\n    \
                 Loop pairs.Length // 2\n        \
                     row[pairs[A_Index * 2 - 1]] := pairs[A_Index * 2]\n    \
                 return row\n\
             }\n\n\
             Col(n, name) {\n    \
                 return Rows[n].Get(name, \"\")\n\
             }\n",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::KeyStroke;
    use crate::model::{ImagePng, RecordingMeta, Rect, Rgb};

    fn key(t: Ms, code: &str, down: bool) -> Event {
        Event::Key { t, down, key: KeyStroke::code(code), ch: None }
    }
    fn btn(t: Ms, x: i32, y: i32, down: bool) -> Event {
        Event::Button { t, x, y, btn: MouseBtn::Left, down, label: String::new() }
    }

    fn mac(events: Vec<Event>) -> Macro {
        let mut m = Macro::new("Invoice \"run\"", RecordingMeta::single_1080p(), events);
        m.playback.humanize = false;
        m
    }

    #[test]
    fn strings_are_quoted_for_autohotkey() {
        assert_eq!(quote("a \"b\" `c`\n\td"), r#""a `"b`" ``c```n`td""#);
        assert_eq!(quote(""), "\"\"");
    }

    #[test]
    fn combos_become_send_keys() {
        let combo = |c: &[&str]| send_combo(&c.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(combo(&["Ctrl", "S"]), "^s");
        assert_eq!(combo(&["Ctrl", "Shift", "Esc"]), "^+{Esc}");
        assert_eq!(combo(&["Alt", "F4"]), "!{F4}");
        assert_eq!(combo(&["Enter"]), "{Enter}");
        assert_eq!(combo(&["Ctrl", "+"]), "^{+}");
        assert_eq!(combo(&["Win"]), "{LWin}");
        assert_eq!(combo(&["Ctrl", "Num 5"]), "^{Numpad5}");
        assert_eq!(combo(&["Del"]), "{Del}");
        assert_eq!(combo(&["Win", "Left"]), "#{Left}");
    }

    #[test]
    fn templates_become_expressions() {
        assert_eq!(template(""), None);
        assert_eq!(
            template("Invoice {date} #{n}: {col:Customer} {clipboard} {{x}}").unwrap(),
            "\"Invoice \" . FormatTime(, \"yyyy-MM-dd\") . \" #\" . N . \": \" . Col(N, \"Customer\") . \" \" . A_Clipboard . \" {x}\""
        );
        assert_eq!(template("{time}").unwrap(), "FormatTime(, \"HH:mm:ss\")");
        assert_eq!(template("50% {off").unwrap(), "\"50% {off\"", "a mistake is typed as it is");
    }

    #[test]
    fn a_threshold_becomes_a_color_variation() {
        assert_eq!((variation(100), variation(85), variation(50)), (0, 30, 100));
    }

    #[test]
    fn modifiers_held_over_a_click_wrap_it() {
        let events = vec![
            key(0, "ShiftLeft", true),
            btn(100, 10, 20, true),
            btn(150, 10, 20, false),
            key(200, "ShiftLeft", false),
            btn(400, 30, 40, true),
            btn(450, 30, 40, false),
        ];
        let s = script(&mac(events), None);
        assert!(s.contains("    Send \"{Shift down}{Click 10 20}{Shift up}\"\n"), "{s}");
        assert!(s.contains("    Click \"30 40\"\n"), "{s}");
    }

    #[test]
    fn a_script_plays_every_step_with_its_timing() {
        let mut m = mac(vec![
            Event::Move { t: 0, x: 500, y: 500 },
            Event::Move { t: 80, x: 840, y: 412 },
            Event::Button { t: 100, x: 840, y: 412, btn: MouseBtn::Left, down: true, label: "Save".into() },
            Event::Button { t: 150, x: 840, y: 412, btn: MouseBtn::Left, down: false, label: "Save".into() },
            key(400, "ControlLeft", true),
            key(420, "KeyS", true),
            key(480, "KeyS", false),
            key(500, "ControlLeft", false),
            Event::Wait { t: 600, dur: 700, label: "Dialog opens".into() },
            Event::PixelWait {
                t: 1300,
                dur: 200,
                x: 5,
                y: 6,
                color: Rgb(0xEC, 0x30, 0x13),
                tolerance: 8,
                timeout_ms: 5000,
                label: "Ready".into(),
            },
            Event::Text { t: 1600, dur: 300, text: "Invoice {n}".into() },
            Event::Wheel { t: 2000, x: 900, y: 600, delta: -240, horizontal: false },
        ]);
        m.playback.repeat = Repeat::Count(3);
        m.playback.speed = 2.0;
        m.playback.humanize = true;
        insta::assert_snapshot!(script(&m, None));
    }

    #[test]
    fn a_data_file_and_images_are_embedded() {
        let png = crate::image::Rgb8 { w: 4, h: 4, px: vec![200; 48] }.encode_png();
        let mut m = mac(vec![
            Event::FindImage {
                t: 0,
                dur: 800,
                image: ImagePng(png.clone()),
                click_x: 2,
                click_y: 3,
                btn: MouseBtn::Left,
                threshold: 85,
                timeout_ms: 5000,
                area: None,
                label: "OK".into(),
            },
            Event::FindImage {
                t: 1000,
                dur: 800,
                image: ImagePng(png),
                click_x: 0,
                click_y: 0,
                btn: MouseBtn::Right,
                threshold: 100,
                timeout_ms: 2000,
                area: Some(Rect { x: -1920, y: 0, w: 100, h: 50 }),
                label: String::new(),
            },
            Event::Text { t: 2000, dur: 100, text: "{col:Customer}".into() },
        ]);
        m.playback.repeat = Repeat::Forever;
        m.playback.stop_on_key = false;
        let data = crate::data::parse("Customer,,Note\nACME,x,\"say \"\"hi\"\"\"\nGlobex".as_bytes()).unwrap();
        let s = script(&m, Some(&data));
        assert!(s.contains("    Row(\"Customer\", \"ACME\", \"Note\", \"say `\"hi`\"\"),\n"), "{s}");
        assert!(s.contains("    Row(\"Customer\", \"Globex\", \"Note\", \"\"),\n"), "{s}");
        assert!(s.contains("Repeat := Rows.Length"));
        assert!(!s.contains("Esc::ExitApp"), "without Stop on key press");
        assert!(s.contains("FindImage(Image1, 30, Screen(), 2, 3, \"Left\", 5000, 1)"), "{s}");
        assert!(s.contains("FindImage(Image2, 0, [-1920, 0, -1821, 49], 0, 0, \"Right\", 2000, 2)"), "{s}");
        assert!(s.contains("SendText Col(N, \"Customer\")"));
        assert_eq!(s.matches("SaveImage(\"").count(), 2);
        assert!(s.contains("Col(n, name)") && s.contains("SaveImage(b64, n)") && !s.contains("WaitPixel(x, y"));
    }

    #[test]
    fn every_sample_exports() {
        for sample in crate::samples::all() {
            let s = script(&sample.macro_, None);
            assert!(s.starts_with(&format!("; {}, exported from Relay", sample.macro_.name)));
            let steps = group_steps(&sample.macro_.events, (&sample.macro_.recording).into()).len();
            assert_eq!(s.matches("\n    ; ").count(), steps, "a block per step in {}", sample.macro_.name);
        }
    }
}
