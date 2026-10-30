use super::*;
use crate::media::tests::{accept_passage, fixture, png};
#[test]
fn ai_context_is_captured_for_image_assistance_and_only_selected_content_stales_it() {
    let (_dir, p) = fixture();
    let mut current = p.view().unwrap();
    let rule = crate::ai_context::tests::note(
        &mut current.world,
        "Image guidance",
        "Use soft window light.",
        AiContextRole::Instructions,
        vec![TextPromptTask::ImagePrompt],
    );
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let media = media(&p);
    let image = ImageConnection::default();
    let draft = proposal(&p, &media, &text(), &image);
    let request = p.start_image_prompt(&draft, 1, &text(), &image).unwrap();
    assert!(request.system.contains("Use soft window light."));
    p.finish_image_prompt(&draft, Ok(request.demo), false, false)
        .unwrap();
    let mut current = p.view().unwrap();
    current
        .world
        .nodes
        .iter_mut()
        .find(|n| n.id == rule)
        .unwrap()
        .visual_description = "Unselected visual text.".into();
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    assert!(!p.image_prompt_view(&draft, &text(), &image).unwrap().stale);
    let mut current = p.view().unwrap();
    current
        .world
        .nodes
        .iter_mut()
        .find(|n| n.id == rule)
        .unwrap()
        .description = "Use side lighting.".into();
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let view = p.image_prompt_view(&draft, &text(), &image).unwrap();
    assert!(view.stale && !view.can_apply);
}
#[test]
fn historical_image_assistance_uses_context_from_its_captured_passage_boundary() {
    let (_dir, p) = fixture();
    let passage = accept_passage(&p, vec![]);
    let subject = p.view().unwrap().world.nodes[1].id.clone();
    let media_id = id();
    p.create_media(
        &media_id,
        MediaTarget {
            guidance_source: None,
            node_id: None,
            passage_id: Some(passage),
            moment: ImageMoment::Before,
            subject_ids: vec![subject],
        },
    )
    .unwrap();
    let mut current = p.view().unwrap();
    crate::ai_context::tests::note(
        &mut current.world,
        "Future instructions",
        "Future-only instruction text.",
        AiContextRole::Instructions,
        vec![TextPromptTask::ImagePrompt],
    );
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let image = ImageConnection::default();
    let draft = proposal(&p, &media_id, &text(), &image);
    let request = p.start_image_prompt(&draft, 1, &text(), &image).unwrap();
    assert!(
        !request.system.contains("Future-only instruction")
            && !request.user.contains("Future-only instruction")
    );
}
#[test]
fn image_prompt_assistance_uses_the_exact_override_and_preserves_captured_facts() {
    let (_dir, p) = fixture();
    let media = media(&p);
    let mut connection = text();
    connection.prompts.overrides.insert(
        TextPromptTask::ImagePrompt,
        "Describe a clear composition using the captured outfit.".into(),
    );
    let image = ImageConnection::default();
    let key = proposal(&p, &media, &connection, &image);
    let request = p.start_image_prompt(&key, 1, &connection, &image).unwrap();
    assert_eq!(
        request.system,
        "Describe a clear composition using the captured outfit."
    );
    assert!(request.user.contains("Blue coat") && request.user.contains("responseSchema"));
    assert_eq!(p.view().unwrap().revision, 0);
}
fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn text() -> LlmConnection {
    LlmConnection {
        provider: TextProvider::Demo,
        ..Default::default()
    }
}
fn media(p: &Project) -> String {
    let key = id();
    let target = MediaTarget {
        guidance_source: None,
        node_id: Some(p.view().unwrap().world.nodes[1].id.clone()),
        passage_id: None,
        moment: ImageMoment::After,
        subject_ids: vec![],
    };
    p.create_media(&key, target).unwrap();
    key
}
fn proposal(p: &Project, media: &str, text: &LlmConnection, image: &ImageConnection) -> String {
    let key = id();
    p.create_image_prompt(
        &key,
        media,
        p.media_draft(media).unwrap().version,
        text,
        image,
    )
    .unwrap();
    key
}
fn output() -> ImagePromptOutput {
    ImagePromptOutput {
        shot: "A close portrait with soft window light.".into(),
        prompt: "Mara wears her blue coat, framed beside the Glasshouse window in soft daylight."
            .into(),
        negative: None,
    }
}
fn ready(p: &Project, key: &str) {
    let mut edits = p.read_prompt(key).unwrap().edits;
    edits.output = output();
    p.save_image_prompt(key, 1, edits).unwrap();
}

