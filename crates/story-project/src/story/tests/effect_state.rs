use super::*;

#[test]
fn omitted_populated_facts_block_publication_and_explicit_reviewed_clearing_survives_restart() {
    let (dir, p) = fixture();
    let mut current = p.view().unwrap();
    current.world.nodes[1]
        .components
        .get_mut("detail.intent")
        .unwrap()
        .insert(
            "objective".into(),
            PropertyValue::Text("Repair the pump".into()),
        );
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let before = p.view().unwrap();
    let prose = "Mara cut her hand on the glass and abandoned her immediate objective of repairing the pump.";
    let mut mara = before.world.nodes[1].clone();
    mara.components
        .get_mut("detail.intent")
        .unwrap()
        .remove("objective");
    mara.components
        .get_mut("detail.condition")
        .unwrap()
        .insert("condition".into(), PropertyValue::Text("Cut hand".into()));
    let mut effects = PassageEffects {
        changes: vec![PassageEffect::PutNode {
            node: mara,
            evidence: prose.into(),
        }],
        ..Default::default()
    };
    let raw = serde_json::to_string(&effects).unwrap();
    let key = id();
    p.save_passage(&key, 0, edits(&p, prose)).unwrap();
    p.start_passage(&key, 1, before.revision, true, &settings())
        .unwrap();
    p.finish_passage(&key, true, Ok(raw.clone()), false, false, &settings())
        .unwrap();
    let expanded = p.passage_view(&key, &settings()).unwrap();
    let full: PassageEffects = serde_json::from_str(&expanded.edits.effects_text).unwrap();
    let PassageEffect::PutNode { node, .. } = &full.changes[0] else {
        panic!()
    };
    assert_eq!(
        node.components["detail.intent"]["objective"],
        before.world.nodes[1].components["detail.intent"]["objective"]
    );
    // Provider omissions preserve the captured baseline; manual full-record
    // omissions remain errors rather than an implicit clearing operation.
    let mut manual = expanded.edits;
    manual.effects_text = raw.clone();
    p.save_passage(&key, expanded.version, manual).unwrap();
    assert!(p
        .validate_passage(&key, expanded.version + 1, &settings())
        .is_err());
    let blocked = p.approve_passage_for_test(&key, &settings()).unwrap();
    assert!(!blocked.can_accept);
    assert!(blocked.effects_review.entries[0]
        .issues
        .iter()
        .any(|e| e.contains("Immediate objective was omitted")));
    assert_eq!(blocked.original_effects, raw);
    assert_eq!(blocked.edits.prose, prose);
    assert!(p
        .accept_passage(&key, blocked.version, &settings())
        .is_err());
    assert_eq!(p.view().unwrap().world, before.world);
    if let PassageEffect::PutNode { node, .. } = &mut effects.changes[0] {
        node.components
            .get_mut("detail.intent")
            .unwrap()
            .insert("objective".into(), PropertyValue::Null);
    }
    let mut edit = blocked.edits;
    edit.effects_text = serde_json::to_string(&effects).unwrap();
    p.save_passage(&key, blocked.version, edit).unwrap();
    p.validate_passage(&key, blocked.version + 1, &settings())
        .unwrap();
    let ready = p.approve_passage_for_test(&key, &settings()).unwrap();
    assert!(ready.can_accept, "{}", ready.error);
    p.accept_passage(&key, ready.version, &settings()).unwrap();
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    let reading = p.read_passage(&key).unwrap();
    assert_eq!(reading.before, before.world);
    assert_eq!(
        reading.after.nodes[1].components["detail.intent"]["objective"],
        PropertyValue::Null
    );
    assert_eq!(
        reading.after.nodes[1].components["detail.condition"]["condition"],
        PropertyValue::Text("Cut hand".into())
    );
    assert!(
        p.accept_passage(&key, 0, &settings())
            .unwrap()
            .commit
            .replayed
    );
}

