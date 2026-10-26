use super::{ai::*, preferences_root, with_project, AppState};
use story_project::*;
use tauri::{Manager, State};

fn view(app: &tauri::AppHandle, story: &str, id: &str) -> Result<PassageView, String> {
    let settings = connection(app)?;
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
                    let mut v = p.passage_view(id, &settings)?;
                    v.stage = PassageStage::Failed;
                    v.can_accept = false;
                    v.error=format!("The response is retained in this window but could not be saved. Keep it open and retry: {error}");
                    if let Ok(text) = outcome.text {
                        if outcome.kind == TextJobKind::Effects {
                            v.edits.effects_text = text.clone();
                            v.original_effects = text;
                        } else {
                            v.edits.prose = text.clone();
                            v.original_prose = text;
                        }
                    }
                    return Ok(v);
                }
            }
        }
        p.passage_view(id, &settings)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn load_story(story_id: String, state: State<AppState>) -> Result<StoryIndex, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.story_index()
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn next_passage_plan(
    story_id: String,
    state: State<AppState>,
) -> Result<PassagePlan, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.next_passage_plan()
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn list_passage_drafts(
    story_id: String,
    state: State<AppState>,
) -> Result<Vec<PassageSummary>, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.list_passage_drafts()
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn get_passage_draft(
    story_id: String,
    id: String,
    app: tauri::AppHandle,
) -> Result<PassageView, String> {
    view(&app, &story_id, &id)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn save_passage_draft(
    story_id: String,
    id: String,
    expected_version: u32,
    edits: PassageEdits,
    app: tauri::AppHandle,
) -> Result<PassageView, String> {
    with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.save_passage(&id, expected_version, edits)
    })?;
    view(&app, &story_id, &id)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn start_passage_job(
    story_id: String,
    id: String,
    expected_version: u32,
    expected_revision: u32,
    checking: bool,
    app: tauri::AppHandle,
) -> Result<PassageView, String> {
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
    let kind = if checking {
        TextJobKind::Effects
    } else {
        TextJobKind::Narration
    };
    if let Some(run) = active.as_ref() {
        if run.id == id && run.story_id == story_id && run.kind == kind {
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
        p.start_passage(
            &id,
            expected_version,
            expected_revision,
            checking,
            &settings,
        )
    })?;
    let (cancel, cancelled) = tokio::sync::oneshot::channel();
    *active = Some(ActiveRun {
        id: id.clone(),
        story_id: story_id.clone(),
        kind,
        cancel: Some(cancel),
    });
    drop(active);
    let job = TextJob {
        kind,
        system: prepared.system,
        user: prepared.user,
        schema: prepared.schema,
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
pub(crate) fn validate_passage(
    story_id: String,
    id: String,
    expected_version: u32,
    app: tauri::AppHandle,
) -> Result<PassageView, String> {
    let settings = connection(&app)?;
    with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.validate_passage(&id, expected_version, &settings)
    })?;
    view(&app, &story_id, &id)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn accept_passage(
    story_id: String,
    id: String,
    expected_version: u32,
    app: tauri::AppHandle,
) -> Result<PassageAcceptance, String> {
    let settings = connection(&app)?;
    with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.accept_passage(&id, expected_version, &settings)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn begin_passage_revision(
    story_id: String,
    id: String,
    passage_id: String,
    source: RecordRef,
    kind: PassageRevisionKind,
    app: tauri::AppHandle,
) -> Result<PassageView, String> {
    with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.begin_passage_revision(&id, &passage_id, source, kind)
    })?;
    view(&app, &story_id, &id)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn clone_passage_draft(
    story_id: String,
    source_id: String,
    expected_version: u32,
    id: String,
    app: tauri::AppHandle,
) -> Result<PassageView, String> {
    with_project(&app.state::<AppState>(), |p| {
        check_story(p, &story_id)?;
        p.clone_passage_draft(&source_id, expected_version, &id)
    })?;
    view(&app, &story_id, &id)
}
#[tauri::command]
#[specta::specta]
pub(crate) fn passage_versions(
    story_id: String,
    id: String,
    state: State<AppState>,
) -> Result<PassageVersions, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.passage_versions(&id)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn read_passage(
    story_id: String,
    id: String,
    state: State<AppState>,
) -> Result<PassageReading, String> {
    with_project(&state, |p| {
        check_story(p, &story_id)?;
        p.read_passage(&id)
    })
}
#[tauri::command]
#[specta::specta]
pub(crate) fn fork_after_passage(
    story_id: String,
    id: String,
    parent_path: String,
    title: String,
    state: State<AppState>,
    jobs: State<AuthoringJobs>,
) -> Result<ProjectView, String> {
    let _idle = idle_guard(&jobs)?;
    let mut guard = state.0.lock().map_err(|_| "The project needs reopening.")?;
    let source = guard.as_ref().ok_or("Open a project first.")?;
    check_story(source, &story_id)?;
    let fork = source.fork_after_passage(&id, std::path::Path::new(&parent_path), title)?;
    let view = fork.view()?;
    *guard = Some(fork);
    Ok(view)
}
