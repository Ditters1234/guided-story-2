use super::*;

#[test]
fn stitched_quotes_report_all_fields_and_manual_repair_preserves_prose_until_acceptance() {
    let (_dir, p) = fixture();
    let before = p.view().unwrap();
    let prose = "Mara opened the door. After a long pause, she closed it. Mara handed her coat to Jon, who folded it over his arm.";
    let mut person = before.world.nodes[1].clone();
    person.description = "Mara checked the door before handing over her coat.".into();
    let worn = before
        .world
        .edges
        .iter()
        .find(|e| e.type_id == "core.worn-by")
        .unwrap();
    let effects = PassageEffects {
        changes: vec![
            PassageEffect::PutNode {
                node: person,
                evidence: "Mara opened the door... she closed it.".into(),
            },
            PassageEffect::PutLink {
                link: WorldEdge {
                    type_id: "core.stored-in".into(),
                    target_id: before.world.nodes[2].id.clone(),
                    ..worn.clone()
                },
                evidence: "handed".into(),
            },
        ],
        notes: vec![
            ContinuityNote {
                text: "Jon carries the coat.".into(),
                evidence: "Mara handed her coat... folded it over his arm.".into(),
                node_ids: vec![before.world.nodes[2].id.clone()],
            },
            ContinuityNote {
                text: "Mara closed the door.".into(),
                evidence: "After a long pause, she closed it.".into(),
                node_ids: vec![before.world.nodes[1].id.clone()],
            },
        ],
        ..Default::default()
    };
    let original = serde_json::to_string(&effects).unwrap();
    let candidate = id();
    p.save_passage(&candidate, 0, edits(&p, prose)).unwrap();
    let request = p
        .start_passage(&candidate, 1, before.revision, true, &settings())
        .unwrap();
    assert!(request.system.contains("Never paraphrase evidence"));
    p.finish_passage(
        &candidate,
        true,
        Ok(original.clone()),
        false,
        false,
        &settings(),
    )
    .unwrap();
    let view = p.passage_view(&candidate, &settings()).unwrap();
    for field in [
        "changes[0].evidence",
        "changes[1].evidence",
        "notes[0].evidence",
    ] {
        assert!(view.error.contains(field), "{}", view.error);
    }
    assert!(!view.error.contains("notes[1].evidence"));
    assert!(view.error.contains("continuous excerpt"));
    assert_eq!(view.edits.prose, prose);
    assert_eq!(view.original_effects, original);
    assert!(!view.can_accept);
    assert_eq!(p.view().unwrap().world_ref, before.world_ref);
    assert!(p.story_index().unwrap().passages.is_empty());

    let mut repaired = effects;
    if let PassageEffect::PutNode { evidence, .. } = &mut repaired.changes[0] {
        *evidence = "Mara opened the door.".into();
    }
    let mut edit = view.edits;
    edit.effects_text = serde_json::to_string(&repaired).unwrap();
    p.save_passage(&candidate, view.version, edit.clone())
        .unwrap();
    let error = p
        .validate_passage(&candidate, view.version + 1, &settings())
        .unwrap_err();
    assert!(!error.contains("changes[0].evidence"));
    assert!(error.contains("changes[1].evidence") && error.contains("notes[0].evidence"));

    let transfer = "Mara handed her coat to Jon, who folded it over his arm.";
    if let PassageEffect::PutLink { evidence, .. } = &mut repaired.changes[1] {
        *evidence = transfer.into();
    }
    repaired.notes[0].evidence = transfer.into();
    edit.effects_text = serde_json::to_string(&repaired).unwrap();
    let version = p.passage_view(&candidate, &settings()).unwrap().version;
    p.save_passage(&candidate, version, edit).unwrap();
    p.validate_passage(&candidate, version + 1, &settings())
        .unwrap();
    let checked = p.passage_view(&candidate, &settings()).unwrap();
    assert!(checked.can_accept);
    assert_eq!(checked.original_effects, original);
    assert_eq!(p.view().unwrap().world_ref, before.world_ref);
    p.accept_passage(&candidate, checked.version, &settings())
        .unwrap();
    let reading = p.read_passage(&candidate).unwrap();
    assert_eq!(reading.passage.prose, prose);
    assert!(reading.after.edges.iter().any(|e| e.id == worn.id
        && e.type_id == "core.stored-in"
        && e.target_id == before.world.nodes[2].id));
}

#[test]
fn literal_ellipses_unicode_and_whitespace_quotes_remain_valid_without_auto_acceptance() {
    let (_dir, p) = fixture();
    let before = p.view().unwrap();
    let prose = "Mara whispered, “Wait... stay here.”\n\nShe closed\t the door.";
    let effects = PassageEffects {
        notes: vec![
            ContinuityNote {
                text: "Mara asked someone to wait.".into(),
                evidence: "“Wait... stay here.”".into(),
                node_ids: vec![before.world.nodes[1].id.clone()],
            },
            ContinuityNote {
                text: "Mara closed the door.".into(),
                evidence: "She closed the door.".into(),
                node_ids: vec![before.world.nodes[1].id.clone()],
            },
        ],
        ..Default::default()
    };
    let candidate = id();
    p.save_passage(&candidate, 0, edits(&p, prose)).unwrap();
    p.start_passage(&candidate, 1, 0, true, &settings())
        .unwrap();
    p.finish_passage(
        &candidate,
        true,
        Ok(serde_json::to_string(&effects).unwrap()),
        false,
        false,
        &settings(),
    )
    .unwrap();
    let checked = p.passage_view(&candidate, &settings()).unwrap();
    assert!(checked.can_accept, "{}", checked.error);
    assert_eq!(p.view().unwrap().world_ref, before.world_ref);
    assert!(p.story_index().unwrap().passages.is_empty());
    p.accept_passage(&candidate, checked.version, &settings())
        .unwrap();
    let reading = p.read_passage(&candidate).unwrap();
    assert_eq!(reading.passage.prose, prose);
    assert_eq!(
        reading.passage.effects.notes[0].evidence,
        "“Wait... stay here.”"
    );
}

#[test]
fn empty_short_long_and_paraphrased_evidence_have_distinct_indexed_errors() {
    let (_dir, p) = fixture();
    let mut edit = edits(&p, "Mara waited beside the window.");
    edit.effects_text = serde_json::to_string(&PassageEffects {
        notes: [
            "".into(),
            "rain".into(),
            "x".repeat(601),
            "Mara stood beside the window.".into(),
        ]
        .into_iter()
        .map(|evidence| ContinuityNote {
            text: "A proposed observation.".into(),
            evidence,
            node_ids: vec![],
        })
        .collect(),
        ..Default::default()
    })
    .unwrap();
    let candidate = id();
    p.save_passage(&candidate, 0, edit).unwrap();
    let error = p.validate_passage(&candidate, 1, &settings()).unwrap_err();
    for index in 0..4 {
        assert!(
            error.contains(&format!("notes[{index}].evidence")),
            "{error}"
        );
    }
    for length in [0, 4, 601] {
        assert!(error.contains(&format!("{length} characters")), "{error}");
    }
    assert!(error.contains("does not appear"));
}
