use super::*;
fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn settings() -> LlmConnection {
    LlmConnection {
        provider: TextProvider::Demo,
        ..Default::default()
    }
}
fn fixture() -> (tempfile::TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let mut world = World::default();
    for (kind, name) in [
        ("core.place", "Glasshouse"),
        ("core.character", "Mara"),
        ("core.thread", "Lost key"),
    ] {
        let preset = world.node_types.iter().find(|t| t.id == kind).unwrap();
        let components = preset
            .components
            .iter()
            .map(|id| {
                let definition = world.component_types.iter().find(|c| &c.id == id).unwrap();
                (
                    id.clone(),
                    definition
                        .fields
                        .iter()
                        .map(|f| (f.key.clone(), f.default_value.clone()))
                        .collect(),
                )
            })
            .collect();
        world.nodes.push(WorldNode {
            id: id(),
            type_id: kind.into(),
            name: name.into(),
            description: format!("{name}: Private profile detail not part of chapter text."),
            visual_description: String::new(),
            components,
        });
    }
    let project = Project::create(
        &dir.path().join("story"),
        "Fictional memory test".into(),
        world,
        id(),
    )
    .unwrap();
    (dir, project)
}
fn accept(p: &Project, text: &str, closed: bool, note: Option<&str>, thread: bool) -> String {
    let world = p.view().unwrap().world;
    let mut plan = p.next_passage_plan().unwrap();
    plan.viewpoint_id = Some(world.nodes[1].id.clone());
    plan.location_id = Some(world.nodes[0].id.clone());
    plan.direction = "Continue this fictional scene.".into();
    if thread {
        plan.reference_ids.push(world.nodes[2].id.clone());
    }
    let notes = note
        .map(|text| {
            vec![ContinuityNote {
                text: text.into(),
                evidence: text_quote(text).into(),
                node_ids: vec![],
            }]
        })
        .unwrap_or_default();
    let effects = PassageEffects {
        changes: vec![],
        notes,
        suggest_chapter_end: false,
    };
    let edits = PassageEdits {
        plan,
        prose: text.into(),
        effects_text: serde_json::to_string(&effects).unwrap(),
        close_chapter: closed,
        approved_effects: vec![],
    };
    let draft = id();
    p.save_passage(&draft, 0, edits).unwrap();
    p.validate_passage(&draft, 1, &settings()).unwrap();
    p.approve_passage_for_test(&draft, &settings()).unwrap();
    p.accept_passage(
        &draft,
        p.passage_view(&draft, &settings()).unwrap().version,
        &settings(),
    )
    .unwrap();
    draft
}
fn text_quote(text: &str) -> &str {
    text
}
fn memory(p: &Project, chapter: &str, text: &str, quote: &str, nodes: Vec<String>) -> String {
    let draft = id();
    p.create_memory_draft(&draft, chapter).unwrap();
    let mut view = p.memory_view(&draft).unwrap();
    view.edits.output = MemoryOutput {
        points: vec![MemoryPoint {
            text: text.into(),
            passage_id: view.sources[0].passage_id.clone(),
            evidence: quote.into(),
            node_ids: nodes,
        }],
        no_lasting_facts: false,
    };
    p.save_memory_draft(&draft, view.version, view.edits)
        .unwrap();
    draft
}
fn apply(p: &Project, draft: &str) -> CommitResult {
    let view = p.memory_view(draft).unwrap();
    assert!(view.can_apply, "{}", view.error);
    p.apply_memory(draft, view.version).unwrap()
}
fn context(p: &Project, budget: u32, reference: bool) -> MemoryContext {
    let world = p.view().unwrap().world;
    let mut plan = p.next_passage_plan().unwrap();
    plan.viewpoint_id = Some(world.nodes[1].id.clone());
    if reference {
        plan.reference_ids.push(world.nodes[2].id.clone());
    }
    p.memory_context(&p.story_index().unwrap(), &plan, &world, budget)
        .unwrap()
}

