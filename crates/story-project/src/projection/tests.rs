use super::*;

fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn node(world: &World, preset: &str, name: &str) -> WorldNode {
    let preset = world.node_types.iter().find(|d| d.id == preset).unwrap();
    WorldNode {
        id: id(),
        type_id: preset.id.clone(),
        name: name.into(),
        description: String::new(),
        visual_description: String::new(),
        components: preset
            .components
            .iter()
            .map(|id| {
                (
                    id.clone(),
                    default_properties(world.component_types.iter().find(|d| &d.id == id).unwrap()),
                )
            })
            .collect(),
    }
}
fn field(key: &str, kind: FieldKind) -> FieldDefinition {
    FieldDefinition {
        key: key.into(),
        label: key.into(),
        kind,
        required: false,
        default_value: PropertyValue::Null,
        metadata: None,
    }
}

#[test]
fn useful_presets_are_sparse_and_native_contracts_do_not_depend_on_the_catalogue() {
    let mut world = World::default();
    let person = node(&world, "core.character", "Nora");
    for id in [
        "detail.profile",
        "detail.appearance",
        "detail.personality",
        "detail.motivation",
        "detail.voice",
        "detail.intent",
    ] {
        assert!(person.components[id].is_empty());
    }
    let item = node(&world, "core.item", "Pump handle");
    assert_eq!(
        item.components["core.item"]["quantity"],
        PropertyValue::Number(1.0)
    );
    world.nodes = vec![person, item];
    world.validate().unwrap();
    let mut minimal: World =
        serde_json::from_str(include_str!("../../../../schema/world-core.json")).unwrap();
    minimal.validate().unwrap();
    minimal
        .component_types
        .iter_mut()
        .find(|d| d.id == "core.item")
        .unwrap()
        .fields[0]
        .required = false;
    assert!(minimal.validate().unwrap_err().contains("built-in"));
}

#[test]
fn optional_zero_false_choice_and_capability_filtered_references_roundtrip() {
    let mut world = World::default();
    let person = node(&world, "core.character", "Nora");
    let place = node(&world, "core.place", "Workshop");
    let mut choice = field("status", FieldKind::Choice);
    choice.metadata = Some(FieldMetadata {
        choices: vec!["open".into(), "closed".into()],
        ..Default::default()
    });
    let mut reference = field("person", FieldKind::NodeRef);
    reference.metadata = Some(FieldMetadata {
        reference_capability: Some("core.character".into()),
        ..Default::default()
    });
    world.component_types.push(ComponentDefinition {
        id: "custom.state".into(),
        name: "State".into(),
        description: String::new(),
        fields: vec![
            field("count", FieldKind::Number),
            field("ready", FieldKind::Boolean),
            choice,
            reference,
        ],
    });
    world.nodes = vec![person, place];
    world.nodes[0].components.insert(
        "custom.state".into(),
        BTreeMap::from([
            ("count".into(), PropertyValue::Null),
            ("ready".into(), PropertyValue::Null),
        ]),
    );
    world.validate().unwrap();
    let person_id = world.nodes[0].id.clone();
    world.nodes[0]
        .components
        .get_mut("custom.state")
        .unwrap()
        .extend([
            ("count".into(), PropertyValue::Number(0.0)),
            ("ready".into(), PropertyValue::Boolean(false)),
            ("status".into(), PropertyValue::Text("open".into())),
            ("person".into(), PropertyValue::Text(person_id)),
        ]);
    world.validate().unwrap();
    assert_eq!(
        serde_json::from_slice::<World>(&serde_json::to_vec(&world).unwrap()).unwrap(),
        world
    );
    world.nodes[0]
        .components
        .get_mut("custom.state")
        .unwrap()
        .insert("status".into(), PropertyValue::Text("maybe".into()));
    assert!(world.validate().unwrap_err().contains("status"));
    let place_id = world.nodes[1].id.clone();
    world.nodes[0]
        .components
        .get_mut("custom.state")
        .unwrap()
        .insert("status".into(), PropertyValue::Null);
    world.nodes[0]
        .components
        .get_mut("custom.state")
        .unwrap()
        .insert("person".into(), PropertyValue::Text(place_id));
    assert!(world.validate().unwrap_err().contains("person"));
}

