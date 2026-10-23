//! Global hotkeys through RegisterHotKey (tauri-plugin-global-shortcut): Relay's
//! own keys, as the settings set them, and the macros'. They fire even when an
//! elevated window is focused, unlike a low-level hook.
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

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use parking_lot::Mutex;
use relay_core::keys::code_for_label;
use relay_core::session::{HotkeySet, Input, RunSource};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use uuid::Uuid;

use crate::coordinator::{Cmd, CoordinatorHandle};
use crate::ipc::Emitter;
use crate::library::Library;
use crate::settings::{RelayKey, RelayKeys, SettingsStore};

/// Relay's keys that are set, as shortcuts. One that doesn't parse is left
/// out (the settings refuse those).
fn shortcuts(keys: &RelayKeys) -> Vec<(RelayKey, Shortcut)> {
    RelayKey::ALL.into_iter().filter_map(|k| Some((k, parse_combo(keys.get(k)?).ok()?))).collect()
}

fn actions(set: HotkeySet) -> &'static [RelayKey] {
    match set {
        HotkeySet::Idle => &RelayKey::ALL,
        HotkeySet::Recording => &[RelayKey::Record, RelayKey::Kill],
        HotkeySet::Playing => &[RelayKey::Play, RelayKey::Kill],
    }
}

/// Which of Relay's own keys `pressed` is.
fn action_for(keys: &RelayKeys, pressed: &Shortcut) -> Option<RelayKey> {
    shortcuts(keys).into_iter().find(|(_, s)| s == pressed).map(|(k, _)| k)
}

fn current_keys<R: Runtime>(app: &AppHandle<R>) -> RelayKeys {
    app.state::<Mutex<SettingsStore>>().lock().current.keys.clone()
}

/// Why `combo` can't be Relay's `key` (`None`: turned off), the others
/// staying as they are in `keys`: it must be a hotkey Relay can register
/// (the rules of [`parse_combo`]), not another of Relay's keys, and not a
/// macro's hotkey. Stop everything can't be turned off.
pub fn check_key(lib: &Library, keys: &RelayKeys, key: RelayKey, combo: Option<&str>) -> Result<(), String> {
    let Some(combo) = combo else {
        return match key {
            RelayKey::Kill => Err("Stop everything can't be turned off. Choose another combination.".into()),
            _ => Ok(()),
        };
    };
    let s = parse_combo(combo)?;
    if let Some((other, _)) = shortcuts(keys).into_iter().find(|&(k, o)| k != key && o == s) {
        return Err(format!("{combo} is already Relay's “{}” key", other.name()));
    }
    match macro_using(lib, None, &s) {
        Some(name) => Err(format!("{combo} already runs “{name}”")),
        None => Ok(()),
    }
}

/// Why these can't be Relay's keys, if they can't (see [`check_key`]).
pub fn check_keys(lib: &Library, keys: &RelayKeys) -> Result<(), String> {
    RelayKey::ALL.into_iter().try_for_each(|k| check_key(lib, keys, k, keys.get(k)))
}

/// The macro (other than `except`) whose hotkey, on, is `s`.
fn macro_using(lib: &Library, except: Option<Uuid>, s: &Shortcut) -> Option<String> {
    lib.all_triggers()
        .iter()
        .filter(|(other, t)| Some(*other) != except && t.hotkey.enabled)
        .find(|(_, t)| parse_combo(&t.hotkey.combo).is_ok_and(|o| o == *s))
        .and_then(|(other, _)| lib.get(*other).map(|e| e.macro_.name.clone()))
}

/// The wanted hotkey set, and what the last refresh registered.
pub struct Hotkeys(Mutex<State>);

struct State {
    wanted: HotkeySet,
    /// A key field in the window is listening: no hotkey takes the keys it waits for.
    held: bool,
    /// Macro hotkeys by shortcut id.
    registered: HashMap<u32, Uuid>,
    /// Why a macro's hotkey isn't registered.
    errors: HashMap<Uuid, String>,
    reported: Reported,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Hotkeys(Mutex::new(State {
            wanted: HotkeySet::Idle,
            held: false,
            registered: HashMap::new(),
            errors: HashMap::new(),
            reported: Reported::default(),
        }))
    }
}

