mod commands;
mod coordinator;
mod engine;
mod history;
mod hotkeys;
mod ipc;
mod library;
mod logging;
mod rec_thread;
mod settings;
mod storage;
mod tray;
mod triggers;
mod window_ctl;

use std::sync::{Arc, Mutex};

use tauri::{Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // First: a second launch just brings the running Relay forward.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| tray::show(app)))
        .plugin(hotkeys::plugin())
        .plugin(tauri_plugin_dialog::init())
        // Registered in HKCU\...\Run; `--autostart` starts Relay in the tray.
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .manage(hotkeys::MacroHotkeys::default())
        .manage(triggers::TriggerState::default())
        .manage(coordinator::SessionMode::default())
        .manage(history::EditHistory::default())
        .invoke_handler(tauri::generate_handler![
            commands::subscribe_engine,
            commands::toggle_record,
            commands::toggle_play,
            commands::stop_session,
            commands::seek,
            commands::list_macros,
            commands::load_macro,
            commands::edit_macro,
            commands::undo_edit,
            commands::set_playback_options,
            commands::export_text,
            commands::export_macro,
            commands::import_macros,
            commands::duplicate_macro,
            commands::delete_macro,
            commands::restore_macro,
            commands::sample_pixel,
            commands::pick_pixel,
            commands::get_settings,
            commands::update_settings,
            commands::get_triggers,
            commands::set_triggers,
            commands::set_triggers_paused,
            commands::list_processes,
            commands::get_autostart,
            commands::set_autostart,
            commands::fit_window,
            commands::window_prefs,
            commands::hide_to_tray,
            commands::quit,
        ])
        .on_window_event(|window, event| {
            let app = window.app_handle();
            let Some(main) = app.get_webview_window("main").filter(|w| w.label() == window.label()) else { return };
            let state = app.state::<window_ctl::WindowState>();
            match event {
                WindowEvent::Moved(_) => window_ctl::on_moved(&main, &state),
                WindowEvent::ScaleFactorChanged { .. } => window_ctl::replace(&main, &state),
                // Alt+F4 and friends: keep running in the tray unless the user opted out.
                WindowEvent::CloseRequested { api, .. }
                    if app.state::<Mutex<settings::SettingsStore>>().lock().unwrap().current.close_to_tray =>
                {
                    api.prevent_close();
                    let _ = main.hide();
                }
                _ => {}
            }
        })
        .setup(|app| {
            let dir = storage::data_dir(app.handle());
            app.manage(logging::init(&dir));
            let (library, problems) = library::Library::open(&dir);
            for p in problems {
                tracing::warn!("skipped a macro file: {p}");
            }
            app.manage(Mutex::new(library));
            app.manage(Mutex::new(settings::SettingsStore::open(&dir)));

            let emit = Arc::new(ipc::Emitter::default());
            app.manage(emit.clone());
            let platform = Arc::new(relay_platform::platform());
            app.manage(platform.clone());
            app.manage(coordinator::spawn(app.handle().clone(), platform.clone(), emit));
            triggers::spawn(app.handle().clone(), platform);

            let window_state = window_ctl::WindowState::open(&dir);
            if let Some(window) = app.get_webview_window("main") {
                window_ctl::apply_modernist_frame(&window);
                // Place it where it was, then show it (the window starts hidden, so it never jumps).
                let css = if window_state.prefs().expanded { window_ctl::EXPANDED } else { window_ctl::COMPACT };
                window_ctl::place(&window, &window_state, css);
                // Started with Windows: stay in the tray until asked.
                if !std::env::args().any(|a| a == "--autostart") {
                    window.show()?;
                }
            }
            app.manage(window_state);
            window_ctl::spawn_autosave(app.handle().clone());
            tray::create(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Relay");
}
