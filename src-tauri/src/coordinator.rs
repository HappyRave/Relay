//! The coordinator owns the session state machine (relay-core `session`) on a
//! dedicated thread. Buttons, hotkeys, the hook and the player all send it
//! [`Cmd`]s; it applies the transition and performs the resulting effects.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, unbounded};
use relay_core::model::{Macro, RecordingMeta, Rect};
use relay_core::playback::PlaySession;
use relay_core::session::{self, Effect, FinishReason, HotkeySet, Input, Mode, SessionConfig};
use relay_core::timeline;
use relay_platform::recorder::{Recorder, RecorderConfig, is_meaningful};
use relay_platform::{HookConfig, HookSession, Platform};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::hotkeys;
use crate::ipc::{EngineMsg, Emitter};
use crate::library::Library;
use crate::player::{Player, PlayerCmd};
use crate::rec_thread::{RecContext, RecThread};
use crate::settings::SettingsStore;

/// F9 stops a recording through its global hotkey; keep it out of the macro.
const VK_F9: u16 = 0x78;

pub enum Cmd {
    Input(Input),
    /// F10: play from wherever the UI's playhead was left.
    HotkeyPlay,
    CountdownDone(u64),
    /// Esc, reported by the hook during a session.
    Escape,
    /// The UI selected a macro.
    Select(Uuid),
    Seek(f64),
    Speed(f64),
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
    player: Option<Player>,
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
        player: None,
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
            Cmd::Input(input) => self.input(input),
            Cmd::HotkeyPlay => self.input(Input::TogglePlay { from: self.idle_playhead.round() as u32 }),
            Cmd::CountdownDone(generation) if generation == self.countdown_gen.load(Ordering::SeqCst) => {
                self.input(Input::CountdownDone)
            }
            Cmd::CountdownDone(_) => {}
            Cmd::Escape => self.input(Input::Stop),
            Cmd::Select(id) => {
                if self.mode == Mode::Idle {
                    self.current = Some(id);
                    self.idle_playhead = 0.0;
                }
            }
            Cmd::Seek(t) => match &self.player {
                Some(p) => p.send(PlayerCmd::Seek(t)),
                None => self.idle_playhead = t,
            },
            Cmd::Speed(v) => {
                if let Some(p) = &self.player {
                    p.send(PlayerCmd::Speed(v));
                }
            }
        }
    }

    fn input(&mut self, input: Input) {
        let cfg = SessionConfig {
            record_countdown_ms: if self.settings().countdown { 3000 } else { 0 },
        };
        let (mode, effects) = session::step(self.mode, input, &cfg);
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
            Effect::PausePlayback => self.player_cmd(PlayerCmd::Pause),
            Effect::ResumePlayback => self.player_cmd(PlayerCmd::Resume),
            Effect::StopPlayback => {
                if let Some(p) = self.player.take() {
                    p.stop();
                }
                self.emit.send(EngineMsg::Finished { reason: FinishReason::Stopped });
            }
            Effect::SetHotkeys(set) => hotkeys::apply(&self.app, set, &self.emit),
            // Triggers arrive in M7.
            Effect::PauseTriggers | Effect::TriggerSkipped(_) => {}
            Effect::EmitMode(mode) => {
                if mode == Mode::Idle {
                    self.player = None;
                }
                self.emit.send(EngineMsg::Session { mode, macro_id: self.current });
            }
        }
    }

    fn player_cmd(&self, cmd: PlayerCmd) {
        if let Some(p) = &self.player {
            p.send(cmd);
        }
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
            swallow_escape: true,
            ignore_injected: settings.ignore_injected,
            drop_vks: vec![VK_F9],
        };
        let (raw_tx, raw_rx) = crossbeam_channel::bounded(8192);
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
            desktop: self.platform.screen.virtual_desktop(),
            group: relay_core::steps::GroupOptions { double_click_ms: ms, double_click_px: px.max(4) },
            now_ms: self.platform.now_ms,
            emit: self.emit.clone(),
            coordinator: self.tx.clone(),
        };
        self.recording = Some(RecThread::spawn(recorder, raw_rx, ctx));
    }

    fn stop_recording(&mut self, keep: bool) {
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
        let session = {
            let lib = lib.lock().unwrap();
            self.current.and_then(|id| lib.get(id)).map(|e| {
                let m = &e.macro_;
                let duration = timeline::duration(&m.events);
                let from = if from + 1 >= duration { 0 } else { from };
                PlaySession::new(from, (self.platform.now_ms)(), m.playback.speed as f64, duration, m.playback.repeat)
            })
        };
        match session {
            Some(s) => {
                self.player = Some(Player::spawn(s, self.platform.now_ms, self.emit.clone(), self.tx.clone()));
            }
            None => {
                self.emit.error("Select a macro to play");
                let _ = self.tx.send(Cmd::Input(Input::PlaybackFinished(FinishReason::Error)));
            }
        }
    }
}
