use super::*;

fn single_relation(world: &mut World) {
    let mut relation = world
        .relation_types
        .iter()
        .find(|r| r.id == "core.related")
        .unwrap()
        .clone();
    relation.id = "custom.single".into();
    relation.name = "Single connection".into();
    relation.single_target = true;
    world.relation_types.push(relation);
}

fn proposal(link: WorldEdge, prose: &str) -> PassageEffect {
    PassageEffect::PutLink {
        link,
        evidence: prose.into(),
    }
}

#[test]
fn implicit_replacement_requires_the_previous_target_and_tracks_its_reviewed_state() {
    for existing_id in [false, true] {
        let (_dir, p) = fixture();
        let before = p.view().unwrap();
        let mut world = before.world;
        single_relation(&mut world);
        let lena = node(&world, "core.character", "Lena");
        world.nodes.push(lena.clone());
        let old = WorldEdge {
            id: id(),
            type_id: "custom.single".into(),
            source_id: world.nodes[1].id.clone(),
            target_id: lena.id.clone(),
            description: "Saved connection".into(),
            properties: Default::default(),
        };
        world.edges.push(old.clone());
        let mut link = WorldEdge {
            id: "new:replacement".into(),
            target_id: world.nodes[2].id.clone(),
            description: "Reviewed connection".into(),
            ..old.clone()
        };
        if existing_id {
            let mut supplied = link.clone();
            supplied.id = id();
            supplied.type_id = "core.related".into();
            link.id = supplied.id.clone();
            world.edges.push(supplied);
        }
        p.commit(before.revision, id(), before.title, world)
            .unwrap();
        let canonical = p.view().unwrap();
        let prose = "Mara waited beside Jon at the window.";
        let mut edit = edits(&p, prose);
        edit.effects_text = serde_json::to_string(&PassageEffects {
            changes: vec![proposal(link.clone(), prose)],
            ..Default::default()
        })
        .unwrap();
        let draft = id();
        p.save_passage(&draft, 0, edit).unwrap();
        let error = p.validate_passage(&draft, 1, &settings()).unwrap_err();
        assert!(
            error.contains("outside the supplied scene") && error.contains("Lena"),
            "{error}"
        );
        let forged = p.approve_passage_for_test(&draft, &settings()).unwrap();
        assert!(!forged.can_accept);
        assert!(p
            .accept_passage(&draft, forged.version, &settings())
            .is_err());
        assert_eq!(p.view().unwrap().world_ref, canonical.world_ref);
        assert!(p.story_index().unwrap().passages.is_empty());

        let mut edit = forged.edits;
        edit.plan.reference_ids.push(lena.id.clone());
        p.save_passage(&draft, forged.version, edit).unwrap();
        let v = p.passage_view(&draft, &settings()).unwrap();
        p.validate_passage(&draft, v.version, &settings()).unwrap();
        let ready = p.approve_passage_for_test(&draft, &settings()).unwrap();
        assert!(ready.can_accept);
        assert_eq!(
            ready.effects_review.entries[0].displaced_links,
            vec![old.clone()]
        );

        let current = p.view().unwrap();
        let mut changed = current.world;
        changed
            .edges
            .iter_mut()
            .find(|e| e.id == old.id)
            .unwrap()
            .description = "Later author edit".into();
        p.commit(current.revision, id(), current.title, changed)
            .unwrap();
        let stale = p.passage_view(&draft, &settings()).unwrap();
        assert!(stale.stale && !stale.can_accept);
        assert!(!stale.effects_review.entries[0].approved);
        assert!(p
            .accept_passage(&draft, stale.version, &settings())
            .is_err());
        p.validate_passage(&draft, stale.version, &settings())
            .unwrap();
        let ready = p.approve_passage_for_test(&draft, &settings()).unwrap();
        assert_eq!(
            ready.effects_review.entries[0].displaced_links[0].description,
            "Later author edit"
        );
        p.accept_passage(&draft, ready.version, &settings())
            .unwrap();
        let accepted = p.read_passage(&draft).unwrap();
        assert!(accepted
            .before
            .edges
            .iter()
            .any(|e| e.id == old.id && e.description == "Later author edit"));
        assert!(!accepted.after.edges.iter().any(|e| e.id == old.id));
        assert!(accepted
            .after
            .edges
            .iter()
            .any(|e| e.type_id == link.type_id && e.target_id == link.target_id));
    }
}

