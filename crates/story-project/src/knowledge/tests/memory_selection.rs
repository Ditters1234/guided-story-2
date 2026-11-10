use super::*;

fn accept_notes(
    p: &Project,
    plan: &PassagePlan,
    notes: Vec<ContinuityNote>,
    close: bool,
) -> String {
    let draft = save(p, plan, "Mara waited beside the workshop window.");
    let v = p.passage_view(&draft, &text()).unwrap();
    let mut edit = v.edits;
    edit.close_chapter = close;
    edit.effects_text = serde_json::to_string(&PassageEffects {
        notes,
        ..Default::default()
    })
    .unwrap();
    p.save_passage(&draft, v.version, edit).unwrap();
    let v = p.passage_view(&draft, &text()).unwrap();
    p.validate_passage(&draft, v.version, &text()).unwrap();
    let ready = p.approve_passage_for_test(&draft, &text()).unwrap();
    p.accept_passage(&draft, ready.version, &text()).unwrap();
    draft
}

fn note(label: &str, nodes: Vec<String>) -> ContinuityNote {
    ContinuityNote {
        text: label.into(),
        evidence: "Mara waited beside the workshop window.".into(),
        node_ids: nodes,
    }
}

#[test]
fn excluded_notes_do_not_crowd_out_eligible_history_by_points_or_bytes_and_saved_perspective_applies(
) {
    let (dir, p, plan) = fixture();
    let mut world = p.view().unwrap().world;
    let c = claim(
        &mut world,
        "Private narrator fact",
        "narrator",
        "unconfirmed",
    );
    commit(&p, world);
    accept_notes(&p, &plan, vec![note("SAFE_HISTORY_POINT", vec![])], false);
    let mut scoped = plan.clone();
    scoped.reference_ids.push(c.clone());
    accept_notes(
        &p,
        &scoped,
        (0..12)
            .map(|i| note(&format!("PRIVATE_POINT_{i:02}"), vec![c.clone()]))
            .collect(),
        false,
    );
    accept(&p, &plan);
    accept(&p, &plan);
    let draft = save(&p, &plan, "");
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    let view = p.passage_view(&draft, &text()).unwrap();
    assert_eq!(
        view.edits.plan.knowledge_perspective,
        KnowledgePerspective::Viewpoint
    );
    assert_eq!(view.memory.coverage.available_points, 13);
    assert_eq!(view.memory.coverage.selected_points, 1);
    assert_eq!(view.memory.coverage.omitted_points, 12);
    assert_eq!(view.memory.points[0].text, "SAFE_HISTORY_POINT");
    let world = p.view().unwrap().world;
    let scene = crate::story::scene_world(&world, &plan).unwrap();
    let minimum = serde_json::to_vec(&view.memory.points[0]).unwrap().len() as u32 + 3;
    for budget in [0, minimum - 1, minimum, 6000] {
        let context = p
            .memory_context(&p.story_index().unwrap(), &plan, &scene, &world, budget)
            .unwrap();
        assert_eq!(context.points.len(), usize::from(budget >= minimum));
        assert_eq!(
            context.coverage.omitted_points,
            13 - context.coverage.selected_points
        );
        if !context.points.is_empty() {
            assert!(serde_json::to_vec(&context.points).unwrap().len() <= budget as usize);
        }
    }
    let mut author = plan.clone();
    author.knowledge_perspective = KnowledgePerspective::Author;
    let context = p
        .memory_context(&p.story_index().unwrap(), &author, &scene, &world, 6000)
        .unwrap();
    assert_eq!(context.points.len(), 12);
    assert!(context
        .points
        .iter()
        .all(|point| point.text.starts_with("PRIVATE_POINT")));
    let req = p
        .start_passage(
            &draft,
            view.version,
            p.view().unwrap().revision,
            false,
            &text(),
        )
        .unwrap();
    assert!(req.user.contains("SAFE_HISTORY_POINT"));
    assert!(!req.user.contains("PRIVATE_POINT"));
}

#[test]
fn excluded_summary_sources_or_captured_bindings_free_the_allowance_for_safe_notes() {
    for captured_binding in [false, true] {
        let (_dir, p, plan) = fixture();
        let mut world = p.view().unwrap().world;
        let c = claim(
            &mut world,
            "Private summary source",
            "withheld",
            "unconfirmed",
        );
        if captured_binding {
            world.ai_context.push(AiContextBinding {
                node_id: c.clone(),
                enabled: true,
                role: AiContextRole::Reference,
                tasks: vec![TextPromptTask::Memory],
                fields: vec![AiContextField::Components],
                order: 0,
                image_preparation: false,
            });
        }
        commit(&p, world);
        accept_notes(&p, &plan, vec![note("SAFE_FALLBACK", vec![])], true);
        let source = accept_notes(&p, &plan, vec![], true);
        let chapter = p.story_index().unwrap().chapters.last().unwrap().id.clone();
        let summary = id();
        p.create_memory_draft(&summary, &chapter).unwrap();
        let output = MemoryOutput {
            points: (0..12)
                .map(|i| MemoryPoint {
                    text: format!("PRIVATE_SUMMARY_{i:02}"),
                    passage_id: source.clone(),
                    evidence: "Mara waited beside the workshop window.".into(),
                    node_ids: vec![],
                })
                .collect(),
            no_lasting_facts: false,
        };
        if captured_binding {
            p.start_memory(&summary, 1, p.view().unwrap().revision, &text())
                .unwrap();
            p.finish_memory(
                &summary,
                Ok(serde_json::to_string(&output).unwrap()),
                false,
                false,
            )
            .unwrap();
        } else {
            let mut v = p.memory_view(&summary).unwrap();
            v.edits.output = output;
            p.save_memory_draft(&summary, v.version, v.edits).unwrap();
        }
        let ready = p.memory_view(&summary).unwrap();
        assert!(ready.can_apply, "{}", ready.error);
        p.apply_memory(&summary, ready.version).unwrap();
        if !captured_binding {
            let reading = p.read_passage(&source).unwrap();
            let world = p.view().unwrap().world;
            let edited = p
                .preview_source_edit(
                    &world,
                    SourceEdit::AddPassage {
                        node_id: c,
                        passage_id: source,
                        version: reading.record,
                        quote: reading.passage.prose,
                        effect_index: None,
                    },
                )
                .unwrap();
            commit(&p, edited.world);
        }
        accept(&p, &plan);
        accept(&p, &plan);
        let draft = save(&p, &plan, "");
        let view = p.passage_view(&draft, &text()).unwrap();
        assert_eq!(view.memory.coverage.current_summaries, 1);
        assert_eq!(view.memory.coverage.available_points, 13);
        assert_eq!(view.memory.coverage.omitted_points, 12);
        assert_eq!(view.memory.points.len(), 1);
        assert_eq!(view.memory.points[0].text, "SAFE_FALLBACK");
        let req = p
            .start_passage(
                &draft,
                view.version,
                p.view().unwrap().revision,
                false,
                &text(),
            )
            .unwrap();
        assert!(req.user.contains("SAFE_FALLBACK"));
        assert!(!req.user.contains("PRIVATE_SUMMARY"));
    }
}
