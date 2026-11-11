use super::*;
use crate::media::tests::{accept_passage, fixture, png};
fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn text() -> LlmConnection {
    LlmConnection {
        provider: TextProvider::Demo,
        ..Default::default()
    }
}
fn style(p: &Project) -> String {
    let mut v = p.view().unwrap();
    let def = v
        .world
        .component_types
        .iter()
        .find(|d| d.id == "detail.visual-style")
        .unwrap();
    let key = id();
    let mut values: BTreeMap<_, _> = def
        .fields
        .iter()
        .map(|f| (f.key.clone(), f.default_value.clone()))
        .collect();
    values.insert(
        "palette".into(),
        PropertyValue::Text("Ultramarine and amber.".into()),
    );
    v.world.nodes.push(WorldNode {
        id: key.clone(),
        type_id: "core.visual-style".into(),
        name: "Watercolour style".into(),
        description: String::new(),
        visual_description: String::new(),
        components: BTreeMap::from([("detail.visual-style".into(), values)]),
    });
    v.world.ai_context.push(AiContextBinding {
        node_id: key.clone(),
        enabled: true,
        role: AiContextRole::Instructions,
        tasks: vec![TextPromptTask::ImagePrompt],
        fields: vec![AiContextField::Components],
        order: -10,
        image_preparation: true,
    });
    p.commit(v.revision, id(), v.title, v.world).unwrap();
    key
}
fn change_style(p: &Project, key: &str, value: &str) {
    let mut v = p.view().unwrap();
    v.world
        .nodes
        .iter_mut()
        .find(|n| n.id == key)
        .unwrap()
        .components
        .get_mut("detail.visual-style")
        .unwrap()
        .insert("palette".into(), PropertyValue::Text(value.into()));
    p.commit(v.revision, id(), v.title, v.world).unwrap();
}
fn portrait(p: &Project) -> String {
    let key = id();
    p.create_media(
        &key,
        MediaTarget {
            node_id: Some(p.view().unwrap().world.nodes[1].id.clone()),
            passage_id: None,
            moment: ImageMoment::After,
            subject_ids: vec![],
            guidance_source: None,
        },
    )
    .unwrap();
    key
}
fn historical(p: &Project, passage: &str, guidance_source: Option<ImageGuidanceSource>) -> String {
    let key = id();
    p.create_media(
        &key,
        MediaTarget {
            node_id: None,
            passage_id: Some(passage.into()),
            moment: ImageMoment::After,
            subject_ids: vec![p.view().unwrap().world.nodes[1].id.clone()],
            guidance_source,
        },
    )
    .unwrap();
    key
}
fn import_accept(p: &Project, key: &str, root: &Path) {
    let file = root.join(format!("{key}.png"));
    fs::write(&file, png()).unwrap();
    let d = p.media_draft(key).unwrap();
    let v = p.import_media_image(key, d.version, &file).unwrap();
    p.accept_media(key, v.draft.version).unwrap();
}

#[test]
fn direct_and_optional_ai_use_the_same_guidance_once_without_rewriting_the_saved_prompt() {
    let (_dir, p) = fixture();
    style(&p);
    let key = portrait(&p);
    let d = p.media_draft(&key).unwrap();
    assert!(!d.request.prompt.contains("Ultramarine"));
    assert!(!d.source.facts[0].appearance.contains("Private planning"));
    let preview = p.preview_image_preparation(&key, false).unwrap();
    assert!(preview.error.is_empty());
    assert_eq!(preview.sections.len(), 1);
    let image = ImageConnection::default();
    let submitted = p.prepare_media(&key, 1, &image).unwrap();
    let b = &image.profile.bindings.prompt;
    assert_eq!(
        submitted.graph[&b.node_id]["inputs"][&b.input],
        preview.prompt
    );
    let mut effective = d.request.clone();
    effective.prompt = preview.prompt.clone();
    assert_eq!(submitted.graph, image.profile.prepare(&effective).unwrap());
    assert_eq!(p.media_draft(&key).unwrap().request, d.request);
    let proposal = id();
    p.create_image_prompt(&proposal, &key, 1, &text(), &image)
        .unwrap();
    let request = p.start_image_prompt(&proposal, 1, &text(), &image).unwrap();
    assert_eq!(request.system.matches("Ultramarine and amber.").count(), 1);
    assert!(!request.user.contains("Ultramarine and amber."));
    assert!(request.user.contains("nativeImageGuidanceNodes"));
    let mut world = p.view().unwrap().world;
    assert_eq!(
        resolve_ai_context(&world, TextPromptTask::ImagePrompt).unwrap(),
        preview.sections
    );
    assert!(resolve_ai_context(&world, TextPromptTask::Passage)
        .unwrap()
        .is_empty());
    world.ai_context[0].tasks.clear();
    assert_eq!(
        resolve_ai_context(&world, TextPromptTask::ImagePrompt).unwrap(),
        preview.sections
    );
}

