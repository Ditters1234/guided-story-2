use super::*;
#[test]
fn creation_request_budget_audit() {
    let w = World::default();
    let mut request = group_request();
    request.creation_scope = Some(CreationScope::World);
    request.include_narration = Some(true);
    request.creation_type_ids = creation::foundation_types(&w);
    let prepared = prepare_authoring(&w, &request, &settings()).unwrap();
    let user: Value = serde_json::from_str(&prepared.user).unwrap();
    assert!(prepared.estimated_input_tokens < 18000);
    assert!(
        user["responseBudget"]["suggestedMaxNewNodes"]
            .as_u64()
            .unwrap()
            >= 10
    );
    assert!(user["creationGuide"]["placeStructure"]["rule"]
        .as_str()
        .unwrap()
        .contains("child house, barn and field"));
    assert_eq!(user["creationGuide"]["narration"]["requested"], true);
    assert!(
        prepared.response_schema["$defs"]["components"]["properties"]
            .get("core.claim")
            .is_none()
    );
    println!(
        "{}",
        json!({"inputBytes":prepared.system.len()+prepared.user.len(),"estimatedInputTokens":prepared.estimated_input_tokens,"outputTokens":settings().max_tokens,"suggestedNodes":user["responseBudget"]["suggestedMaxNewNodes"],"schemaBytes":prepared.response_schema.to_string().len()})
    );
}
fn world_request(w: &World) -> AuthoringRequest {
    AuthoringRequest {
        creation_scope: Some(CreationScope::World),
        include_narration: Some(true),
        creation_type_ids: creation::foundation_types(w),
        ..group_request()
    }
}

#[test]
fn narration_and_place_hierarchy_publish_together_only_on_apply_and_replay_once() {
    let (_dir, p) = create();
    let source = p.view().unwrap();
    let req = world_request(&source.world);
    let key = generated(&p, req.clone());
    let review = p.authoring_view(&key, &settings()).unwrap();
    assert!(review.error.is_empty(), "{}", review.error);
    assert_eq!(p.view().unwrap().revision, 0);
    assert!(p.view().unwrap().world.ai_context.is_empty());
    let candidate: NodeCandidate = serde_json::from_str(&review.candidate_text).unwrap();
    assert_eq!(
        candidate
            .nodes
            .iter()
            .filter(|n| n.components.contains_key("core.place"))
            .count(),
        4
    );
    assert_eq!(
        candidate
            .edges
            .iter()
            .filter(|e| e.type_id == "core.inside")
            .count(),
        3
    );
    assert!(review
        .changes
        .iter()
        .any(|s| s.contains("instructions for passage writing")));
    let result = p
        .apply_authoring(&key, review.version, &settings())
        .unwrap();
    let world = result.commit.project.world;
    assert_eq!(world.ai_context.len(), 1);
    let included = &world.ai_context[0];
    assert_eq!(included.role, AiContextRole::Instructions);
    assert_eq!(
        included.tasks,
        vec![TextPromptTask::Passage, TextPromptTask::Wording]
    );
    assert!(world
        .nodes
        .iter()
        .any(|n| n.id == included.node_id && n.components.contains_key("core.narration-style")));
    assert!(
        !resolve_ai_context(&world, TextPromptTask::Passage).unwrap()[0]
            .content
            .is_empty()
    );
    let replay = p
        .apply_authoring(&key, review.version, &settings())
        .unwrap();
    assert!(replay.commit.replayed);
    assert_eq!(replay.commit.project.world.ai_context.len(), 1);
    assert_eq!(replay.commit.project.revision, 1);
}

#[test]
fn optional_creation_flags_keep_legacy_serialization_and_opt_out_does_not_bind() {
    let (_dir, p) = create();
    let req = group_request();
    let value = serde_json::to_value(&req).unwrap();
    assert!(value.get("includeNarration").is_none());
    assert!(value.get("creationTypeIds").is_none());
    let mut explicit = world_request(&p.view().unwrap().world);
    explicit.include_narration = Some(false);
    let key = generated(&p, explicit);
    let v = p.authoring_view(&key, &settings()).unwrap();
    let result = p.apply_authoring(&key, v.version, &settings()).unwrap();
    assert!(result.commit.project.world.ai_context.is_empty());
}

