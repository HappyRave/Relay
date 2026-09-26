//! The macro library, persisted under the data directory: one `.rly` per
//! macro plus `library.json` for the order and machine-local stats. A first
//! run seeds the design's sample macros. Deleting moves a macro's file to
//! `macros\.trash`, from where it can be restored.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use relay_core::steps::group_steps;
use relay_core::triggers::{HotkeyTrigger, MacroTriggers};
use relay_core::{Macro, MacroListItem, Ms, format, samples, timeline};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::storage::write_atomic;

pub struct Entry {
    pub macro_: Macro,
    pub runs: u32,
    pub last_run: Option<DateTime<Utc>>,
    pub triggers: MacroTriggers,
    /// The Library row's step count and length, recomputed when the macro is
    /// saved rather than on every listing.
    step_count: u32,
    duration: Ms,
}

impl Entry {
    fn new(macro_: Macro) -> Self {
        Entry::with_stats(macro_, 0, None, MacroTriggers::default())
    }

    fn with_stats(macro_: Macro, runs: u32, last_run: Option<DateTime<Utc>>, triggers: MacroTriggers) -> Self {
        let mut e = Entry { macro_, runs, last_run, triggers, step_count: 0, duration: 0 };
        e.summarize();
        e
    }

    fn summarize(&mut self) {
        let m = &self.macro_;
        self.step_count = group_steps(&m.events, (&m.recording).into()).len() as u32;
        self.duration = timeline::duration(&m.events);
    }

    /// The hotkey shown in the Library, when it's on.
    pub fn hotkey(&self) -> Option<String> {
        let h = &self.triggers.hotkey;
        (h.enabled && !h.combo.is_empty()).then(|| h.combo.clone())
    }

    pub fn list_item(&self) -> MacroListItem {
        MacroListItem {
            id: self.macro_.id,
            name: self.macro_.name.clone(),
            duration: self.duration,
            step_count: self.step_count,
            runs: self.runs,
            last_run: self.last_run,
            hotkey: self.hotkey(),
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Index {
    version: u32,
    order: Vec<Uuid>,
    entries: HashMap<Uuid, IndexEntry>,
    /// Deleted macros: their stats and where they were, for restoring.
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
    /// Libraries from before triggers (Relay 0.3–0.7) stored only a hotkey
    /// label; it never did anything, so it migrates as a disabled trigger.
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
    /// Every macro's triggers, rebuilt when they change, so the trigger
    /// threads can poll without copying them each time.
    triggers: Arc<[(Uuid, MacroTriggers)]>,
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
        let mut problems = Vec::new();
        let index_path = dir.join("library.json");
        let index: Option<Index> = match fs::read_to_string(&index_path) {
            Ok(text) => match serde_json::from_str(&text) {
                Ok(index) => Some(index),
                Err(e) => {
                    // Keep it for inspection instead of overwriting it on the next save.
                    let bad = index_path.with_extension("json.bad");
                    let _ = fs::rename(&index_path, &bad);
                    problems.push(format!("{}: {e} (moved to {})", index_path.display(), bad.display()));
                    None
                }
            },
            Err(_) => None,
        };
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

        let mut lib = Library { dir: dir.to_path_buf(), entries: Vec::new(), trash: HashMap::new(), triggers: Arc::new([]) };
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
                Entry::with_stats(
                    m,
                    meta.map_or(0, |e| e.runs),
                    meta.and_then(|e| e.last_run),
                    meta.map(IndexEntry::triggers).unwrap_or_default(),
                )
            })
            .collect();
        lib.refresh_triggers();
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
                let last_run = s.last_run(now);
                Entry::with_stats(s.macro_, s.runs, last_run, triggers)
            })
            .collect();
        for e in &self.entries {
            let _ = self.write_macro(&e.macro_);
        }
        let _ = self.save_index();
        self.refresh_triggers();
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

    /// Adds a new macro at the top of the library and saves it. The macro
    /// stays in the library even if writing it fails, so it isn't lost.
    pub fn insert_front(&mut self, macro_: Macro) -> std::io::Result<()> {
        let id = macro_.id;
        self.entries.insert(0, Entry::new(macro_));
        self.refresh_triggers();
        self.save(id)
    }

    /// Writes one macro's file after a change to it, and the index.
    pub fn save(&mut self, id: Uuid) -> std::io::Result<()> {
        let Some(e) = self.get_mut(id) else { return Ok(()) };
        e.summarize();
        let m = &self.get(id).expect("just found").macro_;
        self.write_macro(m)?;
        self.save_index()
    }

    /// Writes the index only (run counts and last runs changed).
    pub fn save_stats(&self) -> std::io::Result<()> {
        self.save_index()
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
        self.refresh_triggers();
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
        self.refresh_triggers();
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
        self.entries.insert(pos, Entry::with_stats(m, t.meta.runs, t.meta.last_run, t.meta.triggers()));
        self.refresh_triggers();
        Ok(self.save_index()?)
    }

    /// Adds imported macros at the top, in order, saving the index once. A
    /// macro that's already in the library is imported as a copy with a new
    /// id. Returns the ids used.
    pub fn import(&mut self, macros: Vec<Macro>) -> Result<Vec<Uuid>, LibraryError> {
        let mut ids = Vec::with_capacity(macros.len());
        for (i, mut m) in macros.into_iter().enumerate() {
            if self.get(m.id).is_some() || self.trash.contains_key(&m.id) {
                m.id = Uuid::new_v4();
            }
            m.name = self.unique_name(&m.name);
            self.write_macro(&m)?;
            ids.push(m.id);
            self.entries.insert(i, Entry::new(m));
        }
        self.refresh_triggers();
        self.save_index()?;
        Ok(ids)
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

    /// Every macro's triggers, for the trigger runtime (cheap to call).
    pub fn all_triggers(&self) -> Arc<[(Uuid, MacroTriggers)]> {
        self.triggers.clone()
    }

    pub fn set_triggers(&mut self, id: Uuid, triggers: MacroTriggers) -> Result<(), LibraryError> {
        let pos = self.position(id)?;
        self.entries[pos].triggers = triggers;
        self.refresh_triggers();
        Ok(self.save_index()?)
    }

    fn refresh_triggers(&mut self) {
        self.triggers = self.entries.iter().map(|e| (e.macro_.id, e.triggers.clone())).collect();
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
        let imported = lib.import(vec![relay_core::format::from_rly(&exported).unwrap()]).unwrap()[0];
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

        // A library.json from before triggers, with a bare hotkey label.
        let path = dir.path().join("library.json");
        let old = std::fs::read_to_string(&path).unwrap().replace(r#""triggers""#, r#""old_triggers""#);
        let old = old.replacen(r#""runs": 148,"#, r#""runs": 148, "hotkey": "Ctrl + Alt + 9","#, 1);
        std::fs::write(&path, old).unwrap();
        let (migrated, _) = Library::open(dir.path());
        let h = &migrated.get(invoice).unwrap().triggers.hotkey;
        assert_eq!((h.enabled, h.combo.as_str()), (false, "Ctrl + Alt + 9"));
    }

    #[test]
    fn a_broken_index_is_set_aside_and_reported() {
        let dir = tempfile::tempdir().unwrap();
        Library::open(dir.path());
        fs::write(dir.path().join("library.json"), "{oops").unwrap();
        let (lib, problems) = Library::open(dir.path());
        assert_eq!(lib.list().len(), 4, "the macros still load");
        assert_eq!(problems.len(), 1);
        assert!(dir.path().join("library.json.bad").exists());
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