#[test]
fn guidance_refresh_preserves_physical_facts_controls_receipts_and_unrelated_edits() {
    let (dir, p) = fixture();
    let style = style(&p);
    let key = portrait(&p);
    let d = p.media_draft(&key).unwrap();
    let mut request = d.request.clone();
    request.prompt = "Manual composition, keep the blue coat.".into();
    request.seed = Some(57);
    request.width = Some(768);
    request.steps = Some(18);
    let v = p.save_media_request(&key, 1, request.clone()).unwrap();
    let accepted = portrait(&p);
    import_accept(&p, &accepted, dir.path());
    change_style(&p, &style, "Ochre washes.");
    let changed = p.media_view(&key).unwrap();
    assert!(changed.guidance_stale && !changed.stale);
    assert!(p
        .prepare_media(&key, v.draft.version, &ImageConnection::default())
        .unwrap_err()
        .contains("guidance changed"));
    let asset = p.media_asset(&accepted).unwrap();
    assert!(p.media_applicable(&asset.target, &asset.source));
    assert!(p
        .preview_image_preparation(&accepted, true)
        .unwrap()
        .prompt
        .contains("Ultramarine"));
    let refreshed = p.refresh_image_guidance(&key, v.draft.version).unwrap();
    assert_eq!(refreshed.draft.version, v.draft.version + 1);
    assert!(!refreshed.guidance_stale);
    assert_eq!(refreshed.draft.request, request);
    assert_eq!(refreshed.draft.source.world, d.source.world);
    assert_eq!(refreshed.draft.source.facts, d.source.facts);
    assert_eq!(refreshed.draft.prompt_receipts, d.prompt_receipts);
    assert!(refreshed
        .preparation
        .unwrap()
        .prompt
        .contains("Ochre washes."));
    assert_eq!(
        p.refresh_image_guidance(&key, v.draft.version)
            .unwrap()
            .draft
            .version,
        refreshed.draft.version
    );
    let capture = refreshed.draft.source.image_context;
    let mut current = p.view().unwrap();
    current.world.nodes[0].description = "Unrelated planning edit".into();
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let unchanged = p
        .refresh_image_guidance(&key, refreshed.draft.version)
        .unwrap();
    assert_eq!(unchanged.draft.version, refreshed.draft.version);
    assert_eq!(unchanged.draft.source.image_context, capture);
    let submission = p
        .prepare_media(&key, unchanged.draft.version, &ImageConnection::default())
        .unwrap();
    p.begin_media_submission(&key, unchanged.draft.version, submission)
        .unwrap();
    assert!(p
        .refresh_image_guidance(&key, unchanged.draft.version + 1)
        .unwrap_err()
        .contains("unsent"));
}

#[test]
fn historical_guidance_defaults_to_its_saved_boundary_and_current_is_explicit() {
    let (_dir, p) = fixture();
    let style = style(&p);
    let passage = accept_passage(&p, vec![]);
    change_style(&p, &style, "Future silver etching.");
    let saved = historical(&p, &passage, None);
    let current = historical(&p, &passage, Some(ImageGuidanceSource::Current));
    let a = p.media_view(&saved).unwrap();
    let b = p.media_view(&current).unwrap();
    assert_eq!(a.draft.source.facts, b.draft.source.facts);
    assert_eq!(a.draft.source.world, b.draft.source.world);
    assert!(a.preparation.unwrap().prompt.contains("Ultramarine"));
    assert!(b.preparation.unwrap().prompt.contains("Future silver"));
    let image = ImageConnection::default();
    let proposal = id();
    p.create_image_prompt(&proposal, &current, 1, &text(), &image)
        .unwrap();
    let request = p.start_image_prompt(&proposal, 1, &text(), &image).unwrap();
    assert!(request.system.contains("Future silver"));
    p.finish_image_prompt(&proposal, Ok(request.demo), false, false)
        .unwrap();
    change_style(&p, &style, "Later charcoal.");
    assert!(!p.media_view(&saved).unwrap().guidance_stale);
    assert!(p.media_view(&current).unwrap().guidance_stale);
    assert!(
        p.image_prompt_view(&proposal, &text(), &image)
            .unwrap()
            .stale
    );
    p.refresh_image_guidance(&current, 1).unwrap();
    assert!(
        p.image_prompt_view(&proposal, &text(), &image)
            .unwrap()
            .stale
    );
}