#[test]
fn single_type_group_has_a_focused_schema_with_inventory_extensions_and_custom_fields() {
    let mut w = World::default();
    w.component_types.push(ComponentDefinition {
        id: "custom.tool".into(),
        name: "Tool details".into(),
        description: String::new(),
        fields: vec![FieldDefinition {
            key: "description".into(),
            label: "Repair limitations".into(),
            kind: FieldKind::LongText,
            required: false,
            default_value: PropertyValue::Null,
            metadata: None,
        }],
    });
    let mut preset = w
        .node_types
        .iter()
        .find(|t| t.id == "core.item")
        .unwrap()
        .clone();
    preset.id = "custom.supply".into();
    preset.components.push("custom.tool".into());
    w.node_types.push(preset);
    let req = AuthoringRequest {
        creation_scope: Some(CreationScope::Group),
        group_type_id: Some("custom.supply".into()),
        ..group_request()
    };
    let prepared = prepare_authoring(&w, &req, &settings()).unwrap();
    let input: Value = serde_json::from_str(&prepared.user).unwrap();
    assert!(
        prepared.estimated_input_tokens < 11000,
        "{}",
        prepared.estimated_input_tokens
    );
    assert_eq!(input["nodeTypes"].as_array().unwrap().len(), 1);
    assert!(input["creationGuide"].get("placeStructure").is_none());
    let caps = &prepared.response_schema["$defs"]["components"]["properties"];
    assert!(caps.get("core.container").is_some());
    assert!(caps.get("core.wearable").is_some());
    assert!(caps.get("detail.voice").is_none());
    assert!(caps["custom.tool"]["properties"]
        .get("description")
        .is_some());
    assert!(prepared
        .context
        .component_types
        .iter()
        .any(|d| d.id == "detail.voice"));
}

