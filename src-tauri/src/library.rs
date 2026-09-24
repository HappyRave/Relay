//! The macro library, persisted under the data directory: one `.rly` per
//! macro plus `library.json` for the order and machine-local stats. A first
//! run seeds the design's sample macros. Deleting moves a macro's file to
//! `macros\.trash`, from where it can be restored.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use relay_core::triggers::{HotkeyTrigger, MacroTriggers};
use relay_core::{Macro, MacroListItem, format, samples};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::storage::write_atomic;

pub struct Entry {
    pub macro_: Macro,
    pub runs: u32,
    pub last_run: Option<DateTime<Utc>>,
    pub triggers: MacroTriggers,
}

impl Entry {
    fn new(macro_: Macro) -> Self {
        Entry { macro_, runs: 0, last_run: None, triggers: MacroTriggers::default() }
    }

    /// The hotkey shown in the Library, when it's on.
    pub fn hotkey(&self) -> Option<String> {
        let h = &self.triggers.hotkey;
        (h.enabled && !h.combo.is_empty()).then(|| h.combo.clone())
    }

    pub fn list_item(&self) -> MacroListItem {
        MacroListItem::of(&self.macro_, self.runs, self.last_run, self.hotkey())
    }
}

#[derive(Default, Serialize, Deserialize)]
struct Index {
    version: u32,
    order: Vec<Uuid>,
    entries: HashMap<Uuid, IndexEntry>,
    /// Deleted macros: their stats and where they were, for restoring.
    #[serde(default)]
    trash: HashMap<Uuid, TrashEntry>,
}

#[derive(Clone, Serialize, Deserialize)]
struct TrashEntry {
    position: usize,
    #[serde(flatten)]
    meta: IndexEntry,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct IndexEntry {
    runs: u32,
    last_run: Option<DateTime<Utc>>,
    triggers: MacroTriggers,
    /// Before triggers existed (M2–M6) only a hotkey label was stored; it
    /// never did anything, so it migrates as a disabled hotkey trigger.
    #[serde(skip_serializing)]
    hotkey: Option<String>,
}

impl IndexEntry {
    fn of(e: &Entry) -> Self {
        IndexEntry { runs: e.runs, last_run: e.last_run, triggers: e.triggers.clone(), hotkey: None }
    }

    fn triggers(&self) -> MacroTriggers {
        let mut t = self.triggers.clone();
        if let Some(combo) = &self.hotkey
            && t.hotkey.combo.is_empty()
        {
            t.hotkey = HotkeyTrigger { enabled: false, combo: combo.clone() };
        }
        t
    }
}

pub struct Library {
    dir: PathBuf,
    entries: Vec<Entry>,
    trash: HashMap<Uuid, TrashEntry>,
}

#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error("no macro with id {0}")]
    NotFound(Uuid),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Format(#[from] relay_core::format::FormatError),
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

        let mut lib = Library { dir: dir.to_path_buf(), entries: Vec::new(), trash: HashMap::new() };
        if index.is_none() && loaded.is_empty() {
            lib.seed_samples();
            return (lib, problems);
        }

        let index = index.unwrap_or_default();
        lib.trash = index.trash.clone();
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
                    triggers: meta.map(IndexEntry::triggers).unwrap_or_default(),
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
            .map(|s| {
                let mut triggers = MacroTriggers::default();
                // The design's hotkeys are shown but left off: they'd replay samples on your desktop.
                if let Some(combo) = s.hotkey.clone() {
                    triggers.hotkey = HotkeyTrigger { enabled: false, combo };
                }
                Entry { last_run: s.last_run(now), runs: s.runs, triggers, macro_: s.macro_ }
            })
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
        self.entries.insert(0, Entry::new(macro_));
        self.save_index()
    }

    /// Writes one macro's file (after an edit).
    pub fn save(&self, id: Uuid) -> std::io::Result<()> {
        match self.get(id) {
            Some(e) => self.write_macro(&e.macro_).and_then(|_| self.save_index()),
            None => Ok(()),
        }
    }

