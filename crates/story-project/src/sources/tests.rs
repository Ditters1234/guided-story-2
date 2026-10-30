use super::*;
use std::path::Path;
fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn settings() -> LlmConnection {
    LlmConnection {
        provider: TextProvider::Demo,
        ..Default::default()
    }
}
fn node(w: &World, preset: &str, name: &str) -> WorldNode {
    let preset = w.node_types.iter().find(|t| t.id == preset).unwrap();
    WorldNode {
        id: id(),
        type_id: preset.id.clone(),
        name: name.into(),
        description: String::new(),
        visual_description: String::new(),
        components: preset
            .components
            .iter()
            .map(|id| {
                (
                    id.clone(),
                    default_properties(w.component_types.iter().find(|d| &d.id == id).unwrap()),
                )
            })
            .collect(),
    }
}
fn fixture() -> (tempfile::TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let mut w = World::default();
    let place = node(&w, "core.place", "Workshop");
    let person = node(&w, "core.character", "Nora");
    w.edges.push(WorldEdge {
        id: id(),
        type_id: "core.present-at".into(),
        source_id: person.id.clone(),
        target_id: place.id.clone(),
        description: String::new(),
        properties: BTreeMap::new(),
    });
    w.nodes = vec![place, person];
    let p = Project::create(
        &dir.path().join("story"),
        "Fictional event sources".into(),
        w,
        id(),
    )
    .unwrap();
    (dir, p)
}
fn edits(p: &Project, prose: &str, effects: PassageEffects) -> PassageEdits {
    let w = p.view().unwrap().world;
    let mut plan = p.next_passage_plan().unwrap();
    plan.location_id = Some(w.nodes[0].id.clone());
    plan.participant_ids = vec![w.nodes[1].id.clone()];
    plan.reference_ids = w
        .nodes
        .iter()
        .filter(|n| n.components.contains_key("detail.event"))
        .map(|n| n.id.clone())
        .collect();
    PassageEdits {
        plan,
        prose: prose.into(),
        effects_text: serde_json::to_string(&effects).unwrap(),
        close_chapter: false,
        approved_effects: vec![],
    }
}
fn accept(p: &Project, edit: PassageEdits) -> String {
    let draft = id();
    p.save_passage(&draft, 0, edit).unwrap();
    p.validate_passage(&draft, 1, &settings()).unwrap();
    let ready = p.approve_passage_for_test(&draft, &settings()).unwrap();
    assert!(ready.can_accept, "{}", ready.error);
    p.accept_passage(&draft, ready.version, &settings())
        .unwrap();
    draft
}
fn event_effects(p: &Project, name: &str, quote: &str) -> PassageEffects {
    let w = p.view().unwrap().world;
    let mut event = node(&w, "core.event", name);
    event.id = "new:occurrence".into();
    event
        .components
        .get_mut("detail.event")
        .unwrap()
        .insert("occurrence".into(), PropertyValue::Text(quote.into()));
    PassageEffects {
        changes: vec![
            PassageEffect::PutNode {
                node: event,
                evidence: quote.into(),
            },
            PassageEffect::PutLink {
                link: WorldEdge {
                    id: "new:site".into(),
                    type_id: "story.occurred-at".into(),
                    source_id: "new:occurrence".into(),
                    target_id: w.nodes[0].id.clone(),
                    description: String::new(),
                    properties: BTreeMap::new(),
                },
                evidence: quote.into(),
            },
        ],
        ..Default::default()
    }
}
fn event_passage(p: &Project) -> String {
    let prose = "The pump failed beside the workshop.";
    accept(p, edits(p, prose, event_effects(p, "Pump failure", prose)))
}
#[test]
fn authored_events_get_deterministic_manual_provenance_without_executing_effects() {
    let (_dir, p) = fixture();
    let before = p.view().unwrap();
    let mut w = before.world.clone();
    let event = node(&w, "core.event", "The old mill burned years ago");
    let target = event.id.clone();
    w.nodes.push(event);
    let request = id();
    let commit = p
        .commit(0, request.clone(), before.title.clone(), w.clone())
        .unwrap();
    let replay = p.commit(0, request, before.title, w).unwrap();
    assert!(replay.replayed);
    assert_eq!(commit.project.revision, replay.project.revision);
    assert_eq!(commit.project.world.edges, before.world.edges);
    assert!(p.story_index().unwrap().passages.is_empty());
    let info = p.node_sources(&commit.project.world, &target).unwrap();
    assert_eq!(info.len(), 1);
    assert!(matches!(
        info[0].annotation.origin,
        SourceOrigin::Manual { .. }
    ));
}
#[test]
fn passage_event_sources_publish_only_after_approval_with_exact_version_and_effect() {
    let (_dir, p) = fixture();
    let before = p.view().unwrap();
    let prose = "The pump failed beside the workshop.";
    let edit = edits(&p, prose, event_effects(&p, "Pump failure", prose));
    let draft = id();
    p.save_passage(&draft, 0, edit).unwrap();
    p.validate_passage(&draft, 1, &settings()).unwrap();
    let view = p.passage_view(&draft, &settings()).unwrap();
    assert!(!view.can_accept);
    assert!(p.view().unwrap().world.source_annotations.is_empty());
    let ready = p.approve_passage_for_test(&draft, &settings()).unwrap();
    p.accept_passage(&draft, ready.version, &settings())
        .unwrap();
    let reading = p.read_passage(&draft).unwrap();
    assert_eq!(reading.before, before.world);
    assert_eq!(
        reading
            .after
            .edges
            .iter()
            .filter(|e| e.type_id == "core.present-at")
            .count(),
        1
    );
    let event = reading
        .after
        .nodes
        .iter()
        .find(|n| n.name == "Pump failure")
        .unwrap();
    let info = p.node_sources(&reading.after, &event.id).unwrap();
    assert_eq!(info.len(), 1);
    assert!(!info[0].earlier_wording);
    assert!(!info[0].edited_since_source);
    let SourceOrigin::Passage {
        passage_id,
        text_version_id,
        event_id,
        effect_index,
        quote,
        ..
    } = &info[0].annotation.origin
    else {
        panic!("Expected passage provenance")
    };
    assert_eq!(passage_id, &draft);
    assert_eq!(
        Some(text_version_id),
        reading.passage.text_version_id.as_ref()
    );
    assert_eq!(Some(event_id), reading.passage.event_id.as_ref());
    assert_eq!(*effect_index, Some(0));
    assert_eq!(quote, prose);
    let exact = p
        .read_node_source(&reading.after, &info[0].annotation.id)
        .unwrap();
    assert_eq!(exact.prose, prose);
    let replay = p
        .accept_passage(&draft, ready.version, &settings())
        .unwrap();
    assert!(replay.commit.replayed);
    assert_eq!(replay.commit.project.world.source_annotations.len(), 1);
}
#[test]
fn manual_citations_validate_quote_version_effect_scope_and_preserve_snapshot() {
    let (_dir, p) = fixture();
    let pid = event_passage(&p);
    let reading = p.read_passage(&pid).unwrap();
    let current = p.view().unwrap();
    let node_id = current.world.nodes[1].id.clone();
    let req = |quote: &str, effect_index: Option<u32>| SourceEdit::AddPassage {
        node_id: node_id.clone(),
        passage_id: pid.clone(),
        version: reading.record.clone(),
        quote: quote.into(),
        effect_index,
    };
    for (quote, effect) in [
        ("The pump ... workshop.", None),
        ("Nora invented a future event.", None),
        ("The pump failed beside the workshop.", Some(79)),
    ] {
        assert!(p
            .preview_source_edit(&current.world, req(quote, effect))
            .is_err());
    }
    let mut bad = reading.record.clone();
    bad.id = id();
    assert!(p
        .preview_source_edit(
            &current.world,
            SourceEdit::AddPassage {
                node_id: node_id.clone(),
                passage_id: pid.clone(),
                version: bad,
                quote: reading.passage.prose.clone(),
                effect_index: None
            }
        )
        .is_err());
    let preview = p
        .preview_source_edit(
            &current.world,
            req("The pump failed beside the workshop.", Some(0)),
        )
        .unwrap();
    p.commit(current.revision, id(), current.title, preview.world)
        .unwrap();
    assert_eq!(p.read_passage(&pid).unwrap().after, reading.after);
    let mut current = p.view().unwrap();
    current.world.nodes[1].description = "A current author edit".into();
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let info = p.node_sources(&p.view().unwrap().world, &node_id).unwrap();
    assert!(info[0].edited_since_source);
}
#[test]
fn wording_sources_survive_forks_without_copying_alternate_passage_history() {
    for preserve_quote in [true, false] {
        let (dir, p) = fixture();
        let pid = event_passage(&p);
        let original = p.read_passage(&pid).unwrap();
        let draft = id();
        p.begin_passage_revision(
            &draft,
            &pid,
            original.record.clone(),
            PassageRevisionKind::Wording,
        )
        .unwrap();
        let view = p.passage_view(&draft, &settings()).unwrap();
        let mut edit = view.edits;
        edit.prose = if preserve_quote {
            "At dusk, everyone heard the noise. The pump failed beside the workshop."
        } else {
            "The pump had failed beside the workshop."
        }
        .into();
        p.save_passage(&draft, view.version, edit).unwrap();
        let view = p.passage_view(&draft, &settings()).unwrap();
        p.validate_passage(&draft, view.version, &settings())
            .unwrap();
        let view = p.passage_view(&draft, &settings()).unwrap();
        assert!(view.can_accept, "{}", view.error);
        p.accept_passage(&draft, view.version, &settings()).unwrap();
        let current = p.view().unwrap();
        let event = current
            .world
            .nodes
            .iter()
            .find(|n| n.name == "Pump failure")
            .unwrap();
        let info = p.node_sources(&current.world, &event.id).unwrap();
        assert!(info[0].earlier_wording);
        assert_eq!(
            p.read_node_source(&current.world, &info[0].annotation.id)
                .unwrap()
                .prose,
            original.passage.prose
        );
        let fork = p
            .fork_after_passage(&pid, dir.path(), "Independent event sources".into())
            .unwrap();
        drop(p);
        std::fs::rename(
            dir.path().join("story"),
            dir.path().join("source-unavailable"),
        )
        .unwrap();
        let fork_world = fork.view().unwrap().world;
        let info = fork.node_sources(&fork_world, &event.id).unwrap();
        assert!(info[0].earlier_wording);
        assert_eq!(
            fork.read_node_source(&fork_world, &info[0].annotation.id)
                .unwrap()
                .prose,
            original.passage.prose
        );
        assert!(fork.passage_versions(&pid).unwrap().versions.is_empty());
    }
}
#[test]
fn replacement_drops_old_event_facts_and_superseded_sources_cannot_be_reused() {
    let (_dir, p) = fixture();
    let pid = event_passage(&p);
    let original = p.read_passage(&pid).unwrap();
    let old = original.after.source_annotations[0].clone();
    let draft = id();
    p.begin_passage_revision(&draft, &pid, original.record, PassageRevisionKind::Events)
        .unwrap();
    let view = p.passage_view(&draft, &settings()).unwrap();
    let mut edit = view.edits;
    let prose = "Nora repaired the pump beside the workshop.";
    edit.prose = prose.into();
    edit.effects_text = serde_json::to_string(&event_effects(&p, "Pump repaired", prose)).unwrap();
    p.save_passage(&draft, view.version, edit).unwrap();
    let view = p.passage_view(&draft, &settings()).unwrap();
    p.validate_passage(&draft, view.version, &settings())
        .unwrap();
    let view = p.approve_passage_for_test(&draft, &settings()).unwrap();
    p.accept_passage(&draft, view.version, &settings()).unwrap();
    let current = p.view().unwrap();
    assert!(!current.world.nodes.iter().any(|n| n.name == "Pump failure"));
    assert!(current
        .world
        .nodes
        .iter()
        .any(|n| n.name == "Pump repaired"));
    assert!(p
        .read_node_source(&original.after, &old.id)
        .unwrap_err()
        .contains("superseded"));
    let mut forged = current.world;
    let mut a = old;
    a.node_id = forged.nodes[0].id.clone();
    forged.source_annotations.push(a);
    assert!(p
        .commit(current.revision, id(), current.title, forged)
        .is_err());
}
#[test]
fn fork_excludes_future_source_anchors_and_node_removal_cleans_only_current_bindings() {
    let (dir, p) = fixture();
    let first = event_passage(&p);
    let before = p.view().unwrap();
    let prose = "A storm damaged the workshop roof.";
    let second = accept(
        &p,
        edits(&p, prose, event_effects(&p, "Storm damage", prose)),
    );
    let current = p.view().unwrap();
    let future = current
        .world
        .source_annotations
        .iter()
        .find_map(|a| match &a.origin {
            SourceOrigin::Passage {
                passage_id, anchor, ..
            } if passage_id == &second => Some(anchor.clone()),
            _ => None,
        })
        .unwrap();
    let fork = p
        .fork_after_passage(&first, dir.path(), "Before the storm".into())
        .unwrap();
    assert!(!fork
        .root_path()
        .join("source-anchors")
        .join(format!("{}.json", future.id))
        .exists());
    assert!(!fork
        .view()
        .unwrap()
        .world
        .nodes
        .iter()
        .any(|n| n.name == "Storm damage"));
    let mut boundary = p.story_index().unwrap();
    boundary.passages.truncate(1);
    assert!(p.validate_world_sources(&current.world, &boundary).is_err());
    let event = before
        .world
        .nodes
        .iter()
        .find(|n| n.name == "Pump failure")
        .unwrap();
    let preview = preview_world_edit(
        &current.world,
        WorldEdit::RemoveNode {
            id: event.id.clone(),
        },
    )
    .unwrap();
    assert!(!preview
        .world
        .source_annotations
        .iter()
        .any(|a| a.node_id == event.id));
    assert!(p
        .read_passage(&first)
        .unwrap()
        .after
        .nodes
        .iter()
        .any(|n| n.id == event.id));
}
#[test]
fn source_anchors_and_earlier_wording_roundtrip_through_backup_restore() {
    let (dir, p) = fixture();
    let pid = event_passage(&p);
    let original = p.read_passage(&pid).unwrap();
    let draft = id();
    p.begin_passage_revision(&draft, &pid, original.record, PassageRevisionKind::Wording)
        .unwrap();
    let view = p.passage_view(&draft, &settings()).unwrap();
    let mut edit = view.edits;
    edit.prose.push_str(" Nora listened.");
    p.save_passage(&draft, view.version, edit).unwrap();
    let view = p.passage_view(&draft, &settings()).unwrap();
    p.validate_passage(&draft, view.version, &settings())
        .unwrap();
    let view = p.passage_view(&draft, &settings()).unwrap();
    p.accept_passage(&draft, view.version, &settings()).unwrap();
    let source = p.view().unwrap();
    let backup = p.export_backup(dir.path(), &id(), source.revision).unwrap();
    let restored = crate::restore_backup(
        Path::new(&backup.folder_path),
        dir.path(),
        &id(),
        &backup.fingerprint,
    )
    .unwrap();
    let restored = Project::open(Path::new(&restored.folder_path)).unwrap();
    let world = restored.view().unwrap().world;
    assert_eq!(world, source.world);
    let ann = &world.source_annotations[0];
    assert_eq!(
        restored
            .read_node_source(&world, &ann.id)
            .unwrap()
            .passage_id,
        pid
    );
}
#[test]
fn metadata_is_omitted_when_empty_scoped_for_context_and_rejects_forged_anchors() {
    let (_dir, p) = fixture();
    assert!(serde_json::to_value(&p.view().unwrap().world)
        .unwrap()
        .get("sourceAnnotations")
        .is_none());
    let pid = event_passage(&p);
    let current = p.view().unwrap();
    let event = current
        .world
        .nodes
        .iter()
        .find(|n| n.name == "Pump failure")
        .unwrap();
    let mut plan = p.next_passage_plan().unwrap();
    plan.reference_ids.clear();
    plan.participant_ids = vec![current.world.nodes[1].id.clone()];
    plan.location_id = Some(current.world.nodes[0].id.clone());
    let no_event = crate::story::scene_world(&current.world, &plan).unwrap();
    assert!(no_event.source_annotations.is_empty());
    let mut world = current.world.clone();
    if let SourceOrigin::Passage { quote, .. } = &mut world.source_annotations[0].origin {
        *quote = "A quote from an invented future passage.".into();
    }
    assert!(p
        .commit(current.revision, id(), current.title, world)
        .is_err());
    let input = narrative_record(&current.world, event).to_string();
    assert!(input.contains("The pump failed"));
    assert!(input.contains(&pid));
}

