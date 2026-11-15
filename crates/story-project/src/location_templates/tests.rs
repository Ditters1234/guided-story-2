use super::*;
use crate::{LlmConnection, PassageStage, Project, TextProvider};
use std::collections::BTreeSet;

fn raw() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../../schema/location-templates.json")).unwrap()
}

#[test]
fn only_the_small_configured_demo_is_bundled_and_retired_templates_are_unavailable() {
    let entries = catalogue().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].id, "lantern-demo");
    for old in ["school", "homestead", "spaceship", "brasswater"] {
        assert!(story_template(old).is_err());
    }
    let seed = story_template("lantern-demo").unwrap();
    let world = &seed.world;
    world.validate().unwrap();
    assert_eq!(world.nodes.len(), 10);
    assert!(world.nodes.iter().all(|node| ["detail.image", "detail.anatomy", "detail.visual-style"]
        .iter()
        .all(|id| !node.components.contains_key(*id))));
    assert_eq!(
        world
            .nodes
            .iter()
            .filter(|n| n.components.contains_key("core.character"))
            .count(),
        1
    );
    let plan = seed.initial_plan.unwrap();
    let narration = plan.narration_node_id.unwrap();
    let brief = plan.brief_node_id.unwrap();
    for (kind, target) in [
        ("core.narration-pov", plan.viewpoint_id.unwrap()),
        ("core.narration-focus", plan.location_id.unwrap()),
    ] {
        assert!(world
            .edges
            .iter()
            .any(|e| e.type_id == kind && e.source_id == narration && e.target_id == target));
    }
    assert_eq!(world.ai_context.len(), 1);
    assert_eq!(world.ai_context[0].node_id, narration);
    assert!(!world.ai_context.iter().any(|b| b.node_id == brief));
    assert_eq!(plan.reference_ids.len(), 2);
    assert!(plan.direction.contains("discovers a loose battery contact"));
    assert!(!plan.stop_after.is_empty());
    let garments: Vec<_> = world
        .nodes
        .iter()
        .filter(|n| n.components.contains_key("core.wearable"))
        .collect();
    assert_eq!(garments.len(), 3);
    for item in garments {
        assert!(world
            .edges
            .iter()
            .any(|e| e.source_id == item.id && e.type_id == "core.worn-by"));
    }
    assert!(!world
        .edges
        .iter()
        .any(|e| e.type_id == "core.holds-claim" || e.type_id == "story.owned-by"));
}

#[test]
fn demo_instances_have_independent_ids_and_explicit_two_way_access() {
    let a = story_template("lantern-demo").unwrap();
    let b = story_template("lantern-demo").unwrap();
    let ids: BTreeSet<_> = a
        .world
        .nodes
        .iter()
        .map(|n| &n.id)
        .chain(a.world.edges.iter().map(|e| &e.id))
        .collect();
    assert!(!b.world.nodes.iter().any(|n| ids.contains(&n.id)));
    assert!(!b.world.edges.iter().any(|e| ids.contains(&e.id)));
    let route = a
        .world
        .edges
        .iter()
        .find(|e| e.type_id == "story.route-to")
        .unwrap();
    assert_eq!(
        route.properties["direction"],
        PropertyValue::Text("two-way".into())
    );
    assert!(matches!(&route.properties["access"],PropertyValue::Text(s) if !s.is_empty()));
}

#[test]
fn malformed_template_data_reports_its_node_or_edge_path() {
    for (case, expected) in [
        ("preset", "nodes[workshop] / typeId"),
        ("field", "components[detail.place].unknown"),
        ("kind", "components[detail.place].purpose"),
        ("parent", "parent[missing]"),
        ("duplicate", "unique"),
        ("edge-target", "targetKey[missing]"),
        ("edge-type", "typeId: unavailable relationship"),
        ("edge-capability", "requires component core.character"),
        ("edge-property", "properties.unknown"),
        ("route-direction", "explicit travel direction"),
    ] {
        let mut value = raw()[0].clone();
        match case {
            "preset" => value["nodes"][0]["typeId"] = serde_json::json!("missing.preset"),
            "field" => {
                value["nodes"][0]["components"]["detail.place"]["unknown"] =
                    serde_json::json!("unsupported")
            }
            "kind" => {
                value["nodes"][0]["components"]["detail.place"]["purpose"] = serde_json::json!(42)
            }
            "parent" => value["nodes"][0]["parent"] = serde_json::json!("missing"),
            "duplicate" => {
                let duplicate = value["nodes"][0].clone();
                value["nodes"].as_array_mut().unwrap().push(duplicate);
            }
            "edge-target" => value["edges"][0]["targetKey"] = serde_json::json!("missing"),
            "edge-type" => value["edges"][0]["typeId"] = serde_json::json!("missing.relation"),
            "edge-capability" => {
                value["edges"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|e| e["key"] == "pov")
                    .unwrap()["targetKey"] = serde_json::json!("workshop")
            }
            "edge-property" => {
                value["edges"][0]["properties"]["unknown"] = serde_json::json!("value")
            }
            "route-direction" => {
                value["edges"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|e| e["key"] == "doorway")
                    .unwrap()["properties"]
                    .as_object_mut()
                    .unwrap()
                    .remove("direction");
            }
            _ => unreachable!(),
        }
        let result = parse_catalogue(&serde_json::json!([value]).to_string())
            .and_then(|entries| assemble_template(&entries[0]));
        let error = result.unwrap_err();
        assert!(
            error.contains("lantern-demo") && error.contains(expected),
            "{case}: {error}"
        );
    }
}

