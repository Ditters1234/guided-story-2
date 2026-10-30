use super::*;
fn typed() -> AuthoringRequest {
    AuthoringRequest {
        type_id: Some("core.character".into()),
        bundle: true,
        ..request()
    }
}
#[test]
fn typed_bundle_has_a_required_root_without_tuple_schema_features() {
    let w = World::default();
    for mode in [
        StructuredOutput::Json,
        StructuredOutput::Prompt,
        StructuredOutput::Schema,
    ] {
        let s = LlmConnection {
            structured_output: mode,
            ..settings()
        };
        let p = prepare_authoring(&w, &typed(), &s).unwrap();
        let input: Value = serde_json::from_str(&p.user).unwrap();
        for schema in std::iter::once(&p.response_schema).chain(input.get("responseSchema")) {
            assert_eq!(
                schema["properties"]["root"]["properties"]["id"]["const"],
                "new:root"
            );
            assert_eq!(
                schema["properties"]["root"]["properties"]["typeId"]["const"],
                "core.character"
            );
            assert_eq!(schema["properties"]["nodes"]["type"], "object");
            assert_eq!(
                schema["properties"]["nodes"]["required"]
                    .as_array()
                    .unwrap()
                    .len(),
                schema["properties"]["nodes"]["properties"]
                    .as_object()
                    .unwrap()
                    .len()
            );
            assert_eq!(
                schema["properties"]["nodes"]["properties"]["core.item"]["maxItems"],
                19
            );
            assert_eq!(
                schema["properties"]["nodes"]["properties"]["core.wearable-item"]["items"]
                    ["properties"]["typeId"]["const"],
                "core.wearable-item"
            );
            assert!(schema["required"]
                .as_array()
                .unwrap()
                .contains(&json!("root")));
            assert!(!schema.to_string().contains("prefixItems"));
        }
        assert_eq!(input["creationGuide"]["typedRoot"]["id"], "new:root");
        assert_eq!(
            input["creationGuide"]["containers"]["mustInclude"]["capabilityFlags"]
                ["core.container"],
            true
        );
    }
}
#[test]
fn rooted_packets_are_flat_reviewable_and_replay_safe_with_raw_output_retained() {
    let (_dir, p) = create();
    let w = p.view().unwrap().world;
    let root = proposed_node(&w, "core.character", "new:root", "Esme Vale");
    let mut bag = proposed_node(&w, "core.item", "new:bag", "Travel satchel");
    bag.components
        .insert("core.container".into(), BTreeMap::new());
    let remedy = proposed_node(&w, "core.item", "new:remedy", "Willow bark");
    let edges = vec![
        WorldEdge {
            id: "new:carry".into(),
            type_id: "core.stored-in".into(),
            source_id: bag.id.clone(),
            target_id: root.id.clone(),
            description: String::new(),
            properties: BTreeMap::new(),
        },
        WorldEdge {
            id: "new:contents".into(),
            type_id: "core.stored-in".into(),
            source_id: remedy.id.clone(),
            target_id: bag.id.clone(),
            description: String::new(),
            properties: BTreeMap::new(),
        },
    ];
    let raw = json!({"root":root,"nodes":[bag,remedy],"edges":edges}).to_string();
    let key = id();
    p.save_authoring_draft(&key, 0, typed()).unwrap();
    p.start_authoring(&key, 1, 0, &settings()).unwrap();
    p.finish_authoring(&key, Ok(raw.clone()), false, false)
        .unwrap();
    let view = p.authoring_view(&key, &settings()).unwrap();
    assert!(view.error.is_empty(), "{}", view.error);
    assert_eq!(view.raw_response, raw);
    let flat: NodeCandidate = serde_json::from_str(&view.candidate_text).unwrap();
    assert_eq!(flat.nodes.len(), 3);
    assert_eq!(flat.nodes[0].id, "new:root");
    assert!(p.view().unwrap().world.nodes.is_empty());
    p.save_authoring_candidate(&key, view.version, raw.clone())
        .unwrap();
    p.save_authoring_candidate(&key, view.version, raw).unwrap();
    let view = p.authoring_view(&key, &settings()).unwrap();
    let result = p.apply_authoring(&key, view.version, &settings()).unwrap();
    assert_eq!(result.commit.project.world.nodes.len(), 3);
    assert!(
        p.apply_authoring(&key, view.version, &settings())
            .unwrap()
            .commit
            .replayed
    );
}
#[test]
fn wrong_or_foreign_roots_are_retained_and_never_applied() {
    let (_dir, p) = create();
    let w = p.view().unwrap().world;
    let wrong = proposed_node(&w, "core.character", "new:wrong", "Esme");
    let raw = json!({"root":wrong,"nodes":[],"edges":[]}).to_string();
    let key = id();
    p.save_authoring_draft(&key, 0, typed()).unwrap();
    p.start_authoring(&key, 1, 0, &settings()).unwrap();
    p.finish_authoring(&key, Ok(raw.clone()), false, false)
        .unwrap();
    let v = p.authoring_view(&key, &settings()).unwrap();
    assert!(v.error.contains("root"));
    assert_eq!(v.raw_response, raw);
    assert!(p.apply_authoring(&key, v.version, &settings()).is_err());
    assert_eq!(p.view().unwrap().revision, 0);
}

