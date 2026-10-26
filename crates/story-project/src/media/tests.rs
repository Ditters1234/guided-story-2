use super::*;
fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn node(world: &World, kind: &str, name: &str) -> WorldNode {
    let preset = world.node_types.iter().find(|t| t.id == kind).unwrap();
    WorldNode {
        id: id(),
        type_id: kind.into(),
        name: name.into(),
        description: "Private planning notes must not become an image prompt.".into(),
        visual_description: format!("Visible {name}"),
        components: preset
            .components
            .iter()
            .map(|id| {
                let d = world.component_types.iter().find(|c| &c.id == id).unwrap();
                (
                    id.clone(),
                    d.fields
                        .iter()
                        .map(|f| (f.key.clone(), f.default_value.clone()))
                        .collect(),
                )
            })
            .collect(),
    }
}
pub(super) fn fixture() -> (tempfile::TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let mut world = World::default();
    for (kind, name) in [
        ("core.place", "Glasshouse"),
        ("core.character", "Mara"),
        ("core.item", "Blue coat"),
    ] {
        world.nodes.push(node(&world, kind, name));
    }
    world.nodes[2].components.insert(
        "core.wearable".into(),
        BTreeMap::from([
            ("regions".into(), PropertyValue::Text("torso".into())),
            ("layer".into(), PropertyValue::Number(2.0)),
        ]),
    );
    for (kind, source, target) in [
        ("core.present-at", 1, 0),
        ("core.stored-in", 2, 1),
        ("core.worn-by", 2, 1),
    ] {
        world.edges.push(WorldEdge {
            id: id(),
            type_id: kind.into(),
            source_id: world.nodes[source].id.clone(),
            target_id: world.nodes[target].id.clone(),
            description: String::new(),
        });
    }
    let p = Project::create(
        &dir.path().join("story"),
        "Fictional media test".into(),
        world,
        id(),
    )
    .unwrap();
    (dir, p)
}
fn target(p: &Project) -> MediaTarget {
    MediaTarget {
        node_id: Some(p.view().unwrap().world.nodes[1].id.clone()),
        passage_id: None,
        moment: ImageMoment::After,
        subject_ids: vec![],
    }
}
pub(super) fn png() -> Vec<u8> {
    let image = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        16,
        24,
        image::Rgb([80, 140, 180]),
    ));
    let mut bytes = Cursor::new(vec![]);
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    bytes.into_inner()
}
fn ready(p: &Project, target: MediaTarget) -> String {
    let key = id();
    p.create_media(&key, target).unwrap();
    let mut d = p.media_draft(&key).unwrap();
    d.image = Some(p.save_image_bytes(&png()).unwrap());
    d.stage = MediaStage::Ready;
    p.write_media(&d).unwrap();
    key
}
pub(super) fn accept_passage(p: &Project, changes: Vec<PassageEffect>) -> String {
    let world = p.view().unwrap().world;
    let mut plan = p.next_passage_plan().unwrap();
    plan.direction = "A quiet fictional moment.".into();
    plan.viewpoint_id = Some(world.nodes[1].id.clone());
    plan.location_id = Some(world.nodes[0].id.clone());
    let draft = id();
    p.save_passage(
        &draft,
        0,
        PassageEdits {
            plan,
            prose: "Mara removed the blue coat and waited by the window.".into(),
            effects_text: serde_json::to_string(&PassageEffects {
                changes,
                ..Default::default()
            })
            .unwrap(),
            close_chapter: false,
        },
    )
    .unwrap();
    let settings = LlmConnection {
        provider: TextProvider::Demo,
        ..Default::default()
    };
    p.validate_passage(&draft, 1, &settings).unwrap();
    p.accept_passage(
        &draft,
        p.passage_view(&draft, &settings).unwrap().version,
        &settings,
    )
    .unwrap();
    draft
}
fn queued(p: &Project) -> (String, MediaSubmission) {
    let key = id();
    p.create_media(&key, target(p)).unwrap();
    let submission = p
        .prepare_media(&key, 1, &ImageConnection::default())
        .unwrap();
    p.begin_media_submission(&key, 1, submission.clone())
        .unwrap();
    p.finish_media_submission(&key, &ComfySubmission::Queued("remote-123".into()))
        .unwrap();
    (
        key.clone(),
        p.media_draft(&key).unwrap().submission.unwrap(),
    )
}