#[test]
fn node_and_link_omission_guards_preserve_false_zero_and_typed_references() {
    let (_dir, p) = fixture();
    let mut w = p.view().unwrap().world;
    let fields: Vec<_> = [
        ("flag", FieldKind::Boolean),
        ("count", FieldKind::Number),
        ("people", FieldKind::NodeRefs),
    ]
    .into_iter()
    .map(|(key, kind)| FieldDefinition {
        key: key.into(),
        label: key.into(),
        kind,
        required: false,
        default_value: PropertyValue::Null,
        metadata: None,
    })
    .collect();
    w.component_types.push(ComponentDefinition {
        id: "custom.state".into(),
        name: "State".into(),
        description: String::new(),
        fields: fields.clone(),
    });
    let values = BTreeMap::from([
        ("flag".into(), PropertyValue::Boolean(false)),
        ("count".into(), PropertyValue::Number(0.0)),
        (
            "people".into(),
            PropertyValue::NodeRefs(vec![w.nodes[2].id.clone()]),
        ),
    ]);
    w.nodes[1]
        .components
        .insert("custom.state".into(), values.clone());
    let link = WorldEdge {
        id: id(),
        type_id: "custom.connection".into(),
        source_id: w.nodes[1].id.clone(),
        target_id: w.nodes[2].id.clone(),
        description: String::new(),
        properties: values,
    };
    w.relation_types.push(RelationDefinition {
        id: link.type_id.clone(),
        name: "Connection".into(),
        label: "connected".into(),
        source_capability: None,
        target_capability: None,
        physical_placement: false,
        single_target: false,
        description: String::new(),
        fields,
    });
    w.edges.push(link.clone());
    w.validate().unwrap();
    let prose = "Mara and Jon reviewed their situation together.";
    for key in ["flag", "count", "people"] {
        let mut person = w.nodes[1].clone();
        person
            .components
            .get_mut("custom.state")
            .unwrap()
            .remove(key);
        let mut changed_link = link.clone();
        changed_link.properties.remove(key);
        let effects = PassageEffects {
            changes: vec![
                PassageEffect::PutNode {
                    node: person,
                    evidence: prose.into(),
                },
                PassageEffect::PutLink {
                    link: changed_link,
                    evidence: prose.into(),
                },
            ],
            ..Default::default()
        };
        let issues = effect_record_issues(&w, &w, &effects);
        assert!(issues.iter().all(|rows| rows
            .iter()
            .any(|s| s.contains(&format!("{key} was omitted")))));
        assert!(preview_passage_effects(&w, &w, prose, &effects, &mut BTreeMap::new()).is_err());
    }
}

