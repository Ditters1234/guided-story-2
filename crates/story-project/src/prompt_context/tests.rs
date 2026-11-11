use super::*;
use std::collections::BTreeMap;

#[test]
fn shared_concrete_components_and_scoped_identities_expand_without_losing_constraints() {
    let world = World::default();
    let original = expanded_test_schema(&crate::story_effects::effects_schema(&world));
    let mut shared = original.clone();
    share_component_schemas(&mut shared);
    assert!(shared.get("$defs").is_some());
    assert!(shared.to_string().len() < original.to_string().len());
    assert_eq!(expanded_test_schema(&shared), original);
    let literal = json!({"type":"object","properties":{"text":{"type":"string","maxLength":16000}},"required":["text"],"additionalProperties":false});
    let mut contract = json!({"type":"object","properties":{"a":literal,"b":literal},"const":{"properties":{"a":literal,"b":literal}}});
    let protected = contract["const"].clone();
    share_component_schemas(&mut contract);
    assert_eq!(contract["const"], protected);
    let once = contract.clone();
    share_component_schemas(&mut contract);
    assert_eq!(contract, once);
}

#[test]
fn relationship_meanings_and_field_help_survive_catalogue_projection() {
    let world = World::default();
    for formal in [false, true] {
        let values = relationships(&world, formal);
        for definition in &world.relation_types {
            let value = values
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["id"] == definition.id)
                .unwrap();
            if !definition.description.is_empty() {
                assert_eq!(value["description"], definition.description);
            }
            for field in &definition.fields {
                if let Some(help) = field.metadata.as_ref().and_then(|m| m.help.as_ref()) {
                    let output = value["fields"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|f| f["key"] == field.key)
                        .unwrap();
                    assert_eq!(output["help"], *help);
                }
            }
        }
    }
}

#[test]
fn reference_compaction_preserves_markers_required_values_unknown_data_zero_and_false() {
    let mut world = World::default();
    let field = |key: &str, kind, required| FieldDefinition {
        key: key.into(),
        label: key.into(),
        kind,
        required,
        default_value: PropertyValue::Null,
        metadata: None,
    };
    world.component_types.push(ComponentDefinition {
        id: "custom.facts".into(),
        name: "Facts".into(),
        description: String::new(),
        fields: vec![
            field("required", FieldKind::Text, true),
            field("optional", FieldKind::Text, false),
            field("unset", FieldKind::Text, false),
            field("refs", FieldKind::NodeRefs, false),
            field("count", FieldKind::Number, false),
            field("enabled", FieldKind::Boolean, false),
        ],
    });
    let node = WorldNode {
        id: "new:reference".into(),
        type_id: "core.item".into(),
        name: "Supply case".into(),
        description: String::new(),
        visual_description: String::new(),
        components: BTreeMap::from([
            ("core.container".into(), BTreeMap::new()),
            (
                "custom.facts".into(),
                BTreeMap::from([
                    ("required".into(), PropertyValue::Text(String::new())),
                    ("optional".into(), PropertyValue::Text(String::new())),
                    ("unset".into(), PropertyValue::Null),
                    ("refs".into(), PropertyValue::NodeRefs(vec![])),
                    ("count".into(), PropertyValue::Number(0.0)),
                    ("enabled".into(), PropertyValue::Boolean(false)),
                    ("unknown".into(), PropertyValue::Null),
                ]),
            ),
        ]),
    };
    let original = json!(node);
    let compact = reference_node(&world, &node);
    let fields = &compact["components"]["custom.facts"];
    for key in ["optional", "unset", "refs"] {
        assert!(fields.get(key).is_none());
    }
    for key in ["required", "count", "enabled", "unknown"] {
        assert_eq!(fields[key], original["components"]["custom.facts"][key]);
        assert!(fields.get(key).is_some());
    }
    assert_eq!(compact["components"]["core.container"], json!({}));
    assert_eq!(json!(node), original);
    assert!(compact.to_string().len() < original.to_string().len());
}

#[test]
fn shared_catalogue_metadata_expands_to_the_exact_project_semantics_in_all_modes() {
    let world = World::default();
    for formal_schema in [false, true] {
        let catalogue = capabilities(&world, formal_schema);
        for definition in &world.component_types {
            let entry = catalogue
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["id"] == definition.id)
                .unwrap();
            for field in &definition.fields {
                let descriptor = entry["fields"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|f| f["key"] == field.key)
                    .unwrap();
                assert_eq!(
                    descriptor
                        .get("label")
                        .and_then(Value::as_str)
                        .unwrap_or(&field.key)
                        .to_lowercase(),
                    field.label.to_lowercase()
                );
                for key in ["uses", "stability"] {
                    let value = descriptor
                        .get(key)
                        .or_else(|| entry["fieldDefaults"].get(key));
                    let expected = match key {
                        "uses" => field
                            .metadata
                            .as_ref()
                            .filter(|m| !m.uses.is_empty())
                            .map(|m| json!(m.uses)),
                        _ => field
                            .metadata
                            .as_ref()
                            .and_then(|m| m.stability)
                            .map(|s| json!(s)),
                    };
                    assert_eq!(
                        value,
                        expected.as_ref(),
                        "{} / {}",
                        definition.id,
                        field.key
                    );
                }
                if field.default_value != PropertyValue::Null {
                    assert_eq!(descriptor["defaultValue"], json!(field.default_value));
                }
                if !formal_schema {
                    assert_eq!(descriptor["kind"], json!(field.kind));
                    if field.required {
                        assert_eq!(descriptor["required"], true);
                    }
                }
            }
        }
    }
}

#[test]
fn envelope_compaction_cannot_modify_protected_records_or_json_schema_constants() {
    let mut input = json!({
        "sources":[], "direction":"", "currentChapter":null,
        "records":[{"components":{"custom":{"text":"","enabled":false,"count":0}}}],
        "responseSchema":{"type":"object","properties":{},"const":{"text":"","enabled":false}},
        "prose":"  Exact quotation.\n",
        "plan":{"direction":"","viewpointId":null,"referenceIds":[],"targetWords":0}
    });
    let records = input["records"].clone();
    let schema = input["responseSchema"].clone();
    compact_envelope(&mut input);
    assert_eq!(input["records"], records);
    assert_eq!(input["responseSchema"], schema);
    assert_eq!(input["prose"], "  Exact quotation.\n");
    assert_eq!(input["plan"]["targetWords"], 0);
    assert!(input.get("sources").is_none());
}
