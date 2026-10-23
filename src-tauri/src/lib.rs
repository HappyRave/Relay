mod commands;
mod coordinator;
mod data_file;
mod engine;
mod history;
mod hotkeys;
mod ipc;
mod library;
mod logging;
mod look;
mod presence;
mod rec_thread;
mod run_history;
mod screens;
mod settings;
mod storage;
mod tray;
mod triggers;
mod window_ctl;

use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use tauri::{Manager, RunEvent, WindowEvent};

/// The app's config, plus a DevTools port (and no background throttling) when
/// `RELAY_DEVTOOLS_PORT` is set, for the end-to-end tests. WebView2's own
/// `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` does the same, but isn't honored
/// everywhere (CI runners ignore it); arguments the app passes itself always are.
fn context() -> tauri::Context {
    let mut context = tauri::generate_context!();
    if let Some(port) = std::env::var("RELAY_DEVTOOLS_PORT").ok().and_then(|p| p.parse::<u16>().ok()) {
        for w in &mut context.config_mut().app.windows {
            // Setting any arguments replaces wry's defaults, so keep them.
            let base = w
                .additional_browser_args
                .take()
                .unwrap_or_else(|| "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection".into());
            // Timers and frames run at full speed even when the window isn't in front (a CI
            // runner's often isn't), so the tests' timings mean the same everywhere.
            w.additional_browser_args = Some(format!(
                "{base} --remote-debugging-port={port} --disable-background-timer-throttling \
                 --disable-renderer-backgrounding --disable-backgrounding-occluded-windows"
            ));
        }
    }
    context
}

