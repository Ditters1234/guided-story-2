use super::*;

// Structural assertions inspect the resolved wire contract. Separate tests
// verify sharing is lossless and leaves per-preset capability sets concrete.
fn effects_schema(world: &World) -> Value {
    crate::prompt_context::expanded_test_schema(&super::effects_schema(world))
}

fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[test]
fn relationship_planning_fields_are_bound_to_saved_values_or_new_link_defaults() {
    let mut world = World::default();
    let a = make(&world, "core.character", &id(), "Mara");
    let b = make(&world, "core.character", &id(), "Jon");
    let mut definition = world
        .relation_types
        .iter()
        .find(|r| r.id == "core.related")
        .unwrap()
        .clone();
    definition.id = "custom.planning".into();
    definition.fields = [
        ("plan", FieldKind::Text, false, PropertyValue::Null),
        (
            "enabled",
            FieldKind::Boolean,
            false,
            PropertyValue::Boolean(false),
        ),
        (
            "count",
            FieldKind::Number,
            false,
            PropertyValue::Number(0.0),
        ),
        (
            "requiredPlan",
            FieldKind::Text,
            true,
            PropertyValue::Text("Default plan".into()),
        ),
        ("unset", FieldKind::NodeRef, false, PropertyValue::Null),
    ]
    .into_iter()
    .map(|(key, kind, required, default_value)| FieldDefinition {
        key: key.into(),
        label: key.into(),
        kind,
        required,
        default_value,
        metadata: Some(FieldMetadata {
            uses: vec![FieldUse::Planning],
            ..Default::default()
        }),
    })
    .collect();
    for (key, usage) in [("story", FieldUse::Story), ("visual", FieldUse::Visual)] {
        definition.fields.push(FieldDefinition {
            key: key.into(),
            label: key.into(),
            kind: FieldKind::Text,
            required: false,
            default_value: PropertyValue::Null,
            metadata: Some(FieldMetadata {
                uses: vec![usage],
                ..Default::default()
            }),
        });
    }
    let mut old = WorldEdge {
        id: id(),
        type_id: definition.id.clone(),
        source_id: a.id.clone(),
        target_id: b.id.clone(),
        description: String::new(),
        properties: default_field_values(&definition.fields),
    };
    old.properties
        .insert("plan".into(), PropertyValue::Text("Save the reveal".into()));
    world.nodes = vec![a, b];
    world.edges.push(old.clone());
    world.relation_types.push(definition.clone());
    world.validate().unwrap();
    let schema = effects_schema(&world);
    let choices = operation(&schema, "putLinks")["properties"]["link"]["anyOf"]
        .as_array()
        .unwrap();
    let saved = choices
        .iter()
        .find(|v| {
            v["properties"]["typeId"]["const"] == definition.id
                && v["properties"]["id"].to_string().contains(&old.id)
        })
        .unwrap();
    let protected = &saved["properties"]["properties"];
    assert!(saved["required"]
        .as_array()
        .unwrap()
        .contains(&json!("properties")));
    for key in ["plan", "enabled", "count", "requiredPlan"] {
        assert_eq!(
            protected["properties"][key]["const"],
            json!(old.properties[key])
        );
        assert!(protected["required"]
            .as_array()
            .unwrap()
            .contains(&json!(key)));
    }
    assert_eq!(protected["properties"]["unset"], json!({"const":null}));
    assert!(choices
        .iter()
        .filter(|v| v["properties"]["typeId"]["const"] != definition.id)
        .all(|v| !v["properties"]["id"].to_string().contains(&old.id)));
    let fresh = choices
        .iter()
        .find(|v| {
            v["properties"]["typeId"]["const"] == definition.id
                && v["properties"]["id"].get("pattern").is_some()
        })
        .unwrap();
    assert_eq!(
        fresh["properties"]["properties"]["properties"]["requiredPlan"]["const"],
        "Default plan"
    );
    assert_eq!(
        fresh["properties"]["properties"]["properties"]["enabled"]["anyOf"],
        json!([{"const":false},{"type":"null"}])
    );

    let prose = "Mara waited beside Jon at the window.";
    let preview = |link| {
        preview_passage_effects(
            &world,
            &world,
            prose,
            &PassageEffects {
                changes: vec![PassageEffect::PutLink {
                    link,
                    evidence: prose.into(),
                }],
                ..Default::default()
            },
            &mut BTreeMap::new(),
        )
    };
    for key in ["plan", "enabled", "count", "requiredPlan"] {
        let mut changed = old.clone();
        changed.properties.remove(key);
        assert!(preview(changed)
            .unwrap_err()
            .contains("author planning guidance"));
        let mut changed = old.clone();
        changed.properties.insert(key.into(), PropertyValue::Null);
        assert!(preview(changed)
            .unwrap_err()
            .contains("author planning guidance"));
    }
    for (key, value) in [
        ("plan", PropertyValue::Text("Changed plan".into())),
        ("enabled", PropertyValue::Boolean(true)),
        ("count", PropertyValue::Number(1.0)),
        ("unset", PropertyValue::Text(world.nodes[1].id.clone())),
    ] {
        let mut changed = old.clone();
        changed.properties.insert(key.into(), value);
        assert!(preview(changed)
            .unwrap_err()
            .contains("author planning guidance"));
    }
    let mut ordinary = old.clone();
    ordinary.properties.insert(
        "story".into(),
        PropertyValue::Text("They cooperated".into()),
    );
    ordinary.properties.insert(
        "visual".into(),
        PropertyValue::Text("Sleeves rolled".into()),
    );
    ordinary
        .properties
        .insert("unset".into(), PropertyValue::Null);
    assert!(preview(ordinary).is_ok());
    let mut changed_type = old.clone();
    changed_type.type_id = "core.related".into();
    changed_type.properties.clear();
    assert!(preview(changed_type)
        .unwrap_err()
        .contains("author planning guidance"));
    let fresh = WorldEdge {
        id: "new:defaulted".into(),
        source_id: world.nodes[1].id.clone(),
        target_id: world.nodes[0].id.clone(),
        properties: default_field_values(&definition.fields),
        ..old.clone()
    };
    assert!(preview(fresh.clone()).is_ok());
    let mut invalid = fresh.clone();
    invalid
        .properties
        .insert("plan".into(), PropertyValue::Text("Invented plan".into()));
    assert!(preview(invalid)
        .unwrap_err()
        .contains("author planning guidance"));
    let mut unset_defaults = fresh;
    unset_defaults.properties.remove("enabled");
    unset_defaults
        .properties
        .insert("count".into(), PropertyValue::Null);
    assert!(preview(unset_defaults).is_ok());
}