#[test]
fn import_is_a_candidate_until_accept_and_receipts_preserve_later_work() {
    let (dir, p) = fixture();
    let before = p.view().unwrap();
    let key = id();
    p.create_media(&key, target(&p)).unwrap();
    let file = dir.path().join("fictional.png");
    fs::write(&file, png()).unwrap();
    let view = p.import_media_image(&key, 1, &file).unwrap();
    assert_eq!(p.view().unwrap().revision, 0);
    assert!(p.story_index().unwrap().media.is_empty());
    let result = p.accept_media(&key, view.draft.version).unwrap();
    assert_eq!(result.committed_revision, 1);
    assert_eq!(before.world, result.project.world);
    let saved = p.media_asset(&key).unwrap();
    assert!(saved.imported);
    assert_eq!(saved.image.extension, "png");
    assert!(p.media_preview(&key, true).is_ok());
    let mut current = p.view().unwrap();
    current.world.nodes[0].description = "Later manual work".into();
    p.commit(current.revision, id(), current.title, current.world.clone())
        .unwrap();
    let replay = p.accept_media(&key, view.draft.version).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.committed_revision, 1);
    assert_eq!(replay.project.revision, 2);
    assert_eq!(replay.project.world, current.world);
    assert_eq!(
        p.selected_portrait(&target(&p).node_id.unwrap(), &current.world)
            .unwrap()
            .unwrap()
            .id,
        key
    );
    drop(p);
    let reopened = Project::open(&dir.path().join("story")).unwrap();
    assert_eq!(reopened.list_media(None).unwrap().len(), 1);
    assert!(reopened.media_preview(&key, true).is_ok());
}

#[test]
fn capture_uses_historical_outfit_and_never_copies_private_notes() {
    let (_dir, p) = fixture();
    let before = p.view().unwrap().world;
    let worn = before
        .edges
        .iter()
        .find(|e| e.type_id == "core.worn-by")
        .unwrap();
    let passage = accept_passage(
        &p,
        vec![PassageEffect::RemoveLink {
            id: worn.id.clone(),
            evidence: "Mara removed the blue coat".into(),
        }],
    );
    let shot = |moment| MediaTarget {
        node_id: None,
        passage_id: Some(passage.clone()),
        moment,
        subject_ids: vec![before.nodes[1].id.clone()],
    };
    let first = p.create_media(&id(), shot(ImageMoment::Before)).unwrap();
    let last = p.create_media(&id(), shot(ImageMoment::After)).unwrap();
    assert_eq!(first.draft.source.facts[0].outfit.len(), 1);
    assert!(last.draft.source.facts[0].outfit.is_empty());
    assert!(!first.draft.request.prompt.contains("Private planning"));
    let mut current = p.view().unwrap();
    current.world.nodes[1].visual_description = "Future appearance".into();
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let historical = p.create_media(&id(), shot(ImageMoment::Before)).unwrap();
    assert_eq!(first.draft.source, historical.draft.source);
    assert!(!historical
        .draft
        .request
        .prompt
        .contains("Future appearance"));
}

#[test]
fn only_visible_source_changes_stale_node_shots_and_historical_portraits_use_placeholders() {
    let (_dir, p) = fixture();
    let key = id();
    let selected = target(&p);
    let before = p.view().unwrap().world;
    p.create_media(&key, selected.clone()).unwrap();
    let mut current = p.view().unwrap();
    current.world.nodes[0].description = "Unrelated edit".into();
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    assert!(!p.media_view(&key).unwrap().stale);
    let accepted = ready(&p, selected.clone());
    p.accept_media(&accepted, 1).unwrap();
    let mut current = p.view().unwrap();
    current.world.nodes[2].visual_description = "A red coat".into();
    p.commit(current.revision, id(), current.title, current.world.clone())
        .unwrap();
    assert!(p.media_view(&key).unwrap().stale);
    assert!(p
        .prepare_media(&key, 1, &ImageConnection::default())
        .is_err());
    let node = selected.node_id.unwrap();
    assert!(
        !p.selected_portrait(&node, &current.world)
            .unwrap()
            .unwrap()
            .applicable
    );
    assert!(
        p.selected_portrait(&node, &before)
            .unwrap()
            .unwrap()
            .applicable
    );
}

