use super::*;

fn grouped(nodes: Value) -> String {
    json!({"changes":{"putNodes":nodes,"putLinks":[],"removeLinks":[]},"notes":[],"suggestChapterEnd":false}).to_string()
}

#[test]
fn sparse_updates_preserve_captured_facts_and_retain_all_supplied_unknown_values_for_repair() {
    let (_dir, p) = fixture();
    let mut w = p.view().unwrap().world;
    let person = &mut w.nodes[1];
    person.description = "Saved supplementary facts. ".repeat(60);
    person.visual_description = "Saved appearance.".into();
    person.components.get_mut("detail.intent").unwrap().insert(
        "objective".into(),
        PropertyValue::Text("Repair the pump".into()),
    );
    let context = scene_world(&w, &edits(&p, "").plan).unwrap();
    let prose = "Glass cut Mara's hand. Her immediate aim was now to reach the clinic.";
    let raw = grouped(
        json!([{"kind":"putNode","node":{"id":w.nodes[1].id,"components":{"detail.condition":{"condition":"Cut hand"},"detail.intent":{"objective":null}}},"evidence":["Glass cut Mara's hand.","Her immediate aim was now to reach the clinic."]}]),
    );
    let text = materialize_provider_effects(&raw, &context).unwrap();
    let effects: PassageEffects = serde_json::from_str(&text).unwrap();
    let PassageEffect::PutNode { node, evidence } = &effects.changes[0] else {
        panic!()
    };
    assert_eq!(node.description, w.nodes[1].description);
    assert_eq!(node.visual_description, w.nodes[1].visual_description);
    assert_eq!(
        node.components.keys().collect::<Vec<_>>(),
        w.nodes[1].components.keys().collect::<Vec<_>>()
    );
    assert_eq!(
        node.components["detail.intent"]["objective"],
        PropertyValue::Null
    );
    assert_eq!(evidence.quotes().len(), 2);
    assert!(
        text.len() > raw.len() * 2,
        "sparse reply {} bytes, complete review {} bytes",
        raw.len(),
        text.len()
    );
    assert!(preview_passage_effects(&w, &context, prose, &effects, &mut BTreeMap::new()).is_ok());
    let bad = grouped(
        json!([{"kind":"putNode","node":{"id":w.nodes[1].id,"components":{"detail.condition":{"unregisteredFact":"Retain this for repair"}}},"evidence":["Glass cut Mara's hand."]}]),
    );
    let retained = materialize_provider_effects(&bad, &context).unwrap();
    assert!(retained.contains("Retain this for repair"));
    let bad_effects: PassageEffects = serde_json::from_str(&retained).unwrap();
    assert!(
        preview_passage_effects(&w, &context, prose, &bad_effects, &mut BTreeMap::new()).is_err()
    );
    for patch in [
        json!({"id":id(),"description":"Outside scene"}),
        json!({"id":w.nodes[1].id,"components":null}),
        json!({"id":w.nodes[1].id,"components":{"detail.intent":null}}),
    ] {
        assert!(materialize_provider_effects(
            &grouped(
                json!([{"kind":"putNode","node":patch,"evidence":["Glass cut Mara's hand."]}])
            ),
            &context
        )
        .is_err());
    }
    let unknown = grouped(
        json!([{"kind":"putNode","node":{"id":w.nodes[1].id,"unsupportedBaseFact":"Keep original output"},"evidence":["Glass cut Mara's hand."]}]),
    );
    assert!(materialize_provider_effects(&unknown, &context).is_err());
    // New records do not inherit a guessed baseline.
    assert!(materialize_provider_effects(&grouped(json!([{"kind":"putNode","node":{"id":"new:person"},"evidence":["Glass cut Mara's hand."]}])),&context).is_err());
}

