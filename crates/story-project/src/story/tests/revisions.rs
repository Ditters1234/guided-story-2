use super::*;

fn revision(p: &Project, passage: &str, kind: PassageRevisionKind, prose: &str) -> String {
    let draft = id();
    p.begin_passage_revision(
        &draft,
        passage,
        p.read_passage(passage).unwrap().record,
        kind,
    )
    .unwrap();
    let mut view = p.passage_view(&draft, &settings()).unwrap();
    view.edits.prose = prose.into();
    p.save_passage(&draft, view.version, view.edits).unwrap();
    draft
}
fn accept_revision(p: &Project, draft: &str) -> PassageAcceptance {
    let view = p.passage_view(draft, &settings()).unwrap();
    p.validate_passage(draft, view.version, &settings())
        .unwrap();
    p.accept_passage(
        draft,
        p.passage_view(draft, &settings()).unwrap().version,
        &settings(),
    )
    .unwrap()
}
fn handoff(p: &Project) -> String {
    let world = p.view().unwrap().world;
    let prose = "Mara handed her field coat to Jon.";
    let mut draft = edits(p, prose);
    draft.effects_text = serde_json::to_string(&PassageEffects {
        changes: vec![PassageEffect::PutLink {
            link: WorldEdge {
                properties: Default::default(),
                id: "new:handoff".into(),
                type_id: "core.stored-in".into(),
                source_id: world.nodes[3].id.clone(),
                target_id: world.nodes[2].id.clone(),
                description: String::new(),
            },
            evidence: prose.into(),
        }],
        notes: vec![ContinuityNote {
            text: "Jon received the coat.".into(),
            evidence: prose.into(),
            node_ids: vec![world.nodes[2].id.clone()],
        }],
        suggest_chapter_end: false,
    })
    .unwrap();
    accept(p, draft)
}

#[test]
fn old_wording_creates_an_immutable_version_without_rewriting_events_or_later_work() {
    let (dir, p) = fixture();
    let first = accept(&p, edits(&p, "Mara waited beside the window."));
    let original = p.read_passage(&first).unwrap();
    let later = accept(&p, edits(&p, "Jon listened to the rain."));
    let later_record = p.read_passage(&later).unwrap().record;
    let mut current = p.view().unwrap();
    current.world.nodes[0].description = "A later manual edit.".into();
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let before = p.view().unwrap();
    let draft = revision(
        &p,
        &first,
        PassageRevisionKind::Wording,
        "Beside the window, Mara waited.",
    );
    assert_eq!(p.view().unwrap().revision, before.revision);
    let result = accept_revision(&p, &draft);
    assert_eq!(result.passage_id, first);
    assert_eq!(result.commit.project.world, before.world);
    let revised = p.read_passage(&first).unwrap();
    assert_eq!(
        revised.passage.previous_version,
        Some(original.record.clone())
    );
    assert_eq!(revised.passage.before_world, original.passage.before_world);
    assert_eq!(revised.passage.after_world, original.passage.after_world);
    assert_eq!(p.read_passage(&later).unwrap().record, later_record);
    let retained: AcceptedPassage = p.read_record("passages", &original.record).unwrap();
    assert_eq!(retained.prose, original.passage.prose);
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    assert_eq!(
        p.read_passage(&first).unwrap().passage.prose,
        "Beside the window, Mara waited."
    );
    assert_eq!(
        p.passage_versions(&first).unwrap().versions[0].prose,
        retained.prose
    );
    assert!(p.list_passage_drafts().unwrap().is_empty());
}

#[test]
fn wording_reanchors_quotes_or_explicitly_keeps_the_original_evidence_version() {
    let (_dir, p) = fixture();
    let passage = handoff(&p);
    let original = p.read_passage(&passage).unwrap();
    let draft = revision(
        &p,
        &passage,
        PassageRevisionKind::Wording,
        "Jon received the field coat from Mara.",
    );
    accept_revision(&p, &draft);
    let revised = p.read_passage(&passage).unwrap();
    assert_eq!(revised.passage.evidence_source, Some(original.record));
    assert_eq!(revised.evidence_prose, Some(original.passage.prose.clone()));
    let draft = revision(
        &p,
        &passage,
        PassageRevisionKind::Wording,
        "Mara passed her field coat to Jon.",
    );
    accept_revision(&p, &draft);
    assert_eq!(
        p.read_passage(&passage).unwrap().evidence_prose,
        Some(original.passage.prose)
    );
    let draft = revision(
        &p,
        &passage,
        PassageRevisionKind::Wording,
        "Mara handed her field coat to Jon. Rain rattled against the glass.",
    );
    accept_revision(&p, &draft);
    assert!(p
        .read_passage(&passage)
        .unwrap()
        .passage
        .evidence_source
        .is_none());
    assert_eq!(p.passage_versions(&passage).unwrap().versions.len(), 3);
}

