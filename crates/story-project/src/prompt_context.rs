//! Provider-only projections. Stored records and native validation retain their
//! complete representations, including explicit empty values and marker capabilities.
use crate::*;
use serde_json::{json, Value};
mod references;
#[cfg(test)]
mod tests;
pub(crate) use references::ReferenceScope;

fn field_descriptor(field: &FieldDefinition, formal_schema: bool) -> Value {
    let mut value = json!({"key":field.key,"label":field.label});
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
