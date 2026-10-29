use super::*;
fn group() -> AuthoringRequest {
    AuthoringRequest {
        creation_scope: Some(CreationScope::Group),
        group_type_id: Some("core.item".into()),
        type_id: None,
        bundle: true,
        direction: "Winter supplies for Nora Hale.".into(),
        ..request()
    }
}
fn world() -> World {
    let mut w = World::default();
    for (kind, name) in [
        ("core.place", "Old Observatory"),
        ("core.character", "Nora Hale"),
        ("core.item", "Field coat"),
        ("core.item", "Canvas bag"),
        ("core.item", "Travel lantern"),
    ] {
        w.nodes.push(proposed_node(&w, kind, &id(), name));
    }
    w.nodes[1].description = "A botanist with a cautious, dry voice.".into();
    w.nodes[2].components.insert(
        "core.wearable".into(),
        BTreeMap::from([
            ("regions".into(), PropertyValue::Text("torso, arms".into())),
            ("layer".into(), PropertyValue::Number(2.0)),
        ]),
    );
    w.nodes[3]
        .components
        .insert("core.container".into(), BTreeMap::new());
    for (kind, source, target) in [
        ("core.present-at", 1, 0),
        ("core.worn-by", 2, 1),
        ("core.stored-in", 3, 1),
        ("core.stored-in", 4, 3),
    ] {
        w.edges.push(WorldEdge {
            id: id(),
            type_id: kind.into(),
            source_id: w.nodes[source].id.clone(),
            target_id: w.nodes[target].id.clone(),
            description: String::new(),
        });
    }
    for n in 0..18 {
        let mut node = proposed_node(&w, "core.note", &id(), &format!("Archive {n}"));
        node.description = "UNRELATED_FULL_TEXT_MUST_NOT_BE_SENT".into();
        w.nodes.push(node);
    }
    w.validate().unwrap();
    w
}
fn project(world: World) -> (tempfile::TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let p = Project::create(
        &dir.path().join("story"),
        "Existing-world fixture".into(),
        world,
        id(),
    )
    .unwrap();
    (dir, p)
}

