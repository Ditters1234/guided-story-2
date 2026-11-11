use super::{ai::*, preferences_root, with_project, AppState};
use story_project::*;
use tauri::{Manager, State};
fn view(app: &tauri::AppHandle, story: &str, id: &str) -> Result<ImagePromptView, String> {
    let text = connection(app)?;
    let image = load_image_connection(&preferences_root(app)?)?;
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
                    let mut result = p.image_prompt_view(id, &text, &image)?;
                    result.stage = ImagePromptStage::Failed;
                    result.can_apply = false;
                    result.retained = true;
                    result.error=format!("The prompt reply is retained in this window but could not be saved. Keep it open and retry: {error}");
                    if let Ok(raw) = outcome.text {
                        result.edits.raw_json = raw.clone();
                        result.original_output = raw;
                    }
                    return Ok(result);
                }
            }
        }
        p.image_prompt_view(id, &text, &image)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn list_image_prompt_drafts(
    story_id: String,
    media_id: String,
    state: State<AppState>,
) -> Result<Vec<ImagePromptSummary>, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.list_image_prompts(&media_id)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn create_image_prompt_draft(
    story_id: String,
    id: String,
    media_id: String,
    expected_media_version: u32,
    app: tauri::AppHandle,
) -> Result<ImagePromptView, String> {
    let text = connection(&app)?;
    let image = load_image_connection(&preferences_root(&app)?)?;
    with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.create_image_prompt(&id, &media_id, expected_media_version, &text, &image)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn get_image_prompt(
    story_id: String,
    id: String,
    app: tauri::AppHandle,
) -> Result<ImagePromptView, String> {
    view(&app, &story_id, &id)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn save_image_prompt_draft(
    story_id: String,
    id: String,
    expected_version: u32,
    edits: ImagePromptEdits,
    app: tauri::AppHandle,
) -> Result<ImagePromptView, String> {
    with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.save_image_prompt(&id, expected_version, edits)
    })?;
    view(&app, &story_id, &id)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn parse_image_prompt_json(
    story_id: String,
    id: String,
    expected_version: u32,
    app: tauri::AppHandle,
) -> Result<ImagePromptView, String> {
    with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.parse_image_prompt(&id, expected_version)
    })?;
    view(&app, &story_id, &id)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn start_image_prompt(
    story_id: String,
    id: String,
    expected_version: u32,
    app: tauri::AppHandle,
) -> Result<ImagePromptView, String> {
    let text = connection(&app)?;
    text.ready()?;
    let image = load_image_connection(&preferences_root(&app)?)?;
    let key = if text.provider == TextProvider::Compatible {
        connection_api_key(&preferences_root(&app)?, &text)?
    } else {
        None
    };
    let jobs = app.state::<AuthoringJobs>();
    let mut active = jobs
        .active
        .lock()
        .map_err(|_| "Text jobs need reopening.")?;
    if let Some(run) = active.as_ref() {
        if run.id == id && run.story_id == story_id && run.kind == TextJobKind::ImagePrompt {
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
        return Err("Save the retained text reply first.".into());
    }
    let prepared = with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.start_image_prompt(&id, expected_version, &text, &image)
    })?;
    let (cancel, cancelled) = tokio::sync::oneshot::channel();
    *active = Some(ActiveRun {
        id: id.clone(),
        story_id: story_id.clone(),
        kind: TextJobKind::ImagePrompt,
        cancel: Some(cancel),
    });
    drop(active);
    let job = TextJob {
        kind: TextJobKind::ImagePrompt,
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
        text,
        key,
        job,
        cancelled,
    );
    view(&app, &story_id, &id)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn apply_image_prompt(
    story_id: String,
    id: String,
    expected_version: u32,
    app: tauri::AppHandle,
) -> Result<ImagePromptApplyResult, String> {
    let text = connection(&app)?;
    let image = load_image_connection(&preferences_root(&app)?)?;
    with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.apply_image_prompt(&id, expected_version, &text, &image)
    })
}
