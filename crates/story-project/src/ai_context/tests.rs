use super::*;
#[test]
fn bound_nodes_roundtrip_through_reopen_receipts_and_backup_restore() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("story");
    let mut world = World::default();
    let node = note(
        &mut world,
        "Rules",
        "Original rule.",
        AiContextRole::Instructions,
        vec![TextPromptTask::Passage],
    );
    let p = Project::create(
        &root,
        "Context test".into(),
        world,
        uuid::Uuid::new_v4().to_string(),
    )
    .unwrap();
    let old = p.view().unwrap();
    let mut next = old.world.clone();
    next.nodes
        .iter_mut()
        .find(|n| n.id == node)
        .unwrap()
        .description = "Updated rule.".into();
    let operation = uuid::Uuid::new_v4().to_string();
    p.commit(0, operation.clone(), old.title.clone(), next.clone())
        .unwrap();
    drop(p);
    let p = Project::open(&root).unwrap();
    assert!(
        p.commit(0, operation, old.title.clone(), next.clone())
            .unwrap()
            .replayed
    );
    let export_parent = dir.path().join("exports");
    std::fs::create_dir(&export_parent).unwrap();
    let backup = p
        .export_backup(
            &export_parent,
            &uuid::Uuid::new_v4().to_string(),
            p.view().unwrap().revision,
        )
        .unwrap();
    let inspected = inspect_backup(std::path::Path::new(&backup.folder_path)).unwrap();
    let restore_parent = dir.path().join("restored");
    std::fs::create_dir(&restore_parent).unwrap();
    let restored = restore_backup(
        std::path::Path::new(&backup.folder_path),
        &restore_parent,
        &uuid::Uuid::new_v4().to_string(),
        &inspected.fingerprint,
    )
    .unwrap();
    let restored = Project::open(std::path::Path::new(&restored.folder_path)).unwrap();
    assert_eq!(restored.view().unwrap().world, next);
    let historical: World = restored.read_record("worlds", &old.world_ref).unwrap();
    assert!(historical
        .nodes
        .iter()
        .any(|n| n.id == node && n.description == "Original rule."));
}
pub(crate) fn note(
    world: &mut World,
    name: &str,
    text: &str,
    role: AiContextRole,
    tasks: Vec<TextPromptTask>,
) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    world.nodes.push(WorldNode {
        id: id.clone(),
        type_id: "core.note".into(),
        name: name.into(),
        description: text.into(),
        visual_description: "Unselected appearance.".into(),
        components: Default::default(),
    });
    world.ai_context.push(AiContextBinding {
        node_id: id.clone(),
        enabled: true,
        role,
        tasks,
        fields: vec![AiContextField::Description, AiContextField::Components],
        order: 0,
    });
    id
}
#[test]
fn arbitrary_nodes_roles_task_filters_selected_fields_and_order_are_explicit() {
    let mut world = World::default();
    let a = note(
        &mut world,
        "Voice",
        "Use past tense.",
        AiContextRole::Instructions,
        vec![TextPromptTask::Passage],
    );
    let b = note(
        &mut world,
        "Magic",
        "Magic needs contact.",
        AiContextRole::Reference,
        vec![TextPromptTask::Passage, TextPromptTask::World],
    );
    note(
        &mut world,
        "Not selected",
        "Do not send this.",
        AiContextRole::Reference,
        vec![TextPromptTask::Memory],
    );
    world.ai_context[1].order = -10;
    world.edges.push(WorldEdge {
        id: uuid::Uuid::new_v4().to_string(),
        type_id: "core.related".into(),
        source_id: a.clone(),
        target_id: world.nodes[2].id.clone(),
        description: "".into(),
    });
    world.component_types.push(ComponentDefinition {
        id: "custom.style".into(),
        name: "Style".into(),
        description: "".into(),
        fields: vec![FieldDefinition {
            key: "person".into(),
            label: "Person".into(),
            kind: FieldKind::Text,
            required: false,
            default_value: PropertyValue::Text("".into()),
        }],
    });
    world.nodes[0].components.insert(
        "custom.style".into(),
        std::collections::BTreeMap::from([(
            "person".into(),
            PropertyValue::Text("Third person".into()),
        )]),
    );
    let preview =
        preview_ai_context(&world, TextPromptTask::Passage, &LlmConnection::default()).unwrap();
    assert_eq!(
        preview
            .sections
            .iter()
            .map(|s| s.node_id.as_str())
            .collect::<Vec<_>>(),
        vec![b.as_str(), a.as_str()]
    );
    assert!(
        preview.system.contains("Use past tense.")
            && preview.system.contains("Style / Person: Third person")
    );
    assert!(
        !preview.system.contains("Magic needs contact.")
            && !preview.system.contains("Unselected appearance")
    );
    assert_eq!(preview.references.len(), 1);
    assert_eq!(preview.references[0].name, "Magic");
    assert!(!serde_json::to_string(&preview)
        .unwrap()
        .contains("Do not send this."));
    let mut input = json!({});
    let mut system = String::new();
    append_ai_context(&preview.sections, &mut system, &mut input);
    assert!(input.get("includedReferenceFacts").is_some());
    assert!(!input.to_string().contains("Use past tense."));
    world.ai_context[0].enabled = false;
    assert!(
        !preview_ai_context(&world, TextPromptTask::Passage, &LlmConnection::default())
            .unwrap()
            .system
            .contains("Use past tense.")
    );
}
#[test]
fn legacy_worlds_and_original_receipts_keep_their_serialized_shape() {
    let original = World::default();
    let json = serde_json::to_value(&original).unwrap();
    assert!(json.get("aiContext").is_none());
    let stored: crate::model::StoredWorld = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(serde_json::to_value(stored).unwrap(), json);
    let mut world = original;
    note(
        &mut world,
        "Rules",
        "Preserve causal order.",
        AiContextRole::Reference,
        vec![TextPromptTask::Passage],
    );
    let encoded = serde_json::to_vec(&world).unwrap();
    let restored: World = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(restored, world);
    let stored: crate::model::StoredWorld = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(serde_json::to_vec(&stored).unwrap(), encoded);
}
#[test]
fn deletion_cleans_up_bindings_and_invalid_refs_duplicates_and_extreme_order_are_rejected() {
    let mut world = World::default();
    let id = note(
        &mut world,
        "Rules",
        "Keep events consistent.",
        AiContextRole::Reference,
        vec![TextPromptTask::World],
    );
    let deleted = preview_world_edit(&world, WorldEdit::RemoveNode { id: id.clone() }).unwrap();
    assert!(deleted.world.ai_context.is_empty());
    assert_eq!(world.ai_context.len(), 1);
    world.ai_context[0].order = i32::MIN;
    assert!(world.validate().is_err());
    world.ai_context[0].order = 0;
    world.ai_context.push(world.ai_context[0].clone());
    assert!(world.validate().is_err());
    world.ai_context.pop();
    world.ai_context[0].node_id = uuid::Uuid::new_v4().to_string();
    assert!(world.validate().is_err());
}
#[test]
fn mandatory_content_blocks_over_budget_instead_of_truncating_and_does_not_grant_edit_scope() {
    let mut world = World::default();
    let id = note(
        &mut world,
        "Required rules",
        &"Detailed rules. ".repeat(700),
        AiContextRole::Reference,
        vec![TextPromptTask::World],
    );
    let request = AuthoringRequest {
        mode: AuthoringMode::Create,
        direction: "An observatory.".into(),
        type_id: Some("core.place".into()),
        bundle: false,
        target_ids: vec![],
        context_ids: vec![],
        fields: vec![],
    };
    let mut connection = LlmConnection {
        provider: TextProvider::Demo,
        ..Default::default()
    };
    let request_ok = prepare_authoring(&world, &request, &connection).unwrap();
    assert!(request_ok.user.contains("Required rules"));
    assert!(!request_ok.context.nodes.iter().any(|n| n.id == id));
    assert!(request_ok.context.ai_context.is_empty());
    connection.context_tokens = 2048;
    connection.max_tokens = 512;
    let error = prepare_authoring(&world, &request, &connection).unwrap_err();
    assert!(error.contains("Required rules") && error.contains("no required content was dropped"));
    world.nodes[0].description.clear();
    assert!(resolve_ai_context(&world, TextPromptTask::World)
        .unwrap_err()
        .contains("no selected content"));
}