#[test]
fn request_is_scoped_to_captured_visible_facts_and_schema_transport() {
    let (_dir, p) = fixture();
    let media = media(&p);
    for mode in [
        StructuredOutput::Schema,
        StructuredOutput::Json,
        StructuredOutput::Prompt,
    ] {
        let text = LlmConnection {
            structured_output: mode,
            ..text()
        };
        let key = proposal(&p, &media, &text, &ImageConnection::default());
        let prepared = p
            .start_image_prompt(&key, 1, &text, &ImageConnection::default())
            .unwrap();
        let input: Value = serde_json::from_str(&prepared.user).unwrap();
        assert_eq!(input["subjects"].as_array().unwrap().len(), 1);
        assert!(prepared.user.contains("Blue coat"));
        assert!(prepared.user.contains("coverage"));
        assert!(!prepared.user.contains("Private planning"));
        assert!(!prepared.user.contains("class_type"));
        assert!(!prepared.user.contains(&p.view().unwrap().world.nodes[1].id));
        assert_eq!(input["frame"], json!({"width":896,"height":1152}));
        assert_eq!(
            input.get("responseSchema").is_some(),
            mode != StructuredOutput::Schema
        );
        assert_eq!(prepared.output_tokens, 1536);
        assert!(prepared.system.contains("covered anatomy"));
        assert_eq!(p.view().unwrap().revision, 0);
        assert!(p
            .start_image_prompt(&key, 2, &text, &ImageConnection::default())
            .is_err());
        p.finish_image_prompt(&key, Ok(prepared.demo), false, false)
            .unwrap();
    }
}

#[test]
fn historical_requests_exclude_future_profiles_and_full_passage_prose() {
    let (_dir, p) = fixture();
    let passage = accept_passage(&p, vec![]);
    let subject = p.view().unwrap().world.nodes[1].id.clone();
    let media_id = id();
    p.create_media(
        &media_id,
        MediaTarget {
            guidance_source: None,
            node_id: None,
            passage_id: Some(passage),
            moment: ImageMoment::Before,
            subject_ids: vec![subject],
        },
    )
    .unwrap();
    let mut current = p.view().unwrap();
    current.world.nodes[1].visual_description = "Future silver mask".into();
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let key = proposal(&p, &media_id, &text(), &ImageConnection::default());
    let prepared = p
        .start_image_prompt(&key, 1, &text(), &ImageConnection::default())
        .unwrap();
    assert!(prepared.user.contains("Visible Mara"));
    assert!(!prepared.user.contains("Future silver mask"));
    assert!(!prepared.user.contains("removed the blue coat"));
    assert!(prepared.user.contains("\"boundary\":\"before\""));
}

#[test]
fn apply_changes_only_prompt_text_and_keeps_current_controls_and_story() {
    let (_dir, p) = fixture();
    let media = media(&p);
    let before = p.view().unwrap();
    let key = proposal(&p, &media, &text(), &ImageConnection::default());
    ready(&p, &key);
    let mut request = p.media_draft(&media).unwrap().request;
    request.seed = Some(123);
    request.steps = Some(12);
    request.cfg = Some(1.5);
    p.save_media_request(&media, 1, request.clone()).unwrap();
    let result = p
        .apply_image_prompt(&key, 2, &text(), &ImageConnection::default())
        .unwrap();
    assert!(!result.replayed);
    assert_eq!(result.media.draft.request.seed, request.seed);
    assert_eq!(result.media.draft.request.steps, request.steps);
    assert_eq!(result.media.draft.request.cfg, request.cfg);
    assert_eq!(result.media.draft.request.prompt, output().prompt);
    assert_eq!(result.media.draft.stage, MediaStage::Draft);
    assert!(result.media.draft.submission.is_none());
    assert_eq!(p.view().unwrap().revision, before.revision);
    assert_eq!(p.view().unwrap().world, before.world);
    assert!(p.story_index().unwrap().media.is_empty());
    assert_eq!(
        p.image_prompt_view(&key, &text(), &ImageConnection::default())
            .unwrap()
            .stage,
        ImagePromptStage::Applied
    );
}

