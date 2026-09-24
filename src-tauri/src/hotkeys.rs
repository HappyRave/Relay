//! Global hotkeys through RegisterHotKey (tauri-plugin-global-shortcut). They
//! fire even when an elevated window is focused, unlike a low-level hook.
//! Which ones are registered depends on the session state, so combos Relay
//! doesn't need at the moment reach other apps (and recordings). Macro
//! hotkeys are registered only while idle.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Mutex;

use relay_core::keys::code_for_label;
use relay_core::session::{HotkeySet, Input, RunSource};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use uuid::Uuid;

use crate::coordinator::{Cmd, CoordinatorHandle};
use crate::ipc::{EngineMsg, Emitter};
use crate::library::Library;

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

/// Macro hotkeys currently registered, and why others couldn't be.
#[derive(Default)]
pub struct MacroHotkeys {
    registered: Mutex<HashMap<u32, Uuid>>,
    errors: Mutex<HashMap<Uuid, String>>,
    /// Serializes `apply` (the coordinator and the set_triggers command both call it).
    applying: Mutex<()>,
}

impl MacroHotkeys {
    pub fn error(&self, id: Uuid) -> Option<String> {
        self.errors.lock().unwrap().get(&id).cloned()
    }
}

/// Parses a combo as shown in the UI ("Ctrl + Alt + 1", "Shift + F7").
/// A plain key needs at least one modifier, except the function keys.
pub fn parse_combo(combo: &str) -> Result<Shortcut, String> {
    let parts: Vec<&str> = combo.split('+').map(str::trim).filter(|p| !p.is_empty()).collect();
    let (key, mods) = parts.split_last().ok_or("The hotkey is empty")?;
    let mut m = Modifiers::empty();
    for p in mods {
        m |= match *p {
            "Ctrl" => Modifiers::CONTROL,
            "Alt" => Modifiers::ALT,
            "Shift" => Modifiers::SHIFT,
            "Win" => Modifiers::SUPER,
            other => return Err(format!("“{other}” isn't a modifier (use Ctrl, Alt, Shift or Win)")),
        };
    }
    let code = Code::from_str(&code_for_label(key)).map_err(|_| format!("“{key}” isn't a key Relay can use"))?;
    let is_fkey = matches!(key.strip_prefix('F').and_then(|n| n.parse::<u8>().ok()), Some(1..=24));
    if m.is_empty() && !is_fkey {
        return Err("Add Ctrl, Alt, Shift or Win, so the key still types normally".into());
    }
    Ok(Shortcut::new((!m.is_empty()).then_some(m), code))
}

/// Why `combo` can't be `id`'s hotkey, if it can't.
pub fn conflict(lib: &Library, id: Uuid, combo: &str) -> Option<String> {
    let s = match parse_combo(combo) {
        Ok(s) => s,
        Err(e) => return Some(e),
    };
    if ALL.iter().any(|&a| shortcut(a) == s) {
        return Some(format!("{combo} is one of Relay's own hotkeys"));
    }
    lib.all_triggers()
        .into_iter()
        .filter(|(other, t)| *other != id && t.hotkey.enabled)
        .find(|(_, t)| parse_combo(&t.hotkey.combo).is_ok_and(|o| o == s))
        .and_then(|(other, _)| lib.get(other).map(|e| format!("{combo} already runs “{}”", e.macro_.name)))
}

pub fn plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, pressed, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            let coordinator = app.state::<CoordinatorHandle>();
            if let Some(action) = ALL.into_iter().find(|&a| shortcut(a) == *pressed) {
                match action {
                    Action::Record => coordinator.send(Cmd::Input(Input::ToggleRecord)),
                    Action::Play => coordinator.send(Cmd::HotkeyPlay),
                    Action::Kill => coordinator.send(Cmd::Input(Input::Kill)),
                    Action::Compact => app.state::<std::sync::Arc<Emitter>>().send(EngineMsg::ToggleCompact),
                }
                return;
            }
            let id = app.state::<MacroHotkeys>().registered.lock().unwrap().get(&pressed.id()).copied();
            if let Some(id) = id {
                coordinator.send(Cmd::RunMacro { id, source: RunSource::Hotkey });
            }
        })
        .build()
}

/// Registers exactly the hotkeys of `set` (plus, when idle, the macro hotkeys).
pub fn apply(app: &AppHandle, set: HotkeySet, emit: &Emitter) {
    let state = app.state::<MacroHotkeys>();
    let _one_at_a_time = state.applying.lock().unwrap();
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    for &a in actions(set) {
        if let Err(e) = gs.register(shortcut(a)) {
            emit.error(format!("Couldn't register the {a:?} hotkey: {e}"));
        }
    }
    let mut registered = state.registered.lock().unwrap();
    let mut errors = state.errors.lock().unwrap();
    registered.clear();
    errors.clear();
    if set != HotkeySet::Idle {
        return;
    }
    let triggers = app.state::<Mutex<Library>>().lock().unwrap().all_triggers();
    for (id, t) in triggers.into_iter().filter(|(_, t)| t.hotkey.enabled && !t.hotkey.combo.is_empty()) {
        match parse_combo(&t.hotkey.combo) {
            Ok(s) if registered.contains_key(&s.id()) => {
                errors.insert(id, format!("{} is used by another macro", t.hotkey.combo));
            }
            // Registration fails when another app owns the combo.
            Ok(s) => match gs.register(s) {
                Ok(()) => {
                    registered.insert(s.id(), id);
                }
                Err(_) => {
                    errors.insert(id, format!("{} is taken by another app", t.hotkey.combo));
                }
            },
            Err(e) => {
                errors.insert(id, e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ui_combos() {
        assert_eq!(parse_combo("Ctrl + Alt + 1").unwrap(), Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Digit1));
        assert_eq!(parse_combo("Shift + Win + A").unwrap(), Shortcut::new(Some(Modifiers::SHIFT | Modifiers::SUPER), Code::KeyA));
        assert_eq!(parse_combo("F7").unwrap(), Shortcut::new(None, Code::F7));
        assert_eq!(parse_combo("Ctrl + Enter").unwrap(), Shortcut::new(Some(Modifiers::CONTROL), Code::Enter));
        assert!(parse_combo("A").unwrap_err().contains("Add Ctrl"));
        assert!(parse_combo("Hyper + A").unwrap_err().contains("modifier"));
        assert!(parse_combo("").is_err());
    }

    #[test]
    fn conflicts_with_relay_and_other_macros() {
        let dir = tempfile::tempdir().unwrap();
        let (mut lib, _) = Library::open(dir.path());
        let ids: Vec<Uuid> = lib.list().into_iter().map(|i| i.id).collect();
        assert!(conflict(&lib, ids[0], "Ctrl + Alt + End").unwrap().contains("Relay's own"));
        assert!(conflict(&lib, ids[0], "F9").unwrap().contains("Relay's own"));
        assert_eq!(conflict(&lib, ids[0], "Ctrl + Alt + 7"), None);
        let mut t = lib.get(ids[1]).unwrap().triggers.clone();
        t.hotkey = relay_core::triggers::HotkeyTrigger { enabled: true, combo: "Ctrl + Alt + 7".into() };
        lib.set_triggers(ids[1], t).unwrap();
        assert!(conflict(&lib, ids[0], "Ctrl + Alt + 7").unwrap().contains("Fill weekly timesheet"));
        assert_eq!(conflict(&lib, ids[1], "Ctrl + Alt + 7"), None, "its own combo is fine");
    }
}
