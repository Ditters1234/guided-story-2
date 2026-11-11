use super::*;

fn draft(p: &Project) -> String {
    let key = id();
    let mut e = edits(p, "");
    e.plan.direction.clear();
    p.save_passage(&key, 0, e).unwrap();
    key
}

fn narration(p: &Project, key: &str) -> Value {
    let view = p.passage_view(key, &settings()).unwrap();
    let request = p
        .start_passage(
            key,
            view.version,
            p.view().unwrap().revision,
            false,
            &settings(),
        )
        .unwrap();
    serde_json::from_str(&request.user).unwrap()
}

fn noted(p: &Project, prose: &str, note: &str) -> String {
    let mut e = edits(p, prose);
    e.plan.chapter_plan = "Discuss the ferry timetable and settle when to leave.".into();
    e.effects_text = serde_json::to_string(&PassageEffects {
        notes: vec![ContinuityNote {
            text: note.into(),
            evidence: prose.into(),
            node_ids: vec![],
        }],
        ..Default::default()
    })
    .unwrap();
    accept(p, e)
}

#[test]
fn reopened_handoff_uses_exact_latest_version_and_full_recent_prose_without_pending_text() {
    let (dir, p) = fixture();
    accept(
        &p,
        edits(&p, "Mara answered the old question beside the window."),
    );
    let penultimate = "Jon folded the ferry timetable and put it on the workbench.";
    accept(&p, edits(&p, penultimate));
    let latest = format!(
        "{}\n\nMara asked Jon which route they should take.\n\n",
        "A quiet moment. ".repeat(130)
    );
    let latest_id = accept(&p, edits(&p, &latest));
    let source = p.read_passage(&latest_id).unwrap().record;
    let pending = id();
    p.save_passage(&pending, 0, edits(&p, "UNACCEPTED_FUTURE_CONVERSATION"))
        .unwrap();
    let key = draft(&p);
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    let input = narration(&p, &key);
    assert_eq!(input["recentPassages"].as_array().unwrap().len(), 2);
    assert_eq!(input["recentPassages"][0]["prose"], penultimate);
    assert_eq!(input["recentPassages"][1]["prose"], latest);
    assert!(!input.to_string().contains("UNACCEPTED_FUTURE_CONVERSATION"));
    let handoff = &input["passageHandoff"];
    assert_eq!(handoff["passageId"], latest_id);
    assert_eq!(handoff["source"], json!(source));
    assert_eq!(handoff["chapterRelation"], "sameChapter");
    let ending = handoff["endingExcerpt"].as_str().unwrap();
    assert!(ending.len() <= 1200 && latest.trim_end().ends_with(ending));
    assert!(ending.ends_with("Mara asked Jon which route they should take."));
    let preview = p
        .passage_view(&key, &settings())
        .unwrap()
        .scene_context
        .passage_handoff
        .unwrap();
    assert_eq!(json!(preview), *handoff);
}

#[test]
fn planning_and_writing_recall_settled_dialogue_in_an_open_chapter_without_rewriting_the_plan() {
    let (_dir, p) = fixture();
    let first = noted(
        &p,
        "Mara said, 'The ferry leaves at noon.' Jon agreed to that departure.",
        "The ferry departure discussion was settled: Mara and Jon agreed on noon.",
    );
    let original = p.read_passage(&first).unwrap();
    accept(&p, edits(&p, "Mara watched the rain run down the glass."));
    accept(&p, edits(&p, "Jon checked the clock above the workbench."));
    assert!(p.chapter_memory_status().unwrap().is_empty());
    let key = draft(&p);
    let revision = p.view().unwrap().revision;
    for kind in [PlanFillKind::Scene, PlanFillKind::Chapter] {
        let request = p
            .prepare_plan_fill(&key, 1, revision, kind, &settings())
            .unwrap();
        let input: Value = serde_json::from_str(&request.user).unwrap();
        assert_eq!(input["earlierMemory"]["points"][0]["passageId"], first);
        assert!(input["earlierMemory"]["points"][0]["text"]
            .as_str()
            .unwrap()
            .contains("discussion was settled"));
        assert_eq!(
            input["earlierMemory"]["points"][0]["source"],
            json!(original.record)
        );
        assert_eq!(
            input["writingBrief"]["chapterDirection"],
            original.passage.plan.chapter_plan
        );
        assert!(input["writingBrief"].get("chapterRemaining").is_none());
        assert!(!input["passageHandoff"]["withheld"].as_bool().unwrap());
    }
    let input = narration(&p, &key);
    assert_eq!(input["earlierMemory"]["points"][0]["passageId"], first);
    assert_eq!(p.read_passage(&first).unwrap().record, original.record);
    assert_eq!(p.view().unwrap().revision, revision);
}

