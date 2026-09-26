//! Global settings, persisted to settings.json. Unknown or missing fields
//! fall back to defaults, so older files keep loading. A value Relay doesn't
//! understand (a damaged file, or one from a newer Relay) resets only that
//! setting; the file is then set aside as `settings.json.bad` and reported.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::storage::write_atomic;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum PathMode {
    Full,
    Trail,
}

/// When the widget floats above other windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum KeepOnTop {
    Always,
    /// Only while recording or playing (the countdown included).
    Sessions,
    Never,
}

impl KeepOnTop {
    /// Whether the widget should be on top, given whether a session is running.
    pub fn on_top(self, session: bool) -> bool {
        match self {
            KeepOnTop::Always => true,
            KeepOnTop::Sessions => session,
            KeepOnTop::Never => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct Settings {
    pub capture_moves: bool,
    pub capture_keys: bool,
    /// 3-second countdown before recording.
    pub countdown: bool,
    /// Esc stops a recording (and is left out of it). Off, Esc is recorded
    /// like any key and F9 stops. Esc always stops playback.
    pub esc_stops_recording: bool,
    /// Ignore input injected by other programs (remote-desktop tools inject
    /// real user input, so their users may want this off).
    pub ignore_injected: bool,
    pub path_mode: PathMode,
    pub show_click_labels: bool,
    /// The close button (and Alt+F4) hides Relay to the tray instead of quitting.
    pub close_to_tray: bool,
    pub keep_on_top: KeepOnTop,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            capture_moves: true,
            capture_keys: true,
            countdown: true,
            esc_stops_recording: true,
            ignore_injected: true,
            path_mode: PathMode::Full,
            show_click_labels: true,
            close_to_tray: true,
            keep_on_top: KeepOnTop::Always,
        }
    }
}

pub struct SettingsStore {
    path: PathBuf,
    pub current: Settings,
}

impl SettingsStore {
    /// Loads the settings in `dir`, with anything wrong with the file.
    pub fn open(dir: &Path) -> (Self, Vec<String>) {
        let path = dir.join("settings.json");
        let mut problems = Vec::new();
        let current = match std::fs::read_to_string(&path) {
            Ok(text) => {
                let (current, why) = parse(&text);
                if let Some(why) = why {
                    // Keep the original for inspection, and save what could be read from it.
                    let bad = path.with_extension("json.bad");
                    let _ = std::fs::rename(&path, &bad);
                    let _ = write_atomic(&path, &to_json(&current));
                    problems.push(format!("{} {why} (the file was moved to {})", path.display(), bad.display()));
                }
                current
            }
            Err(_) => Settings::default(),
        };
        (SettingsStore { path, current }, problems)
    }

    pub fn set(&mut self, settings: Settings) -> std::io::Result<()> {
        self.current = settings;
        write_atomic(&self.path, &to_json(&self.current))
    }
}

fn to_json(s: &Settings) -> String {
    serde_json::to_string_pretty(s).expect("settings serialize")
}

/// Reads settings.json field by field: a value that doesn't fit its setting
/// takes the default and the others are kept. Returns what was wrong, if anything.
fn parse(text: &str) -> (Settings, Option<String>) {
    use serde_json::{Map, Value};
    let reset = |why: String| (Settings::default(), Some(format!("{why}; every setting is back to its default")));
    let file: Map<String, Value> = match serde_json::from_str(text) {
        Ok(Value::Object(file)) => file,
        Ok(_) => return reset("isn't a settings file".into()),
        Err(e) => return reset(format!("couldn't be read ({e})")),
    };
    let Ok(Value::Object(mut merged)) = serde_json::to_value(Settings::default()) else {
        unreachable!("settings serialize to an object")
    };
    let mut bad = Vec::new();
    for (key, value) in file {
        // Unknown fields are ignored, as serde does.
        let Some(default) = merged.insert(key.clone(), value) else {
            merged.remove(&key);
            continue;
        };
        if serde_json::from_value::<Settings>(Value::Object(merged.clone())).is_err() {
            merged.insert(key.clone(), default);
            bad.push(format!("“{key}”"));
        }
    }
    let settings = serde_json::from_value(Value::Object(merged)).expect("defaults are valid");
    let why = (!bad.is_empty()).then(|| {
        let (what, verb) = if bad.len() == 1 { ("setting", "is") } else { ("settings", "are") };
        format!("had values Relay doesn't understand; the {what} {} {verb} back to the default", bad.join(", "))
    });
    (settings, why)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open(dir: &Path) -> (Settings, Vec<String>) {
        let (s, problems) = SettingsStore::open(dir);
        (s.current, problems)
    }

    #[test]
    fn persists_and_tolerates_partial_files() {
        let dir = tempfile::tempdir().unwrap();
        let (mut s, problems) = SettingsStore::open(dir.path());
        assert_eq!(s.current, Settings::default());
        assert!(problems.is_empty());
        s.set(Settings { countdown: false, ..Settings::default() }).unwrap();
        assert!(!open(dir.path()).0.countdown);

        std::fs::write(dir.path().join("settings.json"), r#"{"capture_keys":false,"from_a_newer_relay":1}"#).unwrap();
        let (s, problems) = open(dir.path());
        assert!(!s.capture_keys && s.capture_moves);
        assert_eq!(s.keep_on_top, KeepOnTop::Always, "older files keep the old behavior");
        assert!(problems.is_empty(), "unknown fields are fine: {problems:?}");
        assert!(!dir.path().join("settings.json.bad").exists());
    }

    #[test]
    fn a_bad_value_resets_only_its_setting_and_the_file_is_set_aside() {
        let dir = tempfile::tempdir().unwrap();
        let text = r#"{"countdown":false,"keep_on_top":"sometimes","path_mode":"trail","capture_keys":"no"}"#;
        std::fs::write(dir.path().join("settings.json"), text).unwrap();
        let (s, problems) = open(dir.path());
        let kept = Settings { countdown: false, path_mode: PathMode::Trail, ..Settings::default() };
        assert_eq!(s, kept, "the valid ones are kept");
        assert_eq!(problems.len(), 1);
        let p = &problems[0];
        assert!(p.contains("“keep_on_top”") && p.contains("“capture_keys”") && p.contains("are back to"), "{p}");
        assert!(!p.contains("countdown"), "{p}");
        assert_eq!(std::fs::read_to_string(dir.path().join("settings.json.bad")).unwrap(), text);
        // What could be read is saved, so the next start has it, without a problem.
        assert_eq!(open(dir.path()), (kept, vec![]));
    }

    #[test]
    fn an_unreadable_file_resets_everything_and_is_set_aside() {
        let dir = tempfile::tempdir().unwrap();
        for text in ["{ not json", "[1, 2]"] {
            std::fs::write(dir.path().join("settings.json"), text).unwrap();
            let (s, problems) = open(dir.path());
            assert_eq!(s, Settings::default());
            assert_eq!(problems.len(), 1);
            assert!(problems[0].contains("every setting is back to its default"), "{}", problems[0]);
            assert_eq!(std::fs::read_to_string(dir.path().join("settings.json.bad")).unwrap(), text);
        }
    }

    #[test]
    fn keep_on_top_follows_sessions_when_asked() {
        assert!(KeepOnTop::Always.on_top(false) && KeepOnTop::Always.on_top(true));
        assert!(!KeepOnTop::Sessions.on_top(false) && KeepOnTop::Sessions.on_top(true));
        assert!(!KeepOnTop::Never.on_top(false) && !KeepOnTop::Never.on_top(true));
    }
}
