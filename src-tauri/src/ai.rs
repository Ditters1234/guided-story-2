use super::{preferences_root, with_project, AppState};
use std::{collections::BTreeMap, sync::Mutex};
use story_project::*;
use tauri::{Manager, State};
use tokio::sync::oneshot;

pub(crate) struct ActiveRun {
    pub id: String,
    pub story_id: String,
    pub kind: TextJobKind,
    pub cancel: Option<oneshot::Sender<()>>,
}
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum TextJobKind {
    World,
    Narration,
    Effects,
    Memory,
    ImagePrompt,
}
#[derive(Clone)]
pub(crate) struct Outcome {
    pub text: Result<String, String>,
    pub cancelled: bool,
    pub incomplete: bool,
    pub kind: TextJobKind,
    pub settings: LlmConnection,
}
#[derive(Default)]
pub(crate) struct AuthoringJobs {
    pub active: Mutex<Option<ActiveRun>>,
    pub unwritten: Mutex<BTreeMap<String, Outcome>>,
    pub media: Mutex<super::media::ImageActivity>,
}
pub(crate) fn idle_guard(
    jobs: &AuthoringJobs,
) -> Result<
    (
        std::sync::MutexGuard<'_, Option<ActiveRun>>,
        std::sync::MutexGuard<'_, super::media::ImageActivity>,
    ),
    String,
> {
    let guard = jobs
        .active
        .lock()
        .map_err(|_| "Authoring state needs reopening.")?;
    if guard.is_some()
        || !jobs
            .unwritten
            .lock()
            .map_err(|_| "Authoring state needs reopening.")?
            .is_empty()
    {
        return Err(
            "Finish/cancel generation and save retained results before switching projects.".into(),
        );
    }
    let media = jobs
        .media
        .lock()
        .map_err(|_| "Image activity needs reopening.")?;
    if media.busy || media.retained.is_some() {
        return Err(
            "Finish the image request and save its retained result before switching projects."
                .into(),
        );
    }
    Ok((guard, media))
}
pub(crate) fn ensure_idle(jobs: &AuthoringJobs) -> Result<(), String> {
    super::media::ensure_image_idle(jobs)?;
    if jobs
        .active
        .lock()
        .map_err(|_| "Authoring state needs to be reopened.")?
        .is_some()
    {
        return Err("Cancel or finish generation before changing projects.".into());
    }
    if !jobs
        .unwritten
        .lock()
        .map_err(|_| "Authoring state needs to be reopened.")?
        .is_empty()
    {
        return Err("Save the retained AI result before changing projects or closing.".into());
    }
    Ok(())
}
pub(crate) fn check_story(project: &Project, story_id: &str) -> Result<(), String> {
    if project.view()?.story_id != story_id {
        return Err("The open story changed. Reload its proposals.".into());
    }
    Ok(())
}
pub(crate) fn connection(app: &tauri::AppHandle) -> Result<LlmConnection, String> {
    Ok(load_connection(&preferences_root(app)?)?.settings)
}
pub(crate) fn finish_outcome(p: &Project, id: &str, outcome: &Outcome) -> Result<(), String> {
    match outcome.kind {
        TextJobKind::World => p.finish_authoring(
            id,
            outcome.text.clone(),
            outcome.cancelled,
            outcome.incomplete,
        ),
        TextJobKind::Memory => p.finish_memory(
            id,
            outcome.text.clone(),
            outcome.cancelled,
            outcome.incomplete,
        ),
        TextJobKind::ImagePrompt => p.finish_image_prompt(
            id,
            outcome.text.clone(),
            outcome.cancelled,
            outcome.incomplete,
        ),
        kind => p.finish_passage(
            id,
            kind == TextJobKind::Effects,
            outcome.text.clone(),
            outcome.cancelled,
            outcome.incomplete,
            &outcome.settings,
        ),
    }
}
pub(crate) struct TextJob {
    pub kind: TextJobKind,
    pub system: String,
    pub user: String,
    pub schema: Option<serde_json::Value>,
    pub output_tokens: u32,
    pub demo: Result<String, String>,
}
pub(crate) fn spawn_text_job(
    app: tauri::AppHandle,
    id: String,
    story: String,
    settings: LlmConnection,
    key: Option<String>,
    job: TextJob,
    cancelled: oneshot::Receiver<()>,
) {
    tauri::async_runtime::spawn(async move {
        let generate = async {
            if settings.provider == TextProvider::Demo {
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                job.demo.clone().map(|text| CompletionText {
                    text,
                    incomplete: false,
                })
            } else {
                complete_text(
                    &settings,
                    key.as_deref(),
                    &job.system,
                    &job.user,
                    job.schema.as_ref(),
                    job.output_tokens,
                )
                .await
            }
        };
        let (text, was_cancelled, incomplete) = tokio::select! {biased;_=cancelled=>(Err("Request cancelled. Saved prompts and outputs were kept.".into()),true,false),reply=generate=>match reply{Ok(reply)=>(Ok(reply.text),false,reply.incomplete),Err(error)=>(Err(error),false,false)}};
        let outcome = Outcome {
            text,
            cancelled: was_cancelled,
            incomplete,
            kind: job.kind,
            settings,
        };
        let jobs = app.state::<AuthoringJobs>();
        if let Ok(mut active) = jobs.active.lock() {
            let result = with_project(&app.state::<AppState>(), |p| {
                check_story(p, &story)?;
                finish_outcome(p, &id, &outcome)
            });
            if result.is_err() {
                if let Ok(mut retained) = jobs.unwritten.lock() {
                    retained.insert(id, outcome);
                }
            }
            *active = None;
        };
    });
}

