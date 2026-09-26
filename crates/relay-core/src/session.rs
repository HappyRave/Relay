//! The session state machine: which inputs (hotkeys, buttons, engine events)
//! move Relay between idle, countdown, recording, playing and paused, and
//! which side effects each transition asks the app to perform. Pure, so every
//! transition is testable without an OS.

use serde::Serialize;
use ts_rs::TS;

use crate::model::Ms;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Mode {
    Idle,
    Countdown,
    Recording,
    Playing,
    Paused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum RunSource {
    Manual,
    Hotkey,
    Schedule,
    AppLaunch,
    Pixel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum FinishReason {
    Completed,
    Stopped,
    KeyPressed,
    Killed,
    PixelTimeout,
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Input {
    /// F9 or the record button.
    ToggleRecord,
    /// F10 or the play button; `from` is the UI playhead when starting.
    TogglePlay {
        from: Ms,
    },
    /// The stop button, Esc (`Stopped`) or another key with "stop on key
    /// press" (`KeyPressed`). Only a session can be stopped.
    Stop(FinishReason),
    /// Ctrl + Alt + End.
    Kill,
    CountdownDone,
    PlaybackFinished(FinishReason),
    /// A trigger wants to run the current macro.
    Trigger(RunSource),
}

/// The hotkeys registered in each state (see the plan's hotkey table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeySet {
    /// F9, F10, Ctrl+Shift+M, Kill and per-macro hotkeys.
    Idle,
    /// F9 (stop) and Kill; everything else must reach the recorded apps.
    Recording,
    /// F10 (pause) and Kill.
    Playing,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    StartCountdown {
        ms: Ms,
    },
    CancelCountdown,
    StartRecording,
    /// Stop capturing and save the recording to the library.
    StopRecording,
    StartPlayback {
        from: Ms,
    },
    PausePlayback,
    ResumePlayback,
    /// Stop the engine; the reason goes to the UI.
    StopPlayback(FinishReason),
    SetHotkeys(HotkeySet),
    PauseTriggers,
    /// Tell the UI the mode changed.
    EmitMode(Mode),
}

#[derive(Debug, Clone, Copy)]
pub struct SessionConfig {
    /// Countdown before recording starts; 0 disables it.
    pub record_countdown_ms: Ms,
}

impl Default for SessionConfig {
    fn default() -> Self {
        SessionConfig { record_countdown_ms: 3000 }
    }
}

/// Applies `input` in `mode`, returning the new mode and the effects to run in order.
pub fn step(mode: Mode, input: Input, cfg: &SessionConfig) -> (Mode, Vec<Effect>) {
    use Effect::*;
    use Mode::*;

    let to = |m: Mode, mut fx: Vec<Effect>| {
        fx.push(EmitMode(m));
        (m, fx)
    };
    let stay = |fx: Vec<Effect>| (mode, fx);

    match (mode, input) {
        (_, Input::Kill) => match mode {
            Idle => stay(vec![PauseTriggers]),
            Countdown => to(Idle, vec![CancelCountdown, SetHotkeys(HotkeySet::Idle), PauseTriggers]),
            Recording => to(Idle, vec![StopRecording, SetHotkeys(HotkeySet::Idle), PauseTriggers]),
            Playing | Paused => {
                to(Idle, vec![StopPlayback(FinishReason::Killed), SetHotkeys(HotkeySet::Idle), PauseTriggers])
            }
        },

        (Idle, Input::ToggleRecord) if cfg.record_countdown_ms > 0 => {
            to(Countdown, vec![SetHotkeys(HotkeySet::Recording), StartCountdown { ms: cfg.record_countdown_ms }])
        }
        (Idle, Input::ToggleRecord) => to(Recording, vec![SetHotkeys(HotkeySet::Recording), StartRecording]),
        (Idle, Input::TogglePlay { from }) => to(Playing, vec![SetHotkeys(HotkeySet::Playing), StartPlayback { from }]),
        (Idle, Input::Trigger(_)) => to(Playing, vec![SetHotkeys(HotkeySet::Playing), StartPlayback { from: 0 }]),

        (Countdown, Input::CountdownDone) => to(Recording, vec![StartRecording]),
        (Countdown, Input::ToggleRecord | Input::Stop(_)) => {
            to(Idle, vec![CancelCountdown, SetHotkeys(HotkeySet::Idle)])
        }

        (Recording, Input::ToggleRecord | Input::Stop(_)) => to(Idle, vec![StopRecording, SetHotkeys(HotkeySet::Idle)]),

        (Playing, Input::TogglePlay { .. }) => to(Paused, vec![PausePlayback]),
        (Paused, Input::TogglePlay { .. }) => to(Playing, vec![ResumePlayback]),
        (Playing | Paused, Input::Stop(reason)) => to(Idle, vec![StopPlayback(reason), SetHotkeys(HotkeySet::Idle)]),
        (Playing | Paused, Input::PlaybackFinished(_)) => to(Idle, vec![SetHotkeys(HotkeySet::Idle)]),

        // Everything else is ignored: F9 while playing, F10 while recording,
        // Stop while idle, a late CountdownDone or PlaybackFinished, a
        // trigger while busy (the coordinator tells the user), …
        _ => stay(vec![]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Effect::*;
    use Mode::*;

    fn run(mode: Mode, input: Input) -> (Mode, Vec<Effect>) {
        step(mode, input, &SessionConfig::default())
    }

    /// Every input in every mode: the new mode and the exact effects, in order.
    #[test]
    fn every_transition() {
        use FinishReason::*;
        let idle = |fx: &[Effect]| (Idle, [fx, &[SetHotkeys(HotkeySet::Idle), EmitMode(Idle)]].concat());
        let killed = |first: Effect| (Idle, vec![first, SetHotkeys(HotkeySet::Idle), PauseTriggers, EmitMode(Idle)]);
        let play = |from| (Playing, vec![SetHotkeys(HotkeySet::Playing), StartPlayback { from }, EmitMode(Playing)]);
        let ignored = |m: Mode| (m, vec![]);
        let inputs = [
            Input::ToggleRecord,
            Input::TogglePlay { from: 700 },
            Input::Stop(Stopped),
            Input::Stop(KeyPressed),
            Input::Kill,
            Input::CountdownDone,
            Input::PlaybackFinished(Completed),
            Input::Trigger(RunSource::Schedule),
        ];
        type Row = [(Mode, Vec<Effect>); 8];
        let table: [(Mode, Row); 5] = [
            (
                Idle,
                [
                    (
                        Countdown,
                        vec![SetHotkeys(HotkeySet::Recording), StartCountdown { ms: 3000 }, EmitMode(Countdown)],
                    ),
                    play(700),
                    ignored(Idle),
                    ignored(Idle),
                    (Idle, vec![PauseTriggers]),
                    ignored(Idle),
                    ignored(Idle),
                    play(0),
                ],
            ),
            (
                Countdown,
                [
                    idle(&[CancelCountdown]),
                    ignored(Countdown),
                    idle(&[CancelCountdown]),
                    idle(&[CancelCountdown]),
                    killed(CancelCountdown),
                    (Recording, vec![StartRecording, EmitMode(Recording)]),
                    ignored(Countdown),
                    ignored(Countdown),
                ],
            ),
            (
                Recording,
                [
                    idle(&[StopRecording]),
                    ignored(Recording),
                    idle(&[StopRecording]),
                    idle(&[StopRecording]),
                    killed(StopRecording), // the kill switch keeps the recording
                    ignored(Recording),
                    ignored(Recording),
                    ignored(Recording),
                ],
            ),
            (
                Playing,
                [
                    ignored(Playing),
                    (Paused, vec![PausePlayback, EmitMode(Paused)]),
                    idle(&[StopPlayback(Stopped)]),
                    idle(&[StopPlayback(KeyPressed)]),
                    killed(StopPlayback(Killed)),
                    ignored(Playing),
                    idle(&[]),
                    ignored(Playing),
                ],
            ),
            (
                Paused,
                [
                    ignored(Paused),
                    (Playing, vec![ResumePlayback, EmitMode(Playing)]),
                    idle(&[StopPlayback(Stopped)]),
                    idle(&[StopPlayback(KeyPressed)]),
                    killed(StopPlayback(Killed)),
                    ignored(Paused),
                    idle(&[]),
                    ignored(Paused),
                ],
            ),
        ];
        for (mode, row) in table {
            for (input, want) in inputs.iter().zip(row) {
                assert_eq!(run(mode, input.clone()), want, "{mode:?} + {input:?}");
            }
        }
        // Without a countdown, recording starts at once.
        let (m, fx) = step(Idle, Input::ToggleRecord, &SessionConfig { record_countdown_ms: 0 });
        assert_eq!((m, fx), (Recording, vec![SetHotkeys(HotkeySet::Recording), StartRecording, EmitMode(Recording)]));
    }

    #[test]
    fn record_with_countdown_then_stop() {
        let (m, fx) = run(Idle, Input::ToggleRecord);
        assert_eq!(m, Countdown);
        assert!(fx.contains(&StartCountdown { ms: 3000 }));
        let (m, fx) = run(m, Input::CountdownDone);
        assert_eq!((m, fx[0].clone()), (Recording, StartRecording));
        let (m, fx) = run(m, Input::ToggleRecord);
        assert_eq!(m, Idle);
        assert_eq!(fx[..2], [StopRecording, SetHotkeys(HotkeySet::Idle)]);
    }

    #[test]
    fn record_without_countdown() {
        let (m, fx) = step(Idle, Input::ToggleRecord, &SessionConfig { record_countdown_ms: 0 });
        assert_eq!(m, Recording);
        assert!(fx.contains(&StartRecording));
    }

    #[test]
    fn esc_during_countdown_cancels() {
        let (m, fx) = run(Countdown, Input::Stop(FinishReason::Stopped));
        assert_eq!(m, Idle);
        assert!(fx.contains(&CancelCountdown));
        assert!(!fx.contains(&StopRecording));
    }

    #[test]
    fn play_pause_resume_finish() {
        let (m, fx) = run(Idle, Input::TogglePlay { from: 1200 });
        assert_eq!(m, Playing);
        assert!(fx.contains(&StartPlayback { from: 1200 }));
        let (m, _) = run(m, Input::TogglePlay { from: 0 });
        assert_eq!(m, Paused);
        let (m, fx) = run(m, Input::TogglePlay { from: 0 });
        assert_eq!((m, fx[0].clone()), (Playing, ResumePlayback));
        let (m, _) = run(m, Input::PlaybackFinished(FinishReason::Completed));
        assert_eq!(m, Idle);
    }

    #[test]
    fn f9_while_playing_and_f10_while_recording_are_ignored() {
        assert_eq!(run(Playing, Input::ToggleRecord), (Playing, vec![]));
        assert_eq!(run(Recording, Input::TogglePlay { from: 0 }), (Recording, vec![]));
        assert_eq!(run(Idle, Input::Stop(FinishReason::Stopped)), (Idle, vec![]));
    }

    #[test]
    fn kill_stops_everything_and_pauses_triggers() {
        for mode in [Countdown, Recording, Playing, Paused] {
            let (m, fx) = run(mode, Input::Kill);
            assert_eq!(m, Idle, "{mode:?}");
            assert!(fx.contains(&PauseTriggers), "{mode:?}");
        }
        assert_eq!(run(Idle, Input::Kill), (Idle, vec![PauseTriggers]));
    }

    #[test]
    fn triggers_run_when_idle_and_are_skipped_when_busy() {
        let (m, fx) = run(Idle, Input::Trigger(RunSource::Schedule));
        assert_eq!(m, Playing);
        assert!(fx.contains(&StartPlayback { from: 0 }));
        assert_eq!(run(Recording, Input::Trigger(RunSource::Hotkey)), (Recording, vec![]));
    }
}
