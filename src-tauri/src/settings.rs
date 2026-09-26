//! Global settings, persisted to settings.json. Unknown or missing fields
//! fall back to defaults, so older files keep loading.

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
    pub fn open(dir: &Path) -> Self {
        let path = dir.join("settings.json");
        let current =
            std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        SettingsStore { path, current }
    }

    pub fn set(&mut self, settings: Settings) -> std::io::Result<()> {
        self.current = settings;
        write_atomic(&self.path, &serde_json::to_string_pretty(&self.current).expect("settings serialize"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_and_tolerates_partial_files() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = SettingsStore::open(dir.path());
        assert_eq!(s.current, Settings::default());
        s.set(Settings { countdown: false, ..Settings::default() }).unwrap();
        assert!(!SettingsStore::open(dir.path()).current.countdown);

        std::fs::write(dir.path().join("settings.json"), r#"{"capture_keys":false}"#).unwrap();
        let s = SettingsStore::open(dir.path()).current;
        assert!(!s.capture_keys && s.capture_moves);
        assert_eq!(s.keep_on_top, KeepOnTop::Always, "older files keep the old behavior");
    }

    #[test]
    fn keep_on_top_follows_sessions_when_asked() {
        assert!(KeepOnTop::Always.on_top(false) && KeepOnTop::Always.on_top(true));
        assert!(!KeepOnTop::Sessions.on_top(false) && KeepOnTop::Sessions.on_top(true));
        assert!(!KeepOnTop::Never.on_top(false) && !KeepOnTop::Never.on_top(true));
    }
}