#[test]
fn forks_and_backups_preserve_captures_but_forks_exclude_future_guidance() {
    let (dir, p) = fixture();
    let style = style(&p);
    let passage = accept_passage(&p, vec![]);
    let saved = historical(&p, &passage, None);
    import_accept(&p, &saved, dir.path());
    change_style(&p, &style, "Future graphite.");
    let future = historical(&p, &passage, Some(ImageGuidanceSource::Current));
    import_accept(&p, &future, dir.path());
    let pending = portrait(&p);
    let fork = p
        .fork_after_passage(&passage, dir.path(), "Retained guidance".into())
        .unwrap();
    assert!(fork.story_index().unwrap().media.contains_key(&saved));
    assert!(!fork.story_index().unwrap().media.contains_key(&future));
    assert!(fork
        .preview_image_preparation(&saved, true)
        .unwrap()
        .prompt
        .contains("Ultramarine"));
    let export = dir.path().join("exports");
    fs::create_dir(&export).unwrap();
    let backup = p
        .export_backup(&export, &id(), p.view().unwrap().revision)
        .unwrap();
    let review = inspect_backup(Path::new(&backup.folder_path)).unwrap();
    let parent = dir.path().join("restore");
    fs::create_dir(&parent).unwrap();
    let restored = restore_backup(
        Path::new(&backup.folder_path),
        &parent,
        &id(),
        &review.fingerprint,
    )
    .unwrap();
    let restored = Project::open(Path::new(&restored.folder_path)).unwrap();
    assert!(restored
        .preview_image_preparation(&saved, true)
        .unwrap()
        .prompt
        .contains("Ultramarine"));
    assert!(restored
        .preview_image_preparation(&future, true)
        .unwrap()
        .prompt
        .contains("Future graphite"));
    assert!(restored
        .preview_image_preparation(&pending, false)
        .unwrap()
        .prompt
        .contains("Future graphite"));
    drop(p);
    let source = dir.path().join("story");
    crate::store::publish_new_path(&source, &dir.path().join("unavailable-source")).unwrap();
    let asset = fork.media_asset(&saved).unwrap();
    let draft = restored.media_draft(&pending).unwrap();
    assert!(fork
        .validate_image_capture(&asset.target, &asset.source)
        .is_ok());
    assert!(restored
        .validate_image_capture(&draft.target, &draft.source)
        .is_ok());
}

#[test]
fn empty_and_oversized_guidance_are_editable_but_never_silently_dropped() {
    let (dir, p) = fixture();
    let style = style(&p);
    let mut v = p.view().unwrap();
    v.world
        .nodes
        .iter_mut()
        .find(|n| n.id == style)
        .unwrap()
        .components
        .get_mut("detail.visual-style")
        .unwrap()
        .values_mut()
        .for_each(|v| *v = PropertyValue::Null);
    p.commit(v.revision, id(), v.title, v.world).unwrap();
    let key = portrait(&p);
    let preview = p.preview_image_preparation(&key, false).unwrap();
    assert!(preview.error.contains("Watercolour style"));
    assert!(p
        .prepare_media(&key, 1, &ImageConnection::default())
        .unwrap_err()
        .contains("no selected content"));
    import_accept(&p, &key, dir.path());
    assert!(p.media_asset(&key).is_ok());
    change_style(&p, &style, &"Large palette instructions. ".repeat(350));
    let mut v = p.view().unwrap();
    v.world
        .nodes
        .iter_mut()
        .find(|n| n.id == style)
        .unwrap()
        .components
        .get_mut("detail.visual-style")
        .unwrap()
        .insert(
            "lighting".into(),
            PropertyValue::Text("Large lighting instructions. ".repeat(350)),
        );
    p.commit(v.revision, id(), v.title, v.world).unwrap();
    let key = portrait(&p);
    let error = p
        .prepare_media(&key, 1, &ImageConnection::default())
        .unwrap_err();
    assert!(error.contains("16,000") && error.contains("Watercolour style"));
}

