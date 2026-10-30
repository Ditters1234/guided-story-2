use super::*;
mod blank_nodes;
mod creation_quality;
mod creation_scopes;
mod deletion;
mod existing_context;
mod quality_probe;
mod relationships;
fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn settings() -> LlmConnection {
    LlmConnection {
        provider: TextProvider::Demo,
        ..LlmConnection::default()
    }
}
fn create() -> (tempfile::TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let p = Project::create(
        &dir.path().join("story"),
        "Fictional world".into(),
        World::default(),
        id(),
    )
    .unwrap();
    (dir, p)
}
fn request() -> AuthoringRequest {
    AuthoringRequest {
        link_target_ids: vec![],
        include_narration: None,
        creation_type_ids: vec![],
        link_fields: vec![],
        mode: AuthoringMode::Create,
        direction: "An old glasshouse".into(),
        type_id: Some("core.place".into()),
        bundle: false,
        creation_scope: None,
        group_type_id: None,
        target_ids: vec![],
        context_ids: vec![],
        fields: vec![],
    }
}
fn group_request() -> AuthoringRequest {
    AuthoringRequest {
        link_target_ids: vec![],
        include_narration: None,
        creation_type_ids: vec![],
        link_fields: vec![],
        type_id: None,
        bundle: true,
        direction: "Develop a fictional isolated homestead with two adult inhabitants.".into(),
        ..request()
    }
}
#[test]
fn creation_contract_and_clothing_pattern_support_custom_items_through_image_preparation() {
    let (_dir, p) = create();
    let mut world = p.view().unwrap().world;
    world
        .node_types
        .retain(|t| !t.components.iter().any(|c| c == "core.wearable"));
    world
        .node_types
        .iter_mut()
        .find(|t| t.id == "core.item")
        .unwrap()
        .id = "custom.possession".into();
    p.commit(0, id(), "Fictional".into(), world.clone())
        .unwrap();
    let req = group_request();
    let prepared = prepare_authoring(&world, &req, &settings()).unwrap();
    let character = prepared.response_schema["properties"]["nodes"]["items"]["anyOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["properties"]["typeId"]["const"] == "core.character")
        .unwrap();
    let required = character["properties"]["components"]["required"]
        .as_array()
        .unwrap();
    assert!(required.contains(&json!("core.character")));
    assert!(required.contains(&json!("core.holder")));
    let input: Value = serde_json::from_str(&prepared.user).unwrap();
    assert_eq!(input["responseBudget"]["outputTokens"], 4096);
    assert!(input["responseBudget"]["suggestedMaxNewNodes"]
        .as_u64()
        .is_some_and(|n| (10..=20).contains(&n)));
    let example = &input["clothingExample"];
    let garments: Vec<WorldNode> = serde_json::from_value(example["nodes"].clone()).unwrap();
    assert!(garments.iter().all(|g| g.type_id == "custom.possession"));
    assert_eq!(garments.len(), 3);
    let person = proposed_node(&world, "core.character", "new:wearer", "Nora Hale");
    let rid = id();
    p.save_authoring_draft(&rid, 0, req).unwrap();
    p.start_authoring(&rid, 1, 1, &settings()).unwrap();
    p.finish_authoring(
        &rid,
        Ok(serde_json::to_string(&NodeCandidate {
            nodes: std::iter::once(person).chain(garments).collect(),
            edges: serde_json::from_value(example["edges"].clone()).unwrap(),
        })
        .unwrap()),
        false,
        false,
    )
    .unwrap();
    let view = p.authoring_view(&rid, &settings()).unwrap();
    assert!(view.error.is_empty(), "{}", view.error);
    assert!(view.warnings.is_empty());
    assert_eq!(p.view().unwrap().world.nodes.len(), 0);
    let world = p
        .apply_authoring(&rid, view.version, &settings())
        .unwrap()
        .commit
        .project
        .world;
    let person = &world
        .nodes
        .iter()
        .find(|n| n.name == "Nora Hale")
        .unwrap()
        .id;
    let image = p
        .create_media(
            &id(),
            MediaTarget {
                guidance_source: None,
                node_id: Some(person.clone()),
                passage_id: None,
                moment: ImageMoment::After,
                subject_ids: vec![],
            },
        )
        .unwrap();
    assert_eq!(image.draft.source.facts[0].outfit.len(), 3);
    assert!(image.draft.source.facts[0]
        .outfit
        .iter()
        .any(|g| g.contains("Linen work shirt")));
}
#[test]
fn review_reports_missing_outfits_but_preserves_intentional_unclothed_choices() {
    let (_dir, p) = create();
    let world = p.view().unwrap().world;
    let mut req = group_request();
    req.direction = "Two intentionally unclothed adult characters.".into();
    let rid = id();
    p.save_authoring_draft(&rid, 0, req).unwrap();
    p.start_authoring(&rid, 1, 0, &settings()).unwrap();
    let candidate = NodeCandidate {
        nodes: vec![
            proposed_node(&world, "core.character", "new:a", "Nora Hale"),
            proposed_node(&world, "core.character", "new:b", "Elias Rowan"),
        ],
        edges: vec![],
    };
    p.finish_authoring(
        &rid,
        Ok(serde_json::to_string(&candidate).unwrap()),
        false,
        false,
    )
    .unwrap();
    let view = p.authoring_view(&rid, &settings()).unwrap();
    assert_eq!(view.warnings.len(), 2);
    assert!(view.error.is_empty());
    assert!(p.apply_authoring(&rid, view.version, &settings()).is_ok());
    let single = AuthoringRequest {
        link_target_ids: vec![],
        include_narration: None,
        creation_type_ids: vec![],
        link_fields: vec![],
        type_id: Some("core.character".into()),
        bundle: false,
        creation_scope: None,
        group_type_id: None,
        ..request()
    };
    assert!(proposal_warnings(&candidate, &single).is_empty());
}
#[test]
fn revised_nodes_keep_their_actual_capabilities_without_reintroducing_removed_presets() {
    let mut world = World::default();
    let mut node = proposed_node(&world, "core.character", &id(), "Elias Rowan");
    node.components.remove("core.character");
    world.nodes.push(node.clone());
    let req = AuthoringRequest {
        link_target_ids: vec![],
        include_narration: None,
        creation_type_ids: vec![],
        link_fields: vec![],
        mode: AuthoringMode::Revise,
        type_id: None,
        bundle: false,
        creation_scope: None,
        group_type_id: None,
        target_ids: vec![node.id.clone()],
        context_ids: vec![],
        fields: vec![EditableField {
            node_id: node.id,
            component_id: None,
            key: "description".into(),
        }],
        direction: "Change the description only.".into(),
    };
    let prepared = prepare_authoring(&world, &req, &settings()).unwrap();
    let required = &prepared.response_schema["properties"]["nodes"]["items"]["anyOf"][0]
        ["properties"]["components"]["required"];
    assert_eq!(
        required,
        &json!(world.nodes[0].components.keys().collect::<Vec<_>>())
    );
    assert!(!required
        .as_array()
        .unwrap()
        .contains(&json!("core.character")));
    assert!(!prepared.user.contains("clothingExample"));
}
fn generated(p: &Project, request: AuthoringRequest) -> String {
    let rid = id();
    p.save_authoring_draft(&rid, 0, request.clone()).unwrap();
    p.start_authoring(&rid, 1, p.view().unwrap().revision, &settings())
        .unwrap();
    let text = demo_authoring(&p.view().unwrap().world, &request).unwrap();
    p.finish_authoring(&rid, Ok(text), false, false).unwrap();
    rid
}
#[test]
fn drafts_and_candidates_survive_reopen_without_changing_canon() {
    let (dir, p) = create();
    let rid = generated(&p, request());
    assert_eq!(p.view().unwrap().revision, 0);
    let view = p.authoring_view(&rid, &settings()).unwrap();
    assert_eq!(view.status, AuthoringStatus::Review);
    assert!(view.error.is_empty());
    let ids = view.created_node_ids;
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    let view = p.authoring_view(&rid, &settings()).unwrap();
    assert_eq!(view.created_node_ids, ids);
    assert_eq!(p.view().unwrap().world.nodes.len(), 0);
}
#[test]
fn apply_commits_once_and_recovers_after_receipt_before_status_update() {
    let (_dir, p) = create();
    let rid = generated(&p, request());
    let view = p.authoring_view(&rid, &settings()).unwrap();
    let result = p.apply_authoring(&rid, view.version, &settings()).unwrap();
    assert_eq!(result.commit.project.revision, 1);
    assert_eq!(
        p.apply_authoring(&rid, view.version, &settings())
            .unwrap()
            .commit
            .replayed,
        true
    );
    assert_eq!(p.view().unwrap().world.nodes.len(), 1);
    let mut record = p.read_authoring(&rid).unwrap();
    record.status = AuthoringStatus::Review;
    record.applied_revision = None;
    p.write_authoring(&record).unwrap();
    assert_eq!(
        p.authoring_view(&rid, &settings()).unwrap().status,
        AuthoringStatus::Applied
    );
    assert_eq!(p.view().unwrap().revision, 1);
}
#[test]
fn custom_capability_fields_use_the_same_generation_and_protection_rules() {
    let (_dir, p) = create();
    let mut world = p.view().unwrap().world;
    world.component_types.push(ComponentDefinition {
        id: "custom.weather".into(),
        name: "Weather".into(),
        description: "A forecast".into(),
        fields: vec![FieldDefinition {
            key: "forecast".into(),
            label: "Forecast".into(),
            kind: FieldKind::Text,
            required: true,
            default_value: PropertyValue::Text(String::new()),
            metadata: None,
        }],
    });
    world.node_types.push(NodeTypeDefinition {
        id: "custom.storm".into(),
        name: "Storm".into(),
        description: String::new(),
        components: vec!["custom.weather".into()],
    });
    p.commit(0, id(), "Fictional".into(), world).unwrap();
    let mut req = request();
    req.type_id = Some("custom.storm".into());
    let rid = generated(&p, req);
    let view = p.authoring_view(&rid, &settings()).unwrap();
    p.apply_authoring(&rid, view.version, &settings()).unwrap();
    let node = p.view().unwrap().world.nodes[0].clone();
    let req = AuthoringRequest {
        link_target_ids: vec![],
        include_narration: None,
        creation_type_ids: vec![],
        link_fields: vec![],
        mode: AuthoringMode::Revise,
        direction: "Rain before evening".into(),
        type_id: None,
        bundle: false,
        creation_scope: None,
        group_type_id: None,
        target_ids: vec![node.id.clone()],
        context_ids: vec![],
        fields: vec![EditableField {
            node_id: node.id.clone(),
            component_id: Some("custom.weather".into()),
            key: "forecast".into(),
        }],
    };
    let rid = generated(&p, req);
    let view = p.authoring_view(&rid, &settings()).unwrap();
    assert!(!view.stale);
    p.apply_authoring(&rid, view.version, &settings()).unwrap();
    let after = p.view().unwrap().world.nodes[0].clone();
    assert_eq!(node.name, after.name);
    assert_ne!(node.components, after.components);
}
#[test]
fn protected_fields_and_unexposed_references_cannot_be_smuggled_into_a_candidate() {
    let (_dir, p) = create();
    let rid = generated(&p, request());
    let view = p.authoring_view(&rid, &settings()).unwrap();
    p.apply_authoring(&rid, view.version, &settings()).unwrap();
    let node = p.view().unwrap().world.nodes[0].clone();
    let req = AuthoringRequest {
        link_target_ids: vec![],
        include_narration: None,
        creation_type_ids: vec![],
        link_fields: vec![],
        mode: AuthoringMode::Revise,
        direction: "More lush".into(),
        type_id: None,
        bundle: false,
        creation_scope: None,
        group_type_id: None,
        target_ids: vec![node.id.clone()],
        context_ids: vec![],
        fields: vec![EditableField {
            node_id: node.id.clone(),
            component_id: None,
            key: "description".into(),
        }],
    };
    let rid = generated(&p, req);
    let mut candidate: NodeCandidate =
        serde_json::from_str(&p.authoring_view(&rid, &settings()).unwrap().candidate_text).unwrap();
    candidate.nodes[0].name = "Unrequested rename".into();
    let version = p.authoring_view(&rid, &settings()).unwrap().version;
    p.save_authoring_candidate(&rid, version, serde_json::to_string(&candidate).unwrap())
        .unwrap();
    let view = p.authoring_view(&rid, &settings()).unwrap();
    assert!(view.error.contains("protected"));
    assert!(p.apply_authoring(&rid, view.version, &settings()).is_err());
}
#[test]
fn source_edits_stale_a_proposal_while_unrelated_world_edits_are_preserved() {
    let (_dir, p) = create();
    let first = generated(&p, request());
    let v = p.authoring_view(&first, &settings()).unwrap();
    p.apply_authoring(&first, v.version, &settings()).unwrap();
    let node = p.view().unwrap().world.nodes[0].clone();
    let req = AuthoringRequest {
        link_target_ids: vec![],
        include_narration: None,
        creation_type_ids: vec![],
        link_fields: vec![],
        mode: AuthoringMode::Revise,
        direction: "More lush".into(),
        type_id: None,
        bundle: false,
        creation_scope: None,
        group_type_id: None,
        target_ids: vec![node.id.clone()],
        context_ids: vec![],
        fields: vec![EditableField {
            node_id: node.id,
            component_id: None,
            key: "description".into(),
        }],
    };
    let rid = generated(&p, req);
    let mut current = p.view().unwrap();
    let mut unrelated = current.world.nodes[0].clone();
    unrelated.id = id();
    unrelated.name = "Unrelated room".into();
    current.world.nodes.push(unrelated.clone());
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let view = p.authoring_view(&rid, &settings()).unwrap();
    assert!(!view.stale);
    p.apply_authoring(&rid, view.version, &settings()).unwrap();
    assert!(p.view().unwrap().world.nodes.contains(&unrelated));
    let rid = generated(&p, p.authoring_view(&rid, &settings()).unwrap().request);
    let mut current = p.view().unwrap();
    current.world.nodes[0].description = "Manual change during generation".into();
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let view = p.authoring_view(&rid, &settings()).unwrap();
    assert!(view.stale);
    assert!(p.apply_authoring(&rid, view.version, &settings()).is_err());
}
#[test]
fn interrupted_and_cancelled_work_is_not_resent_and_bad_candidates_remain_editable() {
    let (_dir, p) = create();
    let rid = id();
    let req = request();
    p.save_authoring_draft(&rid, 0, req).unwrap();
    p.start_authoring(&rid, 1, 0, &settings()).unwrap();
    p.interrupt_authoring().unwrap();
    assert_eq!(
        p.authoring_view(&rid, &settings()).unwrap().status,
        AuthoringStatus::Interrupted
    );
    assert!(p.start_authoring(&rid, 2, 0, &settings()).is_err());
    let rid = generated(&p, request());
    let version = p.authoring_view(&rid, &settings()).unwrap().version;
    p.save_authoring_candidate(&rid, version, "{unfinished".into())
        .unwrap();
    assert!(p
        .authoring_view(&rid, &settings())
        .unwrap()
        .error
        .contains("JSON"));
    assert_eq!(p.view().unwrap().revision, 0);
}
#[test]
fn context_budget_is_enforced_before_any_provider_call() {
    let mut settings = settings();
    settings.context_tokens = 2048;
    settings.max_tokens = 1024;
    assert!(prepare_authoring(&World::default(), &request(), &settings).is_err());
}
#[test]
fn revisions_keep_custom_target_schema_but_exclude_unrelated_catalogue_and_duplicate_schema() {
    let (_dir, p) = create();
    let rid = generated(&p, request());
    let v = p.authoring_view(&rid, &settings()).unwrap();
    p.apply_authoring(&rid, v.version, &settings()).unwrap();
    let mut world = p.view().unwrap().world;
    let node = &world.nodes[0];
    let req = AuthoringRequest {
        link_target_ids: vec![],
        include_narration: None,
        creation_type_ids: vec![],
        link_fields: vec![],
        mode: AuthoringMode::Revise,
        direction: "Change only the room's description".into(),
        type_id: None,
        bundle: false,
        creation_scope: None,
        group_type_id: None,
        target_ids: vec![node.id.clone()],
        context_ids: vec![],
        fields: vec![EditableField {
            node_id: node.id.clone(),
            component_id: None,
            key: "description".into(),
        }],
    };
    let prepared = prepare_authoring(&world, &req, &settings()).unwrap();
    assert_eq!(prepared.context.node_types.len(), 1);
    assert!(!prepared.user.contains("core.character"));
    assert!(!prepared.user.contains("core.wearable"));
    world.component_types.push(ComponentDefinition {
        id: "custom.unused".into(),
        name: "Unrelated subsystem".into(),
        description: "No reference to the selected place".into(),
        fields: vec![],
    });
    let after = prepare_authoring(&world, &req, &settings()).unwrap();
    assert_eq!(prepared.fingerprint, after.fingerprint);
    assert_eq!(
        prepared.estimated_input_tokens,
        after.estimated_input_tokens
    );
    let mut schema_settings = settings();
    schema_settings.structured_output = StructuredOutput::Schema;
    let schema = prepare_authoring(&world, &req, &schema_settings).unwrap();
    assert!(serde_json::from_str::<Value>(&schema.user)
        .unwrap()
        .get("responseSchema")
        .is_none());
    assert!(serde_json::from_str::<Value>(&prepared.user)
        .unwrap()
        .get("responseSchema")
        .is_some());
    assert_eq!(prepared.response_schema, schema.response_schema);
    assert!(schema.estimated_input_tokens <= prepared.estimated_input_tokens);
}

fn proposed_node(world: &World, type_id: &str, node_id: &str, name: &str) -> WorldNode {
    let preset = world.node_types.iter().find(|d| d.id == type_id).unwrap();
    let components = preset
        .components
        .iter()
        .map(|id| {
            let definition = world.component_types.iter().find(|d| &d.id == id).unwrap();
            (
                id.clone(),
                definition
                    .fields
                    .iter()
                    .map(|f| (f.key.clone(), f.default_value.clone()))
                    .collect(),
            )
        })
        .collect();
    WorldNode {
        id: node_id.into(),
        type_id: type_id.into(),
        name: name.into(),
        description: String::new(),
        visual_description: String::new(),
        components,
    }
}
#[test]
fn connected_character_outfit_container_and_custom_references_remap_atomically() {
    let (_dir, p) = create();
    let mut world = p.view().unwrap().world;
    world.component_types.push(ComponentDefinition {
        id: "custom.reference".into(),
        name: "Reference".into(),
        description: String::new(),
        fields: vec![FieldDefinition {
            key: "container".into(),
            label: "Container".into(),
            kind: FieldKind::NodeRef,
            required: false,
            default_value: PropertyValue::Null,
            metadata: None,
        }],
    });
    p.commit(0, id(), "Fictional".into(), world.clone())
        .unwrap();
    let person = proposed_node(&world, "core.character", "new:root", "Adult herbalist");
    let mut coat = proposed_node(&world, "core.item", "new:coat", "Field coat");
    let wearable = world
        .component_types
        .iter()
        .find(|d| d.id == "core.wearable")
        .unwrap();
    coat.components.insert(
        wearable.id.clone(),
        wearable
            .fields
            .iter()
            .map(|f| (f.key.clone(), f.default_value.clone()))
            .collect(),
    );
    let mut bag = proposed_node(&world, "core.item", "new:bag", "Satchel");
    bag.components
        .insert("core.container".into(), BTreeMap::new());
    let mut seeds = proposed_node(&world, "core.item", "new:seeds", "Seeds");
    seeds.components.insert(
        "custom.reference".into(),
        BTreeMap::from([("container".into(), PropertyValue::Text("new:bag".into()))]),
    );
    seeds.description = "Literal text new:bag stays text.".into();
    let edges = [
        ("worn-by", "new:coat", "new:root"),
        ("stored-in", "new:bag", "new:root"),
        ("stored-in", "new:seeds", "new:bag"),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (kind, source, target))| WorldEdge {
        properties: Default::default(),
        id: format!("new:edge{i}"),
        type_id: format!("core.{kind}"),
        source_id: source.into(),
        target_id: target.into(),
        description: String::new(),
    })
    .collect();
    let mut req = request();
    req.type_id = Some("core.character".into());
    req.bundle = true;
    let rid = id();
    p.save_authoring_draft(&rid, 0, req).unwrap();
    p.start_authoring(&rid, 1, 1, &settings()).unwrap();
    p.finish_authoring(
        &rid,
        Ok(serde_json::to_string(&NodeCandidate {
            nodes: vec![person, coat, bag, seeds],
            edges,
        })
        .unwrap()),
        false,
        false,
    )
    .unwrap();
    let v = p.authoring_view(&rid, &settings()).unwrap();
    assert!(v.error.is_empty(), "{}", v.error);
    let after = p
        .apply_authoring(&rid, v.version, &settings())
        .unwrap()
        .commit
        .project
        .world;
    assert_eq!(after.nodes.len(), 4);
    assert_eq!(after.edges.len(), 3);
    after.validate().unwrap();
    let bag = &after.nodes.iter().find(|n| n.name == "Satchel").unwrap().id;
    let seeds = after.nodes.iter().find(|n| n.name == "Seeds").unwrap();
    assert_eq!(
        seeds.components["custom.reference"]["container"],
        PropertyValue::Text(bag.clone())
    );
    assert!(seeds.description.contains("new:bag"));
}
#[test]
fn unselected_world_records_are_not_sent_and_new_links_cannot_move_existing_nodes() {
    let (_dir, p) = create();
    let mut world = p.view().unwrap().world;
    let private = proposed_node(&world, "core.place", &id(), "PRIVATE_NODE_MARKER");
    world.nodes.push(private.clone());
    p.commit(0, id(), "Fictional".into(), world.clone())
        .unwrap();
    let prepared = prepare_authoring(&world, &request(), &settings()).unwrap();
    assert!(!prepared.user.contains("PRIVATE_NODE_MARKER"));
    let mut req = request();
    req.context_ids = vec![private.id.clone()];
    let rid = generated(&p, req);
    let mut view = p.authoring_view(&rid, &settings()).unwrap();
    assert_eq!(view.context.nodes.len(), 1);
    let mut candidate: NodeCandidate = serde_json::from_str(&view.candidate_text).unwrap();
    candidate.edges.push(WorldEdge {
        properties: Default::default(),
        id: "new:bad".into(),
        type_id: "core.related".into(),
        source_id: private.id,
        target_id: "new:root".into(),
        description: String::new(),
    });
    p.save_authoring_candidate(
        &rid,
        view.version,
        serde_json::to_string(&candidate).unwrap(),
    )
    .unwrap();
    view = p.authoring_view(&rid, &settings()).unwrap();
    assert!(!view.error.is_empty());
    assert!(p.apply_authoring(&rid, view.version, &settings()).is_err());
    assert_eq!(p.view().unwrap().revision, 1);
}
#[test]
fn truncated_responses_and_connection_changes_block_apply_without_losing_output() {
    let (_dir, p) = create();
    let req = request();
    let rid = id();
    p.save_authoring_draft(&rid, 0, req.clone()).unwrap();
    p.start_authoring(&rid, 1, 0, &settings()).unwrap();
    let text = demo_authoring(&World::default(), &req).unwrap();
    p.finish_authoring(&rid, Ok(text.clone()), false, true)
        .unwrap();
    let view = p.authoring_view(&rid, &settings()).unwrap();
    assert!(view.error.contains("cut off"));
    assert_eq!(view.raw_response, text);
    assert!(p.apply_authoring(&rid, view.version, &settings()).is_err());
    p.save_authoring_candidate(&rid, view.version, format!("{text}\n"))
        .unwrap();
    let mut different = settings();
    different.temperature = 0.5;
    let view = p.authoring_view(&rid, &different).unwrap();
    assert!(view.stale);
    assert!(p.apply_authoring(&rid, view.version, &different).is_err());
}
