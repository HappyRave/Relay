//! Undo and redo for macro edits: step edits, pauses, labels and renames.
//! Kept in memory per macro for this run of Relay; playback options and
//! triggers are settings, not edits, and aren't part of it.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use relay_core::{EditOp, Event, Macro};
use uuid::Uuid;

/// Edits kept per macro.
const LIMIT: usize = 100;
/// Renames closer together than this are one undo step (the name is saved as
/// it's typed).
const RENAME_BURST: Duration = Duration::from_secs(2);

#[derive(Clone)]
struct Snapshot {
    name: String,
    events: Vec<Event>,
}

impl Snapshot {
    fn of(m: &Macro) -> Self {
        Snapshot { name: m.name.clone(), events: m.events.clone() }
    }

    /// Puts this snapshot into `m` and returns what `m` was.
    fn swap_into(self, m: &mut Macro) -> Snapshot {
        let was = Snapshot::of(m);
        m.name = self.name;
        m.events = self.events;
        m.modified_at = chrono::Utc::now();
        was
    }
}

#[derive(Default)]
struct History {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    last_rename: Option<Instant>,
}

#[derive(Default)]
pub struct EditHistory(Mutex<HashMap<Uuid, History>>);

impl EditHistory {
    /// Records `before`, the macro as it was before `op` was applied to it.
    pub fn record(&self, id: Uuid, before: &Macro, op: &EditOp) {
        let mut all = self.0.lock().unwrap();
        let h = all.entry(id).or_default();
        let now = Instant::now();
        let renaming = matches!(op, EditOp::Rename { .. });
        let same_rename = renaming && h.last_rename.is_some_and(|t| now - t < RENAME_BURST);
        if !same_rename {
            h.undo.push(Snapshot::of(before));
            if h.undo.len() > LIMIT {
                h.undo.remove(0);
            }
        }
        h.last_rename = renaming.then_some(now);
        h.redo.clear();
    }

    /// Reverts `m` to before its last edit. False if there's nothing to undo.
    pub fn undo(&self, id: Uuid, m: &mut Macro) -> bool {
        self.step(id, m, true)
    }

    /// Re-applies the last undone edit. False if there's nothing to redo.
    pub fn redo(&self, id: Uuid, m: &mut Macro) -> bool {
        self.step(id, m, false)
    }

    fn step(&self, id: Uuid, m: &mut Macro, undo: bool) -> bool {
        let mut all = self.0.lock().unwrap();
        let Some(h) = all.get_mut(&id) else { return false };
        let (from, to) = if undo { (&mut h.undo, &mut h.redo) } else { (&mut h.redo, &mut h.undo) };
        let Some(snapshot) = from.pop() else { return false };
        to.push(snapshot.swap_into(m));
        h.last_rename = None;
        true
    }

    /// (can undo, can redo)
    pub fn status(&self, id: Uuid) -> (bool, bool) {
        self.0.lock().unwrap().get(&id).map_or((false, false), |h| (!h.undo.is_empty(), !h.redo.is_empty()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use relay_core::edit::apply;
    use relay_core::model::RecordingMeta;

    fn wait(t: u32) -> Event {
        Event::Wait { t, dur: 100, label: String::new() }
    }

    #[test]
    fn undo_and_redo_edits() {
        let h = EditHistory::default();
        let mut m = Macro::new("m", RecordingMeta::single_1080p(), vec![wait(0), wait(500)]);
        let id = m.id;
        let original = m.events.clone();
        assert_eq!(h.status(id), (false, false));

        let op = EditOp::DeleteStep { index: 0 };
        let before = m.clone();
        apply(&mut m, op.clone()).unwrap();
        h.record(id, &before, &op);
        assert_eq!(h.status(id), (true, false));

        assert!(h.undo(id, &mut m));
        assert_eq!(m.events, original);
        assert_eq!(h.status(id), (false, true));
        assert!(!h.undo(id, &mut m), "nothing left to undo");

        assert!(h.redo(id, &mut m));
        assert_eq!(m.events.len(), 1);
        assert_eq!(h.status(id), (true, false));
    }

    #[test]
    fn a_new_edit_clears_redo_and_typing_a_name_is_one_step() {
        let h = EditHistory::default();
        let mut m = Macro::new("m", RecordingMeta::single_1080p(), vec![wait(0)]);
        let id = m.id;
        for name in ["R", "Re", "Rel"] {
            let op = EditOp::Rename { name: name.into() };
            let before = m.clone();
            apply(&mut m, op.clone()).unwrap();
            h.record(id, &before, &op);
        }
        assert!(h.undo(id, &mut m));
        assert_eq!(m.name, "m", "the three renames undo together");
        assert!(!h.undo(id, &mut m));

        assert!(h.redo(id, &mut m));
        let op = EditOp::SetLabel { index: 0, label: "x".into() };
        let before = m.clone();
        apply(&mut m, op.clone()).unwrap();
        h.record(id, &before, &op);
        assert_eq!(h.status(id), (true, false), "a new edit drops the redo stack");
    }
}
