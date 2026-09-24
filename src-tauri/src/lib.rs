mod commands;
mod library;
mod window_ctl;

use std::sync::Mutex;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(library::Library::with_samples()))
        .invoke_handler(tauri::generate_handler![
            commands::list_macros,
            commands::load_macro,
            commands::edit_macro,
            commands::set_playback_options,
            commands::export_text,
        ])
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                window_ctl::apply_modernist_frame(&window);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Relay");
}
