//! Runs the recorder on its own thread: drains raw hook input, reports
//! progress to the UI and hands the finished recording back on stop.

use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, bounded};
use relay_core::model::Rect;
use relay_core::steps::{GroupOptions, group_steps};
use relay_platform::recorder::{Recorder, Recording};
use relay_platform::{RawInput, RawKind};

use crate::coordinator::Cmd;
use crate::ipc::{EngineMsg, Emitter};

const POLL: Duration = Duration::from_millis(25);
const PROGRESS_EVERY_MS: f64 = 100.0;

pub struct RecThread {
    stop: Sender<()>,
    thread: JoinHandle<Recording>,
}

pub struct RecContext {
    pub desktop: Rect,
    pub group: GroupOptions,
    pub now_ms: fn() -> f64,
    pub emit: Arc<Emitter>,
    pub coordinator: Sender<Cmd>,
}

impl RecThread {
    pub fn spawn(recorder: Recorder, raw: Receiver<RawInput>, ctx: RecContext) -> Self {
        let (stop, stop_rx) = bounded(1);
        let thread = std::thread::Builder::new()
            .name("relay-recorder".into())
            .spawn(move || run(recorder, raw, stop_rx, ctx))
            .expect("spawn recorder thread");
        RecThread { stop, thread }
    }

    pub fn finish(self) -> Recording {
        let _ = self.stop.send(());
        self.thread.join().expect("recorder thread panicked")
    }
}

fn run(mut rec: Recorder, raw: Receiver<RawInput>, stop: Receiver<()>, ctx: RecContext) -> Recording {
    let mut last_progress = f64::MIN;
    let mut grouped_len = usize::MAX;
    loop {
        if stop.try_recv().is_ok() {
            // Keep whatever the hook delivered before it was removed.
            for r in raw.try_iter() {
                rec.push(r);
            }
            return rec.finish((ctx.now_ms)());
        }
        match raw.recv_timeout(POLL) {
            Ok(RawInput { kind: RawKind::Escape, .. }) => {
                let _ = ctx.coordinator.send(Cmd::Escape);
            }
            Ok(r) => rec.push(r),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return rec.finish((ctx.now_ms)()),
        }
        let now = (ctx.now_ms)();
        if now - last_progress >= PROGRESS_EVERY_MS {
            last_progress = now;
            let events = rec.events();
            // Re-group only when something other than the cursor path changed.
            let actions = events.iter().filter(|e| !matches!(e, relay_core::Event::Move { .. })).count();
            let steps = (actions != grouped_len).then(|| {
                grouped_len = actions;
                group_steps(events, ctx.group)
            });
            ctx.emit.send(EngineMsg::RecProgress {
                elapsed_ms: rec.elapsed(now),
                desktop: ctx.desktop,
                moves: rec.take_new_moves(),
                steps,
            });
        }
    }
}