/// Relay's own keys whose registration failure was already reported. When
/// another app owns one, every session change registers it again and fails
/// again; the user hears about it once per run (and combination), not at every change.
#[derive(Default)]
struct Reported(HashSet<(RelayKey, String)>);

impl Reported {
    /// The messages for the failures (a key, its combo, why) not reported before.
    fn new_failures(&mut self, failed: Vec<(RelayKey, String, String)>) -> Vec<String> {
        failed
            .into_iter()
            .filter(|(k, combo, _)| self.0.insert((*k, combo.clone())))
            .map(|(k, combo, e)| {
                format!(
                    "Couldn't register Relay's “{}” key, {combo}: {e}. Choose another in Settings → Keys.",
                    k.name()
                )
            })
            .collect()
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
pub fn set_active<R: Runtime>(app: &AppHandle<R>, set: HotkeySet) {
    app.state::<Hotkeys>().0.lock().wanted = set;
    refresh(app);
}

/// Lets go of every hotkey while a key field listens (`on`), and takes them
/// back after, so the combination pressed reaches the field. Doesn't wait.
pub fn hold<R: Runtime>(app: &AppHandle<R>, on: bool) {
    app.state::<Hotkeys>().0.lock().held = on;
    refresh(app);
}

/// Re-registers the current set, e.g. after macro hotkeys changed. Doesn't wait.
pub fn refresh<R: Runtime>(app: &AppHandle<R>) {
    let a = app.clone();
    let _ = app.run_on_main_thread(move || register(&a));
}

/// Like [`refresh`], and waits until it's done, so the hotkey errors are
/// current. Must not be called on the main thread (it would wait for itself).
pub fn refresh_and_wait<R: Runtime>(app: &AppHandle<R>) {
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
fn register<R: Runtime>(app: &AppHandle<R>) {
    // Not there in the coordinator's unit tests, which mustn't take real hotkeys.
    if app.try_state::<tauri_plugin_global_shortcut::GlobalShortcut<R>>().is_none() {
        return;
    }
    let (wanted, held) = {
        let state = app.state::<Hotkeys>();
        let state = state.0.lock();
        (state.wanted, state.held)
    };
    let gs = app.global_shortcut();
    if held {
        let _ = gs.unregister_all();
        return;
    }
    let keys = current_keys(app);
    let triggers = app.state::<Mutex<Library>>().lock().all_triggers();
    let emit = app.state::<std::sync::Arc<Emitter>>();
    let _ = gs.unregister_all();
    let mut failed = Vec::new();
    for (k, s) in shortcuts(&keys).into_iter().filter(|(k, _)| actions(wanted).contains(k)) {
        if let Err(e) = gs.register(s) {
            failed.push((k, keys.get(k).unwrap_or_default().to_owned(), e.to_string()));
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
    let report = {
        let mut state = hotkeys.0.lock();
        state.registered = registered;
        state.errors = errors;
        state.reported.new_failures(failed)
    };
    for message in report {
        emit.error(message);
    }
}

/// Parses a combo as shown in the UI ("Ctrl + Alt + 1", "Shift + F7").
/// A plain key needs at least one modifier, except the function keys; Shift
/// alone isn't enough for a key that types (Shift + A is a capital A); and
/// the last key can't be a modifier itself.
pub fn parse_combo(combo: &str) -> Result<Shortcut, String> {
    let parts = relay_core::keys::split_combo(combo).ok_or("The hotkey is empty or incomplete")?;
    let (key, mods) = parts.split_last().ok_or("The hotkey is empty")?;
    let modifier = |p: &str| match p {
        "Ctrl" => Some(Modifiers::CONTROL),
        "Alt" => Some(Modifiers::ALT),
        "Shift" => Some(Modifiers::SHIFT),
        "Win" => Some(Modifiers::SUPER),
        _ => None,
    };
    let mut m = Modifiers::empty();
    for p in mods {
        m |= modifier(p).ok_or_else(|| format!("“{p}” isn't a modifier (use Ctrl, Alt, Shift or Win)"))?;
    }
    if modifier(key).is_some() {
        return Err(format!("Add a key after {key}: a hotkey can't end with a modifier"));
    }
    let code_name = code_for_label(key);
    let code = Code::from_str(&code_name).map_err(|_| format!("“{key}” isn't a key Relay can use"))?;
    let is_fkey = matches!(key.strip_prefix('F').and_then(|n| n.parse::<u8>().ok()), Some(1..=24));
    if m.is_empty() && !is_fkey {
        return Err("Add Ctrl, Alt, Shift or Win, so the key still types normally".into());
    }
    if m == Modifiers::SHIFT && types_a_character(&code_name) {
        return Err(format!("Add Ctrl, Alt or Win: Shift + {key} is ordinary typing"));
    }
    Ok(Shortcut::new((!m.is_empty()).then_some(m), code))
}

/// Whether the key with this code types something (a letter, a digit,
/// punctuation, a space or a new line), so Shift + it is just typing.
fn types_a_character(code: &str) -> bool {
    let one_of = |prefix: &str| code.strip_prefix(prefix).is_some_and(|rest| rest.len() == 1);
    one_of("Key")
        || one_of("Digit")
        || matches!(
            code,
            "Minus"
                | "Equal"
                | "BracketLeft"
                | "BracketRight"
                | "Backslash"
                | "IntlBackslash"
                | "Semicolon"
                | "Quote"
                | "Backquote"
                | "Comma"
                | "Period"
                | "Slash"
                | "Space"
                | "Enter"
        )
}

/// Why `combo` can't be `id`'s hotkey, if it can't: it's one of Relay's
/// own keys (as the library knows them) or another macro's.
pub fn conflict(lib: &Library, id: Uuid, combo: &str) -> Option<String> {
    let s = match parse_combo(combo) {
        Ok(s) => s,
        Err(e) => return Some(e),
    };
    if let Some(k) = action_for(lib.relay_keys(), &s) {
        return Some(format!("{combo} is Relay's “{}” key", k.name()));
    }
    macro_using(lib, Some(id), &s).map(|name| format!("{combo} already runs “{name}”"))
}

pub fn plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, pressed, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            let coordinator = app.state::<CoordinatorHandle>();
            if let Some(action) = action_for(&current_keys(app), pressed) {
                match action {
                    RelayKey::Record => coordinator.send(Cmd::Input(Input::ToggleRecord)),
                    RelayKey::Play => coordinator.send(Cmd::HotkeyPlay),
                    RelayKey::Kill => coordinator.send(Cmd::Input(Input::Kill)),
                    RelayKey::Player => {
                        let on = !app.state::<crate::window_ctl::WindowState>().prefs().player_mode;
                        crate::presence::set_player_mode(app, on);
                    }
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
        let (mut lib, _) = Library::open_with_samples(dir.path());
        let ids: Vec<Uuid> = lib.list().into_iter().map(|i| i.id).collect();
        assert_eq!(
            conflict(&lib, ids[0], "Ctrl + Alt + Backspace").unwrap(),
            "Ctrl + Alt + Backspace is Relay's “Stop everything” key"
        );
        assert_eq!(conflict(&lib, ids[0], "Shift + Ctrl + F9").unwrap(), "Shift + Ctrl + F9 is Relay's “Record” key");
        assert_eq!(conflict(&lib, ids[0], "F9"), None, "not one of Relay's keys here");
        // Relay's keys as the settings have them.
        lib.set_relay_keys(crate::settings::RelayKeys::legacy());
        assert_eq!(conflict(&lib, ids[0], "F9").unwrap(), "F9 is Relay's “Record” key");
        assert_eq!(conflict(&lib, ids[0], "Ctrl + Shift + F9"), None);
        assert_eq!(conflict(&lib, ids[0], "Ctrl + Alt + 7"), None);
        let mut t = lib.get(ids[1]).unwrap().triggers.clone();
        t.hotkey = relay_core::triggers::HotkeyTrigger { enabled: true, combo: "Ctrl + Alt + 7".into() };
        lib.set_triggers(ids[1], t).unwrap();
        assert!(conflict(&lib, ids[0], "Ctrl + Alt + 7").unwrap().contains("Fill weekly timesheet"));
        assert_eq!(conflict(&lib, ids[1], "Ctrl + Alt + 7"), None, "its own combo is fine");
    }

    #[test]
    fn each_session_state_registers_only_what_it_needs() {
        assert_eq!(actions(HotkeySet::Idle), RelayKey::ALL);
        // Recording: the record key stops it, and the others reach the recorded app.
        assert_eq!(actions(HotkeySet::Recording), [RelayKey::Record, RelayKey::Kill]);
        assert_eq!(actions(HotkeySet::Playing), [RelayKey::Play, RelayKey::Kill]);
        for set in [HotkeySet::Idle, HotkeySet::Recording, HotkeySet::Playing] {
            assert!(actions(set).contains(&RelayKey::Kill), "the kill switch works in {set:?}");
        }
    }

    #[test]
    fn an_own_hotkey_that_fails_is_reported_once_per_run() {
        let mut r = Reported::default();
        let taken = || (RelayKey::Record, "F9".to_string(), "HotKey already registered".to_string());
        assert_eq!(
            r.new_failures(vec![taken()]),
            [
                "Couldn't register Relay's “Record” key, F9: HotKey already registered. Choose another in Settings → Keys."
            ]
        );
        // Every session change registers again: the same failure isn't repeated.
        assert!(r.new_failures(vec![taken()]).is_empty());
        assert!(r.new_failures(vec![]).is_empty());
        assert!(r.new_failures(vec![taken()]).is_empty(), "not even after it once worked");
        // Another key failing, or the same one set to another combination, is news.
        let play = (RelayKey::Play, "F10".to_string(), "taken".to_string());
        assert_eq!(r.new_failures(vec![taken(), play]).len(), 1);
        let again = (RelayKey::Record, "Ctrl + F9".to_string(), "taken".to_string());
        assert_eq!(r.new_failures(vec![again]).len(), 1);
    }

    #[test]
    fn relays_own_keys_are_recognized_as_set() {
        let keys = RelayKeys::new_default();
        let ctrl_shift = Some(Modifiers::CONTROL | Modifiers::SHIFT);
        assert_eq!(action_for(&keys, &Shortcut::new(ctrl_shift, Code::F9)), Some(RelayKey::Record));
        assert_eq!(action_for(&keys, &Shortcut::new(ctrl_shift, Code::F10)), Some(RelayKey::Play));
        assert_eq!(action_for(&keys, &Shortcut::new(ctrl_shift, Code::KeyM)), Some(RelayKey::Player));
        assert_eq!(
            action_for(&keys, &Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::Backspace)),
            Some(RelayKey::Kill)
        );
        assert_eq!(action_for(&keys, &Shortcut::new(None, Code::F9)), None, "modifiers matter");
        let legacy = RelayKeys::legacy();
        assert_eq!(action_for(&legacy, &Shortcut::new(None, Code::F9)), Some(RelayKey::Record));
        assert_eq!(
            action_for(&legacy, &Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::End)),
            Some(RelayKey::Kill)
        );
        // A key turned off isn't one.
        let off = keys.with(RelayKey::Record, None);
        assert_eq!(action_for(&off, &Shortcut::new(ctrl_shift, Code::F9)), None);
        assert_eq!(shortcuts(&off).len(), 3);
    }

    #[test]
    fn a_relay_key_must_be_usable_and_not_taken() {
        let dir = tempfile::tempdir().unwrap();
        let (mut lib, _) = Library::open_with_samples(dir.path());
        let keys = RelayKeys::new_default();
        let check = |lib: &Library, k, c: Option<&str>| check_key(lib, &keys, k, c);
        assert_eq!(check(&lib, RelayKey::Record, Some("F8")), Ok(()));
        assert_eq!(check(&lib, RelayKey::Record, Some("Ctrl + Shift + F9")), Ok(()), "its own combination");
        assert_eq!(check(&lib, RelayKey::Record, None), Ok(()), "off: the buttons do it");
        assert_eq!(
            check(&lib, RelayKey::Kill, None).unwrap_err(),
            "Stop everything can't be turned off. Choose another combination."
        );
        assert_eq!(
            check(&lib, RelayKey::Play, Some("Shift + Ctrl + F9")).unwrap_err(),
            "Shift + Ctrl + F9 is already Relay's “Record” key"
        );
        assert!(check(&lib, RelayKey::Play, Some("Q")).unwrap_err().contains("Add Ctrl"), "the hotkey rules");
        let id = lib.list()[1].id;
        let mut t = lib.get(id).unwrap().triggers.clone();
        t.hotkey = relay_core::triggers::HotkeyTrigger { enabled: true, combo: "Ctrl + Alt + 7".into() };
        lib.set_triggers(id, t).unwrap();
        assert_eq!(
            check(&lib, RelayKey::Player, Some("Ctrl + Alt + 7")).unwrap_err(),
            "Ctrl + Alt + 7 already runs “Fill weekly timesheet”"
        );
        assert_eq!(check_keys(&lib, &keys), Ok(()));
        let clash = RelayKeys { play: Some("Ctrl + Shift + F9".into()), ..keys.clone() };
        assert!(check_keys(&lib, &clash).is_err());
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
        // A trailing "+" is the plus key; a "+" with nothing on either side is a typo.
        assert_eq!(parse_combo("Ctrl + +").unwrap(), Shortcut::new(Some(Modifiers::CONTROL), Code::Equal));
        assert!(parse_combo("Ctrl + + K").is_err());
        assert!(parse_combo(" + ").is_err());
        assert!(parse_combo("Ctrl +").is_err());
    }

    #[test]
    fn shift_alone_isnt_enough_for_a_key_that_types() {
        for combo in ["Shift + A", "Shift + 7", "Shift + -", "Shift + /", "Shift + ;", "Shift + Space", "Shift + Enter"]
        {
            let e = parse_combo(combo).unwrap_err();
            assert!(e.starts_with("Add Ctrl, Alt or Win") && e.contains("is ordinary typing"), "{combo}: {e}");
        }
        assert_eq!(parse_combo("Shift + A").unwrap_err(), "Add Ctrl, Alt or Win: Shift + A is ordinary typing");
        // Keys that don't type are fine with Shift alone.
        for (combo, code) in [
            ("Shift + F7", Code::F7),
            ("Shift + Left", Code::ArrowLeft),
            ("Shift + Home", Code::Home),
            ("Shift + PgDn", Code::PageDown),
            ("Shift + Del", Code::Delete),
            ("Shift + Tab", Code::Tab),
            ("Shift + Num 1", Code::Numpad1),
        ] {
            assert_eq!(parse_combo(combo).unwrap(), Shortcut::new(Some(Modifiers::SHIFT), code), "{combo}");
        }
        // With another modifier, a typing key is fine.
        assert_eq!(
            parse_combo("Ctrl + Shift + A").unwrap(),
            Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyA)
        );
        assert!(parse_combo("Alt + Shift + Space").is_ok());
    }

    #[test]
    fn a_hotkey_cant_end_with_a_modifier() {
        for (combo, key) in
            [("Ctrl + Shift", "Shift"), ("Alt + Ctrl", "Ctrl"), ("Shift", "Shift"), ("Ctrl + Win", "Win")]
        {
            assert_eq!(
                parse_combo(combo).unwrap_err(),
                format!("Add a key after {key}: a hotkey can't end with a modifier"),
                "{combo}"
            );
        }
        assert!(parse_combo("Ctrl + Alt").unwrap_err().contains("Add a key after Alt"));
    }

    #[test]
    fn unknown_keys_and_lowercase_modifiers_are_explained() {
        assert!(parse_combo("Ctrl + Banana").unwrap_err().contains("Banana"));
        assert!(parse_combo("ctrl + K").unwrap_err().contains("modifier"));
    }

    #[test]
    fn disabled_and_broken_hotkeys_dont_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let (mut lib, _) = Library::open_with_samples(dir.path());
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