#[test]
fn uncertain_submission_restart_keeps_exact_endpoint_graph_and_never_requeues() {
    let (dir, p) = fixture();
    let key = id();
    p.create_media(&key, target(&p)).unwrap();
    let mut settings = ImageConnection::default();
    settings.base_url = "http://127.0.0.1:8189/original".into();
    let submission = p.prepare_media(&key, 1, &settings).unwrap();
    p.begin_media_submission(&key, 1, submission.clone())
        .unwrap();
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    p.interrupt_media().unwrap();
    let d = p.media_draft(&key).unwrap();
    assert_eq!(d.stage, MediaStage::Unknown);
    assert_eq!(d.submission.unwrap(), submission);
    assert!(p
        .prepare_media(&key, d.version, &ImageConnection::default())
        .is_err());
    assert!(p
        .attach_media_prompt(&key, d.version, "../bad".into())
        .is_err());
    let recovered = p
        .attach_media_prompt(&key, d.version, "recovered-id".into())
        .unwrap();
    assert_eq!(recovered.draft.stage, MediaStage::Queued);
    assert_eq!(
        recovered.draft.submission.unwrap().endpoint,
        settings.base_url
    );
}

#[test]
fn queued_job_survives_restart_and_download_is_not_accepted_automatically() {
    let (dir, p) = fixture();
    let (key, submission) = queued(&p);
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    p.interrupt_media().unwrap();
    let d = p.media_draft(&key).unwrap();
    assert_eq!(d.stage, MediaStage::Queued);
    assert_eq!(d.submission.unwrap(), submission);
    let pending = p
        .finish_media_image(&key, d.version, &ComfyResult::Pending)
        .unwrap();
    assert_eq!(pending.draft.stage, MediaStage::Queued);
    let done = p
        .finish_media_image(&key, pending.draft.version, &ComfyResult::Image(png()))
        .unwrap();
    assert_eq!(done.draft.stage, MediaStage::Ready);
    assert_eq!(p.view().unwrap().revision, 0);
    assert!(p.story_index().unwrap().media.is_empty());
    assert!(p.media_preview(&key, false).is_ok());
}

#[test]
fn corrupt_formats_and_assets_are_bounded_and_never_reset_the_world() {
    let (_dir, p) = fixture();
    assert!(decode_image(&vec![0; MAX_IMAGE + 1]).is_err());
    assert!(decode_image(b"not an image").is_err());
    assert!(decode_image(&png()[..20]).is_err());
    for format in [
        image::ImageFormat::Png,
        image::ImageFormat::Jpeg,
        image::ImageFormat::WebP,
    ] {
        let image = image::DynamicImage::ImageRgb8(image::RgbImage::new(12, 12));
        let mut encoded = Cursor::new(vec![]);
        image.write_to(&mut encoded, format).unwrap();
        assert!(decode_image(&encoded.into_inner()).is_ok());
    }
    let key = ready(&p, target(&p));
    p.accept_media(&key, 1).unwrap();
    let image = p.media_asset(&key).unwrap().image;
    fs::write(
        p.root_path()
            .join("images")
            .join(format!("{}.{}", image.id, image.extension)),
        b"damaged",
    )
    .unwrap();
    assert!(p.media_preview(&key, true).is_err());
    assert_eq!(p.view().unwrap().revision, 1);
    let invalid = ImageFile {
        id: "../escape".into(),
        ..image
    };
    assert!(p.read_image_bytes(&invalid).is_err());
}

