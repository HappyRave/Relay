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
use relay_core::model::{Event, MouseBtn, Ms, Repeat, Rgb};
use relay_core::playback::{PlayClock, plan_times};
use relay_core::session::FinishReason;
use relay_core::steps::Step;
use relay_platform::{Injector, Platform};
use serde::Serialize;
use ts_rs::TS;

use crate::coordinator::Cmd;
use crate::ipc::{EngineMsg, Emitter};

const TICK_MS: f64 = 33.0;
/// How often a pixel check samples the screen.
const PIXEL_POLL_MS: f64 = 30.0;

/// Reads one screen pixel (a closure so tests can fake the screen).
pub type PixelReader = Box<dyn FnMut(i32, i32) -> Option<Rgb> + Send>;

/// A pixel check in progress: the clock is frozen until the pixel matches.
struct PixelWaiting {
    event: usize,
    /// Where playback continues once the pixel matches (the end of the IF block).
    resume_at: f64,
    x: i32,
    y: i32,
    color: Rgb,
    tolerance: u8,
    started: f64,
    timeout_ms: f64,
    next_poll: f64,
}

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

/// Lateness in 10 µs buckets up to 20 ms (later counts in the last one), so
/// an endless loop measures in constant memory.
struct Lateness {
    buckets: Vec<u32>,
    count: u32,
    max: f64,
}

impl Lateness {
    const BUCKET_MS: f64 = 0.01;
    const BUCKETS: usize = 2000;

    fn new() -> Self {
        Lateness { buckets: vec![0; Self::BUCKETS], count: 0, max: 0.0 }
    }

    fn add(&mut self, ms: f64) {
        let ms = ms.max(0.0);
        let i = ((ms / Self::BUCKET_MS) as usize).min(Self::BUCKETS - 1);
        self.buckets[i] += 1;
        self.count = self.count.saturating_add(1);
        self.max = self.max.max(ms);
    }

    fn quantile(&self, q: f64) -> f64 {
        let rank = ((self.count as f64 - 1.0) * q).round() as u32;
        let mut seen = 0;
        for (i, n) in self.buckets.iter().enumerate() {
            seen += n;
            if seen > rank {
                return (i as f64 * Self::BUCKET_MS).min(self.max);
            }
        }
        self.max
    }

    fn stats(&self) -> Option<TimingStats> {
        (self.count > 0).then(|| TimingStats {
            events: self.count,
            p50_ms: self.quantile(0.5),
            p99_ms: self.quantile(0.99),
            max_ms: self.max,
        })
    }
}

pub struct Engine {
    plan: PlayPlan,
    injector: Box<dyn Injector>,
    pixel: PixelReader,
    waiting: Option<PixelWaiting>,
    /// Paused by the user (as opposed to frozen by a pixel check).
    user_paused: Option<f64>,
    /// The 1-based step whose pixel check timed out.
    pub timed_out_step: Option<usize>,
    times: Vec<f64>,
    idx: usize,
    loop_idx: u32,
    clock: PlayClock,
    keys_down: Vec<KeyStroke>,
    buttons_down: Vec<MouseBtn>,
    lateness: Lateness,
    /// The first injection failure, reported once.
    pub injection_error: Option<String>,
}

impl Engine {
    pub fn new(plan: PlayPlan, injector: Box<dyn Injector>, pixel: PixelReader, now: f64) -> Self {
        let from = if plan.from.saturating_add(1) >= plan.duration { 0 } else { plan.from };
        let times = plan_times(&plan.events, &plan.steps, plan.jitter_ms, plan.seed);
        let idx = times.partition_point(|&t| t < from as f64);
        let clock = PlayClock::new(from, now, plan.speed);
        Engine {
            plan,
            injector,
            pixel,
            waiting: None,
            user_paused: None,
            timed_out_step: None,
            times,
            idx,
            loop_idx: 0,
            clock,
            keys_down: Vec::new(),
            buttons_down: Vec::new(),
            lateness: Lateness::new(),
            injection_error: None,
        }
    }

    pub fn loops(&self) -> Option<u32> {
        self.plan.repeat.loops()
    }

    pub fn loop_idx(&self) -> u32 {
        self.loop_idx
    }

    pub fn macro_time(&self, now: f64) -> f64 {
        self.clock.at(now).min(self.plan.duration as f64)
    }

    /// Whether the playhead stands still (paused, or waiting for a pixel).
    pub fn paused(&self) -> bool {
        self.clock.paused()
    }

    pub fn speed(&self) -> f64 {
        self.clock.speed()
    }

