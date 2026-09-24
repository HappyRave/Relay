//! The playback engine. [`Engine`] decides what to inject when, given wall
//! times passed in by the caller, so it's unit-tested with a fake injector;
//! [`spawn`] runs it on a high-priority thread with the precision timer.
//!
//! Anything the engine pressed is released on stop, seek, loop end, drop
//! and panic, so a macro never leaves a key or button stuck.

use std::sync::Arc;
use std::thread::JoinHandle;

use crossbeam_channel::{Sender, TryRecvError, unbounded};
use relay_core::keys::KeyStroke;
use relay_core::model::{Event, MouseBtn, Ms, Repeat};
use relay_core::playback::{PlayClock, plan_times};
use relay_core::session::FinishReason;
use relay_core::steps::Step;
use relay_platform::{Injector, Platform};
use serde::Serialize;
use ts_rs::TS;

use crate::coordinator::Cmd;
use crate::ipc::{EngineMsg, Emitter};

const TICK_MS: f64 = 33.0;

/// Everything the engine needs to play one macro.
pub struct PlayPlan {
    pub events: Vec<Event>,
    pub steps: Vec<Step>,
    pub duration: Ms,
    pub repeat: Repeat,
    pub speed: f64,
    /// ± per-step timing jitter ("Humanize"); 0 plays the recording exactly.
    pub jitter_ms: u32,
    pub seed: u64,
    /// Added to every position ("Window" coordinates: where the anchor window moved).
    pub offset: (i32, i32),
    pub from: Ms,
}

/// How late events were injected relative to their deadlines.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct TimingStats {
    pub events: u32,
    pub p50_ms: f64,
    pub p99_ms: f64,
    pub max_ms: f64,
}

pub struct Engine {
    plan: PlayPlan,
    injector: Box<dyn Injector>,
    times: Vec<f64>,
    idx: usize,
    loop_idx: u32,
    clock: PlayClock,
    keys_down: Vec<KeyStroke>,
    buttons_down: Vec<MouseBtn>,
    lateness: Vec<f64>,
    /// The first injection failure, reported once.
    pub injection_error: Option<String>,
}

impl Engine {
    pub fn new(plan: PlayPlan, injector: Box<dyn Injector>, now: f64) -> Self {
        let from = if plan.from + 1 >= plan.duration { 0 } else { plan.from };
        let times = plan_times(&plan.events, &plan.steps, plan.jitter_ms, plan.seed);
        let idx = times.partition_point(|&t| t < from as f64);
        let clock = PlayClock::new(from, now, plan.speed);
        Engine {
            plan,
            injector,
            times,
            idx,
            loop_idx: 0,
            clock,
            keys_down: Vec::new(),
            buttons_down: Vec::new(),
            lateness: Vec::new(),
            injection_error: None,
        }
    }

    pub fn loops(&self) -> Option<u32> {
        match self.plan.repeat {
            Repeat::Count(n) => Some(n.max(1)),
            Repeat::Forever => None,
        }
    }

    pub fn loop_idx(&self) -> u32 {
        self.loop_idx
    }

    pub fn macro_time(&self, now: f64) -> f64 {
        self.clock.at(now).min(self.plan.duration as f64)
    }

    pub fn paused(&self) -> bool {
        self.clock.paused()
    }

    pub fn speed(&self) -> f64 {
        self.clock.speed()
    }

    /// Wall time of the next event (or of the loop end); `None` while paused.
    pub fn next_deadline(&self) -> Option<f64> {
        let target = self.times.get(self.idx).copied().unwrap_or(self.plan.duration as f64);
        self.clock.deadline(target)
    }

