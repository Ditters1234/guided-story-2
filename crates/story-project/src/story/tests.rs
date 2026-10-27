use super::*;
use std::process::Command;
mod ai_context;
mod evidence;
mod link_review;
mod prompts;
mod review;
mod revisions;
fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn settings() -> LlmConnection {
    LlmConnection {
        provider: TextProvider::Demo,
        ..Default::default()
    }
}
fn node(world: &World, kind: &str, name: &str) -> WorldNode {
    let t = world.node_types.iter().find(|t| t.id == kind).unwrap();
    WorldNode {
        id: id(),
        type_id: kind.into(),
        name: name.into(),
        description: String::new(),
        visual_description: String::new(),
        components: t
            .components
            .iter()
            .map(|id| {
                let d = world.component_types.iter().find(|d| &d.id == id).unwrap();
                (
                    id.clone(),
                    d.fields
                        .iter()
                        .map(|f| (f.key.clone(), f.default_value.clone()))
                        .collect(),
                )
            })
            .collect(),
    }
}
fn fixture() -> (tempfile::TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let mut world = World::default();
    world.nodes.push(node(&world, "core.place", "Glasshouse"));
    world.nodes.push(node(&world, "core.character", "Mara"));
    world.nodes.push(node(&world, "core.character", "Jon"));
    world.nodes.push(node(&world, "core.item", "Field coat"));
    world.nodes[3].components.insert(
        "core.wearable".into(),
        BTreeMap::from([
            ("regions".into(), PropertyValue::Text("torso".into())),
            ("layer".into(), PropertyValue::Number(2.0)),
        ]),
    );
    for (kind, source, target) in [
        ("core.present-at", 1, 0),
        ("core.present-at", 2, 0),
        ("core.worn-by", 3, 1),
    ] {
        world.edges.push(WorldEdge {
            id: id(),
            type_id: kind.into(),
            source_id: world.nodes[source].id.clone(),
            target_id: world.nodes[target].id.clone(),
            description: String::new(),
        });
    }
    let p = Project::create(
        &dir.path().join("story"),
        "Fictional story".into(),
        world,
        id(),
    )
    .unwrap();
    (dir, p)
}
fn edits(p: &Project, prose: &str) -> PassageEdits {
    let w = p.view().unwrap().world;
    PassageEdits {
        plan: PassagePlan {
            location_id: Some(w.nodes[0].id.clone()),
            viewpoint_id: Some(w.nodes[1].id.clone()),
            participant_ids: vec![w.nodes[2].id.clone()],
            direction: "Wait beside the window.".into(),
            ..p.next_passage_plan().unwrap()
        },
        prose: prose.into(),
        effects_text: empty_effects(),
        close_chapter: false,
        approved_effects: vec![],
    }
}
fn ready(p: &Project, edits: PassageEdits) -> String {
    let rid = id();
    p.save_passage(&rid, 0, edits).unwrap();
    p.validate_passage(&rid, 1, &settings()).unwrap();
    p.approve_passage_for_test(&rid, &settings()).unwrap();
    rid
}
fn accept(p: &Project, edits: PassageEdits) -> String {
    let rid = ready(p, edits);
    let v = p.passage_view(&rid, &settings()).unwrap();
    p.accept_passage(&rid, v.version, &settings()).unwrap();
    rid
}
#[test]
fn drafts_survive_reopen_without_changing_canonical_world_or_history() {
    let (dir, p) = fixture();
    let rid = id();
    let edit = edits(&p, "Mara waited beside the window.");
    p.save_passage(&rid, 0, edit.clone()).unwrap();
    p.save_passage(&rid, 0, edit.clone()).unwrap();
    assert_eq!(p.view().unwrap().revision, 0);
    assert!(p.story_index().unwrap().passages.is_empty());
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    assert_eq!(p.passage_view(&rid, &settings()).unwrap().edits, edit);
}
#[test]
fn blank_direction_can_generate_first_and_later_passages_without_changing_the_world() {
    let (_dir, p) = fixture();
    for turn in 0..2 {
        let before = p.view().unwrap();
        let rid = id();
        let mut edit = edits(&p, "");
        edit.plan.direction = "  ".into();
        p.save_passage(&rid, 0, edit).unwrap();
        let prepared = p
            .start_passage(&rid, 1, before.revision, false, &settings())
            .unwrap();
        assert!(prepared.system.contains("Passage direction is optional"));
        assert!(prepared.user.contains("Mara") && prepared.user.contains("Field coat"));
        if turn == 1 {
            assert!(prepared.user.contains("The room fell quiet."));
        }
        p.finish_passage(&rid, false, Ok(prepared.demo), false, false, &settings())
            .unwrap();
        let view = p.passage_view(&rid, &settings()).unwrap();
        assert!(!view.edits.prose.trim().is_empty());
        assert_eq!(p.view().unwrap().world_ref, before.world_ref);
        assert_eq!(p.view().unwrap().revision, before.revision);
        assert!(!view.can_accept);
        p.validate_passage(&rid, view.version, &settings()).unwrap();
        let view = p.passage_view(&rid, &settings()).unwrap();
        p.accept_passage(&rid, view.version, &settings()).unwrap();
    }
    assert_eq!(p.story_index().unwrap().passages.len(), 2);
}

