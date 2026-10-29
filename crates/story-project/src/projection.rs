//! Read-only, schema-driven projections. Never writes summaries back into nodes.
use crate::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;
#[cfg(test)]
mod tests;

pub fn default_properties(definition: &ComponentDefinition) -> BTreeMap<String, PropertyValue> {
    default_field_values(&definition.fields)
}
pub fn default_field_values(fields: &[FieldDefinition]) -> BTreeMap<String, PropertyValue> {
    fields
        .iter()
        .filter(|field| field.required || field.default_value != PropertyValue::Null)
        .map(|field| (field.key.clone(), field.default_value.clone()))
        .collect()
}

pub(crate) fn property_references(
    fields: &[FieldDefinition],
    values: &BTreeMap<String, PropertyValue>,
) -> Vec<String> {
    fields
        .iter()
        .flat_map(|field| match (&field.kind, values.get(&field.key)) {
            (FieldKind::NodeRef, Some(PropertyValue::Text(id))) => vec![id.clone()],
            (FieldKind::NodeRefs, Some(PropertyValue::NodeRefs(ids))) => ids.clone(),
            _ => vec![],
        })
        .collect()
}

pub(crate) fn resolve_property_references(
    fields: &[FieldDefinition],
    values: &mut BTreeMap<String, PropertyValue>,
    previous: Option<&BTreeMap<String, PropertyValue>>,
    allowed: &std::collections::BTreeSet<String>,
    resolve: impl Fn(&str) -> String,
) -> Result<(), String> {
    for field in fields {
        if !matches!(field.kind, FieldKind::NodeRef | FieldKind::NodeRefs) {
            continue;
        }
        let refs = property_references(std::slice::from_ref(field), values);
        if previous.and_then(|v| v.get(&field.key)) != values.get(&field.key)
            && refs.iter().any(|id| !allowed.contains(id))
        {
            return Err(format!(
                "{} references a node outside the supplied context.",
                field.label
            ));
        }
        match values.get_mut(&field.key) {
            Some(PropertyValue::Text(id)) => *id = resolve(id),
            Some(PropertyValue::NodeRefs(ids)) => {
                for id in ids {
                    *id = resolve(id);
                }
            }
            _ => (),
        }
    }
    Ok(())
}

pub fn narrative_link(world: &World, edge: &WorldEdge) -> Value {
    let definition = world.relation_types.iter().find(|d| d.id == edge.type_id);
    let fields: Vec<_> = definition
        .into_iter()
        .flat_map(|d| &d.fields)
        .filter(|f| f.used_for(FieldUse::Story))
        .filter_map(|f| {
            edge.properties
                .get(&f.key)
                .filter(|v| {
                    **v != PropertyValue::Null
                        && **v != PropertyValue::Text(String::new())
                        && **v != PropertyValue::NodeRefs(vec![])
                })
                .map(|v| json!({"key":f.key,"label":f.label,"value":v}))
        })
        .collect();
    json!({"id":edge.id,"typeId":edge.type_id,"relationship":definition.map(|d| &d.label),"sourceId":edge.source_id,"targetId":edge.target_id,"description":edge.description,"details":fields})
}

pub fn visual_link_summary(world: &World, edge: &WorldEdge) -> String {
    let mut parts = vec![];
    if !edge.description.trim().is_empty() {
        parts.push(edge.description.clone());
    }
    for field in world
        .relation_types
        .iter()
        .find(|d| d.id == edge.type_id)
        .into_iter()
        .flat_map(|d| &d.fields)
        .filter(|f| f.used_for(FieldUse::Visual))
    {
        if let Some(value) = edge.properties.get(&field.key).filter(|v| {
            **v != PropertyValue::Null
                && **v != PropertyValue::Text(String::new())
                && **v != PropertyValue::NodeRefs(vec![])
        }) {
            parts.push(format!(
                "{}: {}",
                field.label,
                field_value_text(world, field, Some(value))
            ));
        }
    }
    parts.join("\n")
}

pub(crate) fn field_value_text(
    world: &World,
    field: &FieldDefinition,
    value: Option<&PropertyValue>,
) -> String {
    let name = |id: &str| {
        world
            .nodes
            .iter()
            .find(|n| n.id == id)
            .map_or_else(|| "Missing node".into(), |n| n.name.clone())
    };
    match value {
        None | Some(PropertyValue::Null) => "Unspecified".into(),
        Some(PropertyValue::Text(s)) if s.is_empty() => "Unspecified".into(),
        Some(PropertyValue::Text(s)) if matches!(field.kind, FieldKind::NodeRef) => name(s),
        Some(PropertyValue::Text(s)) => s.clone(),
        Some(PropertyValue::Number(n)) => n.to_string(),
        Some(PropertyValue::Boolean(b)) => if *b { "Yes" } else { "No" }.into(),
        Some(PropertyValue::NodeRefs(ids)) => {
            ids.iter().map(|id| name(id)).collect::<Vec<_>>().join(", ")
        }
    }
}

