use super::*;

fn source(p: &Project) -> String {
    let key = ready(
        p,
        edits(p, "Mara waited beside the window. Jon folded his arms."),
    );
    let mut view = p.passage_view(&key, &settings()).unwrap();
    view.edits.prose = "Mara waited beside the window. Jon looked away.".into();
    view.edits.revision_direction =
        "Make the dialogue more hesitant; keep the quiet ending.".into();
    p.save_passage(&key, view.version, view.edits).unwrap();
    key
}

#[test]
fn revision_captures_saved_edits_and_direction_and_requires_fresh_review() {
    let (_dir, p) = fixture();
    let before = p.view().unwrap();
    let source = source(&p);
    let original = p.passage_view(&source, &settings()).unwrap();
    let key = id();
    p.revise_passage_draft(&source, original.version, &key)
        .unwrap();
    p.revise_passage_draft(&source, original.version, &key)
        .unwrap();
    assert!(p
        .clone_passage_draft(&source, original.version, &key)
        .is_err());
    let candidate = p.passage_view(&key, &settings()).unwrap();
    let summaries = p.list_passage_drafts().unwrap();
    let summary = summaries.iter().find(|d| d.id == key).unwrap();
    assert_eq!(summary.source_draft_id.as_deref(), Some(source.as_str()));
    assert_eq!(summary.revision_direction, original.edits.revision_direction);
    assert!(summary.target_passage_id.is_none());
    assert_eq!(candidate.edits.prose, original.edits.prose);
    assert!(candidate.edits.approved_effects.is_empty());
    assert_eq!(candidate.edits.effects_text, empty_effects());
    assert!(!candidate.can_accept);
    let mut config = settings();
    config
        .prompts
        .overrides
        .insert(TextPromptTask::Passage, "Saved author passage base.".into());
    let request = p
        .start_passage(&key, candidate.version, before.revision, false, &config)
        .unwrap();
    let input: Value = serde_json::from_str(&request.user).unwrap();
    assert_eq!(request.system, "Saved author passage base.");
    assert_eq!(input["draftRevision"]["prose"], original.edits.prose);
    assert_eq!(
        input["draftRevision"]["direction"],
        original.edits.revision_direction
    );
    assert!(input.get("records").is_some());
    assert!(request.schema.is_none());
    p.finish_passage(
        &key,
        false,
        Ok("Mara waited. Jon murmured, ‘Perhaps.’".into()),
        false,
        false,
        &config,
    )
    .unwrap();
    let revised = p.passage_view(&key, &config).unwrap();
    assert_eq!(revised.stage, PassageStage::NeedsCheck);
    assert!(!revised.can_accept);
    assert!(!revised.effects_review.checked);
    assert!(revised.edits.approved_effects.is_empty());
    assert!(p.accept_passage(&key, revised.version, &config).is_err());
    assert_eq!(
        p.passage_view(&source, &settings()).unwrap().edits,
        original.edits
    );
    assert_eq!(p.view().unwrap().world_ref, before.world_ref);
    assert_eq!(p.view().unwrap().revision, before.revision);
    assert!(p.story_index().unwrap().passages.is_empty());
}

#[test]
fn revision_survives_interruption_and_failure_without_losing_source_prose() {
    let (dir, p) = fixture();
    let source = source(&p);
    let original = p.passage_view(&source, &settings()).unwrap();
    let key = id();
    p.revise_passage_draft(&source, original.version, &key)
        .unwrap();
    p.start_passage(&key, 1, p.view().unwrap().revision, false, &settings())
        .unwrap();
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    p.interrupt_passages().unwrap();
    let interrupted = p.passage_view(&key, &settings()).unwrap();
    assert_eq!(interrupted.stage, PassageStage::Interrupted);
    assert_eq!(interrupted.edits.prose, original.edits.prose);
    assert_eq!(
        interrupted.draft_revision.as_ref().unwrap().direction,
        original.edits.revision_direction
    );
    p.start_passage(
        &key,
        interrupted.version,
        p.view().unwrap().revision,
        false,
        &settings(),
    )
    .unwrap();
    p.finish_passage(
        &key,
        false,
        Err("Deterministic provider failure".into()),
        false,
        false,
        &settings(),
    )
    .unwrap();
    let failed = p.passage_view(&key, &settings()).unwrap();
    assert_eq!(failed.stage, PassageStage::Failed);
    assert_eq!(failed.edits.prose, original.edits.prose);
    assert_eq!(
        p.passage_view(&source, &settings()).unwrap().edits,
        original.edits
    );
    p.delete_passage_draft(&key, failed.version).unwrap();
    assert!(p
        .read_passage_draft_record(&key)
        .unwrap()
        .draft_revision
        .is_none());
    assert!(p
        .revise_passage_draft(&source, original.version, &key)
        .is_err());
    assert!(p
        .finish_passage(
            &key,
            false,
            Ok("Delayed result".into()),
            false,
            false,
            &settings()
        )
        .is_err());
}