#[test]
fn first_and_later_passages_share_acceptance_and_chapter_planning() {
    let (_dir, p) = fixture();
    let first = accept(&p, edits(&p, "Mara waited beside the window."));
    let mut next = edits(&p, "Jon watched the rain settle.");
    next.plan.chapter_plan = "Finish waiting, then leave tomorrow.".into();
    next.close_chapter = true;
    let second = accept(&p, next);
    let index = p.story_index().unwrap();
    assert_eq!(index.passages.len(), 2);
    assert_eq!(index.chapters.len(), 1);
    assert!(index.chapters[0].closed);
    assert_eq!(p.next_passage_plan().unwrap().chapter_title, "Chapter 2");
    assert!(p.next_passage_plan().unwrap().chapter_plan.is_empty());
    let mut next = edits(&p, "");
    next.plan.direction = "Open the next chapter.".into();
    let rid = id();
    p.save_passage(&rid, 0, next).unwrap();
    let prep = p
        .start_passage(&rid, 1, p.view().unwrap().revision, false, &settings())
        .unwrap();
    assert!(prep.user.contains(&"Mara waited beside the window."));
    assert!(prep.user.contains("Jon watched the rain settle."));
    assert!(!prep.system.contains("first chapter"));
    assert_eq!(p.read_passage(&first).unwrap().passage.id, first);
    assert_eq!(p.read_passage(&second).unwrap().passage.id, second);
}
#[test]
fn evidence_checked_transfer_commits_prose_world_and_history_once() {
    let (_dir, p) = fixture();
    let w = p.view().unwrap().world;
    let stored = w
        .edges
        .iter()
        .find(|e| e.type_id == "core.worn-by")
        .unwrap();
    let mut link = stored.clone();
    link.type_id = "core.stored-in".into();
    link.target_id = w.nodes[2].id.clone();
    let mut edit = edits(&p, "Mara took off the field coat and handed it to Jon.");
    edit.effects_text = serde_json::to_string(&PassageEffects {
        changes: vec![PassageEffect::PutLink {
            link,
            evidence: edit.prose.clone(),
        }],
        ..Default::default()
    })
    .unwrap();
    let rid = ready(&p, edit);
    let v = p.passage_view(&rid, &settings()).unwrap();
    assert!(v.can_accept);
    let accepted = p.accept_passage(&rid, v.version, &settings()).unwrap();
    assert_eq!(accepted.commit.project.revision, 1);
    let replay = p.accept_passage(&rid, v.version, &settings()).unwrap();
    assert!(replay.commit.replayed);
    assert_eq!(p.story_index().unwrap().passages.len(), 1);
    let reading = p.read_passage(&rid).unwrap();
    assert!(reading
        .before
        .edges
        .iter()
        .any(|e| e.type_id == "core.worn-by"));
    assert!(!reading
        .after
        .edges
        .iter()
        .any(|e| e.type_id == "core.worn-by"));
    assert!(reading
        .after
        .edges
        .iter()
        .any(|e| e.id == stored.id && e.target_id == w.nodes[2].id));
}
#[test]
fn taking_off_changes_one_placement_and_conflicting_effect_placements_are_rejected() {
    let (_dir, p) = fixture();
    let source = p.view().unwrap().world;
    let wear = source
        .edges
        .iter()
        .find(|e| e.type_id == "core.worn-by")
        .unwrap()
        .clone();
    let carried = WorldEdge {
        type_id: "core.stored-in".into(),
        ..wear.clone()
    };
    let mut edit = edits(&p, "Mara took off her coat and carried it over her arm.");
    let change = PassageEffect::PutLink {
        link: carried.clone(),
        evidence: edit.prose.clone(),
    };
    let conflict = PassageEffect::PutLink {
        link: WorldEdge {
            id: id(),
            ..wear.clone()
        },
        evidence: edit.prose.clone(),
    };
    edit.effects_text = serde_json::to_string(&PassageEffects {
        changes: vec![change.clone(), conflict],
        ..Default::default()
    })
    .unwrap();
    let proposal = id();
    p.save_passage(&proposal, 0, edit.clone()).unwrap();
    assert!(p.validate_passage(&proposal, 1, &settings()).is_err());
    assert_eq!(p.view().unwrap().world, source);
    edit.effects_text = serde_json::to_string(&PassageEffects {
        changes: vec![change],
        ..Default::default()
    })
    .unwrap();
    let accepted = accept(&p, edit);
    let reading = p.read_passage(&accepted).unwrap();
    assert!(reading.before.edges.contains(&wear));
    assert!(reading.after.edges.contains(&carried));
    assert_eq!(
        reading
            .after
            .edges
            .iter()
            .filter(|e| e.source_id == wear.source_id)
            .count(),
        1
    );
}

