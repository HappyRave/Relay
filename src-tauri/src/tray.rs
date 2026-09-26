//! The tray icon: Relay keeps running (hotkeys and triggers) while
//! the widget is hidden. Left-click toggles the widget; the menu has the
//! session controls, the macros folder and Quit.

use relay_core::session::{FinishReason, Input, Mode};
use tauri::menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::coordinator::{Cmd, CoordinatorHandle};

const TRAY_ID: &str = "main";

/// The "Triggers active" check item, kept so its state can follow the kill switch.
pub struct TriggersItem(CheckMenuItem<tauri::Wry>);

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let triggers = CheckMenuItemBuilder::with_id("triggers", "Triggers active").checked(true).build(app)?;
    let menu = MenuBuilder::new(app)
        .item(&MenuItemBuilder::with_id("toggle", "Show / hide Relay").build(app)?)
        .item(&PredefinedMenuItem::separator(app)?)
        .item(&MenuItemBuilder::with_id("record", "Record\tF9").build(app)?)
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
    Ok(())
}

/// Shows and focuses the widget, or hides it.
pub fn toggle(app: &AppHandle) {
    let Some(w) = app.get_webview_window("main") else { return };
    if w.is_visible().unwrap_or(false) {
        let _ = w.hide();
    } else {
        show(app);
    }
}

pub fn show(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// The tooltip says what Relay is doing, since the widget may be hidden.
pub fn set_mode(app: &AppHandle, mode: Mode) {
    let text = match mode {
        Mode::Idle => "Relay",
        Mode::Countdown => "Relay — get ready",
        Mode::Recording => "Relay — recording (F9 to stop)",
        Mode::Playing => "Relay — playing (Esc to stop)",
        Mode::Paused => "Relay — paused",
    };
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(text));
    }
}

pub fn set_triggers_active(app: &AppHandle, active: bool) {
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
