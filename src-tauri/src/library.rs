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
    /// Its index in the Library when it was trashed. Older libraries only
    /// have this; it's the fallback when `next` is missing or gone.
    #[serde(default)]
    position: usize,
    /// The macro that came right after it, trashed ones included (`null`:
    /// it was last), so it goes back in its place whatever changed since.
    /// Missing in older libraries.
    #[serde(default, deserialize_with = "present", skip_serializing_if = "Option::is_none")]
    next: Option<Option<Uuid>>,
    #[serde(flatten)]
    meta: IndexEntry,
}

/// A field that's there, even as `null` (`Some(None)`); a missing one is `None`.
fn present<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
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
    /// Why `library.json` is left alone this run: it exists but couldn't be
    /// read (another program had it open, say), so writing it would lose
    /// the order, stats, triggers and trash it holds.
    index_unreadable: Option<String>,
    /// Trashed macros whose file couldn't be written, restorable until Relay quits.
    unsaved_trash: HashMap<Uuid, Macro>,
}

/// A change that was made. If saving it failed, it's kept anyway (in memory,
/// until Relay quits) and `saved` says why: see the `commands` module docs.
pub struct Change<T = ()> {
    pub value: T,
    pub saved: std::io::Result<()>,
}

impl Change {
    fn saved(saved: std::io::Result<()>) -> Self {
        Change { value: (), saved }
    }
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
    /// that fail to parse are skipped and reported, never deleted. A
    /// `library.json` that can't be read at all isn't written this run.
    pub fn open(dir: &Path) -> (Self, Vec<String>) {
        let macros_dir = dir.join("macros");
        let mut problems = Vec::new();
        let index_path = dir.join("library.json");
        let mut index_unreadable = None;
        let index: Option<Index> = match fs::read_to_string(&index_path) {
            Ok(text) => match serde_json::from_str(&text) {
                Ok(index) => Some(index),
                Err(e) => {
                    // Keep it for inspection instead of overwriting it on the next save.
                    let bad = index_path.with_extension("json.bad");
                    let _ = fs::rename(&index_path, &bad);
                    problems.push(format!("Couldn't read {}: {e} (moved to {})", index_path.display(), bad.display()));
                    None
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => {
                let why = format!("{} couldn't be read when Relay started ({e})", index_path.display());
                problems.push(format!(
                    "{why}. The Library's order, run counts, triggers and trash are missing, and won't be saved \
                     until you restart Relay."
                ));
                index_unreadable = Some(format!("{why}, so Relay leaves it alone until restarted"));
                None
            }
        };
        let mut loaded: Vec<Macro> = Vec::new();
        if let Ok(files) = fs::read_dir(&macros_dir) {
            for f in files.flatten() {
                let path = f.path();
                if path.extension().is_some_and(|e| e == "rly") {
                    match fs::read_to_string(&path)
                        .map_err(|e| e.to_string())
                        .and_then(|s| format::from_rly(&s).map_err(|e| e.to_string()))
                    {
                        Ok(m) => loaded.push(m),
                        Err(e) => problems.push(format!("{}: {e}", path.display())),
                    }
                }
            }
        }

        let first_run = index.is_none() && index_unreadable.is_none() && loaded.is_empty();
        let mut lib = Library {
            dir: dir.to_path_buf(),
            entries: Vec::new(),
            trash: HashMap::new(),
            triggers: Arc::new([]),
            index_unreadable,
            unsaved_trash: HashMap::new(),
        };
        if first_run {
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
    pub fn duplicate(&mut self, id: Uuid) -> Result<Change<Uuid>, LibraryError> {
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
        Ok(Change { value: new_id, saved: self.save_index() })
    }

    /// Moves a macro to the trash (its file to `macros\.trash`). A macro
    /// whose file couldn't be saved is written there from memory, or if that
    /// fails too, kept in memory to restore until Relay quits.
    pub fn trash(&mut self, id: Uuid) -> Result<Change, LibraryError> {
        let position = self.position(id)?;
        let (from, to) = (self.macro_path(id), self.trash_path(id));
        let moved = to.parent().map_or(Ok(()), fs::create_dir_all).and_then(|()| fs::rename(&from, &to));
        let file = match moved {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && !from.exists() => {
                write_atomic(&to, &format::to_rly(&self.entries[position].macro_))
            }
            Err(e) => return Err(e.into()),
        };
        let next = Some(self.next_after(position));
        let e = self.entries.remove(position);
        if file.is_err() {
            self.unsaved_trash.insert(id, e.macro_.clone());
        }
        self.trash.insert(id, TrashEntry { position, next, meta: IndexEntry::of(&e) });
        self.refresh_triggers();
        let index = self.save_index();
        Ok(Change::saved(file.and(index)))
    }

    /// Brings a trashed macro back where it was, with its stats and triggers.
    /// If another macro has taken its hotkey meanwhile, it comes back with
    /// the hotkey off, and the value says so, for the user.
    pub fn restore(&mut self, id: Uuid) -> Result<Change<Option<String>>, LibraryError> {
        let t = self.trash.get(&id).cloned().ok_or(LibraryError::NotFound(id))?;
        let (m, file) = match self.unsaved_trash.remove(&id) {
            // Its file couldn't be written; try again.
            Some(m) => {
                let file = self.write_macro(&m);
                (m, file)
            }
            None => {
                let from = self.trash_path(id);
                let m = format::from_rly(&fs::read_to_string(&from)?)?;
                fs::rename(&from, self.macro_path(id))?;
                (m, Ok(()))
            }
        };
        self.trash.remove(&id);
        let mut triggers = t.meta.triggers();
        let hotkey = &mut triggers.hotkey;
        let mut notice = None;
        if hotkey.enabled
            && let Some(why) = crate::hotkeys::conflict(self, id, &hotkey.combo)
        {
            hotkey.enabled = false;
            notice = Some(match self.hotkey_owner(&hotkey.combo) {
                Some(other) => {
                    format!("“{}” is back; its hotkey {} is now used by “{other}”, so it's off", m.name, hotkey.combo)
                }
                None => format!("“{}” is back with its hotkey off: {why}", m.name),
            });
        }
        let pos = self.place_of(t.next).unwrap_or(t.position.min(self.entries.len()));
        self.entries.insert(pos, Entry::with_stats(m, t.meta.runs, t.meta.last_run, triggers));
        self.refresh_triggers();
        let index = self.save_index();
        Ok(Change { value: notice, saved: file.and(index) })
    }

    /// The name of the macro whose hotkey (on) is `combo`, however it's written.
    fn hotkey_owner(&self, combo: &str) -> Option<&str> {
        let wanted = crate::hotkeys::parse_combo(combo).ok()?;
        self.entries
            .iter()
            .find(|e| e.hotkey().is_some_and(|c| crate::hotkeys::parse_combo(&c).is_ok_and(|s| s == wanted)))
            .map(|e| e.macro_.name.as_str())
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

    /// Where a trashed macro whose `next` this is goes back: before the first
    /// macro in the Library along the chain of trashed ones after it, or at
    /// the end. `None` if the chain is missing or broken.
    fn place_of(&self, mut next: Option<Option<Uuid>>) -> Option<usize> {
        for _ in 0..=self.trash.len() {
            let Some(id) = next? else { return Some(self.entries.len()) };
            if let Ok(pos) = self.position(id) {
                return Some(pos);
            }
            next = self.trash.get(&id)?.next;
        }
        None // a loop, in a hand-edited file
    }

    /// What comes right after the macro at `pos`, trashed macros included:
    /// the first of those that go back before the macro after it, if any, else that macro.
    fn next_after(&self, pos: usize) -> Option<Uuid> {
        let mut before_next: Vec<Uuid> =
            self.trash.iter().filter(|(_, t)| self.place_of(t.next) == Some(pos + 1)).map(|(id, _)| *id).collect();
        before_next.sort(); // for a stable choice should the file be inconsistent
        let first = before_next.iter().find(|&&id| !before_next.iter().any(|o| self.trash[o].next == Some(Some(id))));
        first.copied().or_else(|| self.entries.get(pos + 1).map(|e| e.macro_.id))
    }

    fn position(&self, id: Uuid) -> Result<usize, LibraryError> {
        self.entries.iter().position(|e| e.macro_.id == id).ok_or(LibraryError::NotFound(id))
    }

    fn macro_path(&self, id: Uuid) -> PathBuf {
        self.dir.join("macros").join(format!("{id}.rly"))
    }

    fn trash_path(&self, id: Uuid) -> PathBuf {
        self.dir.join("macros").join(".trash").join(format!("{id}.rly"))
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

    pub fn set_triggers(&mut self, id: Uuid, triggers: MacroTriggers) -> Result<Change, LibraryError> {
        let pos = self.position(id)?;
        self.entries[pos].triggers = triggers;
        self.refresh_triggers();
        Ok(Change::saved(self.save_index()))
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
        if let Some(why) = &self.index_unreadable {
            return Err(std::io::Error::other(why.clone()));
        }
        let index = Index {
            version: 1,
            order: self.entries.iter().map(|e| e.macro_.id).collect(),
            entries: self.entries.iter().map(|e| (e.macro_.id, IndexEntry::of(e))).collect(),
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
        relay_core::edit::apply(&mut lib.get_mut(invoice).unwrap().macro_, EditOp::Rename { name: "Renamed".into() })
            .unwrap();
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

        let copy = lib.duplicate(invoice).unwrap().value;
        let names: Vec<_> = lib.list().into_iter().map(|i| i.name).collect();
        assert_eq!(names[..2], ["Export invoice to PDF".to_string(), "Export invoice to PDF (copy)".to_string()]);
        assert_eq!(lib.get(copy).unwrap().runs, 0);
        assert_eq!(lib.get(copy).unwrap().hotkey(), None);
        assert_eq!(lib.get(copy).unwrap().macro_.events, lib.get(invoice).unwrap().macro_.events);
        lib.duplicate(invoice).unwrap().saved.unwrap();
        assert_eq!(lib.list()[1].name, "Export invoice to PDF (copy) 2");

        // Trash keeps the file and the stats; restore puts it back in place.
        lib.trash(invoice).unwrap().saved.unwrap();
        assert!(lib.get(invoice).is_none());
        assert!(dir.path().join("macros/.trash").join(format!("{invoice}.rly")).exists());
        let (reopened, problems) = Library::open(dir.path());
        assert!(problems.is_empty());
        assert!(reopened.get(invoice).is_none(), "trashed macros stay out after a restart");
        let (mut lib, _) = (reopened, ());
        lib.restore(invoice).unwrap().saved.unwrap();
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
        lib.set_triggers(invoice, on.clone()).unwrap().saved.unwrap();
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
    fn changes_are_kept_when_the_index_cant_be_saved() {
        let dir = tempfile::tempdir().unwrap();
        let (mut lib, _) = Library::open(dir.path());
        let invoice = lib.list()[0].id;
        // library.json can't be replaced now.
        let index = dir.path().join("library.json");
        fs::remove_file(&index).unwrap();
        fs::create_dir(&index).unwrap();

        let mut on = lib.get(invoice).unwrap().triggers.clone();
        on.hotkey.enabled = true;
        let changed = lib.set_triggers(invoice, on.clone()).unwrap();
        assert!(changed.saved.is_err());
        assert_eq!(lib.get(invoice).unwrap().triggers, on);
        assert_eq!(lib.all_triggers().iter().find(|(id, _)| *id == invoice).unwrap().1, on, "and live");

        let copy = lib.duplicate(invoice).unwrap();
        assert!(copy.saved.is_err());
        assert!(lib.get(copy.value).is_some());
        assert!(lib.trash(copy.value).unwrap().saved.is_err());
        assert!(lib.get(copy.value).is_none());
        assert!(lib.restore(copy.value).unwrap().saved.is_err());
        assert_eq!(lib.list()[1].id, copy.value);
    }

    fn ids(lib: &Library) -> Vec<Uuid> {
        lib.list().into_iter().map(|i| i.id).collect()
    }

    #[test]
    fn restoring_puts_macros_back_between_the_same_neighbours() {
        for (trash_order, restore_order) in [([0, 1], [0, 1]), ([0, 1], [1, 0]), ([1, 0], [0, 1]), ([1, 0], [1, 0])] {
            let dir = tempfile::tempdir().unwrap();
            let (mut lib, _) = Library::open(dir.path());
            let [a, b, c, d] = ids(&lib)[..] else { panic!("four samples") };
            for i in trash_order {
                lib.trash([a, b][i]).unwrap().saved.unwrap();
            }
            // New macros at the top, and a restart in between.
            let new = Macro::new("New", RecordingMeta::single_1080p(), vec![]);
            let n = new.id;
            lib.insert_front(new).unwrap();
            let (mut lib, _) = Library::open(dir.path());
            for i in restore_order {
                lib.restore([a, b][i]).unwrap().saved.unwrap();
            }
            assert_eq!(ids(&lib), [n, a, b, c, d], "trashed {trash_order:?}, restored {restore_order:?}");
        }
    }

    fn set_hotkey(lib: &mut Library, id: Uuid, enabled: bool, combo: &str) {
        let mut t = lib.get(id).unwrap().triggers.clone();
        t.hotkey = HotkeyTrigger { enabled, combo: combo.into() };
        lib.set_triggers(id, t).unwrap().saved.unwrap();
    }

    #[test]
    fn a_macro_whose_hotkey_was_taken_meanwhile_comes_back_with_it_off() {
        let dir = tempfile::tempdir().unwrap();
        let (mut lib, _) = Library::open(dir.path());
        let [invoice, timesheet, ..] = ids(&lib)[..] else { panic!("four samples") };
        set_hotkey(&mut lib, invoice, true, "Ctrl + Alt + 1");
        lib.trash(invoice).unwrap().saved.unwrap();
        set_hotkey(&mut lib, timesheet, true, "Alt+Ctrl+1");

        let restored = lib.restore(invoice).unwrap();
        assert_eq!(
            restored.value.as_deref(),
            Some(
                "“Export invoice to PDF” is back; its hotkey Ctrl + Alt + 1 is now used by “Fill weekly timesheet”, so it's off"
            )
        );
        let h = &lib.get(invoice).unwrap().triggers.hotkey;
        assert_eq!((h.enabled, h.combo.as_str()), (false, "Ctrl + Alt + 1"), "the combo is kept");
        assert_eq!(crate::hotkeys::conflict(&lib, timesheet, "Alt+Ctrl+1"), None);

        // Its hotkey free again: it comes back on, with nothing to say.
        set_hotkey(&mut lib, invoice, true, "Ctrl + Alt + 1");
        lib.trash(invoice).unwrap().saved.unwrap();
        set_hotkey(&mut lib, timesheet, false, "Alt+Ctrl+1");
        assert_eq!(lib.restore(invoice).unwrap().value, None);
        assert!(lib.get(invoice).unwrap().triggers.hotkey.enabled);
    }

    #[test]
    fn the_last_macro_goes_back_last() {
        let dir = tempfile::tempdir().unwrap();
        let (mut lib, _) = Library::open(dir.path());
        let [a, b, c, d] = ids(&lib)[..] else { panic!("four samples") };
        lib.trash(d).unwrap().saved.unwrap();
        lib.trash(b).unwrap().saved.unwrap();
        lib.trash(c).unwrap().saved.unwrap();
        lib.import(vec![Macro::new("New 1", RecordingMeta::single_1080p(), vec![])]).unwrap();
        lib.import(vec![Macro::new("New 2", RecordingMeta::single_1080p(), vec![])]).unwrap();
        assert!(fs::read_to_string(dir.path().join("library.json")).unwrap().contains(r#""next": null"#));
        let (mut lib, _) = Library::open(dir.path());
        for id in [d, b, c] {
            lib.restore(id).unwrap().saved.unwrap();
        }
        assert_eq!(ids(&lib)[2..], [a, b, c, d]);
    }

    #[test]
    fn trash_from_older_libraries_restores_at_its_position() {
        let dir = tempfile::tempdir().unwrap();
        let (mut lib, _) = Library::open(dir.path());
        let [a, b, c, d] = ids(&lib)[..] else { panic!("four samples") };
        lib.trash(b).unwrap().saved.unwrap();
        let path = dir.path().join("library.json");
        let index = fs::read_to_string(&path).unwrap();
        assert!(index.contains(&format!(r#""next": "{c}""#)), "{index}");
        fs::write(&path, index.replace(&format!(r#""next": "{c}","#), "")).unwrap();
        let (mut lib, problems) = Library::open(dir.path());
        assert!(problems.is_empty());
        lib.restore(b).unwrap().saved.unwrap();
        assert_eq!(ids(&lib), [a, b, c, d]);
    }

    /// A new recording whose file couldn't be written (it's kept until you quit).
    fn insert_unsaved(dir: &Path, lib: &mut Library) -> Uuid {
        let rec = Macro::new(lib.next_recording_name(), RecordingMeta::single_1080p(), vec![]);
        let (id, path) = (rec.id, dir.join("macros").join(format!("{}.rly", rec.id)));
        fs::create_dir(&path).unwrap();
        assert!(lib.insert_front(rec).is_err());
        fs::remove_dir(&path).unwrap();
        id
    }

    #[test]
    fn a_macro_whose_file_was_never_saved_can_be_trashed_and_restored() {
        let dir = tempfile::tempdir().unwrap();
        let (mut lib, _) = Library::open(dir.path());
        let id = insert_unsaved(dir.path(), &mut lib);
        lib.trash(id).unwrap().saved.unwrap();
        assert!(lib.get(id).is_none());
        let trashed = dir.path().join("macros/.trash").join(format!("{id}.rly"));
        assert_eq!(format::from_rly(&fs::read_to_string(&trashed).unwrap()).unwrap().name, "Recording 1");
        lib.restore(id).unwrap().saved.unwrap();
        assert_eq!(lib.list()[0].id, id);
        assert!(dir.path().join("macros").join(format!("{id}.rly")).exists(), "saved at last");
    }

    #[test]
    fn a_macro_that_cant_be_written_to_the_trash_is_restored_from_memory() {
        let dir = tempfile::tempdir().unwrap();
        let (mut lib, _) = Library::open(dir.path());
        let id = insert_unsaved(dir.path(), &mut lib);
        fs::create_dir_all(dir.path().join("macros/.trash").join(format!("{id}.rly"))).unwrap();
        let trashed = lib.trash(id).unwrap();
        assert!(trashed.saved.is_err(), "reported");
        assert!(lib.get(id).is_none());
        lib.restore(id).unwrap().saved.unwrap();
        assert_eq!(lib.get(id).unwrap().macro_.name, "Recording 1");
        assert!(dir.path().join("macros").join(format!("{id}.rly")).exists());
    }

    /// Changes are kept but library.json isn't written: `index` is what it held.
    fn assert_index_left_alone(dir: &Path, lib: &mut Library, index: &str) {
        let id = lib.list()[0].id;
        let err = lib.save(id).unwrap_err().to_string();
        assert!(err.contains("library.json couldn't be read when Relay started"), "{err}");
        assert!(err.contains("leaves it alone until restarted"), "{err}");
        lib.get_mut(id).unwrap().runs += 1;
        assert!(lib.save_stats().is_err());
        assert_eq!(lib.get(id).unwrap().runs, 1, "kept in memory");
        assert_eq!(fs::read_to_string(dir.join("library.json")).unwrap(), index);
    }

    #[cfg(windows)]
    #[test]
    fn an_index_another_program_has_open_is_left_alone_for_the_run() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = tempfile::tempdir().unwrap();
        Library::open(dir.path());
        let path = dir.path().join("library.json");
        let index = fs::read_to_string(&path).unwrap();
        // Opened without sharing: reading it fails with a sharing violation.
        let locked = fs::OpenOptions::new().read(true).share_mode(0).open(&path).unwrap();
        let (mut lib, problems) = Library::open(dir.path());
        drop(locked);
        assert_eq!(lib.list().len(), 4, "the macros still load");
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("won't be saved until you restart Relay"), "{}", problems[0]);
        assert!(!dir.path().join("library.json.bad").exists(), "it's fine, just unreadable for now");
        assert_eq!(lib.get(lib.list()[0].id).unwrap().runs, 0, "its stats weren't read");
        assert_index_left_alone(dir.path(), &mut lib, &index);
        // Readable again at the next start, with everything in it.
        let (again, problems) = Library::open(dir.path());
        assert!(problems.is_empty());
        assert_eq!(again.get(again.list()[0].id).unwrap().runs, 148);
    }

    #[test]
    fn an_index_that_cant_be_read_is_left_alone_and_nothing_is_seeded() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("library.json")).unwrap();
        let (lib, problems) = Library::open(dir.path());
        assert_eq!(problems.len(), 1);
        assert!(lib.list().is_empty(), "not a first run: the samples aren't seeded");
        assert!(!dir.path().join("macros").exists());
        assert!(dir.path().join("library.json").is_dir());
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