#[test]
fn forged_earlier_wording_requires_a_registered_text_version_not_just_a_valid_quote_file() {
    let (_dir, p) = fixture();
    let _ = event_passage(&p);
    let current = p.view().unwrap();
    let mut world = current.world;
    let SourceOrigin::Passage {
        anchor,
        text_version_id,
        ..
    } = &mut world.source_annotations[0].origin
    else {
        panic!()
    };
    let mut forged: SourceAnchor = p.read_record("source-anchors", anchor).unwrap();
    forged.text_version_id = id();
    let version = forged.text_version_id.clone();
    *anchor = p.write_record("source-anchors", &forged).unwrap();
    *text_version_id = version;
    assert!(p
        .commit(current.revision, id(), current.title, world)
        .unwrap_err()
        .contains("not retained"));
}
#[test]
fn event_ai_revision_protects_provenance_scopes_it_and_preserves_unrelated_sources() {
    let (_dir, p) = fixture();
    event_passage(&p);
    let current = p.view().unwrap();
    let event = current
        .world
        .nodes
        .iter()
        .find(|n| n.name == "Pump failure")
        .unwrap();
    let req = AuthoringRequest {
        mode: AuthoringMode::Revise,
        direction: "Clarify the occurrence".into(),
        type_id: None,
        bundle: false,
        creation_scope: None,
        group_type_id: None,
        target_ids: vec![event.id.clone()],
        context_ids: vec![],
        fields: vec![EditableField {
            node_id: event.id.clone(),
            component_id: Some("detail.event".into()),
            key: "occurrence".into(),
        }],
        link_target_ids: vec![],
        include_narration: None,
        creation_type_ids: vec![],
        link_fields: vec![],
    };
    let prepared = prepare_authoring(&current.world, &req, &settings()).unwrap();
    let input: serde_json::Value = serde_json::from_str(&prepared.user).unwrap();
    assert_eq!(input["sources"].as_array().unwrap().len(), 1);
    assert!(!prepared.user.contains("effectQuotes"));
    let draft = id();
    p.save_authoring_draft(&draft, 0, req.clone()).unwrap();
    p.start_authoring(&draft, 1, current.revision, &settings())
        .unwrap();
    p.finish_authoring(
        &draft,
        Ok(demo_authoring(&current.world, &req).unwrap()),
        false,
        false,
    )
    .unwrap();
    let extra = p
        .preview_source_edit(
            &current.world,
            SourceEdit::AddManual {
                node_id: current.world.nodes[1].id.clone(),
                note: "Author background for Nora".into(),
            },
        )
        .unwrap();
    p.commit(current.revision, id(), current.title, extra.world)
        .unwrap();
    let view = p.authoring_view(&draft, &settings()).unwrap();
    assert!(!view.stale);
    let applied = p
        .apply_authoring(&draft, view.version, &settings())
        .unwrap();
    assert_eq!(applied.commit.project.world.source_annotations.len(), 2);
    assert!(
        p.node_sources(&applied.commit.project.world, &event.id)
            .unwrap()[0]
            .edited_since_source
    );
    let mut forged = req;
    forged.fields[0].component_id = None;
    forged.fields[0].key = "sourceAnnotations".into();
    assert!(prepare_authoring(&applied.commit.project.world, &forged, &settings()).is_err());
}
#[test]
fn full_source_history_is_a_repairable_per_card_error_before_acceptance() {
    let (_dir, p) = fixture();
    event_passage(&p);
    let mut current = p.view().unwrap();
    let mut event = current
        .world
        .nodes
        .iter()
        .find(|n| n.name == "Pump failure")
        .unwrap()
        .clone();
    for i in 0..31 {
        current.world.source_annotations.push(SourceAnnotation {
            id: id(),
            node_id: event.id.clone(),
            subject_fingerprint: node_source_fingerprint(&event),
            origin: SourceOrigin::Manual {
                note: format!("Author source {i}"),
            },
        });
    }
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let prose = "The pump remained unusable beside the workshop.";
    event
        .components
        .get_mut("detail.event")
        .unwrap()
        .insert("occurrence".into(), PropertyValue::Text(prose.into()));
    let edit = edits(
        &p,
        prose,
        PassageEffects {
            changes: vec![PassageEffect::PutNode {
                node: event,
                evidence: prose.into(),
            }],
            ..Default::default()
        },
    );
    let draft = id();
    p.save_passage(&draft, 0, edit).unwrap();
    assert!(p
        .validate_passage(&draft, 1, &settings())
        .unwrap_err()
        .contains("space for its source"));
    let view = p.passage_view(&draft, &settings()).unwrap();
    assert!(!view.can_accept);
    assert!(!view.effects_review.entries[0].issues.is_empty());
}