#[test]
fn attaching_and_retyping_preserve_values_identity_and_physical_dependencies() {
    let mut world = World::default();
    world.nodes.push(node(&world, "core.character", "Nora"));
    world.nodes.push(node(&world, "core.place", "Workshop"));
    let person = world.nodes[0].id.clone();
    world.nodes[0]
        .components
        .get_mut("detail.voice")
        .unwrap()
        .insert(
            "style".into(),
            PropertyValue::Text("Short practical answers".into()),
        );
    world.edges.push(WorldEdge {
        id: id(),
        type_id: "core.present-at".into(),
        source_id: person.clone(),
        target_id: world.nodes[1].id.clone(),
        description: String::new(),
    });
    let added = preview_world_edit(
        &world,
        WorldEdit::AddCapability {
            id: person.clone(),
            capability: "detail.anatomy".into(),
        },
    )
    .unwrap();
    assert!(!world.nodes[0].components.contains_key("detail.anatomy"));
    assert!(added.world.nodes[0].components["detail.anatomy"].is_empty());
    let revised = preview_world_edit(
        &added.world,
        WorldEdit::RetypeNode {
            id: person.clone(),
            type_id: "core.item".into(),
        },
    )
    .unwrap();
    assert_eq!(revised.world.nodes[0].id, person);
    assert_eq!(
        revised.world.nodes[0].components["detail.voice"],
        world.nodes[0].components["detail.voice"]
    );
    assert!(revised.world.nodes[0]
        .components
        .contains_key("core.character"));
    assert!(revised.world.nodes[0].components.contains_key("core.item"));
    assert_eq!(revised.world.edges, world.edges);
    assert!(preview_world_edit(
        &revised.world,
        WorldEdit::RemoveCapability {
            id: person.clone(),
            capability: "core.character".into()
        }
    )
    .is_err());
    assert!(preview_world_edit(
        &world,
        WorldEdit::AddCapability {
            id: person,
            capability: "core.wearable".into()
        }
    )
    .unwrap_err()
    .contains("inventory"));
}

