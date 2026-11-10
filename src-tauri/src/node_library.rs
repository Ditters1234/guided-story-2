use super::*;
use std::collections::BTreeMap;
use story_project::{LibraryEntry, LibraryExport, LibraryListing, Position};

#[tauri::command]
#[specta::specta]
pub async fn list_node_library(app: tauri::AppHandle) -> Result<LibraryListing, String> {
    let root = preferences_root(&app)?;
    tauri::async_runtime::spawn_blocking(move || story_project::list_library(&root))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
#[specta::specta]
pub async fn choose_node_library(
    folder_path: String,
    app: tauri::AppHandle,
) -> Result<LibraryListing, String> {
    let root = preferences_root(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        story_project::choose_library_folder(&root, &folder_path)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
#[specta::specta]
pub fn prepare_library_export(
    world: World,
    node_ids: Vec<String>,
    positions: BTreeMap<String, Position>,
    name: String,
    description: String,
) -> Result<LibraryExport, String> {
    story_project::export_library_entry(&world, node_ids, positions, name, description)
}
#[tauri::command]
#[specta::specta]
pub async fn save_node_library(
    request_id: String,
    entry: LibraryEntry,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let root = preferences_root(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        story_project::save_library_entry(&root, &request_id, &entry)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
#[specta::specta]
pub async fn load_node_library(
    file_name: String,
    sha256: String,
    app: tauri::AppHandle,
) -> Result<LibraryEntry, String> {
    let root = preferences_root(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        story_project::read_library_entry(&root, &file_name, &sha256)
    })
    .await
    .map_err(|e| e.to_string())?
}
