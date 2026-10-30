use super::*;

fn foundation() -> AuthoringRequest {
    AuthoringRequest {
        creation_scope: Some(CreationScope::World),
        ..group_request()
    }
}

fn custom_world() -> World {
    let mut world = World::default();
    let field = |key: &str, kind, required, default_value, metadata| FieldDefinition {
        key: key.into(),
        label: key.into(),
        kind,
        required,
        default_value,
        metadata,
    };
    world.component_types.push(ComponentDefinition {
        id: "custom.budget".into(),
        name: "Foundation detail".into(),
        description: String::new(),
        fields: vec![
            field(
                "requiredText",
                FieldKind::Text,
                true,
                PropertyValue::Text("Required starting detail".into()),
                None,
            ),
            field(
                "zero",
                FieldKind::Number,
                false,
                PropertyValue::Number(0.0),
                None,
            ),
            field(
                "false",
                FieldKind::Boolean,
                false,
                PropertyValue::Boolean(false),
                None,
            ),
            field(
                "minimum",
                FieldKind::LongText,
                false,
                PropertyValue::Null,
                None,
            ),
            field("fixed", FieldKind::Text, false, PropertyValue::Null, None),
            field("pattern", FieldKind::Text, false, PropertyValue::Null, None),
            field(
                "visual",
                FieldKind::LongText,
                false,
                PropertyValue::Null,
                Some(FieldMetadata {
                    uses: vec![FieldUse::Visual],
                    ..Default::default()
                }),
            ),
            field(
                "instructions",
                FieldKind::LongText,
                false,
                PropertyValue::Null,
                Some(FieldMetadata {
                    uses: vec![FieldUse::Planning],
                    ..Default::default()
                }),
            ),
            field(
                "choice",
                FieldKind::Choice,
                false,
                PropertyValue::Null,
                Some(FieldMetadata {
                    choices: vec!["x".repeat(180), "short".into()],
                    ..Default::default()
                }),
            ),
        ],
    });
    world.node_types.push(NodeTypeDefinition {
        id: "custom.foundation".into(),
        name: "Custom foundation".into(),
        description: String::new(),
        components: vec!["custom.budget".into()],
    });
    world
}

#[test]
fn foundation_budget_preserves_schema_fields_required_defaults_and_string_constraints() {
    let world = custom_world();
    let original_world = world.clone();
    let mut schema = output_schema(&world, &foundation());
    let component = &mut schema["properties"]["nodes"]["properties"]["custom.foundation"]["items"]
        ["properties"]["components"]["properties"]["custom.budget"];
    component["properties"]["minimum"]["anyOf"][0]["minLength"] = json!(180);
    component["properties"]["fixed"]["anyOf"][0]["const"] = json!("f".repeat(200));
    component["properties"]["pattern"]["anyOf"][0]["pattern"] = json!("^.{220}$");
    let before = component.clone();
    creation::limit_foundation_fields(&mut schema, &world, &foundation(), 4096);
    let node = &schema["properties"]["nodes"]["properties"]["custom.foundation"]["items"];
    assert_eq!(node["properties"]["name"]["maxLength"], 120);
    assert_eq!(node["properties"]["description"]["maxLength"], 160);
    assert_eq!(node["properties"]["visualDescription"]["maxLength"], 160);
    let component = &node["properties"]["components"]["properties"]["custom.budget"];
    assert_eq!(component["maxProperties"], 4); // required + zero + false + one extra
    assert_eq!(component["required"], before["required"]);
    assert_eq!(component["properties"].as_object().unwrap().len(), 9);
    assert_eq!(component["properties"]["requiredText"]["maxLength"], 120);
    assert_eq!(
        component["properties"]["visual"]["anyOf"][0]["maxLength"],
        120
    );
    assert_eq!(
        component["properties"]["minimum"]["anyOf"][0]["minLength"],
        180
    );
    assert_eq!(
        component["properties"]["minimum"]["anyOf"][0]["maxLength"],
        180
    );
    assert_eq!(
        component["properties"]["fixed"]["anyOf"][0]["maxLength"],
        200
    );
    assert_eq!(
        component["properties"]["fixed"]["anyOf"][0]["const"],
        "f".repeat(200)
    );
    for key in ["zero", "false", "pattern", "instructions", "choice"] {
        assert_eq!(component["properties"][key], before["properties"][key]);
    }
    assert_eq!(world, original_world);
}

