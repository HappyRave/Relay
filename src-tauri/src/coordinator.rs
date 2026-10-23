//! The coordinator owns the session state machine (relay-core `session`) on a
//! dedicated thread. Buttons, hotkeys, the hook, the engine and the triggers
//! all send it [`Cmd`]s; it applies the transition and performs the effects.
//!
//! It never holds a lock while it calls into the main thread (window, tray,
//! hotkeys), so the main thread can always take the locks it needs.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use parking_lot::{Mutex, RwLock};
use relay_core::model::{Macro, Ms, RecordingMeta, Rect};
use relay_core::runlog::{RunEntry, RunOutcome, SkipReason};
use relay_core::session::{self, Effect, FinishReason, HotkeySet, Input, Mode, RunSource, SessionConfig};
use relay_core::steps::GroupOptions;
use relay_platform::recorder::{Recorder, RecorderConfig, is_meaningful};
use relay_platform::types::MousePulse;
use relay_platform::{HookConfig, HookMode, HookSession, Platform, RawInput, RawKind};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::engine::{self, EngineCmd, EngineHandle, PlayPlan, RunReport, TimingStats};
use crate::hotkeys;
use crate::ipc::{Emitter, EngineMsg};
use crate::library::Library;
use crate::rec_thread::{RecContext, RecThread};
use crate::run_history::RunHistory;
use crate::settings::{Settings, SettingsStore};
use crate::triggers::TriggerState;

/// F9 stops a recording through its global hotkey; keep it out of the macro.
const VK_F9: u16 = 0x78;
/// F10 pauses playback through its global hotkey; it must not count as "any key".
const VK_F10: u16 = 0x79;
/// How often the countdown before a recording is reported.
const COUNTDOWN_TICK: Duration = Duration::from_millis(50);

pub enum Cmd {
    Input(Input),
    /// F10: play from wherever the UI's playhead was left.
    HotkeyPlay,
    /// Esc, reported by the hook during a session.
    Escape,
    /// Another key during playback with "stop on key press".
    StopKey,
    /// Playback `generation` ended on its own: completed, a pixel check
    /// timed out, or the engine failed.
    EngineDone {
        generation: u64,
        reason: FinishReason,
        /// With `PixelTimeout`: the recorded time of the check's step.
        timed_out_at: Option<Ms>,
    },
    /// The UI selected a macro.
    Select(Uuid),
    Seek(f64),
    /// Macro `id`'s speed changed (applies if it's the one playing).
    Speed {
        id: Uuid,
        speed: f64,
    },
    /// A trigger (or a macro hotkey) wants to run a macro.
    RunMacro {
        id: Uuid,
        source: RunSource,
    },
    /// A trigger that won't run, for the run history (a missed schedule).
    LogSkip {
        id: Uuid,
        source: RunSource,
        reason: SkipReason,
    },
    /// From the tray or the Triggers tab.
    SetTriggersPaused(bool),
    /// The recorder's watchdog saw the cursor move without hook events.
    HookLost,
    /// Relay is quitting: stop what's running (saving a recording) and reply.
    Shutdown(Sender<()>),
}

/// The current session mode, readable by commands (e.g. to refuse deleting
/// a macro while it plays). Updated as soon as a transition is decided.
pub struct SessionMode(RwLock<Mode>);

impl Default for SessionMode {
    fn default() -> Self {
        SessionMode(RwLock::new(Mode::Idle))
    }
}

impl SessionMode {
    pub fn is_idle(&self) -> bool {
        *self.0.read() == Mode::Idle
    }

    fn set(&self, mode: Mode) {
        *self.0.write() = mode;
    }
}

#[derive(Clone)]
pub struct CoordinatorHandle(Sender<Cmd>);

impl CoordinatorHandle {
    pub fn send(&self, cmd: Cmd) {
        let _ = self.0.send(cmd);
    }

    /// Stops the running session (so no key stays held and a recording is
    /// saved) and waits for it, up to `timeout`.
    pub fn shutdown(&self, timeout: Duration) {
        let (done, wait) = crossbeam_channel::bounded(1);
        self.send(Cmd::Shutdown(done));
        let _ = wait.recv_timeout(timeout);
    }
}

/// The recording in progress: the hook, how it was configured (to reinstall
/// it if Windows drops it), and the recorder thread.
struct Recording {
    hook: Box<dyn HookSession>,
    hook_cfg: HookConfig,
    raw_tx: Sender<RawInput>,
    thread: RecThread,
    /// The screenshot taken as it started (a JPEG) and the desktop it shows,
    /// taken and encoded off this thread.
    screen: Option<std::thread::JoinHandle<Option<Screenshot>>>,
}