#[test]
fn manual_memory_is_a_saved_proposal_until_apply_and_reopens_with_immutable_sources() {
    let (dir, p) = fixture();
    let prose = "Mara discovered the key beneath the window.";
    accept(&p, prose, true, None, true);
    let chapter = p.story_index().unwrap().chapters[0].id.clone();
    let before = p.view().unwrap();
    let draft = memory(
        &p,
        &chapter,
        "Mara found the missing key.",
        prose,
        vec![before.world.nodes[2].id.clone()],
    );
    assert_eq!(p.view().unwrap().revision, before.revision);
    assert_eq!(
        p.chapter_memory_status().unwrap()[0].status,
        MemoryStatus::Missing
    );
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    assert_eq!(
        p.memory_view(&draft).unwrap().edits.output.points[0].text,
        "Mara found the missing key."
    );
    let result = apply(&p, &draft);
    assert_eq!(result.project.world, before.world);
    assert_eq!(p.story_index().unwrap().passages.len(), 1);
    assert_eq!(
        p.chapter_memory_status().unwrap()[0].status,
        MemoryStatus::Current
    );
    let replay = p.apply_memory(&draft, 0).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.committed_revision, result.committed_revision);
    assert!(p.list_memory_drafts().unwrap().is_empty());
}

#[test]
fn quotes_passages_and_node_references_are_checked_without_discarding_bad_edits() {
    let (_dir, p) = fixture();
    let prose = "Mara discovered the key beneath the window.";
    accept(&p, prose, true, None, true);
    let chapter = p.story_index().unwrap().chapters[0].id.clone();
    let draft = memory(
        &p,
        &chapter,
        "A claimed discovery.",
        "This quote never occurred.",
        vec![],
    );
    let mut view = p.memory_view(&draft).unwrap();
    assert!(!view.can_apply);
    assert!(view.error.contains("exact quote"));
    view.edits.output.points[0].evidence = prose.into();
    view.edits.output.points[0].passage_id = id();
    p.save_memory_draft(&draft, view.version, view.edits)
        .unwrap();
    view = p.memory_view(&draft).unwrap();
    assert!(!view.can_apply);
    view.edits.output.points[0].passage_id = view.sources[0].passage_id.clone();
    view.edits.output.points[0].node_ids = vec![id()];
    p.save_memory_draft(&draft, view.version, view.edits)
        .unwrap();
    view = p.memory_view(&draft).unwrap();
    assert!(!view.can_apply);
    assert_eq!(view.edits.output.points[0].node_ids.len(), 1);
    assert_eq!(
        p.chapter_memory_status().unwrap()[0].status,
        MemoryStatus::Missing
    );
}

#[test]
fn older_notes_and_reviewed_summaries_are_retrieved_without_sending_the_whole_history() {
    let (_dir, p) = fixture();
    let prose = "Mara discovered the key beneath the window.";
    accept(&p, prose, true, Some(prose), true);
    let chapter = p.story_index().unwrap().chapters[0].id.clone();
    accept(
        &p,
        "Rain darkened the garden beyond the glass.",
        false,
        None,
        false,
    );
    accept(
        &p,
        "Mara waited quietly for the others.",
        false,
        None,
        false,
    );
    let notes = context(&p, 6000, true);
    assert_eq!(notes.points.len(), 1);
    assert!(notes.points[0].summary.is_none());
    assert_eq!(notes.coverage.unrepresented_passages, 0);
    let draft = memory(
        &p,
        &chapter,
        "Mara recovered the key.",
        prose,
        vec![p.view().unwrap().world.nodes[2].id.clone()],
    );
    apply(&p, &draft);
    let recalled = context(&p, 6000, true);
    assert_eq!(recalled.points.len(), 1);
    assert!(recalled.points[0].summary.is_some());
    assert_eq!(recalled.points[0].text, "Mara recovered the key.");
    assert_eq!(recalled.coverage.current_summaries, 1);
    let current = p.view().unwrap();
    let draft = id();
    let plan = p.next_passage_plan().unwrap();
    p.save_passage(
        &draft,
        0,
        PassageEdits {
            plan: PassagePlan {
                direction: "Continue.".into(),
                reference_ids: vec![current.world.nodes[2].id.clone()],
                ..plan
            },
            prose: String::new(),
            effects_text: empty_effects(),
            close_chapter: false,
            approved_effects: vec![],
        },
    )
    .unwrap();
    let request = p
        .start_passage(&draft, 1, current.revision, false, &settings())
        .unwrap();
    assert!(request.user.contains("Mara recovered the key."));
    assert!(!request.user.contains(prose));
    assert!(!request.user.contains("Mara: Private profile detail"));
}

