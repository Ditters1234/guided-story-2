use super::*;

#[test]
fn missing_link_and_incompatible_relationship_are_actionable_even_after_all_cards_are_approved() {
    let (dir, p) = fixture();
    let before = p.view().unwrap();
    let old = before
        .world
        .edges
        .iter()
        .find(|e| e.type_id == "core.present-at")
        .unwrap()
        .clone();
    let mut edit = edits(&p, "Mara entered the glasshouse and watched the rain.");
    let mut effects = PassageEffects {
        changes: vec![
            PassageEffect::PutLink {
                link: WorldEdge {
                    properties: Default::default(),
                    id: id(),
                    type_id: "core.inside".into(),
                    ..old.clone()
                },
                evidence: edit.prose.clone(),
            },
            PassageEffect::RemoveLink {
                id: old.id.clone(),
                evidence: edit.prose.clone(),
            },
        ],
        notes: (0..3)
            .map(|i| ContinuityNote {
                text: format!("Scene observation {i}."),
                evidence: edit.prose.clone(),
                node_ids: vec![],
            })
            .collect(),
        ..Default::default()
    };
    edit.effects_text = serde_json::to_string(&effects).unwrap();
    let draft = id();
    p.save_passage(&draft, 0, edit).unwrap();
    let error = p.validate_passage(&draft, 1, &settings()).unwrap_err();
    assert!(error.contains("changes[0]") && error.contains("connection does not exist"));
    let approved = p.approve_passage_for_test(&draft, &settings()).unwrap();
    assert_eq!(approved.effects_review.entries.len(), 5);
    assert!(approved.effects_review.entries.iter().all(|e| e.approved));
    assert!(!approved.effects_review.checked && !approved.can_accept);
    let issues = &approved.effects_review.entries[0].issues;
    assert!(issues.iter().any(|s| s.contains("Create new connection")));
    assert!(issues
        .iter()
        .any(|s| s.contains("Mara") && s.contains("Place capability")));
    assert!(approved.effects_review.entries[1..]
        .iter()
        .all(|e| e.issues.is_empty()));
    assert!(p
        .accept_passage(&draft, approved.version, &settings())
        .is_err());
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    let restored = p.passage_view(&draft, &settings()).unwrap();
    assert_eq!(restored.effects_review.entries[0].issues, *issues);
    assert_eq!(
        restored.edits.prose,
        "Mara entered the glasshouse and watched the rain."
    );
    let mut edit = restored.edits;
    if let PassageEffect::PutLink { link, .. } = &mut effects.changes[0] {
        link.id = "new:movement".into();
        link.type_id = "core.present-at".into();
    }
    edit.effects_text = serde_json::to_string(&effects).unwrap();
    p.save_passage(&draft, restored.version, edit).unwrap();
    let repaired = p.passage_view(&draft, &settings()).unwrap();
    assert!(repaired
        .effects_review
        .entries
        .iter()
        .all(|e| e.issues.is_empty()));
    assert!(!repaired.effects_review.entries[0].approved);
    assert!(repaired.effects_review.entries[1..]
        .iter()
        .all(|e| e.approved));
    p.validate_passage(&draft, repaired.version, &settings())
        .unwrap();
    let ready = p.approve_passage_for_test(&draft, &settings()).unwrap();
    assert!(ready.can_accept);
    assert_eq!(p.view().unwrap().world_ref, before.world_ref);
    p.accept_passage(&draft, ready.version, &settings())
        .unwrap();
    let after = p.read_passage(&draft).unwrap().after;
    let placements = after
        .edges
        .iter()
        .filter(|e| e.source_id == old.source_id)
        .collect::<Vec<_>>();
    assert_eq!(placements.len(), 1);
    assert_ne!(placements[0].id, old.id);
    assert_eq!(placements[0].type_id, "core.present-at");
}

#[test]
fn conflicting_updates_are_marked_on_both_cards_and_can_be_repaired_as_an_existing_link_update() {
    let (_dir, p) = fixture();
    let world = p.view().unwrap().world;
    let old = world
        .edges
        .iter()
        .find(|e| e.type_id == "core.worn-by")
        .unwrap()
        .clone();
    let mut edit = edits(&p, "Mara took off her coat and carried it.");
    let mut effects = PassageEffects {
        changes: vec![
            PassageEffect::PutLink {
                link: WorldEdge {
                    properties: Default::default(),
                    type_id: "core.stored-in".into(),
                    ..old.clone()
                },
                evidence: edit.prose.clone(),
            },
            PassageEffect::RemoveLink {
                id: old.id.clone(),
                evidence: edit.prose.clone(),
            },
        ],
        ..Default::default()
    };
    edit.effects_text = serde_json::to_string(&effects).unwrap();
    let draft = id();
    p.save_passage(&draft, 0, edit).unwrap();
    let view = p.passage_view(&draft, &settings()).unwrap();
    assert!(view
        .effects_review
        .entries
        .iter()
        .all(|e| e.issues.iter().any(|s| s.contains("same record"))));
    effects.changes.remove(1);
    let mut edit = view.edits;
    edit.effects_text = serde_json::to_string(&effects).unwrap();
    p.save_passage(&draft, view.version, edit).unwrap();
    let repaired = p.passage_view(&draft, &settings()).unwrap();
    assert!(repaired.effects_review.entries[0].issues.is_empty());
    p.validate_passage(&draft, repaired.version, &settings())
        .unwrap();
    let ready = p.approve_passage_for_test(&draft, &settings()).unwrap();
    p.accept_passage(&draft, ready.version, &settings())
        .unwrap();
    assert!(p
        .read_passage(&draft)
        .unwrap()
        .after
        .edges
        .iter()
        .any(|e| e.id == old.id && e.type_id == "core.stored-in"));
}
