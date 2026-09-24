//! Tauri commands the UI calls. Each returns `Result<_, IpcError>` so the UI
//! gets a machine-readable code alongside the message.

use std::sync::{Arc, Mutex};

use relay_core::model::{PlaybackOptions, Rgb};
use relay_platform::Platform;
use relay_core::session::Input;
use relay_core::{EditOp, MacroListItem, MacroView, format};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};
use tauri::ipc::Channel;
use ts_rs::TS;
use uuid::Uuid;

use crate::coordinator::{Cmd, CoordinatorHandle};
use crate::ipc::{EngineMsg, Emitter};
use crate::coordinator::SessionMode;
use crate::history::EditHistory;
use crate::library::{Library, LibraryError};
use crate::settings::{Settings, SettingsStore};

type LibraryState<'a> = State<'a, Mutex<Library>>;

#[derive(Debug, Serialize)]
pub struct IpcError {
    code: &'static str,
    message: String,
}

impl IpcError {
    fn not_found(id: Uuid) -> Self {
        IpcError { code: "not_found", message: format!("no macro with id {id}") }
    }
    fn io(e: std::io::Error) -> Self {
        IpcError { code: "io", message: format!("Couldn't save: {e}") }
    }
}

impl From<LibraryError> for IpcError {
    fn from(e: LibraryError) -> Self {
        let code = match e {
            LibraryError::NotFound(_) => "not_found",
            LibraryError::Io(_) => "io",
            LibraryError::Format(_) => "format",
        };
        IpcError { code, message: e.to_string() }
    }
}

impl From<relay_core::EditError> for IpcError {
    fn from(e: relay_core::EditError) -> Self {
        IpcError { code: "edit_rejected", message: e.to_string() }
    }
}

type Result<T> = std::result::Result<T, IpcError>;

// — session —

#[tauri::command]
pub fn subscribe_engine(emit: State<'_, Arc<Emitter>>, channel: Channel<EngineMsg>) {
    emit.subscribe(channel);
}

#[tauri::command]
pub fn toggle_record(c: State<'_, CoordinatorHandle>) {
    c.send(Cmd::Input(Input::ToggleRecord));
}

#[tauri::command]
pub fn toggle_play(c: State<'_, CoordinatorHandle>, from: f64) {
    c.send(Cmd::Input(Input::TogglePlay { from: from.max(0.0).round() as u32 }));
}

#[tauri::command]
pub fn stop_session(c: State<'_, CoordinatorHandle>) {
    c.send(Cmd::Input(Input::Stop));
}

#[tauri::command]
pub fn seek(c: State<'_, CoordinatorHandle>, t: f64) {
    c.send(Cmd::Seek(t));
}

// — library —

#[tauri::command]
pub fn list_macros(lib: LibraryState<'_>) -> Vec<MacroListItem> {
    lib.lock().unwrap().list()
}

/// The UI's view of a macro, with whether it has edits to undo or redo.
fn view_of(m: &relay_core::Macro, history: &EditHistory) -> MacroView {
    let mut view = MacroView::of(m);
    (view.can_undo, view.can_redo) = history.status(m.id);
    view
}

#[tauri::command]
pub fn load_macro(
    lib: LibraryState<'_>,
    history: State<'_, EditHistory>,
    c: State<'_, CoordinatorHandle>,
    id: Uuid,
) -> Result<MacroView> {
    let lib = lib.lock().unwrap();
    let entry = lib.get(id).ok_or(IpcError::not_found(id))?;
    c.send(Cmd::Select(id));
    Ok(view_of(&entry.macro_, &history))
}

#[tauri::command]
pub fn edit_macro(lib: LibraryState<'_>, history: State<'_, EditHistory>, id: Uuid, op: EditOp) -> Result<MacroView> {
    let mut lib = lib.lock().unwrap();
    let entry = lib.get_mut(id).ok_or(IpcError::not_found(id))?;
    let before = entry.macro_.clone();
    relay_core::edit::apply(&mut entry.macro_, op.clone())?;
    history.record(id, &before, &op);
    let view = view_of(&entry.macro_, &history);
    lib.save(id).map_err(IpcError::io)?;
    Ok(view)
}

/// Reverts the macro's last edit (`redo: false`) or re-applies the last undone one.
#[tauri::command]
pub fn undo_edit(lib: LibraryState<'_>, history: State<'_, EditHistory>, id: Uuid, redo: bool) -> Result<MacroView> {
    let mut lib = lib.lock().unwrap();
    let entry = lib.get_mut(id).ok_or(IpcError::not_found(id))?;
    let changed = if redo { history.redo(id, &mut entry.macro_) } else { history.undo(id, &mut entry.macro_) };
    let view = view_of(&entry.macro_, &history);
    if changed {
        lib.save(id).map_err(IpcError::io)?;
    }
    Ok(view)
}