#[test]
fn wording_changes_stale_summaries_and_checked_prose_without_reusing_obsolete_quotes() {
    let (_dir, p) = fixture();
    let prose = "Mara discovered the key beneath the window.";
    let passage = accept(&p, prose, true, None, true);
    let chapter = p.story_index().unwrap().chapters[0].id.clone();
    let draft = memory(&p, &chapter, "Mara found the key.", prose, vec![]);
    apply(&p, &draft);
    accept(
        &p,
        "Rain darkened the garden beyond the glass.",
        false,
        None,
        false,
    );
    accept(
        &p,
        "Mara waited quietly for the others.",
        false,
        None,
        false,
    );
    let pending = id();
    let edits = PassageEdits {
        plan: p.next_passage_plan().unwrap(),
        prose: "Someone knocked at the door.".into(),
        effects_text: empty_effects(),
        close_chapter: false,
        approved_effects: vec![],
    };
    p.save_passage(&pending, 0, edits).unwrap();
    p.validate_passage(&pending, 1, &settings()).unwrap();
    assert!(p.passage_view(&pending, &settings()).unwrap().can_accept);
    let revision = id();
    p.begin_passage_revision(
        &revision,
        &passage,
        p.read_passage(&passage).unwrap().record,
        PassageRevisionKind::Wording,
    )
    .unwrap();
    let mut view = p.passage_view(&revision, &settings()).unwrap();
    view.edits.prose = "Beneath the window, Mara found the missing key.".into();
    p.save_passage(&revision, view.version, view.edits).unwrap();
    view = p.passage_view(&revision, &settings()).unwrap();
    p.validate_passage(&revision, view.version, &settings())
        .unwrap();
    p.accept_passage(
        &revision,
        p.passage_view(&revision, &settings()).unwrap().version,
        &settings(),
    )
    .unwrap();
    assert_eq!(
        p.chapter_memory_status().unwrap()[0].status,
        MemoryStatus::Stale
    );
    let recalled = context(&p, 6000, true);
    assert!(recalled.points.is_empty());
    assert_eq!(recalled.coverage.stale_summaries, 1);
    assert_eq!(recalled.coverage.unrepresented_passages, 1);
    assert!(!p.passage_view(&pending, &settings()).unwrap().can_accept);
}

#[test]
fn memory_generation_is_one_explicit_scoped_request_and_budget_failure_is_not_running() {
    let (_dir, p) = fixture();
    let prose = "Mara discovered the key beneath the window.";
    accept(&p, prose, true, None, true);
    let chapter = p.story_index().unwrap().chapters[0].id.clone();
    accept(
        &p,
        "The future revelation must never enter the older summary.",
        true,
        None,
        false,
    );
    let draft = id();
    p.create_memory_draft(&draft, &chapter).unwrap();
    let mut configured = settings();
    configured.structured_output = StructuredOutput::Schema;
    let request = p
        .start_memory(&draft, 1, p.view().unwrap().revision, &configured)
        .unwrap();
    assert!(request.user.contains(prose));
    assert!(!request.user.contains("future revelation"));
    assert!(!request.user.contains("Private profile detail"));
    assert!(!request.user.contains("responseSchema"));
    assert!(request.output_tokens <= 3072);
    p.finish_memory(&draft, Ok(request.demo), false, false)
        .unwrap();
    let view = p.memory_view(&draft).unwrap();
    assert!(view.can_apply);
    assert_eq!(
        p.chapter_memory_status().unwrap()[0].status,
        MemoryStatus::Missing
    );
    let mut small = settings();
    small.context_tokens = 2048;
    small.max_tokens = 512;
    let huge = "A very long accepted chapter. ".repeat(1500);
    accept(&p, &huge, true, None, false);
    let chapter = p.story_index().unwrap().chapters.last().unwrap().id.clone();
    let blocked = id();
    p.create_memory_draft(&blocked, &chapter).unwrap();
    assert!(p
        .start_memory(&blocked, 1, p.view().unwrap().revision, &small)
        .unwrap_err()
        .contains("budget"));
    assert_eq!(p.memory_view(&blocked).unwrap().stage, MemoryStage::Review);
}