#[test]
fn stale_inputs_block_application_and_preserve_proposals() {
    for case in [
        "prompt",
        "negative",
        "frame",
        "appearance",
        "profile",
        "llm",
    ] {
        let (_dir, p) = fixture();
        let media = media(&p);
        let mut text = text();
        let mut image = ImageConnection::default();
        let key = proposal(&p, &media, &text, &image);
        ready(&p, &key);
        match case {
            "prompt" | "negative" | "frame" => {
                let mut request = p.media_draft(&media).unwrap().request;
                match case {
                    "prompt" => request.prompt = "Newer authored prompt".into(),
                    "negative" => request.negative = Some("New manual negative".into()),
                    _ => request.width = Some(1024),
                }
                p.save_media_request(&media, 1, request).unwrap();
            }
            "appearance" => {
                let mut current = p.view().unwrap();
                current.world.nodes[2].visual_description = "A future red coat".into();
                p.commit(current.revision, id(), current.title, current.world)
                    .unwrap();
            }
            "profile" => image.profile.family = "a-different-family".into(),
            _ => text.temperature = 0.6,
        };
        let before = p.media_draft(&media).unwrap().request;
        let view = p.image_prompt_view(&key, &text, &image).unwrap();
        assert!(view.stale, "{case}");
        assert!(!view.can_apply);
        assert!(p.apply_image_prompt(&key, 2, &text, &image).is_err());
        assert_eq!(p.media_draft(&media).unwrap().request, before);
        assert_eq!(p.read_prompt(&key).unwrap().edits.output, output());
    }
}

#[test]
fn unrelated_world_changes_and_image_endpoint_do_not_stale_prompt_work() {
    let (_dir, p) = fixture();
    let media = media(&p);
    let mut image = ImageConnection::default();
    let key = proposal(&p, &media, &text(), &image);
    ready(&p, &key);
    let mut current = p.view().unwrap();
    current.world.nodes[0].description = "Unrelated private planning edit".into();
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    image.base_url = "http://127.0.0.1:1/new-endpoint".into();
    assert!(
        p.image_prompt_view(&key, &text(), &image)
            .unwrap()
            .can_apply
    );
    p.apply_image_prompt(&key, 2, &text(), &image).unwrap();
    assert_eq!(
        p.view().unwrap().world.nodes[0].description,
        "Unrelated private planning edit"
    );
}

#[test]
fn negative_support_is_explicit_and_preserved_by_apply() {
    let (_dir, p) = fixture();
    let media = media(&p);
    let image = ImageConnection::default();
    let key = proposal(&p, &media, &text(), &image);
    let mut edits = ImagePromptEdits {
        output: ImagePromptOutput {
            negative: Some("blur".into()),
            ..output()
        },
        ..Default::default()
    };
    p.save_image_prompt(&key, 1, edits.clone()).unwrap();
    assert!(
        !p.image_prompt_view(&key, &text(), &image)
            .unwrap()
            .can_apply
    );
    edits.output.negative = None;
    p.save_image_prompt(&key, 2, edits).unwrap();
    assert!(p.apply_image_prompt(&key, 3, &text(), &image).is_ok());
    let mut image = image;
    image.profile.workflow["negative-caption"] =
        json!({"class_type":"CustomNegative","inputs":{"text":""}});
    image.profile.bindings.negative = Some(WorkflowInput {
        node_id: "negative-caption".into(),
        input: "text".into(),
    });
    let next = proposal(&p, &media, &text(), &image);
    let prepared = p.start_image_prompt(&next, 1, &text(), &image).unwrap();
    assert_eq!(
        prepared.schema["properties"]["negative"]["type"],
        json!(["string", "null"])
    );
    p.finish_image_prompt(
        &next,
        Ok(serde_json::to_string(&ImagePromptOutput {
            negative: Some("blur".into()),
            ..output()
        })
        .unwrap()),
        false,
        false,
    )
    .unwrap();
    let version = p.read_prompt(&next).unwrap().version;
    p.apply_image_prompt(&next, version, &text(), &image)
        .unwrap();
    assert_eq!(
        p.media_draft(&media).unwrap().request.negative.as_deref(),
        Some("blur")
    );
}