    /// Wall time of the next event, pixel sample or loop end; `None` while paused.
    pub fn next_deadline(&self) -> Option<f64> {
        if self.user_paused.is_some() {
            return None;
        }
        if let Some(w) = &self.waiting {
            return Some(w.next_poll);
        }
        let target = self.times.get(self.idx).copied().unwrap_or(self.plan.duration as f64);
        self.clock.deadline(target)
    }

    /// Injects every event that is due at `now`. Returns `Some` when the last loop finished.
    pub fn advance(&mut self, now: f64) -> Option<FinishReason> {
        if self.user_paused.is_some() {
            return None;
        }
        if self.waiting.is_some() {
            return self.poll_pixel(now);
        }
        let t = self.clock.at(now);
        while self.idx < self.times.len() && self.times[self.idx] <= t {
            if let Some(deadline) = self.clock.deadline(self.times[self.idx]) {
                self.lateness.add(now - deadline);
            }
            let i = self.idx;
            self.idx += 1;
            if let Event::PixelWait { dur, x, y, color, tolerance, timeout_ms, .. } = self.plan.events[i] {
                // Freeze the playhead at the check and sample right away.
                let at = self.times[i];
                self.clock.seek(at, now);
                self.clock.pause(now);
                self.waiting = Some(PixelWaiting {
                    event: i,
                    resume_at: at + dur as f64,
                    x,
                    y,
                    color,
                    tolerance,
                    started: now,
                    timeout_ms: timeout_ms as f64,
                    next_poll: now,
                });
                return self.poll_pixel(now);
            }
            self.dispatch(i);
        }
        let duration = self.plan.duration as f64;
        if self.idx == self.times.len() && t >= duration {
            self.release_all();
            if self.loops().is_some_and(|n| self.loop_idx + 1 >= n) {
                return Some(FinishReason::Completed);
            }
            self.loop_idx += 1;
            // A fresh humanize pattern each loop.
            self.times = plan_times(&self.plan.events, &self.plan.steps, self.plan.jitter_ms, self.plan.seed ^ self.loop_idx as u64);
            self.idx = 0;
            // Carry the overshoot into the next loop, so loops don't drift.
            self.clock.seek((t - duration).min(duration), now);
        }
        None
    }

    fn poll_pixel(&mut self, now: f64) -> Option<FinishReason> {
        let w = self.waiting.as_mut()?;
        if now < w.next_poll {
            return None;
        }
        let (dx, dy) = self.plan.offset;
        let matched = (self.pixel)(w.x + dx, w.y + dy).is_some_and(|c| c.within(w.color, w.tolerance));
        if matched {
            let resume_at = w.resume_at;
            self.waiting = None;
            self.clock.seek(resume_at, now);
            self.clock.resume(now);
            return self.advance(now);
        }
        if now - w.started >= w.timeout_ms {
            let event = w.event as u32;
            self.timed_out_step = self.plan.steps.iter().position(|s| s.items.contains(&event)).map(|i| i + 1);
            self.release_all();
            return Some(FinishReason::PixelTimeout);
        }
        w.next_poll = now + PIXEL_POLL_MS;
        None
    }

    pub fn pause(&mut self, now: f64) {
        if self.user_paused.is_none() {
            self.clock.pause(now);
            self.user_paused = Some(now);
        }
    }

    pub fn resume(&mut self, now: f64) {
        let Some(since) = self.user_paused.take() else { return };
        match &mut self.waiting {
            // Time spent paused doesn't count toward the check's timeout.
            Some(w) => {
                w.started += now - since;
                w.next_poll = now;
            }
            None => self.clock.resume(now),
        }
    }

    pub fn set_speed(&mut self, speed: f64, now: f64) {
        self.clock.set_speed(speed, now);
    }

    /// Jumps to macro time `t`, releasing what's held; stays paused if paused.
    pub fn seek(&mut self, t: f64, now: f64) {
        self.release_all();
        let t = t.clamp(0.0, self.plan.duration as f64);
        if self.waiting.take().is_some() && self.user_paused.is_none() {
            // The pixel check had frozen the clock; the jump leaves the check.
            self.clock.resume(now);
        }
        self.clock.seek(t, now);
        self.idx = self.times.partition_point(|&x| x < t);
    }