#[test]
fn sparse_provider_results_use_the_saved_source_and_cannot_be_rebased_or_bypass_guards() {
    let (_dir, p) = fixture();
    let before = p.view().unwrap();
    let prose = "Glass cut Mara's hand.";
    let key = id();
    p.save_passage(&key, 0, edits(&p, prose)).unwrap();
    p.start_passage(&key, 1, before.revision, true, &settings())
        .unwrap();
    let mut changed = before.world.clone();
    changed.nodes[1].description = "A newer world edit.".into();
    p.commit(before.revision, id(), before.title, changed.clone())
        .unwrap();
    let raw = grouped(
        json!([{"kind":"putNode","node":{"id":before.world.nodes[1].id,"components":{"detail.condition":{"condition":"Cut hand"}}},"evidence":[prose]}]),
    );
    p.finish_passage(&key, true, Ok(raw.clone()), false, false, &settings())
        .unwrap();
    let view = p.passage_view(&key, &settings()).unwrap();
    assert!(view.stale);
    assert!(!view.can_accept);
    assert_eq!(view.original_effects, raw);
    assert_eq!(view.edits.prose, prose);
    let effects: PassageEffects = serde_json::from_str(&view.edits.effects_text).unwrap();
    let PassageEffect::PutNode { node, .. } = &effects.changes[0] else {
        panic!()
    };
    assert_eq!(node.description, before.world.nodes[1].description);
    assert_ne!(node.description, changed.nodes[1].description);
    assert!(p.accept_passage(&key, view.version, &settings()).is_err());
    assert_eq!(p.view().unwrap().world, changed);
    // Materialization is not permission to add/remove capabilities or change type.
    for patch in [
        json!({"id":before.world.nodes[1].id,"typeId":"core.item"}),
        json!({"id":before.world.nodes[1].id,"components":{"core.container":{}}}),
    ] {
        let text = materialize_provider_effects(
            &grouped(json!([{"kind":"putNode","node":patch,"evidence":[prose]}])),
            &before.world,
        )
        .unwrap();
        let effects: PassageEffects = serde_json::from_str(&text).unwrap();
        assert!(preview_passage_effects(
            &before.world,
            &before.world,
            prose,
            &effects,
            &mut BTreeMap::new()
        )
        .is_err());
    }
}

#[test]
fn every_excerpt_is_checked_and_any_quote_edit_invalidates_only_its_card() {
    let (_dir, p) = fixture();
    let first = "Glass cut Mara's hand.";
    let last = "Her immediate aim was now to reach the clinic.";
    let prose = format!(
        "{first} {} {last}",
        "Rain rattled against the windows. ".repeat(35)
    );
    let key = id();
    p.save_passage(&key, 0, edits(&p, &prose)).unwrap();
    p.start_passage(&key, 1, 0, true, &settings()).unwrap();
    let raw = grouped(
        json!([{"kind":"putNode","node":{"id":p.view().unwrap().world.nodes[1].id,"components":{"detail.condition":{"condition":"Cut hand"},"detail.intent":{"objective":"Reach the clinic"}}},"evidence":[first,last]}]),
    );
    p.finish_passage(&key, true, Ok(raw), false, false, &settings())
        .unwrap();
    let checked = p.passage_view(&key, &settings()).unwrap();
    assert!(checked.effects_review.checked, "{}", checked.error);
    let mut edits = checked.edits;
    let mut effects: PassageEffects = serde_json::from_str(&edits.effects_text).unwrap();
    effects.notes.push(ContinuityNote {
        text: "Rain fell.".into(),
        evidence: "Rain rattled against the windows.".into(),
        node_ids: vec![],
    });
    edits.effects_text = serde_json::to_string(&effects).unwrap();
    p.save_passage(&key, checked.version, edits).unwrap();
    p.validate_passage(&key, checked.version + 1, &settings())
        .unwrap();
    let approved = p.approve_passage_for_test(&key, &settings()).unwrap();
    assert!(approved.can_accept);
    let old_note = approved.effects_review.entries[1].key.clone();
    let mut edit = approved.edits;
    let mut effects: PassageEffects = serde_json::from_str(&edit.effects_text).unwrap();
    let PassageEffect::PutNode { evidence, .. } = &mut effects.changes[0] else {
        panic!()
    };
    *evidence = EffectEvidence::Quotes(vec![
        first.into(),
        "Mara reached the clinic.".into(),
        "short".into(),
    ]);
    edit.effects_text = serde_json::to_string(&effects).unwrap();
    p.save_passage(&key, approved.version, edit).unwrap();
    let error = p
        .validate_passage(&key, approved.version + 1, &settings())
        .unwrap_err();
    assert!(error.contains("changes[0].evidence[1]"));
    assert!(error.contains("changes[0].evidence[2]"));
    let invalid = p.passage_view(&key, &settings()).unwrap();
    assert!(!invalid.can_accept);
    assert!(!invalid.effects_review.entries[0].approved);
    assert_eq!(invalid.effects_review.entries[1].key, old_note);
    assert!(invalid.effects_review.entries[1].approved);
    for quotes in [
        vec![],
        vec![first.to_string(); 5],
        vec![first.into(), format!("  {first}  ")],
    ] {
        let mut effects = effects.clone();
        let PassageEffect::PutNode { evidence, .. } = &mut effects.changes[0] else {
            panic!()
        };
        *evidence = EffectEvidence::Quotes(quotes);
        assert!(preview_passage_effects(
            &p.view().unwrap().world,
            &p.view().unwrap().world,
            &prose,
            &effects,
            &mut BTreeMap::new()
        )
        .is_err());
    }
    assert_eq!(p.view().unwrap().revision, 0);
}