#[test]
fn interruption_cancel_truncation_and_json_repair_never_resend() {
    let (dir, p) = fixture();
    let media = media(&p);
    let key = proposal(&p, &media, &text(), &ImageConnection::default());
    p.start_image_prompt(&key, 1, &text(), &ImageConnection::default())
        .unwrap();
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    p.interrupt_image_prompts().unwrap();
    assert_eq!(
        p.read_prompt(&key).unwrap().stage,
        ImagePromptStage::Interrupted
    );
    assert!(p
        .start_image_prompt(&key, 3, &text(), &ImageConnection::default())
        .is_err());
    let cancelled = proposal(&p, &media, &text(), &ImageConnection::default());
    p.start_image_prompt(&cancelled, 1, &text(), &ImageConnection::default())
        .unwrap();
    p.finish_image_prompt(&cancelled, Err("Cancelled".into()), true, false)
        .unwrap();
    assert_eq!(
        p.read_prompt(&cancelled).unwrap().stage,
        ImagePromptStage::Cancelled
    );
    let truncated = proposal(&p, &media, &text(), &ImageConnection::default());
    p.start_image_prompt(&truncated, 1, &text(), &ImageConnection::default())
        .unwrap();
    let raw = serde_json::to_string(&output()).unwrap();
    p.finish_image_prompt(&truncated, Ok(raw.clone()), false, true)
        .unwrap();
    let view = p
        .image_prompt_view(&truncated, &text(), &ImageConnection::default())
        .unwrap();
    assert!(!view.can_apply);
    let mut edits = view.edits;
    edits.direction = "Changing direction does not repair truncation".into();
    p.save_image_prompt(&truncated, view.version, edits)
        .unwrap();
    assert!(
        !p.image_prompt_view(&truncated, &text(), &ImageConnection::default())
            .unwrap()
            .can_apply
    );
    let version = p.read_prompt(&truncated).unwrap().version;
    p.parse_image_prompt(&truncated, version).unwrap();
    assert!(
        p.image_prompt_view(&truncated, &text(), &ImageConnection::default())
            .unwrap()
            .can_apply
    );
    assert_eq!(p.read_prompt(&truncated).unwrap().original_output, raw);
}

#[test]
fn malformed_and_oversized_typed_outputs_are_retained_without_losing_repair_work() {
    let (_dir, p) = fixture();
    let media = media(&p);
    for raw in [
        "not JSON".into(),
        serde_json::to_string(&ImagePromptOutput {
            prompt: "x".repeat(65000),
            ..output()
        })
        .unwrap(),
    ] {
        let key = proposal(&p, &media, &text(), &ImageConnection::default());
        p.start_image_prompt(&key, 1, &text(), &ImageConnection::default())
            .unwrap();
        p.finish_image_prompt(&key, Ok(raw.clone()), false, false)
            .unwrap();
        let retained_version = p.read_prompt(&key).unwrap().version;
        p.finish_image_prompt(&key, Ok(raw.clone()), false, false)
            .unwrap();
        assert_eq!(p.read_prompt(&key).unwrap().version, retained_version);
        let view = p
            .image_prompt_view(&key, &text(), &ImageConnection::default())
            .unwrap();
        assert_eq!(view.stage, ImagePromptStage::Failed);
        assert_eq!(view.original_output, raw);
        assert_eq!(view.edits.raw_json, raw);
        assert!(view.edits.output.prompt.is_empty());
        let mut edits = view.edits;
        edits.raw_json = serde_json::to_string(&output()).unwrap();
        p.save_image_prompt(&key, view.version, edits).unwrap();
        let version = p.read_prompt(&key).unwrap().version;
        p.parse_image_prompt(&key, version).unwrap();
        assert!(
            p.image_prompt_view(&key, &text(), &ImageConnection::default())
                .unwrap()
                .can_apply
        );
        assert_eq!(p.read_prompt(&key).unwrap().original_output, raw);
    }
}

