use super::{ai::*, preferences_root, with_project, AppState};
use story_project::*;
use tauri::{Manager, State};
#[derive(Clone)]
pub(crate) enum ImageOutcome {
    Submission(ComfySubmission),
    Result(u32, ComfyResult),
}
#[derive(Default)]
pub(crate) struct ImageActivity {
    pub busy: bool,
    pub retained: Option<(String, String, ImageOutcome)>,
}
pub(crate) fn ensure_image_idle(jobs: &AuthoringJobs) -> Result<(), String> {
    let activity = jobs
        .media
        .lock()
        .map_err(|_| "Image activity needs reopening.")?;
    if activity.busy || activity.retained.is_some() {
        return Err("Finish the image request and save its retained result before closing or switching projects.".into());
    }
    Ok(())
}
struct Flight(tauri::AppHandle);
impl Drop for Flight {
    fn drop(&mut self) {
        if let Ok(mut activity) = self.0.state::<AuthoringJobs>().media.lock() {
            activity.busy = false;
        }
    }
}
fn begin(app: &tauri::AppHandle) -> Result<Flight, String> {
    let jobs = app.state::<AuthoringJobs>();
    let mut activity = jobs
        .media
        .lock()
        .map_err(|_| "Image activity needs reopening.")?;
    if activity.busy || activity.retained.is_some() {
        return Err("Finish the image request or save its retained result first.".into());
    }
    activity.busy = true;
    Ok(Flight(app.clone()))
}
fn apply_outcome(p: &Project, id: &str, outcome: &ImageOutcome) -> Result<(), String> {
    match outcome {
        ImageOutcome::Submission(result) => p.finish_media_submission(id, result),
        ImageOutcome::Result(version, result) => {
            p.finish_media_image(id, *version, result).map(|_| ())
        }
    }
}
fn retain_and_save(
    app: &tauri::AppHandle,
    story: &str,
    id: &str,
    outcome: ImageOutcome,
) -> Result<(), String> {
    let jobs = app.state::<AuthoringJobs>();
    {
        jobs.media
            .lock()
            .map_err(|_| "Image activity needs reopening.")?
            .retained = Some((story.into(), id.into(), outcome.clone()));
    }
    with_project(&app.state::<AppState>(),|p|{check_story(p,story)?;apply_outcome(p,id,&outcome)}) .map_err(|error|format!("The image reply is retained in this window but could not be saved. Keep it open and retry: {error}"))?;
    jobs.media
        .lock()
        .map_err(|_| "Image activity needs reopening.")?
        .retained = None;
    Ok(())
}
#[tauri::command]
#[specta::specta]
pub(crate) fn load_image_settings(app: tauri::AppHandle) -> Result<ImageConnection, String> {
    load_image_connection(&preferences_root(&app)?)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn save_image_settings(
    expected_version: u32,
    settings: ImageConnection,
    app: tauri::AppHandle,
) -> Result<ImageConnection, String> {
    save_image_connection(&preferences_root(&app)?, expected_version, settings)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn bundled_image_profile(desktop_layout: bool) -> Result<ImageWorkflowProfile, String> {
    guided_story_image_workflow(desktop_layout)
}
#[tauri::command]
#[specta::specta]
pub(crate) async fn test_image_connection(
    settings: ImageConnection,
) -> Result<ImagePreflight, String> {
    preflight_image(&settings, None).await
}
#[tauri::command]
#[specta::specta]
pub(crate) fn create_image_candidate(
    story_id: String,
    id: String,
    target: MediaTarget,
    state: State<AppState>,
) -> Result<MediaView, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.create_media(&id, target)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn get_image_candidate(
    story_id: String,
    id: String,
    app: tauri::AppHandle,
) -> Result<MediaView, String> {
    let jobs = app.state::<AuthoringJobs>();
    let retained = {
        let activity = jobs
            .media
            .lock()
            .map_err(|_| "Image activity needs reopening.")?;
        if activity.busy {
            None
        } else {
            activity.retained.clone()
        }
    };
    if let Some((story, job, outcome)) = retained {
        if story == story_id && job == id {
            retain_and_save(&app, &story_id, &id, outcome)?;
        }
    }
    with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.media_view(&id)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn list_images(
    story_id: String,
    target: Option<MediaTarget>,
    state: State<AppState>,
) -> Result<Vec<MediaEntry>, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.list_media(target.as_ref())
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn save_image_candidate(
    story_id: String,
    id: String,
    expected_version: u32,
    request: ImageWorkflowRequest,
    state: State<AppState>,
) -> Result<MediaView, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.save_media_request(&id, expected_version, request)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn preview_image_workflow(
    story_id: String,
    id: String,
    expected_version: u32,
    app: tauri::AppHandle,
) -> Result<MediaSubmission, String> {
    let settings = load_image_connection(&preferences_root(&app)?)?;
    with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.prepare_media(&id, expected_version, &settings)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) async fn submit_image(
    story_id: String,
    id: String,
    expected_version: u32,
    app: tauri::AppHandle,
) -> Result<MediaView, String> {
    let _flight = begin(&app)?;
    let settings = load_image_connection(&preferences_root(&app)?)?;
    let prepared = with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.prepare_media(&id, expected_version, &settings)
    })?;
    let preflight = preflight_image(&settings, Some(&prepared.graph)).await?;
    if !preflight.ready {
        return Err(preflight.issues.join("\n"));
    }
    with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.begin_media_submission(&id, expected_version, prepared.clone())
    })?;
    let reply = submit_comfy(&prepared, &id).await;
    retain_and_save(&app, &story_id, &id, ImageOutcome::Submission(reply))?;
    get_image_candidate(story_id, id, app)
}
#[tauri::command]
#[specta::specta]
pub(crate) async fn check_image(
    story_id: String,
    id: String,
    app: tauri::AppHandle,
) -> Result<MediaView, String> {
    let _flight = begin(&app)?;
    let d = with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.media_draft(&id)
    })?;
    if d.stage != MediaStage::Queued {
        return Err("Choose a submitted image with a known ComfyUI prompt ID.".into());
    }
    let reply = check_comfy(d.submission.as_ref().ok_or("Missing submitted workflow.")?).await?;
    retain_and_save(&app, &story_id, &id, ImageOutcome::Result(d.version, reply))?;
    get_image_candidate(story_id, id, app)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn recover_image_prompt(
    story_id: String,
    id: String,
    expected_version: u32,
    prompt_id: String,
    state: State<AppState>,
) -> Result<MediaView, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.attach_media_prompt(&id, expected_version, prompt_id)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn import_image(
    story_id: String,
    id: String,
    expected_version: u32,
    path: String,
    state: State<AppState>,
) -> Result<MediaView, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.import_media_image(&id, expected_version, std::path::Path::new(&path))
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn image_preview(
    story_id: String,
    id: String,
    accepted: bool,
    size: Option<u32>,
    state: State<AppState>,
) -> Result<Vec<u8>, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.media_thumbnail(&id, accepted, size.unwrap_or(768))
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn accept_image(
    story_id: String,
    id: String,
    expected_version: u32,
    state: State<AppState>,
) -> Result<CommitResult, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.accept_media(&id, expected_version)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn choose_node_image(
    story_id: String,
    node_id: String,
    asset_id: String,
    request_id: String,
    state: State<AppState>,
) -> Result<CommitResult, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.select_portrait(&node_id, &asset_id, request_id)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn node_image(
    story_id: String,
    node_id: String,
    world: World,
    state: State<AppState>,
) -> Result<Option<MediaEntry>, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.selected_portrait(&node_id, &world)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn clear_node_image(
    story_id: String,
    node_id: String,
    request_id: String,
    state: State<AppState>,
) -> Result<CommitResult, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.clear_portrait(&node_id, request_id)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn saved_image(
    story_id: String,
    id: String,
    state: State<AppState>,
) -> Result<MediaAsset, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.media_asset(&id)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn map_images(
    story_id: String,
    world: World,
    state: State<AppState>,
) -> Result<std::collections::BTreeMap<String, MediaEntry>, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.node_images(&world)
    })
}

#[tauri::command]
#[specta::specta]
pub(crate) fn refresh_image_guidance(
    story_id: String,
    id: String,
    version: u32,
    state: State<AppState>,
) -> Result<MediaView, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.refresh_image_guidance(&id, version)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn preview_image_preparation(
    story_id: String,
    id: String,
    accepted: bool,
    state: State<AppState>,
) -> Result<ImagePreparationPreview, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.preview_image_preparation(&id, accepted)
    })
}
