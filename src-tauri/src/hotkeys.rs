//! Global hotkeys through RegisterHotKey (tauri-plugin-global-shortcut). They
//! fire even when an elevated window is focused, unlike a low-level hook.
//! Which ones are registered depends on the session state, so combos Relay
//! doesn't need at the moment reach other apps (and recordings).

use relay_core::session::{HotkeySet, Input};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::coordinator::{Cmd, CoordinatorHandle};
use crate::ipc::{EngineMsg, Emitter};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Action {
    Record,
    Play,
    Compact,
    Kill,
}

fn shortcut(a: Action) -> Shortcut {
    match a {
        Action::Record => Shortcut::new(None, Code::F9),
        Action::Play => Shortcut::new(None, Code::F10),
        Action::Compact => Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyM),
        Action::Kill => Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::End),
    }
}

const ALL: [Action; 4] = [Action::Record, Action::Play, Action::Compact, Action::Kill];

fn actions(set: HotkeySet) -> &'static [Action] {
    match set {
        HotkeySet::Idle => &ALL,
        HotkeySet::Recording => &[Action::Record, Action::Kill],
        HotkeySet::Playing => &[Action::Play, Action::Kill],
    }
}

pub fn plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, pressed, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            let Some(action) = ALL.into_iter().find(|&a| shortcut(a) == *pressed) else { return };
            let coordinator = app.state::<CoordinatorHandle>();
            match action {
                Action::Record => coordinator.send(Cmd::Input(Input::ToggleRecord)),
                Action::Play => coordinator.send(Cmd::HotkeyPlay),
                Action::Kill => coordinator.send(Cmd::Input(Input::Kill)),
                Action::Compact => app.state::<std::sync::Arc<Emitter>>().send(EngineMsg::ToggleCompact),
            }
        })
        .build()
}

/// Registers exactly the hotkeys of `set`.
pub fn apply(app: &AppHandle, set: HotkeySet, emit: &Emitter) {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    for &a in actions(set) {
        if let Err(e) = gs.register(shortcut(a)) {
            // Usually another app owns the combo; M7 surfaces conflicts in the UI.
            emit.error(format!("Couldn't register the {a:?} hotkey: {e}"));
        }
    }
}
