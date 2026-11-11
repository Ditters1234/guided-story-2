use super::*;
use crate::Project;
use std::collections::BTreeSet;

#[test]
fn template_connections_resolve_keys_validate_fields_and_report_the_authored_path() {
    let raw: serde_json::Value =
        serde_json::from_str(include_str!("../../../../schema/location-templates.json")).unwrap();
    for (case, expected) in [
        ("type", "/ typeId: unavailable relationship"),
        ("source", "/ sourceKey[missing]: unknown node key"),
        ("target", "/ targetKey[missing]: unknown node key"),
        (
            "capability",
            "/ sourceKey[environment]: requires component core.place",
        ),
        ("field", "/ properties.unknown: undeclared field"),
        ("kind", "/ properties.access: expected LongText"),
        ("shape", "/ properties.access:"),
        ("missing-direction", "/ properties.direction:"),
        ("null-direction", "/ properties.direction:"),
        ("invalid-direction", "/ properties.direction:"),
        ("self", "/ targetKey[parking]: a node cannot link to itself"),
        ("duplicate", "duplicate connection"),
        ("duplicate-key", "keys must be nonempty and unique"),
        ("long-description", "/ description: exceeds"),
        ("unknown", "unknown field"),
        ("physical-conflict", "one target per source"),
    ] {
        let mut value = raw[0].clone();
        let edge = &mut value["edges"][0];
        let mut key = edge["key"].as_str().unwrap().to_string();
        match case {
            "type" => edge["typeId"] = serde_json::json!("missing.relationship"),
            "source" => edge["sourceKey"] = serde_json::json!("missing"),
            "target" => edge["targetKey"] = serde_json::json!("missing"),
            "capability" => edge["sourceKey"] = serde_json::json!("environment"),
            "field" => edge["properties"]["unknown"] = serde_json::json!(true),
            "kind" => edge["properties"]["access"] = serde_json::json!(42),
            "shape" => edge["properties"]["access"] = serde_json::json!({"nested":"value"}),
            "missing-direction" => {
                edge["properties"]
                    .as_object_mut()
                    .unwrap()
                    .remove("direction");
            }
            "null-direction" => edge["properties"]["direction"] = serde_json::Value::Null,
            "invalid-direction" => edge["properties"]["direction"] = serde_json::json!("both"),
            "self" => edge["targetKey"] = edge["sourceKey"].clone(),
            "physical-conflict" => {
                edge["typeId"] = serde_json::json!("core.inside");
                edge["sourceKey"] = serde_json::json!("plaza");
                edge["targetKey"] = serde_json::json!("parking");
                edge["properties"] = serde_json::json!({});
            }
            "long-description" => edge["description"] = serde_json::json!("x".repeat(16001)),
            "duplicate" | "duplicate-key" => {
                let mut duplicate = edge.clone();
                if case == "duplicate" {
                    key = "duplicate_route".into();
                    duplicate["key"] = serde_json::json!(key);
                }
                value["edges"].as_array_mut().unwrap().push(duplicate);
            }
            _ => edge["unknown"] = serde_json::json!(true),
        }
        let error = parse_catalogue(&serde_json::json!([value]).to_string())
            .and_then(|templates| assemble_template(&templates[0]))
            .unwrap_err();
        assert!(
            error.contains("template school")
                && error.contains(&format!("edges[{key}]"))
                && error.contains(expected),
            "{case}: {error}"
        );
    }
    // Container satisfies the native holder contract without an extra holder
    // component. Explicit edges use that same capability interpretation.
    let mut value = raw[1].clone();
    value["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|n| n["key"] == "trail_map")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("parent");
    value["edges"].as_array_mut().unwrap().push(serde_json::json!({
        "key":"map_in_tool_chest", "typeId":"core.stored-in", "sourceKey":"trail_map", "targetKey":"tool_chest"
    }));
    let templates = parse_catalogue(&serde_json::json!([value]).to_string()).unwrap();
    assemble_template(&templates[0])
        .unwrap()
        .validate()
        .unwrap();
}

#[test]
fn starters_author_routes_and_local_conditions_without_implicit_context_or_inheritance() {
    for template in catalogue().unwrap() {
        let world = assemble_template(&template).unwrap();
        let key_for =
            |id: &str| &template.nodes[world.nodes.iter().position(|n| n.id == id).unwrap()].key;
        let routes: Vec<_> = world
            .edges
            .iter()
            .filter(|e| e.type_id == "story.route-to")
            .collect();
        assert!(!routes.is_empty());
        assert!(
            routes
                .iter()
                .all(|e| e.properties.get("direction")
                    == Some(&PropertyValue::Text("two-way".into())))
        );
        assert!(routes.iter().all(
            |e| matches!(e.properties.get("access"),Some(PropertyValue::Text(v)) if !v.is_empty())
        ));
        let conditions: Vec<_> = world
            .edges
            .iter()
            .filter(|e| e.type_id == "story.conditions-at")
            .collect();
        assert!(!conditions.is_empty());
        assert!(conditions
            .iter()
            .all(|e| key_for(&e.source_id) == "environment" && e.properties.contains_key("scope")));
        let covered: BTreeSet<_> = conditions
            .iter()
            .map(|e| key_for(&e.target_id).as_str())
            .collect();
        if template.id == "homestead" {
            assert!(covered.contains("yard"));
            for indoor in ["house", "kitchen", "guest_cabin", "high_tunnel"] {
                assert!(!covered.contains(indoor));
            }
        }
        if template.id == "spaceship" {
            assert!(!covered.contains("shuttle"));
            let berth = routes
                .iter()
                .find(|e| key_for(&e.target_id) == "shuttle")
                .unwrap();
            assert!(
                matches!(berth.properties.get("access"),Some(PropertyValue::Text(v)) if v.contains("No shuttle is present"))
            );
        }
        assert!(world.ai_context.is_empty());
        // No inferred reverse edges, travel, possession or character knowledge.
        assert_eq!(
            world.edges.len(),
            template.nodes.iter().filter(|n| n.parent.is_some()).count() + template.edges.len()
        );
        let mut without_direction = world.clone();
        without_direction
            .edges
            .iter_mut()
            .filter(|e| e.type_id == "story.route-to")
            .for_each(|e| {
                e.properties.remove("direction");
            });
        without_direction.validate().unwrap(); // Existing saved schema remains valid.
    }
    // The original node/parent-only template format is still accepted.
    let raw: serde_json::Value =
        serde_json::from_str(include_str!("../../../../schema/location-templates.json")).unwrap();
    let mut old = raw[0].clone();
    old.as_object_mut().unwrap().remove("edges");
    let templates = parse_catalogue(&serde_json::json!([old]).to_string()).unwrap();
    let world = assemble_template(&templates[0]).unwrap();
    assert!(world
        .edges
        .iter()
        .all(|e| e.type_id == "core.inside" || e.type_id == "core.stored-in"));
}

#[test]
fn import_failures_identify_template_node_component_field_and_parent() {
    let raw: serde_json::Value =
        serde_json::from_str(include_str!("../../../../schema/location-templates.json")).unwrap();
    for (case, expected) in [
        ("preset", "unavailable preset missing.preset"),
        ("extension", "components[missing.extension]"),
        ("field", "components[detail.place].unknown"),
        (
            "kind",
            "components[detail.place].purpose: expected LongText",
        ),
        ("parent", "parent[missing-parent]"),
        ("duplicate", "keys must be nonempty and unique"),
        ("nonphysical-parent", "cannot have a physical parent"),
        ("parse", "unknown field"),
        ("parse-kind", "/ visualDescription: expected string"),
    ] {
        let mut value = raw[0].clone();
        let nodes = value["nodes"].as_array_mut().unwrap();
        let mut key = nodes[0]["key"].as_str().unwrap().to_string();
        match case {
            "preset" => nodes[0]["typeId"] = serde_json::json!("missing.preset"),
            "extension" => nodes[0]["components"]["missing.extension"] = serde_json::json!({}),
            "field" => {
                nodes[0]["components"]["detail.place"]["unknown"] = serde_json::json!("extra")
            }
            "kind" => nodes[0]["components"]["detail.place"]["purpose"] = serde_json::json!(42),
            "parent" => nodes[0]["parent"] = serde_json::json!("missing-parent"),
            "duplicate" => nodes.push(nodes[0].clone()),
            "nonphysical-parent" => {
                // Leave the catalogue's place and its connections intact.
                // Changing its preset alone would retain explicit place capabilities.
                let parent = key;
                key = "nonphysical-child".into();
                nodes.push(serde_json::json!({
                    "key":key, "typeId":"core.time", "name":"Invalid physical child",
                    "description":"A fictional time marker without placement capabilities.",
                    "visualDescription":"", "parent":parent, "components":{}
                }));
            }
            "parse-kind" => nodes[0]["visualDescription"] = serde_json::json!(42),
            _ => nodes[0]["unexpected"] = serde_json::json!(true),
        }
        let text = serde_json::json!([value]).to_string();
        let result = parse_catalogue(&text).and_then(|templates| assemble_template(&templates[0]));
        let error = result.unwrap_err();
        assert!(
            error.contains("template school")
                && error.contains(&format!("nodes[{key}]"))
                && error.contains(expected),
            "{case}: {error}"
        );
    }
    assert!(parse_catalogue("[")
        .unwrap_err()
        .contains("location-templates.json"));
}

#[test]
fn template_parent_eligibility_follows_capabilities_instead_of_the_preset() {
    for has_place in [false, true] {
        let text = serde_json::json!([{
            "id":"capability-fixture", "name":"Capability fixture",
            "description":"An isolated fictional placement fixture.",
            "nodes":[
                {"key":"root", "typeId":"core.place", "name":"Workshop", "description":"A fictional workshop.", "visualDescription":""},
                {"key":"child", "typeId":"core.time", "name":"Time marker", "description":"A fictional time marker.", "visualDescription":"", "parent":"root",
                    "components":if has_place { serde_json::json!({"core.place":{}}) } else { serde_json::json!({}) }}
            ]
        }]).to_string();
        let templates = parse_catalogue(&text).unwrap();
        let result = assemble_template(&templates[0]);
        if has_place {
            let world = result.unwrap();
            world.validate().unwrap();
            let child = world
                .nodes
                .iter()
                .find(|n| n.type_id == "core.time")
                .unwrap();
            assert!(world
                .edges
                .iter()
                .any(|e| e.source_id == child.id && e.type_id == "core.inside"));
        } else {
            let error = result.unwrap_err();
            assert!(
                error.contains("nodes[child] / parent[root]")
                    && error.contains("cannot have a physical parent"),
                "{error}"
            );
        }
    }
}

#[test]
fn location_starters_validate_with_nested_places_and_single_inventory_placements() {
    let templates = catalogue().unwrap();
    let mut template_ids = BTreeSet::new();
    for template in &templates {
        assert!(template_ids.insert(&template.id));
        let world = location_template_world(&template.id).unwrap();
        world.validate().unwrap();
        assert!(world
            .nodes
            .iter()
            .all(|n| !n.description.is_empty() && !n.components.contains_key("core.character")));
        assert!(world
            .nodes
            .iter()
            .filter(|n| n.components.contains_key("core.place")
                || n.components.contains_key("core.item"))
            .all(|n| !n.visual_description.is_empty()));
        assert!(world.ai_context.is_empty() && world.source_annotations.is_empty());
        let roots: Vec<_> = world
            .nodes
            .iter()
            .filter(|n| {
                !world.edges.iter().any(|e| {
                    e.source_id == n.id
                        && world
                            .relation_types
                            .iter()
                            .any(|r| r.id == e.type_id && r.physical_placement)
                })
            })
            .collect();
        let physical_roots: Vec<_> = roots
            .iter()
            .filter(|n| n.components.contains_key("core.place"))
            .collect();
        assert_eq!(physical_roots.len(), 1);
        for preset in [
            "core.time",
            "core.environment",
            "core.narration-style",
            "core.visual-style",
            "core.brief",
            "core.organization",
            "core.lore",
            "core.rule",
        ] {
            assert_eq!(roots.iter().filter(|n| n.type_id == preset).count(), 1);
        }
        for node in world
            .nodes
            .iter()
            .filter(|n| !roots.iter().any(|root| root.id == n.id))
        {
            let links: Vec<_> = world
                .edges
                .iter()
                .filter(|e| {
                    e.source_id == node.id
                        && world
                            .relation_types
                            .iter()
                            .any(|r| r.id == e.type_id && r.physical_placement)
                })
                .collect();
            assert_eq!(links.len(), 1);
            assert_eq!(
                links[0].type_id,
                if node.components.contains_key("core.place") {
                    "core.inside"
                } else {
                    "core.stored-in"
                }
            );
        }
        // Rooms are actual places. Deeper placement includes room wings,
        // decks and objects stored inside containers.
        assert!(world.edges.iter().any(|e| e.type_id == "core.inside"));
        assert!(world.edges.iter().any(|e| world
            .edges
            .iter()
            .any(|parent| parent.source_id == e.target_id)));
        assert!(world
            .nodes
            .iter()
            .any(|n| n.components.contains_key("core.item")));
    }
    assert_eq!(template_ids.len(), 3);
    assert!(location_template_world("unknown").is_err());
    assert!(location_template_world("").is_err());
}

#[test]
fn starter_copies_have_independent_ids_and_editing_one_does_not_change_the_catalogue() {
    for template in catalogue().unwrap() {
        let mut first = location_template_world(&template.id).unwrap();
        let second = location_template_world(&template.id).unwrap();
        let first_ids: BTreeSet<_> = first
            .nodes
            .iter()
            .map(|n| &n.id)
            .chain(first.edges.iter().map(|e| &e.id))
            .collect();
        assert!(second
            .nodes
            .iter()
            .map(|n| &n.id)
            .chain(second.edges.iter().map(|e| &e.id))
            .all(|id| !first_ids.contains(id)));
        first.nodes[0].name = "An author's renamed location".into();
        first.nodes[0]
            .components
            .get_mut("detail.place")
            .unwrap()
            .insert("purpose".into(), PropertyValue::Text("New purpose".into()));
        let fresh = location_template_world(&template.id).unwrap();
        assert_eq!(fresh.nodes[0].name, second.nodes[0].name);
        assert_eq!(fresh.nodes[0].components, second.nodes[0].components);
    }
}

#[test]
fn location_starters_create_reopen_and_extend_through_normal_project_commits() {
    let dir = tempfile::tempdir().unwrap();
    for template in catalogue().unwrap() {
        let path = dir.path().join(&template.id);
        let world = location_template_world(&template.id).unwrap();
        let project = Project::create(
            &path,
            "Fictional starter test".into(),
            world.clone(),
            Uuid::new_v4().to_string(),
        )
        .unwrap();
        let before = project.view().unwrap();
        assert_eq!(before.world, world);
        assert!(project.story_index().unwrap().passages.is_empty());
        drop(project);
        let project = Project::open(&path).unwrap();
        let mut extended = project.view().unwrap().world;
        extended.nodes[0].name = "My location".into();
        let mut room = extended
            .nodes
            .iter()
            .find(|n| n.components.contains_key("core.place"))
            .unwrap()
            .clone();
        room.id = Uuid::new_v4().to_string();
        room.name = "Author's additional room".into();
        extended.edges.push(WorldEdge {
            id: Uuid::new_v4().to_string(),
            type_id: "core.inside".into(),
            source_id: room.id.clone(),
            target_id: extended.nodes[0].id.clone(),
            description: String::new(),
            properties: BTreeMap::new(),
        });
        extended.nodes.push(room);
        let result = project
            .commit(
                before.revision,
                Uuid::new_v4().to_string(),
                before.title,
                extended.clone(),
            )
            .unwrap();
        assert_eq!(result.project.world, extended);
        drop(project);
        assert_eq!(
            Project::open(&path).unwrap().view().unwrap().world,
            extended
        );
        // A new-project action cannot replace an existing story folder.
        assert!(Project::create(
            &path,
            "Replacement".into(),
            world,
            Uuid::new_v4().to_string()
        )
        .is_err());
    }
}