#[test]
fn demo_creation_reopens_with_ready_sources_but_no_generated_or_accepted_prose() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("demo");
    let seed = story_template("lantern-demo").unwrap();
    let original = seed.world.clone();
    let p = Project::create_with_initial_passage(
        &path,
        "Rowan’s Lantern".into(),
        seed.world,
        Uuid::new_v4().to_string(),
        seed.initial_plan,
    )
    .unwrap();
    assert_eq!(p.view().unwrap().revision, 0);
    assert_eq!(p.view().unwrap().world, original);
    assert!(p.story_index().unwrap().passages.is_empty());
    let drafts = p.list_passage_drafts().unwrap();
    assert_eq!(drafts.len(), 1);
    let key = drafts[0].id.clone();
    let config = LlmConnection {
        provider: TextProvider::Demo,
        ..Default::default()
    };
    let view = p.passage_view(&key, &config).unwrap();
    assert_eq!(view.stage, PassageStage::Draft);
    assert!(view.edits.prose.is_empty() && view.original_prose.is_empty());
    assert!(
        view.scene_context.error.is_empty(),
        "{}",
        view.scene_context.error
    );
    assert!(!view.can_accept);
    let step = original
        .nodes
        .iter()
        .find(|n| n.name == "Workshop doorstep")
        .unwrap();
    assert!(!view
        .scene_context
        .resources
        .iter()
        .any(|r| r.node_id == step.id));
    for item in original
        .nodes
        .iter()
        .filter(|n| n.components.contains_key("core.wearable"))
    {
        assert!(view
            .scene_context
            .resources
            .iter()
            .any(|r| r.node_id == item.id));
    }
    drop(p);
    let p = Project::open(&path).unwrap();
    let reopened = p.passage_view(&key, &config).unwrap();
    assert_eq!(reopened.edits, view.edits);
    assert_eq!(p.view().unwrap().world, original);
    let preview = p
        .preview_passage_prompt(
            &key,
            reopened.version,
            0,
            crate::PassagePromptAction::Generate,
            &config,
        )
        .unwrap();
    let input: serde_json::Value = serde_json::from_str(&preview.user).unwrap();
    let records = input["records"].as_array().unwrap();
    for (name, capability, field, label, value) in [
        (
            "Rowan",
            "detail.personality",
            "temperament",
            "Behavior patterns",
            "Checks their own work twice before asking for help.",
        ),
        (
            "Rowan",
            "detail.voice",
            "style",
            "Speech pattern",
            "Short sentences and practical vocabulary.",
        ),
        (
            "Repair workshop",
            "detail.place",
            "resources",
            "Usable features",
            "Bench lamp.",
        ),
    ] {
        let record = records.iter().find(|record| record["name"] == name).unwrap();
        let section = record["details"]
            .as_array()
            .unwrap()
            .iter()
            .find(|section| section["capability"] == capability)
            .unwrap();
        let projected = section["fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|projected| projected["key"] == field)
            .unwrap();
        let definition = original
            .component_types
            .iter()
            .find(|definition| definition.id == capability)
            .unwrap();
        let saved = definition
            .fields
            .iter()
            .find(|saved| saved.key == field)
            .unwrap();
        assert_eq!(projected["label"], label);
        assert_eq!(projected["value"], value);
        assert_eq!(
            projected["help"].as_str(),
            saved.metadata.as_ref().unwrap().help.as_deref()
        );
    }
    let prepared = p
        .start_passage(&key, reopened.version, 0, false, &config)
        .unwrap();
    assert!(prepared.user.contains("loose battery contact"));
    assert_eq!(prepared.system.matches("Stay close to Rowan").count(), 1);
    assert!(p.story_index().unwrap().passages.is_empty());
    assert!(Project::create(
        &path,
        "Overwrite".into(),
        original,
        Uuid::new_v4().to_string()
    )
    .is_err());
}

#[test]
fn template_setup_rejects_forged_or_missing_sources_before_creating_a_folder() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("invalid");
    let mut seed = story_template("lantern-demo").unwrap();
    seed.initial_plan.as_mut().unwrap().narration_node_id = Some(Uuid::new_v4().to_string());
    assert!(Project::create_with_initial_passage(
        &path,
        "Invalid".into(),
        seed.world,
        Uuid::new_v4().to_string(),
        seed.initial_plan
    )
    .is_err());
    assert!(!path.exists());
    let mut value = raw()[0].clone();
    value["writingSetup"]["briefKey"] = serde_json::json!("missing");
    let templates = parse_catalogue(&serde_json::json!([value]).to_string()).unwrap();
    let world = assemble_template(&templates[0]).unwrap();
    assert!(template_plan(&templates[0], &world).is_err());
}

#[test]
#[ignore = "explicit isolated configured demo desktop fixture"]
fn demo_desktop_fixture() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../local")
        .join(format!("lantern-demo-ui-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let seed = story_template("lantern-demo").unwrap();
    Project::create_with_initial_passage(
        &root.join("story"),
        "Rowan’s Lantern".into(),
        seed.world,
        Uuid::new_v4().to_string(),
        seed.initial_plan,
    )
    .unwrap();
    crate::save_connection(
        &root.join("config"),
        0,
        LlmConnection {
            provider: TextProvider::Demo,
            ..Default::default()
        },
        crate::ApiKeyUpdate::Keep,
    )
    .unwrap();
    println!("Demo UI fixture: {}", root.display());
}
