//! The app's side of playback: the engine lives in `relay_playback`, and
//! reports here to the UI and the coordinator.

use std::sync::Arc;

use crossbeam_channel::Sender;
use relay_core::model::Ms;
use relay_core::session::FinishReason;
use relay_platform::Platform;
pub use relay_playback::{EngineCmd, EngineHandle, PlayPlan, RunReport, TimingStats};
use relay_playback::{PlaybackSink, Tick};

use crate::coordinator::Cmd;
use crate::ipc::{Emitter, EngineMsg};

struct AppSink {
    emit: Arc<Emitter>,
    coordinator: Sender<Cmd>,
    generation: u64,
}

impl PlaybackSink for AppSink {
    fn tick(&self, t: Tick) {
        self.emit.send(EngineMsg::PlayTick {
            t: t.t,
            advancing: t.advancing,
            speed: t.speed,
            loop_idx: t.loop_idx,
            loops: t.loops,
        });
    }

    fn notice(&self, message: String) {
        self.emit.send(EngineMsg::Notice { message });
    }

    fn done(&self, reason: FinishReason, timed_out_at: Option<Ms>) {
        let _ = self.coordinator.send(Cmd::EngineDone { generation: self.generation, reason, timed_out_at });
    }
}

/// Runs `plan` on a new engine thread. `generation` identifies this playback
/// in the [`Cmd::EngineDone`] it sends when it ends on its own.
pub fn spawn(
    plan: PlayPlan,
    platform: &Platform,
    emit: Arc<Emitter>,
    coordinator: Sender<Cmd>,
    generation: u64,
) -> EngineHandle {
    relay_playback::spawn(plan, platform, AppSink { emit, coordinator, generation })
}
