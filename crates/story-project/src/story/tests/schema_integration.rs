use super::*;

#[test]
fn ownership_does_not_change_physical_possession_or_expand_scene_scope() {
    let mut world = World::default();
    world.nodes = vec![
        node(&world, "core.place", "Workshop"),
        node(&world, "core.character", "Borrower"),
        node(&world, "core.organization", "Cooperative owner"),
        node(&world, "core.item", "Borrowed toolbox"),
    ];
    for (kind, source, target) in [
        ("core.present-at", 1, 0),
        ("core.stored-in", 3, 1),
        ("story.owned-by", 3, 2),
    ] {
        world.edges.push(WorldEdge {
            id: id(),
            type_id: kind.into(),
            source_id: world.nodes[source].id.clone(),
            target_id: world.nodes[target].id.clone(),
            description: String::new(),
            properties: Default::default(),
        });
    }
    let original = world.clone();
    world.validate().unwrap();
    let scoped = scene_world(
        &world,
        &PassagePlan {
            location_id: Some(world.nodes[0].id.clone()),
            viewpoint_id: Some(world.nodes[1].id.clone()),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(scoped.nodes.iter().any(|n| n.id == world.nodes[3].id));
    assert!(!scoped.nodes.iter().any(|n| n.id == world.nodes[2].id));
    assert_eq!(world, original);
    // Semantically unusual endpoints remain native-valid; UI advice must not
    // become a new blocking contract for existing saved project definitions.
    world.edges[2].source_id = world.nodes[2].id.clone();
    world.edges[2].target_id = world.nodes[3].id.clone();
    world.validate().unwrap();
    assert_eq!(world.edges[1], original.edges[1]);
}

#[test]
fn actual_prose_planning_and_wording_prompts_exclude_extra_visual_notes_but_images_keep_them() {
    let (_dir, p) = fixture();
    let current = p.view().unwrap();
    let mut w = current.world;
    let sentinel = "IMAGE_ONLY_SUPPLEMENT_7231";
    w.nodes[1].visual_description = sentinel.into();
    assert!(visual_summary(&w, &w.nodes[1], &[]).contains(sentinel));
    p.commit(current.revision, id(), current.title, w.clone())
        .unwrap();
    let draft = id();
    p.save_passage(&draft, 0, edits(&p, "")).unwrap();
    let planned = p
        .prepare_plan_fill(&draft, 1, 1, PlanFillKind::Scene, &settings())
        .unwrap();
    assert!(!planned.user.contains(sentinel));
    let narrated = p.start_passage(&draft, 1, 1, false, &settings()).unwrap();
    assert!(!narrated.user.contains(sentinel));
    assert!(!narrated.system.contains(sentinel));
    p.finish_passage(
        &draft,
        false,
        Ok("Mara waited beside the window.".into()),
        false,
        false,
        &settings(),
    )
    .unwrap();

    let prose = "Mara waited beside the window.";
    let mut changed = w.nodes[1].clone();
    changed.description = "Waiting beside the window.".into();
    let mut accepted = edits(&p, prose);
    accepted.effects_text = serde_json::to_string(&PassageEffects {
        changes: vec![PassageEffect::PutNode {
            node: changed,
            evidence: prose.into(),
        }],
        ..Default::default()
    })
    .unwrap();
    let passage = accept(&p, accepted);
    let original = p.read_passage(&passage).unwrap();
    assert!(serde_json::to_string(&original.passage.effects)
        .unwrap()
        .contains(sentinel));
    let draft = id();
    p.begin_passage_revision(
        &draft,
        &passage,
        original.record.clone(),
        PassageRevisionKind::Wording,
    )
    .unwrap();
    let mut view = p.passage_view(&draft, &settings()).unwrap();
    view.edits.plan.direction = "Make the wording more concise.".into();
    p.save_passage(&draft, view.version, view.edits).unwrap();
    let view = p.passage_view(&draft, &settings()).unwrap();
    let wording = p
        .start_passage(
            &draft,
            view.version,
            p.view().unwrap().revision,
            false,
            &settings(),
        )
        .unwrap();
    assert!(!wording.user.contains(sentinel));
    assert_eq!(p.read_passage(&passage).unwrap().record, original.record);
    assert!(
        serde_json::to_string(&p.read_passage(&passage).unwrap().passage.effects)
            .unwrap()
            .contains(sentinel)
    );
}

#[test]
fn image_only_binding_selection_blocks_narrative_explicitly_without_changing_saved_bindings() {
    let mut world = World::default();
    let mut n = node(&world, "core.note", "Image reference");
    n.visual_description = "IMAGE_ONLY_BINDING_982".into();
    world.ai_context.push(AiContextBinding {
        node_id: n.id.clone(),
        enabled: true,
        role: AiContextRole::Reference,
        tasks: vec![
            TextPromptTask::Passage,
            TextPromptTask::Wording,
            TextPromptTask::Planning,
            TextPromptTask::ImagePrompt,
        ],
        fields: vec![AiContextField::VisualDescription],
        order: 0,
        image_preparation: true,
    });
    world.nodes.push(n);
    let original = world.clone();
    for task in [
        TextPromptTask::Passage,
        TextPromptTask::Wording,
        TextPromptTask::Planning,
    ] {
        assert!(resolve_ai_context(&world, task)
            .unwrap_err()
            .contains("image-only visualDescription"));
    }
    assert!(
        resolve_ai_context(&world, TextPromptTask::ImagePrompt).unwrap()[0]
            .content
            .contains("IMAGE_ONLY_BINDING_982")
    );
    assert_eq!(world, original);
}

#[test]
fn starters_keep_supporting_nodes_unbound_and_explicit_references_use_existing_scope() {
    for template in ["school", "homestead", "spaceship"] {
        let world = location_template_world(template).unwrap();
        let place = world
            .nodes
            .iter()
            .find(|n| n.components.contains_key("core.place"))
            .unwrap()
            .id
            .clone();
        let environment = world
            .nodes
            .iter()
            .find(|n| n.type_id == "core.environment")
            .unwrap()
            .id
            .clone();
        let time = world
            .nodes
            .iter()
            .find(|n| n.type_id == "core.time")
            .unwrap()
            .id
            .clone();
        assert!(world.ai_context.is_empty());
        let mut plan = PassagePlan {
            location_id: Some(place.clone()),
            ..Default::default()
        };
        assert!(world
            .edges
            .iter()
            .any(|e| e.type_id == "story.conditions-at"
                && e.source_id == environment
                && e.target_id == place));
        let scoped = scene_world(&world, &plan).unwrap();
        assert!(!scoped
            .nodes
            .iter()
            .any(|n| n.id == environment || n.id == time));
        plan.reference_ids = vec![environment.clone(), time.clone()];
        let scoped = scene_world(&world, &plan).unwrap();
        assert!(scoped.nodes.iter().any(|n| n.id == environment));
        assert!(scoped.nodes.iter().any(|n| n.id == time));
        assert!(scoped
            .edges
            .iter()
            .any(|e| e.type_id == "story.conditions-at"));
    }
}

#[test]
fn narrative_relationship_guidance_is_scoped_directional_and_does_not_acquire_knowledge() {
    let mut world = World::default();
    world.nodes = vec![
        node(&world, "core.place", "Lookout"),
        node(&world, "core.place", "Courtyard"),
        node(&world, "core.environment", "Conditions"),
    ];
    for (kind, source, target) in [
        ("story.route-to", 0, 1),
        ("story.view-of", 0, 1),
        ("story.can-hear-from", 0, 1),
        ("story.owned-by", 0, 1),
        ("story.conditions-at", 2, 1),
    ] {
        world.edges.push(WorldEdge {
            id: id(),
            type_id: kind.into(),
            source_id: world.nodes[source].id.clone(),
            target_id: world.nodes[target].id.clone(),
            description: String::new(),
            properties: if kind == "story.route-to" {
                BTreeMap::from([("direction".into(), PropertyValue::Text("one-way".into()))])
            } else {
                Default::default()
            },
        });
    }
    let original = world.clone();
    let mut input = json!({});
    crate::knowledge::project_records(&world, &mut input, false);
    let guidance = input["relationshipGuidance"].as_array().unwrap();
    assert!(guidance
        .iter()
        .find(|v| v["typeId"] == "story.route-to")
        .unwrap()["fields"][0]["help"]
        .as_str()
        .unwrap()
        .contains("source-to-target"));
    for kind in ["story.view-of", "story.can-hear-from", "story.owned-by"] {
        let definition = world.relation_types.iter().find(|r| r.id == kind).unwrap();
        assert_eq!(
            guidance.iter().find(|g| g["typeId"] == kind).unwrap()["meaning"],
            definition.description
        );
    }
    assert!(!guidance.iter().any(|g| g["typeId"] == "story.caused-by"));
    assert_eq!(input["links"][0]["sourceId"], world.nodes[0].id);
    assert_eq!(input["links"][0]["targetId"], world.nodes[1].id);
    assert_eq!(world, original);
    assert!(world.edges.iter().all(|e| e.type_id != "core.holds-claim"));
}
