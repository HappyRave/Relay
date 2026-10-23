//! The tray icon: Relay keeps running (hotkeys and triggers) while
//! its windows are hidden. Left-click shows or hides Relay; the menu has the
//! session controls, the macros folder and Quit.

use relay_core::session::{FinishReason, Input, Mode};
use tauri::menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItem, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

use crate::coordinator::{Cmd, CoordinatorHandle};

const TRAY_ID: &str = "main";

/// The "Triggers active" check item, kept so its state can follow the kill switch.
pub struct TriggersItem(CheckMenuItem<tauri::Wry>);

/// The Record item, which names the record key.
pub struct RecordItem(MenuItem<tauri::Wry>);

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let keys = app.state::<parking_lot::Mutex<crate::settings::SettingsStore>>().lock().current.keys.clone();
    let triggers = CheckMenuItemBuilder::with_id("triggers", "Triggers active").checked(true).build(app)?;
    let record = MenuItemBuilder::with_id("record", record_label(&keys)).build(app)?;
    let menu = MenuBuilder::new(app)
        .item(&MenuItemBuilder::with_id("toggle", "Show / hide Relay").build(app)?)
        .item(&PredefinedMenuItem::separator(app)?)
        .item(&record)
        .item(&MenuItemBuilder::with_id("stop", "Stop\tEsc").build(app)?)
        .item(&PredefinedMenuItem::separator(app)?)
        .item(&triggers)
        .item(&PredefinedMenuItem::separator(app)?)
        .item(&MenuItemBuilder::with_id("folder", "Open macros folder").build(app)?)
        .item(&MenuItemBuilder::with_id("quit", "Quit Relay").build(app)?)
        .build()?;
    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Relay")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => toggle(app),
            "record" => app.state::<CoordinatorHandle>().send(Cmd::Input(Input::ToggleRecord)),
            "stop" => app.state::<CoordinatorHandle>().send(Cmd::Input(Input::Stop(FinishReason::Stopped))),
            "folder" => open_folder(app),
            "triggers" => {
                // The check mark already flipped; make the state follow it.
                let active = app.state::<TriggersItem>().0.is_checked().unwrap_or(true);
                app.state::<CoordinatorHandle>().send(Cmd::SetTriggersPaused(!active));
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                toggle(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    app.manage(TriggersItem(triggers));
    app.manage(RecordItem(record));
    Ok(())
}

/// Brings Relay back from the tray (or a minimized window), or puts it there.
pub fn toggle(app: &AppHandle) {
    let minimized = app.get_webview_window("main").is_some_and(|w| w.is_minimized().unwrap_or(false));
    crate::presence::set_tucked(app, !crate::presence::tucked(app) && !minimized);
}

/// Shows Relay: the window, focused, or the player in player mode.
pub fn show(app: &AppHandle) {
    crate::presence::set_tucked(app, false);
}

/// The tooltip says what Relay is doing, since its windows may be hidden.
pub fn set_mode<R: Runtime>(app: &AppHandle<R>, mode: Mode) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let keys = app.state::<parking_lot::Mutex<crate::settings::SettingsStore>>().lock().current.keys.clone();
        let _ = tray.set_tooltip(Some(tooltip(mode, keys.record.as_deref())));
    }
}

/// Relay's keys changed: the menu names the record key.
pub fn set_keys<R: Runtime>(app: &AppHandle<R>, keys: &crate::settings::RelayKeys) {
    if let Some(item) = app.try_state::<RecordItem>() {
        let _ = item.0.set_text(record_label(keys));
    }
}

fn record_label(keys: &crate::settings::RelayKeys) -> String {
    keys.record.as_deref().map_or("Record".into(), |k| format!("Record\t{k}"))
}

fn tooltip(mode: Mode, record: Option<&str>) -> String {
    match mode {
        Mode::Idle => "Relay".into(),
        Mode::Countdown => "Relay — get ready".into(),
        Mode::Recording => format!("Relay — recording ({} to stop)", record.unwrap_or("Esc")),
        Mode::Playing => "Relay — playing (Esc to stop)".into(),
        Mode::Paused => "Relay — paused".into(),
    }
}

pub fn set_triggers_active<R: Runtime>(app: &AppHandle<R>, active: bool) {
    if let Some(item) = app.try_state::<TriggersItem>() {
        let _ = item.0.set_checked(active);
    }
}

fn open_folder(app: &AppHandle) {
    let dir = crate::storage::data_dir(app).join("macros");
    let _ = std::fs::create_dir_all(&dir);
    #[cfg(windows)]
    let _ = std::process::Command::new("explorer").arg(&dir).spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tooltip_says_what_relay_is_doing() {
        assert_eq!(tooltip(Mode::Idle, Some("F9")), "Relay");
        assert_eq!(tooltip(Mode::Countdown, Some("F9")), "Relay — get ready");
        assert_eq!(tooltip(Mode::Paused, Some("F9")), "Relay — paused");
        // While input is being captured or sent, it says how to stop, with the key set.
        assert_eq!(
            tooltip(Mode::Recording, Some("Ctrl + Shift + F9")),
            "Relay — recording (Ctrl + Shift + F9 to stop)"
        );
        assert_eq!(tooltip(Mode::Recording, None), "Relay — recording (Esc to stop)");
        assert_eq!(tooltip(Mode::Playing, None), "Relay — playing (Esc to stop)");
    }

    #[test]
    fn the_menu_names_the_record_key() {
        let keys = crate::settings::RelayKeys::new_default();
        assert_eq!(record_label(&keys), "Record\tCtrl + Shift + F9");
        assert_eq!(record_label(&keys.with(crate::settings::RelayKey::Record, None)), "Record");
    }
}
