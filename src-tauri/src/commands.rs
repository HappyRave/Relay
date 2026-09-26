//! Tauri commands the UI calls. Each returns `Result<_, IpcError>` so the UI
//! gets a machine-readable code alongside the message.
//!
//! Commands that write files run off the main thread (`async`), so a slow
//! disk never stalls the window, the tray or the hotkeys. When saving a
//! change fails, the change is kept in memory and the user is told; the
//! command still succeeds, so the UI shows what Relay actually holds.

use std::path::Path;
use std::sync::Arc;

use parking_lot::Mutex;
use relay_core::model::{PlaybackOptions, Rgb};
use relay_core::session::{FinishReason, Input};
use relay_core::{EditOp, Macro, MacroListItem, MacroView, format};
use relay_platform::Platform;
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};
use ts_rs::TS;
use uuid::Uuid;

use crate::coordinator::{Cmd, CoordinatorHandle, SessionMode};
use crate::history::{EditHistory, Snapshot};
use crate::hotkeys;
use crate::ipc::{Emitter, EngineMsg};
use crate::library::{Library, LibraryError};
use crate::settings::{Settings, SettingsStore};

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

fn library(app: &AppHandle) -> State<'_, Mutex<Library>> {
    app.state::<Mutex<Library>>()
}

/// Reports a failed save; the change itself stays (see the module docs).
fn report_unsaved(app: &AppHandle, r: std::io::Result<()>) {
    if let Err(e) = r {
        app.state::<Arc<Emitter>>().error(format!("Couldn't save the change: {e}. It's kept until you quit."));
    }
}

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
    c.send(Cmd::Input(Input::Stop(FinishReason::Stopped)));
}

#[tauri::command]
pub fn seek(c: State<'_, CoordinatorHandle>, t: f64) {
    c.send(Cmd::Seek(t));
}

// — library —

#[tauri::command]
pub fn list_macros(lib: State<'_, Mutex<Library>>) -> Vec<MacroListItem> {
    lib.lock().list()
}

/// The UI's view of a macro, with whether it has edits to undo or redo.
fn view_of(m: &Macro, history: &EditHistory) -> MacroView {
    let mut view = MacroView::of(m);
    (view.can_undo, view.can_redo) = history.status(m.id);
    view
}

#[tauri::command]
pub fn load_macro(
    lib: State<'_, Mutex<Library>>,
    history: State<'_, EditHistory>,
    c: State<'_, CoordinatorHandle>,
    id: Uuid,
) -> Result<MacroView> {
    let lib = lib.lock();
    let entry = lib.get(id).ok_or(IpcError::not_found(id))?;
    c.send(Cmd::Select(id));
    Ok(view_of(&entry.macro_, &history))
}

#[tauri::command(async)]
pub fn edit_macro(app: AppHandle, id: Uuid, op: EditOp) -> Result<MacroView> {
    let history = app.state::<EditHistory>();
    let lib = library(&app);
    let mut lib = lib.lock();
    let entry = lib.get_mut(id).ok_or(IpcError::not_found(id))?;
    let before = Snapshot::before(&entry.macro_, &op);
    relay_core::edit::apply(&mut entry.macro_, op.clone())?;
    history.record(id, before, &op);
    let view = view_of(&entry.macro_, &history);
    report_unsaved(&app, lib.save(id));
    Ok(view)
}

/// Reverts the macro's last edit (`redo: false`) or re-applies the last undone one.
#[tauri::command(async)]
pub fn undo_edit(app: AppHandle, id: Uuid, redo: bool) -> Result<MacroView> {
    let history = app.state::<EditHistory>();
    let lib = library(&app);
    let mut lib = lib.lock();
    let entry = lib.get_mut(id).ok_or(IpcError::not_found(id))?;
    let changed = if redo { history.redo(id, &mut entry.macro_) } else { history.undo(id, &mut entry.macro_) };
    let view = view_of(&entry.macro_, &history);
    if changed {
        report_unsaved(&app, lib.save(id));
    }
    Ok(view)
}

#[tauri::command(async)]
pub fn set_playback_options(app: AppHandle, id: Uuid, options: PlaybackOptions) -> Result<MacroView> {
    let lib = library(&app);
    let mut lib = lib.lock();
    let entry = lib.get_mut(id).ok_or(IpcError::not_found(id))?;
    if entry.macro_.playback.speed != options.speed {
        app.state::<CoordinatorHandle>().send(Cmd::Speed { id, speed: options.speed as f64 });
    }
    entry.macro_.playback = options;
    let view = view_of(&entry.macro_, &app.state::<EditHistory>());
    report_unsaved(&app, lib.save(id));
    Ok(view)
}

#[tauri::command(async)]
pub fn duplicate_macro(app: AppHandle, id: Uuid) -> Result<Uuid> {
    Ok(library(&app).lock().duplicate(id)?)
}