#[test]
fn media_metadata_does_not_invalidate_checked_passage_context() {
    let (_dir, p) = fixture();
    let draft = id();
    let mut plan = p.next_passage_plan().unwrap();
    plan.direction = "Wait in the Glasshouse.".into();
    p.save_passage(
        &draft,
        0,
        PassageEdits {
            plan,
            prose: "The room grew quiet as rain touched the glass.".into(),
            effects_text: empty_effects(),
            close_chapter: false,
        },
    )
    .unwrap();
    let settings = LlmConnection {
        provider: TextProvider::Demo,
        ..Default::default()
    };
    p.validate_passage(&draft, 1, &settings).unwrap();
    assert!(p.passage_view(&draft, &settings).unwrap().can_accept);
    let key = ready(&p, target(&p));
    p.accept_media(&key, 1).unwrap();
    assert!(p.passage_view(&draft, &settings).unwrap().can_accept);
    let node = target(&p).node_id.unwrap();
    p.clear_portrait(&node, id()).unwrap();
    assert!(p
        .selected_portrait(&node, &p.view().unwrap().world)
        .unwrap()
        .is_none());
    assert!(p.media_preview(&key, true).is_ok());
    assert!(p.passage_view(&draft, &settings).unwrap().can_accept);
    p.select_portrait(&node, &key, id()).unwrap();
    assert!(p.passage_view(&draft, &settings).unwrap().can_accept);
}

#[test]
fn forks_copy_retained_media_without_future_outfits_or_source_folder_dependency() {
    let (dir, p) = fixture();
    let node = target(&p).node_id.unwrap();
    let portrait = ready(&p, target(&p));
    p.accept_media(&portrait, 1).unwrap();
    let first = accept_passage(&p, vec![]);
    let first_target = MediaTarget {
        node_id: None,
        passage_id: Some(first.clone()),
        moment: ImageMoment::After,
        subject_ids: vec![node.clone()],
    };
    let illustration = ready(&p, first_target);
    p.accept_media(&illustration, 1).unwrap();
    let mut current = p.view().unwrap();
    current.world.nodes[1].visual_description = "Future golden mask".into();
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let future = ready(&p, target(&p));
    p.accept_media(&future, 1).unwrap();
    let second = accept_passage(&p, vec![]);
    let future_illustration = ready(
        &p,
        MediaTarget {
            node_id: None,
            passage_id: Some(second),
            moment: ImageMoment::After,
            subject_ids: vec![node.clone()],
        },
    );
    p.accept_media(&future_illustration, 1).unwrap();
    let pending = id();
    p.create_media(&pending, target(&p)).unwrap();
    let fork = p
        .fork_after_passage(&first, dir.path(), "Independent image fork".into())
        .unwrap();
    let index = fork.story_index().unwrap();
    assert!(index.media.contains_key(&portrait));
    assert!(index.media.contains_key(&illustration));
    assert!(!index.media.contains_key(&future));
    assert!(!index.media.contains_key(&future_illustration));
    assert!(!index.portraits.contains_key(&node));
    assert!(!fork.root_path().join("media-drafts").exists());
    let original = fork.media_asset(&illustration).unwrap();
    assert_eq!(
        original.source.passage.as_ref(),
        Some(&index.passages[0].record)
    );
    assert!(fork.media_preview(&illustration, true).is_ok());
    drop(p);
    let moved = dir.path().join("source-unavailable");
    for attempt in 0..40 {
        match fs::rename(dir.path().join("story"), &moved) {
            Ok(()) => break,
            Err(e) if attempt < 39 && matches!(e.raw_os_error(), Some(5 | 32 | 33)) => {
                std::thread::sleep(std::time::Duration::from_millis(50))
            }
            Err(e) => panic!("{e}"),
        }
    }
    assert!(fork.media_preview(&illustration, true).is_ok());
    assert!(fork.media_preview(&portrait, true).is_ok());
}