/// A JPEG of the desktop, and the desktop's rect when it was taken.
type Screenshot = (Rect, Vec<u8>);

/// The screenshot to keep with a new recording: only one of the desktop the
/// macro was recorded on (a monitor plugged in or out meanwhile would put it
/// in the wrong place), and none if none was taken (the setting was off, or
/// the screen couldn't be read).
fn screenshot_to_save(shot: Option<Screenshot>, desktop: Rect) -> Option<Vec<u8>> {
    shot.filter(|(area, _)| *area == desktop).map(|(_, jpeg)| jpeg)
}

/// The playback in progress.
struct Playback {
    engine: EngineHandle,
    generation: u64,
    /// The macro's length, where a completed run leaves the playhead.
    duration: Ms,
    /// Watches for Esc and stop keys; `None` if it couldn't start.
    _hook: Option<Box<dyn HookSession>>,
    /// Whether the window was made click-through for this playback.
    click_through: bool,
    run: RunStart,
}

/// How a playback started, for its run history entry.
#[derive(Debug, Clone, PartialEq)]
struct RunStart {
    at: DateTime<Utc>,
    /// In `now_ms` time.
    started: f64,
    macro_id: Uuid,
    macro_name: String,
    source: RunSource,
    from: Ms,
    speed: f32,
    humanize: bool,
}

struct Coordinator {
    app: AppHandle,
    platform: Arc<Platform>,
    emit: Arc<Emitter>,
    tx: Sender<Cmd>,
    mode: Mode,
    current: Option<Uuid>,
    idle_playhead: f64,
    /// When the recording countdown ends (in `now_ms` time).
    countdown_until: Option<f64>,
    recording: Option<Recording>,
    playback: Option<Playback>,
    generation: u64,
    /// What starts the next playback: a trigger's source, else `Manual`.
    pending_source: RunSource,
    /// Whether the user was told the run history couldn't be saved (once, until it saves again).
    history_unsaved_told: bool,
}

pub fn spawn(app: AppHandle, platform: Arc<Platform>, emit: Arc<Emitter>) -> CoordinatorHandle {
    let (tx, rx) = unbounded();
    let mut c = Coordinator {
        app,
        platform,
        emit,
        tx: tx.clone(),
        mode: Mode::Idle,
        current: None,
        idle_playhead: 0.0,
        countdown_until: None,
        recording: None,
        playback: None,
        generation: 0,
        pending_source: RunSource::Manual,
        history_unsaved_told: false,
    };
    std::thread::Builder::new().name("relay-coordinator".into()).spawn(move || c.run(rx)).expect("spawn coordinator");
    CoordinatorHandle(tx)
}

