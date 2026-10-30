use super::*;
fn fixture() -> (tempfile::TempDir, Project, AuthoringRequest, WorldEdge) {
    let (dir, p) = create();
    let mut w = p.view().unwrap().world;
    let a = proposed_node(&w, "core.character", &id(), "Nora");
    let b = proposed_node(&w, "core.character", &id(), "Elias");
    let edge = WorldEdge {
        id: id(),
        type_id: "story.feels-toward".into(),
        source_id: a.id.clone(),
        target_id: b.id.clone(),
        description: "Old notes".into(),
        properties: BTreeMap::from([("attitude".into(), PropertyValue::Text("Trusting".into()))]),
    };
    w.nodes = vec![a, b];
    w.edges = vec![edge.clone()];
    p.commit(0, id(), "Fictional relationship".into(), w)
        .unwrap();
    let request = AuthoringRequest {
        mode: AuthoringMode::Revise,
        type_id: None,
        direction: String::new(),
        link_target_ids: vec![edge.id.clone()],
        link_fields: vec![EditableLinkField {
            link_id: edge.id.clone(),
            property: Some("tension".into()),
        }],
        ..request()
    };
    (dir, p, request, edge)
}
fn submit(p: &Project, request: AuthoringRequest, candidate: NodeCandidate) -> AuthoringView {
    let rid = id();
    p.save_authoring_draft(&rid, 0, request).unwrap();
    p.start_authoring(&rid, 1, p.view().unwrap().revision, &settings())
        .unwrap();
    p.finish_authoring(
        &rid,
        Ok(serde_json::to_string(&candidate).unwrap()),
        false,
        false,
    )
    .unwrap();
    p.authoring_view(&rid, &settings()).unwrap()
}
#[test]
fn link_only_revisions_fill_empty_fields_review_apply_and_replay() {
    let (_dir, p, req, mut edge) = fixture();
    let world = p.view().unwrap().world;
    let prepared = prepare_authoring(&world, &req, &settings()).unwrap();
    assert_eq!(
        prepared.response_schema["properties"]["nodes"]["maxItems"],
        0
    );
    let input: Value = serde_json::from_str(&prepared.user).unwrap();
    assert_eq!(input["records"].as_array().unwrap().len(), 2);
    edge.properties.insert(
        "tension".into(),
        PropertyValue::Text("Fear of disappointing Elias".into()),
    );
    let view = submit(
        &p,
        req,
        NodeCandidate {
            nodes: vec![],
            edges: vec![edge.clone()],
        },
    );
    assert!(view.error.is_empty(), "{}", view.error);
    assert!(!view.stale);
    assert_eq!(
        p.view().unwrap().world.edges[0].properties.get("tension"),
        None
    );
    let applied = p
        .apply_authoring(&view.id, view.version, &settings())
        .unwrap();
    assert_eq!(applied.commit.project.world.edges[0], edge);
    let replay = p
        .apply_authoring(&view.id, view.version, &settings())
        .unwrap();
    assert_eq!(
        replay.commit.project.revision,
        applied.commit.project.revision
    );
}
#[test]
fn relationship_revision_protects_unchecked_fields_endpoints_type_and_target_set() {
    let (_dir, p, req, edge) = fixture();
    for case in 0..5 {
        let mut changed = edge.clone();
        match case {
            0 => changed
                .properties
                .insert("attitude".into(), PropertyValue::Text("Hostile".into())),
            1 => {
                changed.description = "Changed notes".into();
                None
            }
            2 => {
                std::mem::swap(&mut changed.source_id, &mut changed.target_id);
                None
            }
            3 => {
                changed.type_id = "core.related".into();
                None
            }
            _ => {
                changed.id = id();
                None
            }
        };
        let view = submit(
            &p,
            req.clone(),
            NodeCandidate {
                nodes: vec![],
                edges: vec![changed],
            },
        );
        assert!(!view.error.is_empty(), "case {case}");
        assert!(p
            .apply_authoring(&view.id, view.version, &settings())
            .is_err());
    }
    assert_eq!(p.view().unwrap().world.edges[0], edge);
}
#[test]
fn relationship_context_changes_stale_results_unrelated_nodes_merge() {
    let (_dir, p, req, _) = fixture();
    let rid = generated(&p, req.clone());
    let mut current = p.view().unwrap();
    let extra = proposed_node(&current.world, "core.place", &id(), "Unrelated room");
    current.world.nodes.push(extra.clone());
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let view = p.authoring_view(&rid, &settings()).unwrap();
    assert!(!view.stale);
    p.apply_authoring(&rid, view.version, &settings()).unwrap();
    assert!(p.view().unwrap().world.nodes.contains(&extra));
    let rid = generated(&p, req);
    let mut current = p.view().unwrap();
    current.world.nodes[0].description = "Changed source context".into();
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    assert!(p.authoring_view(&rid, &settings()).unwrap().stale);
}
#[test]
fn group_links_with_properties_keep_existing_world_read_only_and_resolve_aliases() {
    let (_dir, p, _, old) = fixture();
    let mut current = p.view().unwrap();
    current
        .world
        .relation_types
        .iter_mut()
        .find(|r| r.id == "story.feels-toward")
        .unwrap()
        .fields
        .push(FieldDefinition {
            key: "witness".into(),
            label: "Witness".into(),
            kind: FieldKind::NodeRef,
            required: false,
            default_value: PropertyValue::Null,
            metadata: None,
        });
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let world = p.view().unwrap().world;
    let new_node = proposed_node(&world, "core.character", "new:visitor", "Visitor");
    let mut edge = WorldEdge {
        id: "new:attitude".into(),
        type_id: "story.feels-toward".into(),
        source_id: new_node.id.clone(),
        target_id: old.target_id.clone(),
        description: String::new(),
        properties: BTreeMap::from([
            ("attitude".into(), PropertyValue::Text("Curious".into())),
            ("witness".into(), PropertyValue::Text("new:visitor".into())),
        ]),
    };
    let req = AuthoringRequest {
        bundle: true,
        type_id: None,
        creation_scope: Some(CreationScope::Group),
        context_ids: vec![old.target_id.clone()],
        ..request()
    };
    let view = submit(
        &p,
        req.clone(),
        NodeCandidate {
            nodes: vec![new_node.clone()],
            edges: vec![edge.clone()],
        },
    );
    assert!(view.error.is_empty(), "{}", view.error);
    let applied = p
        .apply_authoring(&view.id, view.version, &settings())
        .unwrap();
    assert!(applied.commit.project.world.edges.contains(&old));
    let added = applied
        .commit
        .project
        .world
        .edges
        .iter()
        .find(|e| e.id != old.id)
        .unwrap();
    assert_eq!(
        added.properties["witness"],
        PropertyValue::Text(added.source_id.clone())
    );
    edge.source_id = old.source_id;
    let view = submit(
        &p,
        req,
        NodeCandidate {
            nodes: vec![new_node],
            edges: vec![edge],
        },
    );
    assert!(view.error.contains("new source"));
}
#[test]
fn mixed_node_link_revisions_and_scoped_typed_refs_share_protection_pipeline() {
    let (_dir, p, mut req, mut edge) = fixture();
    let mut current = p.view().unwrap();
    let hidden = proposed_node(&current.world, "core.character", &id(), "Hidden witness");
    current.world.nodes.push(hidden.clone());
    current
        .world
        .relation_types
        .iter_mut()
        .find(|r| r.id == edge.type_id)
        .unwrap()
        .fields
        .push(FieldDefinition {
            key: "witness".into(),
            label: "Witness".into(),
            kind: FieldKind::NodeRef,
            required: false,
            default_value: PropertyValue::Null,
            metadata: None,
        });
    p.commit(current.revision, id(), current.title, current.world.clone())
        .unwrap();
    let mut node = current.world.nodes[0].clone();
    node.description = "Revised source notes".into();
    req.target_ids = vec![node.id.clone()];
    req.fields = vec![EditableField {
        node_id: node.id.clone(),
        component_id: None,
        key: "description".into(),
    }];
    req.link_fields.push(EditableLinkField {
        link_id: edge.id.clone(),
        property: Some("witness".into()),
    });
    edge.properties
        .insert("witness".into(), PropertyValue::Text(hidden.id.clone()));
    let bad = submit(
        &p,
        req.clone(),
        NodeCandidate {
            nodes: vec![node.clone()],
            edges: vec![edge.clone()],
        },
    );
    assert!(bad.error.contains("outside the supplied context"));
    req.context_ids.push(hidden.id);
    let good = submit(
        &p,
        req,
        NodeCandidate {
            nodes: vec![node],
            edges: vec![edge],
        },
    );
    assert!(good.error.is_empty(), "{}", good.error);
    p.apply_authoring(&good.id, good.version, &settings())
        .unwrap();
}