#[test]
fn planning_reserves_required_handoff_and_reports_optional_history_omissions() {
    let (_dir, p) = fixture();
    noted(
        &p,
        "Mara and Jon settled the ferry question beside the window.",
        &"Their departure discussion was settled. ".repeat(35),
    );
    accept(&p, edits(&p, "Mara watched the rain run down the glass."));
    let latest = "Jon asked whether they should wait for the rain to stop.";
    accept(&p, edits(&p, latest));
    let key = draft(&p);
    let mut s = settings();
    s.max_tokens = 256;
    let mut blocked = false;
    let mut omitted = false;
    let mut recalled = false;
    for budget in (2048..=10048).step_by(250) {
        s.context_tokens = budget;
        match p.prepare_plan_fill(&key, 1, p.view().unwrap().revision, PlanFillKind::Scene, &s) {
            Err(error) => {
                assert!(error.contains("Required planning context"), "{error}");
                blocked = true;
            }
            Ok(request) => {
                assert!(
                    (request.system.len() + request.user.len()) / 3
                        + 256
                        + request.output_tokens as usize
                        <= budget as usize
                );
                let input: Value = serde_json::from_str(&request.user).unwrap();
                assert_eq!(input["passageHandoff"]["endingExcerpt"], latest);
                let count = input["earlierMemory"]["coverage"]["selectedPoints"]
                    .as_u64()
                    .unwrap();
                if count == 0 {
                    omitted = true;
                    assert_eq!(input["earlierMemory"]["coverage"]["omittedPoints"], 1);
                } else {
                    recalled = true;
                    break;
                }
            }
        }
    }
    assert!(blocked && omitted && recalled);
    assert_eq!(p.passage_view(&key, &s).unwrap().stage, PassageStage::Draft);
}

#[test]
fn planning_uses_current_reviewed_summary_but_excludes_it_after_a_wording_revision() {
    let (_dir, p) = fixture();
    let prose = "Mara answered Jon's ferry question: their departure would be at noon.";
    let mut first = edits(&p, prose);
    first.close_chapter = true;
    let first_id = accept(&p, first);
    let chapter = p.story_index().unwrap().chapters[0].id.clone();
    let summary = id();
    p.create_memory_draft(&summary, &chapter).unwrap();
    let mut view = p.memory_view(&summary).unwrap();
    view.edits.output = MemoryOutput {
        no_lasting_facts: false,
        points: vec![MemoryPoint {
            text: "REVIEWED_SETTLED_DEPARTURE: noon was agreed.".into(),
            passage_id: first_id.clone(),
            evidence: prose.into(),
            node_ids: vec![],
        }],
    };
    p.save_memory_draft(&summary, view.version, view.edits)
        .unwrap();
    let view = p.memory_view(&summary).unwrap();
    p.apply_memory(&summary, view.version).unwrap();
    accept(&p, edits(&p, "Mara watched rain run down the glass."));
    accept(
        &p,
        edits(&p, "Jon looked at the clock above the workbench."),
    );
    let key = draft(&p);
    let prepared = p
        .prepare_plan_fill(
            &key,
            1,
            p.view().unwrap().revision,
            PlanFillKind::Chapter,
            &settings(),
        )
        .unwrap();
    assert!(prepared.user.contains("REVIEWED_SETTLED_DEPARTURE"));
    let input: Value = serde_json::from_str(&prepared.user).unwrap();
    assert_eq!(input["earlierMemory"]["coverage"]["currentSummaries"], 1);
    let wording = id();
    p.begin_passage_revision(
        &wording,
        &first_id,
        p.read_passage(&first_id).unwrap().record,
        PassageRevisionKind::Wording,
    )
    .unwrap();
    let mut view = p.passage_view(&wording, &settings()).unwrap();
    view.edits.prose = "Mara answered Jon about the ferry. They would depart at noon.".into();
    p.save_passage(&wording, view.version, view.edits).unwrap();
    let view = p.passage_view(&wording, &settings()).unwrap();
    p.validate_passage(&wording, view.version, &settings())
        .unwrap();
    let view = p.passage_view(&wording, &settings()).unwrap();
    p.accept_passage(&wording, view.version, &settings())
        .unwrap();
    let prepared = p
        .prepare_plan_fill(
            &key,
            1,
            p.view().unwrap().revision,
            PlanFillKind::Chapter,
            &settings(),
        )
        .unwrap();
    assert!(!prepared.user.contains("REVIEWED_SETTLED_DEPARTURE"));
    let input: Value = serde_json::from_str(&prepared.user).unwrap();
    assert_eq!(input["earlierMemory"]["coverage"]["staleSummaries"], 1);
}