#[test]
fn groups_find_structured_goals_without_sending_unrelated_records_or_expanding_permissions() {
    let mut w = world();
    w.nodes[1]
        .components
        .get_mut("detail.motivation")
        .unwrap()
        .insert(
            "wants".into(),
            PropertyValue::Text("Repair the drinkingwater pump".into()),
        );
    let request = AuthoringRequest {
        direction: "Add supplies for repairing the drinkingwater pump".into(),
        group_type_id: None,
        ..group()
    };
    let prepared = prepare_authoring(&w, &request, &settings()).unwrap();
    let discovered = prepared.existing_context.unwrap();
    assert_eq!(discovered.detailed_node_ids, vec![w.nodes[1].id.clone()]);
    assert!(discovered.catalog.len() <= 100);
    assert!(!prepared
        .user
        .contains("UNRELATED_FULL_TEXT_MUST_NOT_BE_SENT"));
    assert!(prepared.user.contains("Repair the drinkingwater pump"));
    assert!(request.context_ids.is_empty());
}
#[test]
fn groups_discover_existing_names_owner_outfit_and_nested_inventory_without_manual_references() {
    let w = world();
    let req = group();
    assert!(req.context_ids.is_empty());
    let prepared = prepare_authoring(&w, &req, &settings()).unwrap();
    let input: Value = serde_json::from_str(&prepared.user).unwrap();
    let context = prepared.existing_context.as_ref().unwrap();
    assert_eq!(context.catalog.len(), w.nodes.len());
    assert_eq!(context.omitted_nodes, 0);
    for node in &w.nodes[..5] {
        assert!(context.detailed_node_ids.contains(&node.id));
    }
    assert!(input["existingWorld"]["details"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["name"] == "Field coat" && n["components"]["core.wearable"]["layer"] == 2.0));
    let lamp = context
        .catalog
        .iter()
        .find(|n| n.id == w.nodes[4].id)
        .unwrap();
    assert_eq!(lamp.placement.as_ref().unwrap().target_id, w.nodes[3].id);
    assert!(
        prepared.user.contains("botanist")
            && !prepared
                .user
                .contains("UNRELATED_FULL_TEXT_MUST_NOT_BE_SENT")
    );
    assert!(prepared.context.nodes.is_empty());
    assert!(prepared.system.contains("Reuse these existing people"));
    let mut custom = w.clone();
    custom.node_types.push(NodeTypeDefinition {
        id: "custom.supply".into(),
        name: "Supply".into(),
        description: String::new(),
        components: vec!["core.item".into()],
    });
    let mut supplies = group();
    supplies.group_type_id = Some("custom.supply".into());
    supplies.direction = "Miscellaneous equipment.".into();
    let context = prepare_authoring(&custom, &supplies, &settings())
        .unwrap()
        .existing_context
        .unwrap();
    assert!(context.detailed_node_ids.contains(&custom.nodes[2].id));
}
#[test]
fn automatic_context_is_bounded_prioritizes_matches_and_preserves_required_material() {
    let mut w = world();
    for n in 0..250 {
        w.nodes.push(proposed_node(
            &w,
            "core.place",
            &id(),
            &format!("Warehouse {n:03}"),
        ));
    }
    let mut req = group();
    req.context_ids = vec![w.nodes[1].id.clone()];
    let (selection, payload) =
        super::super::existing_context::group_context(&w, &req, 2400).unwrap();
    assert!(payload.to_string().len() <= 2400);
    assert!(selection.omitted_nodes > 0);
    assert!(selection.catalog.iter().any(|n| n.id == w.nodes[1].id));
    assert!(selection.detailed_node_ids.contains(&w.nodes[1].id));
    assert!(super::super::existing_context::group_context(&w, &req, 100).is_err());
    let empty = World::default();
    let mut limited = settings();
    let base = prepare_authoring(&empty, &group(), &limited).unwrap();
    limited.context_tokens = base.estimated_input_tokens + limited.max_tokens + 700;
    let prepared = prepare_authoring(&w, &group(), &limited).unwrap();
    assert!(prepared.estimated_input_tokens + limited.max_tokens <= limited.context_tokens);
    assert!(prepared.existing_context.unwrap().omitted_nodes > 0);
    crate::ai_context::tests::note(
        &mut w,
        "Mandatory world rules",
        &"required fact ".repeat(900),
        AiContextRole::Instructions,
        vec![TextPromptTask::World],
    );
    let error = prepare_authoring(&w, &group(), &limited).unwrap_err();
    assert!(error.contains("Mandatory world rules") || error.contains("128 KB"));
}
#[test]
fn indexed_existing_nodes_are_usable_link_targets_but_unexposed_ids_and_existing_rewrites_are_blocked(
) {
    let mut w = world();
    for n in 0..200 {
        w.nodes.push(proposed_node(
            &w,
            "core.place",
            &id(),
            &format!("Warehouse {n:03}"),
        ));
    }
    let (_dir, p) = project(w.clone());
    let key = id();
    let req = group();
    p.save_authoring_draft(&key, 0, req).unwrap();
    let prepared = p.start_authoring(&key, 1, 0, &settings()).unwrap();
    let included = prepared.existing_context.unwrap();
    let omitted = w
        .nodes
        .iter()
        .find(|n| {
            n.type_id == "core.place" && !included.catalog.iter().any(|entry| entry.id == n.id)
        })
        .unwrap();
    let valid = NodeCandidate {
        nodes: vec![proposed_node(
            &w,
            "core.item",
            "new:blanket",
            "Wool blanket",
        )],
        edges: vec![WorldEdge {
            id: "new:carry".into(),
            type_id: "core.stored-in".into(),
            source_id: "new:blanket".into(),
            target_id: w.nodes[1].id.clone(),
            description: String::new(),
        }],
    };
    let mut bad = valid.clone();
    bad.edges[0].target_id = omitted.id.clone();
    p.finish_authoring(&key, Ok(serde_json::to_string(&bad).unwrap()), false, false)
        .unwrap();
    let blocked = p.authoring_view(&key, &settings()).unwrap();
    assert!(!blocked.error.is_empty());
    assert!(p
        .apply_authoring(&key, blocked.version, &settings())
        .is_err());
    let mut rewrite = valid.clone();
    rewrite.nodes.push(w.nodes[1].clone());
    p.save_authoring_candidate(
        &key,
        blocked.version,
        serde_json::to_string(&rewrite).unwrap(),
    )
    .unwrap();
    let blocked = p.authoring_view(&key, &settings()).unwrap();
    assert!(!blocked.error.is_empty());
    p.save_authoring_candidate(
        &key,
        blocked.version,
        serde_json::to_string(&valid).unwrap(),
    )
    .unwrap();
    let ready = p.authoring_view(&key, &settings()).unwrap();
    assert!(ready.error.is_empty() && !ready.stale);
    let before = p.view().unwrap();
    let result = p.apply_authoring(&key, ready.version, &settings()).unwrap();
    assert_eq!(
        &result.commit.project.world.nodes[..w.nodes.len()],
        w.nodes.as_slice()
    );
    assert_eq!(result.commit.project.world.nodes.len(), w.nodes.len() + 1);
    assert_eq!(
        p.read_record::<World>("worlds", &before.world_ref).unwrap(),
        w
    );
}
#[test]
fn captured_discovery_survives_restart_and_backup_and_cannot_be_forged_or_silently_refreshed() {
    let w = world();
    let (dir, p) = project(w.clone());
    let key = id();
    let req = group();
    p.save_authoring_draft(&key, 0, req.clone()).unwrap();
    p.start_authoring(&key, 1, 0, &settings()).unwrap();
    p.finish_authoring(&key, Ok(demo_authoring(&w, &req).unwrap()), false, false)
        .unwrap();
    let ready = p.authoring_view(&key, &settings()).unwrap();
    let captured = ready.existing_context.clone().unwrap();
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    assert_eq!(
        p.authoring_view(&key, &settings())
            .unwrap()
            .existing_context,
        Some(captured.clone())
    );
    let backup = p.export_backup(dir.path(), &id(), 0).unwrap();
    let source = std::path::Path::new(&backup.folder_path);
    let reviewed = inspect_backup(source).unwrap();
    let restored = restore_backup(source, dir.path(), &id(), &reviewed.fingerprint).unwrap();
    let copy = Project::open(std::path::Path::new(&restored.folder_path)).unwrap();
    assert_eq!(
        copy.authoring_view(&key, &settings())
            .unwrap()
            .existing_context,
        Some(captured.clone())
    );
    let mut record = p.read_authoring_record(&key).unwrap();
    record.existing_context.as_mut().unwrap().catalog[0].name = "Forged context".into();
    p.write_authoring(&record).unwrap();
    assert!(p.authoring_view(&key, &settings()).unwrap().stale);
    assert!(p.apply_authoring(&key, ready.version, &settings()).is_err());
    record.existing_context = Some(captured.clone());
    p.write_authoring(&record).unwrap();
    let mut next = w;
    next.nodes[1].name = "Nora Changed".into();
    p.commit(0, id(), "Changed current world".into(), next)
        .unwrap();
    let stale = p.authoring_view(&key, &settings()).unwrap();
    assert!(stale.stale);
    assert_eq!(stale.existing_context, Some(captured));
    assert!(p.apply_authoring(&key, stale.version, &settings()).is_err());
}
#[test]
fn duplicate_names_are_advisory_and_no_addition_groups_do_not_need_invented_records() {
    let w = world();
    let (_dir, p) = project(w.clone());
    let req = group();
    let key = id();
    p.save_authoring_draft(&key, 0, req.clone()).unwrap();
    let prepared = p.start_authoring(&key, 1, 0, &settings()).unwrap();
    assert_eq!(
        prepared.response_schema["properties"]["nodes"]["minItems"],
        0
    );
    p.finish_authoring(
        &key,
        Ok(serde_json::to_string(&NodeCandidate {
            nodes: vec![proposed_node(&w, "core.item", "new:coat", " field   COAT ")],
            edges: vec![],
        })
        .unwrap()),
        false,
        false,
    )
    .unwrap();
    let duplicate = p.authoring_view(&key, &settings()).unwrap();
    assert!(duplicate.error.is_empty());
    assert!(duplicate
        .warnings
        .iter()
        .any(|s| s.contains("already exists")));
    p.save_authoring_candidate(
        &key,
        duplicate.version,
        "{\"nodes\":[],\"edges\":[]}".into(),
    )
    .unwrap();
    let empty = p.authoring_view(&key, &settings()).unwrap();
    assert!(empty.error.is_empty() && empty.changes.is_empty());
    assert!(empty.warnings.iter().any(|s| s.contains("No new nodes")));
    assert!(p.apply_authoring(&key, empty.version, &settings()).is_err());
    assert_eq!(p.view().unwrap().world, w);
    p.save_authoring_candidate(&key, empty.version, duplicate.candidate_text)
        .unwrap();
    let ready = p.authoring_view(&key, &settings()).unwrap();
    assert!(p.apply_authoring(&key, ready.version, &settings()).is_ok());
}