#[test]
fn budget_failure_does_not_mark_a_request_running() {
    let (_dir, p) = fixture();
    let mut current = p.view().unwrap();
    current.world.nodes[1].visual_description = "A".repeat(16000);
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let media = media(&p);
    let text = LlmConnection {
        context_tokens: 2048,
        max_tokens: 64,
        ..text()
    };
    let key = proposal(&p, &media, &text, &ImageConnection::default());
    assert!(p
        .start_image_prompt(&key, 1, &text, &ImageConnection::default())
        .unwrap_err()
        .contains("context budget"));
    let draft = p.read_prompt(&key).unwrap();
    assert_eq!(draft.stage, ImagePromptStage::Draft);
    assert_eq!(draft.version, 1);
    assert!(p.media_draft(&media).unwrap().submission.is_none());
}

#[test]
fn apply_receipts_reconcile_lost_status_and_preserve_later_prompt_work() {
    let (_dir, p) = fixture();
    let media = media(&p);
    let first = proposal(&p, &media, &text(), &ImageConnection::default());
    ready(&p, &first);
    assert!(p
        .apply_image_prompt_with_hook(&first, 2, &text(), &ImageConnection::default(), |stage| {
            if stage == "after-media" {
                Err("Lost status write".into())
            } else {
                Ok(())
            }
        })
        .is_err());
    assert_eq!(
        p.read_prompt(&first).unwrap().stage,
        ImagePromptStage::Applied
    );
    let second = proposal(&p, &media, &text(), &ImageConnection::default());
    let mut edits = ImagePromptEdits {
        output: output(),
        ..Default::default()
    };
    edits.output.prompt = "A later reviewed image prompt.".into();
    p.save_image_prompt(&second, 1, edits).unwrap();
    p.apply_image_prompt(&second, 2, &text(), &ImageConnection::default())
        .unwrap();
    let replay = p
        .apply_image_prompt(&first, 2, &text(), &ImageConnection::default())
        .unwrap();
    assert!(replay.replayed);
    assert_eq!(
        replay.media.draft.request.prompt,
        "A later reviewed image prompt."
    );
    assert_eq!(
        replay.media.draft.prompt_origin.unwrap().proposal_id,
        second
    );
}

#[test]
fn accepted_origin_survives_fork_without_pending_suggestion_dependencies() {
    let (dir, p) = fixture();
    let media = media(&p);
    let key = proposal(&p, &media, &text(), &ImageConnection::default());
    ready(&p, &key);
    p.apply_image_prompt(&key, 2, &text(), &ImageConnection::default())
        .unwrap();
    let mut d = p.media_draft(&media).unwrap();
    d.image = Some(p.save_image_bytes(&png()).unwrap());
    d.stage = MediaStage::Ready;
    p.write_media(&d).unwrap();
    p.accept_media(&media, d.version).unwrap();
    let passage = accept_passage(&p, vec![]);
    let fork = p
        .fork_after_passage(&passage, dir.path(), "Prompt provenance fork".into())
        .unwrap();
    let origin = fork.media_asset(&media).unwrap().prompt_origin.unwrap();
    assert_eq!(origin.output, output());
    assert!(!fork.root_path().join("image-prompt-drafts").exists());
    assert!(!fork.root_path().join("image-prompt-versions").exists());
    assert!(fork.media_preview(&media, true).is_ok());
}