#[test]
fn withheld_latest_never_leaks_its_ending_or_becomes_an_older_handoff() {
    let (_dir, p) = fixture();
    let permitted = accept(&p, edits(&p, "Mara watched the rain run down the glass."));
    let mut current = p.view().unwrap();
    let mut claim = node(&current.world, "core.claim", "Private departure fact");
    claim
        .components
        .get_mut("core.claim")
        .unwrap()
        .insert("disclosure".into(), PropertyValue::Text("withheld".into()));
    let claim_id = claim.id.clone();
    current.world.nodes.push(claim);
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let mut e = edits(
        &p,
        "SECRET_CONVERSATION_ENDING: Mara answered Jon beside the window.",
    );
    e.plan.title = "SECRET_PASSAGE_TITLE".into();
    let latest = accept(&p, e);
    let reading = p.read_passage(&latest).unwrap();
    let current = p.view().unwrap();
    let changed = p
        .preview_source_edit(
            &current.world,
            SourceEdit::AddPassage {
                node_id: claim_id,
                passage_id: latest,
                version: reading.record,
                quote: reading.passage.prose,
                effect_index: None,
            },
        )
        .unwrap();
    p.commit(current.revision, id(), current.title, changed.world)
        .unwrap();
    let key = draft(&p);
    let request = p
        .prepare_plan_fill(
            &key,
            1,
            p.view().unwrap().revision,
            PlanFillKind::Scene,
            &settings(),
        )
        .unwrap();
    let planning: Value = serde_json::from_str(&request.user).unwrap();
    let writing = narration(&p, &key);
    for input in [planning, writing] {
        assert_eq!(input["passageHandoff"], json!({"withheld":true}));
        assert!(!input.to_string().contains("SECRET_"));
        assert_eq!(input["situatedKnowledge"]["withheldRecentPassages"], 1);
    }
    let preview = p.passage_view(&key, &settings()).unwrap();
    assert_eq!(preview.recent_passages, vec![permitted]);
    assert!(preview.scene_context.passage_handoff.unwrap().withheld);
}

