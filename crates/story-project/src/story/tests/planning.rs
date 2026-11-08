use super::*;

#[test]
fn narrative_request_size_samples() {
    let (_dir, project) = fixture();
    for mode in [StructuredOutput::Json, StructuredOutput::Schema] {
        let mut connection = settings();
        connection.structured_output = mode;
        for checking in [false, true] {
            let draft_id = id();
            project
                .save_passage(
                    &draft_id,
                    0,
                    edits(
                        &project,
                        if checking {
                            "Mara waited by the glasshouse window."
                        } else {
                            ""
                        },
                    ),
                )
                .unwrap();
            let prepared = project
                .start_passage(&draft_id, 1, 0, checking, &connection)
                .unwrap();
            let schema_bytes = if mode == StructuredOutput::Schema {
                prepared.schema.as_ref().map_or(0, |s| s.to_string().len())
            } else {
                0
            };
            println!(
                "{}",
                json!({"task":if checking{"effects"}else{"prose"},"mode":mode,"messageBytes":prepared.system.len()+prepared.user.len(),"schemaBytes":schema_bytes,"totalBytes":prepared.system.len()+prepared.user.len()+schema_bytes})
            );
            let ceiling = if checking { 60000 } else { 10200 };
            assert!(prepared.system.len() + prepared.user.len() + schema_bytes <= ceiling);
        }
        let draft_id = id();
        project
            .save_passage(&draft_id, 0, edits(&project, ""))
            .unwrap();
        let prepared = project
            .prepare_plan_fill(&draft_id, 1, 0, PlanFillKind::Scene, &connection)
            .unwrap();
        println!(
            "{}",
            json!({"task":"planning","mode":mode,"totalBytes":prepared.system.len()+prepared.user.len()})
        );
        assert!(prepared.system.len() + prepared.user.len() < 5700);
    }
}

#[test]
fn plan_fill_is_scoped_and_leaves_the_draft_and_accepted_story_untouched() {
    let (_dir, project) = fixture();
    let before = project.view().unwrap();
    let mut world = before.world;
    let mut brief = node(&world, "core.brief", "Opening brief");
    brief.components.get_mut("detail.brief").unwrap().insert(
        "aims".into(),
        PropertyValue::Text("Keep the first decision reversible.".into()),
    );
    let brief_id = brief.id.clone();
    world.nodes.push(brief);
    project
        .commit(before.revision, id(), before.title, world)
        .unwrap();
    accept(
        &project,
        edits(&project, "Mara waited until the rain eased."),
    );

    let mut draft = edits(&project, "");
    draft.plan.participant_ids.clear();
    draft.plan.reference_ids.push(brief_id);
    let draft_id = id();
    project.save_passage(&draft_id, 0, draft.clone()).unwrap();
    let revision = project.view().unwrap().revision;
    let history = json!(project.story_index().unwrap());
    let mut connection = settings();
    connection.prompts.overrides.insert(
        TextPromptTask::Planning,
        "Plan from the selected facts.".into(),
    );
    for (kind, wire_kind) in [
        (PlanFillKind::Scene, "scene"),
        (PlanFillKind::Chapter, "chapter"),
    ] {
        let prepared = project
            .prepare_plan_fill(&draft_id, 1, revision, kind, &connection)
            .unwrap();
        assert_eq!(prepared.system, "Plan from the selected facts.");
        assert!(prepared.schema.is_none());
        assert!(prepared.output_tokens <= 768);
        let input: Value = serde_json::from_str(&prepared.user).unwrap();
        assert_eq!(input["planKind"], wire_kind);
        assert_eq!(
            input["recentPassages"][0]["endingExcerpt"],
            "Mara waited until the rain eased."
        );
        assert!(prepared
            .user
            .contains("Keep the first decision reversible."));
        assert!(!prepared.user.contains("\"Jon\""));
        assert!(kind
            .review_text(CompletionText {
                text: prepared.demo,
                incomplete: false
            })
            .is_ok());
        assert!(kind
            .review_text(CompletionText {
                text: "Cut off".into(),
                incomplete: true
            })
            .is_err());
    }
    assert_eq!(
        project.read_passage_draft(&draft_id).unwrap().edits.plan,
        draft.plan
    );
    assert_eq!(project.view().unwrap().revision, revision);
    assert_eq!(json!(project.story_index().unwrap()), history);
    project.save_passage(&draft_id, 1, draft.clone()).unwrap();
    project
        .check_plan_fill_source(&draft_id, 1, revision)
        .unwrap();
    draft.plan.direction.push_str(" End at the first decision.");
    project.save_passage(&draft_id, 1, draft).unwrap();
    assert!(project
        .check_plan_fill_source(&draft_id, 1, revision)
        .is_err());
}