#[test]
fn wording_cannot_change_the_plan_closing_choice_or_effects_and_edits_invalidate_review() {
    let (_dir, p) = fixture();
    let passage = accept(&p, edits(&p, "Mara waited beside the window."));
    let draft = revision(
        &p,
        &passage,
        PassageRevisionKind::Wording,
        "Beside the window, Mara waited.",
    );
    for field in 0..3 {
        let mut view = p.passage_view(&draft, &settings()).unwrap();
        let clean = view.edits.clone();
        match field {
            0 => view.edits.plan.title = "Changed event plan".into(),
            1 => view.edits.close_chapter = true,
            _ => view.edits.effects_text = "{}".into(),
        }
        p.save_passage(&draft, view.version, view.edits).unwrap();
        let view = p.passage_view(&draft, &settings()).unwrap();
        assert!(!view.can_accept);
        assert!(p
            .validate_passage(&draft, view.version, &settings())
            .is_err());
        p.save_passage(&draft, view.version, clean).unwrap();
    }
    let mut view = p.passage_view(&draft, &settings()).unwrap();
    p.validate_passage(&draft, view.version, &settings())
        .unwrap();
    view = p.passage_view(&draft, &settings()).unwrap();
    assert!(view.can_accept);
    view.edits.prose.push_str(" ");
    p.save_passage(&draft, view.version, view.edits).unwrap();
    let view = p.passage_view(&draft, &settings()).unwrap();
    assert!(!view.can_accept);
    assert!(p.accept_passage(&draft, view.version, &settings()).is_err());
}

#[test]
fn latest_event_replacement_uses_before_world_excludes_old_prose_and_keeps_passage_identity() {
    let (_dir, p) = fixture();
    let passage = handoff(&p);
    let original = p.read_passage(&passage).unwrap();
    let draft = revision(
        &p,
        &passage,
        PassageRevisionKind::Events,
        "Mara kept her field coat and waited.",
    );
    let mut view = p.passage_view(&draft, &settings()).unwrap();
    let baseline = view.revision.as_ref().unwrap().world.clone();
    assert_eq!(baseline, original.before);
    view.edits.plan.direction = "Mara keeps her coat and waits.".into();
    view.edits.close_chapter = true;
    p.save_passage(&draft, view.version, view.edits).unwrap();
    let view = p.passage_view(&draft, &settings()).unwrap();
    let request = p
        .start_passage(
            &draft,
            view.version,
            p.view().unwrap().revision,
            false,
            &settings(),
        )
        .unwrap();
    assert!(!request.user.contains("handed her field coat"));
    assert!(request.user.contains("\"recentPassages\":[]"));
    p.finish_passage(
        &draft,
        false,
        Ok("Mara kept her field coat and waited.".into()),
        false,
        false,
        &settings(),
    )
    .unwrap();
    let result = accept_revision(&p, &draft);
    assert_eq!(result.passage_id, passage);
    assert_eq!(result.commit.project.world, baseline);
    let index = p.story_index().unwrap();
    assert_eq!(index.passages.len(), 1);
    assert!(index.chapters[0].closed);
    let revised = p.read_passage(&passage).unwrap();
    assert_eq!(revised.passage.previous_version, Some(original.record));
    assert_eq!(revised.passage.before_world, original.passage.before_world);
    assert_eq!(revised.passage.chapter_id, original.passage.chapter_id);
    assert_eq!(
        p.passage_versions(&passage).unwrap().versions[0].prose,
        original.passage.prose
    );
}