#[tauri::command]
#[specta::specta]
pub(crate) fn load_llm_connection(app: tauri::AppHandle) -> Result<ConnectionView, String> {
    load_connection(&preferences_root(&app)?)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn load_prompt_templates(
    app: tauri::AppHandle,
) -> Result<story_project::PromptSettingsView, String> {
    Ok(load_connection(&preferences_root(&app)?)?
        .settings
        .prompt_settings_view())
}
#[tauri::command]
#[specta::specta]
pub(crate) fn save_prompt_templates(
    expected_version: u32,
    overrides: std::collections::BTreeMap<story_project::TextPromptTask, String>,
    app: tauri::AppHandle,
) -> Result<story_project::PromptSettingsView, String> {
    story_project::save_prompt_templates(&preferences_root(&app)?, expected_version, overrides)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn save_llm_connection(
    expected_version: u32,
    settings: LlmConnection,
    key: ApiKeyUpdate,
    app: tauri::AppHandle,
) -> Result<ConnectionView, String> {
    save_connection(&preferences_root(&app)?, expected_version, settings, key)
}
#[tauri::command]
#[specta::specta]
pub(crate) async fn test_llm_connection(app: tauri::AppHandle) -> Result<ConnectionTest, String> {
    ensure_idle(&app.state::<AuthoringJobs>())?;
    let settings = connection(&app)?;
    settings.validate()?;
    if settings.provider == TextProvider::Demo {
        return Ok(ConnectionTest {
            models: vec![],
            message: "Offline demo ready. No API request was sent.".into(),
        });
    }
    let key = connection_api_key(&preferences_root(&app)?, &settings)?;
    let models = discover_models(&settings, key.as_deref()).await?;
    let message = format!(
        "Connected. {} model{} available.",
        models.len(),
        if models.len() == 1 { "" } else { "s" }
    );
    Ok(ConnectionTest { models, message })
}

#[tauri::command]
#[specta::specta]
pub(crate) fn list_authoring(
    story_id: String,
    state: State<AppState>,
) -> Result<Vec<AuthoringSummary>, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.authoring_list()
    })
}
fn view(app: &tauri::AppHandle, story_id: &str, id: &str) -> Result<AuthoringView, String> {
    let settings = connection(app)?;
    let state = app.state::<AppState>();
    let jobs = app.state::<AuthoringJobs>();
    let cached = jobs
        .unwritten
        .lock()
        .map_err(|_| "Authoring state needs reopening.")?
        .get(id)
        .cloned();
    with_project(&state, |p| {
        check_story(p, story_id)?;
        if let Some(outcome) = cached {
            match finish_outcome(p, id, &outcome) {
                Ok(()) => {
                    jobs.unwritten
                        .lock()
                        .map_err(|_| "Authoring state needs reopening.")?
                        .remove(id);
                }
                Err(error) => {
                    let mut view = p.authoring_view(id, &settings)?;
                    view.status = AuthoringStatus::Failed;
                    view.error=format!("The result is retained in this window but could not be saved. Keep it open and retry: {error}");
                    if let Ok(text) = outcome.text {
                        view.raw_response = text.clone();
                        view.candidate_text = text;
                    }
                    return Ok(view);
                }
            }
        }
        p.authoring_view(id, &settings)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn get_authoring(
    story_id: String,
    id: String,
    app: tauri::AppHandle,
) -> Result<AuthoringView, String> {
    view(&app, &story_id, &id)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn save_authoring_draft(
    story_id: String,
    id: String,
    expected_version: u32,
    request: AuthoringRequest,
    app: tauri::AppHandle,
) -> Result<AuthoringView, String> {
    with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.save_authoring_draft(&id, expected_version, request)
    })?;
    view(&app, &story_id, &id)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn save_authoring_candidate(
    story_id: String,
    id: String,
    expected_version: u32,
    text: String,
    app: tauri::AppHandle,
) -> Result<AuthoringView, String> {
    with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.save_authoring_candidate(&id, expected_version, text)
    })?;
    view(&app, &story_id, &id)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn start_authoring(
    story_id: String,
    id: String,
    expected_version: u32,
    expected_revision: u32,
    app: tauri::AppHandle,
) -> Result<AuthoringView, String> {
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
        .map_err(|_| "Authoring state needs reopening.")?;
    if let Some(running) = active.as_ref() {
        if running.id == id && running.story_id == story_id && running.kind == TextJobKind::World {
            drop(active);
            return view(&app, &story_id, &id);
        }
        return Err("One text request is already running. Cancel or finish it first.".into());
    }
    if !jobs
        .unwritten
        .lock()
        .map_err(|_| "Authoring state needs reopening.")?
        .is_empty()
    {
        return Err("Save the retained result before starting another request.".into());
    }
    let (prepared, request) = with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        let prepared = p.start_authoring(&id, expected_version, expected_revision, &settings)?;
        let request = p.authoring_view(&id, &settings)?.request;
        Ok((prepared, request))
    })?;
    let (cancel, cancelled) = oneshot::channel();
    *active = Some(ActiveRun {
        id: id.clone(),
        story_id: story_id.clone(),
        kind: TextJobKind::World,
        cancel: Some(cancel),
    });
    drop(active);
    let job = TextJob {
        kind: TextJobKind::World,
        system: prepared.system,
        user: prepared.user,
        schema: Some(prepared.response_schema),
        output_tokens: settings.max_tokens,
        demo: demo_authoring(&prepared.context, &request),
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
pub(crate) fn cancel_authoring(id: String, jobs: State<AuthoringJobs>) -> Result<(), String> {
    let mut active = jobs
        .active
        .lock()
        .map_err(|_| "Authoring state needs reopening.")?;
    if let Some(run) = active.as_mut() {
        if run.id != id {
            return Err("A different request is running.".into());
        }
        if let Some(cancel) = run.cancel.take() {
            let _ = cancel.send(());
        }
    }
    Ok(())
}
#[tauri::command]
#[specta::specta]
pub(crate) fn apply_authoring(
    story_id: String,
    id: String,
    expected_version: u32,
    app: tauri::AppHandle,
) -> Result<ApplyAuthoringResult, String> {
    let settings = connection(&app)?;
    with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.apply_authoring(&id, expected_version, &settings)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) async fn close_authoring(app: tauri::AppHandle) -> Result<(), String> {
    {
        let jobs = app.state::<AuthoringJobs>();
        let mut active = jobs
            .active
            .lock()
            .map_err(|_| "Authoring state needs reopening.")?;
        if let Some(run) = active.as_mut() {
            if let Some(cancel) = run.cancel.take() {
                let _ = cancel.send(());
            }
        }
    }
    for _ in 0..100 {
        {
            let jobs = app.state::<AuthoringJobs>();
            if jobs
                .active
                .lock()
                .map_err(|_| "Authoring state needs reopening.")?
                .is_none()
            {
                return ensure_idle(&jobs);
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    Err("Generation is still stopping. Keep this window open and retry closing.".into())
}
