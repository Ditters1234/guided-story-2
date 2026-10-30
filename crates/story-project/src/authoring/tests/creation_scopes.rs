use super::*;
fn group(type_id: Option<&str>) -> AuthoringRequest {
    AuthoringRequest {
        link_target_ids: vec![],
        include_narration: None,
        creation_type_ids: vec![],
        link_fields: vec![],
        creation_scope: Some(CreationScope::Group),
        group_type_id: type_id.map(str::to_string),
        direction: "A compact travel kit with a lantern and blanket.".into(),
        type_id: None,
        bundle: true,
        ..request()
    }
}
#[test]
fn world_group_and_node_contracts_are_distinct_and_group_schemas_honor_custom_type_limits() {
    let mut world = World::default();
    world.node_types.push(NodeTypeDefinition {
        id: "custom.supply".into(),
        name: "Supply".into(),
        description: String::new(),
        components: vec!["core.item".into()],
    });
    let legacy = group_request();
    let mut world_request = legacy.clone();
    world_request.creation_scope = Some(CreationScope::World);
    let prepared_world = prepare_authoring(&world, &world_request, &settings()).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&prepared_world.user).unwrap()["creationIntent"],
        "world"
    );
    assert!(serde_json::from_str::<Value>(&prepared_world.user)
        .unwrap()
        .get("clothingExample")
        .is_some());
    assert_eq!(
        serde_json::from_str::<Value>(
            &prepare_authoring(&world, &legacy, &settings())
                .unwrap()
                .user
        )
        .unwrap()["creationIntent"],
        "world"
    );
    assert_eq!(
        serde_json::from_str::<Value>(
            &prepare_authoring(&world, &request(), &settings())
                .unwrap()
                .user
        )
        .unwrap()["creationIntent"],
        "node"
    );
    for kind in ["core.item", "custom.supply"] {
        let req = group(Some(kind));
        let prepared = prepare_authoring(&world, &req, &settings()).unwrap();
        let input: Value = serde_json::from_str(&prepared.user).unwrap();
        assert_eq!(input["creationIntent"], "group");
        assert_eq!(input["allowedNewTypeIds"], json!([kind]));
        assert!(input.get("clothingExample").is_none());
        let choices = prepared.response_schema["properties"]["nodes"]["items"]["anyOf"]
            .as_array()
            .unwrap();
        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0]["properties"]["typeId"]["const"], kind);
        let demo: NodeCandidate =
            serde_json::from_str(&demo_authoring(&world, &req).unwrap()).unwrap();
        assert_eq!(demo.nodes.len(), 2);
        assert!(demo.nodes.iter().all(|n| n.type_id == kind));
        assert_ne!(prepared.fingerprint, prepared_world.fingerprint);
    }
    let mixed = prepare_authoring(&world, &group(None), &settings()).unwrap();
    assert_eq!(
        mixed.response_schema["properties"]["nodes"]["items"]["anyOf"]
            .as_array()
            .unwrap()
            .len(),
        world.node_types.len()
    );
    assert!(
        mixed.system.contains("creationIntent=group")
            && mixed
                .system
                .contains("Do not invent a person or place just to hold requested items")
    );
}
#[test]
fn item_groups_attach_to_selected_existing_nodes_without_expanding_or_replacing_the_world() {
    let (_dir, p) = create();
    let mut world = p.view().unwrap().world;
    let owner = proposed_node(&world, "core.character", &id(), "Nora Hale");
    let unrelated = proposed_node(&world, "core.place", &id(), "Old Observatory");
    world.nodes.extend([owner.clone(), unrelated.clone()]);
    p.commit(0, id(), "Fictional story".into(), world.clone())
        .unwrap();
    let before = p.view().unwrap();
    let mut req = group(Some("core.item"));
    req.context_ids.push(owner.id.clone());
    let key = id();
    p.save_authoring_draft(&key, 0, req.clone()).unwrap();
    let prepared = p
        .start_authoring(&key, 1, before.revision, &settings())
        .unwrap();
    assert_eq!(prepared.context.nodes, vec![owner.clone()]);
    assert!(prepared
        .existing_context
        .as_ref()
        .unwrap()
        .catalog
        .iter()
        .any(|n| n.name == "Old Observatory"));
    let a = proposed_node(&world, "core.item", "new:lantern", "Brass lantern");
    let b = proposed_node(&world, "core.item", "new:blanket", "Wool blanket");
    let valid = NodeCandidate {
        nodes: vec![a, b],
        edges: vec![WorldEdge {
            properties: Default::default(),
            id: "new:carry".into(),
            type_id: "core.stored-in".into(),
            source_id: "new:lantern".into(),
            target_id: owner.id.clone(),
            description: String::new(),
        }],
    };
    let mut bad = valid.clone();
    bad.nodes.push(proposed_node(
        &world,
        "core.place",
        "new:extra",
        "Unrequested village",
    ));
    p.finish_authoring(&key, Ok(serde_json::to_string(&bad).unwrap()), false, false)
        .unwrap();
    let blocked = p.authoring_view(&key, &settings()).unwrap();
    assert!(blocked.error.contains("only Item nodes"));
    assert!(p
        .apply_authoring(&key, blocked.version, &settings())
        .is_err());
    assert_eq!(p.view().unwrap().world, world);
    p.save_authoring_candidate(
        &key,
        blocked.version,
        serde_json::to_string(&valid).unwrap(),
    )
    .unwrap();
    let ready = p.authoring_view(&key, &settings()).unwrap();
    assert!(ready.error.is_empty() && !ready.stale);
    assert_eq!(p.view().unwrap().world_ref, before.world_ref);
    let accepted = p.apply_authoring(&key, ready.version, &settings()).unwrap();
    assert!(
        p.apply_authoring(&key, accepted.proposal.version, &settings())
            .unwrap()
            .commit
            .replayed
    );
    let after = p.view().unwrap().world;
    assert_eq!(&after.nodes[..2], world.nodes.as_slice());
    assert_eq!(after.nodes.len(), 4);
    assert!(after.nodes[2..].iter().all(|n| n.type_id == "core.item"));
    assert_eq!(after.edges[0].target_id, owner.id);
}
#[test]
fn creation_scope_controls_are_validated_and_participate_in_source_checks() {
    let world = World::default();
    let base = group(Some("core.item"));
    for invalid in [
        AuthoringRequest {
            link_target_ids: vec![],
            include_narration: None,
            creation_type_ids: vec![],
            link_fields: vec![],
            type_id: Some("core.item".into()),
            ..base.clone()
        },
        AuthoringRequest {
            link_target_ids: vec![],
            include_narration: None,
            creation_type_ids: vec![],
            link_fields: vec![],
            creation_scope: None,
            ..base.clone()
        },
        AuthoringRequest {
            link_target_ids: vec![],
            include_narration: None,
            creation_type_ids: vec![],
            link_fields: vec![],
            bundle: false,
            ..base.clone()
        },
        AuthoringRequest {
            link_target_ids: vec![],
            include_narration: None,
            creation_type_ids: vec![],
            link_fields: vec![],
            group_type_id: Some("missing.type".into()),
            ..base.clone()
        },
        AuthoringRequest {
            link_target_ids: vec![],
            include_narration: None,
            creation_type_ids: vec![],
            link_fields: vec![],
            mode: AuthoringMode::Revise,
            ..base.clone()
        },
    ] {
        assert!(prepare_authoring(&world, &invalid, &settings()).is_err());
    }
    let (_dir, p) = create();
    let key = id();
    p.save_authoring_draft(&key, 0, base.clone()).unwrap();
    p.start_authoring(&key, 1, 0, &settings()).unwrap();
    p.finish_authoring(
        &key,
        Ok(demo_authoring(&world, &base).unwrap()),
        false,
        false,
    )
    .unwrap();
    let ready = p.authoring_view(&key, &settings()).unwrap();
    assert!(ready.error.is_empty() && !ready.stale);
    let mut record = p.read_authoring_record(&key).unwrap();
    record.request.group_type_id = None;
    p.write_authoring(&record).unwrap();
    assert!(p.authoring_view(&key, &settings()).unwrap().stale);
    assert!(p.apply_authoring(&key, ready.version, &settings()).is_err());
}
#[test]
fn optional_creation_controls_preserve_legacy_requests_and_survive_reopen_and_backup() {
    let original = request();
    let json = serde_json::to_value(&original).unwrap();
    assert!(json.get("creationScope").is_none() && json.get("groupTypeId").is_none());
    assert_eq!(
        serde_json::from_value::<AuthoringRequest>(json).unwrap(),
        original
    );
    let (dir, p) = create();
    let key = id();
    let group = group(Some("core.item"));
    p.save_authoring_draft(&key, 0, group.clone()).unwrap();
    let root = dir.path().join("story");
    drop(p);
    let p = Project::open(&root).unwrap();
    assert_eq!(p.authoring_view(&key, &settings()).unwrap().request, group);
    let backup = p
        .export_backup(dir.path(), &id(), p.view().unwrap().revision)
        .unwrap();
    let source = std::path::Path::new(&backup.folder_path);
    let reviewed = inspect_backup(source).unwrap();
    let restored = restore_backup(source, dir.path(), &id(), &reviewed.fingerprint).unwrap();
    let copy = Project::open(std::path::Path::new(&restored.folder_path)).unwrap();
    assert_eq!(
        copy.authoring_view(&key, &settings()).unwrap().request,
        group
    );
}
