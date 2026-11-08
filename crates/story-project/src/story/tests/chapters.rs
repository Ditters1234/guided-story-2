use super::*;

#[test]
fn explicit_new_chapter_is_draft_only_then_publishes_the_transition_once() {
    let (dir, p) = fixture();
    let mut first = edits(&p, "Mara shut the window.");
    first.plan.chapter_plan = "Leave the glasshouse.".into();
    first.plan.chapter_progress = "The window is shut.".into();
    let first_id = accept(&p, first);
    let original = p.read_passage(&first_id).unwrap();
    let key = id();
    let mut next = edits(&p, "Mara waited for Jon.");
    next.plan = p.new_chapter_plan().unwrap();
    assert_eq!(next.plan.chapter_title, "Chapter 2");
    assert!(next.plan.chapter_plan.is_empty() && next.plan.chapter_progress.is_empty());
    next.plan.chapter_plan = "Decide where to shelter.".into();
    next.plan.chapter_progress = "They have agreed to wait.".into();
    next.plan.stop_after = "Jon answers.".into();
    p.save_passage(&key, 0, next).unwrap();
    assert_eq!(p.story_index().unwrap().chapters.len(), 1);
    assert!(!p.story_index().unwrap().chapters[0].closed);
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    assert!(
        p.passage_view(&key, &settings())
            .unwrap()
            .edits
            .plan
            .start_new_chapter
    );
    p.validate_passage(&key, 1, &settings()).unwrap();
    let view = p.passage_view(&key, &settings()).unwrap();
    let accepted = p.accept_passage(&key, view.version, &settings()).unwrap();
    let index = p.story_index().unwrap();
    assert_eq!(index.chapters.len(), 2);
    assert!(index.chapters[0].closed && !index.chapters[1].closed);
    assert_eq!(index.chapters[1].passage_ids, vec![key.clone()]);
    assert_eq!(p.read_passage(&first_id).unwrap().record, original.record);
    let next = p.next_passage_plan().unwrap();
    assert_eq!(next.chapter_plan, "Decide where to shelter.");
    assert_eq!(next.chapter_progress, "They have agreed to wait.");
    assert!(next.stop_after.is_empty() && !next.start_new_chapter);
    // Reconcile an uncertain final status after the transaction has published.
    let mut draft = p.read_passage_draft(&key).unwrap();
    draft.stage = PassageStage::Ready;
    p.write_passage_draft(&draft).unwrap();
    let replay = p.accept_passage(&key, 0, &settings()).unwrap();
    assert!(replay.commit.replayed);
    assert_eq!(
        replay.commit.project.revision,
        accepted.commit.project.revision
    );
    assert_eq!(p.story_index().unwrap().chapters.len(), 2);
}

#[test]
fn writing_brief_uses_current_remaining_work_and_labels_earlier_chapters_as_history() {
    let (_dir, p) = fixture();
    let mut old = edits(&p, "Mara shut the window.");
    old.plan.chapter_plan = "OLD_PLAN_ONLY_FOR_CHAPTER_ONE".into();
    accept(&p, old);
    let key = id();
    let mut draft = edits(&p, "");
    draft.plan = p.new_chapter_plan().unwrap();
    draft.plan.viewpoint_id = Some(p.view().unwrap().world.nodes[1].id.clone());
    draft.plan.chapter_plan = "NEW_REMAINING_ARC".into();
    draft.plan.chapter_progress = "AUTHOR_RECAP".into();
    draft.plan.direction = "CURRENT_PASSAGE_BEAT".into();
    draft.plan.stop_after = "EXPLICIT_STOPPING_POINT".into();
    p.save_passage(&key, 0, draft).unwrap();
    let request = p
        .start_passage(&key, 1, p.view().unwrap().revision, false, &settings())
        .unwrap();
    assert!(!request.user.contains("OLD_PLAN_ONLY_FOR_CHAPTER_ONE"));
    for text in [
        "NEW_REMAINING_ARC",
        "AUTHOR_RECAP",
        "CURRENT_PASSAGE_BEAT",
        "EXPLICIT_STOPPING_POINT",
    ] {
        assert_eq!(request.user.matches(text).count(), 1);
    }
    let input: Value = serde_json::from_str(&request.user).unwrap();
    assert_eq!(input["writingBrief"]["chapterStatus"], "opening");
    assert_eq!(input["writingBrief"]["acceptedPassages"], 0);
    assert_eq!(
        input["recentPassages"][0]["chapterRelation"],
        "earlierChapter"
    );
    assert!(input.get("currentChapter").is_none());
    assert!(input["plan"].get("chapterPlan").is_none());
    let preview = p
        .passage_view(&key, &settings())
        .unwrap()
        .scene_context
        .writing_brief
        .unwrap();
    assert_eq!(json!(preview), input["writingBrief"]);
}