    /// Injects every event that is due at `now`. Returns `Some` when the last loop finished.
    pub fn advance(&mut self, now: f64) -> Option<FinishReason> {
        if self.clock.paused() {
            return None;
        }
        let t = self.clock.at(now);
        while self.idx < self.times.len() && self.times[self.idx] <= t {
            if let Some(deadline) = self.clock.deadline(self.times[self.idx]) {
                self.lateness.push((now - deadline).max(0.0));
            }
            self.dispatch(self.idx);
            self.idx += 1;
        }
        if self.idx == self.times.len() && t >= self.plan.duration as f64 {
            self.release_all();
            if self.loops().is_some_and(|n| self.loop_idx + 1 >= n) {
                return Some(FinishReason::Completed);
            }
            self.loop_idx += 1;
            // A fresh humanize pattern each loop.
            self.times = plan_times(&self.plan.events, &self.plan.steps, self.plan.jitter_ms, self.plan.seed ^ self.loop_idx as u64);
            self.idx = 0;
            self.clock.seek(0.0, now);
        }
        None
    }

    pub fn pause(&mut self, now: f64) {
        self.clock.pause(now);
    }

    pub fn resume(&mut self, now: f64) {
        self.clock.resume(now);
    }

    pub fn set_speed(&mut self, speed: f64, now: f64) {
        self.clock.set_speed(speed, now);
    }

    pub fn seek(&mut self, t: f64, now: f64) {
        self.release_all();
        let t = t.clamp(0.0, self.plan.duration as f64);
        self.clock.seek(t, now);
        self.idx = self.times.partition_point(|&x| x < t);
    }

    pub fn stats(&self) -> Option<TimingStats> {
        if self.lateness.is_empty() {
            return None;
        }
        let mut l = self.lateness.clone();
        l.sort_by(f64::total_cmp);
        let at = |q: f64| l[((l.len() - 1) as f64 * q).round() as usize];
        Some(TimingStats { events: l.len() as u32, p50_ms: at(0.5), p99_ms: at(0.99), max_ms: *l.last().unwrap() })
    }

    fn check(&mut self, r: relay_platform::Result<()>) {
        if let Err(e) = r
            && self.injection_error.is_none()
        {
            self.injection_error = Some(e.to_string());
        }
    }

    fn dispatch(&mut self, i: usize) {
        let (dx, dy) = self.plan.offset;
        let ev = self.plan.events[i].clone();
        match ev {
            Event::Move { x, y, .. } => {
                let r = self.injector.move_to(x + dx, y + dy);
                self.check(r);
            }
            Event::Button { x, y, btn, down, .. } => {
                let r = self.injector.move_to(x + dx, y + dy).and_then(|_| self.injector.button(btn, down));
                self.check(r);
                self.buttons_down.retain(|b| *b != btn);
                if down {
                    self.buttons_down.push(btn);
                }
            }
            Event::Wheel { x, y, delta, horizontal, .. } => {
                let r = self.injector.move_to(x + dx, y + dy).and_then(|_| self.injector.wheel(delta, horizontal));
                self.check(r);
            }
            Event::Key { down, key, ch, .. } => {
                let r = self.injector.key(&key, down, ch.as_deref());
                self.check(r);
                self.keys_down.retain(|k| k.code != key.code);
                if down {
                    self.keys_down.push(key);
                }
            }
            // Time passes; pixel checks poll the screen from M4.
            Event::Wait { .. } | Event::PixelWait { .. } => {}
        }
    }

    /// Releases every key and button this engine is holding, newest first.
    pub fn release_all(&mut self) {
        while let Some(key) = self.keys_down.pop() {
            let _ = self.injector.key(&key, false, None);
        }
        while let Some(btn) = self.buttons_down.pop() {
            let _ = self.injector.button(btn, false);
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.release_all();
    }
}

pub enum EngineCmd {
    Pause,
    Resume,
    Seek(f64),
    Speed(f64),
    Stop,
}

/// A running engine thread.
pub struct EngineHandle {
    tx: Sender<EngineCmd>,
    wake: Arc<dyn Fn() + Send + Sync>,
    thread: Option<JoinHandle<()>>,
}

impl EngineHandle {
    pub fn send(&self, cmd: EngineCmd) {
        let _ = self.tx.send(cmd);
        (self.wake)();
    }

