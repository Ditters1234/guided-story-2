use super::{check_story, with_project, AppState};
use story_project::*;
use tauri::State;
#[tauri::command]
#[specta::specta]
pub fn preview_source_edit(
    story_id: String,
    world: World,
    edit: SourceEdit,
    state: State<AppState>,
) -> Result<WorldEditPreview, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.preview_source_edit(&world, edit)
    })
}
#[tauri::command]
#[specta::specta]
pub fn node_sources(
    story_id: String,
    world: World,
    node_id: String,
    state: State<AppState>,
) -> Result<Vec<NodeSourceView>, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.node_sources(&world, &node_id)
    })
}
#[tauri::command]
#[specta::specta]
pub fn read_node_source(
    story_id: String,
    world: World,
    id: String,
    state: State<AppState>,
) -> Result<SourceReading, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.read_node_source(&world, &id)
    })
}
