use super::*;

fn group() -> AuthoringRequest {
    AuthoringRequest {
        creation_scope: Some(CreationScope::Group),
        direction: "An adult firefighter with a complete work outfit.".into(),
        ..group_request()
    }
}

#[test]
fn connected_groups_receive_explicit_clothing_edges_and_examples_in_all_transports() {
    let w = World::default();
    for transport in [
        StructuredOutput::Json,
        StructuredOutput::Schema,
        StructuredOutput::Prompt,
    ] {
        let config = LlmConnection {
            structured_output: transport,
            ..settings()
        };
        let prepared = prepare_authoring(&w, &group(), &config).unwrap();
        let input: Value = serde_json::from_str(&prepared.user).unwrap();
        assert!(input["creationGuide"]["physicalPlacement"]["clothing"]
            .as_str()
            .unwrap()
            .contains("Every garment"));
        let example = &input["clothingExample"];
        assert_eq!(example["records"].as_array().unwrap().len(), 3);
        assert_eq!(example["edges"].as_array().unwrap().len(), 3);
        for edge in example["edges"].as_array().unwrap() {
            assert_eq!(edge["typeId"], "core.worn-by");
            assert!(example["records"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n["id"] == edge["sourceId"]));
        }
        assert!(prepared.estimated_input_tokens + config.max_tokens <= config.context_tokens);
    }
}

#[test]
fn item_only_groups_link_to_supplied_wearers_without_authorizing_new_people() {
    let mut w = World::default();
    let wearer = proposed_node(&w, "core.character", &id(), "Rowan");
    w.nodes.push(wearer.clone());
    let req = AuthoringRequest {
        group_type_id: Some("core.wearable-item".into()),
        context_ids: vec![wearer.id.clone()],
        ..group()
    };
    let prepared = prepare_authoring(&w, &req, &settings()).unwrap();
    let input: Value = serde_json::from_str(&prepared.user).unwrap();
    assert!(input.get("clothingExample").is_some());
    assert!(input["clothingExample"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .all(|n| n["typeId"] == "core.wearable-item"));
    assert_eq!(
        prepared.response_schema["properties"]["nodes"]["properties"]
            .as_object()
            .unwrap()
            .len(),
        1
    );
    let no_wearer = prepare_authoring(
        &World::default(),
        &AuthoringRequest {
            context_ids: vec![],
            ..req
        },
        &settings(),
    )
    .unwrap();
    assert!(serde_json::from_str::<Value>(&no_wearer.user)
        .unwrap()
        .get("clothingExample")
        .is_none());
}

#[test]
fn review_identifies_each_unlinked_garment_even_with_a_partially_linked_outfit() {
    let w = World::default();
    let person = proposed_node(&w, "core.character", "new:person", "Rowan");
    let shirt = proposed_node(&w, "core.wearable-item", "new:shirt", "Work shirt");
    let boots = proposed_node(&w, "core.wearable-item", "new:boots", "Work boots");
    let mut candidate = NodeCandidate {
        nodes: vec![person, shirt, boots],
        edges: vec![WorldEdge {
            id: "new:shirt-worn".into(),
            type_id: "core.worn-by".into(),
            source_id: "new:shirt".into(),
            target_id: "new:person".into(),
            description: String::new(),
            properties: BTreeMap::new(),
        }],
    };
    let warnings = proposal_warnings(&candidate, &group(), &w);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("Work boots has no wearer or storage link"));
    candidate.edges.push(WorldEdge {
        id: "new:boots-stored".into(),
        type_id: "core.stored-in".into(),
        source_id: "new:boots".into(),
        target_id: "new:person".into(),
        description: String::new(),
        properties: BTreeMap::new(),
    });
    assert!(proposal_warnings(&candidate, &group(), &w).is_empty());
}

#[test]
fn old_group_candidates_remain_repairable_and_linked_clothing_applies_atomically() {
    let (dir, p) = create();
    let w = p.view().unwrap().world;
    let req = group();
    let key = id();
    let prepared = prepare_authoring(&w, &req, &settings()).unwrap();
    let input: Value = serde_json::from_str(&prepared.user).unwrap();
    let garments = input["clothingExample"]["records"]
        .as_array()
        .unwrap()
        .clone();
    let person = proposed_node(&w, "core.character", "new:person", "Rowan");
    let mut nodes = vec![json!(person)];
    nodes.extend(garments);
    let initial = json!({"nodes":nodes,"edges":[]});
    p.save_authoring_draft(&key, 0, req.clone()).unwrap();
    p.start_authoring(&key, 1, 0, &settings()).unwrap();
    p.finish_authoring(&key, Ok(initial.to_string()), false, false)
        .unwrap();
    let mut record = p.read_authoring_record(&key).unwrap();
    record.source_fingerprint = prepared
        .previous_guide_fingerprints
        .first()
        .unwrap()
        .clone();
    p.write_authoring(&record).unwrap();
    let view = p.authoring_view(&key, &settings()).unwrap();
    assert!(!view.stale && view.error.is_empty(), "{}", view.error);
    assert_eq!(view.warnings.len(), 4);
    assert_eq!(p.view().unwrap().revision, 0);
    let mut repaired: NodeCandidate = serde_json::from_str(&view.candidate_text).unwrap();
    repaired.edges = input["clothingExample"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| {
            let mut e: WorldEdge = serde_json::from_value(edge.clone()).unwrap();
            e.target_id = "new:person".into();
            e
        })
        .collect();
    p.save_authoring_candidate(
        &key,
        view.version,
        serde_json::to_string(&repaired).unwrap(),
    )
    .unwrap();
    let ready = p.authoring_view(&key, &settings()).unwrap();
    assert!(ready.warnings.is_empty() && ready.error.is_empty());
    let applied = p.apply_authoring(&key, ready.version, &settings()).unwrap();
    assert_eq!(applied.commit.project.revision, 1);
    assert_eq!(applied.commit.project.world.edges.len(), 3);
    let raw = ready.raw_response;
    assert_eq!(raw, initial.to_string());
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    assert_eq!(p.view().unwrap().world.edges.len(), 3);
    assert_eq!(
        p.authoring_view(&key, &settings()).unwrap().raw_response,
        raw
    );
}

#[test]
#[ignore = "explicit isolated unlinked-clothing desktop review fixture"]
fn clothing_review_desktop_fixture() {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local");
    let folder = base.join("clothing-links-ui-check-20261108");
    let config = base.join("clothing-links-ui-config");
    assert!(!folder.exists() && !config.exists());
    let p = Project::create(
        &folder,
        "Fictional clothing link review".into(),
        World::default(),
        id(),
    )
    .unwrap();
    let w = p.view().unwrap().world;
    let req = group();
    let prepared = prepare_authoring(&w, &req, &settings()).unwrap();
    let input: Value = serde_json::from_str(&prepared.user).unwrap();
    let mut nodes = vec![json!(proposed_node(
        &w,
        "core.character",
        "new:person",
        "Rowan"
    ))];
    nodes.extend(
        input["clothingExample"]["records"]
            .as_array()
            .unwrap()
            .clone(),
    );
    let key = id();
    p.save_authoring_draft(&key, 0, req).unwrap();
    p.start_authoring(&key, 1, 0, &settings()).unwrap();
    p.finish_authoring(
        &key,
        Ok(json!({"nodes":nodes,"edges":[]}).to_string()),
        false,
        false,
    )
    .unwrap();
    crate::save_connection(&config, 0, settings(), crate::ApiKeyUpdate::Keep).unwrap();
    println!("Fictional review fixture: {}", folder.display());
}