#[test]
fn replacement_blocks_earlier_targets_and_intervening_world_edits_at_every_boundary() {
    let (_dir, p) = fixture();
    let first = accept(&p, edits(&p, "Mara waited beside the window."));
    let last = accept(&p, edits(&p, "Jon listened to the rain."));
    assert!(p
        .begin_passage_revision(
            &id(),
            &first,
            p.read_passage(&first).unwrap().record,
            PassageRevisionKind::Events
        )
        .is_err());
    let draft = revision(
        &p,
        &last,
        PassageRevisionKind::Events,
        "Jon kept listening to the rain.",
    );
    let view = p.passage_view(&draft, &settings()).unwrap();
    p.validate_passage(&draft, view.version, &settings())
        .unwrap();
    let mut current = p.view().unwrap();
    current.world.nodes[0].description = "An intervening edit that must not be rolled back.".into();
    p.commit(current.revision, id(), current.title, current.world.clone())
        .unwrap();
    let view = p.passage_view(&draft, &settings()).unwrap();
    assert!(view.stale);
    assert!(!view.can_accept);
    assert!(p
        .validate_passage(&draft, view.version, &settings())
        .is_err());
    assert!(p.accept_passage(&draft, view.version, &settings()).is_err());
    assert!(p
        .begin_passage_revision(
            &id(),
            &last,
            p.read_passage(&last).unwrap().record,
            PassageRevisionKind::Events
        )
        .is_err());
    assert_eq!(p.view().unwrap().world, current.world);
}

#[test]
fn revision_source_changes_stale_candidates_but_later_acceptance_does_not_block_old_wording() {
    let (_dir, p) = fixture();
    let first = accept(&p, edits(&p, "Mara waited beside the window."));
    let a = revision(
        &p,
        &first,
        PassageRevisionKind::Wording,
        "Beside the window, Mara waited.",
    );
    let b = revision(
        &p,
        &first,
        PassageRevisionKind::Wording,
        "Mara waited by the window.",
    );
    let continuation = ready(&p, edits(&p, "Jon listened to the rain."));
    accept_revision(&p, &a);
    let b_view = p.passage_view(&b, &settings()).unwrap();
    assert!(b_view.error.contains("was revised"));
    assert!(!b_view.can_accept);
    assert!(
        !p.passage_view(&continuation, &settings())
            .unwrap()
            .can_accept
    );
    let old_wording = revision(
        &p,
        &first,
        PassageRevisionKind::Wording,
        "Mara remained beside the window.",
    );
    let view = p.passage_view(&old_wording, &settings()).unwrap();
    p.validate_passage(&old_wording, view.version, &settings())
        .unwrap();
    accept(&p, edits(&p, "Jon listened to the rain."));
    assert!(
        p.passage_view(&old_wording, &settings())
            .unwrap()
            .can_accept
    );
    accept_revision(&p, &old_wording);
    assert_eq!(p.story_index().unwrap().passages.len(), 2);
}

#[test]
fn accepted_revision_receipts_reconcile_even_after_new_versions_and_later_world_edits() {
    let (_dir, p) = fixture();
    let passage = accept(&p, edits(&p, "Mara waited beside the window."));
    let draft = revision(
        &p,
        &passage,
        PassageRevisionKind::Wording,
        "Beside the window, Mara waited.",
    );
    let first_result = accept_revision(&p, &draft);
    let newer = revision(
        &p,
        &passage,
        PassageRevisionKind::Wording,
        "Mara waited by the window.",
    );
    accept_revision(&p, &newer);
    let mut d = p.read_passage_draft(&draft).unwrap();
    d.stage = PassageStage::Ready;
    p.write_passage_draft(&d).unwrap();
    let mut world = p.view().unwrap();
    world.world.nodes[0].description = "Later work.".into();
    p.commit(world.revision, id(), world.title, world.world)
        .unwrap();
    let replay = p.accept_passage(&draft, 0, &settings()).unwrap();
    assert!(replay.commit.replayed);
    assert_eq!(
        replay.commit.committed_revision,
        first_result.commit.committed_revision
    );
    assert_eq!(
        p.read_passage(&passage).unwrap().passage.prose,
        "Mara waited by the window."
    );
    assert!(p.list_passage_drafts().unwrap().is_empty());
}