#[test]
fn legacy_scalar_evidence_serializes_without_normalizing_its_original_shape() {
    let raw = r#"{"kind":"removeLink","id":"captured-link","evidence":"Mara opened the door."}"#;
    let effect: PassageEffect = serde_json::from_str(raw).unwrap();
    assert_eq!(serde_json::to_string(&effect).unwrap(), raw);
    let evidence: EffectEvidence =
        serde_json::from_str(r#"["Mara opened the door.","She closed it again."]"#).unwrap();
    assert_eq!(
        serde_json::to_string(&evidence).unwrap(),
        r#"["Mara opened the door.","She closed it again."]"#
    );
}

#[test]
fn sparse_expansion_respects_the_complete_draft_size_limit() {
    let (_dir, p) = fixture();
    let mut w = p.view().unwrap().world;
    let mut proposals = vec![];
    for index in 0..20 {
        let mut person = node(
            &w,
            "core.character",
            &format!("Fictional participant {index}"),
        );
        person.description = "x".repeat(16000);
        person.visual_description = "y".repeat(16000);
        proposals.push(json!({"kind":"putNode","node":{"id":person.id,"components":{"detail.condition":{"condition":"Wet clothes"}}},"evidence":["Rain soaked everyone's clothes."]}));
        w.nodes.push(person);
    }
    w.validate().unwrap();
    let raw = grouped(json!(proposals));
    assert!(raw.len() < 20000);
    let error = materialize_provider_effects(&raw, &w).unwrap_err();
    assert!(error.contains("500 KB draft limit"));
    assert!(error.contains("original provider response is retained"));
}

#[test]
fn malformed_and_truncated_sparse_results_keep_full_prose_and_raw_output_after_restart() {
    let (dir, p) = fixture();
    let world = p.view().unwrap().world;
    let prose = format!(
        "Glass cut Mara's hand. {}",
        "Rain rattled against the windows. ".repeat(50)
    );
    let cases = [
        (
            grouped(
                json!([{"kind":"putNode","node":{"id":world.nodes[1].id,"unsupportedBaseFact":"Retain for repair"},"evidence":["Glass cut Mara's hand."]}]),
            ),
            false,
        ),
        (
            grouped(
                json!([{"kind":"putNode","node":{"id":world.nodes[1].id,"components":{"detail.condition":{"condition":"Cut hand"}}},"evidence":["Glass cut Mara's hand."]}]),
            ),
            true,
        ),
    ];
    let mut retained = vec![];
    for (raw, incomplete) in cases {
        let key = id();
        p.save_passage(&key, 0, edits(&p, &prose)).unwrap();
        p.start_passage(&key, 1, 0, true, &settings()).unwrap();
        p.finish_passage(&key, true, Ok(raw.clone()), false, incomplete, &settings())
            .unwrap();
        let view = p.passage_view(&key, &settings()).unwrap();
        assert!(!view.can_accept);
        assert_eq!(view.edits.prose, prose);
        assert_eq!(view.original_effects, raw);
        assert!(!view.error.is_empty());
        assert_eq!(p.view().unwrap().world, world);
        retained.push((key, raw));
    }
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    for (key, raw) in retained {
        let view = p.passage_view(&key, &settings()).unwrap();
        assert_eq!(view.original_effects, raw);
        assert_eq!(view.edits.prose, prose);
        assert!(!view.can_accept);
    }
    assert!(p.story_index().unwrap().passages.is_empty());
}