#[test]
fn interrupted_cancelled_and_truncated_summary_work_keeps_outputs_without_resending() {
    let (dir, p) = fixture();
    let prose = "Mara discovered the key beneath the window.";
    accept(&p, prose, true, None, true);
    let chapter = p.story_index().unwrap().chapters[0].id.clone();
    let draft = id();
    p.create_memory_draft(&draft, &chapter).unwrap();
    let request = p
        .start_memory(&draft, 1, p.view().unwrap().revision, &settings())
        .unwrap();
    p.finish_memory(&draft, Ok(request.demo.clone()), false, true)
        .unwrap();
    let mut view = p.memory_view(&draft).unwrap();
    assert!(!view.can_apply);
    assert_eq!(view.original_output, request.demo);
    p.parse_memory_json(&draft, view.version).unwrap();
    view = p.memory_view(&draft).unwrap();
    assert!(view.can_apply);
    p.start_memory(
        &draft,
        view.version,
        p.view().unwrap().revision,
        &settings(),
    )
    .unwrap();
    p.finish_memory(&draft, Err("Cancelled fixture".into()), true, false)
        .unwrap();
    assert_eq!(p.memory_view(&draft).unwrap().stage, MemoryStage::Cancelled);
    let running = id();
    p.create_memory_draft(&running, &chapter).unwrap();
    p.start_memory(&running, 1, p.view().unwrap().revision, &settings())
        .unwrap();
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    p.interrupt_memories().unwrap();
    assert_eq!(
        p.memory_view(&running).unwrap().stage,
        MemoryStage::Interrupted
    );
    assert!(!p.memory_view(&running).unwrap().can_apply);
    assert_eq!(p.memory_view(&draft).unwrap().original_output, request.demo);
}

#[test]
fn source_changes_while_summarizing_block_apply_but_keep_the_candidate() {
    let (_dir, p) = fixture();
    let prose = "Mara discovered the key beneath the window.";
    let passage = accept(&p, prose, true, None, true);
    let chapter = p.story_index().unwrap().chapters[0].id.clone();
    let draft = id();
    p.create_memory_draft(&draft, &chapter).unwrap();
    let request = p
        .start_memory(&draft, 1, p.view().unwrap().revision, &settings())
        .unwrap();
    let replacement = id();
    p.begin_passage_revision(
        &replacement,
        &passage,
        p.read_passage(&passage).unwrap().record,
        PassageRevisionKind::Events,
    )
    .unwrap();
    let mut view = p.passage_view(&replacement, &settings()).unwrap();
    view.edits.close_chapter = false;
    view.edits.prose = "Mara left the key beside the window.".into();
    p.save_passage(&replacement, view.version, view.edits)
        .unwrap();
    view = p.passage_view(&replacement, &settings()).unwrap();
    p.validate_passage(&replacement, view.version, &settings())
        .unwrap();
    p.accept_passage(
        &replacement,
        p.passage_view(&replacement, &settings()).unwrap().version,
        &settings(),
    )
    .unwrap();
    p.finish_memory(&draft, Ok(request.demo.clone()), false, false)
        .unwrap();
    let view = p.memory_view(&draft).unwrap();
    assert!(view.stale);
    assert!(!view.can_apply);
    assert_eq!(view.original_output, request.demo);
    assert!(p.apply_memory(&draft, view.version).is_err());
}

