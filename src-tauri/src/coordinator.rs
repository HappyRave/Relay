//! The coordinator owns the session state machine (relay-core `session`) on a
//! dedicated thread. Buttons, hotkeys, the hook and the player all send it
//! [`Cmd`]s; it applies the transition and performs the resulting effects.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, unbounded};
use relay_core::model::{CoordMode, Event, Macro, RecordingMeta, Rect};
use relay_core::session::{self, Effect, FinishReason, HotkeySet, Input, Mode, RunSource, SessionConfig};
use relay_core::steps::group_steps;
use relay_core::timeline;
use relay_platform::recorder::{Recorder, RecorderConfig, is_meaningful};
use relay_platform::{HookConfig, HookSession, Platform, RawKind};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::hotkeys;
use crate::ipc::{EngineMsg, Emitter};
use crate::library::Library;
use crate::engine::{self, EngineCmd, EngineHandle, PlayPlan};
use crate::rec_thread::{RecContext, RecThread};
use crate::settings::SettingsStore;
use crate::triggers::TriggerState;

/// F9 stops a recording through its global hotkey; keep it out of the macro.
const VK_F9: u16 = 0x78;
/// F10 pauses playback through its global hotkey; it must not count as "any key".
const VK_F10: u16 = 0x79;

pub enum Cmd {
    Input(Input),
    /// F10: play from wherever the UI's playhead was left.
    HotkeyPlay,
    CountdownDone(u64),
    /// Esc, reported by the hook during a session.
    Escape,
    /// Another key during playback with "stop on key press".
    StopKey,
    /// The engine finished on its own (completed, or an error).
    EngineDone(FinishReason),
    /// The UI selected a macro.
    Select(Uuid),
    Seek(f64),
    Speed(f64),
    /// A trigger (or a macro hotkey) wants to run a macro.
    RunMacro { id: Uuid, source: RunSource },
    /// From the tray or the Triggers tab.
    SetTriggersPaused(bool),
    /// The recorder's watchdog saw the cursor move without hook events.
    HookLost,
}

/// The current session mode, readable by commands (e.g. to refuse deleting
/// a macro while it plays).
#[derive(Default)]
pub struct SessionMode(std::sync::RwLock<Option<Mode>>);

impl SessionMode {
    pub fn is_idle(&self) -> bool {
        matches!(*self.0.read().unwrap(), None | Some(Mode::Idle))
    }
}

#[derive(Clone)]
pub struct CoordinatorHandle(Sender<Cmd>);

impl CoordinatorHandle {
    pub fn send(&self, cmd: Cmd) {
        let _ = self.0.send(cmd);
    }
}

struct Coordinator {
    app: AppHandle,
    platform: Arc<Platform>,
    emit: Arc<Emitter>,
    tx: Sender<Cmd>,
    mode: Mode,
    current: Option<Uuid>,
    idle_playhead: f64,
    countdown_gen: Arc<AtomicU64>,
    hook: Option<Box<dyn HookSession>>,
    recording: Option<RecThread>,
    /// How the recording hook was started, to reinstall it if Windows drops it.
    rec_hook: Option<(HookConfig, crossbeam_channel::Sender<relay_platform::RawInput>)>,
    engine: Option<EngineHandle>,
    /// Why the session is being stopped, for the Finished message.
    stop_reason: Option<FinishReason>,
    /// Whether the window was made click-through for this playback.
    click_through: bool,
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
        countdown_gen: Arc::new(AtomicU64::new(0)),
        hook: None,
        recording: None,
        rec_hook: None,
        engine: None,
        stop_reason: None,
        click_through: false,
    };
    std::thread::Builder::new()
        .name("relay-coordinator".into())
        .spawn(move || c.run(rx))
        .expect("spawn coordinator thread");
    CoordinatorHandle(tx)
}