#[test]
fn conflicting_single_target_cards_are_rejected_in_either_order_with_new_or_existing_ids() {
    for existing_id in [false, true] {
        for reverse in [false, true] {
            let (_dir, p) = fixture();
            let before = p.view().unwrap();
            let mut world = before.world;
            single_relation(&mut world);
            let old = WorldEdge {
                id: id(),
                type_id: "custom.single".into(),
                source_id: world.nodes[1].id.clone(),
                target_id: world.nodes[2].id.clone(),
                description: String::new(),
                properties: Default::default(),
            };
            world.edges.push(old.clone());
            p.commit(before.revision, id(), before.title, world.clone())
                .unwrap();
            let prose = "Mara waited beside Jon at the window.";
            let a = WorldEdge {
                id: if existing_id {
                    old.id.clone()
                } else {
                    "new:a".into()
                },
                ..old.clone()
            };
            let b = WorldEdge {
                id: "new:b".into(),
                target_id: world.nodes[0].id.clone(),
                ..old.clone()
            };
            let mut effects = PassageEffects {
                changes: vec![proposal(a, prose), proposal(b, prose)],
                ..Default::default()
            };
            if reverse {
                effects.changes.reverse();
            }
            if !existing_id {
                effects.changes.push(PassageEffect::RemoveLink {
                    id: old.id.clone(),
                    evidence: prose.into(),
                });
            }
            let mut edit = edits(&p, prose);
            edit.effects_text = serde_json::to_string(&effects).unwrap();
            let draft = id();
            p.save_passage(&draft, 0, edit).unwrap();
            assert!(p
                .validate_passage(&draft, 1, &settings())
                .unwrap_err()
                .contains("single-target"));
            let view = p.approve_passage_for_test(&draft, &settings()).unwrap();
            assert!(view.effects_review.entries[..2]
                .iter()
                .all(|e| e.issues.iter().any(|s| s.contains("single-target"))));
            assert!(!view.can_accept);
            assert!(p.accept_passage(&draft, view.version, &settings()).is_err());
            assert_eq!(p.view().unwrap().world, world);
        }
    }
}

#[test]
fn a_single_replacement_can_include_explicit_removal_and_an_independent_relationship() {
    let (_dir, p) = fixture();
    let before = p.view().unwrap();
    let mut world = before.world;
    single_relation(&mut world);
    let old = WorldEdge {
        id: id(),
        type_id: "custom.single".into(),
        source_id: world.nodes[1].id.clone(),
        target_id: world.nodes[2].id.clone(),
        description: String::new(),
        properties: Default::default(),
    };
    world.edges.push(old.clone());
    p.commit(before.revision, id(), before.title, world.clone())
        .unwrap();
    let prose = "Mara waited beside Jon at the window.";
    let next = WorldEdge {
        id: "new:next".into(),
        target_id: world.nodes[0].id.clone(),
        ..old.clone()
    };
    let other = WorldEdge {
        id: "new:other".into(),
        type_id: "core.related".into(),
        ..old.clone()
    };
    let mut edit = edits(&p, prose);
    edit.effects_text = serde_json::to_string(&PassageEffects {
        changes: vec![
            proposal(next, prose),
            PassageEffect::RemoveLink {
                id: old.id.clone(),
                evidence: prose.into(),
            },
            proposal(other, prose),
        ],
        ..Default::default()
    })
    .unwrap();
    let draft = accept(&p, edit);
    let reading = p.read_passage(&draft).unwrap();
    assert!(!reading.after.edges.iter().any(|e| e.id == old.id));
    assert!(reading
        .after
        .edges
        .iter()
        .any(|e| e.type_id == "custom.single" && e.target_id == world.nodes[0].id));
    assert!(reading
        .after
        .edges
        .iter()
        .any(|e| e.type_id == "core.related" && e.target_id == world.nodes[2].id));
}

#[test]
fn new_off_scene_single_target_links_stale_approval_without_expanding_model_context() {
    let (_dir, p) = fixture();
    let before = p.view().unwrap();
    let mut world = before.world;
    single_relation(&mut world);
    let lena = node(&world, "core.character", "Lena");
    world.nodes.push(lena.clone());
    p.commit(before.revision, id(), before.title, world.clone())
        .unwrap();
    let prose = "Mara waited beside Jon at the window.";
    let next = WorldEdge {
        id: "new:next".into(),
        type_id: "custom.single".into(),
        source_id: world.nodes[1].id.clone(),
        target_id: world.nodes[2].id.clone(),
        description: String::new(),
        properties: Default::default(),
    };
    let mut edit = edits(&p, prose);
    edit.effects_text = serde_json::to_string(&PassageEffects {
        changes: vec![proposal(next.clone(), prose)],
        ..Default::default()
    })
    .unwrap();
    let draft = ready(&p, edit);
    // An unrelated source remains outside this proposal's dependencies.
    let current = p.view().unwrap();
    let mut world = current.world;
    world.edges.push(WorldEdge {
        id: id(),
        source_id: lena.id.clone(),
        target_id: world.nodes[0].id.clone(),
        ..next.clone()
    });
    p.commit(current.revision, id(), current.title, world)
        .unwrap();
    assert!(p.passage_view(&draft, &settings()).unwrap().can_accept);
    let current = p.view().unwrap();
    let mut world = current.world;
    world.edges.push(WorldEdge {
        id: id(),
        target_id: lena.id.clone(),
        ..next
    });
    p.commit(current.revision, id(), current.title, world.clone())
        .unwrap();
    let view = p.passage_view(&draft, &settings()).unwrap();
    assert!(view.stale && !view.can_accept);
    assert!(!view.effects_review.entries[0].approved);
    assert!(p.accept_passage(&draft, view.version, &settings()).is_err());
    assert!(p
        .validate_passage(&draft, view.version, &settings())
        .unwrap_err()
        .contains("outside the supplied scene"));
    let req_id = id();
    p.save_passage(&req_id, 0, edits(&p, "")).unwrap();
    let request = p
        .start_passage(&req_id, 1, p.view().unwrap().revision, false, &settings())
        .unwrap();
    assert!(!request.user.contains("Lena") && !request.user.contains(&lena.id));
    assert_eq!(p.view().unwrap().world, world);
}
