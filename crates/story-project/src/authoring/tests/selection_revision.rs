use super::*;

fn fixture() -> (tempfile::TempDir, Project, AuthoringRequest) {
    let (dir, p) = create();
    let mut w = p.view().unwrap().world;
    let person = proposed_node(&w, "core.character", &id(), "Rowan");
    let mut toolbox = proposed_node(&w, "core.item", &id(), "Toolbox");
    toolbox
        .components
        .insert("core.container".into(), BTreeMap::new());
    let secret = proposed_node(&w, "core.note", &id(), "UNSELECTED PRIVATE RECORD");
    let edge = WorldEdge {
        id: id(),
        type_id: "core.stored-in".into(),
        source_id: toolbox.id.clone(),
        target_id: person.id.clone(),
        description: "Original storage".into(),
        properties: BTreeMap::new(),
    };
    let targets = vec![person.id.clone(), toolbox.id.clone()];
    let mut req = request();
    req.mode = AuthoringMode::Revise;
    req.type_id = None;
    req.bundle = true;
    req.creation_scope = None;
    req.target_ids = targets;
    req.creation_type_ids = vec!["core.item".into(), "core.wearable-item".into()];
    req.link_target_ids = vec![edge.id.clone()];
    req.link_fields = vec![];
    req.direction = "Develop Rowan as a firefighter and add a useful tool to the toolbox.".into();
    w.nodes = vec![person, toolbox, secret];
    w.edges = vec![edge];
    req.fields = authoring_fields(&w, &req.target_ids)
        .into_iter()
        .filter(|f| f.key != "name")
        .collect();
    p.commit(0, id(), "Fictional selection".into(), w).unwrap();
    (dir, p, req)
}
fn candidate(w: &World, req: &AuthoringRequest) -> NodeCandidate {
    let mut nodes = w
        .nodes
        .iter()
        .filter(|n| req.target_ids.contains(&n.id))
        .cloned()
        .collect::<Vec<_>>();
    nodes[0].description =
        "An adult firefighter trained in rescue and equipment maintenance.".into();
    let tool = proposed_node(w, "core.item", "new:halligan", "Halligan bar");
    nodes.push(tool);
    let mut edges = w
        .edges
        .iter()
        .filter(|e| req.link_target_ids.contains(&e.id))
        .cloned()
        .collect::<Vec<_>>();
    edges.push(WorldEdge {
        id: "new:tool-link".into(),
        type_id: "core.stored-in".into(),
        source_id: "new:halligan".into(),
        target_id: req.target_ids[1].clone(),
        description: "Stored in the toolbox".into(),
        properties: BTreeMap::new(),
    });
    NodeCandidate { nodes, edges }
}
fn submit(p: &Project, req: AuthoringRequest, text: String) -> AuthoringView {
    let key = id();
    p.save_authoring_draft(&key, 0, req).unwrap();
    p.start_authoring(&key, 1, p.view().unwrap().revision, &settings())
        .unwrap();
    p.finish_authoring(&key, Ok(text), false, false).unwrap();
    p.authoring_view(&key, &settings()).unwrap()
}

