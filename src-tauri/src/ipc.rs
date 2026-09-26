//! The session stream from Rust to the UI. The UI subscribes once with a
//! Tauri `Channel`; everything session-related arrives on it in order.

use parking_lot::Mutex;

use relay_core::Step;
use relay_core::model::Rect;
use relay_core::session::{FinishReason, Mode};
use relay_core::view::MovePoint;
use serde::Serialize;
use tauri::ipc::Channel;
use ts_rs::TS;
use uuid::Uuid;

use crate::engine::TimingStats;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum EngineMsg {
    /// The session mode changed; `macro_id` is the macro the session is about.
    Session {
        mode: Mode,
        macro_id: Option<Uuid>,
    },
    Countdown {
        left_ms: u32,
    },
    /// About 10 times a second while recording: new cursor samples since the
    /// last message and, when they changed, all steps so far.
    RecProgress {
        elapsed_ms: u32,
        desktop: Rect,
        moves: Vec<MovePoint>,
        steps: Option<Vec<Step>>,
    },
    /// About 30 times a second while playing; the UI extrapolates between ticks.
    PlayTick {
        t: f64,
        advancing: bool,
        speed: f64,
        loop_idx: u32,
        loops: Option<u32>,
    },
    Finished {
        reason: FinishReason,
        timing: Option<TimingStats>,
    },
    /// A recording was saved as this macro.
    Saved {
        id: Uuid,
    },
    LibraryChanged,
    /// Ctrl + Shift + M.
    ToggleCompact,
    Error {
        message: String,
    },
    /// Something worth knowing that isn't a failure (e.g. an elevated target).
    Notice {
        message: String,
    },
    /// Triggers were paused (by the kill switch) or resumed.
    TriggersPaused {
        paused: bool,
    },
}

#[derive(Default)]
pub struct Emitter(Mutex<Option<Channel<EngineMsg>>>);

impl Emitter {
    /// Replaces the subscriber (a reloaded UI subscribes again).
    pub fn subscribe(&self, channel: Channel<EngineMsg>) {
        *self.0.lock() = Some(channel);
    }

    pub fn send(&self, msg: EngineMsg) {
        if let Some(c) = self.0.lock().as_ref() {
            let _ = c.send(msg);
        }
    }

    pub fn error(&self, message: impl Into<String>) {
        let message = message.into();
        tracing::warn!("{message}");
        self.send(EngineMsg::Error { message });
    }
}
