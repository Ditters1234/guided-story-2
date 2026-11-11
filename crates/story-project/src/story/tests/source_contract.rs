use super::*;

#[test]
fn pending_checks_from_the_prior_prompt_schema_contract_stale_without_losing_prose() {
    let (_dir, p) = fixture();
    let s = settings();
    let key = id();
    let e = edits(
        &p,
        "Mara waited beside the window while Jon checked the rain.",
    );
    p.save_passage(&key, 0, e.clone()).unwrap();
    p.validate_passage(&key, 1, &s).unwrap();
    let ready = p.passage_view(&key, &s).unwrap();
    assert!(ready.error.is_empty());
    let w = p.view().unwrap().world;
    let (context, input, _, _) = p
        .story_input(&w, &e.plan, &s, default_memory_budget())
        .unwrap();
    let mut old = story_hash(&(
        "passage-source-v9-situated-claims",
        &input,
        &context.node_types,
        &context.component_types,
        &context.relation_types,
        &context.source_annotations,
        &s,
    ))
    .unwrap();
    if let Some(included) = story_ai_context_stamp(&w).unwrap() {
        old = story_hash(&(&old, included)).unwrap();
    }
    let mut draft = p.read_passage_draft(&key).unwrap();
    assert_ne!(draft.source.as_ref().unwrap().fingerprint, old);
    draft.source.as_mut().unwrap().fingerprint = old;
    p.write_passage_draft(&draft).unwrap();
    let view = p.passage_view(&key, &s).unwrap();
    assert!(view.stale);
    assert!(!view.can_accept);
    assert_eq!(view.edits.prose, e.prose);
    assert!(p.accept_passage(&key, view.version, &s).is_err());
    assert_eq!(p.view().unwrap().revision, 0);
    assert!(p.story_index().unwrap().passages.is_empty());
}

#[test]
fn grouped_provider_effects_become_review_cards_without_publishing_and_retain_the_original() {
    let (_dir, p) = fixture();
    let before = p.view().unwrap().world;
    let mut link = before
        .edges
        .iter()
        .find(|e| e.type_id == "core.worn-by")
        .unwrap()
        .clone();
    link.type_id = "core.stored-in".into();
    link.target_id = before.nodes[2].id.clone();
    let prose = "Mara handed her field coat to Jon.";
    let raw=serde_json::json!({"changes":{"putNodes":[],"putLinks":[{"kind":"putLink","link":link,"evidence":prose}],"removeLinks":[]},"notes":[],"suggestChapterEnd":false}).to_string();
    let key = id();
    p.save_passage(&key, 0, edits(&p, prose)).unwrap();
    p.start_passage(&key, 1, 0, true, &settings()).unwrap();
    p.finish_passage(&key, true, Ok(raw.clone()), false, false, &settings())
        .unwrap();
    let view = p.passage_view(&key, &settings()).unwrap();
    assert!(view.error.is_empty(), "{}", view.error);
    assert_eq!(view.original_effects, raw);
    let flat: PassageEffects = serde_json::from_str(&view.edits.effects_text).unwrap();
    assert_eq!(flat.changes.len(), 1);
    assert!(!view.can_accept);
    assert_eq!(p.view().unwrap().world, before);
    let ready = p.approve_passage_for_test(&key, &settings()).unwrap();
    p.accept_passage(&key, ready.version, &settings()).unwrap();
    assert!(p.view().unwrap().world.edges.contains(&link));
}

#[test]
fn incomplete_or_malformed_grouped_effects_preserve_full_prose_and_block_acceptance() {
    for (groups, incomplete) in [
        (
            serde_json::json!({"putNodes":[],"putLinks":[],"removeLinks":[]}),
            true,
        ),
        (
            serde_json::json!({"putNodes":[],"putLinks":[],"removeLinks":[],"foreign":[]}),
            false,
        ),
    ] {
        let (_dir, p) = fixture();
        let prose = "Mara waited beside the window while Jon checked the rain.";
        let key = id();
        p.save_passage(&key, 0, edits(&p, prose)).unwrap();
        p.start_passage(&key, 1, 0, true, &settings()).unwrap();
        let raw =
            serde_json::json!({"changes":groups,"notes":[],"suggestChapterEnd":false}).to_string();
        p.finish_passage(&key, true, Ok(raw.clone()), false, incomplete, &settings())
            .unwrap();
        let view = p.passage_view(&key, &settings()).unwrap();
        assert_eq!(view.edits.prose, prose);
        assert_eq!(view.original_effects, raw);
        assert!(!view.can_accept);
        assert!(!view.error.is_empty());
        assert!(p.accept_passage(&key, view.version, &settings()).is_err());
        assert_eq!(p.view().unwrap().revision, 0);
    }
}