    pub fn stats(&self) -> Option<TimingStats> {
        self.lateness.stats()
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
            // Waits only take time; `advance` handles pixel checks before dispatching.
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

/// A running engine thread. Dropping it stops the engine.
pub struct EngineHandle {
    tx: Sender<EngineCmd>,
    wake: Arc<dyn Fn() + Send + Sync>,
    thread: Option<JoinHandle<Option<TimingStats>>>,
}

impl EngineHandle {
    pub fn send(&self, cmd: EngineCmd) {
        let _ = self.tx.send(cmd);
        (self.wake)();
    }

    /// Stops playback, releases held input, waits for the thread and
    /// returns the timing so far.
    pub fn stop(mut self) -> Option<TimingStats> {
        self.send(EngineCmd::Stop);
        self.thread.take().and_then(|t| t.join().ok().flatten())
    }
}

impl Drop for EngineHandle {
    fn drop(&mut self) {
        if let Some(t) = self.thread.take() {
            self.send(EngineCmd::Stop);
            let _ = t.join();
        }
    }
}

/// Tells the coordinator the engine ended on its own, even by panicking.
struct Done {
    coordinator: Sender<Cmd>,
    generation: u64,
    /// `None` until the engine ends; a panic leaves it `None` → `Error`.
    outcome: Option<(FinishReason, Option<TimingStats>)>,
    /// Stopped by the coordinator, which already knows.
    stopped: bool,
}

impl Drop for Done {
    fn drop(&mut self) {
        if self.stopped {
            return;
        }
        let (reason, timing) = self.outcome.take().unwrap_or((FinishReason::Error, None));
        let _ = self.coordinator.send(Cmd::EngineDone { generation: self.generation, reason, timing });
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
    let (tx, rx) = unbounded::<EngineCmd>();
    let (wake_tx, wake_rx) = crossbeam_channel::bounded(1);
    let (make_timer, make_injector, now_ms) = (platform.timer, platform.injector, platform.now_ms);
    let screen = platform.screen.clone();
    let thread = std::thread::Builder::new()
        .name("relay-engine".into())
        .spawn(move || {
            let mut done = Done { coordinator, generation, outcome: None, stopped: false };
            // Created on this thread: the timer also raises this thread's priority.
            let mut timer = make_timer();
            let _ = wake_tx.send(timer.waker());
            let pixel: PixelReader = Box::new(move |x, y| screen.pixel(x, y));
            let mut engine = Engine::new(plan, make_injector(), pixel, now_ms());
            let mut reported_error = false;
            let mut next_tick = f64::MIN;
            loop {
                loop {
                    let now = now_ms();
                    match rx.try_recv() {
                        Ok(EngineCmd::Pause) => engine.pause(now),
                        Ok(EngineCmd::Resume) => engine.resume(now),
                        Ok(EngineCmd::Seek(t)) => engine.seek(t, now),
                        Ok(EngineCmd::Speed(v)) => engine.set_speed(v, now),
                        Ok(EngineCmd::Stop) | Err(TryRecvError::Disconnected) => {
                            done.stopped = true;
                            return engine.stats(); // dropping the engine releases input
                        }
                        Err(TryRecvError::Empty) => break,
                    }
                    next_tick = f64::MIN; // report the change right away
                }
                let now = now_ms();
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
                    if reason == FinishReason::PixelTimeout {
                        let step = engine.timed_out_step.map_or(String::new(), |n| format!(" at step {n}"));
                        emit.send(EngineMsg::Notice { message: format!("Pixel check timed out{step}; playback stopped.") });
                    }
                    let stats = engine.stats();
                    done.outcome = Some((reason, stats.clone()));
                    return stats;
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
        (Engine::new(plan, Box::new(rec.clone()), Box::new(|_, _| None), 0.0), rec)
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
    fn loops_carry_their_overshoot() {
        let (mut e, _rec) = engine(vec![key(0, "KeyA", true), key(100, "KeyA", false)], Repeat::Count(3), 1.0);
        e.advance(0.0);
        e.advance(100.0);
        // The macro lasts 600 ms; we wake 40 ms late for the loop end.
        assert_eq!(e.advance(640.0), None);
        assert_eq!(e.loop_idx(), 1);
        assert_eq!(e.macro_time(640.0), 40.0);
    }

    #[test]
    fn a_seek_while_paused_stays_paused() {
        let (mut e, _rec) = engine(vec![key(0, "KeyA", true), key(1000, "KeyA", false)], Repeat::Count(1), 1.0);
        e.advance(0.0);
        e.pause(100.0);
        e.seek(500.0, 200.0);
        assert!(e.paused());
        assert_eq!(e.macro_time(9000.0), 500.0);
    }

    #[test]
    fn lateness_quantiles() {
        let mut l = Lateness::new();
        for i in 0..100 {
            l.add(i as f64 * 0.1);
        }
        l.add(80.0);
        let s = l.stats().unwrap();
        assert_eq!(s.events, 101);
        assert!((s.p50_ms - 5.0).abs() < 0.02, "{s:?}");
        assert!((s.p99_ms - 9.9).abs() < 0.02, "{s:?}");
        assert_eq!(s.max_ms, 80.0);
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

    fn pixel_macro() -> Vec<Event> {
        vec![
            Event::PixelWait { t: 100, dur: 900, x: 5, y: 6, color: Rgb(255, 0, 0), tolerance: 8, timeout_ms: 5000, label: String::new() },
            key(1000, "KeyA", true),
            key(1040, "KeyA", false),
        ]
    }

    fn pixel_engine(turns_red_at: f64) -> (Engine, Recorder, Arc<Mutex<f64>>) {
        let rec = Recorder::default();
        let events = pixel_macro();
        let steps = relay_core::steps::group_steps(&events, Default::default());
        let duration = relay_core::timeline::duration(&events);
        let plan = PlayPlan { events, steps, duration, repeat: Repeat::Count(1), speed: 1.0, jitter_ms: 0, seed: 0, offset: (0, 0), from: 0 };
        // The fake screen turns red at `turns_red_at` (wall ms), read through a shared clock.
        let now = Arc::new(Mutex::new(0.0));
        let clock = now.clone();
        let pixel: PixelReader = Box::new(move |x, y| {
            assert_eq!((x, y), (5, 6));
            Some(if *clock.lock().unwrap() >= turns_red_at { Rgb(250, 4, 2) } else { Rgb(255, 255, 255) })
        });
        (Engine::new(plan, Box::new(rec.clone()), pixel, 0.0), rec, now)
    }

    /// Advances the engine to `t`, updating the fake screen's clock.
    fn run_to(e: &mut Engine, now: &Arc<Mutex<f64>>, t: f64) -> Option<FinishReason> {
        *now.lock().unwrap() = t;
        e.advance(t)
    }

    #[test]
    fn pixel_check_waits_until_the_color_matches_then_continues() {
        let (mut e, rec, now) = pixel_engine(1200.0);
        assert_eq!(run_to(&mut e, &now, 100.0), None);
        assert!(e.paused(), "the playhead freezes at the check");
        assert_eq!(e.macro_time(100.0), 100.0);
        assert_eq!(e.next_deadline(), Some(130.0), "it samples every 30 ms");
        for t in [130.0, 600.0, 1190.0] {
            assert_eq!(run_to(&mut e, &now, t), None);
            assert_eq!(e.macro_time(t), 100.0);
        }
        assert!(rec.take().is_empty());
        // Matches at 1200 (within tolerance) and continues from the end of the IF block.
        run_to(&mut e, &now, 1220.0);
        assert!(!e.paused());
        assert_eq!(e.macro_time(1220.0), 1000.0);
        assert_eq!(rec.take(), ["KeyA down"]);
        run_to(&mut e, &now, 1260.0);
        assert_eq!(rec.take(), ["KeyA up"]);
    }

    #[test]
    fn pixel_check_times_out_with_its_step_number() {
        let (mut e, rec, now) = pixel_engine(f64::INFINITY);
        run_to(&mut e, &now, 100.0);
        assert_eq!(run_to(&mut e, &now, 5000.0), None);
        assert_eq!(run_to(&mut e, &now, 5100.0), Some(FinishReason::PixelTimeout));
        assert_eq!(e.timed_out_step, Some(1));
        assert!(rec.take().is_empty());
    }

    #[test]
    fn pausing_during_a_pixel_check_stops_its_timeout() {
        let (mut e, _rec, now) = pixel_engine(f64::INFINITY);
        run_to(&mut e, &now, 100.0);
        e.pause(1000.0);
        assert_eq!(e.next_deadline(), None);
        assert_eq!(run_to(&mut e, &now, 60_000.0), None);
        e.resume(60_000.0);
        // 900 ms of the 5 s timeout were used before the pause.
        assert_eq!(run_to(&mut e, &now, 64_000.0), None);
        assert_eq!(run_to(&mut e, &now, 64_200.0), Some(FinishReason::PixelTimeout));
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
        let mut e = Engine::new(plan, Box::new(rec.clone()), Box::new(|_, _| None), 0.0);
        e.advance(0.0);
        assert_eq!(rec.take(), ["move 130,90"]);
    }
}
