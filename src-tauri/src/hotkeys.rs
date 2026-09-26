//! Global hotkeys through RegisterHotKey (tauri-plugin-global-shortcut). They
//! fire even when an elevated window is focused, unlike a low-level hook.
//! Which ones are registered depends on the session state, so combos Relay
//! doesn't need at the moment reach other apps (and recordings). Macro
//! hotkeys are registered only while idle.
//!
//! Registration always runs on the main thread, one refresh at a time: the
//! plugin hands every (un)register call to the main thread anyway, and a
//! thread that waited for it while holding a lock the main thread needed
//! would deadlock. So the coordinator only records the wanted [`HotkeySet`]
//! and posts a refresh; the refresh reads the latest wanted set, so refreshes
//! from anywhere, in any order, end in the right state.

use std::collections::HashMap;
use std::str::FromStr;

use parking_lot::Mutex;
use relay_core::keys::code_for_label;
use relay_core::session::{HotkeySet, Input, RunSource};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use uuid::Uuid;

use crate::coordinator::{Cmd, CoordinatorHandle};
use crate::ipc::{Emitter, EngineMsg};
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

/// Which of Relay's own hotkeys `pressed` is.
fn action_for(pressed: &Shortcut) -> Option<Action> {
    ALL.into_iter().find(|&a| shortcut(a) == *pressed)
}

/// The wanted hotkey set, and what the last refresh registered.
pub struct Hotkeys(Mutex<State>);

struct State {
    wanted: HotkeySet,
    /// Macro hotkeys by shortcut id.
    registered: HashMap<u32, Uuid>,
    /// Why a macro's hotkey isn't registered.
    errors: HashMap<Uuid, String>,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Hotkeys(Mutex::new(State { wanted: HotkeySet::Idle, registered: HashMap::new(), errors: HashMap::new() }))
    }
}

impl Hotkeys {
    pub fn error(&self, id: Uuid) -> Option<String> {
        self.0.lock().errors.get(&id).cloned()
    }

    fn macro_for(&self, shortcut: &Shortcut) -> Option<Uuid> {
        self.0.lock().registered.get(&shortcut.id()).copied()
    }
}

/// Switches to the hotkeys of `set` (from the coordinator; doesn't wait).
pub fn set_active(app: &AppHandle, set: HotkeySet) {
    app.state::<Hotkeys>().0.lock().wanted = set;
    refresh(app);
}

/// Re-registers the current set, e.g. after macro hotkeys changed. Doesn't wait.
pub fn refresh(app: &AppHandle) {
    let a = app.clone();
    let _ = app.run_on_main_thread(move || register(&a));
}

/// Like [`refresh`], and waits until it's done, so the hotkey errors are
/// current. Must not be called on the main thread (it would wait for itself).
pub fn refresh_and_wait(app: &AppHandle) {
    let (done, wait) = crossbeam_channel::bounded(1);
    let a = app.clone();
    let posted = app.run_on_main_thread(move || {
        register(&a);
        let _ = done.send(());
    });
    if posted.is_ok() {
        let _ = wait.recv();
    }
}

/// Runs on the main thread: registers the wanted set and, when idle, the
/// macro hotkeys. No lock is held while registering.
fn register(app: &AppHandle) {
    let wanted = app.state::<Hotkeys>().0.lock().wanted;
    let triggers = app.state::<Mutex<Library>>().lock().all_triggers();
    let emit = app.state::<std::sync::Arc<Emitter>>();
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    for &a in actions(wanted) {
        if let Err(e) = gs.register(shortcut(a)) {
            emit.error(format!("Couldn't register the {a:?} hotkey: {e}"));
        }
    }
    let mut registered = HashMap::new();
    let mut errors = HashMap::new();
    if wanted == HotkeySet::Idle {
        for (id, t) in triggers.iter().filter(|(_, t)| t.hotkey.enabled && !t.hotkey.combo.is_empty()) {
            match parse_combo(&t.hotkey.combo) {
                Ok(s) if registered.contains_key(&s.id()) => {
                    errors.insert(*id, format!("{} is used by another macro", t.hotkey.combo));
                }
                // Registration fails when another app owns the combo.
                Ok(s) => match gs.register(s) {
                    Ok(()) => {
                        registered.insert(s.id(), *id);
                    }
                    Err(_) => {
                        errors.insert(*id, format!("{} is taken by another app", t.hotkey.combo));
                    }
                },
                Err(e) => {
                    errors.insert(*id, e);
                }
            }
        }
    }
    let hotkeys = app.state::<Hotkeys>();
    let mut state = hotkeys.0.lock();
    state.registered = registered;
    state.errors = errors;
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
    if action_for(&s).is_some() {
        return Some(format!("{combo} is one of Relay's own hotkeys"));
    }
    lib.all_triggers()
        .iter()
        .filter(|(other, t)| *other != id && t.hotkey.enabled)
        .find(|(_, t)| parse_combo(&t.hotkey.combo).is_ok_and(|o| o == s))
        .and_then(|(other, _)| lib.get(*other).map(|e| format!("{combo} already runs “{}”", e.macro_.name)))
}

