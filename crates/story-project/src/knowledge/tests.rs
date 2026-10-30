use super::*;
fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn text() -> LlmConnection {
    LlmConnection {
        provider: TextProvider::Demo,
        ..Default::default()
    }
}
fn node(w: &World, kind: &str, name: &str) -> WorldNode {
    let p = w.node_types.iter().find(|d| d.id == kind).unwrap();
    WorldNode {
        id: id(),
        type_id: kind.into(),
        name: name.into(),
        description: String::new(),
        visual_description: String::new(),
        components: p
            .components
            .iter()
            .map(|id| {
                let d = w.component_types.iter().find(|d| &d.id == id).unwrap();
                (id.clone(), default_properties(d))
            })
            .collect(),
    }
}
fn claim(w: &mut World, name: &str, access: &str, status: &str) -> String {
    let mut n = node(w, "core.claim", name);
    let props = n.components.get_mut("core.claim").unwrap();
    props.insert(
        "proposition".into(),
        PropertyValue::Text(format!("{name} proposition.")),
    );
    props.insert("disclosure".into(), PropertyValue::Text(access.into()));
    props.insert("status".into(), PropertyValue::Text(status.into()));
    let key = n.id.clone();
    w.nodes.push(n);
    key
}
fn held(w: &mut World, claim: &str, holder: &str, stance: &str) -> String {
    let d = w
        .relation_types
        .iter()
        .find(|d| d.id == "core.holds-claim")
        .unwrap();
    let mut props = default_field_values(&d.fields);
    props.insert("stance".into(), PropertyValue::Text(stance.into()));
    let key = id();
    w.edges.push(WorldEdge {
        id: key.clone(),
        type_id: d.id.clone(),
        source_id: claim.into(),
        target_id: holder.into(),
        description: String::new(),
        properties: props,
    });
    key
}
fn fixture() -> (tempfile::TempDir, Project, PassagePlan) {
    let dir = tempfile::tempdir().unwrap();
    let mut w = World::default();
    for (kind, name) in [
        ("core.character", "Mara"),
        ("core.character", "Rowan"),
        ("core.place", "Workshop"),
    ] {
        w.nodes.push(node(&w, kind, name));
    }
    let plan = PassagePlan {
        viewpoint_id: Some(w.nodes[0].id.clone()),
        participant_ids: vec![w.nodes[1].id.clone()],
        location_id: Some(w.nodes[2].id.clone()),
        ..Default::default()
    };
    let p = Project::create(
        &dir.path().join("story"),
        "Fictional claim fixture".into(),
        w,
        id(),
    )
    .unwrap();
    (dir, p, plan)
}
fn commit(p: &Project, w: World) {
    let v = p.view().unwrap();
    p.commit(v.revision, id(), v.title, w).unwrap();
}
fn save(p: &Project, plan: &PassagePlan, prose: &str) -> String {
    let key = id();
    p.save_passage(
        &key,
        0,
        PassageEdits {
            plan: plan.clone(),
            prose: prose.into(),
            effects_text: empty_effects(),
            close_chapter: false,
            approved_effects: vec![],
        },
    )
    .unwrap();
    key
}
fn accept(p: &Project, plan: &PassagePlan) -> String {
    let key = save(p, plan, "Mara waited beside the workshop window.");
    p.validate_passage(&key, 1, &text()).unwrap();
    let version = p.passage_view(&key, &text()).unwrap().version;
    p.accept_passage(&key, version, &text()).unwrap();
    key
}