pub fn projected_node(world: &World, node: &WorldNode, usage: FieldUse) -> WorldNode {
    let mut projected = node.clone();
    for (id, values) in &mut projected.components {
        values.retain(|key, _| {
            world
                .component_types
                .iter()
                .find(|d| &d.id == id)
                .and_then(|d| d.fields.iter().find(|f| &f.key == key))
                .is_some_and(|f| f.used_for(usage))
        });
    }
    projected
}

/// Human-labelled fields keep custom keys meaningful to smaller models. Values
/// appear once; field defaults and visual/planning-only data are not recalled.
pub fn narrative_record(world: &World, node: &WorldNode) -> Value {
    let sections: Vec<_> = world
        .component_types
        .iter()
        .filter_map(|definition| {
            let values = node.components.get(&definition.id)?;
            let fields: Vec<_> = definition
                .fields
                .iter()
                .filter(|field| field.used_for(FieldUse::Story))
                .filter_map(|field| {
                    let value = values.get(&field.key)?;
                    if value == &PropertyValue::Null
                        || value == &PropertyValue::Text(String::new())
                        || value == &PropertyValue::NodeRefs(vec![])
                    {
                        return None;
                    }
                    Some(json!({"key":field.key,"label":field.label,"value":value,
                    "help":field.metadata.as_ref().and_then(|m| m.help.as_ref()),
                    "stability":field.metadata.as_ref().and_then(|m| m.stability)}))
                })
                .collect();
            (!fields.is_empty()).then(
                || json!({"capability":definition.id,"section":definition.name,"fields":fields}),
            )
        })
        .collect();
    json!({"id":node.id,"name":node.name,"typeId":node.type_id,
        "capabilities":node.components.keys().collect::<Vec<_>>(),
        "notes":node.description,"visualNotes":node.visual_description,"details":sections,
        "sources":world.source_annotations.iter().filter(|a|a.node_id==node.id).map(|a|serde_json::json!({"origin":a.origin,"recordEditedSinceCitation":a.subject_fingerprint!=crate::sources::node_source_fingerprint(node)})).collect::<Vec<_>>()})
}

/// Values with no visual hint are not inferred to describe appearance. Supplementary
/// visual notes remain explicit appearance input, and field order is deterministic.
pub fn visual_summary(world: &World, node: &WorldNode, covered_regions: &[String]) -> String {
    let normalize = |s: &str| {
        s.replace(['-', '_'], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    };
    let mut lines = Vec::new();
    if !node.visual_description.trim().is_empty() {
        lines.push(node.visual_description.clone());
    }
    for definition in &world.component_types {
        let Some(values) = node.components.get(&definition.id) else {
            continue;
        };
        for field in &definition.fields {
            if !field.used_for(FieldUse::Visual) {
                continue;
            }
            if field.metadata.as_ref().is_some_and(|meta| {
                meta.covered_by.iter().any(|region| {
                    covered_regions.iter().any(|covered| {
                        let covered = normalize(covered);
                        covered == normalize(region)
                            || matches!(covered.as_str(), "all" | "whole body" | "full body")
                    })
                })
            }) {
                continue;
            }
            let Some(value) = values.get(&field.key) else {
                continue;
            };
            let text = match value {
                PropertyValue::Null => continue,
                PropertyValue::Text(s) if s.trim().is_empty() => continue,
                PropertyValue::Text(s) => s.clone(),
                PropertyValue::Number(n) => n.to_string(),
                PropertyValue::Boolean(b) => b.to_string(),
                PropertyValue::NodeRefs(ids) if ids.is_empty() => continue,
                PropertyValue::NodeRefs(ids) => ids
                    .iter()
                    .filter_map(|id| world.nodes.iter().find(|n| &n.id == id))
                    .map(|n| n.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            };
            let text = if matches!(field.kind, FieldKind::NodeRef) {
                world
                    .nodes
                    .iter()
                    .find(|n| n.id == text)
                    .map(|n| n.name.clone())
                    .unwrap_or(text)
            } else {
                text
            };
            lines.push(format!("{} / {}: {}", definition.name, field.label, text));
        }
    }
    lines.join("\n")
}
