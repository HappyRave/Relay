//! Runs the recorder on its own thread: drains raw hook input, reports
//! progress to the UI and hands the finished recording back on stop.

use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, bounded};
use relay_core::model::Rect;
use relay_core::steps::{GroupOptions, group_steps};
use relay_platform::recorder::{Recorder, Recording};
use relay_platform::types::MousePulse;
use relay_platform::{RawInput, RawKind, Screen};

use crate::coordinator::Cmd;
use crate::ipc::{Emitter, EngineMsg};

const POLL: Duration = Duration::from_millis(25);
const PROGRESS_EVERY_MS: f64 = 100.0;

/// Notices that Windows removed the hook: it does that silently to hooks it
/// considers too slow. The tell is a cursor that moves while the hook sees no
/// mouse events. Events it filters out count too: with "Ignore simulated
/// input", a program moving the cursor (SendInput, SetCursorPos) sends only
/// injected events, which the hook drops but still sees. Reports at most once
/// per `cooldown`.
pub struct HookWatchdog {
    last_cursor: Option<(i32, i32)>,
    last_event: f64,
    last_alarm: f64,
    silence_ms: f64,
    cooldown_ms: f64,
}

impl HookWatchdog {
    pub fn new(now: f64) -> Self {
        HookWatchdog {
            last_cursor: None,
            last_event: now,
            last_alarm: f64::MIN,
            silence_ms: 1000.0,
            cooldown_ms: 5000.0,
        }
    }

    /// The hook saw a mouse event at `time` (reported or filtered out). An
    /// older time than one already seen changes nothing.
    pub fn saw_event(&mut self, time: f64) {
        self.last_event = self.last_event.max(time);
    }

    /// [`check`](Self::check), counting the mouse events the hook saw but
    /// filtered out (`pulse`).
    pub fn check_with(&mut self, pulse: &MousePulse, now: f64, cursor: (i32, i32)) -> bool {
        self.saw_event(pulse.last());
        self.check(now, cursor)
    }

    /// Feeds the current cursor position; true when the hook looks dead.
    pub fn check(&mut self, now: f64, cursor: (i32, i32)) -> bool {
        let moved = self.last_cursor.is_some_and(|c| c != cursor);
        self.last_cursor = Some(cursor);
        if moved && now - self.last_event > self.silence_ms && now - self.last_alarm > self.cooldown_ms {
            self.last_alarm = now;
            self.last_event = now;
            return true;
        }
        false
    }
}

pub struct RecThread {
    stop: Sender<()>,
    thread: JoinHandle<Recording>,
}

pub struct RecContext {
    pub screen: Arc<dyn Screen>,
    pub desktop: Rect,
    pub group: GroupOptions,
    pub now_ms: fn() -> f64,
    pub emit: Arc<Emitter>,
    pub coordinator: Sender<Cmd>,
    /// When the hook last saw a mouse event, filtered out or not.
    pub mouse_pulse: Arc<MousePulse>,
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

    /// Stops recording and returns what was recorded; `None` if the
    /// recorder thread panicked.
    pub fn finish(self) -> Option<Recording> {
        let _ = self.stop.send(());
        self.thread.join().ok()
    }
}

fn run(mut rec: Recorder, raw: Receiver<RawInput>, stop: Receiver<()>, ctx: RecContext) -> Recording {
    let mut last_progress = f64::MIN;
    let mut grouped_len = usize::MAX;
    let mut watchdog = HookWatchdog::new((ctx.now_ms)());
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
            Ok(r) => {
                if matches!(r.kind, RawKind::Move { .. } | RawKind::Button { .. } | RawKind::Wheel { .. }) {
                    watchdog.saw_event(r.time);
                }
                rec.push(r);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return rec.finish((ctx.now_ms)()),
        }
        let now = (ctx.now_ms)();
        if now - last_progress >= PROGRESS_EVERY_MS {
            last_progress = now;
            if watchdog.check_with(&ctx.mouse_pulse, now, ctx.screen.cursor_pos()) {
                let _ = ctx.coordinator.send(Cmd::HookLost);
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watchdog_fires_when_the_cursor_moves_without_events() {
        let mut w = HookWatchdog::new(0.0);
        assert!(!w.check(100.0, (0, 0)), "first sample is the baseline");
        // Moving with events arriving: fine.
        w.saw_event(500.0);
        assert!(!w.check(600.0, (5, 5)));
        // Moving while events stopped for over a second: the hook is gone.
        assert!(!w.check(1400.0, (6, 6)), "only 900 ms of silence");
        assert!(w.check(1700.0, (7, 7)));
        // Not again right away, even though it's still silent.
        assert!(!w.check(3000.0, (8, 8)));
        assert!(w.check(6800.0, (9, 9)), "after the cooldown");
    }

    #[test]
    fn events_the_hook_filters_out_keep_it_alive() {
        // "Ignore simulated input": another program moves the cursor; the hook
        // drops every event, so none arrive, but it beats the pulse.
        let pulse = MousePulse::default();
        let mut w = HookWatchdog::new(0.0);
        for i in 0..100 {
            let now = i as f64 * 100.0;
            pulse.beat(now - 5.0);
            assert!(!w.check_with(&pulse, now, (i, i)), "at {now} ms");
        }
        // The hook is removed: the pulse stops while the cursor keeps moving.
        assert!(!w.check_with(&pulse, 10_500.0, (200, 200)));
        assert!(w.check_with(&pulse, 11_100.0, (201, 201)));
    }

    #[test]
    fn an_older_event_doesnt_move_the_watchdog_back() {
        let mut w = HookWatchdog::new(0.0);
        w.check(0.0, (0, 0));
        w.saw_event(2000.0);
        w.saw_event(100.0); // a pulse that lags a reported event
        assert!(!w.check(2900.0, (1, 1)), "900 ms since the latest");
        assert!(w.check(3100.0, (2, 2)));
    }

    #[test]
    fn a_still_cursor_never_alarms() {
        let mut w = HookWatchdog::new(0.0);
        for t in 0..100 {
            assert!(!w.check(t as f64 * 1000.0, (3, 3)));
        }
    }
}