#[test]
fn false_belief_reader_disclosure_and_author_information_stay_separate() {
    let (_dir, p, mut plan) = fixture();
    let mut w = p.view().unwrap().world;
    let false_claim = claim(&mut w, "The pump is cursed", "narrator", "disproved");
    held(
        &mut w,
        &false_claim,
        plan.viewpoint_id.as_ref().unwrap(),
        "believes",
    );
    let reader = claim(&mut w, "A spare pump exists", "revealed", "established");
    let unknown = claim(&mut w, "Rowan changed the valve", "narrator", "unconfirmed");
    held(&mut w, &unknown, &plan.participant_ids[0], "doubts");
    let hidden = claim(&mut w, "WITHHELD_MARKER", "withheld", "established");
    held(
        &mut w,
        &hidden,
        plan.viewpoint_id.as_ref().unwrap(),
        "believes",
    );
    plan.reference_ids = vec![
        false_claim.clone(),
        reader.clone(),
        unknown.clone(),
        hidden.clone(),
    ];
    let a = preview_knowledge(&w, &plan).unwrap();
    assert_eq!(a.included.len(), 2);
    let belief = a
        .included
        .iter()
        .find(|c| c.node_id == false_claim)
        .unwrap();
    assert_eq!(belief.author_assessment, "disproved");
    assert_eq!(belief.viewpoint_stance.as_deref(), Some("believes"));
    assert!(!belief.reader_disclosed);
    assert!(a
        .included
        .iter()
        .find(|c| c.node_id == reader)
        .unwrap()
        .viewpoint_stance
        .is_none());
    plan.knowledge_perspective = KnowledgePerspective::Author;
    let a = preview_knowledge(&w, &plan).unwrap();
    assert_eq!(a.included.len(), 3);
    assert!(!a.included.iter().any(|c| c.node_id == hidden));
    assert!(a.included.iter().any(|c| c.node_id == unknown));
    assert_eq!(w.nodes.len(), 7);
    assert_eq!(w.edges.len(), 3);
}