#[test]
fn settings_preserve_model_specific_graphs_and_are_versioned_separately() {
    let (dir, p) = fixture();
    let root = dir.path().join("preferences");
    let mut settings = load_image_connection(&root).unwrap();
    let original = settings.profile.workflow.clone();
    settings.base_url = "http://127.0.0.1:8189/comfy".into();
    let saved = save_image_connection(&root, 0, settings.clone()).unwrap();
    assert_eq!(saved.version, 1);
    assert_eq!(saved.profile.workflow, original);
    assert_eq!(
        save_image_connection(&root, 0, settings).unwrap().version,
        1
    );
    let mut changed = saved.clone();
    changed.base_url = "http://127.0.0.1:8190".into();
    assert!(save_image_connection(&root, 0, changed).is_err());
    assert_eq!(load_image_connection(&root).unwrap(), saved);
    assert_eq!(p.view().unwrap().revision, 0);
    assert!(!p.root_path().join("image-connection.json").exists());
    for url in [
        "file:///tmp",
        "http://user:secret@localhost",
        "http://localhost?key=secret",
    ] {
        assert!(image_endpoint(url, "prompt").is_err());
    }
}

#[test]
fn retained_download_replays_after_status_save_or_acceptance_without_duplicate_files() {
    let (_dir, p) = fixture();
    let (key, _) = queued(&p);
    let version = p.media_draft(&key).unwrap().version;
    let response = ComfyResult::Image(png());
    let ready = p.finish_media_image(&key, version, &response).unwrap();
    let replay = p.finish_media_image(&key, version, &response).unwrap();
    assert_eq!(ready.draft.version, replay.draft.version);
    assert_eq!(ready.draft.image, replay.draft.image);
    assert_eq!(
        p.finish_media_image(&key, version, &ComfyResult::Pending)
            .unwrap()
            .draft
            .stage,
        MediaStage::Ready
    );
    p.accept_media(&key, ready.draft.version).unwrap();
    assert_eq!(
        p.finish_media_image(&key, version, &response)
            .unwrap()
            .draft
            .stage,
        MediaStage::Accepted
    );
    assert_eq!(
        fs::read_dir(p.root_path().join("images")).unwrap().count(),
        1
    );
}

#[test]
fn wording_revision_keeps_old_illustration_and_fork_excludes_its_superseded_source() {
    let (dir, p) = fixture();
    let passage = accept_passage(&p, vec![]);
    let key = ready(
        &p,
        MediaTarget {
            node_id: None,
            passage_id: Some(passage.clone()),
            moment: ImageMoment::After,
            subject_ids: vec![],
        },
    );
    p.accept_media(&key, 1).unwrap();
    let old = p.media_asset(&key).unwrap();
    let reading = p.read_passage(&passage).unwrap();
    assert!(reading.replacement_error.is_empty());
    let draft = id();
    p.begin_passage_revision(
        &draft,
        &passage,
        reading.record,
        PassageRevisionKind::Wording,
    )
    .unwrap();
    let settings = LlmConnection {
        provider: TextProvider::Demo,
        ..Default::default()
    };
    let view = p.passage_view(&draft, &settings).unwrap();
    let mut edits = view.edits;
    edits.prose = "Mara waited by the window after removing the blue coat.".into();
    p.save_passage(&draft, view.version, edits).unwrap();
    p.validate_passage(
        &draft,
        p.passage_view(&draft, &settings).unwrap().version,
        &settings,
    )
    .unwrap();
    p.accept_passage(
        &draft,
        p.passage_view(&draft, &settings).unwrap().version,
        &settings,
    )
    .unwrap();
    assert!(
        !p.list_media(None)
            .unwrap()
            .into_iter()
            .find(|e| e.id == key)
            .unwrap()
            .applicable
    );
    assert_eq!(p.media_asset(&key).unwrap().source, old.source);
    assert!(p.media_preview(&key, true).is_ok());
    let fork = p
        .fork_after_passage(&passage, dir.path(), "Revised passage fork".into())
        .unwrap();
    assert!(!fork.story_index().unwrap().media.contains_key(&key));
}