#[test]
fn scoped_selection_revisions_and_related_additions_share_protection_and_atomic_apply() {
    let (dir, p, req) = fixture();
    let source = p.view().unwrap();
    let original = source.world.clone();
    let view = submit(
        &p,
        req.clone(),
        serde_json::to_string(&candidate(&original, &req)).unwrap(),
    );
    assert!(view.error.is_empty(), "{}", view.error);
    assert_eq!(p.view().unwrap().world, original);
    assert_eq!(view.created_node_ids.len(), 1);
    let result = p
        .apply_authoring(&view.id, view.version, &settings())
        .unwrap();
    let world = &result.commit.project.world;
    assert_eq!(world.nodes[0].id, original.nodes[0].id);
    assert_eq!(world.nodes[0].name, "Rowan");
    assert!(world.nodes[0].description.contains("firefighter"));
    assert_eq!(world.nodes[2], original.nodes[2]);
    assert_eq!(world.edges[0], original.edges[0]);
    let tool = world
        .nodes
        .iter()
        .find(|n| n.name == "Halligan bar")
        .unwrap();
    assert!(!tool.id.starts_with("new:"));
    assert!(world
        .edges
        .iter()
        .any(|e| e.source_id == tool.id && e.target_id == original.nodes[1].id));
    let replay = p
        .apply_authoring(&view.id, view.version, &settings())
        .unwrap();
    assert_eq!(
        replay.commit.project.revision,
        result.commit.project.revision
    );
    let bytes = fs::read(
        dir.path()
            .join("story/worlds")
            .join(format!("{}.json", source.world_ref.id)),
    )
    .unwrap();
    assert_eq!(serde_json::from_slice::<World>(&bytes).unwrap(), original);
    drop(p);
    assert_eq!(
        Project::open(&dir.path().join("story"))
            .unwrap()
            .view()
            .unwrap()
            .world,
        *world
    );
}

#[test]
fn all_transports_expose_exact_revisions_and_concrete_optional_addition_contracts() {
    let (_dir, p, req) = fixture();
    let w = p.view().unwrap().world;
    for format in [
        StructuredOutput::Json,
        StructuredOutput::Prompt,
        StructuredOutput::Schema,
    ] {
        let s = LlmConnection {
            structured_output: format,
            ..settings()
        };
        let prepared = prepare_authoring(&w, &req, &s).unwrap();
        let input: Value = serde_json::from_str(&prepared.user).unwrap();
        assert!(!prepared.user.contains("UNSELECTED PRIVATE RECORD"));
        assert_eq!(input["operation"]["kind"], "selectionRevision");
        assert_eq!(
            prepared.response_schema["properties"]["nodes"]["maxItems"],
            2
        );
        let groups = &prepared.response_schema["properties"]["additions"]["properties"]["nodes"]
            ["properties"];
        assert_eq!(groups.as_object().unwrap().len(), 2);
        assert!(groups.get("core.character").is_none());
        assert_eq!(
            groups["core.wearable-item"]["items"]["properties"]["typeId"]["const"],
            "core.wearable-item"
        );
        assert!(prepared.estimated_input_tokens + s.max_tokens <= s.context_tokens);
    }
}

#[test]
fn grouped_provider_additions_flatten_for_review_without_losing_raw_output() {
    let (_dir, p, req) = fixture();
    let w = p.view().unwrap().world;
    let c = candidate(&w, &req);
    let raw=json!({"nodes":c.nodes[..2],"edges":c.edges[..1],"additions":{"nodes":{"core.item":[c.nodes[2]],"core.wearable-item":[]},"edges":[c.edges[1]]}}).to_string();
    let view = submit(&p, req, raw.clone());
    assert!(view.error.is_empty(), "{}", view.error);
    assert_eq!(view.raw_response, raw);
    let flat: NodeCandidate = serde_json::from_str(&view.candidate_text).unwrap();
    assert_eq!(flat.nodes.len(), 3);
    assert_eq!(flat.edges.len(), 2);
}

#[test]
fn forged_scope_types_protected_fields_and_existing_placement_rewrites_are_rejected() {
    let (_dir, p, req) = fixture();
    let w = p.view().unwrap().world;
    for case in 0..6 {
        let mut c = candidate(&w, &req);
        match case {
            0 => c.nodes[0].name = "Unapproved rename".into(),
            1 => c.nodes[0] = w.nodes[2].clone(),
            2 => {
                c.nodes[2] =
                    proposed_node(&w, "core.place", "new:halligan", "Unselected place type")
            }
            3 => c.edges[1].source_id = w.nodes[1].id.clone(),
            4 => c.edges[0].target_id = w.nodes[2].id.clone(),
            _ => c.edges[1].target_id = w.nodes[2].id.clone(),
        }
        let view = submit(&p, req.clone(), serde_json::to_string(&c).unwrap());
        assert!(!view.error.is_empty(), "case {case}");
        assert!(p
            .apply_authoring(&view.id, view.version, &settings())
            .is_err());
        assert_eq!(p.view().unwrap().world, w);
    }
}

