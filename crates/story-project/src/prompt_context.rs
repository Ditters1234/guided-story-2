//! Provider-only projections. Stored records and native validation retain their
//! complete representations, including explicit empty values and marker capabilities.
use crate::*;
use serde_json::{json, Value};
mod references;
#[cfg(test)]
mod tests;
pub(crate) use references::ReferenceScope;

#[cfg(test)]
pub(crate) fn expanded_test_schema(schema: &Value) -> Value {
    fn expand(value: &Value, root: &Value) -> Value {
        if let Some(path) = value.get("$ref").and_then(Value::as_str) {
            return expand(
                root.pointer(path.strip_prefix('#').unwrap())
                    .expect("valid local schema reference"),
                root,
            );
        }
        match value {
            Value::Object(object) => json!(object
                .iter()
                .filter(|(key, _)| *key != "$defs")
                .map(|(key, value)| (key.clone(), expand(value, root)))
                .collect::<serde_json::Map<_, _>>()),
            Value::Array(array) => json!(array.iter().map(|v| expand(v, root)).collect::<Vec<_>>()),
            _ => value.clone(),
        }
    }
    expand(schema, schema)
}

fn field_descriptor(field: &FieldDefinition, formal_schema: bool) -> Value {
    let mut value = json!({"key":field.key});
    // A plain case/spelling duplicate adds no meaning. Keep nontrivial labels
    // (including custom labels); the key is the fallback in this catalogue.
    if !field
        .label
        .eq_ignore_ascii_case(&field.key.replace(['-', '_'], " "))
    {
        value["label"] = json!(field.label);
    }
    if !formal_schema {
        value["kind"] = json!(field.kind);
        if field.required {
            value["required"] = json!(true);
        }
    }
    if field.default_value != PropertyValue::Null {
        value["defaultValue"] = json!(field.default_value);
    }
    if let Some(meta) = &field.metadata {
        if let Some(help) = meta.help.as_ref().filter(|s| !s.is_empty()) {
            value["help"] = json!(help);
        }
        if !meta.uses.is_empty() {
            value["uses"] = json!(meta.uses);
        }
        if let Some(stability) = &meta.stability {
            value["stability"] = json!(stability);
        }
        if !formal_schema && !meta.choices.is_empty() {
            value["choices"] = json!(meta.choices);
        }
        if let Some(capability) = &meta.reference_capability {
            value["referenceCapability"] = json!(capability);
        }
        if !meta.covered_by.is_empty() {
            value["coveredBy"] = json!(meta.covered_by);
        }
    }
    value
}

/// Share repeated concrete field/component contracts, not an all-capability node schema.
/// Every preset still declares its own properties/required capability set. Run
/// after reference permissions are bound; native validation remains complete.
pub(crate) fn share_component_schemas(schema: &mut Value) {
    if schema.get("$defs").is_some() {
        return;
    }
    fn visit(value: &mut Value, f: &mut impl FnMut(&mut Value)) {
        match value {
            Value::Object(object) => {
                if let Some(properties) =
                    object.get_mut("properties").and_then(Value::as_object_mut)
                {
                    for field in properties.values_mut() {
                        if field.get("type").is_some() || field.get("anyOf").is_some() {
                            f(field);
                        }
                    }
                }
                for (keyword, child) in object.iter_mut() {
                    // These are literal values, never schema locations. In
                    // particular, protected const records must remain byte-exact.
                    if !matches!(keyword.as_str(), "const" | "enum" | "default" | "examples") {
                        visit(child, f);
                    }
                }
            }
            Value::Array(values) => {
                for child in values {
                    visit(child, f);
                }
            }
            _ => (),
        }
    }
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    visit(schema, &mut |component| {
        let key = component.to_string();
        *counts.entry(key).or_default() += 1;
    });
    let repeated: std::collections::BTreeMap<_, _> = counts
        .into_iter()
        .filter(|(key, count)| *count > 1 && (*count - 1) * key.len() > *count * 40 + 32)
        .enumerate()
        .map(|(i, (schema, _))| (schema, format!("contract{i}")))
        .collect();
    let mut definitions = serde_json::Map::new();
    visit(schema, &mut |component| {
        if let Some(name) = repeated.get(&component.to_string()) {
            definitions.insert(name.clone(), component.clone());
            *component = json!({"$ref":format!("#/$defs/{name}")});
        }
    });
    if !definitions.is_empty() {
        schema["$defs"] = json!(definitions);
    }
}

