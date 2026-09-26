//! What the UI receives: a macro with derived steps, the cursor path and its
//! duration, but without the raw event list.

use chrono::{DateTime, Utc};
use serde::Serialize;
use ts_rs::TS;
use uuid::Uuid;

use crate::model::{Event, Macro, Ms, PlaybackOptions, RecordingMeta};
use crate::steps::{Step, group_steps};
use crate::timeline;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, TS)]
#[ts(export)]
pub struct MovePoint {
    pub t: Ms,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct MacroView {
    pub id: Uuid,
    pub name: String,
    pub modified_at: DateTime<Utc>,
    pub recording: RecordingMeta,
    pub playback: PlaybackOptions,
    pub steps: Vec<Step>,
    /// Cursor positions over time (moves and presses), for the preview path.
    pub moves: Vec<MovePoint>,
    pub duration: Ms,
    /// Whether the app has edits of this macro to undo or redo (filled in by
    /// the app, which keeps the history; always false from [`MacroView::of`]).
    pub can_undo: bool,
    pub can_redo: bool,
}

impl MacroView {
    pub fn of(m: &Macro) -> Self {
        MacroView {
            id: m.id,
            name: m.name.clone(),
            modified_at: m.modified_at,
            recording: m.recording.clone(),
            playback: m.playback.clone(),
            steps: group_steps(&m.events, (&m.recording).into()),
            moves: cursor_path(&m.events),
            duration: timeline::duration(&m.events),
            can_undo: false,
            can_redo: false,
        }
    }
}

pub fn cursor_path(events: &[Event]) -> Vec<MovePoint> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Move { t, x, y } | Event::Button { t, x, y, .. } => Some(MovePoint { t: *t, x: *x, y: *y }),
            _ => None,
        })
        .collect()
}

/// A row of the Library tab.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct MacroListItem {
    pub id: Uuid,
    pub name: String,
    pub duration: Ms,
    pub step_count: u32,
    pub runs: u32,
    pub last_run: Option<DateTime<Utc>>,
    pub hotkey: Option<String>,
}