#[tauri::command]
pub fn set_playback_options(
    lib: LibraryState<'_>,
    history: State<'_, EditHistory>,
    c: State<'_, CoordinatorHandle>,
    id: Uuid,
    options: PlaybackOptions,
) -> Result<MacroView> {
    let mut lib = lib.lock().unwrap();
    let entry = lib.get_mut(id).ok_or(IpcError::not_found(id))?;
    if entry.macro_.playback.speed != options.speed {
        c.send(Cmd::Speed(options.speed as f64));
    }
    entry.macro_.playback = options;
    let view = view_of(&entry.macro_, &history);
    lib.save(id).map_err(IpcError::io)?;
    Ok(view)
}

#[tauri::command]
pub fn duplicate_macro(lib: LibraryState<'_>, id: Uuid) -> Result<Uuid> {
    Ok(lib.lock().unwrap().duplicate(id)?)
}

/// Moves a macro to the trash; it can be restored with [`restore_macro`].
#[tauri::command]
pub fn delete_macro(lib: LibraryState<'_>, mode: State<'_, SessionMode>, id: Uuid) -> Result<()> {
    if !mode.is_idle() {
        return Err(IpcError { code: "busy", message: "Stop the recording or playback first".into() });
    }
    Ok(lib.lock().unwrap().trash(id)?)
}

#[tauri::command]
pub fn restore_macro(lib: LibraryState<'_>, id: Uuid) -> Result<()> {
    Ok(lib.lock().unwrap().restore(id)?)
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Rly,
    Json,
}

fn export_body(lib: &Library, id: Uuid, format: ExportFormat) -> Result<String> {
    let entry = lib.get(id).ok_or(IpcError::not_found(id))?;
    Ok(match format {
        ExportFormat::Rly => format::to_rly(&entry.macro_),
        ExportFormat::Json => format::to_export_json(&entry.macro_),
    })
}

/// The file contents for an export (the browser preview downloads these).
#[tauri::command]
pub fn export_text(lib: LibraryState<'_>, id: Uuid, format: ExportFormat) -> Result<String> {
    export_body(&lib.lock().unwrap(), id, format)
}

/// Writes an export to `path` (chosen by the user in the save dialog).
#[tauri::command]
pub fn export_macro(lib: LibraryState<'_>, id: Uuid, format: ExportFormat, path: String) -> Result<()> {
    let body = export_body(&lib.lock().unwrap(), id, format)?;
    crate::storage::write_atomic(std::path::Path::new(&path), &body).map_err(IpcError::io)
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ImportResult {
    pub imported: Vec<Uuid>,
    /// Files that couldn't be imported, with the reason.
    pub problems: Vec<String>,
}

/// Imports `.rly` files (and Relay `.json` exports). Each file is independent:
/// a broken one is reported and the rest still import.
#[tauri::command]
pub fn import_macros(lib: LibraryState<'_>, paths: Vec<String>) -> ImportResult {
    let mut lib = lib.lock().unwrap();
    let mut out = ImportResult { imported: Vec::new(), problems: Vec::new() };
    for path in paths {
        let name = std::path::Path::new(&path).file_name().map_or(path.clone(), |n| n.to_string_lossy().into_owned());
        let parsed = std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|s| format::from_rly(&s).map_err(|e| e.to_string()));
        match parsed.map(|m| lib.import(m).map_err(|e| e.to_string())) {
            Ok(Ok(id)) => out.imported.push(id),
            Ok(Err(e)) | Err(e) => out.problems.push(format!("{name}: {e}")),
        }
    }
    out
}

// — screen —

/// The color of one screen pixel (virtual-desktop coordinates).
#[tauri::command]
pub fn sample_pixel(platform: State<'_, Arc<Platform>>, x: i32, y: i32) -> Option<Rgb> {
    platform.screen.pixel(x, y)
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct PickedPixel {
    pub x: i32,
    pub y: i32,
    pub color: Rgb,
}

/// Waits `delay_ms` (so the user can point at something), then reads the
/// pixel under the cursor.
#[tauri::command]
pub async fn pick_pixel(platform: State<'_, Arc<Platform>>, delay_ms: u32) -> Result<PickedPixel> {
    let platform = platform.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms.min(10_000) as u64));
        let (x, y) = platform.screen.cursor_pos();
        platform
            .screen
            .pixel(x, y)
            .map(|color| PickedPixel { x, y, color })
            .ok_or(IpcError { code: "unavailable", message: "Couldn't read the screen there".into() })
    })
    .await
    .unwrap_or(Err(IpcError { code: "unavailable", message: "Couldn't read the screen".into() }))
}