#[test]
fn selected_creation_types_are_enforced_in_schema_candidates_and_native_apply() {
    let (_dir, p) = create();
    let w = p.view().unwrap().world;
    let req = AuthoringRequest {
        creation_scope: Some(CreationScope::Group),
        creation_type_ids: vec!["core.item".into()],
        ..group_request()
    };
    let prepared = prepare_authoring(&w, &req, &settings()).unwrap();
    assert_eq!(
        prepared.response_schema["properties"]["nodes"]["items"]["anyOf"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let key = id();
    p.save_authoring_draft(&key, 0, req).unwrap();
    p.start_authoring(&key, 1, 0, &settings()).unwrap();
    p.finish_authoring(
        &key,
        Ok(serde_json::to_string(&NodeCandidate {
            nodes: vec![proposed_node(
                &w,
                "core.place",
                "new:place",
                "Unexpected town",
            )],
            edges: vec![],
        })
        .unwrap()),
        false,
        false,
    )
    .unwrap();
    let review = p.authoring_view(&key, &settings()).unwrap();
    assert!(review.error.contains("outside the selected creation types"));
    assert!(p
        .apply_authoring(&key, review.version, &settings())
        .is_err());
    assert!(p.view().unwrap().world.nodes.is_empty());
}

#[test]
fn narration_and_type_options_cannot_be_smuggled_into_revisions_or_single_type_groups() {
    let w = World::default();
    let mut req = world_request(&w);
    req.creation_scope = Some(CreationScope::Group);
    assert!(prepare_authoring(&w, &req, &settings())
        .unwrap_err()
        .contains("Narration creation"));
    req.include_narration = None;
    req.group_type_id = Some("core.item".into());
    assert!(prepare_authoring(&w, &req, &settings())
        .unwrap_err()
        .contains("unique saved creation types"));
    let mut req = world_request(&w);
    req.creation_type_ids = vec!["core.place".into()];
    assert!(prepare_authoring(&w, &req, &settings())
        .unwrap_err()
        .contains("Select a narration-style"));
}

#[test]
fn compact_optional_choices_preserve_null_and_property_names() {
    let field = FieldDefinition {
        key: "description".into(),
        label: "Description".into(),
        kind: FieldKind::Choice,
        required: false,
        default_value: PropertyValue::Null,
        metadata: Some(FieldMetadata {
            choices: vec!["open".into(), "closed".into()],
            help: Some("A helpful label".into()),
            ..Default::default()
        }),
    };
    let mut schema = json!({"type":"object","properties":{"description":field_schema(&field)}});
    creation::strip_schema_help(&mut schema);
    let value = &schema["properties"]["description"];
    assert_eq!(value["type"], json!(["string", "null"]));
    assert_eq!(value["enum"], json!(["open", "closed", null]));
    assert!(value.get("description").is_none());
}

#[test]
fn requested_narration_has_actionable_errors_for_missing_or_empty_profiles_and_uses_custom_presets()
{
    let (_dir, p) = create();
    let mut w = p.view().unwrap().world;
    w.node_types
        .iter_mut()
        .find(|t| t.id == "core.narration-style")
        .unwrap()
        .id = "custom.narrator".into();
    p.commit(0, id(), "Fictional".into(), w.clone()).unwrap();
    let req = world_request(&w);
    let key = generated(&p, req);
    let view = p.authoring_view(&key, &settings()).unwrap();
    let original: NodeCandidate = serde_json::from_str(&view.candidate_text).unwrap();
    assert!(original
        .nodes
        .iter()
        .any(|n| n.type_id == "custom.narrator"));
    let mut missing = original.clone();
    missing
        .nodes
        .retain(|n| !n.components.contains_key("core.narration-style"));
    p.save_authoring_candidate(&key, view.version, serde_json::to_string(&missing).unwrap())
        .unwrap();
    let missing = p.authoring_view(&key, &settings()).unwrap();
    assert!(
        missing
            .error
            .contains("Add a proposed Narration style node"),
        "{}",
        missing.error
    );
    assert!(p
        .apply_authoring(&key, missing.version, &settings())
        .is_err());
    let mut blank = original.clone();
    let n = blank
        .nodes
        .iter_mut()
        .find(|n| n.components.contains_key("core.narration-style"))
        .unwrap();
    n.components
        .get_mut("core.narration-style")
        .unwrap()
        .clear();
    p.save_authoring_candidate(
        &key,
        missing.version,
        serde_json::to_string(&blank).unwrap(),
    )
    .unwrap();
    let blank = p.authoring_view(&key, &settings()).unwrap();
    assert!(!blank.error.is_empty());
    assert!(p.apply_authoring(&key, blank.version, &settings()).is_err());
    p.save_authoring_candidate(
        &key,
        blank.version,
        serde_json::to_string(&original).unwrap(),
    )
    .unwrap();
    let repaired = p.authoring_view(&key, &settings()).unwrap();
    assert!(repaired.error.is_empty(), "{}", repaired.error);
    let applied = p
        .apply_authoring(&key, repaired.version, &settings())
        .unwrap();
    assert_eq!(applied.commit.project.world.ai_context.len(), 1);
}

#[test]
fn narration_creation_and_selection_survive_draft_reopen_and_backup() {
    let (dir, p) = create();
    let req = world_request(&p.view().unwrap().world);
    let key = id();
    p.save_authoring_draft(&key, 0, req.clone()).unwrap();
    let root = dir.path().join("story");
    drop(p);
    let p = Project::open(&root).unwrap();
    assert_eq!(p.authoring_view(&key, &settings()).unwrap().request, req);
    let backup = p.export_backup(dir.path(), &id(), 0).unwrap();
    let source = std::path::Path::new(&backup.folder_path);
    let reviewed = inspect_backup(source).unwrap();
    let restored = restore_backup(source, dir.path(), &id(), &reviewed.fingerprint).unwrap();
    let copy = Project::open(std::path::Path::new(&restored.folder_path)).unwrap();
    assert_eq!(copy.authoring_view(&key, &settings()).unwrap().request, req);
}

#[test]
fn interrupted_world_apply_keeps_narration_in_the_same_revision_and_recovers_status_without_duplicates(
) {
    for published in [false, true] {
        let (dir, p) = create();
        let key = generated(&p, world_request(&p.view().unwrap().world));
        let mut record = p.read_authoring(&key).unwrap();
        let current = p.view().unwrap();
        let (world, _) = candidate_world(&current.world, &mut record).unwrap();
        record.apply_attempt = Some(ApplyAttempt {
            expected_revision: 0,
            title: current.title.clone(),
            world_ref: p.write_record("worlds", &world).unwrap(),
        });
        record.version += 1;
        p.write_authoring(&record).unwrap();
        // Stop after the durable Apply intent, or after the world commit but
        // before its final proposal status write. Store crash probes separately
        // exercise the atomic record/manifest/head boundaries.
        if published {
            p.commit(0, key.clone(), current.title, world).unwrap();
        }
        drop(p);
        let p = Project::open(&dir.path().join("story")).unwrap();
        let before = p.view().unwrap();
        assert_eq!(before.revision, u32::from(published));
        assert_eq!(before.world.ai_context.len(), usize::from(published));
        assert_eq!(before.world.nodes.len(), if published { 5 } else { 0 });
        let v = p.authoring_view(&key, &settings()).unwrap();
        let applied = p.apply_authoring(&key, v.version, &settings()).unwrap();
        assert_eq!(applied.commit.replayed, published);
        assert_eq!(applied.commit.project.revision, 1);
        assert_eq!(applied.commit.project.world.ai_context.len(), 1);
        assert_eq!(applied.commit.project.world.nodes.len(), 5);
        let replay = p.apply_authoring(&key, v.version, &settings()).unwrap();
        assert!(replay.commit.replayed);
        assert_eq!(replay.commit.project.world.ai_context.len(), 1);
    }
}
