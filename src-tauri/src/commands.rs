//! Tauri commands the UI calls. Each returns `Result<_, IpcError>` so the UI
//! gets a machine-readable code alongside the message.

use std::sync::Mutex;

use relay_core::model::PlaybackOptions;
use relay_core::{EditOp, MacroListItem, MacroView, format};
use serde::{Deserialize, Serialize};
use tauri::State;
use uuid::Uuid;

use crate::library::Library;

pub type LibraryState = Mutex<Library>;

#[derive(Debug, Serialize)]
pub struct IpcError {
    code: &'static str,
    message: String,
}

impl IpcError {
    fn not_found(id: Uuid) -> Self {
        IpcError { code: "not_found", message: format!("no macro with id {id}") }
    }
}

impl From<relay_core::EditError> for IpcError {
    fn from(e: relay_core::EditError) -> Self {
        IpcError { code: "edit_rejected", message: e.to_string() }
    }
}

type Result<T> = std::result::Result<T, IpcError>;

#[tauri::command]
pub fn list_macros(lib: State<'_, LibraryState>) -> Vec<MacroListItem> {
    lib.lock().unwrap().list()
}

#[tauri::command]
pub fn load_macro(lib: State<'_, LibraryState>, id: Uuid) -> Result<MacroView> {
    let lib = lib.lock().unwrap();
    let entry = lib.get(id).ok_or(IpcError::not_found(id))?;
    Ok(MacroView::of(&entry.macro_))
}

#[tauri::command]
pub fn edit_macro(lib: State<'_, LibraryState>, id: Uuid, op: EditOp) -> Result<MacroView> {
    let mut lib = lib.lock().unwrap();
    let entry = lib.get_mut(id).ok_or(IpcError::not_found(id))?;
    relay_core::edit::apply(&mut entry.macro_, op)?;
    Ok(MacroView::of(&entry.macro_))
}

#[tauri::command]
pub fn set_playback_options(lib: State<'_, LibraryState>, id: Uuid, options: PlaybackOptions) -> Result<MacroView> {
    let mut lib = lib.lock().unwrap();
    let entry = lib.get_mut(id).ok_or(IpcError::not_found(id))?;
    entry.macro_.playback = options;
    Ok(MacroView::of(&entry.macro_))
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Rly,
    Json,
}

/// The file contents for an export. M5 replaces this with a native save dialog.
#[tauri::command]
pub fn export_text(lib: State<'_, LibraryState>, id: Uuid, format: ExportFormat) -> Result<String> {
    let lib = lib.lock().unwrap();
    let entry = lib.get(id).ok_or(IpcError::not_found(id))?;
    Ok(match format {
        ExportFormat::Rly => format::to_rly(&entry.macro_),
        ExportFormat::Json => format::to_export_json(&entry.macro_),
    })
}