#[test]
fn context_selection_prioritizes_references_and_reports_caps_omissions_and_gaps() {
    let (_dir, p) = fixture();
    for chapter in 0..5 {
        let prose = format!("Mara recorded chapter {chapter} beside the window.");
        accept(&p, &prose, true, None, true);
        let id = p.story_index().unwrap().chapters.last().unwrap().id.clone();
        let draft = memory(
            &p,
            &id,
            &format!("Historical point {chapter}."),
            &prose,
            if chapter == 0 {
                vec![p.view().unwrap().world.nodes[2].id.clone()]
            } else {
                vec![]
            },
        );
        let mut view = p.memory_view(&draft).unwrap();
        let template = view.edits.output.points[0].clone();
        view.edits.output.points = (0..8)
            .map(|n| MemoryPoint {
                text: format!("{} Detail {n}.", template.text),
                ..template.clone()
            })
            .collect();
        p.save_memory_draft(&draft, view.version, view.edits)
            .unwrap();
        apply(&p, &draft);
    }
    accept(
        &p,
        "Mara watched the final rain before nightfall.",
        false,
        None,
        false,
    );
    accept(
        &p,
        "A final quiet scene ended the evening.",
        false,
        None,
        false,
    );
    let result = context(&p, 6000, true);
    assert_eq!(result.points.len(), 12);
    assert!(result.points[0].text.contains("Historical point 0"));
    assert!(result.coverage.omitted_points > 0);
    assert_eq!(result.coverage.unrepresented_passages, 0);
    assert!(serde_json::to_vec(&result.points).unwrap().len() <= 6000);
    let empty = context(&p, 0, true);
    assert!(empty.points.is_empty());
    assert_eq!(
        empty.coverage.omitted_points,
        empty.coverage.available_points
    );
    accept(
        &p,
        "A scene without a recorded memory or note.",
        false,
        None,
        false,
    );
    accept(
        &p,
        "Another unrecorded but accepted moment passed.",
        false,
        None,
        false,
    );
    accept(
        &p,
        "The newest passage remains in recent prose.",
        false,
        None,
        false,
    );
    assert!(context(&p, 6000, true).coverage.unrepresented_passages > 0);
}