#[test]
fn handoff_obeys_event_replacement_wording_and_fork_history_boundaries() {
    let (dir, p) = fixture();
    let first = accept(
        &p,
        edits(&p, "Mara settled the ferry question beside the window."),
    );
    let second = accept(&p, edits(&p, "Jon asked which route they should take."));
    let latest = accept(
        &p,
        edits(&p, "FUTURE_ENDING: Mara answered Jon's question."),
    );
    for (kind, target, expected) in [
        (PassageRevisionKind::Events, &latest, &second),
        (PassageRevisionKind::Wording, &second, &first),
    ] {
        let key = id();
        p.begin_passage_revision(&key, target, p.read_passage(target).unwrap().record, kind)
            .unwrap();
        let mut view = p.passage_view(&key, &settings()).unwrap();
        view.edits.plan.direction = "Polish this passage.".into();
        p.save_passage(&key, view.version, view.edits).unwrap();
        let input = narration(&p, &key);
        assert_eq!(input["passageHandoff"]["passageId"], *expected);
        assert!(!input.to_string().contains("FUTURE_ENDING"));
    }
    let fork = p
        .fork_after_passage(&first, dir.path(), "Continuity boundary fork".into())
        .unwrap();
    let input = narration(&fork, &draft(&fork));
    assert_eq!(input["passageHandoff"]["passageId"], first);
    assert!(!input.to_string().contains("FUTURE_ENDING"));
    assert!(!input.to_string().contains("which route"));
}

#[test]
fn copied_text_warning_is_advisory_and_excludes_short_phrases_and_rewritten_paragraphs() {
    let (_dir, p) = fixture();
    let prose = "Mara read the ferry timetable aloud and Jon agreed that they would leave at noon, after the rain stopped.";
    let previous = accept(&p, edits(&p, prose));
    let key = ready(&p, edits(&p, &prose.replace(' ', "  ")));
    let v = p.passage_view(&key, &settings()).unwrap();
    assert_eq!(v.repeated_passages, vec![previous]);
    assert!(v.can_accept);
    let normalized = json!({"recentPassages":[{"id":"previous","prose":prose}]});
    assert!(crate::story::continuity::repeated_passages("Jon agreed.", &normalized).is_empty());
    assert!(crate::story::continuity::repeated_passages("Mara checked the timetable again, then asked a different question about the route to the harbor.", &normalized).is_empty());
    let paragraph = "Mara read the ferry timetable aloud while Jon waited beside the workbench. She traced the route with her finger and asked whether the midday departure would leave them enough time to reach the far shore before evening. Jon nodded and said that noon would be best.";
    let context = json!({"recentPassages":[{"id":"previous","prose":paragraph}]});
    assert_eq!(
        crate::story::continuity::repeated_passages(
            &format!("{paragraph}\n\nA new ending followed."),
            &context
        ),
        vec!["previous"]
    );
}

#[test]
fn multibyte_handoff_is_a_bounded_exact_suffix_and_empty_story_has_none() {
    let prose = format!("{} The final answer was yes.\n\n", "風と雨。 ".repeat(500));
    let ending = crate::story::continuity::ending_excerpt(&prose);
    assert!(ending.len() <= 1200 && prose.trim_end().ends_with(ending));
    assert!(ending.ends_with("The final answer was yes."));
    let (_dir, p) = fixture();
    let key = draft(&p);
    let input = narration(&p, &key);
    assert!(input.get("passageHandoff").is_none());
}

#[test]
#[ignore = "explicit isolated continuity UI fixture"]
fn continuity_desktop_fixture() {
    let Ok(root) = std::env::var("STORY_CONTINUITY_DESKTOP_FIXTURE") else {
        return;
    };
    let (_dir, source) = fixture();
    let p = Project::create(
        Path::new(&root),
        "Continuity verification".into(),
        source.view().unwrap().world,
        id(),
    )
    .unwrap();
    noted(
        &p,
        "Mara said, 'The ferry leaves at noon.' Jon agreed to that departure.",
        "The ferry departure discussion was settled: Mara and Jon agreed on noon.",
    );
    accept(&p, edits(&p, "Mara watched the rain run down the glass."));
    let latest = "Jon put the timetable on the workbench and asked Mara whether they should take the footpath or the road to reach the ferry.";
    accept(&p, edits(&p, latest));
    let key = ready(&p, edits(&p, latest));
    assert!(!p
        .passage_view(&key, &settings())
        .unwrap()
        .repeated_passages
        .is_empty());
}
