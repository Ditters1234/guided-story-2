use super::*;
use serde_json::{json, Value};

#[test]
fn bundled_world_data_matches_native_structures_and_every_preset_default() {
    crate::validate_bundled_world_data().unwrap();
}

#[test]
fn absent_relationship_descriptions_keep_the_original_serialized_representation() {
    // Frozen pre-description serialized relation, including the original field order.
    let original = r#"{"id":"core.related","name":"Related to","label":"related to","sourceCapability":null,"targetCapability":null,"physicalPlacement":false,"singleTarget":false}"#;
    let relation: RelationDefinition = serde_json::from_str(original).unwrap();
    assert!(relation.description.is_empty());
    assert_eq!(serde_json::to_string(&relation).unwrap(), original);
    let mut described = relation.clone();
    described.description = "An association, never physical placement.".into();
    assert_eq!(
        serde_json::from_str::<RelationDefinition>(&serde_json::to_string(&described).unwrap())
            .unwrap(),
        described
    );
    let mut empty: Value = serde_json::from_str(original).unwrap();
    empty["description"] = json!("");
    assert_eq!(
        serde_json::from_value::<RelationDefinition>(empty).unwrap(),
        relation
    );
    let core: World =
        serde_json::from_str(include_str!("../../../../schema/world-core.json")).unwrap();
    core.validate().unwrap();
    let encoded = serde_json::to_string(&core).unwrap();
    assert!(!serde_json::to_value(&core).unwrap()["relationTypes"][0]
        .as_object()
        .unwrap()
        .contains_key("description"));
    let stored: StoredWorld = serde_json::from_str(&encoded).unwrap();
    assert_eq!(serde_json::to_string(&stored).unwrap(), encoded);
}

#[test]
fn explanatory_relationship_text_never_weakens_core_behavior() {
    let w = World::bundled().unwrap();
    for property in [
        "physicalPlacement",
        "singleTarget",
        "sourceCapability",
        "targetCapability",
        "label",
    ] {
        let mut raw = serde_json::to_value(&w).unwrap();
        let relation = raw["relationTypes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["id"] == "core.inside")
            .unwrap();
        relation[property] = match property {
            "physicalPlacement" | "singleTarget" => json!(false),
            "label" => json!("changed"),
            _ => Value::Null,
        };
        let changed: World = serde_json::from_value(raw).unwrap();
        assert!(
            changed
                .validate()
                .unwrap_err()
                .contains("built-in relationship"),
            "{property}"
        );
    }
    let mut w = w;
    w.relation_types
        .iter_mut()
        .find(|r| r.id == "core.related")
        .unwrap()
        .description = "Project explanation".into();
    w.validate().unwrap();
    let mut raw = serde_json::to_value(w).unwrap();
    raw["relationTypes"][0]["unrecognized"] = json!(true);
    assert!(serde_json::from_value::<World>(raw).is_err());
}

#[test]
fn invalid_bundles_return_filename_and_validation_diagnostics_without_upgrading() {
    let bad = parse_bundled_world("broken.json", "{\n\"nodes\":!").unwrap_err();
    assert!(
        bad.contains("broken.json") && bad.contains("line 2"),
        "{bad}"
    );
    let mut raw: Value =
        serde_json::from_str(include_str!("../../../../schema/world-presets.json")).unwrap();
    raw["relationTypes"][0]["unknown"] = json!(true);
    let bad = parse_bundled_world("unknown.json", &raw.to_string()).unwrap_err();
    assert!(
        bad.contains("unknown.json") && bad.contains("unknown field"),
        "{bad}"
    );
    raw["relationTypes"][0]
        .as_object_mut()
        .unwrap()
        .remove("unknown");
    raw["relationTypes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["id"] == "core.worn-by")
        .unwrap()["physicalPlacement"] = json!(false);
    let bad = parse_bundled_world("bad-contract.json", &raw.to_string()).unwrap_err();
    assert!(
        bad.contains("bad-contract.json") && bad.contains("built-in relationship"),
        "{bad}"
    );
    let error = bundled_json::<Value>("knowledge-core.json", "{").unwrap_err();
    assert!(error.contains("knowledge-core.json") && error.contains("line"));
}