impl Coordinator {
    fn run(&mut self, rx: Receiver<Cmd>) {
        hotkeys::set_active(&self.app, HotkeySet::Idle);
        loop {
            let cmd = match self.countdown_until {
                None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
                Some(_) => rx.recv_timeout(COUNTDOWN_TICK),
            };
            match cmd {
                Ok(Cmd::Shutdown(done)) => {
                    self.shutdown();
                    let _ = done.send(());
                    return;
                }
                // A bug in one command mustn't take every later session down.
                Ok(cmd) => {
                    if catch_unwind(AssertUnwindSafe(|| self.handle(cmd))).is_err() {
                        self.emit.error("Something went wrong; Relay stopped what it was doing.");
                        self.shutdown();
                        self.set_mode(Mode::Idle);
                        self.effect(Effect::SetHotkeys(HotkeySet::Idle));
                        self.effect(Effect::EmitMode(Mode::Idle));
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            self.tick_countdown();
        }
    }

    fn tick_countdown(&mut self) {
        let Some(until) = self.countdown_until else { return };
        let left = until - (self.platform.now_ms)();
        if left > 0.0 {
            self.emit.send(EngineMsg::Countdown { left_ms: left as u32 });
        } else {
            self.countdown_until = None;
            self.input(Input::CountdownDone);
        }
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Input(input) => self.input(input),
            Cmd::HotkeyPlay => self.input(Input::TogglePlay { from: self.idle_playhead.round() as u32 }),
            Cmd::Escape => self.input(Input::Stop(FinishReason::Stopped)),
            Cmd::StopKey => self.input(Input::Stop(FinishReason::KeyPressed)),
            Cmd::EngineDone { generation, reason, timed_out_at } => {
                // A late message from a playback that was already stopped.
                if !is_current(self.playback.as_ref().map(|p| p.generation), generation) {
                    return;
                }
                let duration = self.playback.as_ref().map_or(0, |p| p.duration);
                let timing = self.finish_playback(reason);
                self.idle_playhead = playhead_after(reason, duration, timed_out_at);
                if counts_as_run(reason) {
                    self.count_run();
                }
                self.emit.send(EngineMsg::Finished { reason, timing });
                self.input(Input::PlaybackFinished(reason));
            }
            Cmd::Select(id) => {
                if self.mode == Mode::Idle {
                    self.current = Some(id);
                    self.idle_playhead = 0.0;
                }
            }
            Cmd::Seek(t) => {
                // Kept during playback too, like the UI's playhead.
                self.idle_playhead = t;
                self.engine_cmd(EngineCmd::Seek(t));
            }
            Cmd::Speed { id, speed } => {
                if self.current == Some(id) {
                    self.engine_cmd(EngineCmd::Speed(speed));
                }
            }
            Cmd::RunMacro { id, source } => self.run_triggered(id, source),
            Cmd::LogSkip { id, source, reason } => {
                let name = self.app.state::<Mutex<Library>>().lock().get(id).map(|e| e.macro_.name.clone());
                if let Some(name) = name {
                    self.record_run(skip_entry(id, name, source, reason, Utc::now()));
                }
            }
            Cmd::SetTriggersPaused(paused) => self.set_triggers_paused(paused),
            Cmd::HookLost => self.reinstall_hook(),
            Cmd::Shutdown(_) => unreachable!("handled in run"),
        }
    }

    fn input(&mut self, input: Input) {
        let cfg = SessionConfig { record_countdown_ms: if self.settings().countdown { 3000 } else { 0 } };
        let (mode, effects) = session::step(self.mode, input.clone(), &cfg);
        if mode != self.mode {
            tracing::info!(from = ?self.mode, to = ?mode, ?input, "session");
            self.set_mode(mode);
        }
        for e in effects {
            self.effect(e);
        }
    }

    /// Every mode change goes through here, so commands see the same mode.
    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.app.state::<SessionMode>().set(mode);
    }

    fn settings(&self) -> Settings {
        self.app.state::<Mutex<SettingsStore>>().lock().current.clone()
    }

    fn effect(&mut self, effect: Effect) {
        match effect {
            Effect::StartCountdown { ms } => {
                self.countdown_until = Some((self.platform.now_ms)() + ms as f64);
                self.tick_countdown();
            }
            Effect::CancelCountdown => self.countdown_until = None,
            Effect::StartRecording => self.start_recording(),
            Effect::StopRecording => self.stop_recording(),
            Effect::StartPlayback { from } => self.start_playback(from),
            Effect::PausePlayback => self.engine_cmd(EngineCmd::Pause),
            Effect::ResumePlayback => self.engine_cmd(EngineCmd::Resume),
            Effect::StopPlayback(reason) => {
                let timing = self.finish_playback(reason);
                self.idle_playhead = playhead_after(reason, 0, None);
                self.emit.send(EngineMsg::Finished { reason, timing });
            }
            Effect::SetHotkeys(set) => hotkeys::set_active(&self.app, set),
            Effect::PauseTriggers => {
                if !self.app.state::<TriggerState>().paused() {
                    self.set_triggers_paused(true);
                    self.emit.send(EngineMsg::Notice {
                        message: "Triggers are paused. Resume them from the tray or the Triggers tab.".into(),
                    });
                }
            }
            Effect::EmitMode(mode) => {
                let session = mode != Mode::Idle;
                crate::tray::set_mode(&self.app, mode);
                if let Some(w) = self.app.get_webview_window("main") {
                    crate::window_ctl::set_no_activate(&w, session);
                    crate::window_ctl::apply_on_top(&w, self.settings().keep_on_top, session);
                }
                self.emit.send(EngineMsg::Session { mode, macro_id: self.current });
            }
        }
    }

    /// Stops whatever runs, keeping a recording, before Relay quits.
    fn shutdown(&mut self) {
        self.countdown_until = None;
        if self.recording.is_some() {
            self.stop_recording();
        }
        self.end_playback();
    }

    /// A trigger fired: run its macro now if Relay is free and the screen is usable.
    fn run_triggered(&mut self, id: Uuid, source: RunSource) {
        let paused = self.app.state::<TriggerState>().paused();
        let name = self.app.state::<Mutex<Library>>().lock().get(id).map(|e| e.macro_.name.clone());
        let windows = &self.platform.windows;
        let skipped = match admit(paused, name.as_deref(), self.mode, || windows.input_desktop_available()) {
            Admission::Run => None,
            Admission::Ignore => return,
            Admission::Busy(message) => {
                self.emit.send(EngineMsg::Notice { message });
                Some(SkipReason::Busy)
            }
            Admission::NoDesktop => {
                tracing::info!(%id, ?source, "trigger skipped: the input desktop isn't available (locked?)");
                Some(SkipReason::Locked)
            }
        };
        if let (Some(reason), Some(name)) = (skipped, name) {
            self.record_run(skip_entry(id, name, source, reason, Utc::now()));
            return;
        }
        tracing::info!(%id, ?source, "running a triggered macro");
        self.current = Some(id);
        self.idle_playhead = 0.0;
        self.pending_source = source;
        self.input(Input::Trigger(source));
        self.pending_source = RunSource::Manual;
    }

    fn set_triggers_paused(&mut self, paused: bool) {
        self.app.state::<TriggerState>().set_paused(paused);
        crate::tray::set_triggers_active(&self.app, !paused);
        self.emit.send(EngineMsg::TriggersPaused { paused });
    }

    fn engine_cmd(&self, cmd: EngineCmd) {
        if let Some(p) = &self.playback {
            p.engine.send(cmd);
        }
    }

    /// Undoes what playback set up (engine, watching hook, click-through)
    /// and returns how it started and what it did.
    fn end_playback(&mut self) -> Option<(RunStart, Option<RunReport>)> {
        let p = self.playback.take()?;
        let report = p.engine.stop();
        if p.click_through
            && let Some(w) = self.app.get_webview_window("main")
        {
            let _ = w.set_ignore_cursor_events(false);
        }
        Some((p.run, report))
    }

    /// Ends the playback, adds it to the run history and returns its timing.
    fn finish_playback(&mut self, reason: FinishReason) -> Option<TimingStats> {
        let (start, report) = self.end_playback()?;
        let report = report.unwrap_or_default();
        let timing = report.timing.clone();
        let entry = run_entry(start, reason, report, (self.platform.now_ms)());
        self.record_run(entry);
        timing
    }

    fn record_run(&mut self, entry: RunEntry) {
        let saved = self.app.state::<Mutex<RunHistory>>().lock().add(entry);
        match saved {
            Ok(()) => self.history_unsaved_told = false,
            Err(e) => {
                tracing::warn!("couldn't save the run history: {e}");
                if !self.history_unsaved_told {
                    self.history_unsaved_told = true;
                    self.emit.error(format!("Couldn't save the run history: {e}. New runs are kept until you quit."));
                }
            }
        }
        self.emit.send(EngineMsg::RunsChanged);
    }

    fn count_run(&mut self) {
        let Some(id) = self.current else { return };
        {
            let lib = self.app.state::<Mutex<Library>>();
            let mut lib = lib.lock();
            let Some(e) = lib.get_mut(id) else { return };
            e.runs += 1;
            e.last_run = Some(chrono::Utc::now());
            if let Err(e) = lib.save_stats() {
                tracing::warn!("couldn't save the run count: {e}");
            }
        }
        self.emit.send(EngineMsg::LibraryChanged);
    }

    /// Relay's own window, whose clicks and keys must not be recorded.
    fn own_window(&self) -> (Option<Rect>, isize) {
        let Some(w) = self.app.get_webview_window("main") else { return (None, 0) };
        let rect = match (w.outer_position(), w.outer_size()) {
            (Ok(p), Ok(s)) => Some(Rect { x: p.x, y: p.y, w: s.width as i32, h: s.height as i32 }),
            _ => None,
        };
        (rect, crate::window_ctl::main_hwnd(&self.app))
    }

    fn start_recording(&mut self) {
        let settings = self.settings();
        let (_, own_window) = self.own_window();
        let mouse_pulse = Arc::new(MousePulse::default());
        let hook_cfg = HookConfig {
            mode: HookMode::Record { own_window, skip_vks: vec![VK_F9], esc_stops: settings.esc_stops_recording },
            ignore_injected: settings.ignore_injected,
            mouse_pulse: Some(mouse_pulse.clone()),
        };
        let (raw_tx, raw_rx) = crossbeam_channel::bounded(8192);
        let hook = match self.platform.hook.start(hook_cfg.clone(), raw_tx.clone()) {
            Ok(hook) => hook,
            Err(e) => {
                self.emit.error(format!("Couldn't start recording: {e}"));
                // Queued, so this transition's own effects finish first.
                let _ = self.tx.send(Cmd::Input(Input::Stop(FinishReason::Error)));
                return;
            }
        };
        let (ms, px) = self.platform.screen.double_click();
        let recorder = Recorder::new(
            RecorderConfig {
                capture_moves: settings.capture_moves,
                capture_keys: settings.capture_keys,
                move_interval_ms: 16,
            },
            (self.platform.now_ms)(),
            (self.platform.translator)(),
        );
        let ctx = RecContext {
            screen: self.platform.screen.clone(),
            desktop: self.platform.screen.virtual_desktop(),
            group: GroupOptions::new(ms, px),
            now_ms: self.platform.now_ms,
            emit: self.emit.clone(),
            coordinator: self.tx.clone(),
            mouse_pulse,
        };
        let thread = RecThread::spawn(recorder, raw_rx, ctx);
        let screen = settings.capture_screen.then(|| {
            let screen = self.platform.screen.clone();
            std::thread::spawn(move || {
                let area = screen.virtual_desktop();
                let jpeg = crate::screens::encode(&screen.capture(area, crate::screens::MAX_WIDTH, own_window)?)?;
                Some((area, jpeg))
            })
        });
        self.recording = Some(Recording { hook, hook_cfg, raw_tx, thread, screen });
    }

    /// Windows removes low-level hooks it thinks are too slow, without telling
    /// anyone. Put a fresh one in place, feeding the same recorder.
    fn reinstall_hook(&mut self) {
        let Some(rec) = &mut self.recording else { return };
        tracing::warn!("the input hook stopped delivering events; reinstalling it");
        match self.platform.hook.start(rec.hook_cfg.clone(), rec.raw_tx.clone()) {
            Ok(hook) => {
                rec.hook = hook; // dropping the old session removes the old hooks
                self.emit.send(EngineMsg::Notice {
                    message: "Windows dropped Relay's input hook; it was restarted. Check the last steps.".into(),
                });
            }
            Err(e) => self.emit.error(format!("Recording lost its input hook and couldn't restart it: {e}")),
        }
    }

    fn stop_recording(&mut self) {
        let Some(rec) = self.recording.take() else { return };
        drop(rec.hook);
        drop(rec.raw_tx);
        let snapshot = rec.screen.and_then(|t| t.join().ok().flatten());
        let Some(recording) = rec.thread.finish() else {
            self.emit.error("The recording failed and couldn't be saved.");
            return;
        };
        if !is_meaningful(&recording.events) {
            return;
        }
        let screen = &self.platform.screen;
        let (double_click_ms, double_click_px) = screen.double_click();
        let meta = RecordingMeta {
            os: std::env::consts::OS.into(),
            virtual_desktop: screen.virtual_desktop(),
            monitors: screen.monitors(),
            double_click_ms,
            double_click_px,
            anchor_window: recording.first_press.and_then(|(x, y)| self.platform.windows.root_window_at(x, y)),
        };
        let meta_desktop = meta.virtual_desktop;
        tracing::info!(events = recording.events.len(), ms = recording.duration_ms, "recording saved");
        let id = {
            let lib = self.app.state::<Mutex<Library>>();
            let mut lib = lib.lock();
            let m = Macro::new(lib.next_recording_name(), meta, recording.events);
            let id = m.id;
            // The recording is in the library either way; only the file may be missing.
            if let Err(e) = lib.insert_front(m) {
                self.emit.error(format!("Couldn't save the recording to disk: {e}. It's kept until you quit."));
            }
            id
        };
        // Only if it shows the desktop the macro was recorded on (a monitor
        // plugged in or out meanwhile would misplace it).
        if let Some(jpeg) = screenshot_to_save(snapshot, meta_desktop)
            && let Err(e) = crate::screens::save(&crate::storage::data_dir(&self.app), id, &jpeg)
        {
            self.emit
                .error(format!("Couldn't save the screenshot of this recording: {e}. The preview shows the sketch."));
        }
        self.current = Some(id);
        self.emit.send(EngineMsg::Saved { id });
        self.emit.send(EngineMsg::LibraryChanged);
    }

    fn start_playback(&mut self, from: u32) {
        let lib = self.app.state::<Mutex<Library>>();
        let entry = self.current.and_then(|id| lib.lock().get(id).map(|e| (e.macro_.clone(), e.data_file.clone())));
        let Some((m, data_file)) = entry else {
            return self.refuse_playback("Select a macro to play".into());
        };
        let data = match crate::data_file::for_playing(&m.events, data_file.as_deref()) {
            Ok(data) => data,
            Err(why) => return self.refuse_playback(why),
        };
        let (own_rect, own_window) = self.own_window();
        self.hand_focus_back(own_window);
        let offset = self.window_offset(&m);
        let click_through = self.click_through_if_needed(&m, own_rect, offset);
        let hook = self.watch_for_stop_keys(&m);

        self.generation += 1;
        let run = RunStart {
            at: Utc::now(),
            started: (self.platform.now_ms)(),
            macro_id: m.id,
            macro_name: m.name.clone(),
            source: self.pending_source,
            from,
            speed: m.playback.speed,
            humanize: m.playback.humanize,
        };
        let seed = (self.platform.now_ms)().to_bits() ^ (m.id.as_u128() as u64);
        let plan = PlayPlan::for_macro(m, data, from, seed, offset, own_window);
        let duration = plan.duration;
        let engine = engine::spawn(plan, &self.platform, self.emit.clone(), self.tx.clone(), self.generation);
        self.playback =
            Some(Playback { engine, generation: self.generation, duration, _hook: hook, click_through, run });
    }

    fn refuse_playback(&mut self, why: String) {
        self.emit.error(why);
        let _ = self.tx.send(Cmd::Input(Input::PlaybackFinished(FinishReason::Error)));
    }

    /// Started from Relay's own button: give the keyboard back to the app the
    /// user was working in, or keystrokes would type into Relay. Warns when
    /// Windows will block the input to that app.
    fn hand_focus_back(&self, own_window: isize) {
        let windows = &self.platform.windows;
        let mut target = windows.foreground();
        if target.is_some_and(|w| w.hwnd == own_window) {
            target = windows.restore_previous(own_window);
        }
        if target.is_some_and(|t| windows.input_blocked(t.pid)) {
            self.emit.send(EngineMsg::Notice {
                message: "The app in front runs as administrator, so Windows blocks Relay's input to it. \
                          Run Relay as administrator to automate it."
                    .into(),
            });
        }
    }

    /// Clicks and scrolling under the always-on-top widget must reach the app beneath it.
    fn click_through_if_needed(&self, m: &Macro, own_rect: Option<Rect>, offset: (i32, i32)) -> bool {
        own_rect.is_some_and(|r| relay_playback::plan::clicks_inside(m, r, offset))
            && self.app.get_webview_window("main").is_some_and(|w| w.set_ignore_cursor_events(true).is_ok())
    }

    /// Watches for Esc and, if the macro wants it, any other key.
    fn watch_for_stop_keys(&self, m: &Macro) -> Option<Box<dyn HookSession>> {
        let cfg = HookConfig {
            mode: HookMode::Watch {
                stop_on_key: m.playback.stop_on_key,
                pass_vks: vec![VK_F10],
                report_kill_switch: false,
            },
            ignore_injected: self.settings().ignore_injected,
            mouse_pulse: None,
        };
        let (raw_tx, raw_rx) = crossbeam_channel::bounded(64);
        match self.platform.hook.start(cfg, raw_tx) {
            Ok(hook) => {
                let tx = self.tx.clone();
                std::thread::Builder::new()
                    .name("relay-stop-keys".into())
                    .spawn(move || {
                        for raw in raw_rx {
                            let cmd = match raw.kind {
                                RawKind::Escape => Cmd::Escape,
                                RawKind::StopKey => Cmd::StopKey,
                                _ => continue,
                            };
                            let _ = tx.send(cmd);
                        }
                    })
                    .expect("spawn stop-key thread");
                Some(hook)
            }
            Err(e) => {
                self.emit.error(format!("Esc won't stop playback: {e}"));
                None
            }
        }
    }

    /// For "Window" coordinates: how far the anchor window moved since recording.
    fn window_offset(&self, m: &Macro) -> (i32, i32) {
        let windows = &self.platform.windows;
        let (offset, notice) = relay_playback::plan::window_offset(m, |exe, class| windows.find_window(exe, class));
        if let Some(message) = notice {
            self.emit.send(EngineMsg::Notice { message });
        }
        offset
    }
}

/// What a trigger's request to run a macro gets.
#[derive(Debug, PartialEq)]
enum Admission {
    Run,
    /// Triggers are paused, or the macro is gone: nothing to say.
    Ignore,
    /// A session is running: tell the user the run was skipped.
    Busy(String),
    /// Locked, or a UAC prompt: nothing can be clicked or typed. Skipped quietly.
    NoDesktop,
}

/// Whether a trigger may run macro `name` (`None`: not in the library) now.
/// `desktop_available` is asked last, only when everything else allows the run.
fn admit(paused: bool, name: Option<&str>, mode: Mode, desktop_available: impl FnOnce() -> bool) -> Admission {
    let Some(name) = name.filter(|_| !paused) else { return Admission::Ignore };
    if mode != Mode::Idle {
        return Admission::Busy(format!("Skipped “{name}”: Relay was busy"));
    }
    if !desktop_available() {
        return Admission::NoDesktop;
    }
    Admission::Run
}

/// Whether an `EngineDone` from playback `generation` is about the playback
/// running now (`playing`), not a late one from a playback already stopped.
fn is_current(playing: Option<u64>, generation: u64) -> bool {
    playing == Some(generation)
}

/// The run history entry of a playback that ended at `now` (`now_ms` time).
fn run_entry(start: RunStart, reason: FinishReason, report: RunReport, now: f64) -> RunEntry {
    let (checks, checks_dropped) = report.checks.into_parts();
    RunEntry {
        at: start.at,
        macro_id: start.macro_id,
        macro_name: start.macro_name,
        source: start.source,
        outcome: RunOutcome::Finished(reason),
        duration_ms: (now - start.started).max(0.0).round() as Ms,
        from_ms: start.from,
        loops: report.loops.max(1),
        speed: start.speed,
        humanize: start.humanize,
        checks,
        checks_dropped,
    }
}

/// The run history entry of a trigger that fired but didn't run its macro.
fn skip_entry(id: Uuid, name: String, source: RunSource, reason: SkipReason, at: DateTime<Utc>) -> RunEntry {
    RunEntry {
        at,
        macro_id: id,
        macro_name: name,
        source,
        outcome: RunOutcome::Skipped(reason),
        duration_ms: 0,
        from_ms: 0,
        loops: 0,
        speed: 1.0,
        humanize: false,
        checks: Vec::new(),
        checks_dropped: 0,
    }
}

/// A run counts (runs, last run) only when the macro played to the end.
fn counts_as_run(reason: FinishReason) -> bool {
    reason == FinishReason::Completed
}

/// Where F10 plays from after a playback of `duration` ms ended, mirroring the
/// UI's playhead: a stop rewinds to the start, a completed run stays at the end
/// (so the next play starts over), a timed-out pixel check stays on its step.
fn playhead_after(reason: FinishReason, duration: Ms, timed_out_at: Option<Ms>) -> f64 {
    match reason {
        FinishReason::Completed => duration as f64,
        FinishReason::PixelTimeout => timed_out_at.unwrap_or(0) as f64,
        FinishReason::Stopped | FinishReason::KeyPressed | FinishReason::Killed | FinishReason::Error => 0.0,
    }
}

#[cfg(test)]
mod screenshot_tests {
    use super::*;

