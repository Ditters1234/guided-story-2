use super::*;
#[test]
fn typed_wearing_details_require_approval_and_publish_with_immutable_snapshots_and_forks() {
    let (dir, p) = fixture();
    let before = p.view().unwrap();
    let mut link = before
        .world
        .edges
        .iter()
        .find(|e| e.type_id == "core.worn-by")
        .unwrap()
        .clone();
    link.properties.insert(
        "configuration".into(),
        PropertyValue::Text("Unbuttoned, sleeves rolled".into()),
    );
    let mut edit = edits(&p, "Mara unbuttoned her coat and rolled up the sleeves.");
    edit.effects_text = serde_json::to_string(&PassageEffects {
        changes: vec![PassageEffect::PutLink {
            link: link.clone(),
            evidence: edit.prose.clone(),
        }],
        ..Default::default()
    })
    .unwrap();
    let draft = id();
    p.save_passage(&draft, 0, edit).unwrap();
    p.validate_passage(&draft, 1, &settings()).unwrap();
    let unapproved = p.passage_view(&draft, &settings()).unwrap();
    assert!(!unapproved.can_accept);
    assert_eq!(p.view().unwrap().world, before.world);
    let approved = p.approve_passage_for_test(&draft, &settings()).unwrap();
    assert!(approved.can_accept);
    p.accept_passage(&draft, approved.version, &settings())
        .unwrap();
    let reading = p.read_passage(&draft).unwrap();
    assert_eq!(reading.before, before.world);
    assert_eq!(
        reading.after.edges.iter().find(|e| e.id == link.id),
        Some(&link)
    );
    let current = p.view().unwrap();
    let mut changed = current.world;
    changed
        .edges
        .iter_mut()
        .find(|e| e.id == link.id)
        .unwrap()
        .properties
        .insert(
            "configuration".into(),
            PropertyValue::Text("Buttoned".into()),
        );
    p.commit(current.revision, id(), current.title, changed)
        .unwrap();
    assert_eq!(p.read_passage(&draft).unwrap().after, reading.after);
    let fork = p
        .fork_after_passage(&draft, dir.path(), "Forked wearing state".into())
        .unwrap();
    assert_eq!(fork.view().unwrap().world, reading.after);
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    assert_eq!(p.read_passage(&draft).unwrap().after, reading.after);
}
#[test]
fn proposed_link_property_dependencies_invalidate_approval_and_show_per_card_errors() {
    let (_dir, p) = fixture();
    let current = p.view().unwrap();
    let mut world = current.world;
    world
        .relation_types
        .iter_mut()
        .find(|r| r.id == "core.related")
        .unwrap()
        .fields
        .push(FieldDefinition {
            key: "witness".into(),
            label: "Witness".into(),
            kind: FieldKind::NodeRef,
            required: false,
            default_value: PropertyValue::Null,
            metadata: None,
        });
    p.commit(current.revision, id(), current.title, world.clone())
        .unwrap();
    let mut introduced = node(&world, "core.character", "Lena");
    introduced.id = "new:witness".into();
    let link = WorldEdge {
        id: "new:relation".into(),
        type_id: "core.related".into(),
        source_id: world.nodes[1].id.clone(),
        target_id: world.nodes[2].id.clone(),
        description: String::new(),
        properties: BTreeMap::from([(
            "witness".into(),
            PropertyValue::Text(introduced.id.clone()),
        )]),
    };
    let mut effects = PassageEffects {
        changes: vec![
            PassageEffect::PutNode {
                node: introduced,
                evidence: "Lena watched Mara and Jon.".into(),
            },
            PassageEffect::PutLink {
                link,
                evidence: "Lena watched Mara and Jon.".into(),
            },
        ],
        ..Default::default()
    };
    let mut edit = edits(&p, "Lena watched Mara and Jon.");
    edit.effects_text = serde_json::to_string(&effects).unwrap();
    let draft = id();
    p.save_passage(&draft, 0, edit).unwrap();
    p.validate_passage(&draft, 1, &settings()).unwrap();
    let approved = p.approve_passage_for_test(&draft, &settings()).unwrap();
    assert!(approved.can_accept);
    if let PassageEffect::PutNode { node, .. } = &mut effects.changes[0] {
        node.description = "A different background".into();
    }
    let mut edit = approved.edits;
    edit.effects_text = serde_json::to_string(&effects).unwrap();
    p.save_passage(&draft, approved.version, edit).unwrap();
    let changed = p.passage_view(&draft, &settings()).unwrap();
    assert!(!changed.effects_review.entries[1].approved);
    if let PassageEffect::PutLink { link, .. } = &mut effects.changes[1] {
        link.properties
            .insert("unknown".into(), PropertyValue::Boolean(true));
    }
    let mut edit = changed.edits;
    edit.effects_text = serde_json::to_string(&effects).unwrap();
    p.save_passage(&draft, changed.version, edit).unwrap();
    let bad = p.passage_view(&draft, &settings()).unwrap();
    assert!(bad.effects_review.entries[1]
        .issues
        .iter()
        .any(|s| s.contains("Unknown relationship field")));
    if let PassageEffect::PutLink { link, .. } = &mut effects.changes[1] {
        link.properties.remove("unknown");
    }
    let mut edit = bad.edits;
    edit.effects_text = serde_json::to_string(&effects).unwrap();
    p.save_passage(&draft, bad.version, edit).unwrap();
    let view = p.passage_view(&draft, &settings()).unwrap();
    p.validate_passage(&draft, view.version, &settings())
        .unwrap();
    let ready = p.approve_passage_for_test(&draft, &settings()).unwrap();
    p.accept_passage(&draft, ready.version, &settings())
        .unwrap();
    let world = p.view().unwrap().world;
    let witness = world.nodes.iter().find(|n| n.name == "Lena").unwrap();
    assert!(world
        .edges
        .iter()
        .any(|e| e.properties.get("witness") == Some(&PropertyValue::Text(witness.id.clone()))));
}