#[test]
fn selected_nested_items_and_threads_support_reviewed_multi_record_state_updates() {
    let (dir, p) = fixture();
    let mut current = p.view().unwrap();
    let w = &mut current.world;
    w.nodes[1].description = "A careful gardener.".into();
    w.nodes[1]
        .components
        .get_mut("detail.intent")
        .unwrap()
        .insert(
            "objective".into(),
            PropertyValue::Text("Find the remedy".into()),
        );
    let mut satchel = node(w, "core.item", "Satchel");
    satchel
        .components
        .insert("core.container".into(), BTreeMap::new());
    let mut remedy = node(w, "core.item", "Remedy");
    remedy
        .components
        .get_mut("core.item")
        .unwrap()
        .insert("quantity".into(), PropertyValue::Number(2.0));
    let thread = node(w, "core.thread", "Find the remedy");
    let remote = node(w, "core.place", "Distant harbor");
    for (source, target) in [(&satchel.id, &w.nodes[1].id), (&remedy.id, &satchel.id)] {
        w.edges.push(WorldEdge {
            id: id(),
            type_id: "core.stored-in".into(),
            source_id: source.clone(),
            target_id: target.clone(),
            description: String::new(),
            properties: BTreeMap::new(),
        });
    }
    let trust = WorldEdge {
        id: id(),
        type_id: "custom.trust".into(),
        source_id: w.nodes[1].id.clone(),
        target_id: w.nodes[2].id.clone(),
        description: String::new(),
        properties: BTreeMap::from([
            ("assessment".into(), PropertyValue::Text("Wary".into())),
            (
                "reason".into(),
                PropertyValue::Text("They worked together before.".into()),
            ),
        ]),
    };
    w.relation_types.push(RelationDefinition {
        id: trust.type_id.clone(),
        name: "Trust".into(),
        label: "trusts".into(),
        source_capability: None,
        target_capability: None,
        physical_placement: false,
        single_target: false,
        description: String::new(),
        fields: ["assessment", "reason"]
            .into_iter()
            .map(|key| FieldDefinition {
                key: key.into(),
                label: key.into(),
                kind: FieldKind::LongText,
                required: false,
                default_value: PropertyValue::Null,
                metadata: None,
            })
            .collect(),
    });
    w.edges.push(trust.clone());
    w.nodes
        .extend([satchel, remedy.clone(), thread.clone(), remote.clone()]);
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let before = p.view().unwrap();
    let sentences = [
        "Mara handed her field coat to Jon, who took it.",
        "Mara drank one remedy from her satchel.",
        "Glass cut Mara's hand; her immediate aim was now to reach the clinic.",
        "Mara now trusted Jon with her safety.",
        "The search for the remedy was resolved.",
    ];
    let prose = sentences.join(" ");
    let key = id();
    let mut edit = edits(&p, &prose);
    let without = scene_world(&before.world, &edit.plan).unwrap();
    assert!(!without
        .nodes
        .iter()
        .any(|n| n.id == remedy.id || n.id == thread.id));
    edit.plan.reference_ids = vec![remedy.id.clone(), thread.id.clone()];
    p.save_passage(&key, 0, edit).unwrap();
    let request = p
        .start_passage(&key, 1, before.revision, true, &settings())
        .unwrap();
    let input: Value = serde_json::from_str(&request.user).unwrap();
    for selected in [&remedy, &thread] {
        assert!(input["records"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["id"] == selected.id));
    }
    assert!(!input["records"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["id"] == remote.id));
    assert!(request.system.contains("current emotion/objective"));
    let mut mara = before.world.nodes[1].clone();
    mara.components
        .get_mut("detail.condition")
        .unwrap()
        .insert("condition".into(), PropertyValue::Text("Cut hand".into()));
    mara.components.get_mut("detail.intent").unwrap().insert(
        "objective".into(),
        PropertyValue::Text("Reach the clinic".into()),
    );
    remedy
        .components
        .get_mut("core.item")
        .unwrap()
        .insert("quantity".into(), PropertyValue::Number(1.0));
    let mut resolved = thread.clone();
    resolved
        .components
        .get_mut("detail.thread")
        .unwrap()
        .insert("status".into(), PropertyValue::Text("resolved".into()));
    let coat = before
        .world
        .edges
        .iter()
        .find(|e| e.type_id == "core.worn-by")
        .unwrap();
    let mut changed_trust = trust.clone();
    changed_trust.properties.insert(
        "assessment".into(),
        PropertyValue::Text("Trusts with her safety".into()),
    );
    let raw = json!({"changes":{"putNodes":[
        {"kind":"putNode","node":mara,"evidence":sentences[2]},
        {"kind":"putNode","node":remedy,"evidence":sentences[1]},
        {"kind":"putNode","node":resolved,"evidence":sentences[4]}
    ],"putLinks":[
        {"kind":"putLink","link":WorldEdge { type_id:"core.stored-in".into(), target_id:before.world.nodes[2].id.clone(), properties:BTreeMap::new(), ..coat.clone() },"evidence":sentences[0]},
        {"kind":"putLink","link":changed_trust,"evidence":sentences[3]}
    ],"removeLinks":[]},"notes":[],"suggestChapterEnd":false}).to_string();
    p.finish_passage(&key, true, Ok(raw), false, false, &settings())
        .unwrap();
    let checked = p.passage_view(&key, &settings()).unwrap();
    assert!(checked.effects_review.checked, "{}", checked.error);
    assert_eq!(checked.effects_review.entries.len(), 5);
    assert!(!checked.can_accept);
    assert_eq!(p.view().unwrap().world, before.world);
    let approved = p.approve_passage_for_test(&key, &settings()).unwrap();
    p.accept_passage(&key, approved.version, &settings())
        .unwrap();
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    let reading = p.read_passage(&key).unwrap();
    assert_eq!(reading.before, before.world);
    assert_eq!(reading.passage.prose, prose);
    assert_eq!(reading.after.nodes[1].description, "A careful gardener.");
    assert_eq!(
        reading.after.nodes[1].components["detail.condition"]["condition"],
        PropertyValue::Text("Cut hand".into())
    );
    assert_eq!(
        reading.after.nodes[1].components["detail.intent"]["objective"],
        PropertyValue::Text("Reach the clinic".into())
    );
    assert_eq!(
        reading
            .after
            .nodes
            .iter()
            .find(|n| n.id == remedy.id)
            .unwrap()
            .components["core.item"]["quantity"],
        PropertyValue::Number(1.0)
    );
    assert_eq!(
        reading
            .after
            .nodes
            .iter()
            .find(|n| n.id == thread.id)
            .unwrap()
            .components["detail.thread"]["status"],
        PropertyValue::Text("resolved".into())
    );
    assert_eq!(
        reading
            .after
            .edges
            .iter()
            .find(|e| e.id == trust.id)
            .unwrap()
            .properties["reason"],
        trust.properties["reason"]
    );
    assert!(reading.after.edges.iter().any(|e| e.id == coat.id
        && e.type_id == "core.stored-in"
        && e.target_id == before.world.nodes[2].id));
    assert_eq!(
        reading.after.nodes.iter().find(|n| n.id == remote.id),
        Some(&remote)
    );
}