#[test]
fn custom_visual_fields_and_garment_details_reach_actual_media_facts_and_staleness() {
    let mut world = World::default();
    let mut person = node(&world, "core.character", "Nora");
    let coat = node(&world, "core.wearable-item", "Work coat");
    person
        .components
        .get_mut("detail.appearance")
        .unwrap()
        .insert("hair".into(), PropertyValue::Text("Copper curls".into()));
    person.components.insert(
        "detail.anatomy".into(),
        BTreeMap::from([(
            "chest".into(),
            PropertyValue::Text("Covered regional detail".into()),
        )]),
    );
    let mut visual = field("glow", FieldKind::Text);
    visual.metadata = Some(FieldMetadata {
        uses: vec![FieldUse::Visual],
        ..Default::default()
    });
    world.component_types.push(ComponentDefinition {
        id: "custom.surface".into(),
        name: "Surface".into(),
        description: String::new(),
        fields: vec![visual],
    });
    person.components.insert(
        "custom.surface".into(),
        BTreeMap::from([(
            "glow".into(),
            PropertyValue::Text("Blue luminescence".into()),
        )]),
    );
    let person_id = person.id.clone();
    world.nodes = vec![person, coat];
    world.nodes[1]
        .components
        .get_mut("detail.design")
        .unwrap()
        .insert(
            "materials".into(),
            PropertyValue::Text("Waxed linen".into()),
        );
    world.nodes[1]
        .components
        .get_mut("detail.condition")
        .unwrap()
        .insert(
            "condition".into(),
            PropertyValue::Text("Rain soaked".into()),
        );
    world.edges.push(WorldEdge {
        id: id(),
        type_id: "core.worn-by".into(),
        source_id: world.nodes[1].id.clone(),
        target_id: person_id.clone(),
        description: String::new(),
    });
    let dir = tempfile::tempdir().unwrap();
    let p = Project::create(
        &dir.path().join("story"),
        "Fixture".into(),
        world.clone(),
        id(),
    )
    .unwrap();
    let media_id = id();
    let image = p
        .create_media(
            &media_id,
            MediaTarget {
                node_id: Some(person_id),
                passage_id: None,
                moment: ImageMoment::After,
                subject_ids: vec![],
            },
        )
        .unwrap();
    assert!(image.draft.source.facts[0]
        .appearance
        .contains("Copper curls"));
    assert!(image.draft.source.facts[0]
        .appearance
        .contains("Blue luminescence"));
    assert!(!image.draft.source.facts[0]
        .appearance
        .contains("Covered regional detail"));
    assert!(image.draft.request.prompt.contains("Waxed linen"));
    assert!(image.draft.request.prompt.contains("Rain soaked"));
    let before = p.view().unwrap();
    world.nodes[0]
        .components
        .get_mut("detail.voice")
        .unwrap()
        .insert(
            "style".into(),
            PropertyValue::Text("Story-only voice".into()),
        );
    p.commit(before.revision, id(), before.title.clone(), world.clone())
        .unwrap();
    assert!(!p.media_view(&media_id).unwrap().stale);
    world.nodes[0]
        .components
        .get_mut("custom.surface")
        .unwrap()
        .insert(
            "glow".into(),
            PropertyValue::Text("Green luminescence".into()),
        );
    p.commit(before.revision + 1, id(), before.title, world)
        .unwrap();
    assert!(p.media_view(&media_id).unwrap().stale);
    let original: World = p.read_record("worlds", &before.world_ref).unwrap();
    assert_eq!(
        original.nodes[0].components["custom.surface"]["glow"],
        PropertyValue::Text("Blue luminescence".into())
    );
}

#[test]
fn model_projection_labels_custom_fields_and_keeps_visual_and_planning_values_out() {
    let mut world = World::default();
    let mut person = node(&world, "core.character", "Nora");
    let mut custom = field("field0", FieldKind::Text);
    custom.label = "What matters".into();
    world.component_types.push(ComponentDefinition {
        id: "custom.motive".into(),
        name: "Motive".into(),
        description: String::new(),
        fields: vec![custom],
    });
    person.components.insert(
        "custom.motive".into(),
        BTreeMap::from([(
            "field0".into(),
            PropertyValue::Text("Keep the pump working".into()),
        )]),
    );
    person.components.insert(
        "detail.anatomy".into(),
        BTreeMap::from([(
            "chest".into(),
            PropertyValue::Text("Visual-only detail".into()),
        )]),
    );
    person.components.insert(
        "detail.thread".into(),
        BTreeMap::from([(
            "direction".into(),
            PropertyValue::Text("Possible later reveal".into()),
        )]),
    );
    let result = narrative_record(&world, &person).to_string();
    assert!(result.contains("What matters") && result.contains("Keep the pump working"));
    assert!(!result.contains("Visual-only detail") && !result.contains("Possible later reveal"));
    let visual = visual_summary(&world, &person, &[]);
    assert!(visual.contains("Visual-only detail"));
    assert!(!visual.contains("Keep the pump working"));
}

#[test]
fn older_project_head_is_rejected_without_replacing_its_files() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("story");
    let p = Project::create(&folder, "Fixture".into(), World::default(), id()).unwrap();
    assert_eq!(p.view().unwrap().schema_version, 2);
    drop(p);
    let path = folder.join("head.json");
    let mut head: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    head["schemaVersion"] = json!(1);
    let bytes = serde_json::to_vec(&head).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    assert!(Project::open(&folder)
        .err()
        .unwrap()
        .contains("new story folder"));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}
