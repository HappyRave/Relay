mod window_ctl;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                window_ctl::apply_modernist_frame(&window);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Relay");
}