fn add_fields(value: &mut Value, definitions: &[FieldDefinition], formal_schema: bool) {
    if definitions.is_empty() {
        return;
    }
    let mut fields: Vec<_> = definitions
        .iter()
        .map(|f| field_descriptor(f, formal_schema))
        .collect();
    let mut defaults = serde_json::Map::new();
    // Only metadata identical on every field is shared. Individual meanings,
    // constraints and reference filters retain their original field keys.
    if fields.len() > 1 {
        for key in ["uses", "stability"] {
            if let Some(common) = fields[0].get(key).cloned() {
                if fields.iter().all(|f| f.get(key) == Some(&common)) {
                    defaults.insert(key.into(), common);
                    for field in &mut fields {
                        field.as_object_mut().unwrap().remove(key);
                    }
                }
            }
        }
    }
    value["fields"] = json!(fields);
    if !defaults.is_empty() {
        value["fieldDefaults"] = json!(defaults);
    }
}

pub(crate) fn capabilities(world: &World, formal_schema: bool) -> Value {
    json!(world
        .component_types
        .iter()
        .map(|definition| {
            let mut value = json!({"id":definition.id,"name":definition.name});
            if !definition.description.is_empty() {
                value["description"] = json!(definition.description);
            }
            add_fields(&mut value, &definition.fields, formal_schema);
            value
        })
        .collect::<Vec<_>>())
}

pub(crate) fn relationships(world: &World, formal_schema: bool) -> Value {
    json!(world.relation_types.iter().map(|definition| {
        let mut value = json!({"id":definition.id,"label":definition.label,
            "physicalPlacement":definition.physical_placement,"singleTarget":definition.single_target});
        if let Some(capability) = &definition.source_capability {
            value["sourceCapability"] = json!(capability);
        }
        if let Some(capability) = &definition.target_capability {
            value["targetCapability"] = json!(capability);
        }
        if !definition.description.is_empty() {
            value["description"] = json!(definition.description);
        }
        add_fields(&mut value, &definition.fields, formal_schema);
        value
    }).collect::<Vec<_>>())
}

fn unspecified(value: &Value) -> bool {
    value.is_null() || value.as_str() == Some("") || value.as_array().is_some_and(Vec::is_empty)
}

pub(crate) fn reference_node(world: &World, node: &WorldNode) -> Value {
    let mut value = json!(node);
    for definition in &world.component_types {
        if let Some(fields) = value["components"]
            .get_mut(&definition.id)
            .and_then(Value::as_object_mut)
        {
            fields.retain(|key, value| {
                !unspecified(value)
                    || !definition
                        .fields
                        .iter()
                        .any(|f| f.key == *key && !f.required)
            });
        }
    }
    if let Some(object) = value.as_object_mut() {
        for key in ["description", "visualDescription"] {
            if object.get(key).is_some_and(unspecified) {
                object.remove(key);
            }
        }
    }
    value
}

pub(crate) fn compact_envelope(input: &mut Value) {
    // Only task-envelope fields with an explicitly optional meaning. Never
    // recursively strip schemas, evidence, protected records or user content.
    let Some(object) = input.as_object_mut() else {
        return;
    };
    for key in [
        "sources",
        "recentPassages",
        "includedReferenceFacts",
        "nativeImageGuidanceNodes",
        "direction",
        "existingPrompt",
        "existingNegative",
        "currentChapter",
    ] {
        if object.get(key).is_some_and(unspecified) {
            object.remove(key);
        }
    }
    if let Some(plan) = object.get_mut("plan").and_then(Value::as_object_mut) {
        for key in [
            "chapterPlan",
            "direction",
            "viewpointId",
            "locationId",
            "intendedLocationId",
            "participantIds",
            "intendedParticipantIds",
            "referenceIds",
        ] {
            if plan.get(key).is_some_and(unspecified) {
                plan.remove(key);
            }
        }
    }
}
