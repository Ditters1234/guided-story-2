use super::*;

fn candidate(p: &Project) -> String {
    let world = p.view().unwrap().world;
    let prose = "Mara took off her coat and carried it. Jon waited beside the window.";
    let wear = world
        .edges
        .iter()
        .find(|e| e.type_id == "core.worn-by")
        .unwrap();
    let mut edit = edits(p, prose);
    edit.effects_text = serde_json::to_string(&PassageEffects {
        changes: vec![PassageEffect::PutLink {
            link: WorldEdge {
                properties: Default::default(),
                type_id: "core.stored-in".into(),
                ..wear.clone()
            },
            evidence: "Mara took off her coat and carried it.".into(),
        }],
        notes: vec![ContinuityNote {
            text: "Jon waited by the window.".into(),
            evidence: "Jon waited beside the window.".into(),
            node_ids: vec![world.nodes[2].id.clone()],
        }],
        ..Default::default()
    })
    .unwrap();
    let id = id();
    p.save_passage(&id, 0, edit).unwrap();
    p.validate_passage(&id, 1, &settings()).unwrap();
    id
}

#[test]
fn every_included_card_requires_durable_approval_before_atomic_acceptance() {
    let (dir, p) = fixture();
    let before = p.view().unwrap();
    let id = candidate(&p);
    let view = p.passage_view(&id, &settings()).unwrap();
    assert!(view.effects_review.checked);
    assert!(!view.can_accept);
    assert_eq!(view.effects_review.entries.len(), 2);
    assert!(p
        .accept_passage(&id, view.version, &settings())
        .unwrap_err()
        .contains("Approve each included"));
    let mut edit = view.edits;
    edit.approved_effects = vec![view.effects_review.entries[0].key.clone()];
    p.save_passage(&id, view.version, edit).unwrap();
    let partial = p.passage_view(&id, &settings()).unwrap();
    assert!(partial.effects_review.checked);
    assert!(!partial.can_accept);
    assert_eq!(
        partial
            .effects_review
            .entries
            .iter()
            .filter(|e| e.approved)
            .count(),
        1
    );
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    assert_eq!(
        p.passage_view(&id, &settings())
            .unwrap()
            .effects_review
            .entries
            .iter()
            .filter(|e| e.approved)
            .count(),
        1
    );
    let ready = p.approve_passage_for_test(&id, &settings()).unwrap();
    assert!(ready.can_accept);
    assert_eq!(p.view().unwrap().world_ref, before.world_ref);
    assert!(p.story_index().unwrap().passages.is_empty());
    p.accept_passage(&id, ready.version, &settings()).unwrap();
    let reading = p.read_passage(&id).unwrap();
    assert!(reading
        .before
        .edges
        .iter()
        .any(|e| e.type_id == "core.worn-by"));
    assert!(reading
        .after
        .edges
        .iter()
        .any(|e| e.type_id == "core.stored-in"));
    assert!(
        p.accept_passage(&id, ready.version, &settings())
            .unwrap()
            .commit
            .replayed
    );
}

