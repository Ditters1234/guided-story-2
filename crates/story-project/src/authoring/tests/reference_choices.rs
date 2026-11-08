use super::*;

fn field(key: &str, kind: FieldKind, capability: &str) -> FieldDefinition {
    FieldDefinition {
        key: key.into(),
        label: key.into(),
        kind,
        required: false,
        default_value: PropertyValue::Null,
        metadata: Some(FieldMetadata {
            reference_capability: Some(capability.into()),
            ..Default::default()
        }),
    }
}

fn world() -> World {
    let mut w = World::default();
    w.component_types.push(ComponentDefinition {
        id: "custom.refs".into(),
        name: "References".into(),
        description: String::new(),
        fields: vec![
            field("mentor", FieldKind::NodeRef, "core.character"),
            field("witnesses", FieldKind::NodeRefs, "core.character"),
            field("storage", FieldKind::NodeRef, "core.holder"),
            field("absent", FieldKind::NodeRefs, "detail.organization"),
        ],
    });
    w.node_types.push(NodeTypeDefinition {
        id: "custom.supply".into(),
        name: "Supply".into(),
        description: String::new(),
        components: vec!["core.item".into(), "custom.refs".into()],
    });
    w.nodes = vec![
        proposed_node(&w, "core.character", &id(), "Mara"),
        proposed_node(&w, "core.character", &id(), "Jon"),
        proposed_node(&w, "core.character", &id(), "Unexposed person"),
        proposed_node(&w, "core.place", &id(), "Glasshouse"),
    ];
    let hidden = w.nodes[2].id.clone();
    let allowed = w.nodes[1].id.clone();
    w.nodes[0].components.insert(
        "custom.refs".into(),
        BTreeMap::from([
            ("mentor".into(), PropertyValue::Text(hidden.clone())),
            (
                "witnesses".into(),
                PropertyValue::NodeRefs(vec![hidden, allowed]),
            ),
        ]),
    );
    w
}

fn revision(w: &World) -> AuthoringRequest {
    AuthoringRequest {
        mode: AuthoringMode::Revise,
        type_id: None,
        target_ids: vec![w.nodes[0].id.clone()],
        context_ids: vec![w.nodes[1].id.clone(), w.nodes[3].id.clone()],
        fields: ["mentor", "witnesses"]
            .into_iter()
            .map(|key| EditableField {
                node_id: w.nodes[0].id.clone(),
                component_id: Some("custom.refs".into()),
                key: key.into(),
            })
            .collect(),
        ..request()
    }
}

fn enums(value: &Value) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    if let Some(object) = value.as_object() {
        for (key, value) in object {
            if key == "enum" {
                result.extend(
                    value
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .map(str::to_owned),
                );
            } else if key != "const" {
                result.extend(enums(value));
            }
        }
    } else if let Some(array) = value.as_array() {
        for value in array {
            result.extend(enums(value));
        }
    }
    result
}