#[test]
fn process_exit_publishes_whole_media_index_and_image_once() {
    use std::process::{Command, Stdio};
    for stage in [
        "after-story",
        "after-world",
        "after-manifest",
        "before-head",
        "after-head",
    ] {
        let (dir, p) = fixture();
        let root = p.root_path().to_path_buf();
        let key = ready(&p, target(&p));
        drop(p);
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "media::tests::media_crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("STORY_MEDIA_CRASH_ROOT", &root)
            .env("STORY_MEDIA_CRASH_KEY", &key)
            .env("STORY_MEDIA_CRASH_STAGE", stage)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(75));
        let p = Project::open(&root).unwrap();
        let published = stage == "after-head";
        assert_eq!(p.view().unwrap().revision, u32::from(published));
        assert_eq!(p.story_index().unwrap().media.contains_key(&key), published);
        let candidate = p.media_draft(&key).unwrap();
        assert_eq!(
            candidate.stage,
            if published {
                MediaStage::Accepted
            } else {
                MediaStage::Ready
            }
        );
        let result = p.accept_media(&key, candidate.version).unwrap();
        assert_eq!(result.project.revision, 1);
        assert_eq!(result.replayed, published);
        assert!(p.media_preview(&key, true).is_ok());
        assert!(dir.path().exists());
    }
}

#[test]
#[ignore = "only invoked by the isolated media interruption test"]
fn media_crash_child() {
    let Ok(root) = std::env::var("STORY_MEDIA_CRASH_ROOT") else {
        return;
    };
    let key = std::env::var("STORY_MEDIA_CRASH_KEY").unwrap();
    let stage = std::env::var("STORY_MEDIA_CRASH_STAGE").unwrap();
    let p = Project::open(Path::new(&root)).unwrap();
    let d = p.media_draft(&key).unwrap();
    let current = p.view().unwrap();
    let mut index = p.story_index().unwrap();
    let asset = MediaAsset {
        schema_version: 1,
        id: key.clone(),
        target: d.target.clone(),
        source: d.source,
        request: d.request,
        submission: d.submission,
        image: d.image.unwrap(),
        imported: d.imported,
        prompt_origin: d.prompt_origin,
    };
    index
        .media
        .insert(key.clone(), p.write_record("media", &asset).unwrap());
    index
        .portraits
        .insert(d.target.node_id.unwrap(), key.clone());
    p.commit_bundle(
        current.revision,
        key,
        current.title,
        current.world,
        Some(index),
        |point| {
            if point == stage {
                std::process::exit(75);
            }
            Ok(())
        },
    )
    .unwrap();
    panic!("Interruption point not reached");
}

#[test]
#[ignore = "builds a fictional desktop fixture only when explicitly requested"]
fn media_ui_fixture() {
    let Ok(parent) = std::env::var("STORY_MEDIA_UI_ROOT") else {
        return;
    };
    let parent = Path::new(&parent);
    fs::create_dir_all(parent).unwrap();
    let (_dir, p) = fixture();
    let world = p.view().unwrap().world;
    let project = Project::create(
        &parent.join("project"),
        "Media verification".into(),
        world,
        id(),
    )
    .unwrap();
    accept_passage(&project, vec![]);
    fs::write(parent.join("fictional.png"), png()).unwrap();
    let settings = ImageConnection {
        base_url: std::env::var("STORY_MEDIA_UI_ENDPOINT")
            .unwrap_or_else(|_| "http://127.0.0.1:1".into()),
        ..Default::default()
    };
    save_image_connection(&parent.join("preferences"), 0, settings).unwrap();
    save_preferences(
        &parent.join("preferences"),
        0,
        Preferences {
            theme_id: "dark".into(),
            text_scale: 120,
            ..Default::default()
        },
    )
    .unwrap();
    save_connection(
        &parent.join("preferences"),
        0,
        LlmConnection {
            provider: TextProvider::Demo,
            ..Default::default()
        },
        ApiKeyUpdate::Keep,
    )
    .unwrap();
}