#[test]
fn prose_edits_invalidate_checked_effects_and_close_choice_is_metadata() {
    let (_dir, p) = fixture();
    let rid = ready(&p, edits(&p, "Mara waited beside the window."));
    let v = p.passage_view(&rid, &settings()).unwrap();
    let mut changed = v.edits;
    changed.prose = "Mara turned away from the window.".into();
    p.save_passage(&rid, v.version, changed).unwrap();
    let v = p.passage_view(&rid, &settings()).unwrap();
    assert!(!v.can_accept);
    assert!(p.accept_passage(&rid, v.version, &settings()).is_err());
    p.validate_passage(&rid, v.version, &settings()).unwrap();
    let v = p.passage_view(&rid, &settings()).unwrap();
    let mut changed = v.edits;
    changed.close_chapter = true;
    p.save_passage(&rid, v.version, changed).unwrap();
    assert!(p.passage_view(&rid, &settings()).unwrap().can_accept);
}
#[test]
fn related_changes_stale_checks_and_unrelated_manual_edits_survive_acceptance() {
    let (_dir, p) = fixture();
    let rid = ready(&p, edits(&p, "Mara waited beside the window."));
    let mut current = p.view().unwrap();
    let extra = node(&current.world, "core.place", "Faraway tower");
    let extra_id = extra.id.clone();
    current.world.nodes.push(extra);
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    assert!(p.passage_view(&rid, &settings()).unwrap().can_accept);
    let v = p.passage_view(&rid, &settings()).unwrap();
    p.accept_passage(&rid, v.version, &settings()).unwrap();
    assert!(p
        .view()
        .unwrap()
        .world
        .nodes
        .iter()
        .any(|n| n.id == extra_id));
    let rid = ready(&p, edits(&p, "Mara waited again beside the window."));
    let mut current = p.view().unwrap();
    current.world.nodes[1].description = "Now distrusts Jon.".into();
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let v = p.passage_view(&rid, &settings()).unwrap();
    assert!(v.stale);
    assert!(!v.can_accept);
}
#[test]
fn extraction_failure_truncation_and_interruption_retain_prose_without_resend() {
    let (dir, p) = fixture();
    let rid = id();
    p.save_passage(&rid, 0, edits(&p, "")).unwrap();
    let prepared = p.start_passage(&rid, 1, 0, false, &settings()).unwrap();
    p.finish_passage(&rid, false, Ok(prepared.demo), false, false, &settings())
        .unwrap();
    let prose = p.passage_view(&rid, &settings()).unwrap().edits.prose;
    let v = p.passage_view(&rid, &settings()).unwrap();
    p.start_passage(&rid, v.version, 0, true, &settings())
        .unwrap();
    p.finish_passage(
        &rid,
        true,
        Err("Fixture extraction failure".into()),
        false,
        false,
        &settings(),
    )
    .unwrap();
    let v = p.passage_view(&rid, &settings()).unwrap();
    assert_eq!(v.edits.prose, prose);
    assert!(!v.can_accept);
    p.start_passage(&rid, v.version, 0, true, &settings())
        .unwrap();
    p.finish_passage(&rid, true, Ok("{broken".into()), false, true, &settings())
        .unwrap();
    let v = p.passage_view(&rid, &settings()).unwrap();
    assert_eq!(v.original_effects, "{broken");
    assert_eq!(v.edits.prose, prose);
    p.start_passage(&rid, v.version, 0, true, &settings())
        .unwrap();
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    p.interrupt_passages().unwrap();
    let v = p.passage_view(&rid, &settings()).unwrap();
    assert_eq!(v.stage, PassageStage::Interrupted);
    assert_eq!(v.edits.prose, prose);
    assert_eq!(p.view().unwrap().revision, 0);
}
#[test]
fn source_changes_during_extraction_cannot_be_silently_rebased() {
    let (_dir, p) = fixture();
    let rid = id();
    p.save_passage(&rid, 0, edits(&p, "Mara waited beside the window."))
        .unwrap();
    p.start_passage(&rid, 1, 0, true, &settings()).unwrap();
    let mut view = p.view().unwrap();
    view.world.nodes[0].description = "The window was removed.".into();
    p.commit(0, id(), view.title, view.world).unwrap();
    p.finish_passage(&rid, true, Ok(empty_effects()), false, false, &settings())
        .unwrap();
    let v = p.passage_view(&rid, &settings()).unwrap();
    assert!(v.stale);
    assert!(!v.can_accept);
    assert!(v.error.contains("source changed"));
}
#[test]
fn unsupported_evidence_and_out_of_context_records_are_rejected() {
    let (_dir, p) = fixture();
    let mut v = p.view().unwrap();
    let secret = node(&v.world, "core.character", "Secret observer");
    let mut changed = secret.clone();
    changed.description = "Revealed.".into();
    v.world.nodes.push(secret);
    p.commit(0, id(), v.title, v.world).unwrap();
    let mut edit = edits(&p, "Mara waited beside the window.");
    edit.effects_text = serde_json::to_string(&PassageEffects {
        changes: vec![PassageEffect::PutNode {
            node: changed,
            evidence: edit.prose.clone(),
        }],
        ..Default::default()
    })
    .unwrap();
    let rid = id();
    p.save_passage(&rid, 0, edit).unwrap();
    assert!(p
        .validate_passage(&rid, 1, &settings())
        .unwrap_err()
        .contains("outside"));
    let w = p.view().unwrap().world;
    let mut changed = w.nodes[1].clone();
    changed.description = "Changed".into();
    let mut edit = edits(&p, "Mara waited beside the window.");
    edit.effects_text = serde_json::to_string(&PassageEffects {
        changes: vec![PassageEffect::PutNode {
            node: changed,
            evidence: "Mara took a train".into(),
        }],
        ..Default::default()
    })
    .unwrap();
    let rid = id();
    p.save_passage(&rid, 0, edit).unwrap();
    assert!(p
        .validate_passage(&rid, 1, &settings())
        .unwrap_err()
        .contains("evidence"));
}
#[test]
fn fork_is_self_contained_and_excludes_future_worlds_plans_and_pending_work() {
    let (dir, p) = fixture();
    let first = accept(&p, edits(&p, "Mara waited beside the window."));
    let mut later = edits(&p, "Jon left after the rain.");
    later.plan.chapter_plan = "A future secret plan.".into();
    later.close_chapter = true;
    accept(&p, later);
    let mut view = p.view().unwrap();
    let future = node(&view.world, "core.place", "Future city");
    view.world.nodes.push(future);
    p.commit(view.revision, id(), view.title, view.world)
        .unwrap();
    p.save_passage(&id(), 0, edits(&p, "A pending secret draft."))
        .unwrap();
    let fork = p
        .fork_after_passage(&first, dir.path(), "Independent fork".into())
        .unwrap();
    let fork_path = std::path::PathBuf::from(fork.folder_path());
    assert_ne!(fork.view().unwrap().story_id, p.view().unwrap().story_id);
    assert_eq!(fork.story_index().unwrap().passages.len(), 1);
    assert!(!fork.story_index().unwrap().chapters[0].closed);
    assert!(!fork
        .view()
        .unwrap()
        .world
        .nodes
        .iter()
        .any(|n| n.name == "Future city"));
    assert!(fork.list_passage_drafts().unwrap().is_empty());
    assert!(!fork_path.join("proposals").exists());
    assert!(!fork.story_index().unwrap().chapters[0]
        .plan
        .contains("future"));
    drop(fork);
    drop(p);
    fs::rename(
        dir.path().join("story"),
        dir.path().join("unavailable-original"),
    )
    .unwrap();
    let fork = Project::open(&fork_path).unwrap();
    assert_eq!(
        fork.read_passage(&first).unwrap().passage.prose,
        "Mara waited beside the window."
    );
}
#[test]
fn acceptance_receipt_reconciles_a_status_write_failure_after_later_world_work() {
    let (_dir, p) = fixture();
    let rid = accept(&p, edits(&p, "Mara waited beside the window."));
    let path = p.passage_path(&rid).unwrap();
    let mut d: PassageDraft = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    d.stage = PassageStage::Ready;
    p.write_passage_draft(&d).unwrap();
    let mut v = p.view().unwrap();
    v.world.nodes[0].description = "Later manual work.".into();
    p.commit(v.revision, id(), v.title, v.world).unwrap();
    let v = p.passage_view(&rid, &settings()).unwrap();
    assert_eq!(v.stage, PassageStage::Accepted);
    let replay = p.accept_passage(&rid, 0, &settings()).unwrap();
    assert!(replay.commit.replayed);
    assert_eq!(replay.commit.project.revision, 2);
    assert_eq!(p.story_index().unwrap().passages.len(), 1);
}
#[test]
fn scene_selection_adds_placement_and_direct_inventory_without_teleporting() {
    let (_dir, p) = fixture();
    let before = p.view().unwrap().world;
    let plan = edits(&p, "").plan;
    let context = scene_world(&before, &plan).unwrap();
    assert_eq!(context.nodes.len(), 4);
    assert!(context.edges.iter().any(|e| e.type_id == "core.worn-by"));
    assert_eq!(p.view().unwrap().revision, 0);
}
#[test]
fn budget_failure_does_not_mark_a_job_running() {
    let (_dir, p) = fixture();
    let rid = id();
    p.save_passage(&rid, 0, edits(&p, "")).unwrap();
    let mut s = settings();
    s.context_tokens = 2048;
    s.max_tokens = 1024;
    assert!(p.start_passage(&rid, 1, 0, false, &s).is_err());
    assert_eq!(
        p.passage_view(&rid, &settings()).unwrap().stage,
        PassageStage::Draft
    );
}
#[test]
fn introduced_outfit_container_and_notes_resolve_aliases_atomically() {
    let (_dir, p) = fixture();
    let w = p.view().unwrap().world;
    let mut cloak = node(&w, "core.item", "Blue cloak");
    cloak.id = "new:cloak".into();
    cloak.components.insert(
        "core.wearable".into(),
        BTreeMap::from([
            ("regions".into(), PropertyValue::Text("torso".into())),
            ("layer".into(), PropertyValue::Number(1.0)),
        ]),
    );
    let mut pouch = node(&w, "core.item", "New pouch");
    pouch.id = "new:pouch".into();
    pouch
        .components
        .insert("core.container".into(), BTreeMap::new());
    let mut note = node(&w, "core.item", "Folded note");
    note.id = "new:note".into();
    let prose = "Mara found a blue cloak, put it on and placed a folded note in a new pouch.";
    let mut changes = vec![cloak, pouch, note]
        .into_iter()
        .map(|node| PassageEffect::PutNode {
            node,
            evidence: prose.into(),
        })
        .collect::<Vec<_>>();
    for (i, kind, source, target) in [
        (1, "core.worn-by", "new:cloak", w.nodes[1].id.as_str()),
        (2, "core.stored-in", "new:pouch", w.nodes[1].id.as_str()),
        (3, "core.stored-in", "new:note", "new:pouch"),
    ] {
        changes.push(PassageEffect::PutLink {
            link: WorldEdge {
                id: format!("new:link{i}"),
                type_id: kind.into(),
                source_id: source.into(),
                target_id: target.into(),
                description: String::new(),
            },
            evidence: prose.into(),
        });
    }
    let mut edit = edits(&p, prose);
    edit.effects_text = serde_json::to_string(&PassageEffects {
        changes,
        notes: vec![ContinuityNote {
            text: "Mara has a blue cloak.".into(),
            evidence: prose.into(),
            node_ids: vec!["new:cloak".into()],
        }],
        suggest_chapter_end: false,
    })
    .unwrap();
    let rid = accept(&p, edit);
    let read = p.read_passage(&rid).unwrap();
    assert_eq!(read.after.nodes.len(), 7);
    assert!(!read.passage.effects.notes[0].node_ids[0].starts_with("new:"));
    assert!(read
        .after
        .edges
        .iter()
        .any(|e| e.type_id == "core.worn-by"
            && e.source_id == read.passage.effects.notes[0].node_ids[0]));
}
#[test]
fn unfinished_plan_titles_save_but_cannot_generate_or_accept() {
    let (_dir, p) = fixture();
    let rid = id();
    let mut edit = edits(&p, "Mara waited beside the window.");
    edit.plan.title.clear();
    edit.plan.chapter_title.clear();
    p.save_passage(&rid, 0, edit.clone()).unwrap();
    assert_eq!(p.passage_view(&rid, &settings()).unwrap().edits, edit);
    assert!(p.start_passage(&rid, 1, 0, false, &settings()).is_err());
    assert!(p.validate_passage(&rid, 1, &settings()).is_err());
    assert_eq!(p.view().unwrap().revision, 0);
}
#[test]
fn interrupted_siblings_are_recovered_even_when_another_draft_is_corrupt() {
    let (_dir, p) = fixture();
    let rid = id();
    p.save_passage(&rid, 0, edits(&p, "")).unwrap();
    p.start_passage(&rid, 1, 0, false, &settings()).unwrap();
    fs::write(p.passage_path(&id()).unwrap(), "{broken").unwrap();
    assert!(p.interrupt_passages().is_err());
    assert_eq!(
        p.passage_view(&rid, &settings()).unwrap().stage,
        PassageStage::Interrupted
    );
}
#[test]
#[ignore = "isolated process-exit probe"]
fn story_crash_child() {
    let Ok(root) = std::env::var("STORY_PASSAGE_CRASH_ROOT") else {
        return;
    };
    let phase = std::env::var("STORY_PASSAGE_CRASH_PHASE").unwrap();
    let p = Project::open(Path::new(&root)).unwrap();
    let v = p.view().unwrap();
    let mut world = v.world;
    world.nodes[0].name = "After transaction".into();
    let mut index = p.story_index().unwrap();
    index.chapters[0].title = "After transaction".into();
    let _ = p.commit_bundle(v.revision, id(), v.title, world, Some(index), |at| {
        if at == phase {
            std::process::exit(86);
        }
        Ok(())
    });
}
#[test]
#[ignore = "creates an isolated fictional desktop fixture when explicitly requested"]
fn desktop_fixture() {
    let Ok(root) = std::env::var("STORY_DESKTOP_FIXTURE") else {
        return;
    };
    let (_dir, p) = fixture();
    let world = p.view().unwrap().world;
    Project::create(
        Path::new(&root),
        "Composer verification".into(),
        world,
        id(),
    )
    .unwrap();
}
#[test]
fn process_exit_cannot_publish_half_a_world_and_story_index() {
    for phase in [
        "after-story",
        "before-world",
        "after-world",
        "after-manifest",
        "before-head",
        "after-head",
    ] {
        let (_dir, p) = fixture();
        accept(&p, edits(&p, "Mara waited beside the window."));
        let path = p.folder_path();
        drop(p);
        let result = Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "story::tests::story_crash_child",
                "--nocapture",
            ])
            .env("STORY_PASSAGE_CRASH_ROOT", &path)
            .env("STORY_PASSAGE_CRASH_PHASE", phase)
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(86),
            "{phase}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let p = Project::open(Path::new(&path)).unwrap();
        let view = p.view().unwrap();
        let index = p.story_index().unwrap();
        assert_eq!(
            view.world.nodes[0].name == "After transaction",
            index.chapters[0].title == "After transaction"
        );
        assert_eq!(view.revision, if phase == "after-head" { 2 } else { 1 });
    }
}