pub fn plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, pressed, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            let coordinator = app.state::<CoordinatorHandle>();
            if let Some(action) = action_for(pressed) {
                match action {
                    Action::Record => coordinator.send(Cmd::Input(Input::ToggleRecord)),
                    Action::Play => coordinator.send(Cmd::HotkeyPlay),
                    Action::Kill => coordinator.send(Cmd::Input(Input::Kill)),
                    Action::Compact => app.state::<std::sync::Arc<Emitter>>().send(EngineMsg::ToggleCompact),
                }
                return;
            }
            if let Some(id) = app.state::<Hotkeys>().macro_for(pressed) {
                coordinator.send(Cmd::RunMacro { id, source: RunSource::Hotkey });
            }
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ui_combos() {
        assert_eq!(
            parse_combo("Ctrl + Alt + 1").unwrap(),
            Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Digit1)
        );
        assert_eq!(
            parse_combo("Shift + Win + A").unwrap(),
            Shortcut::new(Some(Modifiers::SHIFT | Modifiers::SUPER), Code::KeyA)
        );
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

    #[test]
    fn each_session_state_registers_only_what_it_needs() {
        assert_eq!(actions(HotkeySet::Idle), ALL);
        // Recording: F9 stops it, and F10/Ctrl+Shift+M reach the recorded app.
        assert_eq!(actions(HotkeySet::Recording), [Action::Record, Action::Kill]);
        assert_eq!(actions(HotkeySet::Playing), [Action::Play, Action::Kill]);
        for set in [HotkeySet::Idle, HotkeySet::Recording, HotkeySet::Playing] {
            assert!(actions(set).contains(&Action::Kill), "the kill switch works in {set:?}");
        }
    }

    #[test]
    fn relays_own_hotkeys_are_recognized() {
        assert_eq!(action_for(&Shortcut::new(None, Code::F9)), Some(Action::Record));
        assert_eq!(action_for(&Shortcut::new(None, Code::F10)), Some(Action::Play));
        assert_eq!(
            action_for(&Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyM)),
            Some(Action::Compact)
        );
        assert_eq!(
            action_for(&Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::End)),
            Some(Action::Kill)
        );
        assert_eq!(action_for(&Shortcut::new(Some(Modifiers::SHIFT), Code::F9)), None, "modifiers matter");
        assert_eq!(action_for(&Shortcut::new(None, Code::F11)), None);
    }

    #[test]
    fn function_keys_alone_are_allowed_up_to_f24() {
        assert_eq!(parse_combo("F1").unwrap(), Shortcut::new(None, Code::F1));
        assert_eq!(parse_combo("F24").unwrap(), Shortcut::new(None, Code::F24));
        assert!(parse_combo("F25").is_err());
        assert!(parse_combo("F0").is_err());
        assert!(parse_combo("Enter").unwrap_err().contains("Add Ctrl"));
    }

    #[test]
    fn combos_tolerate_spacing_and_modifier_order() {
        let want = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyK);
        assert_eq!(parse_combo("Ctrl+Shift+K").unwrap(), want);
        assert_eq!(parse_combo("  Shift +Ctrl+   K ").unwrap(), want);
        assert_eq!(parse_combo("Ctrl + Ctrl + Shift + K").unwrap(), want, "a repeated modifier counts once");
        assert_eq!(parse_combo("Ctrl + + K").unwrap(), Shortcut::new(Some(Modifiers::CONTROL), Code::KeyK));
        assert!(parse_combo(" + ").is_err());
    }

    #[test]
    fn unknown_keys_and_lowercase_modifiers_are_explained() {
        assert!(parse_combo("Ctrl + Banana").unwrap_err().contains("Banana"));
        assert!(parse_combo("ctrl + K").unwrap_err().contains("modifier"));
    }

    #[test]
    fn disabled_and_broken_hotkeys_dont_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let (mut lib, _) = Library::open(dir.path());
        let ids: Vec<Uuid> = lib.list().into_iter().map(|i| i.id).collect();
        let set = |lib: &mut Library, id, enabled, combo: &str| {
            let mut t = lib.get(id).unwrap().triggers.clone();
            t.hotkey = relay_core::triggers::HotkeyTrigger { enabled, combo: combo.into() };
            lib.set_triggers(id, t).unwrap();
        };
        set(&mut lib, ids[1], false, "Ctrl + Alt + 7");
        assert_eq!(conflict(&lib, ids[0], "Ctrl + Alt + 7"), None, "a disabled hotkey frees its combo");
        set(&mut lib, ids[1], true, "Ctrl + Banana");
        assert_eq!(conflict(&lib, ids[0], "Ctrl + Alt + 7"), None);
        // The same combo written differently still conflicts.
        set(&mut lib, ids[1], true, "Alt+Ctrl+7");
        assert!(conflict(&lib, ids[0], "Ctrl + Alt + 7").is_some());
        assert!(conflict(&lib, ids[0], "Q").unwrap().contains("Add Ctrl"), "a bad combo reports why");
    }
}