#[test]
fn legacy_media_defaults_and_corrupt_sibling_recovery_preserve_data() {
    let (_dir, p) = fixture();
    let media = media(&p);
    let mut value = serde_json::to_value(p.media_draft(&media).unwrap()).unwrap();
    value.as_object_mut().unwrap().remove("promptOrigin");
    value.as_object_mut().unwrap().remove("promptReceipts");
    let legacy: MediaDraft = serde_json::from_value(value).unwrap();
    assert!(legacy.prompt_origin.is_none());
    assert!(legacy.prompt_receipts.is_empty());
    let key = proposal(&p, &media, &text(), &ImageConnection::default());
    p.start_image_prompt(&key, 1, &text(), &ImageConnection::default())
        .unwrap();
    fs::write(p.prompt_path(&id()).unwrap(), "damaged").unwrap();
    assert!(p.interrupt_image_prompts().is_err());
    assert_eq!(
        p.read_prompt(&key).unwrap().stage,
        ImagePromptStage::Interrupted
    );
    assert_eq!(p.view().unwrap().revision, 0);
}

#[test]
fn process_exit_preserves_prompt_apply_exactly_once() {
    use std::process::{Command, Stdio};
    for stage in ["before-media", "after-media", "after-status"] {
        let (_dir, p) = fixture();
        let mut current = p.view().unwrap();
        crate::ai_context::tests::note(
            &mut current.world,
            "Captured image style",
            "Use visible watercolour washes.",
            AiContextRole::Instructions,
            vec![TextPromptTask::ImagePrompt],
        );
        current
            .world
            .ai_context
            .last_mut()
            .unwrap()
            .image_preparation = true;
        p.commit(current.revision, id(), current.title, current.world)
            .unwrap();
        let root = p.root_path().to_path_buf();
        let media = media(&p);
        let capture = p.media_draft(&media).unwrap().source.image_context.unwrap();
        let key = proposal(&p, &media, &text(), &ImageConnection::default());
        ready(&p, &key);
        drop(p);
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "media::prompts::tests::prompt_crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("STORY_PROMPT_CRASH_ROOT", &root)
            .env("STORY_PROMPT_CRASH_ID", &key)
            .env("STORY_PROMPT_CRASH_STAGE", stage)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(77));
        let p = Project::open(&root).unwrap();
        p.interrupt_image_prompts().unwrap();
        let view = p
            .image_prompt_view(&key, &text(), &ImageConnection::default())
            .unwrap();
        let applied = stage != "before-media";
        assert_eq!(view.stage == ImagePromptStage::Applied, applied);
        let result = p
            .apply_image_prompt(&key, view.version, &text(), &ImageConnection::default())
            .unwrap();
        assert_eq!(result.replayed, applied);
        assert_eq!(result.media.draft.request.prompt, output().prompt);
        assert_eq!(result.media.draft.prompt_receipts.len(), 1);
        assert_eq!(result.media.draft.source.image_context, Some(capture));
        assert_eq!(p.view().unwrap().revision, 1);
    }
}

#[test]
#[ignore = "only invoked by the isolated image-prompt interruption test"]
fn prompt_crash_child() {
    let Ok(root) = std::env::var("STORY_PROMPT_CRASH_ROOT") else {
        return;
    };
    let key = std::env::var("STORY_PROMPT_CRASH_ID").unwrap();
    let stage = std::env::var("STORY_PROMPT_CRASH_STAGE").unwrap();
    let p = Project::open(Path::new(&root)).unwrap();
    p.apply_image_prompt_with_hook(&key, 2, &text(), &ImageConnection::default(), |point| {
        if point == stage {
            std::process::exit(77);
        }
        Ok(())
    })
    .unwrap();
    panic!("Interruption point not reached");
}