#[test]
fn effect_reference_fields_filter_capabilities_and_keep_exact_previous_lists() {
    let mut world = World::default();
    world.component_types.push(ComponentDefinition {
        id: "custom.refs".into(),
        name: "References".into(),
        description: String::new(),
        fields: vec![FieldDefinition {
            key: "witnesses".into(),
            label: "Witnesses".into(),
            kind: FieldKind::NodeRefs,
            required: false,
            default_value: PropertyValue::Null,
            metadata: Some(FieldMetadata {
                reference_capability: Some("core.character".into()),
                ..Default::default()
            }),
        }],
    });
    let mut actor = make(&world, "core.character", &id(), "Mara");
    let other = make(&world, "core.character", &id(), "Jon");
    let place = make(&world, "core.place", &id(), "Glasshouse");
    let unexposed = id();
    actor.components.insert(
        "custom.refs".into(),
        BTreeMap::from([(
            "witnesses".into(),
            PropertyValue::NodeRefs(vec![unexposed.clone()]),
        )]),
    );
    world.nodes = vec![actor.clone(), other.clone(), place.clone()];
    let schema = effects_schema(&world);
    let record = node_variants(&schema)
        .iter()
        .find(|v| {
            v["properties"]["id"]["enum"]
                .as_array()
                .is_some_and(|ids| ids.contains(&json!(actor.id)))
        })
        .unwrap();
    let field =
        &record["properties"]["components"]["properties"]["custom.refs"]["properties"]["witnesses"];
    let alternatives = field["anyOf"].as_array().unwrap();
    assert!(alternatives.contains(&json!({"const":[unexposed]})));
    let changed = alternatives.iter().find(|v| v["type"] == "array").unwrap();
    let targets = changed["items"]["anyOf"].as_array().unwrap();
    let known = targets.iter().find(|v| v.get("enum").is_some()).unwrap();
    let expected: BTreeSet<_> = [actor.id, other.id].into_iter().collect();
    assert_eq!(known["enum"], json!(expected));
    assert!(!known["enum"].as_array().unwrap().contains(&json!(place.id)));
    assert!(targets.iter().any(|v| v.get("pattern").is_some()));
    assert!(!schema.to_string().contains("$ref"));
    assert!(!schema.to_string().contains("allOf"));
}
fn make(world: &World, type_id: &str, id: &str, name: &str) -> WorldNode {
    let preset = world.node_types.iter().find(|p| p.id == type_id).unwrap();
    WorldNode {
        id: id.into(),
        type_id: type_id.into(),
        name: name.into(),
        description: String::new(),
        visual_description: String::new(),
        components: preset
            .components
            .iter()
            .map(|id| {
                (
                    id.clone(),
                    default_properties(world.component_types.iter().find(|c| &c.id == id).unwrap()),
                )
            })
            .collect(),
    }
}
fn operation<'a>(schema: &'a Value, group: &str) -> &'a Value {
    &schema["properties"]["changes"]["properties"][group]["items"]
}
fn node_variants(schema: &Value) -> &Vec<Value> {
    operation(schema, "putNodes")["properties"]["node"]["anyOf"]
        .as_array()
        .unwrap()
}