/// Whether Relay was started with Windows (the *Run* key passes `--autostart`),
/// and so stays in the tray until asked.
fn starts_hidden(args: impl IntoIterator<Item = impl AsRef<str>>) -> bool {
    args.into_iter().any(|a| a.as_ref() == "--autostart")
}

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
        .manage(hotkeys::Hotkeys::default())
        .manage(triggers::TriggerState::default())
        .manage(coordinator::SessionMode::default())
        .manage(presence::PresenceState::default())
        .manage(history::EditHistory::default())
        .invoke_handler(tauri::generate_handler![
            commands::subscribe_engine,
            commands::toggle_record,
            commands::toggle_play,
            commands::stop_session,
            commands::seek,
            commands::list_macros,
            commands::list_runs,
            commands::load_macro,
            commands::current_macro,
            commands::screenshot,
            commands::edit_macro,
            commands::undo_edit,
            commands::set_playback_options,
            commands::export_macro,
            commands::import_macros,
            commands::duplicate_macro,
            commands::delete_macro,
            commands::restore_macro,
            commands::sample_pixel,
            commands::pick_pixel,
            commands::paste_image,
            commands::load_image,
            commands::snip_image,
            commands::cancel_snip,
            commands::test_find_image,
            commands::show_match,
            commands::preview_text,
            commands::get_data_file,
            commands::set_data_file,
            commands::get_settings,
            commands::update_settings,
            commands::get_triggers,
            commands::set_triggers,
            commands::set_triggers_paused,
            commands::list_processes,
            commands::get_autostart,
            commands::set_autostart,
            commands::system_look,
            commands::window_prefs,
            commands::set_player_mode,
            commands::show_window,
            commands::save_panes,
            commands::hide_to_tray,
            commands::quit,
        ])
        .on_window_event(|window, event| {
            let app = window.app_handle();
            if window.label() == "player" {
                return on_player_event(app, event);
            }
            let Some(main) = app.get_webview_window("main").filter(|w| w.label() == window.label()) else { return };
            let state = app.state::<window_ctl::WindowState>();
            match event {
                WindowEvent::Moved(_) => window_ctl::on_moved(&main, &state),
                // Windows' transparency effects may have changed while Relay was in the background.
                WindowEvent::Focused(true) => {
                    let look = look::SystemLook::read();
                    let changed = std::mem::replace(&mut *app.state::<Mutex<look::SystemLook>>().lock(), look) != look;
                    if changed {
                        let settings = app.state::<Mutex<settings::SettingsStore>>().lock().current.clone();
                        for w in app.webview_windows().values() {
                            look::apply(w, &settings, look);
                        }
                        app.state::<Arc<ipc::Emitter>>().send(ipc::EngineMsg::SystemLook { look });
                    }
                }
                WindowEvent::Resized(_) => window_ctl::on_resized(&main, &state),
                // Moved onto a monitor with another scale (or its scaling changed). The
                // window system applies its own rect for the new scale after this event,
                // so re-size once that's done, not here. Called on the main thread,
                // `run_on_main_thread` runs the task right away, so post it from another.
                WindowEvent::ScaleFactorChanged { .. } => {
                    let app = app.clone();
                    std::thread::spawn(move || {
                        let _ = app.clone().run_on_main_thread(move || {
                            if let Some(main) = app.get_webview_window("main") {
                                window_ctl::rescale(&main, &app.state::<window_ctl::WindowState>());
                            }
                        });
                    });
                }
                // Alt+F4 and friends: keep running in the tray unless the user opted out.
                WindowEvent::CloseRequested { api, .. } if window_ctl::close_or_hide(app) => api.prevent_close(),
                _ => {}
            }
        })
        .setup(|app| {
            let dir = storage::data_dir(app.handle());
            app.manage(logging::init(&dir));
            let emit = Arc::new(ipc::Emitter::default());
            app.manage(emit.clone());
            // Damaged files are skipped or set aside; the user hears about them (and the log has them).
            let (library, library_problems) = library::Library::open(&dir);
            app.manage(Mutex::new(library));
            let (settings, settings_problems) = settings::SettingsStore::open(&dir);
            app.manage(Mutex::new(settings));
            let (runs, runs_problems) = run_history::RunHistory::open(&dir);
            app.manage(Mutex::new(runs));
            for p in library_problems.into_iter().chain(settings_problems).chain(runs_problems) {
                emit.error(p);
            }

            let platform = Arc::new(relay_platform::platform());
            app.manage(platform.clone());
            app.manage(coordinator::spawn(app.handle().clone(), platform.clone(), emit));
            triggers::spawn(app.handle().clone(), platform);

            // Managed before the window moves, since moving it raises events that read it.
            app.manage(window_ctl::WindowState::open(&dir));
            let window_state = app.state::<window_ctl::WindowState>();
            // The windows come once everything their first commands read is managed.
            for config in app.config().app.windows.clone() {
                tauri::WebviewWindowBuilder::from_config(app.handle(), &config)?.build()?;
            }
            let system_look = look::SystemLook::read();
            app.manage(Mutex::new(system_look));
            let settings = app.state::<Mutex<settings::SettingsStore>>().lock().current.clone();
            if let Some(window) = app.get_webview_window("main") {
                window_ctl::apply_frame(&window);
                window_ctl::watch_move_size(&window);
                look::apply(&window, &settings, system_look);
                window_ctl::init_text_size(&window_state, settings.text_size);
                // Placed where it was before it shows (it starts hidden, so it never jumps).
                window_ctl::place(&window, &window_state, window_ctl::expanded_size(&window_state.prefs()));
            }
            if let Some(player) = app.get_webview_window("player") {
                window_ctl::apply_frame(&player);
                window_ctl::set_no_activate(&player, true);
                look::apply(&player, &settings, system_look);
            }
            if starts_hidden(std::env::args()) {
                presence::set_tucked(app.handle(), true);
            } else {
                presence::refresh(app.handle(), true);
            }
            window_ctl::spawn_autosave(app.handle().clone());
            tray::create(app.handle())?;
            Ok(())
        })
        .build(context())
        .expect("error while building Relay")
        .run(|app, event| {
            // Quitting (tray, close button, Windows shutting down) mid-session:
            // stop cleanly, so no key stays held and a recording is saved.
            if let RunEvent::Exit = event {
                app.state::<coordinator::CoordinatorHandle>().shutdown(Duration::from_secs(3));
                let lost = app.state::<parking_lot::Mutex<library::Library>>().lock().save_unsaved_trash();
                if lost > 0 {
                    tracing::warn!("{lost} trashed macro(s) couldn't be written to the trash, and are gone");
                }
                tracing::info!("Relay quit");
                app.state::<logging::LogGuard>().flush();
            }
        });
}

/// The player: a drag moves its anchor; dropped on a monitor with another
/// scale, it's placed again for it once the drag ends.
fn on_player_event(app: &tauri::AppHandle, event: &WindowEvent) {
    let Some(player) = app.get_webview_window("player") else { return };
    match event {
        WindowEvent::Moved(_) => presence::on_player_moved(&player),
        WindowEvent::ScaleFactorChanged { .. } => {
            let app = app.clone();
            std::thread::spawn(move || {
                while window_ctl::in_move_loop(&player) {
                    std::thread::sleep(Duration::from_millis(50));
                }
                let _ = app.clone().run_on_main_thread(move || presence::resize_player(&app));
            });
        }
        WindowEvent::CloseRequested { api, .. } => api.prevent_close(),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_start_with_windows_stays_in_the_tray() {
        assert!(starts_hidden(["relay.exe", "--autostart"]));
        assert!(!starts_hidden(["relay.exe"]));
        assert!(!starts_hidden(["relay.exe", "--autostart=no", "autostart"]));
        assert!(!starts_hidden(Vec::<String>::new()));
    }
}