// — triggers —

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct TriggerStatus {
    pub triggers: relay_core::triggers::MacroTriggers,
    /// RFC 3339, local time.
    pub next_run: Option<String>,
    /// Why the hotkey isn't working (taken by another app, …).
    pub hotkey_error: Option<String>,
    pub paused: bool,
}

fn trigger_status(app: &tauri::AppHandle, id: Uuid) -> Result<TriggerStatus> {
    let lib = app.state::<Mutex<Library>>();
    let lib = lib.lock().unwrap();
    let triggers = lib.get(id).ok_or(IpcError::not_found(id))?.triggers.clone();
    Ok(TriggerStatus {
        next_run: crate::triggers::next_scheduled(&triggers, chrono::Local::now()).map(|t| t.to_rfc3339()),
        hotkey_error: triggers.hotkey.enabled.then(|| app.state::<crate::hotkeys::MacroHotkeys>().error(id)).flatten(),
        paused: app.state::<crate::triggers::TriggerState>().paused(),
        triggers,
    })
}

#[tauri::command]
pub fn get_triggers(app: tauri::AppHandle, id: Uuid) -> Result<TriggerStatus> {
    trigger_status(&app, id)
}

/// Saves a macro's triggers. A hotkey that can't work (clashes with Relay's
/// own or another macro's) is refused; one another app owns is saved and
/// reported in `hotkey_error`.
#[tauri::command]
pub fn set_triggers(
    app: tauri::AppHandle,
    mode: State<'_, SessionMode>,
    id: Uuid,
    triggers: relay_core::triggers::MacroTriggers,
) -> Result<TriggerStatus> {
    {
        let lib = app.state::<Mutex<Library>>();
        let mut lib = lib.lock().unwrap();
        if triggers.hotkey.enabled
            && let Some(why) = crate::hotkeys::conflict(&lib, id, &triggers.hotkey.combo)
        {
            return Err(IpcError { code: "hotkey", message: why });
        }
        lib.set_triggers(id, triggers)?;
    }
    // Macro hotkeys are only registered while idle; a session re-registers them when it ends.
    if mode.is_idle() {
        let emit = app.state::<Arc<Emitter>>();
        crate::hotkeys::apply(&app, relay_core::session::HotkeySet::Idle, &emit);
    }
    trigger_status(&app, id)
}

#[tauri::command]
pub fn set_triggers_paused(c: State<'_, CoordinatorHandle>, paused: bool) {
    c.send(Cmd::SetTriggersPaused(paused));
}

/// Running apps, for the "When app launches" picker.
#[tauri::command]
pub fn list_processes() -> Vec<String> {
    let mut names: Vec<String> = relay_platform::processes::ProcessWatcher::new().running().into_iter().collect();
    names.sort();
    names
}

#[tauri::command]
pub fn get_autostart(app: tauri::AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<bool> {
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    let r = if enabled { al.enable() } else { al.disable() };
    r.map_err(|e| IpcError { code: "autostart", message: format!("Couldn't change start with Windows: {e}") })?;
    Ok(al.is_enabled().unwrap_or(false))
}

// — window —

/// The UI measured the widget at this size (CSS px); fit the window around it.
#[tauri::command]
pub fn fit_window(window: tauri::WebviewWindow, state: State<'_, crate::window_ctl::WindowState>, width: f64, height: f64, expanded: bool) {
    crate::window_ctl::fit(&window, &state, (width, height), expanded);
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct WindowPrefsView {
    pub expanded: bool,
}

#[tauri::command]
pub fn window_prefs(state: State<'_, crate::window_ctl::WindowState>) -> WindowPrefsView {
    WindowPrefsView { expanded: state.prefs().expanded }
}

#[tauri::command]
pub fn hide_to_tray(window: tauri::WebviewWindow, s: State<'_, Mutex<SettingsStore>>) {
    // Without the tray option the close button quits, like a normal window.
    if s.lock().unwrap().current.close_to_tray {
        let _ = window.hide();
    } else {
        window.app_handle().exit(0);
    }
}

#[tauri::command]
pub fn quit(app: tauri::AppHandle) {
    app.exit(0);
}

// — settings —

#[tauri::command]
pub fn get_settings(s: State<'_, Mutex<SettingsStore>>) -> Settings {
    s.lock().unwrap().current.clone()
}

#[tauri::command]
pub fn update_settings(s: State<'_, Mutex<SettingsStore>>, settings: Settings) -> Result<Settings> {
    let mut s = s.lock().unwrap();
    s.set(settings).map_err(IpcError::io)?;
    Ok(s.current.clone())
}