#[test]
fn provider_groups_have_single_kind_items_and_explicit_empty_unavailable_operations() {
    let w = World::default();
    let schema = effects_schema(&w);
    let changes = &schema["properties"]["changes"];
    assert_eq!(changes["type"], "object");
    assert_eq!(
        changes["required"],
        json!(["putNodes", "putLinks", "removeLinks"])
    );
    assert_eq!(changes["additionalProperties"], false);
    for (group, kind, maximum) in [("putNodes", "putNode", 80), ("putLinks", "putLink", 80)] {
        assert_eq!(changes["properties"][group]["maxItems"], maximum);
        let item = operation(&schema, group);
        assert!(item.get("anyOf").is_none());
        assert_eq!(item["properties"]["kind"]["const"], kind);
        assert_eq!(item["properties"]["evidence"]["items"]["minLength"], 8);
        assert_eq!(item["properties"]["evidence"]["maxItems"], 4);
        assert_eq!(item["additionalProperties"], false);
    }
    assert_eq!(changes["properties"]["removeLinks"]["maxItems"], 0);
    let mut empty = w;
    empty.node_types.clear();
    empty.component_types.clear();
    empty.relation_types.clear();
    let schema = effects_schema(&empty);
    for group in ["putNodes", "putLinks", "removeLinks"] {
        assert_eq!(
            schema["properties"]["changes"]["properties"][group]["maxItems"],
            0
        );
    }
}

#[test]
fn provider_groups_flatten_without_changing_facts_or_existing_flat_text() {
    let flat = " {\n  \"changes\": [], \"notes\": [], \"suggestChapterEnd\": false\n}\n";
    assert_eq!(materialize_effects_text(flat).unwrap(), flat);
    let w = World::default();
    let node = make(&w, "core.item", "new:lantern", "Storm lantern");
    let quote = "Nora set the storm lantern on the bench.";
    let put_node = json!({"kind":"putNode","node":node,"evidence":quote});
    let put_link = json!({"kind":"putLink","link":{
        "id":"new:lantern-location","typeId":"core.stored-in","sourceId":"new:lantern","targetId":"captured-workshop",
        "description":"Set on the workbench", "properties":{"custom.detail":"Brass handle facing the door"}
    },"evidence":quote});
    let remove_link = json!({"kind":"removeLink","id":"captured-old-link","evidence":quote});
    let notes =
        json!([{"text":"The lantern remains lit.","evidence":quote,"nodeIds":["new:lantern"]}]);
    let grouped = json!({"changes":{"removeLinks":[remove_link.clone()],"putLinks":[put_link.clone()],"putNodes":[put_node.clone()]},"notes":notes,"suggestChapterEnd":true});
    let text = materialize_effects_text(&grouped.to_string()).unwrap();
    let converted: PassageEffects = serde_json::from_str(&text).unwrap();
    let value = serde_json::to_value(converted).unwrap();
    // Transport conversion preserves even unsupported typed facts for the
    // native validator to report; it never repairs or discards them.
    assert_eq!(value["changes"], json!([put_node, put_link, remove_link]));
    assert_eq!(value["notes"], grouped["notes"]);
    assert_eq!(value["suggestChapterEnd"], true);
}

#[test]
fn provider_groups_reject_mismatched_kinds_missing_groups_and_unknown_fields() {
    let empty = json!({"changes":{"putNodes":[],"putLinks":[],"removeLinks":[]},"notes":[],"suggestChapterEnd":false});
    let removal =
        json!({"kind":"removeLink","id":"captured-link","evidence":"Nora took off her coat."});
    for group in ["putNodes", "putLinks"] {
        let mut value = empty.clone();
        value["changes"][group] = json!([removal.clone()]);
        assert!(materialize_effects_text(&value.to_string())
            .unwrap_err()
            .contains(group));
    }
    let mut value = empty.clone();
    value["changes"]["removeLinks"] = json!([{
        "kind":"putNode", "node":make(&World::default(), "core.item", "new:coat", "Coat"), "evidence":"Nora took off her coat."
    }]);
    assert!(materialize_effects_text(&value.to_string())
        .unwrap_err()
        .contains("removeLinks"));
    let mut value = empty.clone();
    value["changes"].as_object_mut().unwrap().remove("putLinks");
    assert!(materialize_effects_text(&value.to_string()).is_err());
    for pointer in ["", "/changes"] {
        let mut value = empty.clone();
        value
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unknown".into(), json!(true));
        assert!(materialize_effects_text(&value.to_string())
            .unwrap_err()
            .contains("unknown"));
    }
    let mut value = empty.clone();
    let mut removal = removal;
    removal["unknown"] = json!("retained error");
    value["changes"]["removeLinks"] = json!([removal]);
    assert!(materialize_effects_text(&value.to_string())
        .unwrap_err()
        .contains("unknown"));
    let mut value = empty;
    value["notes"] = json!([{"text":"A note", "evidence":"Nora took off her coat.","nodeIds":[], "unknown":true}]);
    assert!(materialize_effects_text(&value.to_string())
        .unwrap_err()
        .contains("unknown"));
}