#[test]
fn disjoint_updates_to_one_node_keep_separate_approvals_and_publish_together() {
    let (dir, p) = fixture();
    let before = p.view().unwrap();
    let saved = &before.world.nodes[1];
    let injury = "Glass cut Mara's hand.";
    let emotion = "She felt afraid.";
    let mut edit = edits(&p, &format!("{injury} {emotion}"));
    let mut condition_card = saved.clone();
    let mut intent_card = saved.clone();
    condition_card.components.get_mut("detail.condition").unwrap().insert(
        "condition".into(), PropertyValue::Text("Cut hand".into()),
    );
    intent_card.components.get_mut("detail.intent").unwrap().insert(
        "emotion".into(), PropertyValue::Text("Afraid".into()),
    );
    let mut effects = PassageEffects {
        changes: vec![
            PassageEffect::PutNode { node: condition_card, evidence: injury.into() },
            PassageEffect::PutNode { node: intent_card, evidence: emotion.into() },
        ],
        notes: vec![ContinuityNote {
            text: "Mara suffered a cut.".into(),
            node_ids: vec![saved.id.clone()],
            evidence: injury.into(),
        }],
        ..Default::default()
    };
    edit.effects_text = serde_json::to_string(&effects).unwrap();
    let draft = ready(&p, edit);
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    let approved = p.passage_view(&draft, &settings()).unwrap();
    assert!(approved.can_accept);
    assert!(approved.effects_review.entries.iter().all(|card| card.issues.is_empty()));
    assert_eq!(p.view().unwrap().world_ref, before.world_ref);

    let mut edit = approved.edits;
    let PassageEffect::PutNode { node, .. } = &mut effects.changes[0] else { unreachable!() };
    node.components.get_mut("detail.condition").unwrap().insert(
        "condition".into(), PropertyValue::Text("Cut on her hand".into()),
    );
    let condition = node.components["detail.condition"].clone();
    edit.effects_text = serde_json::to_string(&effects).unwrap();
    p.save_passage(&draft, approved.version, edit).unwrap();
    let changed = p.passage_view(&draft, &settings()).unwrap();
    assert!(!changed.effects_review.entries[0].approved);
    assert!(changed.effects_review.entries[1].approved);
    assert!(!changed.effects_review.entries[2].approved);
    assert!(!changed.can_accept);

    p.validate_passage(&draft, changed.version, &settings()).unwrap();
    let approved = p.approve_passage_for_test(&draft, &settings()).unwrap();
    assert_eq!(p.view().unwrap().world, before.world);
    p.accept_passage(&draft, approved.version, &settings()).unwrap();
    let reading = p.read_passage(&draft).unwrap();
    let mut expected = before.world;
    expected.nodes[1].components.insert("detail.condition".into(), condition);
    expected.nodes[1].components.get_mut("detail.intent").unwrap().insert(
        "emotion".into(), PropertyValue::Text("Afraid".into()),
    );
    assert_eq!(reading.after, expected);
    assert_eq!(reading.passage.effects.changes.len(), 2);
}

#[test]
fn overlapping_updates_mark_only_the_cards_that_change_the_same_field() {
    let (_dir, p) = fixture();
    let world = p.view().unwrap().world;
    let saved = &world.nodes[1];
    let prose = "Mara asked to be called Fern.";
    let changes = [
        WorldNode { name: "Fern".into(), ..saved.clone() },
        WorldNode { name: "Willow".into(), ..saved.clone() },
        WorldNode { description: "Goes by a new name.".into(), ..saved.clone() },
    ]
    .into_iter()
    .map(|node| PassageEffect::PutNode { node, evidence: prose.into() })
    .collect();
    let effects = PassageEffects { changes, ..Default::default() };
    let issues = effect_record_issues(&world, &world, &effects);
    assert!(issues[..2].iter().all(|card| card.iter().any(|issue| issue.contains("also changes Name"))));
    assert!(issues[2].is_empty());
    assert!(preview_passage_effects(&world, &world, prose, &effects, &mut BTreeMap::new()).is_err());
}

#[test]
fn editing_a_proposed_record_invalidates_it_and_dependants_but_preserves_other_approvals() {
    let (_dir, p) = fixture();
    let world = p.view().unwrap().world;
    let mut room = node(&world, "core.place", "Study");
    room.id = "new:study".into();
    let movement = world
        .edges
        .iter()
        .find(|e| e.type_id == "core.present-at" && e.source_id == world.nodes[1].id)
        .unwrap();
    let mut edit = edits(&p, "Mara entered the study. Jon waited beside the window.");
    edit.effects_text = serde_json::to_string(&PassageEffects {
        changes: vec![
            PassageEffect::PutNode {
                node: room,
                evidence: "Mara entered the study.".into(),
            },
            PassageEffect::PutLink {
                link: WorldEdge {
                    properties: Default::default(),
                    target_id: "new:study".into(),
                    ..movement.clone()
                },
                evidence: "Mara entered the study.".into(),
            },
        ],
        notes: vec![ContinuityNote {
            text: "Jon waited.".into(),
            evidence: "Jon waited beside the window.".into(),
            node_ids: vec![world.nodes[2].id.clone()],
        }],
        ..Default::default()
    })
    .unwrap();
    let id = id();
    p.save_passage(&id, 0, edit).unwrap();
    p.validate_passage(&id, 1, &settings()).unwrap();
    let ready = p.approve_passage_for_test(&id, &settings()).unwrap();
    assert!(ready.can_accept);
    let mut edit = ready.edits;
    let mut effects: PassageEffects = serde_json::from_str(&edit.effects_text).unwrap();
    if let PassageEffect::PutNode { node, .. } = &mut effects.changes[0] {
        node.components.get_mut("detail.design").unwrap().insert("materials".into(), PropertyValue::Text("Dark oak panels.".into()));
    }
    edit.effects_text = serde_json::to_string(&effects).unwrap();
    p.save_passage(&id, ready.version, edit).unwrap();
    let changed = p.passage_view(&id, &settings()).unwrap();
    assert!(!changed.effects_review.checked);
    assert!(!changed.can_accept);
    assert!(!changed.effects_review.entries[0].approved);
    assert!(!changed.effects_review.entries[1].approved);
    assert!(changed.effects_review.entries[2].approved);
    p.validate_passage(&id, changed.version, &settings())
        .unwrap();
    let ready = p.approve_passage_for_test(&id, &settings()).unwrap();
    let mut edit = ready.edits;
    edit.prose.push_str(" Mara looked around.");
    p.save_passage(&id, ready.version, edit).unwrap();
    assert!(p
        .passage_view(&id, &settings())
        .unwrap()
        .effects_review
        .entries
        .iter()
        .all(|e| !e.approved));
}

