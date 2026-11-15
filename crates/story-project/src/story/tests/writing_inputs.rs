use super::*;

fn sources(p: &Project) -> (String, String) {
    let before = p.view().unwrap();
    let mut world = before.world;
    install_bundled_writing_fields(&mut world, None).unwrap();
    world
        .relation_types
        .extend(bundled_writing_relationships().unwrap());
    let mut narration = node(&world, "core.narration-style", "Workshop narration");
    narration
        .components
        .get_mut("core.narration-style")
        .unwrap()
        .insert(
            "instructions".into(),
            PropertyValue::Text("Use restrained dialogue.".into()),
        );
    for (role, target) in [("core.narration-pov", 2), ("core.narration-focus", 0)] {
        world.edges.push(WorldEdge {
            id: id(),
            type_id: role.into(),
            source_id: narration.id.clone(),
            target_id: world.nodes[target].id.clone(),
            description: String::new(),
            properties: Default::default(),
        });
    }
    let mut brief = node(&world, "core.brief", "Workshop brief");
    brief.components.get_mut("detail.brief").unwrap().extend([
        (
            "chapterDirection".into(),
            PropertyValue::Text("Resolve the broken latch.".into()),
        ),
        (
            "passageDirection".into(),
            PropertyValue::Text("Jon examines the latch.".into()),
        ),
        (
            "stopAfter".into(),
            PropertyValue::Text("The fault is identified.".into()),
        ),
        (
            "aims".into(),
            PropertyValue::Text("Keep the stakes local.".into()),
        ),
    ]);
    let ids = (narration.id.clone(), brief.id.clone());
    world.ai_context.push(AiContextBinding {
        node_id: ids.0.clone(),
        enabled: true,
        role: AiContextRole::Instructions,
        tasks: vec![TextPromptTask::Passage, TextPromptTask::Wording],
        fields: vec![AiContextField::Components],
        order: 0,
        image_preparation: false,
    });
    world.nodes.extend([narration, brief]);
    p.commit(before.revision, id(), before.title, world)
        .unwrap();
    ids
}
fn selected(p: &Project, sources: &(String, String), prose: &str) -> PassageEdits {
    let mut edit = edits(p, prose);
    edit.plan.narration_node_id = Some(sources.0.clone());
    edit.plan.brief_node_id = Some(sources.1.clone());
    edit
}
fn change_brief(p: &Project, brief: &str, field: &str, value: &str) {
    let before = p.view().unwrap();
    let mut world = before.world;
    world
        .nodes
        .iter_mut()
        .find(|n| n.id == brief)
        .unwrap()
        .components
        .get_mut("detail.brief")
        .unwrap()
        .insert(field.into(), PropertyValue::Text(value.into()));
    p.commit(before.revision, id(), before.title, world)
        .unwrap();
}

#[test]
fn selected_nodes_drive_preview_scope_generation_and_captured_plan_once() {
    let (_dir, p) = fixture();
    let ids = sources(&p);
    let key = id();
    let edit = selected(&p, &ids, "");
    p.save_passage(&key, 0, edit.clone()).unwrap();
    let preview = p.passage_view(&key, &settings()).unwrap();
    assert_eq!(preview.edits, edit); // Preview does not rewrite the saved draft.
    assert_eq!(
        preview.resolved_plan.as_ref().unwrap().viewpoint_id,
        p.view().unwrap().world.nodes.get(2).map(|n| n.id.clone())
    );
    assert!(!preview
        .scene_context
        .resources
        .iter()
        .any(|r| r.name == "Mara" || r.name == "Field coat"));
    let plan = p
        .prepare_plan_fill(
            &key,
            1,
            p.view().unwrap().revision,
            PlanFillKind::Scene,
            &settings(),
        )
        .unwrap();
    assert!(plan.user.contains("Jon examines the latch."));
    let mut config = settings();
    config.max_tokens = 1536;
    let revision = p.view().unwrap().revision;
    let prompt = p
        .preview_passage_prompt(&key, 1, revision, PassagePromptAction::Generate, &config)
        .unwrap();
    let unchanged = p.passage_view(&key, &config).unwrap();
    assert_eq!(unchanged.edits, edit);
    assert_eq!(unchanged.version, 1);
    assert_eq!(unchanged.stage, preview.stage);
    let prepared = p.start_passage(&key, 1, revision, false, &config).unwrap();
    assert_eq!(prompt.system, prepared.system);
    assert_eq!(prompt.user, prepared.user);
    assert!(prompt.response_schema.is_none() && prompt.offline);
    let input: Value = serde_json::from_str(&prepared.user).unwrap();
    assert_eq!(prepared.output_tokens, 1536);
    assert!(input["plan"].get("targetWords").is_none());
    assert_eq!(
        input["writingBrief"]["passageDirection"],
        "Jon examines the latch."
    );
    assert_eq!(
        input["writingBrief"]["chapterDirection"],
        "Resolve the broken latch."
    );
    assert_eq!(
        prepared.system.matches("Use restrained dialogue.").count(),
        1
    );
    assert!(!prepared.system.contains("Jon examines the latch."));
    assert!(prepared.system.contains("Keep the stakes local."));
    assert!(!input["records"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["id"] == ids.0 || n["id"] == ids.1));
    let running = p.passage_view(&key, &config).unwrap();
    assert_eq!(running.edits.plan.direction, "Jon examines the latch.");
    assert!(p.story_index().unwrap().passages.is_empty());
}