#[test]
fn disabled_additions_and_noop_revisions_keep_the_legacy_boundary() {
    let (_dir, p, mut req) = fixture();
    let w = p.view().unwrap().world;
    req.bundle = false;
    req.creation_type_ids.clear();
    let view = submit(
        &p,
        req.clone(),
        serde_json::to_string(&candidate(&w, &req)).unwrap(),
    );
    assert!(!view.error.is_empty());
    let unchanged = NodeCandidate {
        nodes: w.nodes[..2].to_vec(),
        edges: w.edges.clone(),
    };
    let view = submit(&p, req, serde_json::to_string(&unchanged).unwrap());
    assert!(view.error.is_empty(), "{}", view.error);
    assert!(view.changes.is_empty());
    assert!(p
        .apply_authoring(&view.id, view.version, &settings())
        .is_err());
}

#[test]
fn uncertain_apply_reconciles_both_edits_and_additions_as_one_revision_after_restart() {
    for published in [false, true] {
        let (dir, p, req) = fixture();
        let original = p.view().unwrap();
        let view = submit(
            &p,
            req.clone(),
            serde_json::to_string(&candidate(&original.world, &req)).unwrap(),
        );
        let mut record = p.read_authoring(&view.id).unwrap();
        let (world, _) = candidate_world(&original.world, &mut record).unwrap();
        record.apply_attempt = Some(ApplyAttempt {
            expected_revision: original.revision,
            title: original.title.clone(),
            world_ref: p.write_record("worlds", &world).unwrap(),
        });
        record.version += 1;
        p.write_authoring(&record).unwrap();
        if published {
            p.commit(original.revision, view.id.clone(), original.title, world)
                .unwrap();
        }
        drop(p);
        let p = Project::open(&dir.path().join("story")).unwrap();
        let reviewed = p.authoring_view(&view.id, &settings()).unwrap();
        let applied = p
            .apply_authoring(&view.id, reviewed.version, &settings())
            .unwrap();
        assert_eq!(applied.commit.project.revision, 2);
        assert_eq!(applied.commit.project.world.nodes.len(), 4);
        assert!(applied.commit.project.world.nodes[0]
            .description
            .contains("firefighter"));
        assert!(
            p.apply_authoring(&view.id, reviewed.version, &settings())
                .unwrap()
                .commit
                .replayed
        );
    }
}

#[test]
fn relevant_edits_stale_a_mixed_proposal_while_unrelated_edits_merge() {
    for relevant in [false, true] {
        let (_dir, p, req) = fixture();
        let original = p.view().unwrap().world;
        let view = submit(
            &p,
            req.clone(),
            serde_json::to_string(&candidate(&original, &req)).unwrap(),
        );
        let mut changed = original.clone();
        changed.nodes[if relevant { 1 } else { 2 }].description = "Manual later edit".into();
        p.commit(1, id(), "Fictional selection".into(), changed.clone())
            .unwrap();
        let latest = p.authoring_view(&view.id, &settings()).unwrap();
        assert_eq!(latest.stale, relevant);
        let applied = p.apply_authoring(&view.id, latest.version, &settings());
        if relevant {
            assert!(applied.is_err());
            assert_eq!(p.view().unwrap().world, changed);
        } else {
            let world = applied.unwrap().commit.project.world;
            assert_eq!(world.nodes[2], changed.nodes[2]);
            assert_eq!(world.nodes.len(), 4);
        }
    }
}