#[test]
fn removing_an_unrelated_card_preserves_content_approvals_and_duplicate_keys_are_distinct() {
    let (_dir, p) = fixture();
    let mut edit = edits(&p, "Mara waited beside the window.");
    let note = ContinuityNote {
        text: "Mara waited.".into(),
        evidence: edit.prose.clone(),
        node_ids: vec![],
    };
    edit.effects_text = serde_json::to_string(&PassageEffects {
        notes: vec![
            note.clone(),
            ContinuityNote {
                text: "She remained at the window.".into(),
                ..note.clone()
            },
            note,
        ],
        ..Default::default()
    })
    .unwrap();
    let id = id();
    p.save_passage(&id, 0, edit).unwrap();
    p.validate_passage(&id, 1, &settings()).unwrap();
    let ready = p.approve_passage_for_test(&id, &settings()).unwrap();
    assert_ne!(
        ready.effects_review.entries[0].key,
        ready.effects_review.entries[2].key
    );
    let mut edit = ready.edits;
    let mut effects: PassageEffects = serde_json::from_str(&edit.effects_text).unwrap();
    effects.notes.remove(1);
    edit.effects_text = serde_json::to_string(&effects).unwrap();
    p.save_passage(&id, ready.version, edit).unwrap();
    let changed = p.passage_view(&id, &settings()).unwrap();
    assert!(changed.effects_review.entries.iter().all(|e| e.approved));
    assert!(!changed.can_accept);
    p.validate_passage(&id, changed.version, &settings())
        .unwrap();
    assert!(p.passage_view(&id, &settings()).unwrap().can_accept);
}

#[test]
fn forged_approvals_and_changed_source_facts_cannot_bypass_review() {
    let (_dir, p) = fixture();
    let id = candidate(&p);
    let view = p.passage_view(&id, &settings()).unwrap();
    let mut edit = view.edits;
    edit.approved_effects = vec!["0".repeat(64)];
    p.save_passage(&id, view.version, edit).unwrap();
    let view = p.passage_view(&id, &settings()).unwrap();
    assert!(!view.can_accept);
    assert!(p.accept_passage(&id, view.version, &settings()).is_err());
    let ready = p.approve_passage_for_test(&id, &settings()).unwrap();
    let mut current = p.view().unwrap();
    current.world.nodes[1].description = "Now distrusts Jon.".into();
    p.commit(current.revision, super::id(), current.title, current.world)
        .unwrap();
    let stale = p.passage_view(&id, &settings()).unwrap();
    assert!(stale.stale);
    assert!(!stale.can_accept);
    assert!(stale.effects_review.entries.iter().all(|e| !e.approved));
    assert!(p.accept_passage(&id, ready.version, &settings()).is_err());
}

#[test]
fn regenerating_the_same_proposals_resets_approval_and_never_sends_decisions_to_the_model() {
    let (_dir, p) = fixture();
    let id = candidate(&p);
    let ready = p.approve_passage_for_test(&id, &settings()).unwrap();
    let request = p
        .start_passage(&id, ready.version, 0, true, &settings())
        .unwrap();
    assert!(!request.user.contains("approvedEffects"));
    for entry in &ready.effects_review.entries {
        assert!(!request.user.contains(&entry.key));
    }
    p.finish_passage(
        &id,
        true,
        Ok(ready.edits.effects_text),
        false,
        false,
        &settings(),
    )
    .unwrap();
    let regenerated = p.passage_view(&id, &settings()).unwrap();
    assert!(regenerated.effects_review.checked);
    assert!(!regenerated.can_accept);
    assert!(regenerated
        .effects_review
        .entries
        .iter()
        .all(|e| !e.approved));
    assert_eq!(p.view().unwrap().revision, 0);
}