#[test]
fn capture_tampering_is_rejected_and_legacy_optional_fields_remain_omitted() {
    let (_dir, p) = fixture();
    let key = portrait(&p);
    let d = p.media_draft(&key).unwrap();
    let source = serde_json::to_value(&d.source).unwrap();
    assert!(source.get("imageContext").is_none());
    assert!(serde_json::to_value(&d.target)
        .unwrap()
        .get("guidanceSource")
        .is_none());
    style(&p);
    let key = portrait(&p);
    let mut d = p.media_draft(&key).unwrap();
    d.source.image_context.as_mut().unwrap().sections[0].content =
        "Forged unsaved instructions.".into();
    p.write_media(&d).unwrap();
    assert!(p.media_draft(&key).unwrap_err().contains("does not match"));
}

#[test]
fn historical_capture_cannot_silently_substitute_a_later_valid_world() {
    let (dir, p) = fixture();
    let style = style(&p);
    let passage = accept_passage(&p, vec![]);
    let saved = historical(&p, &passage, None);
    change_style(&p, &style, "Future-only palette.");
    let current = historical(&p, &passage, Some(ImageGuidanceSource::Current));
    let mut forged = p.media_draft(&saved).unwrap();
    forged.source.image_context = p.media_draft(&current).unwrap().source.image_context;
    p.write_media(&forged).unwrap();
    assert!(p
        .media_draft(&saved)
        .unwrap_err()
        .contains("explicit current-world"));
    let export = dir.path().join("exports");
    fs::create_dir(&export).unwrap();
    assert!(p
        .export_backup(&export, &id(), p.view().unwrap().revision)
        .unwrap_err()
        .contains("explicit current-world"));
}

#[test]
fn image_only_inclusion_does_not_stale_narrative_checks_and_does_not_expand_neighbors() {
    let (_dir, p) = fixture();
    let style = style(&p);
    let mut v = p.view().unwrap();
    let before = crate::ai_context::story_ai_context_stamp(&v.world).unwrap();
    let binding = v
        .world
        .ai_context
        .iter_mut()
        .find(|b| b.node_id == style)
        .unwrap();
    binding.tasks = vec![TextPromptTask::Passage];
    let before_with_passage = crate::ai_context::story_ai_context_stamp(&v.world).unwrap();
    v.world.ai_context[0].image_preparation = false;
    assert_eq!(
        before_with_passage,
        crate::ai_context::story_ai_context_stamp(&v.world).unwrap()
    );
    assert!(before.is_none());
    let direct = preview_image_context(&v.world).unwrap();
    assert!(direct.sections.is_empty());
    assert!(serde_json::to_value(&v.world.ai_context[0])
        .unwrap()
        .get("imagePreparation")
        .is_none());
}

#[test]
fn prose_schema_excludes_unused_author_profiles_and_effects_cannot_rewrite_planning() {
    let (_dir, p) = fixture();
    let mut world = p.view().unwrap().world;
    let projected = crate::story_effects::prose_effect_schema(&world);
    assert!(!projected
        .node_types
        .iter()
        .any(|d| d.id == "core.visual-style"));
    assert!(projected
        .component_types
        .iter()
        .any(|d| d.id == "detail.appearance"));
    let mut visual = world
        .component_types
        .iter()
        .find(|d| d.id == "detail.visual-style")
        .unwrap()
        .clone();
    visual.id = "custom.rendering".into();
    world.component_types.push(visual);
    world.nodes[1].components.insert(
        "custom.rendering".into(),
        BTreeMap::from([(
            "palette".into(),
            PropertyValue::Text("Saved palette.".into()),
        )]),
    );
    let effect = |node: WorldNode| PassageEffects {
        changes: vec![PassageEffect::PutNode {
            node,
            evidence: "Mara waited by the window.".into(),
        }],
        ..Default::default()
    };
    let mut node = world.nodes[1].clone();
    node.description = "Changed story facts".into();
    assert!(
        crate::story_effects::effect_record_issues(&world, &world, &effect(node.clone()))[0]
            .is_empty()
    );
    node.components.remove("custom.rendering");
    assert!(
        crate::story_effects::effect_record_issues(&world, &world, &effect(node))[0]
            .iter()
            .any(|e| e.contains("author planning guidance"))
    );
    let style = style(&p);
    let world = p.view().unwrap().world;
    let mut node = world.nodes.iter().find(|n| n.id == style).unwrap().clone();
    node.components.clear();
    node.type_id = "core.note".into();
    assert!(
        crate::story_effects::effect_record_issues(&world, &world, &effect(node))[0]
            .iter()
            .any(|e| e.contains("Author-only"))
    );
}
