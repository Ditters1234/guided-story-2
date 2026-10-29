use super::*;
use crate::*;
use serde_json::{json, Value};

fn world() -> World {
    let mut w = World::default();
    let person = node(&w, "core.character", "Nora Hale");
    let mut coat = node(&w, "core.item", "Work coat");
    coat.components.insert(
        "core.wearable".into(),
        BTreeMap::from([
            ("regions".into(), PropertyValue::Text("torso, arms".into())),
            ("layer".into(), PropertyValue::Number(2.0)),
        ]),
    );
    w.edges.push(edge("core.worn-by", &coat.id, &person.id));
    w.nodes = vec![person, coat];
    w.validate().unwrap();
    w
}

fn legacy(w: &World) -> Value {
    let mut value = serde_json::to_value(w).unwrap();
    value["relationTypes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["id"] == "core.worn-by")
        .unwrap()["physicalPlacement"] = json!(false);
    let mut held = value["edges"][0].clone();
    held["id"] = json!(request());
    held["typeId"] = json!("core.stored-in");
    held["description"] = json!("Issued by the homestead.");
    value["edges"][0]["description"] = json!("Buttoned against the rain.");
    value["edges"].as_array_mut().unwrap().push(held);
    value
}

#[test]
fn legacy_pairs_collapse_idempotently_with_notes_and_invalid_property_drafts_preserved() {
    let w = world();
    let old = legacy(&w);
    let upgraded: World = serde_json::from_value(old.clone()).unwrap();
    upgraded.validate().unwrap();
    assert_eq!(upgraded.edges.len(), 1);
    assert_eq!(upgraded.edges[0].id, w.edges[0].id);
    assert!(upgraded.edges[0]
        .description
        .contains("Buttoned against the rain."));
    assert!(upgraded.edges[0]
        .description
        .contains("Issued by the homestead."));
    assert_eq!(
        upgraded,
        serde_json::from_slice::<World>(&encode(&upgraded).unwrap()).unwrap()
    );
    let mut draft = old;
    draft["nodes"][1]["name"] = json!("");
    draft["nodes"][1]["components"]["core.item"]["quantity"] = json!(2);
    let draft: World = serde_json::from_value(draft).unwrap();
    assert_eq!(draft.nodes[1].name, "");
    assert_eq!(
        draft.nodes[1].components["core.item"]["quantity"],
        PropertyValue::Number(2.0)
    );
    assert!(draft.validate().is_err());
}

#[test]
fn damaged_legacy_links_are_rejected_instead_of_hidden_by_conversion() {
    let w = world();
    let original = legacy(&w);
    for case in 0..5 {
        let mut bad = original.clone();
        match case {
            0 => {
                bad["edges"].as_array_mut().unwrap().pop();
            }
            1 => bad["edges"][1]["id"] = bad["edges"][0]["id"].clone(),
            2 => bad["edges"][1]["id"] = json!("bad-id"),
            3 => bad["edges"][1]["targetId"] = json!(w.nodes[1].id),
            _ => {
                let duplicate = bad["edges"][1].clone();
                bad["edges"].as_array_mut().unwrap().push(duplicate);
            }
        }
        assert!(serde_json::from_value::<World>(bad).is_err(), "case {case}");
    }
    let mut too_long = original;
    too_long["edges"][0]["description"] = json!("a".repeat(16000));
    too_long["edges"][1]["description"] = json!("b".repeat(16000));
    assert!(serde_json::from_value::<World>(too_long)
        .unwrap_err()
        .to_string()
        .contains("notes exceed"));
    let mut modern = w.clone();
    modern
        .edges
        .push(edge("core.stored-in", &w.nodes[1].id, &w.nodes[0].id));
    assert!(
        serde_json::from_value::<World>(serde_json::to_value(&modern).unwrap())
            .unwrap()
            .validate()
            .is_err()
    );
}

#[test]
fn legacy_custom_placement_keeps_its_meaning_in_the_surviving_wear_link() {
    let w = world();
    let mut old = legacy(&w);
    let mut custom = old["relationTypes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "core.stored-in")
        .unwrap()
        .clone();
    custom["id"] = json!("custom.entrusted");
    old["relationTypes"].as_array_mut().unwrap().push(custom);
    old["edges"][1]["typeId"] = json!("custom.entrusted");
    let upgraded: World = serde_json::from_value(old).unwrap();
    upgraded.validate().unwrap();
    assert_eq!(upgraded.edges.len(), 1);
    assert!(upgraded.edges[0].description.contains("custom.entrusted"));
    assert!(upgraded.edges[0]
        .description
        .contains("Issued by the homestead."));
}

#[test]
fn legacy_project_history_forks_backups_and_media_keep_original_records() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("legacy");
    let w = world();
    let creation = request();
    let title = "Fictional legacy world";
    let p = Project::create(&root, title.into(), w.clone(), creation.clone()).unwrap();
    let mut head = p.read_head().unwrap();
    let mut manifest: Manifest = p.read_record("revisions", &head.revision).unwrap();
    let world_path = p.record_path("worlds", &manifest.world_ref).unwrap();
    let old_bytes = encode(&legacy(&w)).unwrap();
    atomic_write(&world_path, &old_bytes).unwrap();
    manifest.world_ref.sha256 = digest(&old_bytes);
    let old_reference = manifest.world_ref.clone();
    let stored: StoredWorld = serde_json::from_slice(&old_bytes).unwrap();
    manifest.receipts.get_mut(&creation).unwrap().fingerprint =
        Project::fingerprint(title, &stored).unwrap();
    let story_request = request();
    let index = StoryIndex::default();
    let index_ref = p.write_record("stories", &index).unwrap();
    manifest.story_ref = Some(index_ref.clone());
    manifest.receipts.insert(
        story_request.clone(),
        Receipt {
            fingerprint: digest(&encode(&("story-commit-v1", title, &stored, &index)).unwrap()),
            revision: 0,
        },
    );
    let revision_path = p.record_path("revisions", &head.revision).unwrap();
    let revision_bytes = encode(&manifest).unwrap();
    atomic_write(&revision_path, &revision_bytes).unwrap();
    head.revision.sha256 = digest(&revision_bytes);
    atomic_write(&root.join("head.json"), &encode(&head).unwrap()).unwrap();
    let original_head = fs::read(root.join("head.json")).unwrap();
    drop(p);
    let p = Project::open(&root).unwrap();
    assert_eq!(p.view().unwrap().world.edges.len(), 1);
    assert_eq!(p.view().unwrap().world_ref, old_reference);
    assert_eq!(fs::read(root.join("head.json")).unwrap(), original_head);
    assert_eq!(
        p.committed_revision_for(&creation, title, &old_reference)
            .unwrap(),
        Some(0)
    );
    assert_eq!(
        p.committed_story_request(&story_request, title, &old_reference, &index)
            .unwrap(),
        Some(0)
    );
    assert!(p
        .committed_revision_for(&creation, "Different title", &old_reference)
        .is_err());
    assert!(p
        .committed_story_request(&story_request, "Different title", &old_reference, &index)
        .is_err());
    let settings = LlmConnection {
        provider: TextProvider::Demo,
        ..Default::default()
    };
    let passage = request();
    let plan = PassagePlan {
        viewpoint_id: Some(w.nodes[0].id.clone()),
        ..p.next_passage_plan().unwrap()
    };
    p.save_passage(
        &passage,
        0,
        PassageEdits {
            plan,
            prose: "Nora waited in her work coat.".into(),
            effects_text: empty_effects(),
            close_chapter: false,
            approved_effects: vec![],
        },
    )
    .unwrap();
    p.validate_passage(&passage, 1, &settings).unwrap();
    let view = p.passage_view(&passage, &settings).unwrap();
    p.accept_passage(&passage, view.version, &settings).unwrap();
    let reading = p.read_passage(&passage).unwrap();
    assert_eq!(reading.passage.before_world, old_reference);
    assert_eq!(reading.before.edges.len(), 1);
    assert_eq!(reading.after.edges.len(), 1);
    assert_eq!(fs::read(&world_path).unwrap(), old_bytes);
    assert_eq!(
        p.committed_revision_for(&creation, title, &old_reference)
            .unwrap(),
        Some(0)
    );
    assert_eq!(
        p.committed_story_request(&story_request, title, &old_reference, &index)
            .unwrap(),
        Some(0)
    );
    let image = p
        .create_media(
            &request(),
            MediaTarget {
                guidance_source: None,
                node_id: Some(w.nodes[0].id.clone()),
                passage_id: None,
                moment: ImageMoment::After,
                subject_ids: vec![],
            },
        )
        .unwrap();
    assert_eq!(image.draft.source.facts[0].outfit.len(), 1);
    let fork = p
        .fork_after_passage(&passage, dir.path(), "Fork".into())
        .unwrap();
    assert_eq!(fork.read_passage(&passage).unwrap().before.edges.len(), 1);
    assert_eq!(fork.view().unwrap().world.edges.len(), 1);
    let backup = p
        .export_backup(dir.path(), &request(), p.view().unwrap().revision)
        .unwrap();
    let reviewed = inspect_backup(Path::new(&backup.folder_path)).unwrap();
    let restored = restore_backup(
        Path::new(&backup.folder_path),
        dir.path(),
        &request(),
        &reviewed.fingerprint,
    )
    .unwrap();
    let restored = Project::open(Path::new(&restored.folder_path)).unwrap();
    assert_eq!(
        restored.read_passage(&passage).unwrap().before.edges.len(),
        1
    );
    assert_eq!(
        fs::read(restored.record_path("worlds", &old_reference).unwrap()).unwrap(),
        old_bytes
    );
}
