use super::{preferences_root, with_project, AppState};
use std::{collections::BTreeMap, sync::Mutex};
use story_project::*;
use tauri::{Manager, State};
use tokio::sync::oneshot;

struct ActiveRun {
    id: String,
    story_id: String,
    cancel: Option<oneshot::Sender<()>>,
}
#[derive(Clone)]
struct Outcome {
    text: Result<String, String>,
    cancelled: bool,
    incomplete: bool,
}
#[derive(Default)]
pub(crate) struct AuthoringJobs {
    active: Mutex<Option<ActiveRun>>,
    unwritten: Mutex<BTreeMap<String, Outcome>>,
}
pub(crate) fn ensure_idle(jobs: &AuthoringJobs) -> Result<(), String> {
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
fn check_story(project: &Project, story_id: &str) -> Result<(), String> {
    if project.view()?.story_id != story_id {
        return Err("The open story changed. Reload its proposals.".into());
    }
    Ok(())
}
fn connection(app: &tauri::AppHandle) -> Result<LlmConnection, String> {
    Ok(load_connection(&preferences_root(app)?)?.settings)
}

#[tauri::command]
#[specta::specta]
pub(crate) fn load_llm_connection(app: tauri::AppHandle) -> Result<ConnectionView, String> {
    load_connection(&preferences_root(&app)?)
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
pub(crate) async fn test_llm_connection(app: tauri::AppHandle) -> Result<String, String> {
    ensure_idle(&app.state::<AuthoringJobs>())?;
    let settings = connection(&app)?;
    settings.ready()?;
    if settings.provider == TextProvider::Demo {
        return Ok("Offline demo ready. No model request was sent.".into());
    }
    let key = connection_api_key(&preferences_root(&app)?, &settings)?;
    let reply = complete_text(
        &settings,
        key.as_deref(),
        "Reply with exactly OK. This is a connection test.",
        "Connection test",
        None,
        64,
    )
    .await?;
    if reply.incomplete {
        return Err(
            "The connection responded, but the test was cut off at its output allowance.".into(),
        );
    }
    Ok(format!(
        "Connection responded: {}",
        reply.text.chars().take(120).collect::<String>()
    ))
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
            match p.finish_authoring(
                id,
                outcome.text.clone(),
                outcome.cancelled,
                outcome.incomplete,
            ) {
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
        if running.id == id && running.story_id == story_id {
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
        cancel: Some(cancel),
    });
    drop(active);
    let worker_app = app.clone();
    let worker_id = id.clone();
    let worker_story = story_id.clone();
    tauri::async_runtime::spawn(async move {
        let generate = async {
            if settings.provider == TextProvider::Demo {
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                demo_authoring(&prepared.context, &request).map(|text| CompletionText {
                    text,
                    incomplete: false,
                })
            } else {
                complete_text(
                    &settings,
                    key.as_deref(),
                    &prepared.system,
                    &prepared.user,
                    Some(&prepared.response_schema),
                    settings.max_tokens,
                )
                .await
            }
        };
        let outcome = tokio::select! {biased;_=cancelled=>Outcome{text:Err("Generation cancelled. The saved prompt and previous candidates were kept.".into()),cancelled:true,incomplete:false},reply=generate=>match reply{Ok(reply)=>Outcome{text:Ok(reply.text),cancelled:false,incomplete:reply.incomplete},Err(error)=>Outcome{text:Err(error),cancelled:false,incomplete:false}}};
        let jobs = worker_app.state::<AuthoringJobs>();
        if let Ok(mut active) = jobs.active.lock() {
            let result = with_project(&worker_app.state::<AppState>(), |p| {
                check_story(p, &worker_story)?;
                p.finish_authoring(
                    &worker_id,
                    outcome.text.clone(),
                    outcome.cancelled,
                    outcome.incomplete,
                )
            });
            if result.is_err() {
                if let Ok(mut retained) = jobs.unwritten.lock() {
                    retained.insert(worker_id.clone(), outcome);
                }
            }
            *active = None;
        };
    });
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