    /// Stops playback, releases held input and waits for the thread.
    pub fn stop(mut self) {
        self.send(EngineCmd::Stop);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

pub fn spawn(plan: PlayPlan, platform: &Platform, emit: Arc<Emitter>, coordinator: Sender<Cmd>) -> EngineHandle {
    let (tx, rx) = unbounded::<EngineCmd>();
    let (wake_tx, wake_rx) = crossbeam_channel::bounded(1);
    let (make_timer, make_injector) = (platform.timer, platform.injector);
    let thread = std::thread::Builder::new()
        .name("relay-engine".into())
        .spawn(move || {
            // Created on this thread: the timer also raises this thread's priority.
            let mut timer = make_timer();
            let _ = wake_tx.send(timer.waker());
            let mut engine = Engine::new(plan, make_injector(), timer.now_ms());
            let mut reported_error = false;
            let mut next_tick = f64::MIN;
            loop {
                loop {
                    let now = timer.now_ms();
                    match rx.try_recv() {
                        Ok(EngineCmd::Pause) => engine.pause(now),
                        Ok(EngineCmd::Resume) => engine.resume(now),
                        Ok(EngineCmd::Seek(t)) => engine.seek(t, now),
                        Ok(EngineCmd::Speed(v)) => engine.set_speed(v, now),
                        Ok(EngineCmd::Stop) | Err(TryRecvError::Disconnected) => return, // Drop releases input
                        Err(TryRecvError::Empty) => break,
                    }
                    next_tick = f64::MIN; // report the change right away
                }
                let now = timer.now_ms();
                let finished = engine.advance(now);
                if !reported_error && let Some(e) = &engine.injection_error {
                    reported_error = true;
                    emit.send(EngineMsg::Notice { message: format!("Playback: {e}") });
                }
                if now >= next_tick || finished.is_some() {
                    next_tick = now + TICK_MS;
                    emit.send(EngineMsg::PlayTick {
                        t: engine.macro_time(now),
                        advancing: !engine.paused() && finished.is_none(),
                        speed: engine.speed(),
                        loop_idx: engine.loop_idx(),
                        loops: engine.loops(),
                    });
                }
                if let Some(reason) = finished {
                    emit.send(EngineMsg::Finished { reason, timing: engine.stats() });
                    let _ = coordinator.send(Cmd::EngineDone(reason));
                    return;
                }
                let deadline = engine.next_deadline().map_or(next_tick, |d| d.min(next_tick));
                timer.wait_until(deadline);
            }
        })
        .expect("spawn engine thread");
    let wake = wake_rx.recv().expect("engine thread started");
    EngineHandle { tx, wake, thread: Some(thread) }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// Records every injected action as text.
    #[derive(Clone, Default)]
    struct Recorder(Arc<Mutex<Vec<String>>>);

    impl Injector for Recorder {
        fn move_to(&mut self, x: i32, y: i32) -> relay_platform::Result<()> {
            self.0.lock().unwrap().push(format!("move {x},{y}"));
            Ok(())
        }
        fn button(&mut self, btn: MouseBtn, down: bool) -> relay_platform::Result<()> {
            self.0.lock().unwrap().push(format!("{btn:?} {}", if down { "down" } else { "up" }));
            Ok(())
        }
        fn wheel(&mut self, delta: i32, _: bool) -> relay_platform::Result<()> {
            self.0.lock().unwrap().push(format!("wheel {delta}"));
            Ok(())
        }
        fn key(&mut self, key: &KeyStroke, down: bool, _: Option<&str>) -> relay_platform::Result<()> {
            self.0.lock().unwrap().push(format!("{} {}", key.code, if down { "down" } else { "up" }));
            Ok(())
        }
    }

    impl Recorder {
        fn take(&self) -> Vec<String> {
            std::mem::take(&mut self.0.lock().unwrap())
        }
    }

    fn key(t: Ms, code: &str, down: bool) -> Event {
        Event::Key { t, down, key: KeyStroke::code(code), ch: None }
    }
    fn btn(t: Ms, down: bool) -> Event {
        Event::Button { t, x: 10, y: 20, btn: MouseBtn::Left, down, label: String::new() }
    }

    fn engine(events: Vec<Event>, repeat: Repeat, speed: f64) -> (Engine, Recorder) {
        let rec = Recorder::default();
        let steps = relay_core::steps::group_steps(&events, Default::default());
        let duration = relay_core::timeline::duration(&events);
        let plan = PlayPlan { events, steps, duration, repeat, speed, jitter_ms: 0, seed: 0, offset: (0, 0), from: 0 };
        (Engine::new(plan, Box::new(rec.clone()), 0.0), rec)
    }

    #[test]
    fn injects_on_schedule_and_measures_lateness() {
        let (mut e, rec) = engine(vec![key(0, "KeyA", true), key(100, "KeyA", false), key(200, "KeyB", true)], Repeat::Count(1), 1.0);
        assert_eq!(e.advance(0.0), None);
        assert_eq!(rec.take(), ["KeyA down"]);
        assert_eq!(e.next_deadline(), Some(100.0));
        e.advance(103.0);
        assert_eq!(rec.take(), ["KeyA up"]);
        e.advance(250.0);
        assert_eq!(rec.take(), ["KeyB down"]);
        // Duration is 200 + 500 ms tail; the held key is released at the end.
        assert_eq!(e.advance(700.0), Some(FinishReason::Completed));
        assert_eq!(rec.take(), ["KeyB up"]);
        let stats = e.stats().unwrap();
        assert_eq!(stats.events, 3);
        assert_eq!(stats.max_ms, 50.0);
    }

    #[test]
    fn speed_halves_deadlines() {
        let (mut e, rec) = engine(vec![key(0, "KeyA", true), key(1000, "KeyA", false)], Repeat::Count(1), 2.0);
        e.advance(0.0);
        assert_eq!(e.next_deadline(), Some(500.0));
        e.advance(499.0);
        assert_eq!(rec.take(), ["KeyA down"]);
        e.advance(500.0);
        assert_eq!(rec.take(), ["KeyA up"]);
    }

    #[test]
    fn pause_seek_and_speed_changes() {
        let (mut e, rec) = engine(vec![key(0, "KeyA", true), key(1000, "KeyA", false), key(2000, "KeyB", true)], Repeat::Count(1), 1.0);
        e.advance(0.0);
        e.pause(300.0);
        assert_eq!(e.next_deadline(), None);
        assert_eq!(e.advance(5000.0), None);
        e.resume(5000.0);
        assert_eq!(e.next_deadline(), Some(5700.0));
        // Seeking releases what's held and skips ahead.
        e.seek(1500.0, 6000.0);
        assert_eq!(rec.take(), ["KeyA down", "KeyA up"]);
        e.set_speed(4.0, 6000.0);
        assert_eq!(e.next_deadline(), Some(6125.0));
    }

    #[test]
    fn loops_replay_and_release_between_loops() {
        let (mut e, rec) = engine(vec![btn(0, true), btn(100, false)], Repeat::Count(2), 1.0);
        e.advance(0.0);
        e.advance(100.0);
        assert_eq!(e.advance(600.0), None);
        assert_eq!(e.loop_idx(), 1);
        e.advance(600.0);
        e.advance(700.0);
        assert_eq!(e.advance(1200.0), Some(FinishReason::Completed));
        let actions = rec.take();
        assert_eq!(actions.iter().filter(|a| a.ends_with("down")).count(), 2);
        assert_eq!(actions.iter().filter(|a| a.ends_with("up")).count(), 2);
    }

    #[test]
    fn dropping_mid_drag_releases_the_button() {
        let (mut e, rec) = engine(vec![btn(0, true), Event::Move { t: 50, x: 90, y: 20 }, btn(100, false)], Repeat::Count(1), 1.0);
        e.advance(60.0);
        drop(e);
        assert_eq!(rec.take(), ["move 10,20", "Left down", "move 90,20", "Left up"]);
    }

    #[test]
    fn window_offset_moves_every_position() {
        let rec = Recorder::default();
        let events = vec![Event::Move { t: 0, x: 100, y: 100 }];
        let plan = PlayPlan {
            steps: vec![],
            duration: 500,
            repeat: Repeat::Count(1),
            speed: 1.0,
            jitter_ms: 0,
            seed: 0,
            offset: (30, -10),
            from: 0,
            events,
        };
        let mut e = Engine::new(plan, Box::new(rec.clone()), 0.0);
        e.advance(0.0);
        assert_eq!(rec.take(), ["move 130,90"]);
    }
}
