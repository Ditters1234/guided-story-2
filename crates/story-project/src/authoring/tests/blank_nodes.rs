use super::*;
use crate::ai_context::tests::note;

fn blank_node(world: &World, type_id: &str, name: &str) -> WorldNode {
    let preset = world.node_types.iter().find(|t| t.id == type_id).unwrap();
    WorldNode {
        id: id(),
        type_id: type_id.into(),
        name: name.into(),
        description: String::new(),
        visual_description: String::new(),
        components: preset
            .components
            .iter()
            .map(|id| {
                let def = world.component_types.iter().find(|d| &d.id == id).unwrap();
                (
                    id.clone(),
                    def.fields
                        .iter()
                        .map(|f| (f.key.clone(), f.default_value.clone()))
                        .collect(),
                )
            })
            .collect(),
    }
}
fn revise(node: &WorldNode) -> AuthoringRequest {
    AuthoringRequest {
        mode: AuthoringMode::Revise,
        direction: String::new(),
        type_id: None,
        bundle: false,
        target_ids: vec![node.id.clone()],
        context_ids: vec![],
        fields: vec![EditableField {
            node_id: node.id.clone(),
            component_id: None,
            key: "description".into(),
        }],
    }
}
#[test]
fn blank_nodes_and_custom_fields_generate_without_directions_and_stay_unapplied_until_review() {
    for (kind, name) in [
        ("core.place", "Glasshouse"),
        ("core.character", "Nora Hale"),
        ("core.item", "Field coat"),
        ("core.note", "Narration style"),
        ("core.narration-style", "Narration style"),
        ("custom.guidance", "World rules"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut world = World::default();
        world.component_types.push(ComponentDefinition {
            id: "custom.voice".into(),
            name: "Voice".into(),
            description: "".into(),
            fields: vec![FieldDefinition {
                key: "voice".into(),
                label: "Voice".into(),
                kind: FieldKind::Text,
                required: false,
                default_value: PropertyValue::Text("".into()),
            }],
        });
        world.node_types.push(NodeTypeDefinition {
            id: "custom.guidance".into(),
            name: "Guidance".into(),
            description: "".into(),
            components: vec!["custom.voice".into()],
        });
        let mut node = blank_node(&world, kind, name);
        if kind == "custom.guidance" {
            node.components.get_mut("custom.voice").unwrap().clear();
        }
        world.nodes.push(node.clone());
        world.ai_context.push(AiContextBinding {
            node_id: node.id.clone(),
            enabled: true,
            role: AiContextRole::Instructions,
            tasks: vec![TextPromptTask::World, TextPromptTask::Passage],
            fields: vec![AiContextField::Description],
            order: 0,
        });
        note(
            &mut world,
            "Other guidance",
            "Keep the setting grounded.",
            AiContextRole::Instructions,
            vec![TextPromptTask::World],
        );
        let project = Project::create(
            &dir.path().join("story"),
            "Blank node fixture".into(),
            world.clone(),
            id(),
        )
        .unwrap();
        let before = project.view().unwrap();
        let mut req = revise(&node);
        if kind == "core.narration-style" {
            req.fields.push(EditableField {
                node_id: node.id.clone(),
                component_id: Some("core.narration-style".into()),
                key: "instructions".into(),
            });
        }
        if kind == "custom.guidance" {
            req.fields.push(EditableField {
                node_id: node.id.clone(),
                component_id: Some("custom.voice".into()),
                key: "voice".into(),
            });
        }
        let key = id();
        project.save_authoring_draft(&key, 0, req.clone()).unwrap();
        let prepared = project
            .start_authoring(&key, 1, before.revision, &settings())
            .unwrap();
        let input: Value = serde_json::from_str(&prepared.user).unwrap();
        assert!(input["request"]["direction"]
            .as_str()
            .unwrap()
            .contains("Existing descriptions are not required"));
        assert_eq!(input["records"][0]["description"], "");
        assert_eq!(input["targetContextBindings"][0]["nodeId"], node.id);
        assert!(prepared.system.contains("Keep the setting grounded."));
        assert_eq!(req.direction, "");
        let output = demo_authoring(&world, &req).unwrap();
        project
            .finish_authoring(&key, Ok(output), false, false)
            .unwrap();
        let review = project.authoring_view(&key, &settings()).unwrap();
        assert!(review.error.is_empty(), "{}", review.error);
        assert!(!review.stale);
        assert!(!review.changes.is_empty());
        assert_eq!(project.view().unwrap().world_ref, before.world_ref);
        assert_eq!(project.view().unwrap().world.nodes[0].description, "");
        let mut malicious: NodeCandidate = serde_json::from_str(&review.candidate_text).unwrap();
        malicious.nodes[0].name = "Unrequested rename".into();
        project
            .save_authoring_candidate(
                &key,
                review.version,
                serde_json::to_string(&malicious).unwrap(),
            )
            .unwrap();
        let blocked = project.authoring_view(&key, &settings()).unwrap();
        assert!(blocked.error.contains("protected"));
        assert!(project
            .apply_authoring(&key, blocked.version, &settings())
            .is_err());
        project
            .save_authoring_candidate(&key, blocked.version, review.candidate_text)
            .unwrap();
        let ready = project.authoring_view(&key, &settings()).unwrap();
        let accepted = project
            .apply_authoring(&key, ready.version, &settings())
            .unwrap();
        assert!(!accepted.commit.replayed);
        let after = project.view().unwrap().world;
        assert!(!after.nodes[0].description.is_empty());
        assert_eq!(after.nodes[0].name, node.name);
        assert_eq!(after.nodes[0].visual_description, "");
        assert_eq!(after.ai_context, world.ai_context);
        assert_eq!(after.nodes[1], world.nodes[1]);
        if kind == "core.narration-style" {
            assert!(
                matches!(&after.nodes[0].components["core.narration-style"]["instructions"],PropertyValue::Text(value) if !value.is_empty())
            );
            assert_eq!(
                after.nodes[0].components["core.narration-style"]["person"],
                node.components["core.narration-style"]["person"]
            );
            assert_eq!(
                after.nodes[0].components["core.narration-style"]["tense"],
                node.components["core.narration-style"]["tense"]
            );
        }
        if kind == "custom.guidance" {
            assert!(
                matches!(&after.nodes[0].components["custom.voice"]["voice"],PropertyValue::Text(value) if !value.is_empty())
            );
        }
        drop(project);
        let reopened = Project::open(&dir.path().join("story")).unwrap();
        assert_eq!(reopened.view().unwrap().world, after);
    }
}
#[test]
fn target_exemption_keeps_other_required_context_and_creation_and_field_guards() {
    let mut world = World::default();
    let target = note(
        &mut world,
        "Narration style",
        "",
        AiContextRole::Instructions,
        vec![TextPromptTask::World],
    );
    let other = note(
        &mut world,
        "Other mandatory guidance",
        "",
        AiContextRole::Reference,
        vec![TextPromptTask::World],
    );
    let target_node = world.nodes.iter().find(|n| n.id == target).unwrap().clone();
    let mut req = revise(&target_node);
    let error = prepare_authoring(&world, &req, &settings()).unwrap_err();
    assert!(error.contains("Other mandatory guidance") && error.contains("no selected content"));
    world
        .nodes
        .iter_mut()
        .find(|n| n.id == other)
        .unwrap()
        .description = "Magic requires contact.".into();
    let prepared = prepare_authoring(&world, &req, &settings()).unwrap();
    assert!(prepared.user.contains("Magic requires contact."));
    assert_eq!(prepared.context.nodes.len(), 1);
    req.direction = "Write concise third-person instructions.".into();
    let directed = prepare_authoring(&world, &req, &settings()).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&directed.user).unwrap()["request"]["direction"],
        req.direction
    );
    req.fields.clear();
    assert!(prepare_authoring(&world, &req, &settings())
        .unwrap_err()
        .contains("fields AI may change"));
    let mut creation = request();
    creation.direction.clear();
    assert!(prepare_authoring(&world, &creation, &settings())
        .unwrap_err()
        .contains("Describe what"));
}
