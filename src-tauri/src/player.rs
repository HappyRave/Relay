//! Playback timing on its own thread. In M2 it only drives the playhead; M3
//! adds input injection on the same clock.

use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use relay_core::playback::PlaySession;
use relay_core::session::{FinishReason, Input};

use crate::coordinator::Cmd;
use crate::ipc::{EngineMsg, Emitter};

const TICK: Duration = Duration::from_millis(33);

pub enum PlayerCmd {
    Pause,
    Resume,
    Seek(f64),
    Speed(f64),
    Stop,
}

pub struct Player {
    tx: Sender<PlayerCmd>,
    thread: Option<JoinHandle<()>>,
}

impl Player {
    pub fn spawn(session: PlaySession, now_ms: fn() -> f64, emit: Arc<Emitter>, done: Sender<Cmd>) -> Self {
        let (tx, rx) = unbounded();
        let thread = std::thread::Builder::new()
            .name("relay-player".into())
            .spawn(move || run(session, now_ms, rx, &emit, &done))
            .expect("spawn player thread");
        Player { tx, thread: Some(thread) }
    }

    pub fn send(&self, cmd: PlayerCmd) {
        let _ = self.tx.send(cmd);
    }

    pub fn stop(mut self) {
        self.send(PlayerCmd::Stop);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn run(mut s: PlaySession, now_ms: fn() -> f64, rx: Receiver<PlayerCmd>, emit: &Emitter, done: &Sender<Cmd>) {
    loop {
        match rx.recv_timeout(TICK) {
            Ok(cmd) => {
                let now = now_ms();
                match cmd {
                    PlayerCmd::Pause => s.clock.pause(now),
                    PlayerCmd::Resume => s.clock.resume(now),
                    PlayerCmd::Seek(t) => s.clock.seek(t.clamp(0.0, s.duration as f64), now),
                    PlayerCmd::Speed(v) => s.clock.set_speed(v, now),
                    PlayerCmd::Stop => return,
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        let tick = s.tick(now_ms());
        emit.send(EngineMsg::PlayTick {
            t: tick.t,
            advancing: !s.clock.paused() && !tick.finished,
            speed: s.clock.speed(),
            loop_idx: tick.loop_idx,
            loops: s.loops(),
        });
        if tick.finished {
            emit.send(EngineMsg::Finished { reason: FinishReason::Completed });
            let _ = done.send(Cmd::Input(Input::PlaybackFinished(FinishReason::Completed)));
            return;
        }
    }
}
