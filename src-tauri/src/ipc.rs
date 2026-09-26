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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tauri::ipc::InvokeResponseBody;

    fn channel() -> (Channel<EngineMsg>, Arc<Mutex<Vec<serde_json::Value>>>) {
        let got = Arc::new(Mutex::new(Vec::new()));
        let sink = got.clone();
        let c = Channel::new(move |body| {
            let InvokeResponseBody::Json(s) = body else { panic!("expected JSON") };
            sink.lock().push(serde_json::from_str(&s).unwrap());
            Ok(())
        });
        (c, got)
    }

    #[test]
    fn nothing_is_sent_before_the_ui_subscribes() {
        let e = Emitter::default();
        e.send(EngineMsg::LibraryChanged);
        e.error("lost");
    }

    #[test]
    fn messages_are_tagged_by_type() {
        let e = Emitter::default();
        let (c, got) = channel();
        e.subscribe(c);
        e.send(EngineMsg::Countdown { left_ms: 2000 });
        e.send(EngineMsg::TriggersPaused { paused: true });
        e.error("Couldn't register the Play hotkey");
        assert_eq!(
            *got.lock(),
            [
                serde_json::json!({"type": "countdown", "left_ms": 2000}),
                serde_json::json!({"type": "triggers_paused", "paused": true}),
                serde_json::json!({"type": "error", "message": "Couldn't register the Play hotkey"}),
            ]
        );
    }

    #[test]
    fn a_reloaded_ui_replaces_the_old_subscriber() {
        let e = Emitter::default();
        let (old, old_got) = channel();
        let (new, new_got) = channel();
        e.subscribe(old);
        e.send(EngineMsg::ToggleCompact);
        e.subscribe(new);
        e.send(EngineMsg::LibraryChanged);
        assert_eq!(*old_got.lock(), [serde_json::json!({"type": "toggle_compact"})]);
        assert_eq!(*new_got.lock(), [serde_json::json!({"type": "library_changed"})]);
    }
}