impl Coordinator {
    fn run(&mut self, rx: Receiver<Cmd>) {
        hotkeys::apply(&self.app, HotkeySet::Idle, &self.emit);
        for cmd in rx {
            self.handle(cmd);
        }
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Input(Input::Kill) => {
                self.stop_reason = Some(FinishReason::Killed);
                self.input(Input::Kill);
            }
            Cmd::Input(input) => self.input(input),
            Cmd::HotkeyPlay => self.input(Input::TogglePlay { from: self.idle_playhead.round() as u32 }),
            Cmd::CountdownDone(generation) if generation == self.countdown_gen.load(Ordering::SeqCst) => {
                self.input(Input::CountdownDone)
            }
            Cmd::CountdownDone(_) => {}
            Cmd::Escape => self.input(Input::Stop),
            Cmd::StopKey => {
                self.stop_reason = Some(FinishReason::KeyPressed);
                self.input(Input::Stop);
            }
            Cmd::EngineDone(reason) => {
                self.engine = None;
                if reason == FinishReason::Completed {
                    self.count_run();
                }
                self.input(Input::PlaybackFinished(reason));
            }
            Cmd::Select(id) => {
                if self.mode == Mode::Idle {
                    self.current = Some(id);
                    self.idle_playhead = 0.0;
                }
            }
            Cmd::Seek(t) => match &self.engine {
                Some(e) => e.send(EngineCmd::Seek(t)),
                None => self.idle_playhead = t,
            },
            Cmd::Speed(v) => self.engine_cmd(EngineCmd::Speed(v)),
            Cmd::RunMacro { id, source } => self.run_triggered(id, source),
            Cmd::SetTriggersPaused(paused) => self.set_triggers_paused(paused),
            Cmd::HookLost => self.reinstall_hook(),
        }
    }

    fn input(&mut self, input: Input) {
        let cfg = SessionConfig {
            record_countdown_ms: if self.settings().countdown { 3000 } else { 0 },
        };
        let (mode, effects) = session::step(self.mode, input.clone(), &cfg);
        if mode != self.mode {
            tracing::info!(from = ?self.mode, to = ?mode, ?input, "session");
        }
        self.mode = mode;
        for e in effects {
            self.effect(e);
        }
    }

    fn settings(&self) -> crate::settings::Settings {
        self.app.state::<Mutex<SettingsStore>>().lock().unwrap().current.clone()
    }

    fn effect(&mut self, effect: Effect) {
        match effect {
            Effect::StartCountdown { ms } => self.start_countdown(ms),
            Effect::CancelCountdown => {
                self.countdown_gen.fetch_add(1, Ordering::SeqCst);
            }
            Effect::StartRecording => self.start_recording(),
            Effect::StopRecording { keep } => self.stop_recording(keep),
            Effect::StartPlayback { from, .. } => self.start_playback(from),
            Effect::PausePlayback => self.engine_cmd(EngineCmd::Pause),
            Effect::ResumePlayback => self.engine_cmd(EngineCmd::Resume),
            Effect::StopPlayback => {
                if let Some(e) = self.engine.take() {
                    e.stop();
                }
                let reason = self.stop_reason.take().unwrap_or(FinishReason::Stopped);
                self.emit.send(EngineMsg::Finished { reason, timing: None });
            }
            Effect::SetHotkeys(set) => hotkeys::apply(&self.app, set, &self.emit),
            Effect::PauseTriggers => {
                if !self.app.state::<TriggerState>().paused() {
                    self.set_triggers_paused(true);
                    self.emit.send(EngineMsg::Notice {
                        message: "Triggers are paused. Resume them from the tray or the Triggers tab.".into(),
                    });
                }
            }
            Effect::TriggerSkipped(_) => {}
            Effect::EmitMode(mode) => {
                if mode == Mode::Idle {
                    self.end_playback();
                    self.stop_reason = None;
                }
                *self.app.state::<SessionMode>().0.write().unwrap() = Some(mode);
                crate::tray::set_mode(&self.app, mode);
                if let Some(w) = self.app.get_webview_window("main") {
                    crate::window_ctl::set_no_activate(&w, mode != Mode::Idle);
                }
                self.emit.send(EngineMsg::Session { mode, macro_id: self.current });
            }
        }
    }

    /// A trigger fired: run its macro now if Relay is free and the screen is usable.
    fn run_triggered(&mut self, id: Uuid, source: RunSource) {
        if self.app.state::<TriggerState>().paused() {
            return;
        }
        let name = self.app.state::<Mutex<Library>>().lock().unwrap().get(id).map(|e| e.macro_.name.clone());
        let Some(name) = name else { return };
        if self.mode != Mode::Idle {
            self.emit.send(EngineMsg::Notice { message: format!("Skipped “{name}”: Relay was busy") });
            return;
        }
        if !self.platform.windows.input_desktop_available() {
            // Locked, or a UAC prompt: nothing can be clicked or typed.
            tracing::info!(%id, ?source, "trigger skipped: the input desktop isn't available (locked?)");
            return;
        }
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
        if let Some(e) = &self.engine {
            e.send(cmd);
        }
    }

    /// Undoes what playback set up: the watching hook and click-through.
    fn end_playback(&mut self) {
        if let Some(e) = self.engine.take() {
            e.stop();
        }
        if let Some(hook) = self.hook.take() {
            hook.stop();
        }
        if self.click_through {
            self.click_through = false;
            if let Some(w) = self.app.get_webview_window("main") {
                let _ = w.set_ignore_cursor_events(false);
            }
        }
    }

    fn count_run(&mut self) {
        let Some(id) = self.current else { return };
        let lib = self.app.state::<Mutex<Library>>();
        let mut lib = lib.lock().unwrap();
        if let Some(e) = lib.get_mut(id) {
            e.runs += 1;
            e.last_run = Some(chrono::Utc::now());
            let _ = lib.save(id);
        }
        drop(lib);
        self.emit.send(EngineMsg::LibraryChanged);
    }

    fn start_countdown(&mut self, ms: u32) {
        let generation = self.countdown_gen.fetch_add(1, Ordering::SeqCst) + 1;
        let current = self.countdown_gen.clone();
        let (emit, tx, now_ms) = (self.emit.clone(), self.tx.clone(), self.platform.now_ms);
        std::thread::spawn(move || {
            let start = now_ms();
            while current.load(Ordering::SeqCst) == generation {
                let left = ms as f64 - (now_ms() - start);
                if left <= 0.0 {
                    let _ = tx.send(Cmd::CountdownDone(generation));
                    return;
                }
                emit.send(EngineMsg::Countdown { left_ms: left as u32 });
                std::thread::sleep(Duration::from_millis(50));
            }
        });
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
        let (own_rect, own_window) = self.own_window();
        let hook_cfg = HookConfig {
            own_rect,
            own_window,
            swallow_escape: settings.esc_stops_recording,
            ignore_injected: settings.ignore_injected,
            drop_vks: vec![VK_F9],
            record: true,
            stop_on_key: false,
        };
        let (raw_tx, raw_rx) = crossbeam_channel::bounded(8192);
        self.rec_hook = Some((hook_cfg.clone(), raw_tx.clone()));
        match self.platform.hook.start(hook_cfg, raw_tx) {
            Ok(hook) => self.hook = Some(hook),
            Err(e) => {
                self.emit.error(format!("Couldn't start recording: {e}"));
                // Queued, so this transition's own effects finish first.
                let _ = self.tx.send(Cmd::Input(Input::Stop));
                return;
            }
        }
        let (ms, px) = self.platform.screen.double_click();
        let recorder = Recorder::new(
            RecorderConfig { capture_moves: settings.capture_moves, capture_keys: settings.capture_keys, move_interval_ms: 16 },
            (self.platform.now_ms)(),
            (self.platform.translator)(),
        );
        let ctx = RecContext {
            screen: self.platform.screen.clone(),
            desktop: self.platform.screen.virtual_desktop(),
            group: relay_core::steps::GroupOptions { double_click_ms: ms, double_click_px: px.max(4) },
            now_ms: self.platform.now_ms,
            emit: self.emit.clone(),
            coordinator: self.tx.clone(),
        };
        self.recording = Some(RecThread::spawn(recorder, raw_rx, ctx));
    }

    /// Windows removes low-level hooks it thinks are too slow, without telling
    /// anyone. Put a fresh one in place, feeding the same recorder.
    fn reinstall_hook(&mut self) {
        if self.mode != Mode::Recording {
            return;
        }
        let Some((cfg, tx)) = self.rec_hook.clone() else { return };
        if let Some(hook) = self.hook.take() {
            hook.stop();
        }
        tracing::warn!("the input hook stopped delivering events; reinstalling it");
        match self.platform.hook.start(cfg, tx) {
            Ok(hook) => {
                self.hook = Some(hook);
                self.emit.send(EngineMsg::Notice {
                    message: "Windows dropped Relay's input hook; it was restarted. Check the last steps.".into(),
                });
            }
            Err(e) => self.emit.error(format!("Recording lost its input hook and couldn't restart it: {e}")),
        }
    }

    fn stop_recording(&mut self, keep: bool) {
        self.rec_hook = None;
        if let Some(hook) = self.hook.take() {
            hook.stop();
        }
        let Some(rec) = self.recording.take() else { return };
        let recording = rec.finish();
        if !keep || !is_meaningful(&recording.events) {
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
        let lib = self.app.state::<Mutex<Library>>();
        let mut lib = lib.lock().unwrap();
        tracing::info!(events = recording.events.len(), ms = recording.duration_ms, "recording saved");
        let m = Macro::new(lib.next_recording_name(), meta, recording.events);
        let id = m.id;
        if let Err(e) = lib.insert_front(m) {
            self.emit.error(format!("Couldn't save the recording: {e}"));
        }
        drop(lib);
        self.current = Some(id);
        self.emit.send(EngineMsg::Saved { id });
        self.emit.send(EngineMsg::LibraryChanged);
    }

    fn start_playback(&mut self, from: u32) {
        let lib = self.app.state::<Mutex<Library>>();
        let Some(m) = self.current.and_then(|id| lib.lock().unwrap().get(id).map(|e| e.macro_.clone())) else {
            self.emit.error("Select a macro to play");
            let _ = self.tx.send(Cmd::Input(Input::PlaybackFinished(FinishReason::Error)));
            return;
        };
        let (own_rect, own_window) = self.own_window();
        let windows = &self.platform.windows;

        // Started from Relay's own button: give the keyboard back to the app
        // the user was working in, or keystrokes would type into Relay.
        let mut target = windows.foreground();
        if target.is_some_and(|w| w.hwnd == own_window) {
            target = windows.restore_previous(own_window);
        }
        if let Some(t) = target
            && windows.is_elevated(t.pid)
            && !windows.self_elevated()
        {
            self.emit.send(EngineMsg::Notice {
                message: "The app in front runs as administrator, so Windows blocks Relay's input to it. \
                          Run Relay as administrator to automate it."
                    .into(),
            });
        }

        let offset = self.window_offset(&m);
        // Clicks under the always-on-top widget must reach the app beneath it.
        let clicks_under_widget = own_rect.is_some_and(|r| {
            m.events.iter().any(|e| matches!(e, Event::Button { x, y, .. } if r.contains(x + offset.0, y + offset.1)))
        });
        if clicks_under_widget && let Some(w) = self.app.get_webview_window("main") {
            self.click_through = w.set_ignore_cursor_events(true).is_ok();
        }

        // Watch for Esc and, if enabled, any other key.
        let (raw_tx, raw_rx) = crossbeam_channel::bounded(64);
        let hook_cfg = HookConfig {
            own_rect,
            own_window,
            swallow_escape: true,
            ignore_injected: self.settings().ignore_injected,
            drop_vks: vec![VK_F10],
            record: false,
            stop_on_key: m.playback.stop_on_key,
        };
        match self.platform.hook.start(hook_cfg, raw_tx) {
            Ok(hook) => {
                self.hook = Some(hook);
                let tx = self.tx.clone();
                std::thread::spawn(move || {
                    for raw in raw_rx {
                        let cmd = match raw.kind {
                            RawKind::Escape => Cmd::Escape,
                            RawKind::StopKey => Cmd::StopKey,
                            _ => continue,
                        };
                        let _ = tx.send(cmd);
                    }
                });
            }
            Err(e) => self.emit.error(format!("Esc won't stop playback: {e}")),
        }

        let steps = group_steps(&m.events, (&m.recording).into());
        let plan = PlayPlan {
            duration: timeline::duration(&m.events),
            repeat: m.playback.repeat,
            speed: m.playback.speed as f64,
            jitter_ms: if m.playback.humanize { m.playback.jitter_ms } else { 0 },
            seed: (self.platform.now_ms)().to_bits() ^ (m.id.as_u128() as u64),
            offset,
            from,
            steps,
            events: m.events,
        };
        self.engine = Some(engine::spawn(plan, &self.platform, self.emit.clone(), self.tx.clone()));
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
