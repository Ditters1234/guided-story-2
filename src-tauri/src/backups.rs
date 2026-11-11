use super::{ai::idle_guard, with_project, AppState, AuthoringJobs};
use std::path::Path;
use story_project::{inspect_backup, restore_backup, BackupReport};
use tauri::Manager;

#[tauri::command]
#[specta::specta]
pub(crate) async fn export_project_backup(
    story_id: String,
    expected_revision: u32,
    parent_path: String,
    operation_id: String,
    app: tauri::AppHandle,
) -> Result<BackupReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let jobs = app.state::<AuthoringJobs>();
        let _idle = idle_guard(&jobs)?;
        with_project(&app.state::<AppState>(), |project| {
            if project.view()?.story_id != story_id {
                return Err("The active story changed. Reopen backup tools.".into());
            }
            project.export_backup(Path::new(&parent_path), &operation_id, expected_revision)
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
#[specta::specta]
pub(crate) async fn inspect_project_backup(folder_path: String) -> Result<BackupReport, String> {
    tauri::async_runtime::spawn_blocking(move || inspect_backup(Path::new(&folder_path)))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
#[specta::specta]
pub(crate) async fn restore_project_backup(
    folder_path: String,
    parent_path: String,
    operation_id: String,
    fingerprint: String,
    app: tauri::AppHandle,
) -> Result<BackupReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let jobs = app.state::<AuthoringJobs>();
        let _idle = idle_guard(&jobs)?;
        let guard = app.state::<AppState>();
        let current = guard
            .0
            .lock()
            .map_err(|_| "The project session needs reopening.")?;
        let parent = Path::new(&parent_path)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if current
            .as_ref()
            .is_some_and(|p| parent.starts_with(Path::new(&p.folder_path())))
        {
            return Err("Choose a restore destination outside the open project.".into());
        }
        restore_backup(
            Path::new(&folder_path),
            &parent,
            &operation_id,
            &fingerprint,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
