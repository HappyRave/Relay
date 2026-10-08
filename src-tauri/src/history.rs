//! Undo and redo for macro edits: step edits, pauses, labels and renames.
//! Kept in memory per macro for this run of Relay; playback options and
//! triggers are settings, not edits, and aren't part of it.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use relay_core::splice::Splice;
use relay_core::{EditOp, Event, Macro};
use uuid::Uuid;

/// Edits kept per macro.
const LIMIT: usize = 100;
/// Renames closer together than this are one undo step (the name is saved as
/// it's typed).
const RENAME_BURST: Duration = Duration::from_secs(2);

/// The macro as an edit is about to change it: the name, and for all but
/// renames, the events. Held only until the edit is recorded.
pub struct Snapshot {
    name: String,
    events: Option<Vec<Event>>,
}

impl Snapshot {
    /// The part of `m` that `op` is about to change.
    pub fn before(m: &Macro, op: &EditOp) -> Self {
        let events = (!matches!(op, EditOp::Rename { .. })).then(|| m.events.clone());
        Snapshot { name: m.name.clone(), events }
    }
}

/// What an edit changed, kept to undo it: the name it replaced, and what it
/// did to the events (see [`Splice`]).
struct Change {
    name: String,
    events: Option<Splice>,
}

impl Change {
    /// The change from `before` to `m`; `None` if the edit changed nothing.
    fn of(before: Snapshot, m: &Macro) -> Option<Change> {
        let events = before.events.map(|e| Splice::between(&e, &m.events)).filter(|s| !s.is_empty());
        (before.name != m.name || events.is_some()).then_some(Change { name: before.name, events })
    }

    /// Reverts this change in `m` and returns the change that re-applies it.
    fn revert(self, m: &mut Macro) -> Change {
        let name = std::mem::replace(&mut m.name, self.name);
        let events = self.events.map(|s| s.revert(&mut m.events));
        m.modified_at = chrono::Utc::now();
        Change { name, events }
    }
}

#[derive(Default)]
struct History {
    undo: Vec<Change>,
    redo: Vec<Change>,
    last_rename: Option<Instant>,
}

#[derive(Default)]
pub struct EditHistory(Mutex<HashMap<Uuid, History>>);

impl EditHistory {
    /// Records `before` (see [`Snapshot::before`]) once `op` has been applied
    /// to `m`. An edit that changed nothing (a label set to what it was) isn't one.
    pub fn record(&self, id: Uuid, before: Snapshot, op: &EditOp, m: &Macro) {
        let Some(change) = Change::of(before, m) else { return };
        let mut all = self.0.lock();
        let h = all.entry(id).or_default();
        let now = Instant::now();
        let renaming = matches!(op, EditOp::Rename { .. });
        let same_rename = renaming && h.last_rename.is_some_and(|t| now - t < RENAME_BURST);
        if !same_rename {
            h.undo.push(change);
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
        let mut all = self.0.lock();
        let Some(h) = all.get_mut(&id) else { return false };
        let (from, to) = if undo { (&mut h.undo, &mut h.redo) } else { (&mut h.redo, &mut h.undo) };
        let Some(change) = from.pop() else { return false };
        to.push(change.revert(m));
        h.last_rename = None;
        true
    }

    /// (can undo, can redo)
    pub fn status(&self, id: Uuid) -> (bool, bool) {
        self.0.lock().get(&id).map_or((false, false), |h| (!h.undo.is_empty(), !h.redo.is_empty()))
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
        let before = Snapshot::before(&m, &op);
        apply(&mut m, op.clone()).unwrap();
        h.record(id, before, &op, &m);
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
    fn an_edit_keeps_what_it_changed_not_a_copy_of_the_macro() {
        let h = EditHistory::default();
        let events: Vec<Event> =
            (0..5000).map(|i| Event::Wait { t: i * 100, dur: 50, label: format!("{i}") }).collect();
        let mut m = Macro::new("m", RecordingMeta::single_1080p(), events);
        let (id, original) = (m.id, m.events.clone());
        // Each moves every event after it.
        for op in [
            EditOp::InsertWait { at: 50, dur: 500, label: "new".into() },
            EditOp::DeleteStep { index: 2 },
            EditOp::SetLabel { index: 4000, label: "x".into() },
        ] {
            let before = Snapshot::before(&m, &op);
            apply(&mut m, op.clone()).unwrap();
            h.record(id, before, &op, &m);
        }
        let kept: Vec<usize> = h.0.lock()[&id].undo.iter().map(|c| c.events.as_ref().unwrap().kept()).collect();
        assert_eq!(kept, [0, 1, 1]);
        while h.undo(id, &mut m) {}
        assert_eq!(m.events, original);
    }

    #[test]
    fn a_new_edit_clears_redo_and_typing_a_name_is_one_step() {
        let h = EditHistory::default();
        let mut m = Macro::new("m", RecordingMeta::single_1080p(), vec![wait(0)]);
        let id = m.id;
        for name in ["R", "Re", "Rel"] {
            let op = EditOp::Rename { name: name.into() };
            let before = Snapshot::before(&m, &op);
            apply(&mut m, op.clone()).unwrap();
            h.record(id, before, &op, &m);
        }
        assert!(h.undo(id, &mut m));
        assert_eq!(m.name, "m", "the three renames undo together");
        assert!(!h.undo(id, &mut m));

        assert!(h.redo(id, &mut m));
        let op = EditOp::SetLabel { index: 0, label: "x".into() };
        let before = Snapshot::before(&m, &op);
        apply(&mut m, op.clone()).unwrap();
        h.record(id, before, &op, &m);
        assert_eq!(h.status(id), (true, false), "a new edit drops the redo stack");
    }

    #[test]
    fn an_edit_that_changes_nothing_isnt_undoable() {
        let h = EditHistory::default();
        let mut m = Macro::new("m", RecordingMeta::single_1080p(), vec![wait(0), wait(500)]);
        let id = m.id;
        let edit = |m: &mut Macro, op: EditOp| {
            let before = Snapshot::before(m, &op);
            apply(m, op.clone()).unwrap();
            h.record(id, before, &op, m);
        };
        edit(&mut m, EditOp::SetLabel { index: 1, label: "x".into() });
        assert!(h.undo(id, &mut m));
        assert_eq!(h.status(id), (false, true));

        // None of these change anything, so the redo is still there and nothing is undoable.
        let pause = relay_core::steps::group_steps(&m.events, (&m.recording).into())[1].pause;
        for op in [
            EditOp::Rename { name: "m".into() },
            EditOp::SetLabel { index: 1, label: String::new() },
            EditOp::SetPause { index: 1, dur: pause },
        ] {
            edit(&mut m, op.clone());
            assert_eq!(h.status(id), (false, true), "{op:?}");
        }
        edit(&mut m, EditOp::SetPause { index: 1, dur: pause + 100 });
        assert_eq!(h.status(id), (true, false));
    }
}
