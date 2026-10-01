//! The playback engine. [`Engine`] decides what to inject when, given wall
//! times passed in by the caller, so it's unit-tested with a fake injector;
//! [`spawn`] runs it on a high-priority thread with the precision timer.
//!
//! Anything the engine pressed is released on stop, seek, loop end, drop
//! and panic, so a macro never leaves a key or button stuck.

use std::sync::Arc;
use std::thread::JoinHandle;

use crossbeam_channel::{Sender, TryRecvError, unbounded};
use relay_core::image::{Gray, Match, Rgb8};
use relay_core::keys::KeyStroke;
use relay_core::model::{Event, MouseBtn, Ms, Rect, Repeat, Rgb};
use relay_core::playback::{PlayClock, plan_times};
use relay_core::runlog::{CheckLog, CheckOutcome, CheckResult};
use relay_core::session::FinishReason;
use relay_core::steps::{Step, StepKind};
use relay_core::text::CHAR_MS;
use relay_platform::{Injector, Platform};
use serde::Serialize;
use ts_rs::TS;

const TICK_MS: f64 = 33.0;
/// How often a pixel check samples the screen.
const PIXEL_POLL_MS: f64 = 30.0;
/// How often a Find image step looks at the screen (a search takes 20–50 ms).
const IMAGE_POLL_MS: f64 = 250.0;

/// Reads one screen pixel (a closure so tests can fake the screen).
pub type PixelReader = Box<dyn FnMut(i32, i32) -> Option<Rgb> + Send>;
/// Looks for an image in an area of the screen (or all of it), and returns
/// the best match at or above a score, in screen pixels.
pub type ImageFinder = Box<dyn FnMut(&Gray, Option<Rect>, f32) -> Option<Match> + Send>;
/// What a Text step's template types on repeat `n` (from 1), now.
pub type TextFiller = Box<dyn FnMut(&str, u32) -> String + Send>;

/// What a check waits for.
enum Check {
    Pixel {
        x: i32,
        y: i32,
        color: Rgb,
        tolerance: u8,
    },
    /// `image` is `None` when the step's image can't be read: it's never found.
    Image {
        image: Option<Gray>,
        click: (i32, i32),
        btn: MouseBtn,
        threshold: f32,
        area: Option<Rect>,
    },
}

/// A pixel check or Find image step in progress: the clock is frozen until
/// the pixel matches or the image shows up.
struct Waiting {
    event: usize,
    /// Where playback continues once it's found (the end of the step's block).
    resume_at: f64,
    check: Check,
    started: f64,
    timeout_ms: f64,
    next_poll: f64,
}

/// A Text step being typed: the clock is frozen at its start while the
/// characters go out one at a time.
struct Typing {
    chars: Vec<char>,
    typed: usize,
    /// Where the step starts and ends, in macro time.
    at: f64,
    end: f64,
    /// Wall time of the next character, or of the end once all are typed.
    next: f64,
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
    /// Relay's window, which image searches never look inside.
    pub own_window: isize,
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

/// What a playback did, for the run history.
#[derive(Debug, Default)]
pub struct RunReport {
    pub timing: Option<TimingStats>,
    pub checks: CheckLog,
    /// Loops played, the current one included.
    pub loops: u32,
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

/// The first event to play when starting (or seeking) at macro time `t`. It
/// goes by the recorded times, not the humanized ones: Humanize moves a step
/// earlier or later as a whole, so starting at a step's time plays all of it,
/// press and release, wherever the jitter put it.
fn start_index(events: &[Event], t: f64) -> usize {
    events.partition_point(|e| (e.t() as f64) < t)
}

pub struct Engine {
    plan: PlayPlan,
    injector: Box<dyn Injector>,
    pixel: PixelReader,
    find: ImageFinder,
    fill: TextFiller,
    waiting: Option<Waiting>,
    typing: Option<Typing>,
    /// Paused by the user (as opposed to frozen by a pixel check).
    user_paused: Option<f64>,
    /// The 1-based step whose pixel check or Find image step timed out.
    pub timed_out_step: Option<usize>,
    times: Vec<f64>,
    idx: usize,
    loop_idx: u32,
    clock: PlayClock,
    keys_down: Vec<KeyStroke>,
    buttons_down: Vec<MouseBtn>,
    lateness: Lateness,
    checks: CheckLog,
    /// The first injection failure, and whether it was reported.
    injection_error: Option<String>,
    error_reported: bool,
}

impl Engine {
    pub fn new(
        plan: PlayPlan,
        injector: Box<dyn Injector>,
        pixel: PixelReader,
        find: ImageFinder,
        fill: TextFiller,
        now: f64,
    ) -> Self {
        let from = if plan.from.saturating_add(1) >= plan.duration { 0 } else { plan.from };
        let times = plan_times(&plan.events, &plan.steps, plan.jitter_ms, plan.seed);
        let idx = start_index(&plan.events, from as f64);
        let clock = PlayClock::new(from, now, plan.speed);
        Engine {
            plan,
            injector,
            pixel,
            find,
            fill,
            waiting: None,
            typing: None,
            user_paused: None,
            timed_out_step: None,
            times,
            idx,
            loop_idx: 0,
            clock,
            keys_down: Vec::new(),
            buttons_down: Vec::new(),
            lateness: Lateness::new(),
            checks: CheckLog::default(),
            injection_error: None,
            error_reported: false,
        }
    }

    pub fn loops(&self) -> Option<u32> {
        self.plan.repeat.loops()
    }

    pub fn loop_idx(&self) -> u32 {
        self.loop_idx
    }

    /// The recorded time of the step whose pixel check timed out.
    pub fn timed_out_at(&self) -> Option<Ms> {
        self.timed_out_step.and_then(|n| self.plan.steps.get(n - 1)).map(|s| s.t)
    }