#[test]
fn workflow_default_controls_and_bridge_numbers_are_bounded_before_submission() {
    let (_dir, p) = fixture();
    let mut current = p.view().unwrap();
    current.world.nodes[1].visual_description = "a".repeat(16000);
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let key = id();
    let candidate = p.create_media(&key, target(&p)).unwrap();
    assert!(candidate.draft.request.prompt.is_empty());
    assert_eq!(candidate.draft.source.facts[0].appearance.len(), 16000);
    assert!(!candidate.draft.error.is_empty());
    let repaired = p
        .save_media_request(
            &key,
            1,
            ImageWorkflowRequest {
                prompt: "A concise manual portrait.".into(),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(repaired.draft.error.is_empty());
    let mut profile = guided_story_image_workflow(true).unwrap();
    profile.workflow["6"]["inputs"]["width"] = json!(100000);
    assert!(profile
        .prepare(&ImageWorkflowRequest {
            prompt: "A quiet glasshouse.".into(),
            ..Default::default()
        })
        .is_err());
    profile.workflow["6"]["inputs"]["width"] = json!(896);
    profile.workflow["7"]["inputs"]["seed"] = json!(9_007_199_254_740_992u64);
    assert!(profile
        .prepare(&ImageWorkflowRequest {
            prompt: "A quiet glasshouse.".into(),
            ..Default::default()
        })
        .is_err());
}

#[test]
fn preflight_reports_missing_custom_nodes_and_model_choices_without_rewriting_graph() {
    let profile = guided_story_image_workflow(true).unwrap();
    let mut objects = serde_json::Map::new();
    for node in profile.workflow.as_object().unwrap().values() {
        objects.insert(
            node["class_type"].as_str().unwrap().into(),
            json!({"output_node":true,"input":{"required":{}}}),
        );
    }
    assert!(validate_comfy_objects(&profile.workflow, "9", &Value::Object(objects.clone())).ready);
    objects.remove("SeeSeeCivitaiSaveImage");
    objects.insert(
        "UNETLoader".into(),
        json!({"input":{"required":{"unet_name":[["unavailable.safetensors"]]}}}),
    );
    let report = validate_comfy_objects(&profile.workflow, "9", &Value::Object(objects));
    assert!(!report.ready);
    assert!(report
        .issues
        .iter()
        .any(|s| s.contains("SeeSeeCivitaiSaveImage")));
    assert!(report.issues.iter().any(|s| s.contains("unet_name")));
    assert_eq!(
        profile.workflow,
        guided_story_image_workflow(true).unwrap().workflow
    );
}

async fn server(
    responses: Vec<(String, u16, Vec<u8>)>,
) -> (String, tokio::task::JoinHandle<Vec<String>>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let handle = tokio::spawn(async move {
        let mut requests = vec![];
        for (route, status, bytes) in responses {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut input = vec![];
            let mut chunk = [0; 8192];
            loop {
                let size = stream.read(&mut chunk).await.unwrap();
                if size == 0 {
                    break;
                }
                input.extend_from_slice(&chunk[..size]);
                if let Some(end) = input.windows(4).position(|b| b == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&input[..end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length: ")
                                .and_then(|v| v.parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if input.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let text = String::from_utf8(input).unwrap();
            assert!(
                text.starts_with(&route),
                "Unexpected route: {}",
                text.lines().next().unwrap_or("")
            );
            stream.write_all(format!("HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len()).as_bytes()).await.unwrap();
            stream.write_all(&bytes).await.unwrap();
            requests.push(text);
        }
        requests
    });
    (address, handle)
}
#[tokio::test]
async fn comfy_http_submission_history_and_download_use_original_graph_and_endpoint() {
    let (_dir, p) = fixture();
    let (_key, mut submission) = queued(&p);
    let history = json!({"remote-123":{"prompt":[0,"remote-123",submission.graph,{},["9"]],"status":{"completed":true,"status_str":"success"},"outputs":{"9":{"images":[{"filename":"fictional.png","subfolder":"shots","type":"output"}]}}}});
    let (endpoint, task) = server(vec![
        (
            "POST /prompt ".into(),
            200,
            br#"{"prompt_id":"remote-123"}"#.to_vec(),
        ),
        (
            "GET /history/remote-123 ".into(),
            200,
            serde_json::to_vec(&history).unwrap(),
        ),
        (
            "GET /view?filename=fictional.png&subfolder=shots&type=output ".into(),
            200,
            png(),
        ),
    ])
    .await;
    submission.endpoint = endpoint;
    assert!(matches!(
        submit_comfy(&submission, "client-fixture").await,
        ComfySubmission::Queued(_)
    ));
    assert!(matches!(
        check_comfy(&submission).await.unwrap(),
        ComfyResult::Image(_)
    ));
    let requests = task.await.unwrap();
    let body = requests[0].split("\r\n\r\n").nth(1).unwrap();
    let payload: Value = serde_json::from_str(body).unwrap();
    assert_eq!(payload["prompt"], submission.graph);
    assert_eq!(payload["client_id"], "client-fixture");
}
#[tokio::test]
async fn wrong_history_graph_and_unsafe_output_never_download_images() {
    let (_dir, p) = fixture();
    let (_, mut submission) = queued(&p);
    let wrong = json!({"remote-123":{"prompt":[0,"remote-123",{},{}],"status":{"completed":true},"outputs":{}}});
    let (endpoint, task) = server(vec![(
        "GET /history/remote-123 ".into(),
        200,
        serde_json::to_vec(&wrong).unwrap(),
    )])
    .await;
    submission.endpoint = endpoint;
    assert!(check_comfy(&submission)
        .await
        .unwrap_err()
        .contains("exact submitted workflow"));
    assert_eq!(task.await.unwrap().len(), 1);
    let unsafe_history = json!({"remote-123":{"prompt":[0,"remote-123",submission.graph,{}],"status":{"completed":true},"outputs":{"9":{"images":[{"filename":"../private.png"}]}}}});
    let (endpoint, task) = server(vec![(
        "GET /history/remote-123 ".into(),
        200,
        serde_json::to_vec(&unsafe_history).unwrap(),
    )])
    .await;
    submission.endpoint = endpoint;
    assert!(check_comfy(&submission)
        .await
        .unwrap_err()
        .contains("unsafe"));
    assert_eq!(task.await.unwrap().len(), 1);
}
#[tokio::test]
async fn submission_errors_are_distinguished_from_uncertainty_without_retry() {
    let (_dir, p) = fixture();
    let (_, mut submission) = queued(&p);
    for (status, bytes, rejected) in [
        (
            400,
            br#"{"error":{"message":"missing weight"}}"#.to_vec(),
            true,
        ),
        (200, b"invalid response".to_vec(), false),
        (503, b"unavailable".to_vec(), false),
        (302, vec![], false),
    ] {
        let (endpoint, task) = server(vec![("POST /prompt ".into(), status, bytes)]).await;
        submission.endpoint = endpoint;
        let result = submit_comfy(&submission, "fixture").await;
        assert_eq!(matches!(result, ComfySubmission::Rejected(_)), rejected);
        assert_eq!(task.await.unwrap().len(), 1);
    }
}
#[tokio::test]
async fn preflight_only_performs_bounded_get_requests() {
    let mut settings = ImageConnection::default();
    let mut objects = serde_json::Map::new();
    for node in settings.profile.workflow.as_object().unwrap().values() {
        objects.insert(
            node["class_type"].as_str().unwrap().into(),
            json!({"output_node":true,"input":{"required":{}}}),
        );
    }
    let (endpoint, task) = server(vec![
        ("GET /system_stats ".into(), 200, b"{}".to_vec()),
        (
            "GET /object_info ".into(),
            200,
            serde_json::to_vec(&objects).unwrap(),
        ),
    ])
    .await;
    settings.base_url = endpoint;
    assert!(preflight_image(&settings, None).await.unwrap().ready);
    assert_eq!(task.await.unwrap().len(), 2);
}
