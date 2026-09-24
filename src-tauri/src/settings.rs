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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct Settings {
    pub capture_moves: bool,
    pub capture_keys: bool,
    /// 3-second countdown before recording.
    pub countdown: bool,
    /// Ignore input injected by other programs (remote-desktop tools inject
    /// real user input, so their users may want this off).
    pub ignore_injected: bool,
    pub path_mode: PathMode,
    pub show_click_labels: bool,
    /// The close button (and Alt+F4) hides Relay to the tray instead of quitting.
    pub close_to_tray: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            capture_moves: true,
            capture_keys: true,
            countdown: true,
            ignore_injected: true,
            path_mode: PathMode::Full,
            show_click_labels: true,
            close_to_tray: true,
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
        let current = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
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
    }
}