    /// Copies a macro under a new id, right after the original. Returns the copy's id.
    pub fn duplicate(&mut self, id: Uuid) -> Result<Uuid, LibraryError> {
        let pos = self.position(id)?;
        let mut copy = self.entries[pos].macro_.clone();
        copy.id = Uuid::new_v4();
        copy.name = self.unique_name(&format!("{} (copy)", copy.name));
        copy.created_at = Utc::now();
        copy.modified_at = copy.created_at;
        self.write_macro(&copy)?;
        let new_id = copy.id;
        // A copy starts with no triggers: two macros on one hotkey or schedule would collide.
        self.entries.insert(pos + 1, Entry::new(copy));
        self.save_index()?;
        Ok(new_id)
    }

    /// Moves a macro to the trash (its file to `macros\.trash`).
    pub fn trash(&mut self, id: Uuid) -> Result<(), LibraryError> {
        let position = self.position(id)?;
        let trash_dir = self.dir.join("macros").join(".trash");
        fs::create_dir_all(&trash_dir)?;
        fs::rename(self.macro_path(id), trash_dir.join(format!("{id}.rly")))?;
        let e = self.entries.remove(position);
        self.trash.insert(id, TrashEntry { position, meta: IndexEntry::of(&e) });
        Ok(self.save_index()?)
    }

    /// Brings a trashed macro back where it was, with its stats.
    pub fn restore(&mut self, id: Uuid) -> Result<(), LibraryError> {
        let t = self.trash.get(&id).cloned().ok_or(LibraryError::NotFound(id))?;
        let from = self.dir.join("macros").join(".trash").join(format!("{id}.rly"));
        let m = format::from_rly(&fs::read_to_string(&from)?)?;
        fs::rename(&from, self.macro_path(id))?;
        self.trash.remove(&id);
        let pos = t.position.min(self.entries.len());
        self.entries.insert(pos, Entry { macro_: m, runs: t.meta.runs, last_run: t.meta.last_run, triggers: t.meta.triggers() });
        Ok(self.save_index()?)
    }

    /// Adds an imported macro at the top. A macro that's already in the
    /// library is imported as a copy with a new id. Returns the id used.
    pub fn import(&mut self, mut m: Macro) -> Result<Uuid, LibraryError> {
        if self.get(m.id).is_some() || self.trash.contains_key(&m.id) {
            m.id = Uuid::new_v4();
        }
        m.name = self.unique_name(&m.name);
        let id = m.id;
        self.insert_front(m)?;
        Ok(id)
    }

    fn position(&self, id: Uuid) -> Result<usize, LibraryError> {
        self.entries.iter().position(|e| e.macro_.id == id).ok_or(LibraryError::NotFound(id))
    }

    fn macro_path(&self, id: Uuid) -> PathBuf {
        self.dir.join("macros").join(format!("{id}.rly"))
    }

    /// `name`, or `name 2`, `name 3`… if a macro already has it.
    fn unique_name(&self, name: &str) -> String {
        let taken = |n: &str| self.entries.iter().any(|e| e.macro_.name == n);
        if !taken(name) {
            return name.to_string();
        }
        (2..).map(|i| format!("{name} {i}")).find(|n| !taken(n)).expect("a free name")
    }

    /// Every macro's triggers, for the trigger runtime.
    pub fn all_triggers(&self) -> Vec<(Uuid, MacroTriggers)> {
        self.entries.iter().map(|e| (e.macro_.id, e.triggers.clone())).collect()
    }

    pub fn set_triggers(&mut self, id: Uuid, triggers: MacroTriggers) -> Result<(), LibraryError> {
        let pos = self.position(id)?;
        self.entries[pos].triggers = triggers;
        Ok(self.save_index()?)
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
        write_atomic(&self.macro_path(m.id), &format::to_rly(m))
    }