#[test]
fn checked_references_preserve_old_values_without_authorizing_hidden_ids_in_new_lists() {
    let w = world();
    let req = revision(&w);
    let expected: BTreeSet<_> = w.nodes[..2].iter().map(|n| n.id.clone()).collect();
    for mode in [
        StructuredOutput::Json,
        StructuredOutput::Prompt,
        StructuredOutput::Schema,
    ] {
        let mut settings = settings();
        settings.structured_output = mode;
        let prepared = prepare_authoring(&w, &req, &settings).unwrap();
        let props = &prepared.response_schema["properties"]["nodes"]["items"]["anyOf"][0]
            ["properties"]["components"]["properties"]["custom.refs"]["properties"];
        assert_eq!(enums(&props["mentor"]), expected);
        assert_eq!(enums(&props["witnesses"]), expected);
        assert!(!props.to_string().contains("pattern"));
        let old = &w.nodes[0].components["custom.refs"];
        assert!(props["mentor"]["anyOf"]
            .as_array()
            .unwrap()
            .contains(&json!({"const":old["mentor"]})));
        assert!(props["witnesses"]["anyOf"]
            .as_array()
            .unwrap()
            .contains(&json!({"const":old["witnesses"]})));
        assert!(!prepared.user.contains("Unexposed person"));
    }
    let (_dir, p) = create();
    p.commit(0, id(), "Reference fixture".into(), w.clone())
        .unwrap();
    let key = id();
    p.save_authoring_draft(&key, 0, req).unwrap();
    p.start_authoring(&key, 1, 1, &settings()).unwrap();
    let mut candidate = NodeCandidate {
        nodes: vec![w.nodes[0].clone()],
        edges: vec![],
    };
    p.finish_authoring(
        &key,
        Ok(serde_json::to_string(&candidate).unwrap()),
        false,
        false,
    )
    .unwrap();
    let review = p.authoring_view(&key, &settings()).unwrap();
    assert!(review.error.is_empty(), "{}", review.error);
    candidate.nodes[0]
        .components
        .get_mut("custom.refs")
        .unwrap()
        .insert(
            "witnesses".into(),
            PropertyValue::NodeRefs(vec![w.nodes[2].id.clone(), w.nodes[0].id.clone()]),
        );
    p.save_authoring_candidate(
        &key,
        review.version,
        serde_json::to_string(&candidate).unwrap(),
    )
    .unwrap();
    let review = p.authoring_view(&key, &settings()).unwrap();
    assert!(review.error.contains("outside"), "{}", review.error);
    assert!(p
        .apply_authoring(&key, review.version, &settings())
        .is_err());
}

#[test]
fn group_reference_choices_match_captured_index_and_budget_in_all_transports() {
    let mut w = world();
    w.nodes[0].components.remove("custom.refs");
    for i in 0..110 {
        w.nodes.push(proposed_node(
            &w,
            "core.character",
            &id(),
            &format!("Visitor {i:03}"),
        ));
    }
    let mut bag = proposed_node(&w, "core.item", &id(), "Container");
    bag.components
        .insert("core.container".into(), BTreeMap::new());
    w.nodes.push(bag.clone());
    let req = AuthoringRequest {
        type_id: None,
        bundle: true,
        creation_scope: Some(CreationScope::Group),
        group_type_id: Some("custom.supply".into()),
        context_ids: vec![bag.id.clone()],
        direction: "A supply for the container.".into(),
        ..request()
    };
    for mode in [
        StructuredOutput::Json,
        StructuredOutput::Prompt,
        StructuredOutput::Schema,
    ] {
        let mut settings = settings();
        settings.structured_output = mode;
        let prepared = prepare_authoring(&w, &req, &settings).unwrap();
        let captured = prepared.existing_context.as_ref().unwrap();
        let props = &prepared.response_schema["properties"]["nodes"]["properties"]["custom.supply"]
            ["items"]["properties"]["components"]["properties"]["custom.refs"]["properties"];
        let expected: BTreeSet<_> = captured
            .catalog
            .iter()
            .filter(|n| n.capabilities.iter().any(|c| c == "core.character"))
            .map(|n| n.id.clone())
            .collect();
        assert_eq!(enums(&props["mentor"]), expected);
        assert!(!props["mentor"].to_string().contains("pattern"));
        assert!(props["storage"].to_string().contains("pattern"));
        assert!(enums(&props["storage"]).contains(&bag.id));
        assert_eq!(props["absent"]["maxItems"], 0);
        assert!(captured.omitted_nodes > 0);
        if mode != StructuredOutput::Schema {
            let input: Value = serde_json::from_str(&prepared.user).unwrap();
            let pool = input["referenceChoices"]["pools"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["referenceCapability"] == "core.character")
                .unwrap();
            assert_eq!(pool["existingIds"], json!(expected));
            assert!(pool.get("newAliasPattern").is_none());
            let mut forged = input.clone();
            forged["referenceChoices"]["pools"][0]["existingIds"]
                .as_array_mut()
                .unwrap()
                .push(json!(id()));
            let model = creation::schema_world(&prepared.context, &req);
            let scope = crate::authoring::references::scope(
                &w,
                &model,
                &prepared.context,
                &req,
                prepared.existing_context.as_ref(),
            );
            assert!(crate::authoring::prompt_contract::lint(
                &prepared.context,
                &model,
                &req,
                &forged,
                &prepared.response_schema,
                &scope
            )
            .is_err());
        }
        settings.context_tokens = prepared.estimated_input_tokens + settings.max_tokens - 500;
        let tight = prepare_authoring(&w, &req, &settings).unwrap();
        assert!(tight.estimated_input_tokens + settings.max_tokens <= settings.context_tokens);
        assert!(tight
            .existing_context
            .as_ref()
            .unwrap()
            .detailed_node_ids
            .contains(&bag.id));
    }
}

