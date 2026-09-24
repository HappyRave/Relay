//! Tauri commands the UI calls. Each returns `Result<_, IpcError>` so the UI
//! gets a machine-readable code alongside the message.

use std::sync::{Arc, Mutex};

use relay_core::model::PlaybackOptions;
use relay_core::session::Input;
use relay_core::{EditOp, MacroListItem, MacroView, format};
use serde::{Deserialize, Serialize};
use tauri::State;
use tauri::ipc::Channel;
use uuid::Uuid;

use crate::coordinator::{Cmd, CoordinatorHandle};
use crate::ipc::{EngineMsg, Emitter};
use crate::library::Library;
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

#[tauri::command]
pub fn load_macro(lib: LibraryState<'_>, c: State<'_, CoordinatorHandle>, id: Uuid) -> Result<MacroView> {
    let lib = lib.lock().unwrap();
    let entry = lib.get(id).ok_or(IpcError::not_found(id))?;
    c.send(Cmd::Select(id));
    Ok(MacroView::of(&entry.macro_))
}

#[tauri::command]
pub fn edit_macro(lib: LibraryState<'_>, id: Uuid, op: EditOp) -> Result<MacroView> {
    let mut lib = lib.lock().unwrap();
    let entry = lib.get_mut(id).ok_or(IpcError::not_found(id))?;
    relay_core::edit::apply(&mut entry.macro_, op)?;
    let view = MacroView::of(&entry.macro_);
    lib.save(id).map_err(IpcError::io)?;
    Ok(view)
}

#[tauri::command]
pub fn set_playback_options(
    lib: LibraryState<'_>,
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
    let view = MacroView::of(&entry.macro_);
    lib.save(id).map_err(IpcError::io)?;
    Ok(view)
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Rly,
    Json,
}

/// The file contents for an export. M5 replaces this with a native save dialog.
#[tauri::command]
pub fn export_text(lib: LibraryState<'_>, id: Uuid, format: ExportFormat) -> Result<String> {
    let lib = lib.lock().unwrap();
    let entry = lib.get(id).ok_or(IpcError::not_found(id))?;
    Ok(match format {
        ExportFormat::Rly => format::to_rly(&entry.macro_),
        ExportFormat::Json => format::to_export_json(&entry.macro_),
    })
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