#[test]
fn forks_freeze_older_text_identities_without_modifying_the_source_project() {
    let (dir, p) = fixture();
    let pid = accept(
        &p,
        edits(&p, "Nora watched the workshop.", PassageEffects::default()),
    );
    let mut current = p.view().unwrap();
    let mut index = p.story_index().unwrap();
    let mut legacy = p.read_passage(&pid).unwrap().passage;
    legacy.text_version_id = None;
    legacy.event_id = None;
    legacy.text_anchor = None;
    legacy.earlier_text_anchors.clear();
    let legacy_ref = p.write_record("passages", &legacy).unwrap();
    index.passages[0].record = legacy_ref.clone();
    p.commit_story(current.revision, id(), current.title, current.world, index)
        .unwrap();
    current = p.view().unwrap();
    let preview = p
        .preview_source_edit(
            &current.world,
            SourceEdit::AddPassage {
                node_id: current.world.nodes[1].id.clone(),
                passage_id: pid.clone(),
                version: legacy_ref,
                quote: legacy.prose.clone(),
                effect_index: None,
            },
        )
        .unwrap();
    p.commit(current.revision, id(), current.title, preview.world)
        .unwrap();
    let latest = accept(
        &p,
        edits(
            &p,
            "Nora waited for the rain to stop.",
            PassageEffects::default(),
        ),
    );
    let anchors = std::fs::read_dir(p.root_path().join("source-anchors"))
        .unwrap()
        .count();
    let fork = p
        .fork_after_passage(&latest, dir.path(), "Independent legacy citation".into())
        .unwrap();
    assert_eq!(
        std::fs::read_dir(p.root_path().join("source-anchors"))
            .unwrap()
            .count(),
        anchors
    );
    let world = fork.view().unwrap().world;
    let source = fork
        .read_node_source(&world, &world.source_annotations[0].id)
        .unwrap();
    assert_eq!(source.prose, legacy.prose);
    assert!(!source.earlier_wording);
}

#[test]
fn removing_last_passage_citation_keeps_explicit_author_provenance_with_an_honest_preview() {
    let (_dir, p) = fixture();
    event_passage(&p);
    let current = p.view().unwrap();
    let annotation = &current.world.source_annotations[0];
    let preview = p
        .preview_source_edit(
            &current.world,
            SourceEdit::Remove {
                id: annotation.id.clone(),
            },
        )
        .unwrap();
    assert!(preview
        .changes
        .iter()
        .any(|s| s.contains("no passage citation remains")));
    assert_eq!(preview.world.nodes, current.world.nodes);
    assert_eq!(preview.world.source_annotations.len(), 1);
    assert!(matches!(
        preview.world.source_annotations[0].origin,
        SourceOrigin::Manual { .. }
    ));
    assert!(p
        .preview_source_edit(
            &preview.world,
            SourceEdit::Remove {
                id: preview.world.source_annotations[0].id.clone()
            }
        )
        .unwrap_err()
        .contains("retains an author source"));
}