#[test]
fn narration_budget_keeps_person_tense_and_full_planning_instructions_available() {
    let world = World::default();
    let mut schema = output_schema(&world, &foundation());
    let before = schema["properties"]["nodes"]["properties"]["core.narration-style"]["items"]
        ["properties"]["components"]["properties"]["core.narration-style"]
        .clone();
    creation::limit_foundation_fields(&mut schema, &world, &foundation(), 4096);
    let style = &schema["properties"]["nodes"]["properties"]["core.narration-style"]["items"]
        ["properties"]["components"]["properties"]["core.narration-style"];
    assert_eq!(style["maxProperties"], 3);
    assert_eq!(style["properties"], before["properties"]);
    assert_eq!(style["required"], before["required"]);
}

#[test]
fn foundation_limits_exclude_revisions_typed_nodes_groups_and_larger_responses() {
    let mut world = World::default();
    let node = proposed_node(&world, "core.place", &id(), "Glasshouse");
    world.nodes.push(node.clone());
    let revision = AuthoringRequest {
        mode: AuthoringMode::Revise,
        type_id: None,
        target_ids: vec![node.id.clone()],
        fields: vec![EditableField {
            node_id: node.id,
            component_id: None,
            key: "description".into(),
        }],
        ..request()
    };
    let group = AuthoringRequest {
        creation_scope: Some(CreationScope::Group),
        ..group_request()
    };
    let typed = AuthoringRequest {
        bundle: true,
        ..request()
    };
    for (request, tokens) in [
        (revision, 4096),
        (request(), 4096),
        (typed, 4096),
        (group, 4096),
        (foundation(), 4097),
    ] {
        let mut schema = output_schema(&world, &request);
        let original = schema.clone();
        creation::limit_foundation_fields(&mut schema, &world, &request, tokens);
        assert_eq!(schema, original);
    }
    // Unspecified legacy world scope is still a foundation, not a group.
    let mut schema = output_schema(&world, &group_request());
    creation::limit_foundation_fields(&mut schema, &world, &group_request(), 4096);
    assert_eq!(
        schema["properties"]["nodes"]["properties"]["core.place"]["items"]["properties"]
            ["description"]["maxLength"],
        160
    );
}

#[test]
fn manual_foundation_candidates_keep_full_native_lengths_and_all_optional_fields() {
    let (_dir, project) = create();
    let world = custom_world();
    project
        .commit(0, id(), "Custom foundation".into(), world.clone())
        .unwrap();
    let request = AuthoringRequest {
        creation_type_ids: vec!["custom.foundation".into()],
        ..foundation()
    };
    let mut node = proposed_node(&world, "custom.foundation", "new:record", &"N".repeat(150));
    node.description = "d".repeat(300);
    node.visual_description = "v".repeat(300);
    let values = node.components.get_mut("custom.budget").unwrap();
    for key in [
        "requiredText",
        "minimum",
        "fixed",
        "pattern",
        "visual",
        "instructions",
    ] {
        values.insert(key.into(), PropertyValue::Text("a".repeat(500)));
    }
    values.insert("choice".into(), PropertyValue::Text("x".repeat(180)));
    let key = id();
    project.save_authoring_draft(&key, 0, request).unwrap();
    project.start_authoring(&key, 1, 1, &settings()).unwrap();
    project
        .finish_authoring(
            &key,
            Ok(serde_json::to_string(&NodeCandidate {
                nodes: vec![node.clone()],
                edges: vec![],
            })
            .unwrap()),
            false,
            false,
        )
        .unwrap();
    let review = project.authoring_view(&key, &settings()).unwrap();
    assert!(review.error.is_empty(), "{}", review.error);
    let applied = project
        .apply_authoring(&key, review.version, &settings())
        .unwrap();
    assert_eq!(applied.commit.project.world.nodes[0].name, node.name);
    assert_eq!(
        applied.commit.project.world.nodes[0].description,
        node.description
    );
    assert_eq!(
        applied.commit.project.world.nodes[0].visual_description,
        node.visual_description
    );
    assert_eq!(
        applied.commit.project.world.nodes[0].components,
        node.components
    );
}