#[test]
fn link_reference_revisions_preserve_unchecked_pointers_and_filter_checked_lists() {
    let mut w = world();
    let definition = w
        .relation_types
        .iter_mut()
        .find(|r| r.id == "story.feels-toward")
        .unwrap();
    definition.fields.extend([
        field("witnesses", FieldKind::NodeRefs, "core.character"),
        field("fixed", FieldKind::NodeRef, "core.character"),
    ]);
    let edge = WorldEdge {
        id: id(),
        type_id: "story.feels-toward".into(),
        source_id: w.nodes[0].id.clone(),
        target_id: w.nodes[1].id.clone(),
        description: String::new(),
        properties: BTreeMap::from([
            (
                "witnesses".into(),
                PropertyValue::NodeRefs(vec![w.nodes[2].id.clone()]),
            ),
            ("fixed".into(), PropertyValue::Text(w.nodes[2].id.clone())),
        ]),
    };
    w.edges.push(edge.clone());
    let req = AuthoringRequest {
        mode: AuthoringMode::Revise,
        type_id: None,
        link_target_ids: vec![edge.id.clone()],
        link_fields: vec![EditableLinkField {
            link_id: edge.id.clone(),
            property: Some("witnesses".into()),
        }],
        ..request()
    };
    let prepared = prepare_authoring(&w, &req, &settings()).unwrap();
    let props = &prepared.response_schema["properties"]["edges"]["items"]["anyOf"][0]["properties"]
        ["properties"]["properties"];
    assert_eq!(
        enums(&props["witnesses"]),
        w.nodes[..2].iter().map(|n| n.id.clone()).collect()
    );
    assert_eq!(props["fixed"], json!({"const":w.nodes[2].id}));
    assert!(!prepared.user.contains("Unexposed person"));
}

#[test]
fn single_node_reference_choices_offer_only_the_known_root_and_no_invented_targets() {
    let w = world();
    let req = AuthoringRequest {
        type_id: Some("custom.supply".into()),
        ..request()
    };
    let prepared = prepare_authoring(&w, &req, &settings()).unwrap();
    let fields = &prepared.response_schema["properties"]["nodes"]["items"]["anyOf"][0]
        ["properties"]["components"]["properties"]["custom.refs"]["properties"];
    assert_eq!(fields["mentor"], json!({"type":"null"}));
    assert_eq!(fields["witnesses"]["maxItems"], 0);
    assert!(fields["storage"]["anyOf"]
        .as_array()
        .unwrap()
        .contains(&json!({"const":"new:root"})));
    assert!(!fields["storage"].to_string().contains("pattern"));
    let input: Value = serde_json::from_str(&prepared.user).unwrap();
    let storage = input["referenceChoices"]["pools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["referenceCapability"] == "core.holder")
        .unwrap();
    assert_eq!(storage["newAliasId"], "new:root");
}
