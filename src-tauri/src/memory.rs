use super::{ai::*, preferences_root, with_project, AppState};
use story_project::*;
use tauri::{Manager, State};

fn view(app: &tauri::AppHandle, story: &str, id: &str) -> Result<MemoryView, String> {
    let jobs = app.state::<AuthoringJobs>();
    let cached = jobs
        .unwritten
        .lock()
        .map_err(|_| "Text jobs need reopening.")?
        .get(id)
        .cloned();
    with_project(&app.state::<AppState>(), |p| {
        check_story(p, story)?;
        if let Some(outcome) = cached {
            match finish_outcome(p, id, &outcome) {
                Ok(()) => {
                    jobs.unwritten
                        .lock()
                        .map_err(|_| "Text jobs need reopening.")?
                        .remove(id);
                }
                Err(error) => {
                    let mut result = p.memory_view(id)?;
                    result.stage = MemoryStage::Failed;
                    result.can_apply = false;
                    result.error=format!("The response is retained in this window but could not be saved. Keep it open and retry: {error}");
                    if let Ok(text) = outcome.text {
                        result.edits.raw_json = text.clone();
                        result.original_output = text;
                    }
                    return Ok(result);
                }
            }
        }
        p.memory_view(id)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn chapter_memory_status(
    story_id: String,
    state: State<AppState>,
) -> Result<Vec<ChapterMemoryStatus>, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.chapter_memory_status()
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn list_memory_drafts(
    story_id: String,
    state: State<AppState>,
) -> Result<Vec<MemoryDraftSummary>, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.list_memory_drafts()
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn create_memory_draft(
    story_id: String,
    id: String,
    chapter_id: String,
    app: tauri::AppHandle,
) -> Result<MemoryView, String> {
    with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.create_memory_draft(&id, &chapter_id)
    })?;
    view(&app, &story_id, &id)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn get_memory_draft(
    story_id: String,
    id: String,
    app: tauri::AppHandle,
) -> Result<MemoryView, String> {
    view(&app, &story_id, &id)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn save_memory_draft(
    story_id: String,
    id: String,
    expected_version: u32,
    edits: MemoryEdits,
    app: tauri::AppHandle,
) -> Result<MemoryView, String> {
    with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.save_memory_draft(&id, expected_version, edits)
    })?;
    view(&app, &story_id, &id)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn parse_memory_json(
    story_id: String,
    id: String,
    expected_version: u32,
    app: tauri::AppHandle,
) -> Result<MemoryView, String> {
    with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.parse_memory_json(&id, expected_version)
    })?;
    view(&app, &story_id, &id)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn memory_source_passage(
    story_id: String,
    id: String,
    passage_id: String,
    state: State<AppState>,
) -> Result<String, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.memory_source_passage(&id, &passage_id)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn start_memory_job(
    story_id: String,
    id: String,
    expected_version: u32,
    expected_revision: u32,
    app: tauri::AppHandle,
) -> Result<MemoryView, String> {
    let settings = connection(&app)?;
    settings.ready()?;
    let key = if settings.provider == TextProvider::Compatible {
        connection_api_key(&preferences_root(&app)?, &settings)?
    } else {
        None
    };
    let jobs = app.state::<AuthoringJobs>();
    let mut active = jobs
        .active
        .lock()
        .map_err(|_| "Text jobs need reopening.")?;
    if let Some(run) = active.as_ref() {
        if run.id == id && run.story_id == story_id && run.kind == TextJobKind::Memory {
            drop(active);
            return view(&app, &story_id, &id);
        }
        return Err("One text request is already running. Cancel or finish it first.".into());
    }
    if !jobs
        .unwritten
        .lock()
        .map_err(|_| "Text jobs need reopening.")?
        .is_empty()
    {
        return Err("Save the retained text response first.".into());
    }
    let prepared = with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.start_memory(&id, expected_version, expected_revision, &settings)
    })?;
    let (cancel, cancelled) = tokio::sync::oneshot::channel();
    *active = Some(ActiveRun {
        id: id.clone(),
        story_id: story_id.clone(),
        kind: TextJobKind::Memory,
        cancel: Some(cancel),
    });
    drop(active);
    let job = TextJob {
        kind: TextJobKind::Memory,
        system: prepared.system,
        user: prepared.user,
        schema: Some(prepared.schema),
        output_tokens: prepared.output_tokens,
        demo: Ok(prepared.demo),
    };
    spawn_text_job(
        app.clone(),
        id.clone(),
        story_id.clone(),
        settings,
        key,
        job,
        cancelled,
    );
    view(&app, &story_id, &id)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn apply_memory(
    story_id: String,
    id: String,
    expected_version: u32,
    app: tauri::AppHandle,
) -> Result<CommitResult, String> {
    with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.apply_memory(&id, expected_version)
    })
}
