//! Read-only, schema-driven projections. Never writes summaries back into nodes.
use crate::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;
#[cfg(test)]
mod tests;

pub fn default_properties(definition: &ComponentDefinition) -> BTreeMap<String, PropertyValue> {
    definition
        .fields
        .iter()
        .filter(|field| field.required || field.default_value != PropertyValue::Null)
        .map(|field| (field.key.clone(), field.default_value.clone()))
        .collect()
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
        "notes":node.description,"visualNotes":node.visual_description,"details":sections})
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