#[test]
fn world_change_prompt_preview_includes_the_configured_response_schema() {
    let (_dir, p) = fixture();
    let key = id();
    let revision = p.view().unwrap().revision;
    p.save_passage(&key, 0, edits(&p, "Mara examines the latch."))
        .unwrap();
    let mut config = settings();
    config.structured_output = StructuredOutput::Schema;
    let preview = p
        .preview_passage_prompt(
            &key,
            1,
            revision,
            PassagePromptAction::CheckChanges,
            &config,
        )
        .unwrap();
    let actual = p.start_passage(&key, 1, revision, true, &config).unwrap();
    assert!(preview.response_schema.is_some());
    assert_eq!(
        preview.response_schema,
        actual.schema.as_ref().map(Value::to_string)
    );
    assert_eq!(preview.system, actual.system);
    assert_eq!(preview.user, actual.user);
}

#[test]
fn approved_continuity_updates_publish_with_brief_links_and_passage_citations() {
    let (dir, p) = fixture();
    let ids = sources(&p);
    let mut edit = selected(&p, &ids, "Jon examines the latch.");
    let quote = edit.prose.clone();
    let note = "Jon has inspected the latch.";
    edit.effects_text = serde_json::to_string(&PassageEffects {
        notes: vec![ContinuityNote {
            text: note.into(),
            evidence: quote.clone(),
            node_ids: vec![],
        }],
        ..Default::default()
    })
    .unwrap();
    let before = p.view().unwrap();
    let key = ready(&p, edit);
    let review = p.passage_view(&key, &settings()).unwrap();
    assert!(review.can_accept);
    assert_eq!(p.view().unwrap().world, before.world);
    p.accept_passage(&key, review.version, &settings()).unwrap();
    let reading = p.read_passage(&key).unwrap();
    let node = reading
        .after
        .nodes
        .iter()
        .find(|n| n.description == note)
        .unwrap();
    assert_eq!(node.type_id, "core.note");
    assert!(!reading.before.nodes.iter().any(|n| n.id == node.id));
    assert!(reading
        .after
        .edges
        .iter()
        .any(|edge| edge.type_id == "core.related"
            && edge.source_id == node.id
            && edge.target_id == ids.1));
    assert!(reading.after.source_annotations.iter().any(|annotation| {
        annotation.node_id == node.id
            && matches!(&annotation.origin,
            SourceOrigin::Passage { passage_id, quote: excerpt, effect_index: None, .. }
                if passage_id == &key && excerpt == &quote)
    }));
    p.accept_passage(&key, review.version, &settings()).unwrap();
    assert_eq!(p.view().unwrap().world, reading.after);

    let next_key = id();
    let mut next = selected(&p, &ids, "");
    next.plan.reference_ids.push(node.id.clone());
    p.save_passage(&next_key, 0, next).unwrap();
    let prompt = p
        .preview_passage_prompt(
            &next_key,
            1,
            p.view().unwrap().revision,
            PassagePromptAction::Generate,
            &settings(),
        )
        .unwrap();
    let input: Value = serde_json::from_str(&prompt.user).unwrap();
    let sources = input["sources"].as_array().unwrap();
    assert!(sources
        .iter()
        .any(|source| source["nodeId"] == node.id && source["origin"]["quote"] == quote));
    let record = input["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["id"] == node.id)
        .unwrap();
    assert_eq!(record["sources"][0]["origin"]["quote"], quote);
    assert!(!prompt.user.contains("\"sha256\"") && !prompt.user.contains("\"subjectFingerprint\""));

    let fork = p
        .fork_after_passage(&key, dir.path(), "Continuity update fork".into())
        .unwrap();
    let forked = fork.read_passage(&key).unwrap();
    assert_eq!(
        forked.after.source_annotations,
        reading.after.source_annotations
    );
    assert!(forked
        .after
        .edges
        .iter()
        .any(|edge| edge.source_id == node.id && edge.target_id == ids.1));
}

#[test]
fn unbound_brief_edits_stale_review_without_losing_prose_or_mutating_history() {
    let (_dir, p) = fixture();
    let ids = sources(&p);
    let key = ready(&p, selected(&p, &ids, "Jon studies the latch."));
    assert!(p.passage_view(&key, &settings()).unwrap().can_accept);
    let before = p.view().unwrap();
    let mut world = before.world;
    let note = node(&world, "core.note", "Unrelated note");
    world.nodes.push(note);
    p.commit(before.revision, id(), before.title, world)
        .unwrap();
    assert!(p.passage_view(&key, &settings()).unwrap().can_accept);
    change_brief(&p, &ids.1, "chapterDirection", "Find a different repair.");
    let stale = p.passage_view(&key, &settings()).unwrap();
    assert!(stale.stale && !stale.can_accept);
    assert_eq!(stale.edits.prose, "Jon studies the latch.");
    assert_eq!(stale.edits.plan.chapter_plan, "Resolve the broken latch.");
    assert_eq!(
        stale.resolved_plan.unwrap().chapter_plan,
        "Find a different repair."
    );
    p.validate_passage(&key, stale.version, &settings())
        .unwrap();
    assert!(p.passage_view(&key, &settings()).unwrap().can_accept);
    assert!(p.story_index().unwrap().passages.is_empty());
}

#[test]
fn next_passage_consumes_only_the_unchanged_passage_direction_and_never_rewrites_nodes() {
    let (_dir, p) = fixture();
    let ids = sources(&p);
    accept(&p, selected(&p, &ids, "Jon examines the latch."));
    let first = p.next_passage_plan().unwrap();
    assert!(first.consumed_brief_passage.is_some());
    let key = id();
    let mut next = edits(&p, "Jon considers the repair.");
    next.plan = first.clone();
    p.save_passage(&key, 0, next.clone()).unwrap();
    let resolved = p
        .passage_view(&key, &settings())
        .unwrap()
        .resolved_plan
        .unwrap();
    assert!(resolved.direction.is_empty() && resolved.stop_after.is_empty());
    assert_eq!(resolved.chapter_plan, "Resolve the broken latch.");
    accept(&p, next);
    let third = p.next_passage_plan().unwrap();
    let world = p.view().unwrap().world;
    assert!(super::super::writing_inputs::resolve(&world, &third)
        .unwrap()
        .direction
        .is_empty());
    let mut reused = third.clone();
    reused.consumed_brief_passage = None;
    assert_eq!(
        super::super::writing_inputs::resolve(&world, &reused)
            .unwrap()
            .direction,
        "Jon examines the latch."
    );
    change_brief(&p, &ids.1, "passageDirection", "Jon chooses a tool.");
    assert_eq!(
        super::super::writing_inputs::resolve(&p.view().unwrap().world, &third)
            .unwrap()
            .direction,
        "Jon chooses a tool."
    );
    let chapter = p.new_chapter_plan().unwrap();
    assert!(chapter.brief_node_id.is_none());
    assert_eq!(chapter.narration_node_id, Some(ids.0));
    assert_eq!(
        world
            .nodes
            .iter()
            .find(|n| n.id == ids.1)
            .unwrap()
            .components["detail.brief"]["passageDirection"],
        PropertyValue::Text("Jon examines the latch.".into())
    );
}

#[test]
fn missing_and_wrong_sources_block_requests_and_never_grant_effect_edit_permission() {
    let (_dir, p) = fixture();
    let ids = sources(&p);
    let key = id();
    let mut edit = selected(&p, &ids, "");
    edit.plan.narration_node_id = Some(p.view().unwrap().world.nodes[0].id.clone());
    p.save_passage(&key, 0, edit).unwrap();
    assert!(p
        .start_passage(&key, 1, p.view().unwrap().revision, false, &settings())
        .err()
        .unwrap()
        .contains("writing source"));
    assert_eq!(
        p.passage_view(&key, &settings()).unwrap().stage,
        PassageStage::Draft
    );
    let key = id();
    let mut edit = selected(&p, &ids, "Jon examines the latch.");
    edit.plan.brief_node_id = Some(id());
    p.save_passage(&key, 0, edit).unwrap();
    assert!(p
        .passage_view(&key, &settings())
        .unwrap()
        .scene_context
        .error
        .contains("missing"));
    let key = id();
    let mut edit = selected(&p, &ids, "Jon examines the latch.");
    let mut brief = p
        .view()
        .unwrap()
        .world
        .nodes
        .into_iter()
        .find(|n| n.id == ids.1)
        .unwrap();
    brief.components.get_mut("detail.brief").unwrap().insert(
        "passageDirection".into(),
        PropertyValue::Text("AI changes the author plan".into()),
    );
    edit.effects_text = serde_json::to_string(&PassageEffects {
        changes: vec![PassageEffect::PutNode {
            node: brief,
            evidence: "Jon examines the latch.".into(),
        }],
        ..Default::default()
    })
    .unwrap();
    p.save_passage(&key, 0, edit).unwrap();
    assert!(p.validate_passage(&key, 1, &settings()).is_err());
}

#[test]
fn selected_sources_remain_historical_through_wording_fork_and_backup() {
    let (dir, p) = fixture();
    let ids = sources(&p);
    let accepted = accept(&p, selected(&p, &ids, "Jon examines the latch."));
    let original = p.read_passage(&accepted).unwrap();
    change_brief(&p, &ids.1, "chapterDirection", "FUTURE secret repair plan");
    let revision = id();
    p.begin_passage_revision(
        &revision,
        &accepted,
        original.record.clone(),
        PassageRevisionKind::Wording,
    )
    .unwrap();
    let mut draft = p.passage_view(&revision, &settings()).unwrap();
    draft.edits.prose = "Jon quietly examines the latch.".into();
    draft.edits.plan.direction = "Make the wording quieter.".into();
    p.save_passage(&revision, draft.version, draft.edits)
        .unwrap();
    let current = p.passage_view(&revision, &settings()).unwrap();
    let prepared = p
        .start_passage(
            &revision,
            current.version,
            p.view().unwrap().revision,
            false,
            &settings(),
        )
        .unwrap();
    assert!(!prepared.user.contains("FUTURE") && !prepared.system.contains("FUTURE"));
    p.finish_passage(
        &revision,
        false,
        Ok("Jon quietly examines the latch.".into()),
        false,
        false,
        &settings(),
    )
    .unwrap();
    let current = p.passage_view(&revision, &settings()).unwrap();
    p.validate_passage(&revision, current.version, &settings())
        .unwrap();
    let fork = p
        .fork_after_passage(&accepted, dir.path(), "Writing source fork".into())
        .unwrap();
    let child = fork;
    assert!(!serde_json::to_string(&child.view().unwrap().world)
        .unwrap()
        .contains("FUTURE"));
    drop(child);
    assert_eq!(
        p.read_passage(&accepted).unwrap().passage.plan,
        original.passage.plan
    );
    let backup = p
        .export_backup(dir.path(), &id(), p.view().unwrap().revision)
        .unwrap();
    let reviewed = crate::inspect_backup(Path::new(&backup.folder_path)).unwrap();
    let restored = crate::restore_backup(
        Path::new(&backup.folder_path),
        dir.path(),
        &id(),
        &reviewed.fingerprint,
    )
    .unwrap();
    let restored = Project::open(Path::new(&restored.folder_path)).unwrap();
    let restored = restored.passage_view(&revision, &settings()).unwrap();
    assert!(restored.can_accept);
    assert_eq!(restored.edits.plan.brief_node_id, Some(ids.1));
}

#[test]
fn absent_source_fields_keep_the_original_plan_serialization() {
    let plan = PassagePlan::default();
    let value = serde_json::to_value(&plan).unwrap();
    assert!(
        value.get("narrationNodeId").is_none()
            && value.get("briefNodeId").is_none()
            && value.get("consumedBriefPassage").is_none()
    );
    assert_eq!(
        serde_json::from_value::<PassagePlan>(value.clone()).unwrap(),
        plan
    );
    assert_eq!(serde_json::to_value(&plan).unwrap(), value);
}

#[test]
fn retired_target_words_loads_without_entering_new_plan_serialization() {
    let plan = PassagePlan::default();
    let mut old = serde_json::to_value(&plan).unwrap();
    old["targetWords"] = json!(2400);
    let loaded: PassagePlan = serde_json::from_value(old).unwrap();
    assert_eq!(loaded, plan);
    assert!(serde_json::to_value(&loaded).unwrap().get("targetWords").is_none());
}

#[test]
fn selected_narration_needs_its_own_scene_focus_link_even_in_old_worlds() {
    let (_dir, p) = fixture();
    let mut before = p.view().unwrap();
    let style = node(&before.world, "core.narration-style", "Selected narration");
    let other = node(&before.world, "core.narration-style", "Other narration");
    let style_id = style.id.clone();
    let other_id = other.id.clone();
    let place = before.world.nodes[0].id.clone();
    before.world.nodes.extend([style, other]);
    p.commit(before.revision, id(), before.title, before.world).unwrap();
    let mut draft = edits(&p, "");
    draft.plan.narration_node_id = Some(style_id.clone());
    let key = id();
    p.save_passage(&key, 0, draft).unwrap();
    assert!(p.passage_view(&key, &settings()).unwrap().resolved_plan.unwrap().location_id.is_none());

    let mut before = p.view().unwrap();
    before.world.relation_types.extend(bundled_writing_relationships().unwrap());
    before.world.edges.push(WorldEdge {
        id: id(), type_id: "core.narration-focus".into(), source_id: other_id,
        target_id: place.clone(), description: String::new(), properties: Default::default(),
    });
    p.commit(before.revision, id(), before.title, before.world).unwrap();
    assert!(p.passage_view(&key, &settings()).unwrap().resolved_plan.unwrap().location_id.is_none());
    let mut before = p.view().unwrap();
    before.world.edges.last_mut().unwrap().source_id = style_id;
    p.commit(before.revision, id(), before.title, before.world).unwrap();
    assert_eq!(p.passage_view(&key, &settings()).unwrap().resolved_plan.unwrap().location_id, Some(place));
}

#[test]
fn retired_pov_is_preserved_in_world_data_and_omitted_from_prose_requests() {
    let (_dir, p) = fixture();
    let ids = sources(&p);
    let mut before = p.view().unwrap();
    let schema = before.world.component_types.iter_mut().find(|d| d.id == "core.narration-style").unwrap();
    let mut old = schema.fields.iter().find(|f| f.key == "instructions").unwrap().clone();
    old.key = "viewpointId".into();
    old.label = "Old viewpoint".into();
    schema.fields.push(old);
    let mut other = node(&before.world, "core.narration-style", "Older narration");
    let marker = "RETIRED_VIEWPOINT_VALUE";
    let values = other.components.get_mut("core.narration-style").unwrap();
    values.insert("viewpointId".into(), PropertyValue::Text(marker.into()));
    values.insert("instructions".into(), PropertyValue::Text("Use short sentences.".into()));
    before.world.ai_context.push(AiContextBinding {
        node_id: other.id.clone(), enabled: true, role: AiContextRole::Instructions,
        tasks: vec![TextPromptTask::Passage], fields: vec![AiContextField::Components], order: 10,
        image_preparation: false,
    });
    let other_id = other.id.clone();
    before.world.nodes.push(other);
    let committed = p.commit(before.revision, id(), before.title, before.world).unwrap();
    let mut draft = selected(&p, &ids, "");
    draft.plan.reference_ids.push(other_id.clone());
    let key = id();
    p.save_passage(&key, 0, draft).unwrap();
    let prompt = p.preview_passage_prompt(&key, 1, committed.project.revision, PassagePromptAction::Generate, &settings()).unwrap();
    assert!(prompt.system.contains("Use short sentences."));
    assert!(!prompt.system.contains(marker) && !prompt.user.contains(marker));
    let view = p.passage_view(&key, &settings()).unwrap();
    let mut draft = view.edits;
    draft.prose = "Jon examined the latch.".into();
    p.save_passage(&key, view.version, draft).unwrap();
    let prompt = p.preview_passage_prompt(&key, view.version + 1, committed.project.revision, PassagePromptAction::CheckChanges, &settings()).unwrap();
    assert!(!prompt.user.contains(marker));
    assert!(!prompt.response_schema.as_deref().unwrap_or_default().contains(marker));
    let world = p.view().unwrap().world;
    let older = world.nodes.iter().find(|n| n.id == other_id).unwrap();
    assert_eq!(older.components["core.narration-style"]["viewpointId"], PropertyValue::Text(marker.into()));
}

#[test]
fn brief_preserves_draft_direction_through_validation_and_reopen() {
    let (dir, p) = fixture();
    let mut before = p.view().unwrap();
    let definition = before.world.component_types.iter_mut().find(|d| d.id == "detail.brief").unwrap();
    definition.fields.retain(|field| !super::super::writing_inputs::BRIEF_FIELDS.contains(&field.key.as_str()));
    let brief = node(&before.world, "core.brief", "Older brief");
    let brief_id = brief.id.clone();
    before.world.nodes.push(brief);
    p.commit(before.revision, id(), before.title, before.world).unwrap();
    let mut draft = edits(&p, "Mara examined the latch.");
    draft.plan.brief_node_id = Some(brief_id.clone());
    draft.plan.direction = "Draft passage direction.".into();
    draft.plan.chapter_plan = "Draft chapter arc.".into();
    draft.plan.stop_after = "Draft stopping point.".into();
    let key = id();
    p.save_passage(&key, 0, draft).unwrap();
    let view = p.passage_view(&key, &settings()).unwrap();
    assert_eq!(view.resolved_plan.unwrap().direction, "Draft passage direction.");

    let mut before = p.view().unwrap();
    install_bundled_writing_fields(&mut before.world, Some("detail.brief")).unwrap();
    p.commit(before.revision, id(), before.title, before.world).unwrap();
    assert_eq!(p.passage_view(&key, &settings()).unwrap().resolved_plan.unwrap().direction, "Draft passage direction.");
    change_brief(&p, &brief_id, "passageDirection", "Brief passage direction.");
    let prompt = p.preview_passage_prompt(&key, 1, p.view().unwrap().revision, PassagePromptAction::Generate, &settings()).unwrap();
    assert!(prompt.user.contains("Brief passage direction.") && prompt.user.contains("Draft stopping point."));
    assert!(!prompt.user.contains("Draft passage direction.") && !prompt.user.contains("savedDraftDirection"));
    p.validate_passage(&key, 1, &settings()).unwrap();
    let path = p.folder_path();
    drop(p);
    let p = Project::open(Path::new(&path)).unwrap();
    let view = p.passage_view(&key, &settings()).unwrap();
    assert!(view.can_accept);
    assert_eq!(view.edits.plan.saved_draft_direction.as_ref().unwrap().direction, "Draft passage direction.");
    p.accept_passage(&key, view.version, &settings()).unwrap();
    let accepted = p.read_passage(&key).unwrap().passage;
    assert_eq!(accepted.plan.direction, "Brief passage direction.");
    assert_eq!(accepted.plan.stop_after, "Draft stopping point.");
    assert!(accepted.plan.saved_draft_direction.is_none());
    drop(p);
    drop(dir);
}

#[test]
fn current_drafts_use_only_the_selected_narrations_pov_link() {
    let (_dir, project) = fixture();
    let (narration, _) = sources(&project);
    let mut edit = edits(&project, "Mara and Jon inspect the latch.");
    let former = edit.plan.viewpoint_id.clone().unwrap();
    let linked = edit.plan.participant_ids[0].clone();
    let draft_id = id();
    project.save_passage(&draft_id, 0, edit.clone()).unwrap();
    let view = project.passage_view(&draft_id, &settings()).unwrap();
    let resolved = view.resolved_plan.unwrap();
    assert_eq!(view.edits, edit);
    assert!(resolved.viewpoint_id.is_none());
    assert_eq!(resolved.participant_ids, [former.clone(), linked.clone()]);
    assert!(view.scene_context.guidance.iter().any(|g| g.node_id == narration));
    assert!(view
        .scene_context
        .resources
        .iter()
        .all(|r| r.role != SceneResourceRole::Viewpoint));

    edit.plan.participant_ids = resolved.participant_ids;
    edit.plan.narration_node_id = Some(narration.clone());
    project.save_passage(&draft_id, view.version, edit).unwrap();
    let view = project.passage_view(&draft_id, &settings()).unwrap();
    let resolved = view.resolved_plan.unwrap();
    assert_eq!(resolved.viewpoint_id.as_ref(), Some(&linked));
    assert_eq!(resolved.participant_ids, [former]);
    assert!(view
        .scene_context
        .resources
        .iter()
        .any(|r| r.node_id == linked && r.role == SceneResourceRole::Viewpoint));

    let before = project.view().unwrap();
    let mut world = before.world;
    world
        .edges
        .retain(|edge| edge.source_id != narration || edge.type_id != "core.narration-pov");
    project
        .commit(before.revision, id(), before.title, world)
        .unwrap();
    let view = project.passage_view(&draft_id, &settings()).unwrap();
    assert!(view.resolved_plan.unwrap().viewpoint_id.is_none());
    assert!(view
        .scene_context
        .resources
        .iter()
        .all(|r| r.role != SceneResourceRole::Viewpoint));
}

#[test]
fn narration_links_drive_scope_and_stale_checks_without_moving_anyone() {
    let (dir, p) = fixture();
    let ids = sources(&p);
    let key = ready(&p, selected(&p, &ids, "Jon examines the latch."));
    let before = p.view().unwrap();
    let physical: Vec<_> = before
        .world
        .edges
        .iter()
        .filter(|e| e.type_id == "core.present-at")
        .cloned()
        .collect();
    let mut world = before.world;
    let jon = world.nodes[2].id.clone();
    let mara = world.nodes[1].id.clone();
    world
        .edges
        .iter_mut()
        .find(|e| e.type_id == "core.narration-pov")
        .unwrap()
        .target_id = mara.clone();
    let place = node(&world, "core.place", "Workroom");
    let focus = place.id.clone();
    world.nodes.push(place);
    world
        .edges
        .iter_mut()
        .find(|e| e.type_id == "core.narration-focus")
        .unwrap()
        .target_id = focus.clone();
    // Keep the named actor as an explicit reference so only the link-defined role changes.
    let mut draft = p.passage_view(&key, &settings()).unwrap();
    draft.edits.plan.reference_ids.push(jon);
    p.save_passage(&key, draft.version, draft.edits).unwrap();
    p.commit(before.revision, id(), before.title, world)
        .unwrap();
    let view = p.passage_view(&key, &settings()).unwrap();
    assert!(view.stale && !view.can_accept);
    let resolved = view.resolved_plan.unwrap();
    assert_eq!(resolved.viewpoint_id, Some(mara));
    assert_eq!(resolved.location_id, Some(focus));
    assert_eq!(
        p.view()
            .unwrap()
            .world
            .edges
            .into_iter()
            .filter(|e| e.type_id == "core.present-at")
            .collect::<Vec<_>>(),
        physical
    );
    assert_eq!(view.edits.prose, "Jon examines the latch.");
    let folder = p.folder_path();
    drop(p);
    let reopened = Project::open(Path::new(&folder)).unwrap();
    assert_eq!(
        reopened
            .passage_view(&key, &settings())
            .unwrap()
            .resolved_plan
            .unwrap(),
        resolved
    );
    drop(reopened);
    drop(dir);
}

#[test]
fn narration_link_contracts_and_prose_effect_protection_are_native_owned() {
    let (_dir, p) = fixture();
    let _ids = sources(&p);
    let world = p.view().unwrap().world;
    let role = world
        .edges
        .iter()
        .find(|e| e.type_id == "core.narration-pov")
        .unwrap();
    let mut wrong = world.clone();
    wrong
        .edges
        .iter_mut()
        .find(|e| e.id == role.id)
        .unwrap()
        .target_id = world.nodes[0].id.clone();
    assert!(wrong.validate().is_err());
    let mut wrong = world.clone();
    wrong
        .relation_types
        .iter_mut()
        .find(|d| d.id == role.type_id)
        .unwrap()
        .physical_placement = true;
    assert!(wrong.validate().is_err());
    let mut wrong = world.clone();
    let mut duplicate = role.clone();
    duplicate.id = id();
    wrong.edges.push(duplicate);
    assert!(wrong.validate().is_err());
    for effect in [
        PassageEffect::RemoveLink {
            id: role.id.clone(),
            evidence: "Jon examines the latch.".into(),
        },
        PassageEffect::PutLink {
            link: role.clone(),
            evidence: "Jon examines the latch.".into(),
        },
    ] {
        let issues = crate::story_effects::effect_record_issues(
            &world,
            &world,
            &PassageEffects {
                changes: vec![effect],
                ..Default::default()
            },
        );
        assert!(issues[0].iter().any(|s| s.contains("author planning")));
    }
    let schema = super::super::effects_schema(&world).to_string();
    assert!(!schema.contains("core.narration-pov") && !schema.contains(&role.id));
}

#[test]
fn changing_a_selected_source_during_generation_retains_the_result_for_review() {
    let (_dir, p) = fixture();
    let ids = sources(&p);
    let key = id();
    p.save_passage(&key, 0, selected(&p, &ids, "")).unwrap();
    p.start_passage(&key, 1, p.view().unwrap().revision, false, &settings())
        .unwrap();
    change_brief(&p, &ids.1, "passageDirection", "Jon chooses a tool.");
    p.finish_passage(
        &key,
        false,
        Ok("Jon examines the latch.".into()),
        false,
        false,
        &settings(),
    )
    .unwrap();
    let retained = p.passage_view(&key, &settings()).unwrap();
    assert_eq!(retained.edits.prose, "Jon examines the latch.");
    assert_eq!(retained.edits.plan.direction, "Jon examines the latch.");
    assert!(retained.stale && !retained.can_accept);
    assert!(p.story_index().unwrap().passages.is_empty());
}

#[test]
fn narration_perspective_is_resolved_before_claim_retrieval() {
    let (_dir, p) = fixture();
    let ids = sources(&p);
    let before = p.view().unwrap();
    let mut world = before.world;
    let mut secret = node(&world, "core.claim", "Hidden latch history");
    secret.components.get_mut("core.claim").unwrap().extend([
        (
            "proposition".into(),
            PropertyValue::Text("The latch was sabotaged.".into()),
        ),
        ("disclosure".into(), PropertyValue::Text("narrator".into())),
        ("status".into(), PropertyValue::Text("established".into())),
    ]);
    let secret_id = secret.id.clone();
    world.nodes.push(secret);
    p.commit(before.revision, id(), before.title, world)
        .unwrap();
    let key = id();
    let mut edit = selected(&p, &ids, "");
    edit.plan.reference_ids.push(secret_id.clone());
    p.save_passage(&key, 0, edit).unwrap();
    let viewpoint = p.passage_view(&key, &settings()).unwrap();
    assert!(!viewpoint
        .scene_context
        .resources
        .iter()
        .any(|r| r.node_id == secret_id));
    let before = p.view().unwrap();
    let mut world = before.world;
    world
        .nodes
        .iter_mut()
        .find(|n| n.id == ids.0)
        .unwrap()
        .components
        .get_mut("core.narration-style")
        .unwrap()
        .insert(
            "knowledgePerspective".into(),
            PropertyValue::Text("Author / omniscient".into()),
        );
    p.commit(before.revision, id(), before.title, world)
        .unwrap();
    let author = p.passage_view(&key, &settings()).unwrap();
    assert_eq!(
        author.resolved_plan.unwrap().knowledge_perspective,
        KnowledgePerspective::Author
    );
    assert!(author
        .scene_context
        .resources
        .iter()
        .any(|r| r.node_id == secret_id));
}

#[test]
fn malformed_consumed_direction_keys_are_rejected_before_saving() {
    let (_dir, p) = fixture();
    let mut edit = edits(&p, "");
    edit.plan.consumed_brief_passage = Some("invalid".into());
    assert!(p
        .save_passage(&id(), 0, edit)
        .unwrap_err()
        .contains("consumed brief"));
}

#[test]
fn event_replacement_can_edit_the_captured_plan_without_rewriting_or_repeating_the_brief() {
    let (_dir, p) = fixture();
    let ids = sources(&p);
    let accepted = accept(&p, selected(&p, &ids, "Jon examines the latch."));
    let original = p.read_passage(&accepted).unwrap();
    let key = id();
    p.begin_passage_revision(
        &key,
        &accepted,
        original.record,
        PassageRevisionKind::Events,
    )
    .unwrap();
    let mut revision = p.passage_view(&key, &settings()).unwrap();
    revision.edits.plan.direction = "Jon decides to leave the latch alone.".into();
    revision.edits.prose = "Jon leaves the latch alone.".into();
    p.save_passage(&key, revision.version, revision.edits)
        .unwrap();
    let revision = p.passage_view(&key, &settings()).unwrap();
    assert_eq!(
        revision.resolved_plan.unwrap().direction,
        "Jon decides to leave the latch alone."
    );
    p.validate_passage(&key, revision.version, &settings())
        .unwrap();
    let revision = p.passage_view(&key, &settings()).unwrap();
    p.accept_passage(&key, revision.version, &settings())
        .unwrap();
    let next = p.next_passage_plan().unwrap();
    let world = p.view().unwrap().world;
    assert!(super::super::writing_inputs::resolve(&world, &next)
        .unwrap()
        .direction
        .is_empty());
    assert_eq!(
        world
            .nodes
            .iter()
            .find(|n| n.id == ids.1)
            .unwrap()
            .components["detail.brief"]["passageDirection"],
        PropertyValue::Text("Jon examines the latch.".into())
    );
}

#[test]
#[ignore = "explicit isolated offline World writing-input fixture"]
fn writing_inputs_desktop_fixture() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join(format!("writing-inputs-ui-{}", id()));
    fs::create_dir_all(&base).unwrap();
    let (_dir, seed) = fixture();
    let p = Project::create(
        &base.join("story"),
        "Fictional World writing inputs".into(),
        seed.view().unwrap().world,
        id(),
    )
    .unwrap();
    let ids = sources(&p);
    p.save_passage(&id(), 0, selected(&p, &ids, "")).unwrap();
    crate::save_connection(
        &base.join("config"),
        0,
        settings(),
        crate::ApiKeyUpdate::Keep,
    )
    .unwrap();
    println!("Offline UI fixture: {}", base.display());
}