#[test]
fn checked_reference_fields_can_point_to_scoped_new_nodes_without_leaking_hidden_ids() {
    let (_dir, p, mut req) = fixture();
    let mut world = p.view().unwrap().world;
    world.component_types.push(ComponentDefinition {
        id: "custom.tool-choice".into(),
        name: "Preferred tool".into(),
        description: String::new(),
        fields: vec![FieldDefinition {
            key: "tool".into(),
            label: "Tool".into(),
            kind: FieldKind::NodeRef,
            required: false,
            default_value: PropertyValue::Null,
            metadata: Some(FieldMetadata {
                reference_capability: Some("core.item".into()),
                ..FieldMetadata::default()
            }),
        }],
    });
    world.nodes[1]
        .components
        .insert("custom.tool-choice".into(), BTreeMap::new());
    p.commit(1, id(), "Fictional selection".into(), world.clone())
        .unwrap();
    req.fields = authoring_fields(&world, &req.target_ids)
        .into_iter()
        .filter(|f| f.key != "name")
        .collect();
    let mut c = candidate(&world, &req);
    c.nodes[1]
        .components
        .get_mut("custom.tool-choice")
        .unwrap()
        .insert("tool".into(), PropertyValue::Text("new:halligan".into()));
    let view = submit(&p, req, serde_json::to_string(&c).unwrap());
    assert!(view.error.is_empty(), "{}", view.error);
    let applied = p
        .apply_authoring(&view.id, view.version, &settings())
        .unwrap()
        .commit
        .project
        .world;
    let tool = applied
        .nodes
        .iter()
        .find(|n| n.name == "Halligan bar")
        .unwrap();
    assert_eq!(
        applied.nodes[1].components["custom.tool-choice"]["tool"],
        PropertyValue::Text(tool.id.clone())
    );
}

#[test]
fn formalwear_revision_keeps_wearing_links_and_character_body_facts() {
    let entry: LibraryEntry = serde_json::from_slice(include_bytes!(
        "../../../../../examples/node-library/surveyor-and-belongings.json"
    ))
    .unwrap();
    let source = preview_world_edit(&World::default(), WorldEdit::InsertLibrary { entry })
        .unwrap()
        .world;
    let temp = tempfile::tempdir().unwrap();
    let p = Project::create(
        &temp.path().join("story"),
        "Fictional formalwear".into(),
        source.clone(),
        id(),
    )
    .unwrap();
    let mut req = request();
    req.mode = AuthoringMode::Revise;
    req.type_id = None;
    req.direction =
        "Redesign the selected daywear as formal evening wear, preserving the wearer.".into();
    req.target_ids = source
        .nodes
        .iter()
        .filter(|n| n.components.contains_key("core.wearable"))
        .map(|n| n.id.clone())
        .collect();
    req.link_target_ids = source
        .edges
        .iter()
        .filter(|e| e.type_id == "core.worn-by")
        .map(|e| e.id.clone())
        .collect();
    req.fields = req
        .target_ids
        .iter()
        .map(|id| EditableField {
            node_id: id.clone(),
            component_id: None,
            key: "visualDescription".into(),
        })
        .collect();
    let mut nodes = source
        .nodes
        .iter()
        .filter(|n| req.target_ids.contains(&n.id))
        .cloned()
        .collect::<Vec<_>>();
    for node in &mut nodes {
        node.visual_description = format!("Tailored formalwear version of {}", node.name);
    }
    let edges = source
        .edges
        .iter()
        .filter(|e| req.link_target_ids.contains(&e.id))
        .cloned()
        .collect();
    let view = submit(
        &p,
        req,
        serde_json::to_string(&NodeCandidate { nodes, edges }).unwrap(),
    );
    assert!(view.error.is_empty(), "{}", view.error);
    let applied = p
        .apply_authoring(&view.id, view.version, &settings())
        .unwrap()
        .commit
        .project
        .world;
    assert_eq!(applied.nodes[0], source.nodes[0]);
    assert_eq!(applied.nodes[1], source.nodes[1]);
    assert_eq!(applied.edges, source.edges);
    assert_eq!(applied.nodes.len(), source.nodes.len());
}
