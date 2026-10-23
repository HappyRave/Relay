//! The four sample macros from the design prototype (`buildSample` in
//! Design/Macro Recorder.dc.html). They are generated in the prototype's own
//! high-level (v0) shape and loaded through the v0 → v1 migration, which makes
//! them a realistic fixture for both.

use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::format::from_rly;
use crate::model::{Macro, Rect, Repeat, WindowInfo};
use crate::runlog::{CheckOutcome, CheckResult, RunEntry, RunOutcome, SkipReason};
use crate::session::{FinishReason, RunSource};
use crate::steps::{StepKind, group_steps};

pub struct Sample {
    pub macro_: Macro,
    pub runs: u32,
    /// How long before "now" the sample last ran.
    pub last_run_ago: Option<TimeDelta>,
    pub hotkey: Option<String>,
}

impl Sample {
    pub fn last_run(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        self.last_run_ago.map(|ago| now - ago)
    }
}

enum Op {
    Move(f64, f64, f64, f64),
    Click(&'static str, &'static str),
    Key(&'static str, f64),
    Type(&'static str),
    Wait(f64, &'static str),
    Pixel(f64, &'static str),
}
use Op::*;

const W: f64 = 1920.0;
const H: f64 = 1080.0;

fn ease(u: f64) -> f64 {
    if u < 0.5 { 2.0 * u * u } else { 1.0 - (-2.0 * u + 2.0).powi(2) / 2.0 }
}

/// Port of the prototype's `buildSample`, producing v0 events in pixels.
fn build(script: &[Op]) -> Vec<Value> {
    let (mut px, mut py, mut t) = (0.5, 0.62, 0.0f64);
    let at = |x: f64, y: f64| ((x * W).round(), (y * H).round());
    let mut ev = vec![json!({ "t": 0, "type": "move", "x": at(px, py).0, "y": at(px, py).1 })];
    for op in script {
        match *op {
            Move(x, y, dur, bend) => {
                let (mx, my) = ((px + x) / 2.0, (py + y) / 2.0);
                let (cx, cy) = (mx - (y - py) * bend, my + (x - px) * bend);
                let n = (dur / 16.0).round() as i32;
                for i in 1..=n {
                    let u = ease(i as f64 / n as f64);
                    let v = 1.0 - u;
                    let (qx, qy) =
                        (v * v * px + 2.0 * v * u * cx + u * u * x, v * v * py + 2.0 * v * u * cy + u * u * y);
                    let (sx, sy) = at(qx, qy);
                    ev.push(json!({ "t": (t + i as f64 * dur / n as f64).round(), "type": "move", "x": sx, "y": sy }));
                }
                t += dur;
                (px, py) = (x, y);
            }
            Click(label, btn) => {
                let (x, y) = at(px, py);
                let (btn, count) = if btn == "Double" { ("Left", 2) } else { (btn, 1) };
                ev.push(json!({ "t": t, "type": "click", "x": x, "y": y, "btn": btn, "count": count, "label": label }));
                t += 250.0;
            }
            Key(k, gap) => {
                ev.push(json!({ "t": t, "type": "key", "key": k }));
                t += gap;
            }
            Type(s) => {
                for c in s.chars() {
                    ev.push(json!({ "t": t, "type": "char", "key": c.to_string() }));
                    t += 85.0;
                }
                t += 200.0;
            }
            Wait(ms, label) => {
                ev.push(json!({ "t": t, "type": "wait", "dur": ms, "label": label }));
                t += ms;
            }
            Pixel(ms, label) => {
                let (x, y) = at(px, py);
                ev.push(
                    json!({ "t": t, "type": "cond", "dur": ms, "label": label, "x": x, "y": y, "color": "#9B9797" }),
                );
                t += ms;
            }
        }
    }
    ev
}

fn sample(n: u128, name: &str, script: &[Op], runs: u32, last_run_ago_h: Option<i64>, hotkey: Option<&str>) -> Sample {
    let doc = json!({ "format": "relay-macro", "version": 0, "name": name, "events": build(script) });
    let mut m = from_rly(&doc.to_string()).expect("sample migrates");
    // Stable ids and dates, so generated fixtures don't change between runs.
    m.id = Uuid::from_u128(n);
    m.created_at = Utc.with_ymd_and_hms(2026, 9, 1, 9, 0, 0).unwrap();
    m.modified_at = m.created_at;
    // The prototype's wireframe app window, scaled from its 1600×900 canvas.
    m.recording.anchor_window = Some(WindowInfo {
        exe: "EXCEL.EXE".into(),
        class: "XLMAIN".into(),
        title: name.into(),
        rect: Rect { x: 48, y: 36, w: 1416, h: 936 },
    });
    m.playback.repeat = Repeat::Count(3);
    Sample { macro_: m, runs, last_run_ago: last_run_ago_h.map(TimeDelta::hours), hotkey: hotkey.map(Into::into) }
}

pub fn invoice() -> Sample {
    sample(
        1,
        "Export invoice to PDF",
        &[
            Move(0.07, 0.065, 850.0, 0.25),
            Click("File menu", "Left"),
            Move(0.12, 0.3, 650.0, -0.3),
            Click("Export as PDF", "Left"),
            Wait(700.0, "Dialog opens"),
            Move(0.55, 0.47, 800.0, 0.2),
            Click("Filename field", "Double"),
            Key("Ctrl + A", 300.0),
            Type("invoice_0924"),
            Move(0.65, 0.63, 750.0, -0.2),
            Click("Save", "Left"),
            Pixel(900.0, "Save button turns grey"),
            Key("Ctrl + W", 400.0),
            Move(0.87, 0.16, 1100.0, 0.25),
            Click("Invoices folder", "Right"),
            Move(0.9, 0.3, 420.0, -0.15),
            Click("Upload to portal", "Left"),
            Key("Enter", 500.0),
        ],
        148,
        Some(3),
        Some("Ctrl + Alt + 1"),
    )
}

pub fn all() -> Vec<Sample> {
    vec![
        invoice(),
        sample(
            2,
            "Fill weekly timesheet",
            &[
                Move(0.3, 0.2, 700.0, 0.2),
                Click("Mon cell", "Left"),
                Type("8"),
                Key("Tab", 150.0),
                Type("8"),
                Key("Tab", 150.0),
                Type("8"),
                Key("Tab", 150.0),
                Type("7.5"),
                Key("Tab", 150.0),
                Type("6"),
                Move(0.62, 0.74, 900.0, -0.2),
                Click("Submit", "Left"),
                Wait(800.0, "Confirmation"),
                Move(0.52, 0.55, 500.0, 0.1),
                Click("OK", "Left"),
            ],
            36,
            Some(6 * 24 + 16),
            Some("Ctrl + Alt + 2"),
        ),
        sample(
            3,
            "Batch rename photos",
            &[
                Move(0.2, 0.35, 600.0, 0.2),
                Click("First photo", "Left"),
                Key("Ctrl + A", 300.0),
                Key("F2", 300.0),
                Type("trip_2026"),
                Key("Enter", 300.0),
                Pixel(1200.0, "Explorer refresh"),
                Move(0.8, 0.1, 900.0, -0.2),
                Click("Sort by date", "Left"),
                Move(0.82, 0.2, 300.0, 0.1),
                Click("Date taken", "Left"),
            ],
            4,
            Some(12 * 24),
            None,
        ),
        sample(
            4,
            "Open standup tools",
            &[
                Key("Win + R", 300.0),
                Type("teams"),
                Key("Enter", 300.0),
                Wait(1500.0, "Teams loads"),
                Move(0.1, 0.4, 700.0, 0.2),
                Click("Calendar", "Left"),
                Move(0.45, 0.3, 700.0, -0.2),
                Click("Standup", "Double"),
                Key("Ctrl + Shift + M", 300.0),
            ],
            212,
            Some(4),
            Some("Ctrl + Alt + 4"),
        ),
    ]
}

/// A run history for the samples, newest first: how long before "now" each
/// entry happened, and the entry (its `at` is filled in from that).
pub fn runs() -> Vec<(TimeDelta, RunEntry)> {
    let samples = all();
    let placeholder = Utc.with_ymd_and_hms(2026, 9, 1, 9, 0, 0).unwrap();
    let entry = |i: usize, source, outcome, duration_ms, loops, checks: Vec<CheckResult>| {
        let m = &samples[i].macro_;
        RunEntry {
            at: placeholder,
            macro_id: m.id,
            macro_name: m.name.clone(),
            source,
            outcome,
            duration_ms,
            from_ms: 0,
            loops,
            speed: m.playback.speed,
            humanize: m.playback.humanize,
            checks,
            checks_dropped: 0,
            note: None,
        }
    };
    // The 1-based step of a sample's pixel check.
    let pixel_step = |i: usize| {
        let m = &samples[i].macro_;
        let steps = group_steps(&m.events, (&m.recording).into());
        steps.iter().position(|s| matches!(s.kind, StepKind::PixelWait { .. })).expect("a pixel check") as u32 + 1
    };
    let check =
        |i, loop_idx, after_ms, outcome| CheckResult { step: pixel_step(i), loop_idx, image: false, after_ms, outcome };
    let done = RunOutcome::Finished(FinishReason::Completed);
    vec![
        (
            TimeDelta::hours(3),
            entry(
                0,
                RunSource::Hotkey,
                done,
                31_840,
                3,
                (0..3).map(|l| check(0, l, [640, 910, 420][l as usize], CheckOutcome::Matched)).collect(),
            ),
        ),
        (TimeDelta::hours(4), entry(3, RunSource::Schedule, done, 20_310, 3, vec![])),
        (TimeDelta::hours(27), entry(3, RunSource::Schedule, RunOutcome::Skipped(SkipReason::Busy), 0, 0, vec![])),
        (
            TimeDelta::hours(30),
            entry(
                2,
                RunSource::Manual,
                RunOutcome::Finished(FinishReason::PixelTimeout),
                9_120,
                1,
                vec![check(2, 0, 5000, CheckOutcome::TimedOut)],
            ),
        ),
        (TimeDelta::hours(52), entry(3, RunSource::Schedule, RunOutcome::Skipped(SkipReason::Missed), 0, 0, vec![])),
        (
            TimeDelta::hours(6 * 24 + 16),
            entry(1, RunSource::Manual, RunOutcome::Finished(FinishReason::Stopped), 4_730, 1, vec![]),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::check_invariants;
    use crate::steps::{StepKind, group_steps};

    fn describe(kind: &StepKind) -> String {
        match kind {
            StepKind::Click { btn, count, label, .. } => format!("CLICK {btn:?}×{count} {label}"),
            StepKind::Drag { .. } => "DRAG".into(),
            StepKind::Scroll { .. } => "SCROLL".into(),
            StepKind::Keys { combo } => format!("KEYS {}", combo.join(" + ")),
            StepKind::Type { text, .. } => format!("TYPE {text}"),
            StepKind::Wait { dur, label } => format!("WAIT {dur} {label}"),
            StepKind::PixelWait { label, .. } => format!("IF {label}"),
            StepKind::FindImage { label, .. } => format!("FIND {label}"),
            StepKind::Text { text, .. } => format!("TEXT {text}"),
            StepKind::Move { .. } => "MOVE".into(),
        }
    }

    #[test]
    fn invoice_groups_into_the_designs_twelve_steps() {
        let m = invoice().macro_;
        let all: Vec<String> =
            group_steps(&m.events, (&m.recording).into()).iter().map(|s| describe(&s.kind)).collect();
        // The cursor moving to each click is a MOVE step before it.
        let moves = all.iter().enumerate().filter(|(_, s)| *s == "MOVE").map(|(i, _)| i).collect::<Vec<_>>();
        assert_eq!(moves, [0, 2, 5, 9, 13, 15]);
        let steps: Vec<_> = all.iter().filter(|s| *s != "MOVE").collect();
        assert_eq!(
            steps,
            [
                "CLICK Left×1 File menu",
                "CLICK Left×1 Export as PDF",
                "WAIT 700 Dialog opens",
                "CLICK Left×2 Filename field",
                "KEYS Ctrl + A",
                "TYPE invoice_0924",
                "CLICK Left×1 Save",
                "IF Save button turns grey",
                "KEYS Ctrl + W",
                "CLICK Right×1 Invoices folder",
                "CLICK Left×1 Upload to portal",
                "KEYS Enter",
            ]
        );
    }

    /// Writes the fixture the UI uses when it runs in a plain browser
    /// (`npm run dev`), like ts-rs does for the bindings.
    #[test]
    fn export_browser_fixture() {
        let items: Vec<_> = all()
            .iter()
            .map(|s| {
                json!({
                    "view": crate::view::MacroView::of(&s.macro_),
                    "runs": s.runs,
                    "last_run_ago_ms": s.last_run_ago.map(|d| d.num_milliseconds()),
                    "hotkey": s.hotkey,
                })
            })
            .collect();
        let body = serde_json::to_string_pretty(&items).unwrap() + "\n";
        write_fixture("sample-views.json", body);
        let runs: Vec<_> = runs()
            .into_iter()
            .map(|(ago, entry)| json!({ "ago_ms": ago.num_milliseconds(), "entry": entry }))
            .collect();
        write_fixture(
            "sample-runs.json",
            serde_json::to_string_pretty(&runs).unwrap()
                + "
",
        );
    }

    fn write_fixture(name: &str, body: String) {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src/lib/dev").join(name);
        if std::fs::read_to_string(&path).ok().as_deref() != Some(body.as_str()) {
            std::fs::write(&path, body).unwrap();
        }
    }

    #[test]
    fn every_sample_is_valid_and_matches_the_design() {
        let counts: Vec<usize> = all()
            .iter()
            .map(|s| {
                check_invariants(&s.macro_.events).unwrap();
                let steps = group_steps(&s.macro_.events, (&s.macro_.recording).into());
                steps.iter().filter(|s| !matches!(s.kind, crate::steps::StepKind::Move { .. })).count()
            })
            .collect();
        // Timesheet: its "8", Tab, "8"… alternate, so every entry is its own step.
        assert_eq!(counts, [12, 13, 8, 7], "besides the moves");
    }
}
