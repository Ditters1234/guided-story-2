use super::*;
use crate::ai_context::tests::note;
#[test]
fn included_nodes_are_task_scoped_without_granting_effect_scope_and_history_forks_keep_their_versions(
) {
    let (dir, p) = fixture();
    let mut current = p.view().unwrap();
    let style = note(
        &mut current.world,
        "Narration",
        "Use past tense and third person.",
        AiContextRole::Instructions,
        vec![TextPromptTask::Passage, TextPromptTask::Wording],
    );
    let rule = note(
        &mut current.world,
        "Magic rules",
        "Magic requires physical contact.",
        AiContextRole::Reference,
        vec![TextPromptTask::Effects],
    );
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let before = p.view().unwrap();
    let draft = id();
    p.save_passage(&draft, 0, edits(&p, "")).unwrap();
    let request = p
        .start_passage(&draft, 1, before.revision, false, &settings())
        .unwrap();
    assert!(request.system.contains("Use past tense and third person."));
    assert!(!request.user.contains("Magic requires physical contact"));
    let records: Value = serde_json::from_str(&request.user).unwrap();
    assert!(!records["records"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["id"] == style));
    let prose = "Mara waited beside the window.";
    p.finish_passage(&draft, false, Ok(prose.into()), false, false, &settings())
        .unwrap();
    let view = p.passage_view(&draft, &settings()).unwrap();
    let check = p
        .start_passage(&draft, view.version, before.revision, true, &settings())
        .unwrap();
    assert!(!check.system.contains("Use past tense and third person."));
    assert!(check.user.contains("Magic requires physical contact"));
    let context = scene_world(&before.world, &view.edits.plan).unwrap();
    let mut forbidden = before
        .world
        .nodes
        .iter()
        .find(|n| n.id == rule)
        .unwrap()
        .clone();
    forbidden.description = "Changed rules.".into();
    assert!(preview_passage_effects(
        &before.world,
        &context,
        prose,
        &PassageEffects {
            changes: vec![PassageEffect::PutNode {
                node: forbidden,
                evidence: prose.into()
            }],
            ..Default::default()
        },
        &mut BTreeMap::new()
    )
    .is_err());
    p.finish_passage(&draft, true, Ok(empty_effects()), false, false, &settings())
        .unwrap();
    let ready = p.passage_view(&draft, &settings()).unwrap();
    p.accept_passage(&draft, ready.version, &settings())
        .unwrap();
    let original = p.read_passage(&draft).unwrap();
    assert_eq!(original.before.ai_context, before.world.ai_context);
    let mut later = p.view().unwrap();
    later
        .world
        .nodes
        .iter_mut()
        .find(|n| n.id == style)
        .unwrap()
        .description = "Use present tense.".into();
    note(
        &mut later.world,
        "Future rules",
        "Future-only background.",
        AiContextRole::Reference,
        vec![TextPromptTask::Passage],
    );
    p.commit(later.revision, id(), later.title, later.world)
        .unwrap();
    let parent = dir.path().join("forks");
    std::fs::create_dir(&parent).unwrap();
    let fork = p
        .fork_after_passage(&draft, &parent, "Independent".into())
        .unwrap();
    let fork_world = fork.view().unwrap().world;
    assert!(fork_world
        .nodes
        .iter()
        .any(|n| n.id == style && n.description.contains("past tense")));
    assert!(!fork_world.nodes.iter().any(|n| n.name == "Future rules"));
    assert_eq!(fork_world.ai_context, before.world.ai_context);
    let wording = id();
    p.begin_passage_revision(
        &wording,
        &draft,
        original.record,
        PassageRevisionKind::Wording,
    )
    .unwrap();
    let mut edit = p.passage_view(&wording, &settings()).unwrap();
    edit.edits.plan.direction = "Smooth the phrasing.".into();
    p.save_passage(&wording, edit.version, edit.edits).unwrap();
    let view = p.passage_view(&wording, &settings()).unwrap();
    let polish = p
        .start_passage(
            &wording,
            view.version,
            p.view().unwrap().revision,
            false,
            &settings(),
        )
        .unwrap();
    assert!(polish.system.contains("past tense") && !polish.system.contains("Use present tense"));
}
#[test]
fn changing_an_included_source_during_narration_retains_output_and_stales_it() {
    let (_dir, p) = fixture();
    let mut current = p.view().unwrap();
    let style = note(
        &mut current.world,
        "Style",
        "Use past tense.",
        AiContextRole::Instructions,
        vec![TextPromptTask::Passage],
    );
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let draft = id();
    p.save_passage(&draft, 0, edits(&p, "")).unwrap();
    p.start_passage(&draft, 1, p.view().unwrap().revision, false, &settings())
        .unwrap();
    let mut current = p.view().unwrap();
    current
        .world
        .nodes
        .iter_mut()
        .find(|n| n.id == style)
        .unwrap()
        .description = "Use present tense.".into();
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    p.finish_passage(
        &draft,
        false,
        Ok("Mara waited beside the window.".into()),
        false,
        false,
        &settings(),
    )
    .unwrap();
    let view = p.passage_view(&draft, &settings()).unwrap();
    assert!(view.stale && !view.can_accept);
    assert_eq!(view.edits.prose, "Mara waited beside the window.");
    assert!(p.story_index().unwrap().passages.is_empty());
}
#[test]
fn context_for_unrelated_tools_does_not_stale_a_checked_passage() {
    let (_dir, p) = fixture();
    let draft = ready(&p, edits(&p, "Mara waited beside the window."));
    let mut current = p.view().unwrap();
    note(
        &mut current.world,
        "Summary rules",
        "Prefer concise historical notes.",
        AiContextRole::Instructions,
        vec![TextPromptTask::Memory],
    );
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let view = p.passage_view(&draft, &settings()).unwrap();
    assert!(!view.stale && view.can_accept);
    p.accept_passage(&draft, view.version, &settings()).unwrap();
    assert_eq!(p.read_passage(&draft).unwrap().after.ai_context.len(), 1);
}