#[test]
fn ai_polish_and_reroll_keep_revision_scope_and_saved_manual_candidates() {
    let (dir, p) = fixture();
    let passage = handoff(&p);
    let draft = revision(
        &p,
        &passage,
        PassageRevisionKind::Wording,
        "Jon received the coat from Mara.",
    );
    let mut view = p.passage_view(&draft, &settings()).unwrap();
    view.edits.plan.direction = "Make the sentences more direct without changing events.".into();
    p.save_passage(&draft, view.version, view.edits).unwrap();
    let view = p.passage_view(&draft, &settings()).unwrap();
    let candidate = id();
    p.clone_passage_draft(&draft, view.version, &candidate)
        .unwrap();
    p.clone_passage_draft(&draft, view.version, &candidate)
        .unwrap();
    let cloned = p.passage_view(&candidate, &settings()).unwrap();
    assert!(cloned.edits.prose.is_empty());
    assert_eq!(cloned.revision.unwrap().kind, PassageRevisionKind::Wording);
    assert_eq!(
        p.passage_view(&draft, &settings()).unwrap().edits.prose,
        "Jon received the coat from Mara."
    );
    let request = p
        .start_passage(
            &candidate,
            cloned.version,
            p.view().unwrap().revision,
            false,
            &settings(),
        )
        .unwrap();
    assert!(request.system.contains("wording polish only"));
    assert!(request.user.contains("originalProse"));
    assert!(request.schema.is_none());
    p.finish_passage(
        &candidate,
        false,
        Ok("Jon received the field coat from Mara.".into()),
        false,
        false,
        &settings(),
    )
    .unwrap();
    let view = p.passage_view(&candidate, &settings()).unwrap();
    assert!(!view.can_accept);
    assert!(!view.edits.effects_text.contains("\"changes\":[]"));
    assert!(p
        .start_passage(
            &candidate,
            view.version,
            p.view().unwrap().revision,
            true,
            &settings()
        )
        .is_err());
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    assert_eq!(
        p.passage_view(&candidate, &settings())
            .unwrap()
            .revision
            .unwrap()
            .kind,
        PassageRevisionKind::Wording
    );
    accept_revision(&p, &candidate);
}

#[test]
fn forks_omit_superseded_alternate_events_and_keep_needed_wording_evidence() {
    let (dir, p) = fixture();
    let passage = handoff(&p);
    let draft = revision(
        &p,
        &passage,
        PassageRevisionKind::Events,
        "Mara kept her coat instead of handing it over.",
    );
    accept_revision(&p, &draft);
    let fork = p
        .fork_after_passage(&passage, dir.path(), "No alternate events".into())
        .unwrap();
    assert!(fork.passage_versions(&passage).unwrap().versions.is_empty());
    let content = fs::read_dir(Path::new(&fork.folder_path()).join("passages"))
        .unwrap()
        .map(|e| fs::read_to_string(e.unwrap().path()).unwrap())
        .collect::<String>();
    assert!(!content.contains("Mara handed her field coat to Jon."));
    drop(fork);
    let word = revision(
        &p,
        &passage,
        PassageRevisionKind::Wording,
        "Mara retained her coat and waited.",
    );
    accept_revision(&p, &word);
    let fork = p
        .fork_after_passage(&passage, dir.path(), "Worded independent copy".into())
        .unwrap();
    let fork_path = fork.folder_path();
    assert!(fork.passage_versions(&passage).unwrap().versions.is_empty());
    drop(fork);
    drop(p);
    fs::rename(
        dir.path().join("story"),
        dir.path().join("unavailable-source"),
    )
    .unwrap();
    let fork = Project::open(Path::new(&fork_path)).unwrap();
    assert_eq!(
        fork.read_passage(&passage).unwrap().passage.prose,
        "Mara retained her coat and waited."
    );
}

#[test]
fn wording_fork_materializes_its_old_quote_without_copying_superseded_history() {
    let (dir, p) = fixture();
    let passage = handoff(&p);
    let draft = revision(
        &p,
        &passage,
        PassageRevisionKind::Wording,
        "Jon received the coat from Mara.",
    );
    accept_revision(&p, &draft);
    let fork = p
        .fork_after_passage(&passage, dir.path(), "Anchored wording copy".into())
        .unwrap();
    let reading = fork.read_passage(&passage).unwrap();
    assert!(reading
        .evidence_prose
        .unwrap()
        .contains("Mara handed her field coat to Jon."));
    assert!(reading.passage.previous_version.is_none());
    let anchor: AcceptedPassage = fork
        .read_record(
            "passages",
            reading.passage.evidence_source.as_ref().unwrap(),
        )
        .unwrap();
    assert!(anchor.previous_version.is_none());
    assert!(anchor.evidence_source.is_none());
}
