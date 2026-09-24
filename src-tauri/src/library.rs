//! The macro library. In M1 it lives in memory, seeded with the design's
//! sample macros; M2/M5 persist it under %APPDATA%\Relay.

use chrono::{DateTime, Utc};
use relay_core::samples;
use relay_core::{Macro, MacroListItem};
use uuid::Uuid;

pub struct Entry {
    pub macro_: Macro,
    pub runs: u32,
    pub last_run: Option<DateTime<Utc>>,
    pub hotkey: Option<String>,
}

impl Entry {
    pub fn list_item(&self) -> MacroListItem {
        MacroListItem::of(&self.macro_, self.runs, self.last_run, self.hotkey.clone())
    }
}

#[derive(Default)]
pub struct Library {
    entries: Vec<Entry>,
}

impl Library {
    pub fn with_samples() -> Self {
        let now = Utc::now();
        let entries = samples::all()
            .into_iter()
            .map(|s| Entry { last_run: s.last_run(now), runs: s.runs, hotkey: s.hotkey, macro_: s.macro_ })
            .collect();
        Library { entries }
    }

    pub fn list(&self) -> Vec<MacroListItem> {
        self.entries.iter().map(Entry::list_item).collect()
    }

    pub fn get(&self, id: Uuid) -> Option<&Entry> {
        self.entries.iter().find(|e| e.macro_.id == id)
    }

    pub fn get_mut(&mut self, id: Uuid) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|e| e.macro_.id == id)
    }
}
