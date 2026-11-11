use super::*;
use std::collections::BTreeMap;
fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn profile(world: &mut World) -> String {
    let definition = world
        .component_types
        .iter()
        .find(|c| c.id == "core.narration-style")
        .unwrap();
    let node_id = id();
    world.nodes.push(WorldNode {
        id: node_id.clone(),
        type_id: "core.narration-style".into(),
        name: "Narration style".into(),
        description: "Use concrete sensory details.".into(),
        visual_description: String::new(),
        components: BTreeMap::from([(
            "core.narration-style".into(),
            definition
                .fields
                .iter()
                .map(|f| (f.key.clone(), f.default_value.clone()))
                .collect(),
        )]),
    });
    world.ai_context.push(AiContextBinding {
        image_preparation: false,
        node_id: node_id.clone(),
        enabled: true,
        role: AiContextRole::Instructions,
        tasks: vec![TextPromptTask::Passage, TextPromptTask::Wording],
        fields: vec![AiContextField::Description, AiContextField::Components],
        order: 0,
    });
    node_id
}
#[test]
fn narration_preset_is_optional_for_old_worlds_and_legacy_receipts_keep_their_schema() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("story");
    let mut legacy = World::default();
    legacy.node_types.retain(|d| d.id != "core.narration-style");
    legacy
        .component_types
        .retain(|d| d.id != "core.narration-style");
    legacy.validate().unwrap();
    let bytes = serde_json::to_vec(&legacy).unwrap();
    let p = Project::create(&root, "Legacy fictional story".into(), legacy.clone(), id()).unwrap();
    let request = id();
    p.commit(
        0,
        request.clone(),
        "Edited legacy story".into(),
        legacy.clone(),
    )
    .unwrap();
    let before = p.view().unwrap();
    let catalog = World::default();
    let mut next = legacy.clone();
    next.node_types.push(
        catalog
            .node_types
            .into_iter()
            .find(|d| d.id == "core.narration-style")
            .unwrap(),
    );
    next.component_types.push(
        catalog
            .component_types
            .into_iter()
            .find(|d| d.id == "core.narration-style")
            .unwrap(),
    );
    profile(&mut next);
    next.validate().unwrap();
    p.commit(before.revision, id(), before.title.clone(), next.clone())
        .unwrap();
    assert!(
        p.commit(0, request, "Edited legacy story".into(), legacy)
            .unwrap()
            .replayed
    );
    let historical: World = p.read_record("worlds", &before.world_ref).unwrap();
    assert_eq!(serde_json::to_vec(&historical).unwrap(), bytes);
    drop(p);
    let p = Project::open(&root).unwrap();
    assert_eq!(p.view().unwrap().world, next);
    let mut changed = next.clone();
    changed
        .component_types
        .iter_mut()
        .find(|d| d.id == "core.narration-style")
        .unwrap()
        .fields[0]
        .default_value = PropertyValue::Text("Changed builtin".into());
    assert!(changed.validate().is_err());
    let mut missing = next;
    missing.component_types.retain(|d| d.id != "core.holder");
    assert!(missing.validate().is_err());
}
#[test]
fn narration_fields_use_generic_validation_and_task_scoped_prompt_sections() {
    let mut world = World::default();
    let node = profile(&mut world);
    world.validate().unwrap();
    let settings = LlmConnection::default();
    for task in [
        TextPromptTask::World,
        TextPromptTask::Passage,
        TextPromptTask::Wording,
        TextPromptTask::Effects,
        TextPromptTask::Memory,
        TextPromptTask::ImagePrompt,
    ] {
        let preview = preview_ai_context(&world, task, &settings).unwrap();
        if matches!(task, TextPromptTask::Passage | TextPromptTask::Wording) {
            assert!(preview
                .system
                .contains("Narration style / Narrative person: Third person"));
            assert!(
                preview.system.contains("Past tense")
                    && preview.system.contains("concrete sensory details")
            );
            assert_eq!(preview.sections.len(), 1);
            assert!(preview.references.is_empty());
        } else {
            assert!(preview.sections.is_empty());
        }
    }
    let source = world.nodes.iter_mut().find(|n| n.id == node).unwrap();
    source
        .components
        .get_mut("core.narration-style")
        .unwrap()
        .insert(
            "instructions".into(),
            PropertyValue::Text(
                "Keep the viewpoint close.\nAvoid explaining emotions twice.".into(),
            ),
        );
    world.validate().unwrap();
    assert!(
        resolve_ai_context(&world, TextPromptTask::Passage).unwrap()[0]
            .content
            .contains("\nAvoid explaining")
    );
    let mut invalid = world.clone();
    invalid.nodes[0]
        .components
        .get_mut("core.narration-style")
        .unwrap()
        .insert("instructions".into(), PropertyValue::Boolean(true));
    assert!(invalid.validate().is_err());
    invalid.nodes[0]
        .components
        .get_mut("core.narration-style")
        .unwrap()
        .insert(
            "instructions".into(),
            PropertyValue::Text("x".repeat(16001)),
        );
    assert!(invalid.validate().is_err());
    world.ai_context[0].enabled = false;
    assert!(resolve_ai_context(&world, TextPromptTask::Passage)
        .unwrap()
        .is_empty());
}