#[test]
fn actual_request_excludes_hidden_records_links_citations_and_sends_labelled_claims_once() {
    let (_dir, p, mut plan) = fixture();
    let mut w = p.view().unwrap().world;
    let hidden = claim(&mut w, "WITHHELD_RECORD", "withheld", "established");
    w.nodes.last_mut().unwrap().description = "WITHHELD_DESCRIPTION".into();
    let e = held(
        &mut w,
        &hidden,
        plan.viewpoint_id.as_ref().unwrap(),
        "believes",
    );
    w.edges
        .iter_mut()
        .find(|e2| e2.id == e)
        .unwrap()
        .description = "WITHHELD_LINK".into();
    let false_claim = claim(&mut w, "The pump is cursed", "narrator", "disproved");
    held(
        &mut w,
        &false_claim,
        plan.viewpoint_id.as_ref().unwrap(),
        "believes",
    );
    plan.reference_ids = vec![hidden, false_claim];
    commit(&p, w);
    let key = save(&p, &plan, "");
    let req = p
        .start_passage(&key, 1, p.view().unwrap().revision, false, &text())
        .unwrap();
    assert!(!req.user.contains("WITHHELD"));
    let input: serde_json::Value = serde_json::from_str(&req.user).unwrap();
    assert_eq!(
        input["situatedKnowledge"]["claims"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        req.user.matches("The pump is cursed proposition.").count(),
        1
    );
    assert!(input["records"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["typeId"] != "core.claim"));
    assert!(req
        .system
        .contains("Author assessment is not character knowledge"));
    p.finish_passage(
        &key,
        false,
        Ok("Mara waited beside the workshop window.".into()),
        false,
        false,
        &text(),
    )
    .unwrap();
    let v = p.passage_view(&key, &text()).unwrap();
    let check = p
        .start_passage(&key, v.version, p.view().unwrap().revision, true, &text())
        .unwrap();
    assert!(check.user.contains("WITHHELD_RECORD"));
}

#[test]
fn custom_presets_use_capability_contracts_and_cannot_forge_or_duplicate_stances() {
    let (_dir, p, mut plan) = fixture();
    let mut w = p.view().unwrap().world;
    let c = claim(&mut w, "Faulty valve", "narrator", "unconfirmed");
    let mut custom = w
        .node_types
        .iter()
        .find(|t| t.id == "core.claim")
        .unwrap()
        .clone();
    custom.id = "custom.rumour".into();
    w.node_types.push(custom);
    w.nodes.last_mut().unwrap().type_id = "custom.rumour".into();
    held(&mut w, &c, plan.viewpoint_id.as_ref().unwrap(), "doubts");
    plan.reference_ids.push(c.clone());
    assert!(w.validate().is_ok());
    assert_eq!(
        preview_knowledge(&w, &plan).unwrap().included[0]
            .viewpoint_stance
            .as_deref(),
        Some("doubts")
    );
    let old = w.clone();
    held(&mut w, &c, plan.viewpoint_id.as_ref().unwrap(), "rejects");
    assert!(w.validate().unwrap_err().contains("one stance"));
    w = old.clone();
    w.relation_types
        .iter_mut()
        .find(|d| d.id == "core.holds-claim")
        .unwrap()
        .physical_placement = true;
    assert!(w.validate().unwrap_err().contains("contract"));
    w = old;
    w.component_types
        .iter_mut()
        .find(|d| d.id == "core.claim")
        .unwrap()
        .fields[1]
        .default_value = PropertyValue::Text("true".into());
    assert!(w.validate().unwrap_err().contains("contract"));
}

#[test]
fn required_bindings_cannot_bypass_the_scene_perspective() {
    let (_dir, p, mut plan) = fixture();
    let mut w = p.view().unwrap().world;
    let c = claim(&mut w, "Unknown to Mara", "narrator", "established");
    plan.reference_ids.push(c.clone());
    w.ai_context.push(AiContextBinding {
        node_id: c.clone(),
        enabled: true,
        role: AiContextRole::Instructions,
        tasks: vec![TextPromptTask::Passage],
        fields: vec![AiContextField::Components],
        order: 0,
        image_preparation: false,
    });
    commit(&p, w);
    let key = save(&p, &plan, "");
    assert!(p
        .start_passage(&key, 1, p.view().unwrap().revision, false, &text())
        .err()
        .unwrap()
        .contains("Required AI context"));
    assert_eq!(
        p.passage_view(&key, &text()).unwrap().stage,
        PassageStage::Draft
    );
    plan.knowledge_perspective = KnowledgePerspective::Author;
    let key = save(&p, &plan, "");
    assert!(p
        .start_passage(&key, 1, p.view().unwrap().revision, false, &text())
        .is_ok());
}

#[test]
fn relevant_stance_edits_stale_checked_drafts_but_unrelated_claims_do_not() {
    let (_dir, p, plan) = fixture();
    let mut w = p.view().unwrap().world;
    let c = claim(&mut w, "Pump defect", "narrator", "unconfirmed");
    let edge = held(&mut w, &c, plan.viewpoint_id.as_ref().unwrap(), "believes");
    commit(&p, w);
    let key = save(&p, &plan, "Mara waited beside the workshop window.");
    p.validate_passage(&key, 1, &text()).unwrap();
    assert!(p.passage_view(&key, &text()).unwrap().can_accept);
    let mut w = p.view().unwrap().world;
    claim(
        &mut w,
        "Distant unrelated rumour",
        "withheld",
        "unconfirmed",
    );
    commit(&p, w);
    assert!(p.passage_view(&key, &text()).unwrap().can_accept);
    let mut w = p.view().unwrap().world;
    w.edges
        .iter_mut()
        .find(|e| e.id == edge)
        .unwrap()
        .properties
        .insert("stance".into(), PropertyValue::Text("rejects".into()));
    commit(&p, w);
    assert!(!p.passage_view(&key, &text()).unwrap().can_accept);
}

#[test]
fn snapshots_forks_and_backup_restore_retain_their_own_claim_boundaries() {
    let (dir, p, mut plan) = fixture();
    let mut w = p.view().unwrap().world;
    let c = claim(&mut w, "Valve backstory", "withheld", "unconfirmed");
    plan.reference_ids.push(c.clone());
    commit(&p, w);
    let passage = accept(&p, &plan);
    let mut w = p.view().unwrap().world;
    w.nodes
        .iter_mut()
        .find(|n| n.id == c)
        .unwrap()
        .components
        .get_mut("core.claim")
        .unwrap()
        .insert("disclosure".into(), PropertyValue::Text("revealed".into()));
    held(&mut w, &c, plan.viewpoint_id.as_ref().unwrap(), "believes");
    commit(&p, w);
    assert_eq!(
        preview_knowledge(&p.read_passage(&passage).unwrap().after, &plan)
            .unwrap()
            .included
            .len(),
        0
    );
    let fork = p
        .fork_after_passage(&passage, dir.path(), "Earlier knowledge".into())
        .unwrap();
    assert!(preview_knowledge(&fork.view().unwrap().world, &plan)
        .unwrap()
        .included
        .is_empty());
    let parent = dir.path().join("exports");
    std::fs::create_dir(&parent).unwrap();
    let backup = p
        .export_backup(&parent, &id(), p.view().unwrap().revision)
        .unwrap();
    let checked = inspect_backup(std::path::Path::new(&backup.folder_path)).unwrap();
    let target = dir.path().join("restore");
    std::fs::create_dir(&target).unwrap();
    let r = restore_backup(
        std::path::Path::new(&backup.folder_path),
        &target,
        &id(),
        &checked.fingerprint,
    )
    .unwrap();
    let r = Project::open(std::path::Path::new(&r.folder_path)).unwrap();
    assert_eq!(
        preview_knowledge(&r.view().unwrap().world, &plan)
            .unwrap()
            .included
            .len(),
        1
    );
    drop(p);
    crate::store::publish_new_path(&dir.path().join("story"), &dir.path().join("unavailable"))
        .unwrap();
    assert!(fork.read_passage(&passage).is_ok());
    assert!(r.read_passage(&passage).is_ok());
}

#[test]
fn default_plan_serialization_and_optional_old_schemas_stay_unchanged() {
    let mut w = World::default();
    w.node_types.retain(|t| t.id != "core.claim");
    w.component_types.retain(|t| t.id != "core.claim");
    w.relation_types.retain(|t| t.id != "core.holds-claim");
    assert!(w.validate().is_ok());
    assert!(serde_json::to_value(PassagePlan::default())
        .unwrap()
        .get("knowledgePerspective")
        .is_none());
    let mut p = PassagePlan::default();
    p.knowledge_perspective = KnowledgePerspective::Author;
    assert_eq!(
        serde_json::to_value(p).unwrap()["knowledgePerspective"],
        "author"
    );
}

#[test]
fn cited_history_and_typed_memory_cannot_reintroduce_an_excluded_claim() {
    let (_dir, p, mut plan) = fixture();
    let mut w = p.view().unwrap().world;
    let c = claim(&mut w, "WITHHELD_CLAIM", "withheld", "unconfirmed");
    plan.reference_ids.push(c.clone());
    commit(&p, w);
    let key = save(&p, &plan, "Mara waited beside the workshop window.");
    let mut edits = p.passage_view(&key, &text()).unwrap().edits;
    edits.effects_text = serde_json::to_string(&PassageEffects {
        notes: vec![ContinuityNote {
            text: "WITHHELD_MEMORY".into(),
            evidence: edits.prose.clone(),
            node_ids: vec![c.clone()],
        }],
        ..Default::default()
    })
    .unwrap();
    p.save_passage(&key, 1, edits).unwrap();
    p.validate_passage(&key, 2, &text()).unwrap();
    p.approve_passage_for_test(&key, &text()).unwrap();
    p.accept_passage(
        &key,
        p.passage_view(&key, &text()).unwrap().version,
        &text(),
    )
    .unwrap();
    let reading = p.read_passage(&key).unwrap();
    let world = p.view().unwrap().world;
    let changed = p
        .preview_source_edit(
            &world,
            SourceEdit::AddPassage {
                node_id: c,
                passage_id: key.clone(),
                version: reading.record,
                quote: reading.passage.prose,
                effect_index: None,
            },
        )
        .unwrap();
    commit(&p, changed.world);
    let draft = save(&p, &plan, "");
    let req = p
        .start_passage(&draft, 1, p.view().unwrap().revision, false, &text())
        .unwrap();
    assert!(!req.user.contains("Mara waited beside the workshop window."));
    let input: serde_json::Value = serde_json::from_str(&req.user).unwrap();
    assert_eq!(input["situatedKnowledge"]["withheldRecentPassages"], 1);
    accept(&p, &plan);
    accept(&p, &plan);
    let draft = save(&p, &plan, "");
    let req = p
        .start_passage(&draft, 1, p.view().unwrap().revision, false, &text())
        .unwrap();
    assert!(!req.user.contains("WITHHELD_MEMORY"));
    let input: serde_json::Value = serde_json::from_str(&req.user).unwrap();
    assert_eq!(input["earlierMemory"]["coverage"]["omittedPoints"], 1);
}

#[test]
fn claim_retrieval_is_bounded_and_explicit_references_win() {
    let (_dir, p, mut plan) = fixture();
    let mut w = p.view().unwrap().world;
    let mut last = String::new();
    for i in 0..25 {
        last = claim(
            &mut w,
            &format!("Valve claim {i}"),
            "revealed",
            "unconfirmed",
        );
        w.nodes
            .last_mut()
            .unwrap()
            .components
            .get_mut("core.claim")
            .unwrap()
            .insert(
                "subjects".into(),
                PropertyValue::NodeRefs(vec![plan.location_id.clone().unwrap()]),
            );
    }
    plan.reference_ids.push(last.clone());
    let preview = preview_knowledge(&w, &plan).unwrap();
    assert_eq!(preview.omitted_by_limit, 5);
    assert_eq!(preview.included.len(), 20);
    assert_eq!(preview.included[0].node_id, last);
    commit(&p, w);
    let draft = save(&p, &plan, "");
    let req = p
        .start_passage(&draft, 1, p.view().unwrap().revision, false, &text())
        .unwrap();
    let input: serde_json::Value = serde_json::from_str(&req.user).unwrap();
    assert_eq!(
        input["situatedKnowledge"]["claims"]
            .as_array()
            .unwrap()
            .len(),
        20
    );
}

#[test]
fn wording_never_silently_trims_a_required_source_containing_excluded_claims() {
    let (_dir, p, mut plan) = fixture();
    let mut w = p.view().unwrap().world;
    let c = claim(&mut w, "Hidden cause", "withheld", "established");
    plan.reference_ids.push(c.clone());
    commit(&p, w);
    let key = save(&p, &plan, "Mara waited beside the workshop window.");
    let mut edits = p.passage_view(&key, &text()).unwrap().edits;
    edits.effects_text = serde_json::to_string(&PassageEffects {
        notes: vec![ContinuityNote {
            text: "Author note about the hidden cause.".into(),
            evidence: edits.prose.clone(),
            node_ids: vec![c],
        }],
        ..Default::default()
    })
    .unwrap();
    p.save_passage(&key, 1, edits).unwrap();
    p.validate_passage(&key, 2, &text()).unwrap();
    p.approve_passage_for_test(&key, &text()).unwrap();
    p.accept_passage(
        &key,
        p.passage_view(&key, &text()).unwrap().version,
        &text(),
    )
    .unwrap();
    let accepted = p.read_passage(&key).unwrap();
    let revision = id();
    p.begin_passage_revision(
        &revision,
        &key,
        accepted.record.clone(),
        PassageRevisionKind::Wording,
    )
    .unwrap();
    let v = p.passage_view(&revision, &text()).unwrap();
    let mut edits = v.edits;
    edits.plan.direction = "Polish the sentence.".into();
    p.save_passage(&revision, v.version, edits).unwrap();
    let v = p.passage_view(&revision, &text()).unwrap();
    assert!(p
        .start_passage(
            &revision,
            v.version,
            p.view().unwrap().revision,
            false,
            &text()
        )
        .err()
        .unwrap()
        .contains("required original text"));
    assert_eq!(
        p.read_passage(&key).unwrap().passage.prose,
        accepted.passage.prose
    );
    let v = p.passage_view(&revision, &text()).unwrap();
    let mut edits = v.edits;
    edits.prose = "Mara stood quietly beside the workshop window.".into();
    p.save_passage(&revision, v.version, edits).unwrap();
    let v = p.passage_view(&revision, &text()).unwrap();
    p.validate_passage(&revision, v.version, &text()).unwrap();
    assert!(p.passage_view(&revision, &text()).unwrap().can_accept);
}