#[test]
fn grouped_effects_preserve_native_combined_eighty_change_limit() {
    let w = World::default();
    let quote = "Nora set the storm lantern on the bench.";
    let put_node = json!({"kind":"putNode","node":make(&w,"core.item","new:lantern","Storm lantern"),"evidence":quote});
    let removal = json!({"kind":"removeLink","id":"captured-link","evidence":quote});
    let grouped = json!({"changes":{
        "putNodes":vec![put_node;41], "putLinks":[], "removeLinks":vec![removal;40]
    },"notes":[],"suggestChapterEnd":false});
    let text = materialize_effects_text(&grouped.to_string()).unwrap();
    let effects: PassageEffects = serde_json::from_str(&text).unwrap();
    assert_eq!(effects.changes.len(), 81);
    let error = preview_passage_effects(&w, &w, quote, &effects, &mut BTreeMap::new()).unwrap_err();
    assert!(error.contains("Too many proposed effects"));
}

#[test]
fn effects_use_concrete_custom_presets_and_actual_existing_capabilities() {
    let mut w = World::default();
    w.component_types.push(ComponentDefinition {
        id: "custom.tool".into(),
        name: "Tool".into(),
        description: String::new(),
        fields: vec![FieldDefinition {
            key: "condition".into(),
            label: "Condition".into(),
            kind: FieldKind::Choice,
            required: true,
            default_value: PropertyValue::Text("sound".into()),
            metadata: Some(FieldMetadata {
                choices: vec!["sound".into(), "worn".into()],
                ..Default::default()
            }),
        }],
    });
    w.node_types.push(NodeTypeDefinition {
        id: "custom.supply".into(),
        name: "Supply".into(),
        description: String::new(),
        components: vec!["core.item".into(), "custom.tool".into()],
    });
    let mut old = make(&w, "core.character", &id(), "Nora");
    old.components.remove("core.character");
    let old_id = old.id.clone();
    w.nodes.push(old);
    let schema = effects_schema(&w);
    let nodes = node_variants(&schema);
    let existing = nodes
        .iter()
        .find(|v| {
            v["properties"]["id"]["enum"]
                .as_array()
                .is_some_and(|ids| ids.contains(&json!(old_id)))
        })
        .unwrap();
    let caps = &existing["properties"]["components"];
    assert!(caps["properties"].get("core.character").is_none());
    assert_eq!(caps["required"], json!([]));
    assert_eq!(
        caps["properties"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        w.nodes[0].components.keys().collect::<Vec<_>>()
    );
    let supply = nodes
        .iter()
        .find(|v| v["properties"]["typeId"]["const"] == "custom.supply")
        .unwrap();
    let caps = &supply["properties"]["components"];
    assert_eq!(
        caps["properties"]["custom.tool"]["properties"]["condition"]["enum"],
        json!(["sound", "worn"])
    );
    assert!(caps["properties"].get("core.container").is_some());
    assert!(caps["properties"].get("core.wearable").is_some());
    assert!(caps["properties"].get("core.character").is_none());
    assert_eq!(caps["additionalProperties"], false);
    assert!(!schema.to_string().contains("$ref"));
    assert!(!schema.to_string().contains("allOf"));
}

#[test]
fn schema_preserves_planning_values_and_native_evidence_checks_remain_authoritative() {
    let mut w = World::default();
    w.component_types.push(ComponentDefinition {
        id: "custom.guidance".into(),
        name: "Guidance".into(),
        description: String::new(),
        fields: vec![FieldDefinition {
            key: "plan".into(),
            label: "Plan".into(),
            kind: FieldKind::LongText,
            required: false,
            default_value: PropertyValue::Null,
            metadata: Some(FieldMetadata {
                uses: vec![FieldUse::Planning],
                ..Default::default()
            }),
        }],
    });
    let mut person = make(&w, "core.character", &id(), "Nora");
    person.components.insert(
        "custom.guidance".into(),
        BTreeMap::from([(
            "plan".into(),
            PropertyValue::Text("Keep the lantern lit.".into()),
        )]),
    );
    w.nodes.push(person.clone());
    let schema = effects_schema(&w);
    let existing = node_variants(&schema)
        .iter()
        .find(|v| {
            v["properties"]["id"]["enum"]
                .as_array()
                .is_some_and(|ids| ids.contains(&json!(person.id)))
        })
        .unwrap();
    assert_eq!(
        existing["properties"]["components"]["properties"]["custom.guidance"]["properties"]["plan"]
            ["const"],
        "Keep the lantern lit."
    );
    assert!(node_variants(&schema)
        .iter()
        .all(|v| v["properties"]["typeId"]["const"] != "core.narration-style"));
    let prose = "Nora took shelter from the storm.";
    let mut invalid = person.clone();
    invalid
        .components
        .get_mut("custom.guidance")
        .unwrap()
        .insert(
            "plan".into(),
            PropertyValue::Text("Extinguish the lantern.".into()),
        );
    let effects = PassageEffects {
        changes: vec![PassageEffect::PutNode {
            node: invalid,
            evidence: prose.into(),
        }],
        notes: vec![],
        suggest_chapter_end: false,
    };
    let error = preview_passage_effects(&w, &w, prose, &effects, &mut BTreeMap::new()).unwrap_err();
    assert!(error.contains("author planning guidance"));
    let effects = PassageEffects {
        changes: vec![PassageEffect::PutNode {
            node: person,
            evidence: "This did not happen in the passage.".into(),
        }],
        notes: vec![],
        suggest_chapter_end: false,
    };
    assert!(
        preview_passage_effects(&w, &w, prose, &effects, &mut BTreeMap::new())
            .unwrap_err()
            .contains("Evidence quotes need correction")
    );
}

#[test]
fn reference_contracts_offer_only_captured_ids_or_new_aliases() {
    let mut w = World::default();
    let a = make(&w, "core.character", &id(), "Nora");
    let b = make(&w, "core.place", &id(), "Workshop");
    let edge = WorldEdge {
        id: id(),
        type_id: "core.present-at".into(),
        source_id: a.id.clone(),
        target_id: b.id.clone(),
        description: String::new(),
        properties: BTreeMap::new(),
    };
    w.nodes = vec![a, b];
    w.edges.push(edge.clone());
    let schema = effects_schema(&w);
    let removal = operation(&schema, "removeLinks");
    assert_eq!(removal["properties"]["id"]["enum"], json!([edge.id]));
    let link = operation(&schema, "putLinks");
    let generic = link["properties"]["link"]["anyOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["properties"]["typeId"]["const"] == "core.related")
        .unwrap();
    let refs = &generic["properties"]["sourceId"]["anyOf"];
    assert_eq!(refs[0]["enum"].as_array().unwrap().len(), 2);
    assert_eq!(refs[1]["pattern"], "^new:[A-Za-z0-9_-]{1,60}$");
    w.edges.clear();
    assert_eq!(
        effects_schema(&w)["properties"]["changes"]["properties"]["removeLinks"]["maxItems"],
        0
    );
}

#[test]
fn link_endpoint_enums_follow_capabilities_including_container_holders_and_custom_roles() {
    let mut w = World::default();
    let person = make(&w, "core.character", &id(), "Nora");
    let place = make(&w, "core.place", &id(), "Workshop");
    let plain = make(&w, "core.item", &id(), "Wrench");
    let mut container = make(&w, "core.item", &id(), "Tool chest");
    container
        .components
        .insert("core.container".into(), BTreeMap::new());
    container
        .components
        .insert("custom.mount".into(), BTreeMap::new());
    w.component_types.push(ComponentDefinition {
        id: "custom.mount".into(),
        name: "Mount".into(),
        description: String::new(),
        fields: vec![],
    });
    w.relation_types.push(RelationDefinition {
        description: String::new(),
        id: "custom.mounted-at".into(),
        name: "Mounted at".into(),
        label: "mounted at".into(),
        source_capability: Some("custom.mount".into()),
        target_capability: Some("core.place".into()),
        physical_placement: false,
        single_target: false,
        fields: vec![],
    });
    w.nodes = vec![
        person.clone(),
        place.clone(),
        plain.clone(),
        container.clone(),
    ];
    let schema = effects_schema(&w);
    let link = operation(&schema, "putLinks");
    let choices = link["properties"]["link"]["anyOf"].as_array().unwrap();
    let relationship = |id: &str| {
        &choices
            .iter()
            .find(|v| v["properties"]["typeId"]["const"] == id)
            .unwrap()["properties"]
    };
    let stored = relationship("core.stored-in");
    let mut holders = vec![person.id.clone(), place.id.clone(), container.id.clone()];
    holders.sort();
    assert_eq!(stored["targetId"]["anyOf"][0]["enum"], json!(holders));
    assert!(!stored["targetId"]["anyOf"][0]["enum"]
        .as_array()
        .unwrap()
        .contains(&json!(plain.id)));
    let present = relationship("core.present-at");
    assert_eq!(present["sourceId"]["anyOf"][0]["enum"], json!([person.id]));
    assert_eq!(present["targetId"]["anyOf"][0]["enum"], json!([place.id]));
    let mounted = relationship("custom.mounted-at");
    assert_eq!(
        mounted["sourceId"]["anyOf"][0]["enum"],
        json!([container.id])
    );
    assert_eq!(
        mounted["sourceId"]["anyOf"][1]["pattern"],
        "^new:[A-Za-z0-9_-]{1,60}$"
    );
    // No captured wearable exists, but a proposed new garment can still be used.
    let worn = relationship("core.worn-by");
    assert_eq!(worn["sourceId"]["pattern"], "^new:[A-Za-z0-9_-]{1,60}$");
    assert!(worn["sourceId"].get("enum").is_none());
}

#[test]
fn identical_existing_profiles_share_a_scoped_id_branch_without_merging_capabilities() {
    let mut w = World::default();
    w.nodes.push(make(&w, "core.character", &id(), "Nora"));
    w.nodes.push(make(&w, "core.character", &id(), "Otis"));
    let schema = effects_schema(&w);
    let existing: Vec<_> = node_variants(&schema)
        .iter()
        .filter(|v| v["properties"]["id"].get("enum").is_some())
        .collect();
    assert_eq!(existing.len(), 1);
    assert_eq!(
        existing[0]["properties"]["id"]["enum"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn sparse_existing_nodes_omit_unchanged_fields_while_links_keep_complete_contracts() {
    let mut w = World::default();
    let mut nora = make(&w, "core.character", &id(), "Nora");
    let otis = make(&w, "core.character", &id(), "Otis");
    let mut lena = make(&w, "core.character", &id(), "Lena");
    nora.components.get_mut("detail.intent").unwrap().insert(
        "objective".into(),
        PropertyValue::Text("Repair the pump".into()),
    );
    lena.components.get_mut("detail.intent").unwrap().insert(
        "objective".into(),
        PropertyValue::Text("Find shelter".into()),
    );
    let component = ComponentDefinition {
        id: "custom.state".into(),
        name: "State".into(),
        description: String::new(),
        fields: [
            ("flag", FieldKind::Boolean),
            ("count", FieldKind::Number),
            ("people", FieldKind::NodeRefs),
        ]
        .into_iter()
        .map(|(key, kind)| FieldDefinition {
            key: key.into(),
            label: key.into(),
            kind,
            required: false,
            default_value: PropertyValue::Null,
            metadata: None,
        })
        .collect(),
    };
    let values = BTreeMap::from([
        ("flag".into(), PropertyValue::Boolean(false)),
        ("count".into(), PropertyValue::Number(0.0)),
        (
            "people".into(),
            PropertyValue::NodeRefs(vec![otis.id.clone()]),
        ),
    ]);
    nora.components.insert(component.id.clone(), values.clone());
    lena.components.insert(component.id.clone(), values.clone());
    w.component_types.push(component.clone());
    let link = WorldEdge {
        id: id(),
        type_id: "custom.state-link".into(),
        source_id: nora.id.clone(),
        target_id: otis.id.clone(),
        description: String::new(),
        properties: values,
    };
    w.relation_types.push(RelationDefinition {
        id: link.type_id.clone(),
        name: "State link".into(),
        label: "state".into(),
        source_capability: None,
        target_capability: None,
        physical_placement: false,
        single_target: false,
        description: String::new(),
        fields: component.fields,
    });
    w.edges.push(link.clone());
    w.nodes.extend([nora.clone(), otis.clone(), lena.clone()]);
    let schema = effects_schema(&w);
    let branch = |id: &str| {
        node_variants(&schema)
            .iter()
            .find(|v| {
                v["properties"]["id"]["enum"]
                    .as_array()
                    .is_some_and(|ids| ids.contains(&json!(id)))
            })
            .unwrap()
    };
    let nora_schema = branch(&nora.id);
    assert_eq!(
        nora_schema["properties"]["id"]["enum"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(nora_schema["required"], json!(["id"]));
    assert_eq!(
        nora_schema["properties"]["components"]["required"],
        json!([])
    );
    assert!(
        !branch(&otis.id)["properties"]["components"]["properties"]["detail.intent"]["required"]
            .as_array()
            .unwrap()
            .contains(&json!("objective"))
    );
    let required =
        &nora_schema["properties"]["components"]["properties"]["custom.state"]["required"];
    for key in ["flag", "count", "people"] {
        assert!(!required.as_array().unwrap().contains(&json!(key)));
    }
    // Required presence is not a value lock: evidenced clearing stays explicit.
    assert!(
        nora_schema["properties"]["components"]["properties"]["detail.intent"]["properties"]
            ["objective"]
            .to_string()
            .contains("null")
    );
    let links = operation(&schema, "putLinks")["properties"]["link"]["anyOf"]
        .as_array()
        .unwrap();
    let saved = links
        .iter()
        .find(|v| {
            v["properties"]["typeId"]["const"] == link.type_id
                && v["properties"]["id"]["enum"]
                    .as_array()
                    .is_some_and(|ids| ids.contains(&json!(link.id)))
        })
        .unwrap();
    let fresh = links
        .iter()
        .find(|v| {
            v["properties"]["typeId"]["const"] == link.type_id
                && v["properties"]["id"].to_string().contains("new:")
        })
        .unwrap();
    for key in ["flag", "count", "people"] {
        assert!(saved["properties"]["properties"]["required"]
            .as_array()
            .unwrap()
            .contains(&json!(key)));
        assert!(!fresh["properties"]["properties"]["required"]
            .as_array()
            .unwrap()
            .contains(&json!(key)));
    }
}

#[test]
fn compact_contract_retains_custom_field_metadata_and_nullable_constraints() {
    let mut w = World::default();
    let fields = vec![
        FieldDefinition {
            key: "description".into(),
            label: "Observed description".into(),
            kind: FieldKind::LongText,
            required: false,
            default_value: PropertyValue::Null,
            metadata: Some(FieldMetadata {
                help: Some("Record observable wear without inventing its cause.".into()),
                uses: vec![FieldUse::Story, FieldUse::Visual],
                stability: Some(FieldStability::State),
                covered_by: vec!["torso".into()],
                ..Default::default()
            }),
        },
        FieldDefinition {
            key: "condition".into(),
            label: "Condition".into(),
            kind: FieldKind::Choice,
            required: false,
            default_value: PropertyValue::Null,
            metadata: Some(FieldMetadata {
                choices: vec!["sound".into(), "worn".into()],
                ..Default::default()
            }),
        },
        FieldDefinition {
            key: "owner".into(),
            label: "Owner".into(),
            kind: FieldKind::NodeRef,
            required: false,
            default_value: PropertyValue::Null,
            metadata: Some(FieldMetadata {
                reference_capability: Some("core.character".into()),
                ..Default::default()
            }),
        },
        FieldDefinition {
            key: "enabled".into(),
            label: "Enabled".into(),
            kind: FieldKind::Boolean,
            required: true,
            default_value: PropertyValue::Boolean(false),
            metadata: None,
        },
        FieldDefinition {
            key: "count".into(),
            label: "Count".into(),
            kind: FieldKind::Number,
            required: false,
            default_value: PropertyValue::Number(0.0),
            metadata: None,
        },
    ];
    w.component_types.push(ComponentDefinition {
        id: "custom.condition".into(),
        name: "Condition".into(),
        description: "Condition observed in this passage.".into(),
        fields: fields.clone(),
    });
    w.node_types.push(NodeTypeDefinition {
        id: "custom.observation".into(),
        name: "Observation".into(),
        description: String::new(),
        components: vec!["custom.condition".into()],
    });
    w.relation_types.push(RelationDefinition {
        description: String::new(),
        id: "custom.observed-by".into(),
        name: "Observed by".into(),
        label: "observed by".into(),
        source_capability: Some("custom.condition".into()),
        target_capability: Some("core.character".into()),
        physical_placement: false,
        single_target: true,
        fields,
    });
    let schema = effects_schema(&w);
    let record = node_variants(&schema)
        .iter()
        .find(|v| v["properties"]["typeId"]["const"] == "custom.observation")
        .unwrap();
    // A literal field called description is data, not a schema annotation.
    assert_eq!(record["properties"]["description"]["type"], "string");
    let properties = &record["properties"]["components"]["properties"]["custom.condition"];
    assert_eq!(properties["required"], json!(["enabled"]));
    assert_eq!(
        properties["properties"]["description"]["type"],
        json!(["string", "null"])
    );
    assert_eq!(properties["properties"]["description"]["maxLength"], 16000);
    assert!(properties["properties"]["description"]
        .get("description")
        .is_none());
    assert_eq!(
        properties["properties"]["condition"]["enum"],
        json!(["sound", "worn", null])
    );
    assert_eq!(
        properties["properties"]["condition"]["type"],
        json!(["string", "null"])
    );
    assert_eq!(
        properties["properties"]["owner"]["type"],
        json!(["string", "null"])
    );
    assert_eq!(properties["properties"]["enabled"]["type"], "boolean");
    assert_eq!(properties["additionalProperties"], false);
    let catalogue = crate::prompt_context::capabilities(&w, false);
    let component = catalogue
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "custom.condition")
        .unwrap();
    let descriptors = component["fields"].as_array().unwrap();
    assert_eq!(
        descriptors[0],
        json!({
            "key":"description", "label":"Observed description", "kind":"longText",
            "help":"Record observable wear without inventing its cause.",
            "uses":["story","visual"], "stability":"state", "coveredBy":["torso"]
        })
    );
    assert_eq!(descriptors[1]["choices"], json!(["sound", "worn"]));
    assert_eq!(descriptors[2]["referenceCapability"], "core.character");
    assert_eq!(descriptors[3]["defaultValue"], false);
    assert_eq!(descriptors[3]["required"], true);
    assert_eq!(descriptors[4]["defaultValue"], 0.0);
    let catalogue = crate::prompt_context::relationships(&w, false);
    let relationship = catalogue
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "custom.observed-by")
        .unwrap();
    assert_eq!(relationship["fields"], component["fields"]);
    assert_eq!(relationship["sourceCapability"], "custom.condition");
    assert_eq!(relationship["targetCapability"], "core.character");
    assert_eq!(relationship["singleTarget"], true);
    assert_eq!(relationship["physicalPlacement"], false);
}

#[test]
fn furnished_two_person_scene_fits_real_effects_preflight_at_32k() {
    let mut w = World::default();
    let lodge = make(&w, "core.place", &id(), "Ridgeway Lodge");
    let room = make(&w, "core.place", &id(), "Workshop");
    let nora = make(&w, "core.character", &id(), "Nora Hale");
    let otis = make(&w, "core.character", &id(), "Otis Reed");
    for (kind, source, target) in [
        ("core.inside", &room.id, &lodge.id),
        ("core.present-at", &nora.id, &room.id),
        ("core.present-at", &otis.id, &room.id),
    ] {
        w.edges.push(WorldEdge {
            id: id(),
            type_id: kind.into(),
            source_id: source.clone(),
            target_id: target.clone(),
            description: String::new(),
            properties: BTreeMap::new(),
        });
    }
    for person in [&nora, &otis] {
        for (name, regions) in [
            ("Wool shirt", "torso, arms"),
            ("Work trousers", "waist, hips, legs"),
            ("Leather boots", "feet"),
        ] {
            let mut garment = make(
                &w,
                "core.wearable-item",
                &id(),
                &format!("{}'s {name}", person.name),
            );
            garment
                .components
                .get_mut("core.wearable")
                .unwrap()
                .insert("regions".into(), PropertyValue::Text(regions.into()));
            w.edges.push(WorldEdge {
                id: id(),
                type_id: "core.worn-by".into(),
                source_id: garment.id.clone(),
                target_id: person.id.clone(),
                description: String::new(),
                properties: BTreeMap::new(),
            });
            w.nodes.push(garment);
        }
    }
    for (name, holder) in [
        ("Brass key", &nora.id),
        ("Storm lantern", &room.id),
        ("Coil of rope", &room.id),
    ] {
        let item = make(&w, "core.item", &id(), name);
        w.edges.push(WorldEdge {
            id: id(),
            type_id: "core.stored-in".into(),
            source_id: item.id.clone(),
            target_id: holder.clone(),
            description: String::new(),
            properties: BTreeMap::new(),
        });
        w.nodes.push(item);
    }
    let (room_id, nora_id, otis_id) = (room.id.clone(), nora.id.clone(), otis.id.clone());
    w.nodes.extend([lodge, room, nora, otis]);
    let dir = tempfile::tempdir().unwrap();
    let p = Project::create(
        &dir.path().join("story"),
        "Fictional budget regression".into(),
        w.clone(),
        id(),
    )
    .unwrap();
    let draft = id();
    let mut plan = p.next_passage_plan().unwrap();
    plan.location_id = Some(room_id);
    plan.viewpoint_id = Some(nora_id);
    plan.participant_ids = vec![otis_id];
    plan.target_words = 180;
    plan.direction = "Nora hands the brass key to Otis. Stay in the workshop.".into();
    p.save_passage(&draft, 0, PassageEdits {
        plan,
        prose: "Nora pressed the brass key into Otis's hand. He closed his fingers around it and waited beside the workshop bench.".into(),
        effects_text: serde_json::to_string(&PassageEffects::default()).unwrap(),
        close_chapter: false,
        revision_direction: String::new(),
        approved_effects: vec![],
    }).unwrap();
    let settings = LlmConnection {
        provider: TextProvider::Demo,
        structured_output: StructuredOutput::Schema,
        context_tokens: 32000,
        max_tokens: 1800,
        ..Default::default()
    };
    let prepared = p.start_passage(&draft, 1, 0, true, &settings).unwrap();
    let schema = prepared.schema.as_ref().unwrap();
    assert!(
        (prepared.system.len() + prepared.user.len() + schema.to_string().len()) / 3
            + 256
            + prepared.output_tokens as usize
            <= 32000
    );
    assert!(prepared.user.contains("Brass key"));
    assert_eq!(p.view().unwrap().revision, 0);
    assert_eq!(p.view().unwrap().world, w);
    assert!(p.story_index().unwrap().passages.is_empty());
}