#[test]
fn forks_copy_only_current_fully_retained_chapter_memory_and_work_without_the_source() {
    let (dir, p) = fixture();
    let first_text = "Mara recovered the key beside the window.";
    let first = accept(&p, first_text, true, None, true);
    let chapter = p.story_index().unwrap().chapters[0].id.clone();
    let draft = memory(&p, &chapter, "The key was recovered.", first_text, vec![]);
    apply(&p, &draft);
    let middle = accept(
        &p,
        "Mara carried the secret into another chapter.",
        false,
        None,
        false,
    );
    let future = "A future-only discovery must be excluded.";
    accept(&p, future, true, None, false);
    let chapter = p.story_index().unwrap().chapters.last().unwrap().id.clone();
    let draft = memory(
        &p,
        &chapter,
        "Future-only summary.",
        "Mara carried the secret into another chapter.",
        vec![],
    );
    apply(&p, &draft);
    let fork = p
        .fork_after_passage(&middle, dir.path(), "Independent memory copy".into())
        .unwrap();
    assert_eq!(fork.chapter_memory_status().unwrap().len(), 1);
    assert_eq!(
        fork.chapter_memory_status().unwrap()[0].status,
        MemoryStatus::Current
    );
    assert_eq!(fork.story_index().unwrap().memories.len(), 1);
    let copy_path = fork.folder_path();
    let files = fs::read_dir(std::path::Path::new(&copy_path).join("memories"))
        .unwrap()
        .map(|e| fs::read_to_string(e.unwrap().path()).unwrap())
        .collect::<String>();
    assert!(!files.contains("Future-only summary"));
    assert!(files.contains("The key was recovered"));
    drop(fork);
    let boundary = p
        .fork_after_passage(&first, dir.path(), "First boundary".into())
        .unwrap();
    assert_eq!(boundary.story_index().unwrap().memories.len(), 1);
    drop(boundary);
    drop(p);
    // Windows scanners can briefly hold newly written fixture files when the suite
    // runs in parallel. Still fail if the source cannot actually be moved away.
    let source = dir.path().join("story");
    let destination = dir.path().join("unavailable-source");
    let mut attempts = 0;
    loop {
        match fs::rename(&source, &destination) {
            Ok(()) => break,
            Err(error)
                if cfg!(windows)
                    && matches!(error.raw_os_error(), Some(5 | 32 | 33))
                    && attempts < 10 =>
            {
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(error) => panic!("Could not make the fixture source unavailable: {error}"),
        }
    }
    let fork = Project::open(std::path::Path::new(&copy_path)).unwrap();
    assert_eq!(
        fork.chapter_memory_status().unwrap()[0].status,
        MemoryStatus::Current
    );
}

#[test]
fn damaged_memory_is_reported_and_excluded_without_resetting_the_world() {
    let (dir, p) = fixture();
    let prose = "Mara discovered the key beneath the window.";
    accept(&p, prose, true, None, true);
    let chapter = p.story_index().unwrap().chapters[0].id.clone();
    let draft = memory(&p, &chapter, "A memory point.", prose, vec![]);
    apply(&p, &draft);
    let reference = p.story_index().unwrap().memories[&chapter].clone();
    fs::write(
        dir.path()
            .join("story/memories")
            .join(format!("{}.json", reference.id)),
        b"damaged",
    )
    .unwrap();
    let world = p.view().unwrap().world;
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    assert_eq!(p.view().unwrap().world, world);
    assert_eq!(
        p.chapter_memory_status().unwrap()[0].status,
        MemoryStatus::Unavailable
    );
    accept(
        &p,
        "Mara waited beside the window in silence.",
        false,
        None,
        false,
    );
    accept(
        &p,
        "The rain eventually eased across the garden.",
        false,
        None,
        false,
    );
    let recalled = context(&p, 6000, true);
    assert!(recalled.points.is_empty());
    assert_eq!(recalled.coverage.unavailable_summaries, 1);
}

#[test]
fn apply_receipts_preserve_newer_summaries_after_an_uncertain_status_write() {
    let (_dir, p) = fixture();
    let prose = "Mara discovered the key beneath the window.";
    accept(&p, prose, true, None, true);
    let chapter = p.story_index().unwrap().chapters[0].id.clone();
    let first = memory(&p, &chapter, "First reviewed summary.", prose, vec![]);
    let original = apply(&p, &first);
    let newer = memory(&p, &chapter, "Newer reviewed summary.", prose, vec![]);
    apply(&p, &newer);
    let pointer = p.story_index().unwrap().memories[&chapter].clone();
    let mut draft = p.read_memory_draft(&first).unwrap();
    draft.stage = MemoryStage::Review;
    p.write_memory_draft(&draft).unwrap();
    let replay = p.apply_memory(&first, 0).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.committed_revision, original.committed_revision);
    assert_eq!(p.story_index().unwrap().memories[&chapter], pointer);
}