#[test]
fn custom_prompts_keep_their_paths_but_never_receive_a_stale_chapter_plan() {
    let (_dir, p) = fixture();
    let mut first = edits(&p, "Mara shut the window.");
    first.plan.chapter_plan = "OBSOLETE_SAVED_DIRECTION".into();
    accept(&p, first);
    let mut draft = edits(&p, "");
    draft.plan.chapter_plan = "CURRENT_DRAFT_DIRECTION".into();
    let key = id();
    p.save_passage(&key, 0, draft).unwrap();
    let mut connection = settings();
    connection.prompts.overrides.insert(
        TextPromptTask::Passage,
        "Use plan.direction and currentChapter.plan to continue the scene.".into(),
    );
    let request = p
        .start_passage(&key, 1, p.view().unwrap().revision, false, &connection)
        .unwrap();
    assert_eq!(
        request.system,
        connection.system_prompt(TextPromptTask::Passage)
    );
    assert!(!request.user.contains("OBSOLETE_SAVED_DIRECTION"));
    let input: Value = serde_json::from_str(&request.user).unwrap();
    assert_eq!(input["plan"]["chapterPlan"], "CURRENT_DRAFT_DIRECTION");
    assert_eq!(input["currentChapter"]["plan"], "CURRENT_DRAFT_DIRECTION");
    assert_eq!(input["writingBrief"]["chapterStatus"], "continuing");
    assert_eq!(input["recentPassages"][0]["chapterRelation"], "sameChapter");
}

#[test]
fn progress_and_remaining_direction_follow_the_fork_boundary_and_edits_stale_checks() {
    let (dir, p) = fixture();
    let mut first = edits(&p, "Mara shut the window.");
    first.plan.chapter_plan = "First remaining work.".into();
    first.plan.chapter_progress = "First recap.".into();
    let first_id = accept(&p, first);
    let mut second = edits(&p, "Jon waited for Mara.");
    second.plan = p.new_chapter_plan().unwrap();
    second.plan.chapter_plan = "Second remaining work.".into();
    second.plan.chapter_progress = "Second recap.".into();
    let second_id = accept(&p, second);
    let mut later = edits(&p, "Mara nodded.");
    later.plan.chapter_plan = "Future direction.".into();
    later.plan.chapter_progress = "Future recap.".into();
    accept(&p, later);
    let earlier = p
        .fork_after_passage(&first_id, dir.path(), "Earlier fork".into())
        .unwrap();
    let earlier_index = earlier.story_index().unwrap();
    assert_eq!(earlier_index.chapters.len(), 1);
    assert!(!earlier_index.chapters[0].closed);
    assert_eq!(
        earlier.next_passage_plan().unwrap().chapter_progress,
        "First recap."
    );
    let fork = p
        .fork_after_passage(&second_id, dir.path(), "Second fork".into())
        .unwrap();
    let index = fork.story_index().unwrap();
    assert!(index.chapters[0].closed);
    assert_eq!(index.chapters[1].progress, "Second recap.");
    assert_eq!(index.chapters[1].plan, "Second remaining work.");
    let key = ready(&p, edits(&p, "Mara waited."));
    let mut view = p.passage_view(&key, &settings()).unwrap();
    view.edits.plan.stop_after = "End at the window.".into();
    p.save_passage(&key, view.version, view.edits).unwrap();
    assert!(!p.passage_view(&key, &settings()).unwrap().can_accept);
}

#[test]
#[ignore = "explicit isolated chapter UI fixture"]
fn chapter_desktop_fixture() {
    let Ok(root) = std::env::var("STORY_CHAPTER_DESKTOP_FIXTURE") else {
        return;
    };
    let (_dir, p) = fixture();
    let desktop = Project::create(
        Path::new(&root),
        "Chapter controls verification".into(),
        p.view().unwrap().world,
        id(),
    )
    .unwrap();
    let mut first = edits(
        &desktop,
        "Mara shut the window. Jon waited beside the workbench.",
    );
    first.plan.chapter_plan = "Decide whether to leave the glasshouse.".into();
    first.plan.chapter_progress = "The window has been shut.".into();
    accept(&desktop, first);
}