    #[test]
    fn a_screenshot_is_kept_only_if_it_shows_the_recordings_desktop() {
        let desktop = Rect { x: 0, y: 0, w: 6400, h: 1600 };
        assert_eq!(screenshot_to_save(Some((desktop, vec![1, 2])), desktop), Some(vec![1, 2]));
        // A monitor unplugged during the recording: the desktop changed.
        let smaller = Rect { w: 4480, ..desktop };
        assert_eq!(screenshot_to_save(Some((smaller, vec![1, 2])), desktop), None);
        // None was taken (Screenshot off, or a locked screen).
        assert_eq!(screenshot_to_save(None, desktop), None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigger_admission() {
        let unlocked = || true;
        assert_eq!(admit(false, Some("Report"), Mode::Idle, unlocked), Admission::Run);
        assert_eq!(admit(true, Some("Report"), Mode::Idle, unlocked), Admission::Ignore, "triggers paused");
        assert_eq!(admit(false, None, Mode::Idle, unlocked), Admission::Ignore, "the macro was deleted");
        for busy in [Mode::Countdown, Mode::Recording, Mode::Playing, Mode::Paused] {
            assert_eq!(
                admit(false, Some("Report"), busy, unlocked),
                Admission::Busy("Skipped “Report”: Relay was busy".into()),
                "{busy:?}"
            );
        }
        assert_eq!(admit(true, Some("Report"), Mode::Playing, unlocked), Admission::Ignore, "paused says nothing");
        assert_eq!(admit(false, Some("Report"), Mode::Idle, || false), Admission::NoDesktop, "locked");
        // The desktop is only asked when the run could go ahead.
        let asked = std::cell::Cell::new(false);
        admit(false, Some("Report"), Mode::Playing, || {
            asked.set(true);
            true
        });
        assert!(!asked.get());
    }

    #[test]
    fn only_the_current_playback_ends_it() {
        assert!(is_current(Some(3), 3));
        assert!(!is_current(Some(3), 2), "a late message from an older playback");
        assert!(!is_current(None, 3), "nothing plays any more");
    }

    #[test]
    fn only_a_completed_run_counts() {
        assert!(counts_as_run(FinishReason::Completed));
        for r in [
            FinishReason::Stopped,
            FinishReason::KeyPressed,
            FinishReason::Killed,
            FinishReason::PixelTimeout,
            FinishReason::Error,
        ] {
            assert!(!counts_as_run(r), "{r:?}");
        }
    }

    #[test]
    fn the_hotkey_playhead_follows_how_playback_ended() {
        for r in [FinishReason::Stopped, FinishReason::KeyPressed, FinishReason::Killed, FinishReason::Error] {
            assert_eq!(playhead_after(r, 5000, Some(1200)), 0.0, "{r:?} rewinds");
        }
        assert_eq!(playhead_after(FinishReason::Completed, 5000, None), 5000.0, "stays at the end");
        assert_eq!(playhead_after(FinishReason::PixelTimeout, 5000, Some(1200)), 1200.0, "stays on the check");
        assert_eq!(playhead_after(FinishReason::PixelTimeout, 5000, None), 0.0);
    }

    fn started() -> RunStart {
        RunStart {
            at: DateTime::from_timestamp(1_790_000_000, 0).unwrap(),
            started: 10_000.0,
            macro_id: Uuid::from_u128(4),
            macro_name: "Export invoice".into(),
            source: RunSource::AppLaunch,
            from: 1500,
            speed: 2.0,
            humanize: true,
        }
    }

    #[test]
    fn a_finished_run_is_logged_with_how_it_started_and_what_it_did() {
        use relay_core::runlog::{CheckOutcome, CheckResult};
        let mut report = RunReport { loops: 3, ..Default::default() };
        let check = CheckResult { step: 2, loop_idx: 2, image: false, after_ms: 40, outcome: CheckOutcome::TimedOut };
        report.checks.push(check);
        let e = run_entry(started(), FinishReason::PixelTimeout, report, 12_345.6);
        assert_eq!(e.at, started().at);
        assert_eq!((e.macro_id, e.macro_name.as_str()), (Uuid::from_u128(4), "Export invoice"));
        assert_eq!(e.source, RunSource::AppLaunch);
        assert_eq!(e.outcome, RunOutcome::Finished(FinishReason::PixelTimeout));
        assert_eq!((e.duration_ms, e.from_ms, e.loops), (2346, 1500, 3));
        assert_eq!((e.speed, e.humanize), (2.0, true));
        assert_eq!((e.checks, e.checks_dropped), (vec![check], 0));
        // An engine that panicked reports nothing: still one loop, no checks.
        let e = run_entry(started(), FinishReason::Error, RunReport::default(), 10_000.0);
        assert_eq!((e.loops, e.duration_ms, e.checks.len()), (1, 0, 0));
    }

    #[test]
    fn a_skipped_trigger_is_logged_with_why() {
        let at = DateTime::from_timestamp(1_790_000_000, 0).unwrap();
        let e = skip_entry(Uuid::from_u128(4), "Backup".into(), RunSource::Schedule, SkipReason::Missed, at);
        assert_eq!(e.outcome, RunOutcome::Skipped(SkipReason::Missed));
        assert_eq!((e.at, e.source, e.macro_name.as_str()), (at, RunSource::Schedule, "Backup"));
        assert_eq!((e.duration_ms, e.loops, e.checks.len()), (0, 0, 0));
    }
}