#[test]
fn explicit_marker_choices_materialize_only_declared_empty_capabilities() {
    let w = World::default();
    let r = typed();
    let root = proposed_node(&w, "core.character", "new:root", "Esme");
    let mut bag =
        serde_json::to_value(proposed_node(&w, "core.item", "new:bag", "Satchel")).unwrap();
    bag["capabilityFlags"] = json!({"core.container":true});
    let raw = json!({"root":root,"nodes":[bag.clone()],"edges":[]}).to_string();
    let flat = creation::materialize_text(&w, &r, &raw).unwrap();
    let candidate: NodeCandidate = serde_json::from_str(&flat).unwrap();
    assert!(candidate.nodes[1].components.contains_key("core.container"));
    assert!(!flat.contains("capabilityFlags"));
    assert_eq!(creation::materialize_text(&w, &r, &flat).unwrap(), flat);
    for flag in [
        json!({"core.container":"yes"}),
        json!({"core.holder":true}),
        json!({"detail.item":true}),
    ] {
        bag["capabilityFlags"] = flag;
        assert!(creation::materialize_text(
            &w,
            &r,
            &json!({"root":root,"nodes":[bag.clone()],"edges":[]}).to_string()
        )
        .is_err());
    }
    bag["capabilityFlags"] = json!({"core.container":false});
    bag["components"]["core.container"] = json!({});
    assert!(creation::materialize_text(
        &w,
        &r,
        &json!({"root":root,"nodes":[bag],"edges":[]}).to_string()
    )
    .unwrap_err()
    .contains("contradicts"));
}