/// Moves a macro to the trash; it can be restored with [`restore_macro`].
#[tauri::command(async)]
pub fn delete_macro(app: AppHandle, id: Uuid) -> Result<()> {
    if !app.state::<SessionMode>().is_idle() {
        return Err(IpcError { code: "busy", message: "Stop the recording or playback first".into() });
    }
    library(&app).lock().trash(id)?;
    hotkeys::refresh(&app); // its hotkey goes with it
    Ok(())
}

#[tauri::command(async)]
pub fn restore_macro(app: AppHandle, id: Uuid) -> Result<()> {
    library(&app).lock().restore(id)?;
    hotkeys::refresh(&app); // and its hotkey comes back
    Ok(())
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

/// Writes an export to `path` (chosen by the user in the save dialog).
#[tauri::command(async)]
pub fn export_macro(app: AppHandle, id: Uuid, format: ExportFormat, path: String) -> Result<()> {
    let body = export_body(&library(&app).lock(), id, format)?;
    crate::storage::write_atomic(Path::new(&path), &body).map_err(IpcError::io)
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
#[tauri::command(async)]
pub fn import_macros(app: AppHandle, paths: Vec<String>) -> ImportResult {
    // Read and parse without holding the library.
    let (macros, mut problems) = read_imports(&paths);
    let imported = match library(&app).lock().import(macros) {
        Ok(ids) => ids,
        Err(e) => {
            problems.push(e.to_string());
            Vec::new()
        }
    };
    hotkeys::refresh(&app);
    ImportResult { imported, problems }
}

/// Reads and parses each file; a file that can't be read or parsed becomes a
/// problem named after it.
fn read_imports(paths: &[String]) -> (Vec<Macro>, Vec<String>) {
    let mut problems = Vec::new();
    let macros = paths
        .iter()
        .filter_map(|path| {
            let name = Path::new(path).file_name().map_or(path.clone(), |n| n.to_string_lossy().into_owned());
            std::fs::read_to_string(path)
                .map_err(|e| e.to_string())
                .and_then(|s| format::from_rly(&s).map_err(|e| e.to_string()))
                .map_err(|e| problems.push(format!("{name}: {e}")))
                .ok()
        })
        .collect();
    (macros, problems)
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

fn trigger_status(app: &AppHandle, id: Uuid) -> Result<TriggerStatus> {
    // Copy the triggers out first: the hotkey state isn't read under the library lock.
    let triggers = library(app).lock().get(id).ok_or(IpcError::not_found(id))?.triggers.clone();
    Ok(TriggerStatus {
        next_run: crate::triggers::next_scheduled(&triggers, chrono::Local::now()).map(|t| t.to_rfc3339()),
        hotkey_error: triggers.hotkey.enabled.then(|| app.state::<hotkeys::Hotkeys>().error(id)).flatten(),
        paused: app.state::<crate::triggers::TriggerState>().paused(),
        triggers,
    })
}

#[tauri::command(async)]
pub fn get_triggers(app: AppHandle, id: Uuid) -> Result<TriggerStatus> {
    trigger_status(&app, id)
}

/// Saves a macro's triggers. A hotkey that can't work (clashes with Relay's
/// own or another macro's) is refused; one another app owns is saved and
/// reported in `hotkey_error`, which is current when this returns.
#[tauri::command(async)]
pub fn set_triggers(app: AppHandle, id: Uuid, triggers: relay_core::triggers::MacroTriggers) -> Result<TriggerStatus> {
    {
        let lib = library(&app);
        let mut lib = lib.lock();
        if triggers.hotkey.enabled
            && let Some(why) = hotkeys::conflict(&lib, id, &triggers.hotkey.combo)
        {
            return Err(IpcError { code: "hotkey", message: why });
        }
        lib.set_triggers(id, triggers)?;
    }
    hotkeys::refresh_and_wait(&app);
    trigger_status(&app, id)
}

#[tauri::command]
pub fn set_triggers_paused(c: State<'_, CoordinatorHandle>, paused: bool) {
    c.send(Cmd::SetTriggersPaused(paused));
}

/// Running apps, for the "When app launches" picker.
#[tauri::command(async)]
pub fn list_processes() -> Vec<String> {
    let mut names: Vec<String> = relay_platform::processes::ProcessWatcher::new().running().into_iter().collect();
    names.sort();
    names
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool> {
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    let r = if enabled { al.enable() } else { al.disable() };
    r.map_err(|e| IpcError { code: "autostart", message: format!("Couldn't change start with Windows: {e}") })?;
    Ok(al.is_enabled().unwrap_or(false))
}

// — window —

/// The UI measured the widget at this size (CSS px); fit the window around it.
#[tauri::command]
pub fn fit_window(
    window: tauri::WebviewWindow,
    state: State<'_, crate::window_ctl::WindowState>,
    width: f64,
    height: f64,
    expanded: bool,
) {
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

/// The close button: like closing the window (see `window_ctl::close_or_hide`).
#[tauri::command]
pub fn hide_to_tray(window: tauri::WebviewWindow) {
    if !crate::window_ctl::close_or_hide(&window) {
        window.app_handle().exit(0);
    }
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}

// — settings —

#[tauri::command]
pub fn get_settings(s: State<'_, Mutex<SettingsStore>>) -> Settings {
    s.lock().current.clone()
}

#[tauri::command(async)]
pub fn update_settings(app: AppHandle, window: tauri::WebviewWindow, settings: Settings) -> Result<Settings> {
    let current = {
        let s = app.state::<Mutex<SettingsStore>>();
        let mut s = s.lock();
        report_unsaved(&app, s.set(settings));
        s.current.clone()
    };
    // Not under the settings lock: this calls into the main thread.
    crate::window_ctl::apply_on_top(&window, current.keep_on_top, !app.state::<SessionMode>().is_idle());
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library() -> (tempfile::TempDir, Library) {
        let dir = tempfile::tempdir().unwrap();
        let (lib, _) = Library::open(dir.path());
        (dir, lib)
    }

    #[test]
    fn errors_carry_a_code_for_the_ui() {
        let id = Uuid::from_u128(7);
        let e = serde_json::to_value(IpcError::from(LibraryError::NotFound(id))).unwrap();
        assert_eq!(e["code"], "not_found");
        assert_eq!(e["message"], format!("no macro with id {id}"));
        let io = std::io::Error::other("disk full");
        assert_eq!(IpcError::from(LibraryError::Io(io)).code, "io");
        assert_eq!(IpcError::io(std::io::Error::other("disk full")).message, "Couldn't save: disk full");
        let bad = format::from_rly("nope").unwrap_err();
        assert_eq!(IpcError::from(LibraryError::Format(bad)).code, "format");
    }

    #[test]
    fn exports_are_readable_again() {
        let (_dir, lib) = library();
        let id = lib.list()[0].id;
        let original = &lib.get(id).unwrap().macro_;
        for f in [ExportFormat::Rly, ExportFormat::Json] {
            let body = export_body(&lib, id, f).unwrap();
            let back = format::from_rly(&body).unwrap();
            assert_eq!(back.name, original.name, "{f:?}");
            assert_eq!(back.events, original.events, "{f:?}");
        }
        assert_eq!(
            export_body(&lib, Uuid::from_u128(u128::MAX), ExportFormat::Rly).map(|_| ()).unwrap_err().code,
            "not_found"
        );
    }

    #[test]
    fn export_formats_use_the_ui_names() {
        assert!(matches!(serde_json::from_str::<ExportFormat>(r#""rly""#), Ok(ExportFormat::Rly)));
        assert!(matches!(serde_json::from_str::<ExportFormat>(r#""json""#), Ok(ExportFormat::Json)));
        assert!(serde_json::from_str::<ExportFormat>(r#""ahk""#).is_err());
    }

    #[test]
    fn a_broken_import_file_doesnt_stop_the_others() {
        let (_dir, lib) = library();
        let files = tempfile::tempdir().unwrap();
        let good = files.path().join("good.rly");
        let json = files.path().join("export.json");
        let broken = files.path().join("broken.rly");
        let m = &lib.get(lib.list()[0].id).unwrap().macro_;
        std::fs::write(&good, format::to_rly(m)).unwrap();
        std::fs::write(&json, format::to_export_json(m)).unwrap();
        std::fs::write(&broken, "{ this isn't a macro").unwrap();
        let missing = files.path().join("gone.rly");
        let paths: Vec<String> =
            [&good, &broken, &json, &missing].iter().map(|p| p.to_string_lossy().into_owned()).collect();

        let (macros, problems) = read_imports(&paths);
        assert_eq!(macros.len(), 2);
        assert_eq!(problems.len(), 2);
        assert!(problems[0].starts_with("broken.rly: "), "{}", problems[0]);
        assert!(problems[1].starts_with("gone.rly: "), "named by file, not full path: {}", problems[1]);
    }

    #[test]
    fn views_report_undo_and_redo() {
        let (_dir, mut lib) = library();
        let id = lib.list()[0].id;
        let history = EditHistory::default();
        let m = &mut lib.get_mut(id).unwrap().macro_;
        assert_eq!((view_of(m, &history).can_undo, view_of(m, &history).can_redo), (false, false));
        let op = EditOp::Rename { name: "Renamed".into() };
        let before = Snapshot::before(m, &op);
        relay_core::edit::apply(m, op.clone()).unwrap();
        history.record(id, before, &op);
        let v = view_of(m, &history);
        assert_eq!((v.can_undo, v.can_redo, v.name.as_str()), (true, false, "Renamed"));
        history.undo(id, m);
        let v = view_of(m, &history);
        assert_eq!((v.can_undo, v.can_redo), (false, true));
    }
}
