use std::{path::Path, sync::Mutex};
use story_project::{CommitResult, EditorDraft, Project, ProjectView, Workspace, World};
use tauri::State;

#[derive(Default)]
struct AppState(Mutex<Option<Project>>);

fn with_project<T>(
    state: &AppState,
    action: impl FnOnce(&Project) -> Result<T, String>,
) -> Result<T, String> {
    let guard = state
        .0
        .lock()
        .map_err(|_| "The project session needs to be reopened.".to_string())?;
    action(guard.as_ref().ok_or("Open or create a project first.")?)
}

#[tauri::command]
#[specta::specta]
fn create_project(
    parent_path: String,
    title: String,
    world: World,
    request_id: String,
    state: State<AppState>,
) -> Result<ProjectView, String> {
    let parent = Path::new(&parent_path)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !parent.is_dir() {
        return Err("Choose a parent folder for your story.".into());
    }
    story_project::validate_id(&request_id)?;
    let root = parent.join(format!("guided-story-{}", &request_id[..8]));
    let mut guard = state
        .0
        .lock()
        .map_err(|_| "The project session needs to be reopened.")?;
    let project = Project::create(&root, title, world, request_id)?;
    let view = project.view()?;
    *guard = Some(project);
    Ok(view)
}

#[tauri::command]
#[specta::specta]
fn open_project(folder_path: String, state: State<AppState>) -> Result<ProjectView, String> {
    let path = Path::new(&folder_path)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let mut guard = state
        .0
        .lock()
        .map_err(|_| "The project session needs to be reopened.")?;
    if let Some(current) = guard.as_ref() {
        if Path::new(&current.folder_path()) == path {
            return current.view();
        }
    }
    let project = Project::open(&path)?;
    let view = project.view()?;
    *guard = Some(project);
    Ok(view)
}

#[tauri::command]
#[specta::specta]
fn startup_project(state: State<AppState>) -> Result<Option<ProjectView>, String> {
    let arguments: Vec<String> = std::env::args().collect();
    if let Some(pair) = arguments.windows(2).find(|pair| pair[0] == "--project") {
        return open_project(pair[1].clone(), state).map(Some);
    }
    Ok(None)
}

#[tauri::command]
#[specta::specta]
fn recover_project(folder_path: String, state: State<AppState>) -> Result<ProjectView, String> {
    let mut guard = state
        .0
        .lock()
        .map_err(|_| "The project session needs to be reopened.")?;
    if guard
        .as_ref()
        .is_some_and(|p| Path::new(&p.folder_path()) == Path::new(&folder_path))
    {
        return Err("Reopen the app before recovering the project currently in use.".into());
    }
    let project = Project::recover(Path::new(&folder_path))?;
    let view = project.view()?;
    *guard = Some(project);
    Ok(view)
}

#[tauri::command]
#[specta::specta]
fn save_project(
    expected_revision: u32,
    request_id: String,
    title: String,
    world: World,
    state: State<AppState>,
) -> Result<CommitResult, String> {
    with_project(&state, |project| {
        project.commit(expected_revision, request_id, title, world)
    })
}

#[tauri::command]
#[specta::specta]
fn load_draft(state: State<AppState>) -> Result<Option<EditorDraft>, String> {
    with_project(&state, Project::load_draft)
}

#[tauri::command]
#[specta::specta]
fn save_draft(draft: EditorDraft, state: State<AppState>) -> Result<(), String> {
    with_project(&state, |project| project.save_draft(draft))
}

#[tauri::command]
#[specta::specta]
fn clear_matching_draft(title: String, world: World, state: State<AppState>) -> Result<(), String> {
    with_project(&state, |project| {
        project.clear_matching_draft(&title, &world)
    })
}

#[tauri::command]
#[specta::specta]
fn load_workspace(state: State<AppState>) -> Result<Workspace, String> {
    with_project(&state, Project::load_workspace)
}

#[tauri::command]
#[specta::specta]
fn save_workspace(
    expected_version: u32,
    workspace: Workspace,
    state: State<AppState>,
) -> Result<Workspace, String> {
    with_project(&state, |project| {
        project.save_workspace(expected_version, workspace)
    })
}

fn command_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::new().commands(tauri_specta::collect_commands![
        create_project,
        open_project,
        startup_project,
        recover_project,
        save_project,
        load_workspace,
        save_workspace,
        load_draft,
        save_draft,
        clear_matching_draft
    ])
}

pub fn run() {
    let builder = command_builder();
    tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(builder.invoke_handler())
        .run(tauri::generate_context!())
        .expect("The desktop application could not start");
}

pub fn export_bindings() {
    command_builder()
        .export(
            specta_typescript::Typescript::default(),
            concat!(env!("CARGO_MANIFEST_DIR"), "/../src/lib/bindings.ts"),
        )
        .expect("Could not export typed commands");
}