#[test]
fn repeated_revision_uses_current_text_while_reroll_omits_revision_inputs() {
    let (_dir, p) = fixture();
    let source = source(&p);
    let original = p.passage_view(&source, &settings()).unwrap();
    let key = id();
    p.revise_passage_draft(&source, original.version, &key)
        .unwrap();
    let mut current = p.passage_view(&key, &settings()).unwrap();
    current.edits.prose = "Mara said nothing. Jon waited.".into();
    current.edits.revision_direction = "Add one line of dialogue.".into();
    p.save_passage(&key, current.version, current.edits.clone())
        .unwrap();
    let next = id();
    p.revise_passage_draft(&key, current.version + 1, &next)
        .unwrap();
    let next_view = p.passage_view(&next, &settings()).unwrap();
    assert_eq!(next_view.draft_revision.unwrap().prose, current.edits.prose);
    let reroll = id();
    p.clone_passage_draft(&next, 1, &reroll).unwrap();
    let rerolled = p.passage_view(&reroll, &settings()).unwrap();
    assert!(rerolled.draft_revision.is_none());
    assert!(rerolled.edits.prose.is_empty());
    assert!(rerolled.edits.revision_direction.is_empty());
    let request = p
        .start_passage(&reroll, 1, p.view().unwrap().revision, false, &settings())
        .unwrap();
    assert!(!request.user.contains("draftRevision"));
    assert_eq!(p.list_passage_drafts().unwrap().len(), 4);
}

#[test]
fn revision_rejects_stale_running_empty_and_oversized_inputs() {
    let (_dir, p) = fixture();
    let key = id();
    p.save_passage(&key, 0, edits(&p, "Mara waited.")).unwrap();
    assert!(p
        .revise_passage_draft(&key, 1, &id())
        .unwrap_err()
        .contains("direction"));
    let mut draft = p.passage_view(&key, &settings()).unwrap();
    draft.edits.revision_direction = "a".repeat(8001);
    assert!(p
        .save_passage(&key, draft.version, draft.edits.clone())
        .is_err());
    draft.edits.revision_direction = "Make this shorter.".into();
    p.save_passage(&key, draft.version, draft.edits).unwrap();
    assert!(p.revise_passage_draft(&key, 1, &id()).is_err());
    p.start_passage(&key, 2, p.view().unwrap().revision, false, &settings())
        .unwrap();
    assert!(p.revise_passage_draft(&key, 3, &id()).is_err());
    assert_eq!(p.list_passage_drafts().unwrap().len(), 1);
}

#[test]
fn revision_text_and_direction_are_counted_in_required_context_budget() {
    let (_dir, p) = fixture();
    let key = id();
    let mut edit = edits(&p, &"Quiet rain. ".repeat(8000));
    edit.revision_direction = "Shorten this.".into();
    p.save_passage(&key, 0, edit).unwrap();
    let candidate = id();
    p.revise_passage_draft(&key, 1, &candidate).unwrap();
    let config = LlmConnection {
        context_tokens: 4096,
        ..settings()
    };
    assert!(p
        .start_passage(&candidate, 1, p.view().unwrap().revision, false, &config)
        .err()
        .unwrap()
        .contains("budget"));
    assert_eq!(
        p.passage_view(&candidate, &settings()).unwrap().stage,
        PassageStage::NeedsCheck
    );
}

#[test]
fn draft_revision_backup_restores_standalone_source_and_review_data() {
    let (dir, p) = fixture();
    let source = source(&p);
    let original = p.passage_view(&source, &settings()).unwrap();
    let key = id();
    p.revise_passage_draft(&source, original.version, &key).unwrap();
    let backup = p.export_backup(dir.path(), &id(), p.view().unwrap().revision).unwrap();
    let reviewed = crate::inspect_backup(std::path::Path::new(&backup.folder_path)).unwrap();
    let restored = crate::restore_backup(std::path::Path::new(&backup.folder_path), dir.path(), &id(), &reviewed.fingerprint).unwrap();
    drop(p);
    fs::rename(dir.path().join("story"), dir.path().join("source-unavailable")).unwrap();
    let p = Project::open(std::path::Path::new(&restored.folder_path)).unwrap();
    let candidate = p.passage_view(&key, &settings()).unwrap();
    assert_eq!(candidate.draft_revision.unwrap().prose, original.edits.prose);
    assert_eq!(p.passage_view(&source, &settings()).unwrap().edits, original.edits);
    assert!(!candidate.can_accept);
}

#[test]
#[ignore = "explicit isolated offline draft revision desktop fixture"]
fn draft_revision_desktop_fixture() {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join(format!("draft-revision-ui-{}", id()));
    let (_dir, seed) = fixture();
    fs::create_dir_all(&base).unwrap();
    let p = Project::create(
        &base.join("story"),
        "Fictional draft revision check".into(),
        seed.view().unwrap().world,
        id(),
    )
    .unwrap();
    let key = id();
    p.save_passage(
        &key,
        0,
        edits(&p, "Mara waited beside the window.\n\nJon looked away."),
    )
    .unwrap();
    crate::save_connection(
        &base.join("config"),
        0,
        settings(),
        crate::ApiKeyUpdate::Keep,
    )
    .unwrap();
    println!("Offline UI fixture: {}", base.display());
}
