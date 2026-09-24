mod commands;
mod coordinator;
mod engine;
mod hotkeys;
mod ipc;
mod library;
mod rec_thread;
mod settings;
mod storage;
mod window_ctl;

use std::sync::{Arc, Mutex};

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(hotkeys::plugin())
        .invoke_handler(tauri::generate_handler![
            commands::subscribe_engine,
            commands::toggle_record,
            commands::toggle_play,
            commands::stop_session,
            commands::seek,
            commands::list_macros,
            commands::load_macro,
            commands::edit_macro,
            commands::set_playback_options,
            commands::export_text,
            commands::get_settings,
            commands::update_settings,
        ])
        .setup(|app| {
            let dir = storage::data_dir(app.handle());
            let (library, problems) = library::Library::open(&dir);
            for p in problems {
                eprintln!("relay: skipped a macro file: {p}");
            }
            app.manage(Mutex::new(library));
            app.manage(Mutex::new(settings::SettingsStore::open(&dir)));

            let emit = Arc::new(ipc::Emitter::default());
            app.manage(emit.clone());
            let platform = Arc::new(relay_platform::platform());
            app.manage(coordinator::spawn(app.handle().clone(), platform, emit));

            if let Some(window) = app.get_webview_window("main") {
                window_ctl::apply_modernist_frame(&window);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Relay");
}
