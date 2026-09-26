//! The coordinator owns the session state machine (relay-core `session`) on a
//! dedicated thread. Buttons, hotkeys, the hook, the engine and the triggers
//! all send it [`Cmd`]s; it applies the transition and performs the effects.
//!
//! It never holds a lock while it calls into the main thread (window, tray,
//! hotkeys), so the main thread can always take the locks it needs.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use parking_lot::{Mutex, RwLock};
use relay_core::model::{CoordMode, Event, Macro, RecordingMeta, Rect};
use relay_core::session::{self, Effect, FinishReason, HotkeySet, Input, Mode, RunSource, SessionConfig};
use relay_core::steps::{GroupOptions, group_steps};
use relay_core::timeline;
use relay_platform::recorder::{Recorder, RecorderConfig, is_meaningful};
use relay_platform::{HookConfig, HookMode, HookSession, Platform, RawInput, RawKind};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::engine::{self, EngineCmd, EngineHandle, PlayPlan, TimingStats};
use crate::hotkeys;
use crate::ipc::{Emitter, EngineMsg};
use crate::library::Library;
use crate::rec_thread::{RecContext, RecThread};
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
        timing: Option<TimingStats>,
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
}

/// The playback in progress.
struct Playback {
    engine: EngineHandle,
    generation: u64,
    /// Watches for Esc and stop keys; `None` if it couldn't start.
    _hook: Option<Box<dyn HookSession>>,
    /// Whether the window was made click-through for this playback.
    click_through: bool,
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
                        self.mode = Mode::Idle;
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
            Cmd::EngineDone { generation, reason, timing } => {
                // A late message from a playback that was already stopped.
                if self.playback.as_ref().is_none_or(|p| p.generation != generation) {
                    return;
                }
                self.end_playback();
                if reason == FinishReason::Completed {
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
            Cmd::Seek(t) => match &self.playback {
                Some(p) => p.engine.send(EngineCmd::Seek(t)),
                None => self.idle_playhead = t,
            },
            Cmd::Speed { id, speed } => {
                if self.current == Some(id) {
                    self.engine_cmd(EngineCmd::Speed(speed));
                }
            }
            Cmd::RunMacro { id, source } => self.run_triggered(id, source),
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
            self.mode = mode;
            *self.app.state::<SessionMode>().0.write() = mode;
        }
        for e in effects {
            self.effect(e);
        }
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
                let timing = self.end_playback();
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
        if self.app.state::<TriggerState>().paused() {
            return;
        }
        let Some(name) = self.app.state::<Mutex<Library>>().lock().get(id).map(|e| e.macro_.name.clone()) else {
            return;
        };
        if self.mode != Mode::Idle {
            self.emit.send(EngineMsg::Notice { message: format!("Skipped “{name}”: Relay was busy") });
            return;
        }
        if !self.platform.windows.input_desktop_available() {
            // Locked, or a UAC prompt: nothing can be clicked or typed.
            tracing::info!(%id, ?source, "trigger skipped: the input desktop isn't available (locked?)");
            return;
        }
        tracing::info!(%id, ?source, "running a triggered macro");
        self.current = Some(id);
        self.idle_playhead = 0.0;
        self.input(Input::Trigger(source));
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
    /// and returns the timing of the run.
    fn end_playback(&mut self) -> Option<TimingStats> {
        let p = self.playback.take()?;
        let timing = p.engine.stop();
        if p.click_through
            && let Some(w) = self.app.get_webview_window("main")
        {
            let _ = w.set_ignore_cursor_events(false);
        }
        timing
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
        #[cfg(windows)]
        let hwnd = w.hwnd().map(|h| h.0 as isize).unwrap_or(0);
        #[cfg(not(windows))]
        let hwnd = 0;
        (rect, hwnd)
    }

    fn start_recording(&mut self) {
        let settings = self.settings();
        let (_, own_window) = self.own_window();
        let hook_cfg = HookConfig {
            mode: HookMode::Record { own_window, skip_vks: vec![VK_F9], esc_stops: settings.esc_stops_recording },
            ignore_injected: settings.ignore_injected,
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
        };
        let thread = RecThread::spawn(recorder, raw_rx, ctx);
        self.recording = Some(Recording { hook, hook_cfg, raw_tx, thread });
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
        self.current = Some(id);
        self.emit.send(EngineMsg::Saved { id });
        self.emit.send(EngineMsg::LibraryChanged);
    }

    fn start_playback(&mut self, from: u32) {
        let m = self.current.and_then(|id| self.app.state::<Mutex<Library>>().lock().get(id).map(|e| e.macro_.clone()));
        let Some(m) = m else {
            self.emit.error("Select a macro to play");
            let _ = self.tx.send(Cmd::Input(Input::PlaybackFinished(FinishReason::Error)));
            return;
        };
        let (own_rect, own_window) = self.own_window();
        self.hand_focus_back(own_window);
        let offset = self.window_offset(&m);
        let click_through = self.click_through_if_needed(&m, own_rect, offset);
        let hook = self.watch_for_stop_keys(&m);

        self.generation += 1;
        let plan = PlayPlan {
            duration: timeline::duration(&m.events),
            repeat: m.playback.repeat,
            speed: m.playback.speed as f64,
            jitter_ms: if m.playback.humanize { m.playback.jitter_ms } else { 0 },
            seed: (self.platform.now_ms)().to_bits() ^ (m.id.as_u128() as u64),
            offset,
            from,
            steps: group_steps(&m.events, (&m.recording).into()),
            events: m.events,
        };
        let engine = engine::spawn(plan, &self.platform, self.emit.clone(), self.tx.clone(), self.generation);
        self.playback = Some(Playback { engine, generation: self.generation, _hook: hook, click_through });
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

    /// Clicks under the always-on-top widget must reach the app beneath it.
    fn click_through_if_needed(&self, m: &Macro, own_rect: Option<Rect>, offset: (i32, i32)) -> bool {
        let under_widget = own_rect.is_some_and(|r| {
            m.events.iter().any(|e| matches!(e, Event::Button { x, y, .. } if r.contains(x + offset.0, y + offset.1)))
        });
        under_widget && self.app.get_webview_window("main").is_some_and(|w| w.set_ignore_cursor_events(true).is_ok())
    }

    /// Watches for Esc and, if the macro wants it, any other key.
    fn watch_for_stop_keys(&self, m: &Macro) -> Option<Box<dyn HookSession>> {
        let cfg = HookConfig {
            mode: HookMode::Watch { stop_on_key: m.playback.stop_on_key, pass_vks: vec![VK_F10] },
            ignore_injected: self.settings().ignore_injected,
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
        if m.playback.coord_mode != CoordMode::Window {
            return (0, 0);
        }
        let Some(anchor) = &m.recording.anchor_window else {
            self.emit.send(EngineMsg::Notice {
                message: "This macro has no anchor window; playing at screen coordinates.".into(),
            });
            return (0, 0);
        };
        match self.platform.windows.find_window(&anchor.exe, &anchor.class) {
            Some(now) => (now.rect.x - anchor.rect.x, now.rect.y - anchor.rect.y),
            None => {
                self.emit.send(EngineMsg::Notice {
                    message: format!("Couldn't find {}; playing at screen coordinates.", anchor.exe),
                });
                (0, 0)
            }
        }
    }
}