#[test]
fn explicit_empty_memory_does_not_invent_facts_and_invalid_draft_siblings_do_not_block_recovery() {
    let (dir, p) = fixture();
    accept(&p, "Mara waited quietly by the window.", true, None, false);
    let chapter = p.story_index().unwrap().chapters[0].id.clone();
    let empty = id();
    p.create_memory_draft(&empty, &chapter).unwrap();
    let mut view = p.memory_view(&empty).unwrap();
    assert!(!view.can_apply);
    view.edits.output.no_lasting_facts = true;
    p.save_memory_draft(&empty, view.version, view.edits)
        .unwrap();
    apply(&p, &empty);
    let running = id();
    p.create_memory_draft(&running, &chapter).unwrap();
    p.start_memory(&running, 1, p.view().unwrap().revision, &settings())
        .unwrap();
    fs::write(
        dir.path()
            .join("story/memory-drafts")
            .join(format!("{}.json", id())),
        b"broken fixture",
    )
    .unwrap();
    assert!(p.interrupt_memories().is_err());
    assert_eq!(
        p.memory_view(&running).unwrap().stage,
        MemoryStage::Interrupted
    );
    assert_eq!(p.chapter_memory_status().unwrap()[0].points, 0);
}

#[test]
fn extraction_reserves_required_context_before_memory_and_reproduces_its_checked_source() {
    let (_dir, p) = fixture();
    let prose = "Mara discovered the key beneath the window.";
    accept(&p, prose, true, None, true);
    let chapter = p.story_index().unwrap().chapters[0].id.clone();
    let draft = memory(
        &p,
        &chapter,
        "Mara recovered the key beside the window.",
        prose,
        vec![],
    );
    apply(&p, &draft);
    accept(
        &p,
        "Mara paused before the following scene.",
        false,
        None,
        false,
    );
    accept(
        &p,
        "The newest rain fell against the window.",
        false,
        None,
        false,
    );
    let candidate = id();
    p.save_passage(
        &candidate,
        0,
        PassageEdits {
            plan: PassagePlan {
                direction: "Wait by the window.".into(),
                ..p.next_passage_plan().unwrap()
            },
            prose: "Mara waited beside the glasshouse window. ".repeat(500),
            effects_text: empty_effects(),
            close_chapter: false,
            approved_effects: vec![],
        },
    )
    .unwrap();
    let mut configured = settings();
    configured.max_tokens = 1024;
    let mut found = false;
    for limit in (8000..22000).step_by(500) {
        configured.context_tokens = limit;
        match p.start_passage(&candidate, 1, p.view().unwrap().revision, true, &configured) {
            Err(error) => assert!(error.contains("budget")),
            Ok(_) => {
                let running = p.passage_view(&candidate, &configured).unwrap();
                assert!(running.memory.coverage.byte_budget < 6000);
                p.finish_passage(
                    &candidate,
                    true,
                    Ok(empty_effects()),
                    false,
                    false,
                    &configured,
                )
                .unwrap();
                assert!(p.passage_view(&candidate, &configured).unwrap().can_accept);
                found = true;
                break;
            }
        }
    }
    assert!(
        found,
        "An in-budget extraction should fit after optional memory is narrowed."
    );
}

#[test]
fn blank_quotes_and_oversized_structured_responses_cannot_be_applied() {
    let (_dir, p) = fixture();
    let prose = "Mara discovered the key beneath the window.";
    accept(&p, prose, true, None, true);
    let chapter = p.story_index().unwrap().chapters[0].id.clone();
    let draft = memory(&p, &chapter, "Mara found the key.", "         ", vec![]);
    assert!(!p.memory_view(&draft).unwrap().can_apply);
    let view = p.memory_view(&draft).unwrap();
    p.start_memory(
        &draft,
        view.version,
        p.view().unwrap().revision,
        &settings(),
    )
    .unwrap();
    let point = MemoryPoint {
        text: "Mara found the key.".into(),
        passage_id: view.sources[0].passage_id.clone(),
        evidence: prose.into(),
        node_ids: vec![],
    };
    let raw = serde_json::to_string(&MemoryOutput {
        points: vec![point; 13],
        no_lasting_facts: false,
    })
    .unwrap();
    p.finish_memory(&draft, Ok(raw.clone()), false, false)
        .unwrap();
    let failed = p.memory_view(&draft).unwrap();
    assert_eq!(failed.stage, MemoryStage::Failed);
    assert!(!failed.can_apply);
    assert!(failed.edits.output.points.len() <= 12);
    assert_eq!(failed.original_output, raw);
}