    /// Why playback stopped at `timed_out_step`, for the notice.
    pub fn timeout_notice(&self) -> Option<String> {
        let n = self.timed_out_step?;
        let image = self.plan.steps.get(n - 1).is_some_and(|s| matches!(s.kind, StepKind::FindImage { .. }));
        let what = if image { "Image not found" } else { "Pixel check timed out" };
        Some(format!("{what} at step {n}; playback stopped."))
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

    /// Wall time of the next event, character, pixel sample or loop end; `None` while paused.
    pub fn next_deadline(&self) -> Option<f64> {
        if self.user_paused.is_some() {
            return None;
        }
        if let Some(w) = &self.waiting {
            return Some(w.next_poll);
        }
        if let Some(ty) = &self.typing {
            return Some(ty.next);
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
        if self.typing.is_some() {
            return self.type_due(now);
        }
        let t = self.clock.at(now);
        while self.idx < self.times.len() && self.times[self.idx] <= t {
            if let Some(deadline) = self.clock.deadline(self.times[self.idx]) {
                self.lateness.add(now - deadline);
            }
            let i = self.idx;
            self.idx += 1;
            if let Event::Text { dur, text, .. } = &self.plan.events[i] {
                let (at, dur) = (self.times[i], *dur as f64);
                let chars = (self.fill)(text, self.loop_idx + 1).chars().collect();
                self.clock.seek(at, now);
                self.clock.pause(now);
                self.typing = Some(Typing { chars, typed: 0, at, end: at + dur, next: now });
                return self.type_due(now);
            }
            let check = match &self.plan.events[i] {
                &Event::PixelWait { dur, x, y, color, tolerance, timeout_ms, .. } => {
                    Some((dur, timeout_ms, Check::Pixel { x, y, color, tolerance }))
                }
                Event::FindImage { dur, image, click_x, click_y, btn, threshold, timeout_ms, area, .. } => {
                    let (dx, dy) = self.plan.offset;
                    Some((
                        *dur,
                        *timeout_ms,
                        Check::Image {
                            image: Rgb8::decode(&image.0).ok().map(|img| img.gray()),
                            click: (*click_x, *click_y),
                            btn: *btn,
                            threshold: *threshold as f32 / 100.0,
                            area: area.map(|a| Rect { x: a.x + dx, y: a.y + dy, ..a }),
                        },
                    ))
                }
                _ => None,
            };
            if let Some((dur, timeout_ms, check)) = check {
                // Freeze the playhead at the check and look right away.
                let at = self.times[i];
                self.clock.seek(at, now);
                self.clock.pause(now);
                self.waiting = Some(Waiting {
                    event: i,
                    resume_at: at + dur as f64,
                    check,
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
            self.times = plan_times(
                &self.plan.events,
                &self.plan.steps,
                self.plan.jitter_ms,
                self.plan.seed ^ self.loop_idx as u64,
            );
            self.idx = 0;
            // Carry the overshoot into the next loop, so loops don't drift.
            self.clock.seek((t - duration).min(duration), now);
        }
        None
    }

    fn poll_pixel(&mut self, now: f64) -> Option<FinishReason> {
        let w = self.waiting.as_ref()?;
        if now < w.next_poll {
            return None;
        }
        let (dx, dy) = self.plan.offset;
        let mut to_click = None;
        let (matched, poll) = match &w.check {
            &Check::Pixel { x, y, color, tolerance } => {
                ((self.pixel)(x + dx, y + dy).is_some_and(|c| c.within(color, tolerance)), PIXEL_POLL_MS)
            }
            Check::Image { image, click, btn, threshold, area } => {
                let found = image.as_ref().and_then(|img| (self.find)(img, *area, *threshold));
                to_click = found.map(|m| (m, *click, *btn));
                (found.is_some(), IMAGE_POLL_MS)
            }
        };
        if let Some((m, click, btn)) = to_click {
            self.click_found(m, click, btn);
        }
        if matched {
            let outcome = match to_click {
                Some((m, ..)) => CheckOutcome::Found { x: m.x, y: m.y, score: (m.score * 100.0).round() as u8 },
                None => CheckOutcome::Matched,
            };
            self.record_check(now, outcome);
        }
        let w = self.waiting.as_mut()?;
        if matched {
            let resume_at = w.resume_at;
            self.waiting = None;
            self.clock.seek(resume_at, now);
            self.clock.resume(now);
            return self.advance(now);
        }
        if now - w.started >= w.timeout_ms {
            let event = w.event;
            self.timed_out_step = self.step_of(event);
            self.record_check(now, CheckOutcome::TimedOut);
            self.waiting = None;
            self.release_all();
            return Some(FinishReason::PixelTimeout);
        }
        // Looked at once more when the timeout ends, so it ends on time.
        w.next_poll = (now + poll).min(w.started + w.timeout_ms);
        None
    }

    /// Types the characters that are due, then goes on from where typing
    /// got to in the step, or from its end when the text ran over it.
    fn type_due(&mut self, now: f64) -> Option<FinishReason> {
        loop {
            let pace = CHAR_MS as f64 / self.clock.speed();
            let ty = self.typing.as_mut()?;
            if ty.next > now {
                return None;
            }
            let Some(&c) = ty.chars.get(ty.typed) else {
                let resume_at = (ty.at + ty.chars.len() as f64 * CHAR_MS as f64).min(ty.end);
                self.typing = None;
                self.clock.seek(resume_at, now);
                self.clock.resume(now);
                return self.advance(now);
            };
            ty.typed += 1;
            ty.next += pace;
            let r = self.injector.text(c.encode_utf8(&mut [0; 4]));
            self.check(r);
        }
    }

    /// The 1-based step that event `event` belongs to.
    fn step_of(&self, event: usize) -> Option<usize> {
        self.plan.steps.iter().position(|s| s.items.contains(&(event as u32))).map(|i| i + 1)
    }

    /// Adds the result of the check being waited on to the run's checks.
    fn record_check(&mut self, now: f64, outcome: CheckOutcome) {
        let Some(w) = &self.waiting else { return };
        // Paused by the user: the wait stopped counting when the pause began.
        let end = self.user_paused.unwrap_or(now);
        let check = CheckResult {
            step: self.step_of(w.event).unwrap_or(0) as u32,
            loop_idx: self.loop_idx,
            image: matches!(w.check, Check::Image { .. }),
            after_ms: (end - w.started).max(0.0).round() as u32,
            outcome,
        };
        self.checks.push(check);
    }

    /// Ends the run: a check still waiting counts as interrupted.
    pub fn report(&mut self, now: f64) -> RunReport {
        self.record_check(now, CheckOutcome::Interrupted);
        self.waiting = None;
        RunReport { timing: self.stats(), checks: std::mem::take(&mut self.checks), loops: self.loop_idx + 1 }
    }

    /// Clicks `btn` at `click` (in the image's pixels, scaled like it) from
    /// where the image was found.
    fn click_found(&mut self, m: Match, click: (i32, i32), btn: MouseBtn) {
        let at = |o: i32, c: i32| o + (c as f32 * m.scale).round() as i32;
        let r = self.injector.move_to(at(m.x, click.0), at(m.y, click.1)).and_then(|_| self.injector.button(btn, true));
        self.check(r);
        self.buttons_down.retain(|b| *b != btn);
        self.buttons_down.push(btn);
        let r = self.injector.button(btn, false);
        self.check(r);
        self.buttons_down.retain(|b| *b != btn);
    }

    pub fn pause(&mut self, now: f64) {
        if self.user_paused.is_none() {
            self.clock.pause(now);
            self.user_paused = Some(now);
        }
    }

    pub fn resume(&mut self, now: f64) {
        let Some(since) = self.user_paused.take() else { return };
        match (&mut self.waiting, &mut self.typing) {
            // Time spent paused doesn't count toward the check's timeout.
            (Some(w), _) => {
                w.started += now - since;
                w.next_poll = now;
            }
            (_, Some(ty)) => ty.next += now - since,
            (None, None) => self.clock.resume(now),
        }
    }

    pub fn set_speed(&mut self, speed: f64, now: f64) {
        self.clock.set_speed(speed, now);
    }

    /// Jumps to macro time `t`, releasing what's held; stays paused if paused.
    pub fn seek(&mut self, t: f64, now: f64) {
        self.release_all();
        let t = t.clamp(0.0, self.plan.duration as f64);
        let frozen = self.waiting.take().is_some() | self.typing.take().is_some();
        if frozen && self.user_paused.is_none() {
            // The check or the typing had frozen the clock; the jump leaves it.
            self.clock.resume(now);
        }
        self.clock.seek(t, now);
        self.idx = start_index(&self.plan.events, t);
    }

    pub fn stats(&self) -> Option<TimingStats> {
        self.lateness.stats()
    }

    /// The first injection failure, once: `None` before any and after it was
    /// taken, so the user hears about a failing injector once, not per event.
    pub fn unreported_error(&mut self) -> Option<String> {
        if self.error_reported {
            return None;
        }
        self.error_reported = self.injection_error.is_some();
        self.injection_error.clone()
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
            // Waits only take time; `advance` handles checks and text before dispatching.
            Event::Wait { .. } | Event::PixelWait { .. } | Event::FindImage { .. } | Event::Text { .. } => {}
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
    thread: Option<JoinHandle<RunReport>>,
}

impl EngineHandle {
    pub fn send(&self, cmd: EngineCmd) {
        let _ = self.tx.send(cmd);
        (self.wake)();
    }

    /// Stops playback, releases held input, waits for the thread and
    /// returns what it did so far (`None` if it panicked).
    pub fn stop(mut self) -> Option<RunReport> {
        self.send(EngineCmd::Stop);
        self.thread.take().and_then(|t| t.join().ok())
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

/// Where the playhead is, sent about 30 times a second and after every command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tick {
    pub t: f64,
    /// False while paused, waiting for a check, or finished.
    pub advancing: bool,
    pub speed: f64,
    pub loop_idx: u32,
    /// `None`: forever.
    pub loops: Option<u32>,
}

/// Where a running engine reports to (the app's coordinator, the player's window).
pub trait PlaybackSink: Send + 'static {
    fn tick(&self, tick: Tick);
    fn notice(&self, message: String);
    /// The engine ended on its own (not by [`EngineCmd::Stop`]); a panic ends in `Error`.
    fn done(&self, reason: FinishReason, timed_out_at: Option<Ms>);
}

/// Reports the end to the sink when the engine ends on its own, even by panicking.
struct Done<S: PlaybackSink> {
    sink: S,
    /// `None` until the engine ends; a panic leaves it `None` → `Error`.
    outcome: Option<FinishReason>,
    /// The recorded time of the step whose pixel check timed out.
    timed_out_at: Option<Ms>,
    /// Stopped by the coordinator, which already knows.
    stopped: bool,
}

impl<S: PlaybackSink> Drop for Done<S> {
    fn drop(&mut self) {
        if self.stopped {
            return;
        }
        let reason = self.outcome.take().unwrap_or(FinishReason::Error);
        self.sink.done(reason, self.timed_out_at);
    }
}

/// Runs `plan` on a new engine thread, reporting to `sink`.
pub fn spawn<S: PlaybackSink>(plan: PlayPlan, platform: &Platform, sink: S) -> EngineHandle {
    let (tx, rx) = unbounded::<EngineCmd>();
    let (wake_tx, wake_rx) = crossbeam_channel::bounded(1);
    let (make_timer, make_injector, now_ms) = (platform.timer, platform.injector, platform.now_ms);
    let screen = platform.screen.clone();
    let platform_windows = platform.windows.clone();
    let clipboard = platform.clipboard.clone();
    let thread = std::thread::Builder::new()
        .name("relay-engine".into())
        .spawn(move || {
            let mut done = Done { sink, outcome: None, timed_out_at: None, stopped: false };
            // Created on this thread: the timer also raises this thread's priority.
            let mut timer = make_timer();
            let _ = wake_tx.send(timer.waker());
            let own_window = plan.own_window;
            let (finder_screen, windows) = (screen.clone(), platform_windows);
            let pixel: PixelReader = Box::new(move |x, y| screen.pixel(x, y));
            let find: ImageFinder = Box::new(move |image, area, threshold| {
                // Where Relay's window is now: it may have moved since playback started.
                let hide = crate::finder::Hidden::of(&*windows, own_window);
                crate::finder::find_on_screen(&*finder_screen, image, area, threshold, &hide)
            });
            let fill: TextFiller = Box::new(move |template, n| {
                relay_core::text::fill(template, n, chrono::Local::now().naive_local(), || clipboard.text())
            });
            let mut engine = Engine::new(plan, make_injector(), pixel, find, fill, now_ms());
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
                            return engine.report(now); // dropping the engine releases input
                        }
                        Err(TryRecvError::Empty) => break,
                    }
                    next_tick = f64::MIN; // report the change right away
                }
                let now = now_ms();
                let finished = engine.advance(now);
                if let Some(e) = engine.unreported_error() {
                    done.sink.notice(format!("Playback: {e}"));
                }
                if now >= next_tick || finished.is_some() {
                    next_tick = now + TICK_MS;
                    done.sink.tick(Tick {
                        t: engine.macro_time(now),
                        advancing: !engine.paused() && finished.is_none(),
                        speed: engine.speed(),
                        loop_idx: engine.loop_idx(),
                        loops: engine.loops(),
                    });
                }
                if let Some(reason) = finished {
                    if reason == FinishReason::PixelTimeout
                        && let Some(message) = engine.timeout_notice()
                    {
                        done.sink.notice(message);
                    }
                    done.outcome = Some(reason);
                    done.timed_out_at = engine.timed_out_at();
                    return engine.report(now);
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
        fn text(&mut self, text: &str) -> relay_platform::Result<()> {
            self.0.lock().unwrap().push(format!("type {text}"));
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

    /// Fills in `{n}` only: enough to see which repeat typed what.
    fn filler() -> TextFiller {
        Box::new(|template, n| template.replace("{n}", &n.to_string()))
    }

    fn plan(events: Vec<Event>, repeat: Repeat, speed: f64) -> PlayPlan {
        let steps = relay_core::steps::group_steps(&events, Default::default());
        let duration = relay_core::timeline::duration(&events);
        PlayPlan {
            events,
            steps,
            duration,
            repeat,
            speed,
            jitter_ms: 0,
            seed: 0,
            offset: (0, 0),
            from: 0,
            own_window: 0,
        }
    }

    fn engine_for(plan: PlayPlan) -> (Engine, Recorder) {
        let rec = Recorder::default();
        (Engine::new(plan, Box::new(rec.clone()), Box::new(|_, _| None), Box::new(|_, _, _| None), filler(), 0.0), rec)
    }

    fn engine(events: Vec<Event>, repeat: Repeat, speed: f64) -> (Engine, Recorder) {
        engine_for(plan(events, repeat, speed))
    }

    #[test]
    fn injects_on_schedule_and_measures_lateness() {
        let (mut e, rec) =
            engine(vec![key(0, "KeyA", true), key(100, "KeyA", false), key(200, "KeyB", true)], Repeat::Count(1), 1.0);
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

    /// A macro with a click at 1000 ms, after a key press.
    fn click_at_1000() -> Vec<Event> {
        vec![key(0, "KeyA", true), key(50, "KeyA", false), btn(1000, true), btn(1080, false)]
    }

    /// Runs `e` from `now` to its end; returns what it injected.
    fn run_out(e: &mut Engine, rec: &Recorder, mut now: f64) -> Vec<String> {
        while e.advance(now).is_none() {
            now += 10.0;
            assert!(now < 100_000.0, "never finished");
        }
        rec.take()
    }

    #[test]
    fn with_humanize_starting_at_a_step_plays_all_of_it() {
        for seed in 0..50 {
            let p = PlayPlan { jitter_ms: 200, seed, from: 1000, ..plan(click_at_1000(), Repeat::Count(1), 1.0) };
            let (mut e, rec) = engine_for(p);
            let actions = run_out(&mut e, &rec, 0.0);
            assert_eq!(actions, ["move 10,20", "Left down", "move 10,20", "Left up"], "seed {seed}");
        }
    }

    #[test]
    fn with_humanize_seeking_to_a_step_plays_all_of_it() {
        for seed in 0..50 {
            let p = PlayPlan { jitter_ms: 200, seed, ..plan(click_at_1000(), Repeat::Count(1), 1.0) };
            let (mut e, rec) = engine_for(p);
            e.advance(0.0);
            e.seek(1000.0, 1.0);
            let actions = run_out(&mut e, &rec, 1.0);
            let after_seek: Vec<_> = actions.iter().skip_while(|a| !a.starts_with("move")).collect();
            assert_eq!(after_seek, ["move 10,20", "Left down", "move 10,20", "Left up"], "seed {seed}: {actions:?}");
        }
    }

    #[test]
    fn pause_seek_and_speed_changes() {
        let (mut e, rec) = engine(
            vec![key(0, "KeyA", true), key(1000, "KeyA", false), key(2000, "KeyB", true)],
            Repeat::Count(1),
            1.0,
        );
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
        // KeyQ is pressed and never released: held across the loop boundary.
        let (mut e, rec) = engine(vec![btn(0, true), btn(100, false), key(200, "KeyQ", true)], Repeat::Count(2), 1.0);
        e.advance(0.0);
        e.advance(200.0);
        assert_eq!(rec.take(), ["move 10,20", "Left down", "move 10,20", "Left up", "KeyQ down"]);
        // The macro lasts 700 ms: the loop end releases the key before replaying.
        assert_eq!(e.advance(700.0), None);
        assert_eq!(e.loop_idx(), 1);
        assert_eq!(rec.take(), ["KeyQ up"]);
        e.advance(700.0);
        assert_eq!(rec.take(), ["move 10,20", "Left down"]);
        e.advance(900.0);
        assert_eq!(rec.take(), ["move 10,20", "Left up", "KeyQ down"]);
        assert_eq!(e.advance(1400.0), Some(FinishReason::Completed));
        assert_eq!(rec.take(), ["KeyQ up"]);
    }

    #[test]
    fn repeat_forever_never_finishes() {
        let (mut e, rec) = engine(vec![key(0, "KeyA", true), key(100, "KeyA", false)], Repeat::Forever, 1.0);
        assert_eq!(e.loops(), None);
        for i in 0..200 {
            let start = i as f64 * 600.0;
            assert_eq!(e.advance(start), None);
            assert_eq!(e.advance(start + 100.0), None);
            assert_eq!(e.loop_idx(), i);
        }
        let actions = rec.take();
        assert_eq!(actions.len(), 400);
        assert!(actions.chunks(2).all(|c| c == ["KeyA down", "KeyA up"]));
    }

    #[test]
    fn starting_at_the_end_starts_over_but_near_the_end_doesnt() {
        let events = vec![key(0, "KeyA", true), key(100, "KeyA", false)];
        // 600 ms long; the UI sends its playhead, at most the duration.
        for from in [599, 600] {
            let (mut e, rec) = engine_for(PlayPlan { from, ..plan(events.clone(), Repeat::Count(1), 1.0) });
            e.advance(0.0);
            assert_eq!(rec.take(), ["KeyA down"], "from {from}");
            assert_eq!(e.macro_time(0.0), 0.0);
        }
        let (mut e, rec) = engine_for(PlayPlan { from: 598, ..plan(events, Repeat::Count(1), 1.0) });
        assert_eq!(e.advance(0.0), None);
        assert!(rec.take().is_empty());
        assert_eq!(e.advance(2.0), Some(FinishReason::Completed));
    }

    #[test]
    fn dropping_mid_drag_releases_the_button() {
        let (mut e, rec) =
            engine(vec![btn(0, true), Event::Move { t: 50, x: 90, y: 20 }, btn(100, false)], Repeat::Count(1), 1.0);
        e.advance(60.0);
        drop(e);
        assert_eq!(rec.take(), ["move 10,20", "Left down", "move 90,20", "Left up"]);
    }

    /// Fails every injection, counting the attempts.
    struct Failing(Arc<Mutex<u32>>);

    impl Failing {
        fn fail(&mut self) -> relay_platform::Result<()> {
            let mut n = self.0.lock().unwrap();
            *n += 1;
            Err(relay_platform::PlatformError::Os(format!("blocked #{n}")))
        }
    }

    impl Injector for Failing {
        fn move_to(&mut self, _: i32, _: i32) -> relay_platform::Result<()> {
            self.fail()
        }
        fn button(&mut self, _: MouseBtn, _: bool) -> relay_platform::Result<()> {
            self.fail()
        }
        fn wheel(&mut self, _: i32, _: bool) -> relay_platform::Result<()> {
            self.fail()
        }
        fn key(&mut self, _: &KeyStroke, _: bool, _: Option<&str>) -> relay_platform::Result<()> {
            self.fail()
        }
        fn text(&mut self, _: &str) -> relay_platform::Result<()> {
            self.fail()
        }
    }

    #[test]
    fn an_injection_error_is_reported_once() {
        let calls = Arc::new(Mutex::new(0));
        let events = vec![key(0, "KeyA", true), key(100, "KeyA", false), key(200, "KeyB", true)];
        let injector = Box::new(Failing(calls.clone()));
        let mut e = Engine::new(
            plan(events, Repeat::Count(1), 1.0),
            injector,
            Box::new(|_, _| None),
            Box::new(|_, _, _| None),
            filler(),
            0.0,
        );
        assert_eq!(e.unreported_error(), None, "nothing failed yet");
        e.advance(0.0);
        assert_eq!(e.unreported_error().as_deref(), Some("blocked #1"));
        assert_eq!(e.unreported_error(), None);
        e.advance(250.0);
        assert_eq!(*calls.lock().unwrap(), 3, "playback goes on");
        assert_eq!(e.unreported_error(), None, "later failures aren't reported again");
    }

    fn pixel_macro() -> Vec<Event> {
        vec![
            Event::PixelWait {
                t: 100,
                dur: 900,
                x: 5,
                y: 6,
                color: Rgb(255, 0, 0),
                tolerance: 8,
                timeout_ms: 5000,
                label: String::new(),
            },
            key(1000, "KeyA", true),
            key(1040, "KeyA", false),
        ]
    }

    fn pixel_engine(turns_red_at: f64) -> (Engine, Recorder, Arc<Mutex<f64>>) {
        pixel_engine_for(pixel_macro(), turns_red_at)
    }

    fn pixel_engine_for(events: Vec<Event>, turns_red_at: f64) -> (Engine, Recorder, Arc<Mutex<f64>>) {
        let rec = Recorder::default();
        let steps = relay_core::steps::group_steps(&events, Default::default());
        let duration = relay_core::timeline::duration(&events);
        let plan = PlayPlan {
            events,
            steps,
            duration,
            repeat: Repeat::Count(1),
            speed: 1.0,
            jitter_ms: 0,
            seed: 0,
            offset: (0, 0),
            from: 0,
            own_window: 0,
        };
        // The fake screen turns red at `turns_red_at` (wall ms), read through a shared clock.
        let now = Arc::new(Mutex::new(0.0));
        let clock = now.clone();
        let pixel: PixelReader = Box::new(move |x, y| {
            assert_eq!((x, y), (5, 6));
            Some(if *clock.lock().unwrap() >= turns_red_at { Rgb(250, 4, 2) } else { Rgb(255, 255, 255) })
        });
        (Engine::new(plan, Box::new(rec.clone()), pixel, Box::new(|_, _, _| None), filler(), 0.0), rec, now)
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
    fn pixel_check_times_out_with_its_step_number_and_releases_what_is_held() {
        // KeyQ is held into the check.
        let mut events = vec![key(50, "KeyQ", true)];
        events.extend(pixel_macro());
        let (mut e, rec, now) = pixel_engine_for(events, f64::INFINITY);
        run_to(&mut e, &now, 100.0);
        assert_eq!(rec.take(), ["KeyQ down"]);
        assert_eq!(run_to(&mut e, &now, 5000.0), None);
        assert_eq!(run_to(&mut e, &now, 5100.0), Some(FinishReason::PixelTimeout));
        assert_eq!(e.timed_out_step, Some(2));
        assert_eq!(rec.take(), ["KeyQ up"]);
    }

    #[test]
    fn seeking_out_of_a_pixel_check_continues_from_there() {
        let (mut e, rec, now) = pixel_engine(f64::INFINITY);
        run_to(&mut e, &now, 100.0);
        assert!(e.paused());
        e.seek(1000.0, 300.0);
        assert!(!e.paused(), "the jump leaves the check");
        assert_eq!(e.macro_time(300.0), 1000.0);
        run_to(&mut e, &now, 300.0);
        assert_eq!(rec.take(), ["KeyA down"]);
        // Back before the check: it waits there again, with a fresh timeout.
        e.seek(0.0, 400.0);
        assert_eq!(rec.take(), ["KeyA up"], "the seek released the key");
        assert_eq!(run_to(&mut e, &now, 450.0), None);
        assert!(!e.paused());
        run_to(&mut e, &now, 500.0);
        assert!(e.paused(), "the check waits again");
        assert_eq!(e.macro_time(9000.0), 100.0);
        assert_eq!(run_to(&mut e, &now, 5490.0), None);
        assert_eq!(run_to(&mut e, &now, 5530.0), Some(FinishReason::PixelTimeout));
    }

    #[test]
    fn seeking_out_of_a_pixel_check_while_paused_stays_paused() {
        let (mut e, rec, now) = pixel_engine(f64::INFINITY);
        run_to(&mut e, &now, 100.0);
        e.pause(200.0);
        e.seek(1000.0, 300.0);
        assert!(e.paused());
        assert_eq!(e.next_deadline(), None);
        assert_eq!(run_to(&mut e, &now, 60_000.0), None, "no timeout: the check was left");
        assert!(rec.take().is_empty());
        e.resume(60_000.0);
        assert_eq!(e.macro_time(60_000.0), 1000.0);
        run_to(&mut e, &now, 60_000.0);
        assert_eq!(rec.take(), ["KeyA down"]);
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
    fn a_timed_out_check_tells_its_steps_time() {
        let (mut e, _rec, now) = pixel_engine(f64::INFINITY);
        assert_eq!(e.timed_out_at(), None);
        run_to(&mut e, &now, 100.0);
        assert_eq!(run_to(&mut e, &now, 5100.0), Some(FinishReason::PixelTimeout));
        assert_eq!(e.timed_out_at(), Some(100), "where the playhead stays");
    }

    #[test]
    fn window_offset_moves_every_position() {
        let rec = Recorder::default();
        let events = vec![
            Event::Move { t: 0, x: 100, y: 100 },
            Event::Button { t: 10, x: 50, y: 60, btn: MouseBtn::Left, down: true, label: String::new() },
            Event::Button { t: 20, x: 50, y: 60, btn: MouseBtn::Left, down: false, label: String::new() },
            Event::Wheel { t: 30, x: 70, y: 80, delta: -120, horizontal: false },
            Event::PixelWait {
                t: 40,
                dur: 10,
                x: 5,
                y: 6,
                color: Rgb(255, 0, 0),
                tolerance: 0,
                timeout_ms: 1000,
                label: String::new(),
            },
        ];
        let plan = PlayPlan { offset: (30, -10), ..plan(events, Repeat::Count(1), 1.0) };
        let reads = Arc::new(Mutex::new(Vec::new()));
        let log = reads.clone();
        let pixel: PixelReader = Box::new(move |x, y| {
            log.lock().unwrap().push((x, y));
            Some(Rgb(255, 0, 0))
        });
        let mut e = Engine::new(plan, Box::new(rec.clone()), pixel, Box::new(|_, _, _| None), filler(), 0.0);
        e.advance(100.0);
        assert_eq!(
            rec.take(),
            ["move 130,90", "move 80,50", "Left down", "move 80,50", "Left up", "move 100,70", "wheel -120"]
        );
        assert_eq!(*reads.lock().unwrap(), [(35, -4)], "the pixel check follows the window too");
        assert!(!e.paused(), "and matched there");
    }

    fn png() -> relay_core::model::ImagePng {
        let img = Rgb8 { w: 20, h: 10, px: (0..600).map(|i| (i * 53 % 256) as u8).collect() };
        relay_core::model::ImagePng(img.encode_png())
    }

    fn find_image(t: Ms, area: Option<Rect>) -> Event {
        Event::FindImage {
            t,
            dur: 800,
            image: png(),
            click_x: 10,
            click_y: 4,
            btn: MouseBtn::Right,
            threshold: 85,
            timeout_ms: 5000,
            area,
            label: String::new(),
        }
    }

    /// Where the fake finder is asked to look.
    type Searches = Arc<Mutex<Vec<(usize, Option<Rect>, f32)>>>;

    /// An engine whose fake screen shows the image (at 1.5 times its size,
    /// at 300, 200) from `appears_at` (wall ms).
    fn image_engine(
        events: Vec<Event>,
        offset: (i32, i32),
        appears_at: f64,
    ) -> (Engine, Recorder, Arc<Mutex<f64>>, Searches) {
        let rec = Recorder::default();
        let plan = PlayPlan { offset, ..plan(events, Repeat::Count(1), 1.0) };
        let now = Arc::new(Mutex::new(0.0));
        let (clock, searches) = (now.clone(), Searches::default());
        let log = searches.clone();
        let find: ImageFinder = Box::new(move |image, area, threshold| {
            log.lock().unwrap().push((image.w, area, threshold));
            let m = Match { x: 300, y: 200, w: 30, h: 15, scale: 1.5, score: 0.97 };
            (*clock.lock().unwrap() >= appears_at).then_some(m)
        });
        (Engine::new(plan, Box::new(rec.clone()), Box::new(|_, _| None), find, filler(), 0.0), rec, now, searches)
    }

    #[test]
    fn find_image_waits_for_the_image_then_clicks_it_and_continues() {
        let events = vec![find_image(100, None), key(900, "KeyA", true), key(940, "KeyA", false)];
        let (mut e, rec, now, searches) = image_engine(events, (0, 0), 1100.0);
        assert_eq!(run_to(&mut e, &now, 100.0), None);
        assert!(e.paused(), "the playhead freezes at the step");
        assert_eq!(e.next_deadline(), Some(350.0), "it looks every 250 ms");
        assert_eq!(run_to(&mut e, &now, 350.0), None);
        assert_eq!(*searches.lock().unwrap(), [(20, None, 0.85), (20, None, 0.85)], "the decoded image, everywhere");
        assert!(rec.take().is_empty());
        // Found at 1100: the click point (10, 4) scales with the image.
        // The key is due where the step ends, so it follows right away.
        run_to(&mut e, &now, 1100.0);
        assert_eq!(rec.take(), ["move 315,206", "Right down", "Right up", "KeyA down"]);
        assert!(!e.paused());
        assert_eq!(e.macro_time(1100.0), 900.0, "on from the end of the step");
    }

    #[test]
    fn find_image_times_out_with_its_own_notice() {
        let (mut e, rec, now, _) = image_engine(vec![btn(0, true), find_image(100, None)], (0, 0), f64::INFINITY);
        run_to(&mut e, &now, 100.0);
        assert_eq!(rec.take(), ["move 10,20", "Left down"]);
        assert_eq!(run_to(&mut e, &now, 5050.0), None);
        assert_eq!(run_to(&mut e, &now, 5100.0), Some(FinishReason::PixelTimeout));
        assert_eq!(rec.take(), ["Left up"], "what was held is released");
        assert_eq!(e.timeout_notice().as_deref(), Some("Image not found at step 2; playback stopped."));
        assert_eq!(e.timed_out_at(), Some(100));
        let (mut e, _, now) = pixel_engine(f64::INFINITY);
        run_to(&mut e, &now, 100.0);
        run_to(&mut e, &now, 5100.0);
        assert_eq!(e.timeout_notice().as_deref(), Some("Pixel check timed out at step 1; playback stopped."));
    }

    #[test]
    fn find_image_looks_in_its_area_moved_with_the_window() {
        let area = Rect { x: 100, y: 50, w: 400, h: 300 };
        let (mut e, _, now, searches) = image_engine(vec![find_image(0, Some(area))], (30, -10), 0.0);
        run_to(&mut e, &now, 0.0);
        assert_eq!(searches.lock().unwrap()[0].1, Some(Rect { x: 130, y: 40, w: 400, h: 300 }));
    }

    #[test]
    fn an_unreadable_image_is_never_found() {
        let mut ev = find_image(0, None);
        if let Event::FindImage { image, .. } = &mut ev {
            image.0.truncate(20);
        }
        let (mut e, rec, now, searches) = image_engine(vec![ev], (0, 0), 0.0);
        assert_eq!(run_to(&mut e, &now, 0.0), None);
        assert!(searches.lock().unwrap().is_empty() && rec.take().is_empty());
        assert_eq!(run_to(&mut e, &now, 5000.0), Some(FinishReason::PixelTimeout));
    }

    fn outcomes(report: &RunReport) -> Vec<(u32, u32, bool, u32, CheckOutcome)> {
        let (checks, _) = report.checks.clone().into_parts();
        checks.iter().map(|c| (c.step, c.loop_idx, c.image, c.after_ms, c.outcome)).collect()
    }

    #[test]
    fn a_matched_pixel_check_is_reported_with_how_long_it_waited() {
        let (mut e, _rec, now) = pixel_engine(1200.0);
        run_to(&mut e, &now, 100.0);
        run_to(&mut e, &now, 1190.0);
        run_to(&mut e, &now, 1220.0);
        run_to(&mut e, &now, 1600.0);
        let report = e.report(1600.0);
        assert_eq!(outcomes(&report), [(1, 0, false, 1120, CheckOutcome::Matched)]);
        assert_eq!(report.loops, 1);
        assert!(report.timing.is_some());
    }

    #[test]
    fn a_timed_out_check_is_reported_once() {
        let (mut e, _rec, now) = pixel_engine(f64::INFINITY);
        run_to(&mut e, &now, 100.0);
        assert_eq!(run_to(&mut e, &now, 5100.0), Some(FinishReason::PixelTimeout));
        assert_eq!(outcomes(&e.report(5100.0)), [(1, 0, false, 5000, CheckOutcome::TimedOut)]);
    }

    #[test]
    fn a_found_image_is_reported_where_it_was_found() {
        let events = vec![find_image(100, None), key(900, "KeyA", true)];
        let (mut e, _rec, now, _) = image_engine(events, (0, 0), 1100.0);
        run_to(&mut e, &now, 100.0);
        run_to(&mut e, &now, 1100.0);
        let found = CheckOutcome::Found { x: 300, y: 200, score: 97 };
        assert_eq!(outcomes(&e.report(1100.0)), [(1, 0, true, 1000, found)]);
    }

    #[test]
    fn stopping_during_a_check_reports_it_interrupted_without_the_pause() {
        let (mut e, _rec, now) = pixel_engine(f64::INFINITY);
        run_to(&mut e, &now, 100.0);
        e.pause(400.0);
        assert_eq!(outcomes(&e.report(9000.0)), [(1, 0, false, 300, CheckOutcome::Interrupted)]);
        // Stopped outside a check: nothing to add.
        let (mut e, _rec, now) = pixel_engine(f64::INFINITY);
        run_to(&mut e, &now, 50.0);
        assert!(outcomes(&e.report(50.0)).is_empty());
    }

    #[test]
    fn each_loop_reports_its_own_checks() {
        let rec = Recorder::default();
        let pixel: PixelReader = Box::new(|_, _| Some(Rgb(255, 0, 0)));
        let mut e = Engine::new(
            plan(pixel_macro(), Repeat::Count(2), 1.0),
            Box::new(rec),
            pixel,
            Box::new(|_, _, _| None),
            filler(),
            0.0,
        );
        let mut t = 0.0;
        while e.advance(t).is_none() {
            t += 10.0;
            assert!(t < 10_000.0, "never finished");
        }
        let report = e.report(t);
        let loops: Vec<u32> = outcomes(&report).iter().map(|c| c.1).collect();
        assert_eq!(loops, [0, 1]);
        assert_eq!(report.loops, 2);
    }

    fn text(t: Ms, dur: Ms, text: &str) -> Event {
        Event::Text { t, dur, text: text.into() }
    }

    #[test]
    fn a_text_is_typed_a_character_at_a_time_then_playback_goes_on() {
        let (mut e, rec) = engine(vec![text(100, 500, "ab{n}"), key(600, "KeyA", true)], Repeat::Count(1), 1.0);
        assert_eq!(e.advance(100.0), None);
        assert_eq!(rec.take(), ["type a"]);
        assert!(e.paused(), "the playhead waits at the step while it types");
        assert_eq!(e.next_deadline(), Some(110.0));
        e.advance(115.0);
        assert_eq!(rec.take(), ["type b"]);
        e.advance(120.0);
        assert_eq!(rec.take(), ["type 1"]);
        // Typed by 130: the step goes on from 30 ms in, so what follows keeps its time.
        e.advance(130.0);
        assert!(!e.paused());
        assert_eq!(e.macro_time(130.0), 130.0);
        assert_eq!(e.next_deadline(), Some(600.0));
        e.advance(600.0);
        assert_eq!(rec.take(), ["KeyA down"]);
    }

    #[test]
    fn a_text_longer_than_its_step_pushes_what_follows_back() {
        let (mut e, rec) = engine(vec![text(0, 20, "abcde"), key(20, "KeyA", true)], Repeat::Count(1), 1.0);
        for t in [0.0, 10.0, 20.0, 30.0, 40.0] {
            e.advance(t);
        }
        assert_eq!(rec.take(), ["type a", "type b", "type c", "type d", "type e"]);
        e.advance(50.0);
        assert_eq!(rec.take(), ["KeyA down"], "right after the text, from the step's end");
        assert_eq!(e.macro_time(50.0), 20.0);
    }

    #[test]
    fn each_repeat_fills_in_its_own_number() {
        let (mut e, rec) = engine(vec![text(0, 100, "#{n}")], Repeat::Count(2), 1.0);
        let typed: Vec<_> = run_out(&mut e, &rec, 0.0).into_iter().filter(|a| a.starts_with("type")).collect();
        assert_eq!(typed, ["type #", "type 1", "type #", "type 2"]);
    }

    #[test]
    fn typing_follows_the_speed_and_stops_while_paused() {
        let (mut e, rec) = engine(vec![text(0, 100, "abc")], Repeat::Count(1), 2.0);
        e.advance(0.0);
        assert_eq!(e.next_deadline(), Some(5.0), "twice as fast");
        e.pause(3.0);
        assert_eq!(e.next_deadline(), None);
        assert_eq!(e.advance(500.0), None);
        assert_eq!(rec.take(), ["type a"]);
        e.resume(1000.0);
        assert_eq!(e.next_deadline(), Some(1002.0), "the pause doesn't count");
        e.advance(1002.0);
        assert_eq!(rec.take(), ["type b"]);
    }

    #[test]
    fn seeking_leaves_the_text_untyped() {
        let (mut e, rec) = engine(vec![text(0, 100, "abc"), key(500, "KeyA", true)], Repeat::Count(1), 1.0);
        e.advance(0.0);
        e.seek(400.0, 5.0);
        assert!(!e.paused());
        e.advance(105.0);
        assert_eq!(rec.take(), ["type a", "KeyA down"]);
    }

    #[test]
    fn an_empty_text_types_nothing() {
        let (mut e, rec) = engine(vec![text(0, 100, ""), key(100, "KeyA", true)], Repeat::Count(1), 1.0);
        e.advance(0.0);
        assert!(rec.take().is_empty());
        assert!(!e.paused());
        e.advance(100.0);
        assert_eq!(rec.take(), ["KeyA down"]);
    }
}
