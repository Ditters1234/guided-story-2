use super::*;

#[test]
fn group_relation_sources_come_from_new_nodes_while_existing_targets_and_container_holders_remain_available(
) {
    let (_dir, p) = create();
    let mut world = p.view().unwrap().world;
    world.component_types.push(ComponentDefinition {
        id: "custom.reference-only".into(),
        name: "Reference marker".into(),
        description: "A capability attached only to the supplied existing character.".into(),
        fields: vec![],
    });
    let mut person = proposed_node(&world, "core.character", &id(), "Nora Hale");
    person
        .components
        .insert("custom.reference-only".into(), BTreeMap::new());
    world.nodes = vec![person.clone()];
    for (id, source) in [
        ("custom.holder-source", "core.holder"),
        ("custom.reference-source", "custom.reference-only"),
    ] {
        world.relation_types.push(RelationDefinition {
            id: id.into(),
            name: id.into(),
            label: "supplies".into(),
            source_capability: Some(source.into()),
            target_capability: Some("core.character".into()),
            physical_placement: false,
            single_target: false,
            fields: vec![],
        });
    }
    p.commit(0, id(), "Fictional group scope".into(), world.clone())
        .unwrap();
    let request = AuthoringRequest {
        type_id: None,
        bundle: true,
        creation_scope: Some(CreationScope::Group),
        group_type_id: Some("core.item".into()),
        context_ids: vec![person.id.clone()],
        direction: "A supply crate carried by Nora Hale.".into(),
        ..request()
    };
    let prepared = prepare_authoring(&world, &request, &settings()).unwrap();
    let input: Value = serde_json::from_str(&prepared.user).unwrap();
    let relations = input["relationships"].as_array().unwrap();
    let includes = |id: &str| relations.iter().any(|r| r["id"] == id);
    assert!(includes("core.stored-in"));
    assert!(includes("core.worn-by"));
    assert!(includes("custom.holder-source"));
    assert!(!includes("core.present-at"));
    assert!(!includes("story.feels-toward"));
    assert!(!includes("custom.reference-source"));
    assert!(input["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["id"] == "custom.reference-only"));
    assert!(input["records"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["id"] == person.id));
    assert!(prepared
        .existing_context
        .as_ref()
        .unwrap()
        .catalog
        .iter()
        .any(|n| n.id == person.id));
    let edge_choices = prepared.response_schema["properties"]["edges"]["items"]["anyOf"]
        .as_array()
        .unwrap();
    assert!(!edge_choices
        .iter()
        .any(|s| s["properties"]["typeId"]["const"] == "core.present-at"));
    let mut crate_node = proposed_node(&world, "core.item", "new:crate", "Supply crate");
    crate_node
        .components
        .insert("core.container".into(), BTreeMap::new());
    assert!(!crate_node.components.contains_key("core.holder"));
    let edges = ["core.stored-in", "custom.holder-source"]
        .into_iter()
        .enumerate()
        .map(|(index, kind)| WorldEdge {
            id: format!("new:link-{index}"),
            type_id: kind.into(),
            source_id: crate_node.id.clone(),
            target_id: person.id.clone(),
            description: String::new(),
            properties: BTreeMap::new(),
        })
        .collect();
    let key = id();
    p.save_authoring_draft(&key, 0, request).unwrap();
    p.start_authoring(&key, 1, 1, &settings()).unwrap();
    p.finish_authoring(
        &key,
        Ok(serde_json::to_string(&NodeCandidate {
            nodes: vec![crate_node],
            edges,
        })
        .unwrap()),
        false,
        false,
    )
    .unwrap();
    let review = p.authoring_view(&key, &settings()).unwrap();
    assert!(review.error.is_empty(), "{}", review.error);
    assert_eq!(p.view().unwrap().world, world);
    let after = p
        .apply_authoring(&key, review.version, &settings())
        .unwrap()
        .commit
        .project
        .world;
    assert!(after.nodes.contains(&person));
    assert_eq!(after.nodes.len(), 2);
    assert_eq!(after.edges.len(), 2);
}

#[test]
fn creation_schema_limits_match_native_single_root_world_and_empty_group_rules() {
    let world = World::default();
    let single = request();
    let mut foundation = group_request();
    foundation.creation_scope = Some(CreationScope::World);
    let mut group = foundation.clone();
    group.creation_scope = Some(CreationScope::Group);
    for mode in [
        StructuredOutput::Schema,
        StructuredOutput::Json,
        StructuredOutput::Prompt,
    ] {
        let mut settings = settings();
        settings.structured_output = mode;
        for request in [&single, &foundation, &group] {
            let prepared = prepare_authoring(&world, request, &settings).unwrap();
            let user: Value = serde_json::from_str(&prepared.user).unwrap();
            let formal_nodes = &prepared.response_schema["properties"]["nodes"];
            let prompt_nodes = user
                .get("responseSchema")
                .map(|schema| &schema["properties"]["nodes"]);
            if request.bundle {
                for nodes in std::iter::once(formal_nodes).chain(prompt_nodes) {
                    assert_eq!(nodes["type"], "object");
                    assert_eq!(nodes["additionalProperties"], false);
                    assert_eq!(
                        nodes["required"].as_array().unwrap().len(),
                        nodes["properties"].as_object().unwrap().len()
                    );
                    let groups = nodes["properties"].as_object().unwrap();
                    assert_eq!(groups.len(), world.node_types.len());
                    for preset in &world.node_types {
                        let records = &groups[&preset.id];
                        assert_eq!(records["type"], "array");
                        assert_eq!(records["maxItems"], 20);
                        assert_eq!(records["items"]["properties"]["typeId"]["const"], preset.id);
                        assert!(records["items"].get("anyOf").is_none());
                    }
                }
            } else {
                assert_eq!(formal_nodes["type"], "array");
                assert_eq!(formal_nodes["minItems"], 1);
                assert_eq!(formal_nodes["maxItems"], 1);
                if let Some(prompt_nodes) = prompt_nodes {
                    assert_eq!(prompt_nodes["minItems"], 1);
                    assert_eq!(prompt_nodes["maxItems"], 1);
                    assert_eq!(
                        prompt_nodes["items"]["properties"]["id"]["const"],
                        "new:root"
                    );
                }
                assert_eq!(
                    formal_nodes["items"]["anyOf"][0]["properties"]["id"]["const"],
                    "new:root"
                );
                assert_eq!(user["responseBudget"]["suggestedMaxNewNodes"], 1);
            }
        }
    }
    // Empty keyed groups are a legitimate no-addition proposal. An empty world
    // still fails canonical creation validation; grouping never weakens that rule.
    for (request, rejects_empty) in [(foundation, true), (group, false)] {
        let (_dir, project) = create();
        let key = id();
        project.save_authoring_draft(&key, 0, request).unwrap();
        project.start_authoring(&key, 1, 0, &settings()).unwrap();
        project
            .finish_authoring(
                &key,
                Ok(json!({"nodes":{},"edges":[]}).to_string()),
                false,
                false,
            )
            .unwrap();
        let view = project.authoring_view(&key, &settings()).unwrap();
        assert_eq!(!view.error.is_empty(), rejects_empty, "{}", view.error);
        assert!(project
            .apply_authoring(&key, view.version, &settings())
            .is_err());
        assert_eq!(project.view().unwrap().revision, 0);
        assert!(project.view().unwrap().world.nodes.is_empty());
    }
}

fn fields() -> Vec<FieldDefinition> {
    let make = |key: &str, kind, required, default_value| FieldDefinition {
        key: key.into(),
        label: key.into(),
        kind,
        required,
        default_value,
        metadata: None,
    };
    let mut choice = make("status", FieldKind::Choice, false, PropertyValue::Null);
    choice.metadata = Some(FieldMetadata {
        choices: vec!["open".into(), "closed".into()],
        ..Default::default()
    });
    vec![
        make("generated", FieldKind::LongText, false, PropertyValue::Null),
        choice,
        make(
            "flag",
            FieldKind::Boolean,
            true,
            PropertyValue::Boolean(false),
        ),
        make("zero", FieldKind::Number, false, PropertyValue::Null),
        make("empty", FieldKind::Text, false, PropertyValue::Null),
        make("emptyList", FieldKind::NodeRefs, false, PropertyValue::Null),
        make("nil", FieldKind::Text, false, PropertyValue::Null),
        make("missing", FieldKind::Text, false, PropertyValue::Null),
        make("reference", FieldKind::NodeRef, false, PropertyValue::Null),
        make(
            "references",
            FieldKind::NodeRefs,
            false,
            PropertyValue::Null,
        ),
    ]
}
fn values(reference: &str) -> BTreeMap<String, PropertyValue> {
    BTreeMap::from([
        ("status".into(), PropertyValue::Text("open".into())),
        ("flag".into(), PropertyValue::Boolean(false)),
        ("zero".into(), PropertyValue::Number(0.0)),
        ("empty".into(), PropertyValue::Text(String::new())),
        ("emptyList".into(), PropertyValue::NodeRefs(vec![])),
        ("nil".into(), PropertyValue::Null),
        ("reference".into(), PropertyValue::Text(reference.into())),
        (
            "references".into(),
            PropertyValue::NodeRefs(vec![reference.into()]),
        ),
    ])
}
fn revision(target: &str, reference: &str) -> AuthoringRequest {
    AuthoringRequest {
        mode: AuthoringMode::Revise,
        type_id: None,
        bundle: false,
        direction: String::new(),
        target_ids: vec![target.into()],
        context_ids: vec![reference.into()],
        fields: ["generated", "status"]
            .into_iter()
            .map(|key| EditableField {
                node_id: target.into(),
                component_id: Some("custom.editor".into()),
                key: key.into(),
            })
            .collect(),
        ..request()
    }
}
fn assert_protection(schema: &Value, reference: &str) {
    let properties = &schema["properties"];
    assert!(properties["generated"].get("const").is_none());
    assert_eq!(
        properties["status"]["enum"],
        json!(["open", "closed", null])
    );
    for (key, expected) in [
        ("flag", json!(false)),
        ("zero", json!(0.0)),
        ("empty", json!("")),
        ("emptyList", json!([])),
        ("reference", json!(reference)),
        ("references", json!([reference])),
    ] {
        assert_eq!(properties[key]["const"], expected, "{key}");
        assert!(
            schema["required"].as_array().unwrap().contains(&json!(key)),
            "{key}"
        );
    }
    for key in ["nil", "missing"] {
        assert_eq!(properties[key], json!({"type":"null"}));
        assert!(!schema["required"].as_array().unwrap().contains(&json!(key)));
    }
    assert_eq!(schema["additionalProperties"], false);
}

#[test]
fn node_revision_schema_locks_all_unchecked_values_without_collapsing_unspecified_fields() {
    let (_dir, p) = create();
    let mut world = p.view().unwrap().world;
    world.component_types.push(ComponentDefinition {
        id: "custom.editor".into(),
        name: "Custom editor".into(),
        description: String::new(),
        fields: fields(),
    });
    world.node_types.push(NodeTypeDefinition {
        id: "custom.record".into(),
        name: "Custom record".into(),
        description: String::new(),
        components: vec!["custom.editor".into()],
    });
    let reference = proposed_node(&world, "core.place", &id(), "Workshop");
    let mut node = proposed_node(&world, "custom.record", &id(), "Brass lantern notes");
    node.description = "Keep these author notes.".into();
    node.visual_description = "Blackened brass.".into();
    node.components
        .insert("custom.editor".into(), values(&reference.id));
    world.nodes = vec![reference.clone(), node.clone()];
    p.commit(0, id(), "Fictional field protection".into(), world.clone())
        .unwrap();
    let request = revision(&node.id, &reference.id);
    let prepared = prepare_authoring(&world, &request, &settings()).unwrap();
    let schema = &prepared.response_schema["properties"]["nodes"]["items"]["anyOf"][0];
    for (key, expected) in [
        ("id", &node.id),
        ("name", &node.name),
        ("description", &node.description),
        ("visualDescription", &node.visual_description),
    ] {
        assert_eq!(schema["properties"][key]["const"], json!(expected));
    }
    assert_protection(
        &schema["properties"]["components"]["properties"]["custom.editor"],
        &reference.id,
    );
    assert_eq!(
        prepared.response_schema["properties"]["nodes"]["minItems"],
        1
    );
    assert_eq!(
        prepared.response_schema["properties"]["edges"]["maxItems"],
        0
    );
    let mut with_name = request.clone();
    with_name.fields.push(EditableField {
        node_id: node.id.clone(),
        component_id: None,
        key: "name".into(),
    });
    let editable_name = prepare_authoring(&world, &with_name, &settings()).unwrap();
    let editable_name =
        &editable_name.response_schema["properties"]["nodes"]["items"]["anyOf"][0]["properties"];
    assert!(editable_name["name"].get("const").is_none());
    assert_eq!(editable_name["description"]["const"], node.description);

    assert!(
        serde_json::from_str::<Value>(&prepared.user).unwrap()["responseBudget"]
            .get("foundationDetailAllowance")
            .is_none()
    );
    let key = id();
    p.save_authoring_draft(&key, 0, request).unwrap();
    p.start_authoring(&key, 1, 1, &settings()).unwrap();
    let mut candidate = node.clone();
    let properties = candidate.components.get_mut("custom.editor").unwrap();
    properties.insert(
        "generated".into(),
        PropertyValue::Text("A cracked lens scatters the beam into three uneven bands.".into()),
    );
    properties.insert("status".into(), PropertyValue::Text("closed".into()));
    properties.remove("nil");
    properties.insert("missing".into(), PropertyValue::Null);
    let response = serde_json::to_string(&NodeCandidate {
        nodes: vec![candidate.clone()],
        edges: vec![],
    })
    .unwrap();
    p.finish_authoring(&key, Ok(response.clone()), false, false)
        .unwrap();
    let review = p.authoring_view(&key, &settings()).unwrap();
    assert!(review.error.is_empty(), "{}", review.error);
    assert_eq!(p.view().unwrap().world, world);
    candidate
        .components
        .get_mut("custom.editor")
        .unwrap()
        .insert("zero".into(), PropertyValue::Number(1.0));
    p.save_authoring_candidate(
        &key,
        review.version,
        serde_json::to_string(&NodeCandidate {
            nodes: vec![candidate],
            edges: vec![],
        })
        .unwrap(),
    )
    .unwrap();
    let blocked = p.authoring_view(&key, &settings()).unwrap();
    assert!(blocked.error.contains("protected"));
    assert!(p
        .apply_authoring(&key, blocked.version, &settings())
        .is_err());
    p.save_authoring_candidate(&key, blocked.version, response)
        .unwrap();
    let ready = p.authoring_view(&key, &settings()).unwrap();
    let after = p
        .apply_authoring(&key, ready.version, &settings())
        .unwrap()
        .commit
        .project
        .world;
    let saved = after.nodes.iter().find(|n| n.id == node.id).unwrap();
    let mut expected = node.clone();
    expected
        .components
        .get_mut("custom.editor")
        .unwrap()
        .insert(
            "generated".into(),
            PropertyValue::Text("A cracked lens scatters the beam into three uneven bands.".into()),
        );
    expected
        .components
        .get_mut("custom.editor")
        .unwrap()
        .insert("status".into(), PropertyValue::Text("closed".into()));
    assert_eq!(saved, &expected);
    assert!(after.nodes.contains(&reference));
}

#[test]
fn typed_link_revision_schema_locks_identity_endpoints_notes_and_unchecked_properties() {
    let (_dir, p) = create();
    let mut world = p.view().unwrap().world;
    let a = proposed_node(&world, "core.place", &id(), "Workshop");
    let b = proposed_node(&world, "core.place", &id(), "Store room");
    world.relation_types.push(RelationDefinition {
        id: "custom.obligation".into(),
        name: "Obligation".into(),
        label: "owes maintenance to".into(),
        source_capability: None,
        target_capability: None,
        physical_placement: false,
        single_target: false,
        fields: fields(),
    });
    let edge = WorldEdge {
        id: id(),
        type_id: "custom.obligation".into(),
        source_id: a.id.clone(),
        target_id: b.id.clone(),
        description: "A standing maintenance agreement.".into(),
        properties: values(&a.id),
    };
    world.nodes = vec![a, b];
    world.edges = vec![edge.clone()];
    p.commit(0, id(), "Fictional link protection".into(), world.clone())
        .unwrap();
    let request = AuthoringRequest {
        mode: AuthoringMode::Revise,
        type_id: None,
        bundle: false,
        direction: String::new(),
        fields: vec![],
        link_target_ids: vec![edge.id.clone()],
        link_fields: ["generated", "status"]
            .into_iter()
            .map(|key| EditableLinkField {
                link_id: edge.id.clone(),
                property: Some(key.into()),
            })
            .collect(),
        ..request()
    };
    let prepared = prepare_authoring(&world, &request, &settings()).unwrap();
    let schema = &prepared.response_schema["properties"]["edges"]["items"]["anyOf"][0];
    for (key, expected) in [
        ("id", &edge.id),
        ("typeId", &edge.type_id),
        ("sourceId", &edge.source_id),
        ("targetId", &edge.target_id),
        ("description", &edge.description),
    ] {
        assert_eq!(schema["properties"][key]["const"], json!(expected));
    }
    assert_protection(&schema["properties"]["properties"], &edge.source_id);
    assert!(schema["required"]
        .as_array()
        .unwrap()
        .contains(&json!("properties")));
    assert_eq!(
        prepared.response_schema["properties"]["nodes"]["minItems"],
        0
    );
    assert_eq!(
        prepared.response_schema["properties"]["edges"]["minItems"],
        1
    );
    let mut with_notes = request.clone();
    with_notes.link_fields.push(EditableLinkField {
        link_id: edge.id.clone(),
        property: None,
    });
    let editable_notes = prepare_authoring(&world, &with_notes, &settings()).unwrap();
    assert!(
        editable_notes.response_schema["properties"]["edges"]["items"]["anyOf"][0]["properties"]
            ["description"]
            .get("const")
            .is_none()
    );
    let key = id();
    p.save_authoring_draft(&key, 0, request).unwrap();
    p.start_authoring(&key, 1, 1, &settings()).unwrap();
    let mut candidate = edge.clone();
    candidate.properties.insert(
        "generated".into(),
        PropertyValue::Text("The workshop replaces the storeroom's lamp wicks each month.".into()),
    );
    candidate
        .properties
        .insert("status".into(), PropertyValue::Text("closed".into()));
    candidate.properties.remove("nil");
    candidate
        .properties
        .insert("missing".into(), PropertyValue::Null);
    let response = serde_json::to_string(&NodeCandidate {
        nodes: vec![],
        edges: vec![candidate.clone()],
    })
    .unwrap();
    p.finish_authoring(&key, Ok(response.clone()), false, false)
        .unwrap();
    let review = p.authoring_view(&key, &settings()).unwrap();
    assert!(review.error.is_empty(), "{}", review.error);
    candidate.target_id = candidate.source_id.clone();
    p.save_authoring_candidate(
        &key,
        review.version,
        serde_json::to_string(&NodeCandidate {
            nodes: vec![],
            edges: vec![candidate],
        })
        .unwrap(),
    )
    .unwrap();
    let blocked = p.authoring_view(&key, &settings()).unwrap();
    assert!(blocked.error.contains("endpoints"));
    assert!(p
        .apply_authoring(&key, blocked.version, &settings())
        .is_err());
    p.save_authoring_candidate(&key, blocked.version, response)
        .unwrap();
    let ready = p.authoring_view(&key, &settings()).unwrap();
    let after = p
        .apply_authoring(&key, ready.version, &settings())
        .unwrap()
        .commit
        .project
        .world;
    let mut expected = edge;
    expected.properties.insert(
        "generated".into(),
        PropertyValue::Text("The workshop replaces the storeroom's lamp wicks each month.".into()),
    );
    expected
        .properties
        .insert("status".into(), PropertyValue::Text("closed".into()));
    assert_eq!(after.edges, vec![expected]);
    assert_eq!(after.nodes, world.nodes);
}