    fn save_index(&self) -> std::io::Result<()> {
        let index = Index {
            version: 1,
            order: self.entries.iter().map(|e| e.macro_.id).collect(),
            entries: self
                .entries
                .iter()
                .map(|e| (e.macro_.id, IndexEntry::of(e)))
                .collect(),
            trash: self.trash.clone(),
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
    fn duplicate_trash_restore_and_import() {
        let dir = tempfile::tempdir().unwrap();
        let (mut lib, _) = Library::open(dir.path());
        let invoice = lib.list()[0].id;

        let copy = lib.duplicate(invoice).unwrap();
        let names: Vec<_> = lib.list().into_iter().map(|i| i.name).collect();
        assert_eq!(names[..2], ["Export invoice to PDF".to_string(), "Export invoice to PDF (copy)".to_string()]);
        assert_eq!(lib.get(copy).unwrap().runs, 0);
        assert_eq!(lib.get(copy).unwrap().hotkey(), None);
        assert_eq!(lib.get(copy).unwrap().macro_.events, lib.get(invoice).unwrap().macro_.events);
        lib.duplicate(invoice).unwrap();
        assert_eq!(lib.list()[1].name, "Export invoice to PDF (copy) 2");

        // Trash keeps the file and the stats; restore puts it back in place.
        lib.trash(invoice).unwrap();
        assert!(lib.get(invoice).is_none());
        assert!(dir.path().join("macros/.trash").join(format!("{invoice}.rly")).exists());
        let (reopened, problems) = Library::open(dir.path());
        assert!(problems.is_empty());
        assert!(reopened.get(invoice).is_none(), "trashed macros stay out after a restart");
        let (mut lib, _) = (reopened, ());
        lib.restore(invoice).unwrap();
        assert_eq!(lib.list()[0].id, invoice);
        assert_eq!(lib.get(invoice).unwrap().runs, 148);
        assert!(matches!(lib.restore(invoice), Err(LibraryError::NotFound(_))));

        // Importing a macro that's already here makes a copy with a new id.
        let exported = relay_core::format::to_rly(&lib.get(invoice).unwrap().macro_);
        let imported = lib.import(relay_core::format::from_rly(&exported).unwrap()).unwrap();
        assert_ne!(imported, invoice);
        assert_eq!(lib.list()[0].id, imported);
        assert_eq!(lib.list()[0].name, "Export invoice to PDF 2");
        assert_eq!(lib.get(imported).unwrap().macro_.events, lib.get(invoice).unwrap().macro_.events);
    }

    #[test]
    fn triggers_persist_and_old_hotkeys_migrate_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let (mut lib, _) = Library::open(dir.path());
        let invoice = lib.list()[0].id;
        // Seeded samples show the design's hotkey but leave it off.
        let t = lib.get(invoice).unwrap().triggers.clone();
        assert_eq!(t.hotkey, HotkeyTrigger { enabled: false, combo: "Ctrl + Alt + 1".into() });
        assert_eq!(lib.list()[0].hotkey, None);

        let mut on = t.clone();
        on.hotkey.enabled = true;
        on.app_launch.enabled = true;
        on.app_launch.exe = "notepad.exe".into();
        lib.set_triggers(invoice, on.clone()).unwrap();
        let (again, _) = Library::open(dir.path());
        assert_eq!(again.get(invoice).unwrap().triggers, on);
        assert_eq!(again.list()[0].hotkey.as_deref(), Some("Ctrl + Alt + 1"));

        // An M2–M6 library.json with a bare hotkey label.
        let path = dir.path().join("library.json");
        let old = std::fs::read_to_string(&path).unwrap().replace(r#""triggers""#, r#""old_triggers""#);
        let old = old.replacen(r#""runs": 148,"#, r#""runs": 148, "hotkey": "Ctrl + Alt + 9","#, 1);
        std::fs::write(&path, old).unwrap();
        let (migrated, _) = Library::open(dir.path());
        let h = &migrated.get(invoice).unwrap().triggers.hotkey;
        assert_eq!((h.enabled, h.combo.as_str()), (false, "Ctrl + Alt + 9"));
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
