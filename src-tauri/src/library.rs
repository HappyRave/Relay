//! The macro library, persisted under the data directory: one `.rly` per
//! macro plus `library.json` for the order and machine-local stats. A first
//! run seeds the design's sample macros.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use relay_core::{Macro, MacroListItem, format, samples};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::storage::write_atomic;

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

#[derive(Default, Serialize, Deserialize)]
struct Index {
    version: u32,
    order: Vec<Uuid>,
    entries: HashMap<Uuid, IndexEntry>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct IndexEntry {
    runs: u32,
    last_run: Option<DateTime<Utc>>,
    hotkey: Option<String>,
}

pub struct Library {
    dir: PathBuf,
    entries: Vec<Entry>,
}

impl Library {
    /// Loads the library in `dir`, seeding the samples on first run. Files
    /// that fail to parse are skipped and reported, never deleted.
    pub fn open(dir: &Path) -> (Self, Vec<String>) {
        let macros_dir = dir.join("macros");
        let index: Option<Index> =
            fs::read_to_string(dir.join("library.json")).ok().and_then(|s| serde_json::from_str(&s).ok());
        let mut problems = Vec::new();
        let mut loaded: Vec<Macro> = Vec::new();
        if let Ok(files) = fs::read_dir(&macros_dir) {
            for f in files.flatten() {
                let path = f.path();
                if path.extension().is_some_and(|e| e == "rly") {
                    match fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|s| format::from_rly(&s).map_err(|e| e.to_string())) {
                        Ok(m) => loaded.push(m),
                        Err(e) => problems.push(format!("{}: {e}", path.display())),
                    }
                }
            }
        }

        let mut lib = Library { dir: dir.to_path_buf(), entries: Vec::new() };
        if index.is_none() && loaded.is_empty() {
            lib.seed_samples();
            return (lib, problems);
        }

        let index = index.unwrap_or_default();
        let rank = |id: &Uuid| index.order.iter().position(|o| o == id).unwrap_or(usize::MAX);
        // Indexed macros in their saved order, then any unindexed files newest first.
        loaded.sort_by(|a, b| rank(&a.id).cmp(&rank(&b.id)).then(b.modified_at.cmp(&a.modified_at)));
        lib.entries = loaded
            .into_iter()
            .map(|m| {
                let meta = index.entries.get(&m.id);
                Entry {
                    runs: meta.map_or(0, |e| e.runs),
                    last_run: meta.and_then(|e| e.last_run),
                    hotkey: meta.and_then(|e| e.hotkey.clone()),
                    macro_: m,
                }
            })
            .collect();
        (lib, problems)
    }

    fn seed_samples(&mut self) {
        let now = Utc::now();
        self.entries = samples::all()
            .into_iter()
            .map(|s| Entry { last_run: s.last_run(now), runs: s.runs, hotkey: s.hotkey, macro_: s.macro_ })
            .collect();
        for e in &self.entries {
            let _ = self.write_macro(&e.macro_);
        }
        let _ = self.save_index();
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

    /// Adds a new macro at the top of the library and saves it.
    pub fn insert_front(&mut self, macro_: Macro) -> std::io::Result<()> {
        self.write_macro(&macro_)?;
        self.entries.insert(0, Entry { macro_, runs: 0, last_run: None, hotkey: None });
        self.save_index()
    }

    /// Writes one macro's file (after an edit).
    pub fn save(&self, id: Uuid) -> std::io::Result<()> {
        match self.get(id) {
            Some(e) => self.write_macro(&e.macro_).and_then(|_| self.save_index()),
            None => Ok(()),
        }
    }

    /// The next free "Recording N" name.
    pub fn next_recording_name(&self) -> String {
        let n = self
            .entries
            .iter()
            .filter_map(|e| e.macro_.name.strip_prefix("Recording ")?.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        format!("Recording {}", n + 1)
    }

    fn write_macro(&self, m: &Macro) -> std::io::Result<()> {
        write_atomic(&self.dir.join("macros").join(format!("{}.rly", m.id)), &format::to_rly(m))
    }

    fn save_index(&self) -> std::io::Result<()> {
        let index = Index {
            version: 1,
            order: self.entries.iter().map(|e| e.macro_.id).collect(),
            entries: self
                .entries
                .iter()
                .map(|e| (e.macro_.id, IndexEntry { runs: e.runs, last_run: e.last_run, hotkey: e.hotkey.clone() }))
                .collect(),
        };
        write_atomic(&self.dir.join("library.json"), &serde_json::to_string_pretty(&index).expect("index serializes"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use relay_core::EditOp;
    use relay_core::model::RecordingMeta;

    #[test]
    fn first_run_seeds_samples_then_persists_changes() {
        let dir = tempfile::tempdir().unwrap();
        let (mut lib, problems) = Library::open(dir.path());
        assert!(problems.is_empty());
        assert_eq!(lib.list().len(), 4);
        assert_eq!(fs::read_dir(dir.path().join("macros")).unwrap().count(), 4);

        let rec = Macro::new(lib.next_recording_name(), RecordingMeta::single_1080p(), vec![]);
        let rec_id = rec.id;
        lib.insert_front(rec).unwrap();
        let invoice = lib.list()[1].id;
        relay_core::edit::apply(&mut lib.get_mut(invoice).unwrap().macro_, EditOp::Rename { name: "Renamed".into() }).unwrap();
        lib.save(invoice).unwrap();

        let (again, _) = Library::open(dir.path());
        let names: Vec<_> = again.list().into_iter().map(|i| i.name).collect();
        assert_eq!(names[..2], ["Recording 1".to_string(), "Renamed".to_string()]);
        assert_eq!(again.list()[0].id, rec_id);
        assert_eq!(again.get(invoice).unwrap().runs, 148);
        assert_eq!(again.next_recording_name(), "Recording 2");
    }

    #[test]
    fn broken_files_are_reported_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        Library::open(dir.path());
        fs::write(dir.path().join("macros").join("broken.rly"), "{not json").unwrap();
        let (lib, problems) = Library::open(dir.path());
        assert_eq!(lib.list().len(), 4);
        assert_eq!(problems.len(), 1);
    }
}