#[test]
fn provider_marker_schema_requires_a_decision_without_changing_revision_contracts() {
    let w = World::default();
    let s = output_schema(&w, &typed());
    let item = &s["properties"]["nodes"]["properties"]["core.item"]["items"];
    assert!(item["required"]
        .as_array()
        .unwrap()
        .contains(&json!("capabilityFlags")));
    assert_eq!(
        item["properties"]["capabilityFlags"]["properties"]["core.container"]["type"],
        "boolean"
    );
    assert!(item["properties"]["components"]["properties"]
        .get("core.container")
        .is_none());
    assert_eq!(item["properties"]["id"], creation::new_alias_schema());
    for link in s["properties"]["edges"]["items"]["anyOf"]
        .as_array()
        .unwrap()
    {
        assert_eq!(link["properties"]["id"], creation::new_alias_schema());
        assert_eq!(link["properties"]["sourceId"], creation::new_alias_schema());
    }
    let (_dir, p) = create();
    let root = proposed_node(&w, "core.character", "new:root", "Esme");
    let mut bag =
        serde_json::to_value(proposed_node(&w, "core.item", "new:bag", "Satchel")).unwrap();
    bag["capabilityFlags"] = json!({"core.container":true});
    let raw = json!({"root":root,"nodes":{"core.item":[bag]},"edges":[]}).to_string();
    let key = id();
    p.save_authoring_draft(&key, 0, typed()).unwrap();
    p.start_authoring(&key, 1, 0, &settings()).unwrap();
    p.finish_authoring(&key, Ok(raw.clone()), false, false)
        .unwrap();
    let view = p.authoring_view(&key, &settings()).unwrap();
    assert!(view.error.is_empty(), "{}", view.error);
    assert_eq!(view.raw_response, raw);
    assert!(p.view().unwrap().world.nodes.is_empty());
    p.save_authoring_candidate(&key, view.version, raw.clone())
        .unwrap();
    p.save_authoring_candidate(&key, view.version, raw).unwrap();
    let view = p.authoring_view(&key, &settings()).unwrap();
    let applied = p.apply_authoring(&key, view.version, &settings()).unwrap();
    assert!(applied
        .commit
        .project
        .world
        .nodes
        .iter()
        .any(|n| n.components.contains_key("core.container")));
}

#[test]
fn provider_shape_normalization_does_not_clear_a_truncation_gate() {
    let (_dir, p) = create();
    let w = p.view().unwrap().world;
    let root = proposed_node(&w, "core.character", "new:root", "Esme");
    let mut bag =
        serde_json::to_value(proposed_node(&w, "core.item", "new:bag", "Satchel")).unwrap();
    bag["capabilityFlags"] = json!({"core.container":true});
    let raw = json!({"root":root,"nodes":[bag],"edges":[]}).to_string();
    let key = id();
    p.save_authoring_draft(&key, 0, typed()).unwrap();
    p.start_authoring(&key, 1, 0, &settings()).unwrap();
    p.finish_authoring(&key, Ok(raw), false, true).unwrap();
    let view = p.authoring_view(&key, &settings()).unwrap();
    assert!(p.read_authoring(&key).unwrap().incomplete);
    p.save_authoring_candidate(&key, view.version, view.candidate_text.clone())
        .unwrap();
    let view = p.authoring_view(&key, &settings()).unwrap();
    assert!(p.read_authoring(&key).unwrap().incomplete);
    assert!(p.apply_authoring(&key, view.version, &settings()).is_err());
    let mut edited: Value = serde_json::from_str(&view.candidate_text).unwrap();
    edited["nodes"][0]["description"] = json!("Reviewed and completed by the author.");
    p.save_authoring_candidate(&key, view.version, edited.to_string())
        .unwrap();
    let view = p.authoring_view(&key, &settings()).unwrap();
    assert!(!p.read_authoring(&key).unwrap().incomplete);
    assert!(view.error.is_empty(), "{}", view.error);
    p.apply_authoring(&key, view.version, &settings()).unwrap();
}

#[test]
fn related_type_groups_keep_declared_types_and_reject_mismatches() {
    let w = World::default();
    let root = proposed_node(&w, "core.character", "new:root", "Esme");
    let item = proposed_node(&w, "core.item", "new:remedy", "Willow bark");
    assert!(creation::materialize_text(
        &w,
        &typed(),
        &json!({"nodes":{"core.character":[root]},"edges":[]}).to_string()
    )
    .unwrap_err()
    .contains("root"));
    for group in [
        json!({"core.place":[item]}),
        json!({"foreign.type":[]}),
        json!({"core.item":{}}),
    ] {
        let raw = json!({"root":root,"nodes":group,"edges":[]}).to_string();
        assert!(creation::materialize_text(&w, &typed(), &raw).is_err());
    }
}
