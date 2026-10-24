//! The session stream from Rust to the UI. Each of Relay's windows (the main
//! one and the player) subscribes once with a Tauri `Channel`; everything
//! session-related arrives on each in order.

use std::collections::VecDeque;

use parking_lot::Mutex;

use relay_core::Step;
use relay_core::model::Rect;
use relay_core::session::{FinishReason, Mode, RunSource};
use relay_core::view::MovePoint;
use serde::Serialize;
use tauri::ipc::Channel;
use ts_rs::TS;
use uuid::Uuid;

use relay_playback::TimingStats;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum EngineMsg {
    /// The session mode changed; `macro_id` is the macro the session is
    /// about, and `source` what started its playback.
    Session {
        mode: Mode,
        macro_id: Option<Uuid>,
        source: Option<RunSource>,
    },
    /// The macro Relay's play key and the player play changed: `by` the
    /// window that opened it, or by Relay (a recording, the end of a run of
    /// another macro).
    Current {
        macro_id: Option<Uuid>,
        by: Option<String>,
    },
    /// The main window was asked to show this macro (from the player).
    Open {
        macro_id: Uuid,
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
    /// An entry was added to the run history.
    RunsChanged,
    /// The player took the main window's place, or gave it back.
    PlayerMode {
        on: bool,
    },
    Error {
        message: String,
    },
    /// A change couldn't be written to disk: it's kept until Relay quits.
    Unsaved {
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
    /// Windows' transparency effects were turned on or off.
    SystemLook {
        look: crate::look::SystemLook,
    },
    /// The settings were saved (from either window).
    Settings {
        settings: crate::settings::Settings,
    },
}

/// How many errors and notices are kept for a UI that hasn't subscribed yet.
const EARLY_LIMIT: usize = 20;

/// The window that gets what was sent before any window subscribed.
const MAIN: &str = "main";

#[derive(Default)]
pub struct Emitter(Mutex<Subscribers>);

#[derive(Default)]
struct Subscribers {
    /// Each window's channel, by its label.
    channels: Vec<(String, Channel<EngineMsg>)>,
    /// Whether the main window has subscribed.
    main: bool,
    /// Errors and notices sent before the main window first subscribed (e.g.
    /// a hotkey that couldn't be registered at startup), the latest [`EARLY_LIMIT`].
    early: VecDeque<EngineMsg>,
}

impl Emitter {
    /// Replaces `window`'s subscriber (a reloaded UI subscribes again). The
    /// main window's first one gets the errors and notices sent before it, in order.
    pub fn subscribe(&self, window: &str, channel: Channel<EngineMsg>) {
        let mut s = self.0.lock();
        if window == MAIN && !s.main {
            s.main = true;
            for msg in s.early.drain(..) {
                let _ = channel.send(msg);
            }
        }
        s.channels.retain(|(w, _)| w != window);
        s.channels.push((window.to_owned(), channel));
    }

    /// Sends `msg` to every window. Before the main window subscribes, errors
    /// and notices wait for it; the rest (session state, ticks) would be stale by then.
    pub fn send(&self, msg: EngineMsg) {
        let mut s = self.0.lock();
        if !s.main && matches!(msg, EngineMsg::Error { .. } | EngineMsg::Unsaved { .. } | EngineMsg::Notice { .. }) {
            if s.early.len() == EARLY_LIMIT {
                s.early.pop_front();
            }
            s.early.push_back(msg.clone());
        }
        for (_, c) in &s.channels {
            let _ = c.send(msg.clone());
        }
    }

    pub fn error(&self, message: impl Into<String>) {
        let message = message.into();
        tracing::warn!("{message}");
        self.send(EngineMsg::Error { message });
    }

    /// A change that couldn't be written (see [`EngineMsg::Unsaved`]).
    pub fn unsaved(&self, message: impl Into<String>) {
        let message = message.into();
        tracing::warn!("{message}");
        self.send(EngineMsg::Unsaved { message });
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
    fn errors_and_notices_wait_for_the_first_subscriber() {
        let e = Emitter::default();
        e.error("Couldn't register the Record hotkey");
        e.send(EngineMsg::LibraryChanged);
        e.send(EngineMsg::Session { mode: Mode::Idle, macro_id: None, source: None });
        e.send(EngineMsg::Notice { message: "Triggers are paused".into() });
        e.send(EngineMsg::PlayTick { t: 0.0, advancing: false, speed: 1.0, loop_idx: 0, loops: Some(1) });
        let (c, got) = channel();
        e.subscribe("main", c);
        assert_eq!(
            *got.lock(),
            [
                serde_json::json!({"type": "error", "message": "Couldn't register the Record hotkey"}),
                serde_json::json!({"type": "notice", "message": "Triggers are paused"}),
            ],
            "in order, without the session and tick messages"
        );
        e.send(EngineMsg::LibraryChanged);
        assert_eq!(got.lock().len(), 3, "then everything is sent");

        // A reloaded UI doesn't get them again.
        let (again, again_got) = channel();
        e.subscribe("main", again);
        assert!(again_got.lock().is_empty());
    }

    #[test]
    fn only_the_latest_early_messages_are_kept() {
        let e = Emitter::default();
        for i in 0..EARLY_LIMIT + 5 {
            e.error(format!("error {i}"));
        }
        let (c, got) = channel();
        e.subscribe("main", c);
        let got = got.lock();
        assert_eq!(got.len(), EARLY_LIMIT);
        assert_eq!(got[0], serde_json::json!({"type": "error", "message": "error 5"}));
        assert_eq!(got[EARLY_LIMIT - 1], serde_json::json!({"type": "error", "message": "error 24"}));
    }

    #[test]
    fn messages_are_tagged_by_type() {
        let e = Emitter::default();
        let (c, got) = channel();
        e.subscribe("main", c);
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
        e.subscribe("main", old);
        e.send(EngineMsg::PlayerMode { on: true });
        e.subscribe("main", new);
        e.send(EngineMsg::LibraryChanged);
        assert_eq!(*old_got.lock(), [serde_json::json!({"type": "player_mode", "on": true})]);
        assert_eq!(*new_got.lock(), [serde_json::json!({"type": "library_changed"})]);
    }

    #[test]
    fn every_window_gets_every_message() {
        let e = Emitter::default();
        e.error("Couldn't register the Record hotkey");
        let (player, player_got) = channel();
        e.subscribe("player", player);
        e.send(EngineMsg::Current { macro_id: None, by: None });
        let (main, main_got) = channel();
        e.subscribe("main", main);
        e.send(EngineMsg::LibraryChanged);
        assert_eq!(
            *player_got.lock(),
            [
                serde_json::json!({"type": "current", "macro_id": null, "by": null}),
                serde_json::json!({"type": "library_changed"}),
            ],
            "the player gets nothing from before it subscribed"
        );
        assert_eq!(
            *main_got.lock(),
            [
                serde_json::json!({"type": "error", "message": "Couldn't register the Record hotkey"}),
                serde_json::json!({"type": "library_changed"}),
            ],
            "the main window gets the early errors, and what was sent since"
        );
    }
}
