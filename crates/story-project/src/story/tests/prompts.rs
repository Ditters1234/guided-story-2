use super::*;
#[test]
fn passage_effects_and_wording_use_editable_instructions_and_prompt_edits_stale_checks() {
    let (_dir, p) = fixture();
    let before = p.view().unwrap();
    let mut connection = settings();
    for (task, text) in [
        (
            TextPromptTask::Passage,
            "Narrate with precise sensory details.",
        ),
        (TextPromptTask::Effects, "Extract source-supported changes."),
        (
            TextPromptTask::Wording,
            "Polish sentence rhythm while preserving events.",
        ),
    ] {
        connection.prompts.overrides.insert(task, text.into());
    }
    let draft = id();
    p.save_passage(&draft, 0, edits(&p, "")).unwrap();
    let prepared = p
        .start_passage(&draft, 1, before.revision, false, &connection)
        .unwrap();
    assert_eq!(prepared.system, "Narrate with precise sensory details.");
    assert!(prepared.user.contains("Mara"));
    p.finish_passage(
        &draft,
        false,
        Ok("Mara waited beside the window.".into()),
        false,
        false,
        &connection,
    )
    .unwrap();
    let view = p.passage_view(&draft, &connection).unwrap();
    let check = p
        .start_passage(&draft, view.version, before.revision, true, &connection)
        .unwrap();
    assert_eq!(check.system, "Extract source-supported changes.");
    assert!(check.user.contains("responseSchema"));
    p.finish_passage(&draft, true, Ok(empty_effects()), false, false, &connection)
        .unwrap();
    let ready = p.passage_view(&draft, &connection).unwrap();
    assert!(ready.can_accept);
    let mut changed = connection.clone();
    changed
        .prompts
        .overrides
        .insert(TextPromptTask::Passage, "Use a different style.".into());
    let stale = p.passage_view(&draft, &changed).unwrap();
    assert!(stale.stale && !stale.can_accept);
    assert_eq!(p.view().unwrap().world_ref, before.world_ref);
    p.accept_passage(&draft, ready.version, &connection)
        .unwrap();
    let revision = id();
    p.begin_passage_revision(
        &revision,
        &draft,
        p.read_passage(&draft).unwrap().record,
        PassageRevisionKind::Wording,
    )
    .unwrap();
    let mut edit = p.passage_view(&revision, &connection).unwrap();
    edit.edits.plan.direction = "Smooth the wording.".into();
    p.save_passage(&revision, edit.version, edit.edits).unwrap();
    let view = p.passage_view(&revision, &connection).unwrap();
    let polish = p
        .start_passage(
            &revision,
            view.version,
            p.view().unwrap().revision,
            false,
            &connection,
        )
        .unwrap();
    assert_eq!(
        polish.system,
        "Polish sentence rhythm while preserving events."
    );
    assert_eq!(
        p.read_passage(&draft).unwrap().passage.prose,
        "Mara waited beside the window."
    );
}
