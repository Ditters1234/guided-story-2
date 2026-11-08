use crate::{PropertyValue, World, WorldEdge, WorldNode};
use serde::Deserialize;
use std::collections::BTreeMap;
use uuid::Uuid;

// Authoring data only. Templates become ordinary project-local nodes; there is
// no live inheritance or dependency on this catalogue after project creation.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LocationTemplate {
    id: String,
    name: String,
    description: String,
    nodes: Vec<TemplateNode>,
    #[serde(default)]
    edges: Vec<TemplateEdge>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TemplateEdge {
    key: String,
    type_id: String,
    source_key: String,
    target_key: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    properties: BTreeMap<String, PropertyValue>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TemplateNode {
    key: String,
    type_id: String,
    name: String,
    description: String,
    visual_description: String,
    parent: Option<String>,
    #[serde(default)]
    components: BTreeMap<String, BTreeMap<String, PropertyValue>>,
}

fn catalogue() -> Result<Vec<LocationTemplate>, String> {
    parse_catalogue(include_str!("../../../schema/location-templates.json"))
}

fn parse_catalogue(text: &str) -> Result<Vec<LocationTemplate>, String> {
    let entries: Vec<serde_json::Value> =
        crate::model::bundled_json("location-templates.json", text)?;
    let mut seen = std::collections::BTreeSet::new();
    entries.into_iter().enumerate().map(|(index, value)| {
        let id = value["id"].as_str().unwrap_or("<missing id>");
        let prefix = format!("location-templates.json / template {id} (index {index})");
        if !seen.insert(id.to_string()) { return Err(format!("{prefix}: duplicate template ID")); }
        if let Some(nodes) = value["nodes"].as_array() {
            for (i, node) in nodes.iter().enumerate() {
                let key = node["key"].as_str().unwrap_or("<missing key>");
                    for field in ["key", "typeId", "name", "description", "visualDescription"] {
                        if node.get(field).is_some_and(|value| !value.is_string()) {
                            return Err(format!("{prefix} / nodes[{key}] / {field}: expected string"));
                        }
                    }
                    if node.get("parent").is_some_and(|value| !value.is_null() && !value.is_string()) {
                        return Err(format!("{prefix} / nodes[{key}] / parent: expected string key or null"));
                    }
                    if node.get("components").is_some_and(|value| !value.is_object()) {
                        return Err(format!("{prefix} / nodes[{key}] / components: expected extension object"));
                    }
                if let Some(components) = node["components"].as_object() {
                    for (component, values) in components {
                        let fields = values.as_object().ok_or_else(|| format!("{prefix} / nodes[{key}] / components[{component}]: expected field object"))?;
                        for (field, value) in fields {
                            serde_json::from_value::<PropertyValue>(value.clone()).map_err(|e|
                                format!("{prefix} / nodes[{key}] / components[{component}].{field}: {e}"))?;
                        }
                    }
                }
                serde_json::from_value::<TemplateNode>(node.clone())
                    .map_err(|error| format!("{prefix} / nodes[{key}] (index {i}): {error}"))?;
            }
        }
        if let Some(edges) = value["edges"].as_array() {
            for (i, edge) in edges.iter().enumerate() {
                let key = edge["key"].as_str().unwrap_or("<missing key>");
                let path = format!("{prefix} / edges[{key}] (index {i})");
                if let Some(properties) = edge["properties"].as_object() {
                    for (field, value) in properties {
                        serde_json::from_value::<PropertyValue>(value.clone())
                            .map_err(|e| format!("{path} / properties.{field}: {e}"))?;
                    }
                }
                serde_json::from_value::<TemplateEdge>(edge.clone())
                    .map_err(|e| format!("{path}: {e}"))?;
            }
        }
        serde_json::from_value(value.clone()).map_err(|error| format!("{prefix}: {error}"))
    }).collect()
}

/// Assemble and validate a fresh editable world without touching project files.
pub fn location_template_world(template_id: &str) -> Result<World, String> {
    let template = catalogue()?
        .into_iter()
        .find(|t| t.id == template_id)
        .ok_or_else(|| format!("Location template {template_id:?} is unavailable."))?;
    assemble_template(&template)
}

fn assemble_template(template: &LocationTemplate) -> Result<World, String> {
    let prefix = format!("template {}", template.id);
    crate::validate_name(&template.id).map_err(|e| format!("{prefix} / id: {e}"))?;
    crate::validate_name(&template.name).map_err(|e| format!("{prefix} / name: {e}"))?;
    if template.description.trim().is_empty() {
        return Err(format!(
            "{prefix} / description: a location template needs a description."
        ));
    }
    let mut world = World::bundled()?;
    let mut ids = BTreeMap::new();
    for node in &template.nodes {
        if node.key.trim().is_empty()
            || ids
                .insert(node.key.clone(), Uuid::new_v4().to_string())
                .is_some()
        {
            return Err(format!(
                "{prefix} / nodes[{}]: keys must be nonempty and unique.",
                node.key
            ));
        }
    }
    for seed in &template.nodes {
        let path = format!("{prefix} / nodes[{}]", seed.key);
        crate::validate_name(&seed.name).map_err(|e| format!("{path} / name: {e}"))?;
        for (field, value) in [
            ("description", &seed.description),
            ("visualDescription", &seed.visual_description),
        ] {
            if value.chars().count() > 16000 {
                return Err(format!("{path} / {field}: exceeds 16,000 characters"));
            }
        }
        let preset = world
            .node_types
            .iter()
            .find(|p| p.id == seed.type_id)
            .ok_or_else(|| format!("{path} / typeId: unavailable preset {}", seed.type_id))?;
        let mut components = BTreeMap::new();
        for capability in preset.components.iter().chain(seed.components.keys()) {
            let definition = world
                .component_types
                .iter()
                .find(|c| &c.id == capability)
                .ok_or_else(|| {
                    format!("{path} / components[{capability}]: unavailable extension")
                })?;
            let mut values: BTreeMap<_, _> = definition
                .fields
                .iter()
                .filter(|f| f.required || f.default_value != PropertyValue::Null)
                .map(|f| (f.key.clone(), f.default_value.clone()))
                .collect();
            if let Some(overrides) = seed.components.get(capability) {
                values.extend(overrides.clone());
            }
            components.insert(capability.clone(), values);
        }
        let node = WorldNode {
            id: ids[&seed.key].clone(),
            type_id: seed.type_id.clone(),
            name: seed.name.clone(),
            description: seed.description.clone(),
            visual_description: seed.visual_description.clone(),
            components,
        };
        if let Some(parent) = &seed.parent {
            let target_id = ids
                .get(parent)
                .ok_or_else(|| format!("{path} / parent[{parent}]: unknown parent key"))?;
            // Behavior follows capabilities, never the preset's display name.
            let relation = if node.components.contains_key("core.place") {
                "core.inside"
            } else if node.components.contains_key("core.item") {
                "core.stored-in"
            } else {
                return Err(format!(
                    "{path} / parent[{parent}]: this node cannot have a physical parent."
                ));
            };
            world.edges.push(WorldEdge {
                id: Uuid::new_v4().to_string(),
                type_id: relation.into(),
                source_id: node.id.clone(),
                target_id: target_id.clone(),
                description: String::new(),
                properties: BTreeMap::new(),
            });
        }
        world.nodes.push(node);
    }
    // Validate overrides against the complete allocated index so forward node
    // references retain normal native validation rather than a partial schema.
    let index = world.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    for (seed, node) in template.nodes.iter().zip(&world.nodes) {
        for (component, values) in &node.components {
            let definition = world
                .component_types
                .iter()
                .find(|d| &d.id == component)
                .ok_or_else(|| {
                    format!(
                        "{prefix} / nodes[{}] / components[{component}]: unavailable extension",
                        seed.key
                    )
                })?;
            for (key, value) in values {
                let path = format!(
                    "{prefix} / nodes[{}] / components[{component}].{key}",
                    seed.key
                );
                let field = definition
                    .fields
                    .iter()
                    .find(|f| &f.key == key)
                    .ok_or_else(|| format!("{path}: undeclared field"))?;
                if !crate::model::field_matches(value, field, &index) {
                    return Err(format!(
                        "{path}: expected {:?}; received {:?}",
                        field.kind, value
                    ));
                }
            }
        }
    }
    let mut edge_keys = BTreeMap::new();
    let mut seen_edges = std::collections::BTreeSet::new();
    for seed in &template.edges {
        let path = format!("{prefix} / edges[{}] ({})", seed.key, seed.type_id);
        if seed.key.trim().is_empty() || !seen_edges.insert(&seed.key) {
            return Err(format!("{path}: keys must be nonempty and unique."));
        }
        let definition = world
            .relation_types
            .iter()
            .find(|r| r.id == seed.type_id)
            .ok_or_else(|| format!("{path} / typeId: unavailable relationship"))?;
        let resolve = |key: &str, field: &str, capability: &Option<String>| {
            let id = ids
                .get(key)
                .ok_or_else(|| format!("{path} / {field}[{key}]: unknown node key"))?;
            let node = index[id.as_str()];
            if let Some(required) = capability {
                if !crate::model::capability(node, required) {
                    return Err(format!(
                        "{path} / {field}[{key}]: requires component {required}"
                    ));
                }
            }
            Ok(id.clone())
        };
        let source_id = resolve(&seed.source_key, "sourceKey", &definition.source_capability)?;
        let target_id = resolve(&seed.target_key, "targetKey", &definition.target_capability)?;
        // Starter routes are authored explicitly. Saved-world validation remains
        // compatible with optional/absent direction fields in existing projects.
        if seed.type_id == "story.route-to"
            && !matches!(seed.properties.get("direction"),
            Some(PropertyValue::Text(value)) if value == "one-way" || value == "two-way")
        {
            return Err(format!("{path} / properties.direction: starter routes require an explicit travel direction"));
        }
        if source_id == target_id {
            return Err(format!(
                "{path} / targetKey[{}]: a node cannot link to itself",
                seed.target_key
            ));
        }
        if seed.description.chars().count() > 16000 {
            return Err(format!("{path} / description: exceeds 16,000 characters"));
        }
        if world.edges.iter().any(|e| {
            e.type_id == seed.type_id && e.source_id == source_id && e.target_id == target_id
        }) {
            return Err(format!(
                "{path}: duplicate connection from nodes[{}] to nodes[{}]",
                seed.source_key, seed.target_key
            ));
        }
        let mut properties: BTreeMap<_, _> = definition
            .fields
            .iter()
            .filter(|f| f.required || f.default_value != PropertyValue::Null)
            .map(|f| (f.key.clone(), f.default_value.clone()))
            .collect();
        properties.extend(seed.properties.clone());
        for (key, value) in &properties {
            let field = definition
                .fields
                .iter()
                .find(|f| &f.key == key)
                .ok_or_else(|| format!("{path} / properties.{key}: undeclared field"))?;
            if !crate::model::field_matches(value, field, &index) {
                return Err(format!(
                    "{path} / properties.{key}: expected {:?}; received {:?}",
                    field.kind, value
                ));
            }
        }
        let id = Uuid::new_v4().to_string();
        edge_keys.insert(id.clone(), seed.key.clone());
        world.edges.push(WorldEdge {
            id,
            type_id: seed.type_id.clone(),
            source_id,
            target_id,
            description: seed.description.clone(),
            properties,
        });
    }
    world.validate().map_err(|error| {
        let keys = template
            .nodes
            .iter()
            .zip(&world.nodes)
            .filter(|(_, node)| error.contains(&node.name) || error.contains(&node.id))
            .map(|(seed, node)| format!("{} ({}) = nodes[{}]", node.id, node.name, seed.key))
            .collect::<Vec<_>>()
            .join("; ");
        let edges = edge_keys
            .iter()
            .filter(|(id, _)| {
                world
                    .edges
                    .iter()
                    .find(|e| &e.id == *id)
                    .is_some_and(|edge| {
                        error.contains(id.as_str())
                            || [&edge.source_id, &edge.target_id].iter().any(|id| {
                                index
                                    .get(id.as_str())
                                    .is_some_and(|node| error.contains(&node.name))
                            })
                            || world
                                .relation_types
                                .iter()
                                .find(|r| r.id == edge.type_id)
                                .is_some_and(|r| error.contains(&r.name))
                    })
            })
            .map(|(id, key)| format!("{id} = edges[{key}]"))
            .collect::<Vec<_>>()
            .join("; ");
        format!("{prefix}: {error}\nTemplate node keys: {keys}\nTemplate edge keys: {edges}")
    })?;
    Ok(world)
}

/// Build/package preflight using the exact runtime structures and validators.
pub fn validate_bundled_world_data() -> Result<(), String> {
    let mut world = World::bundled()?;
    for preset in &world.node_types {
        world.nodes.push(WorldNode {
            id: Uuid::new_v4().to_string(),
            type_id: preset.id.clone(),
            name: preset.name.clone(),
            description: String::new(),
            visual_description: String::new(),
            components: preset
                .components
                .iter()
                .map(|id| {
                    let definition = world
                        .component_types
                        .iter()
                        .find(|d| &d.id == id)
                        .ok_or_else(|| {
                            format!("preset {} / component {id}: missing definition", preset.id)
                        })?;
                    Ok((id.clone(), crate::default_properties(definition)))
                })
                .collect::<Result<_, String>>()?,
        });
    }
    world
        .validate()
        .map_err(|e| format!("Bundled preset defaults: {e}"))?;
    for template in catalogue()? {
        assemble_template(&template)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
