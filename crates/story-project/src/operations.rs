//! Pure structural edits: preview a complete valid result before publishing a revision.
use crate::{FieldKind, PropertyValue, World, WorldEdge, WorldNode};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum WorldEdit {
    PutLink {
        link: WorldEdge,
    },
    RemoveLink {
        id: String,
    },
    RemoveNode {
        id: String,
    },
    RemoveSelection {
        node_ids: Vec<String>,
        edge_ids: Vec<String>,
    },
    PasteNodes {
        nodes: Vec<WorldNode>,
        edges: Vec<WorldEdge>,
    },
    InsertLibrary {
        entry: crate::LibraryEntry,
    },
    RemoveCapability {
        id: String,
        capability: String,
    },
    AddCapability {
        id: String,
        capability: String,
    },
    RetypeNode {
        id: String,
        type_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorldEditPreview {
    pub world: World,
    pub changes: Vec<String>,
}

fn physical(world: &World, type_id: &str) -> bool {
    world
        .relation_types
        .iter()
        .any(|d| d.id == type_id && d.physical_placement)
}

fn label(world: &World, edge: &WorldEdge) -> String {
    let name = |id: &str| {
        world
            .nodes
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.name.as_str())
            .unwrap_or("Unknown node")
    };
    let relation = world
        .relation_types
        .iter()
        .find(|d| d.id == edge.type_id)
        .map(|d| d.label.as_str())
        .unwrap_or(&edge.type_id);
    format!(
        "{} → {} → {}",
        name(&edge.source_id),
        relation,
        name(&edge.target_id)
    )
}

/// Never mutates the source, and never returns a partially valid edit.
pub fn preview_world_edit(source: &World, edit: WorldEdit) -> Result<WorldEditPreview, String> {
    source
        .validate()
        .map_err(|e| format!("Finish the current world edits first: {e}"))?;
    let mut world = source.clone();
    let mut changes = Vec::new();
    match edit {
        WorldEdit::AddCapability { id, capability } => {
            let definition = world
                .component_types
                .iter()
                .find(|d| d.id == capability)
                .ok_or("Choose a saved extension.")?;
            let node = world
                .nodes
                .iter_mut()
                .find(|n| n.id == id)
                .ok_or("The node no longer exists.")?;
            if node.components.contains_key(&capability) {
                return Err("This extension is already attached.".into());
            }
            node.components
                .insert(capability, crate::default_properties(definition));
            changes.push(format!("Add {} to {} with its declared defaults; other fields and connections stay intact.", definition.name, node.name));
        }
        WorldEdit::RetypeNode { id, type_id } => {
            let preset = world
                .node_types
                .iter()
                .find(|d| d.id == type_id)
                .ok_or("Choose a saved preset.")?;
            let node = world
                .nodes
                .iter_mut()
                .find(|n| n.id == id)
                .ok_or("The node no longer exists.")?;
            if node.type_id != type_id {
                changes.push(format!(
                    "Use the {} preset for {}. Existing extensions and values are retained.",
                    preset.name, node.name
                ));
                node.type_id = type_id;
            }
            for id in &preset.components {
                if !node.components.contains_key(id) {
                    let definition = world
                        .component_types
                        .iter()
                        .find(|d| &d.id == id)
                        .ok_or("Missing preset extension.")?;
                    node.components
                        .insert(id.clone(), crate::default_properties(definition));
                    changes.push(format!(
                        "Add {} with its declared defaults.",
                        definition.name
                    ));
                }
            }
        }
        WorldEdit::PutLink { link } => {
            crate::validate_id(&link.id)?;
            let definition = world
                .relation_types
                .iter()
                .find(|d| d.id == link.type_id)
                .ok_or("Unknown relationship type.")?
                .clone();
            world.edges.retain(|e| e.id != link.id);
            if definition.physical_placement || definition.single_target {
                let physical_types: BTreeSet<_> = world
                    .relation_types
                    .iter()
                    .filter(|d| d.physical_placement)
                    .map(|d| d.id.clone())
                    .collect();
                world.edges.retain(|e| {
                    !(e.source_id == link.source_id
                        && ((definition.physical_placement && physical_types.contains(&e.type_id))
                            || (definition.single_target && e.type_id == link.type_id)))
                });
            }
            world.edges.push(link);
        }
        WorldEdit::RemoveLink { id } => {
            if !world.edges.iter().any(|e| e.id == id) {
                return Err("The connection no longer exists.".into());
            }
            world.edges.retain(|e| e.id != id);
        }
        WorldEdit::RemoveNode { id } => remove_nodes(&mut world, &mut changes, vec![id])?,
        WorldEdit::RemoveSelection { node_ids, edge_ids } => {
            if node_ids.is_empty() && edge_ids.is_empty() {
                return Err("Select nodes or connections to delete.".into());
            }
            let edges: BTreeSet<_> = edge_ids.iter().collect();
            if edges.len() != edge_ids.len()
                || edge_ids
                    .iter()
                    .any(|id| !source.edges.iter().any(|e| &e.id == id))
            {
                return Err("The selection contains duplicate or unavailable connections.".into());
            }
            remove_nodes(&mut world, &mut changes, node_ids)?;
            world.edges.retain(|e| !edges.contains(&e.id));
        }
        WorldEdit::PasteNodes { nodes, edges } => {
            paste_nodes(&mut world, &mut changes, nodes, edges, true)?;
        }
        WorldEdit::InsertLibrary { entry } => {
            crate::node_library::insert_library(&mut world, &mut changes, entry)?;
        }
        WorldEdit::RemoveCapability { id, capability } => {
            let node = world
                .nodes
                .iter_mut()
                .find(|n| n.id == id)
                .ok_or("The node no longer exists.")?;
            if node.components.remove(&capability).is_none() {
                return Err("The node does not have that capability.".into());
            }
            let name = world
                .component_types
                .iter()
                .find(|d| d.id == capability)
                .map(|d| d.name.as_str())
                .unwrap_or(&capability);
            changes.push(format!(
                "Remove {name} and its field values from {}.",
                node.name
            ));
        }
    }
    crate::sources::ensure_manual_event_sources(&mut world);
    world
        .validate()
        .map_err(|e| format!("Cannot apply this change: {e}"))?;
    for edge in &source.edges {
        match world.edges.iter().find(|e| e.id == edge.id) {
            None => changes.push(format!("Remove connection: {}.", label(source, edge))),
            Some(next)
                if next.source_id == edge.source_id
                    && next.target_id == edge.target_id
                    && next.type_id == edge.type_id
                    && next.description != edge.description =>
            {
                changes.push(format!("Update connection notes: {}.", label(&world, next)))
            }
            Some(next) if next != edge => changes.push(format!(
                "Change connection: {} → {}.",
                label(source, edge),
                label(&world, next)
            )),
            _ => (),
        }
    }
    for edge in &world.edges {
        let old = source.edges.iter().find(|e| e.id == edge.id);
        let fields = world
            .relation_types
            .iter()
            .find(|d| d.id == edge.type_id)
            .map(|d| d.fields.as_slice())
            .unwrap_or_default();
        let previous_fields = old
            .and_then(|e| source.relation_types.iter().find(|d| d.id == e.type_id))
            .map(|d| d.fields.as_slice())
            .unwrap_or_default();
        let keys: BTreeSet<_> = edge
            .properties
            .keys()
            .chain(old.into_iter().flat_map(|e| e.properties.keys()))
            .collect();
        for key in keys {
            let before_field = previous_fields.iter().find(|f| &f.key == key);
            let after_field = fields.iter().find(|f| &f.key == key);
            let field = after_field
                .or(before_field)
                .expect("validated relationship field");
            let before = old.and_then(|e| e.properties.get(key));
            let after = edge.properties.get(key);
            if before.unwrap_or(&PropertyValue::Null) != after.unwrap_or(&PropertyValue::Null) {
                changes.push(format!(
                    "{} / {}: {} → {}.",
                    label(&world, edge),
                    field.label,
                    crate::projection::field_value_text(
                        source,
                        before_field.unwrap_or(field),
                        before
                    ),
                    crate::projection::field_value_text(
                        &world,
                        after_field.unwrap_or(field),
                        after
                    )
                ));
            }
        }
        if !source.edges.iter().any(|e| e.id == edge.id) {
            changes.push(format!("Add connection: {}.", label(&world, edge)));
        }
    }
    Ok(WorldEditPreview { world, changes })
}

fn remove_nodes(
    world: &mut World,
    changes: &mut Vec<String>,
    ids: Vec<String>,
) -> Result<(), String> {
    let removed: BTreeSet<_> = ids.iter().cloned().collect();
    if removed.len() != ids.len()
        || ids
            .iter()
            .any(|id| !world.nodes.iter().any(|n| &n.id == id))
    {
        return Err("The selection contains duplicate or unavailable nodes.".into());
    }
    for id in ids {
        let node = world
            .nodes
            .iter()
            .find(|n| n.id == id)
            .ok_or("The node no longer exists.")?;
        changes.push(format!(
            "Remove node: {}. Its previous saved versions remain unchanged.",
            node.name
        ));
        for edge in &world.edges {
            if edge.target_id == id
                && !removed.contains(&edge.source_id)
                && physical(world, &edge.type_id)
            {
                let child = &world
                    .nodes
                    .iter()
                    .find(|n| n.id == edge.source_id)
                    .unwrap()
                    .name;
                changes.push(format!(
                    "Keep {child} as an unplaced node, with its own contents intact."
                ));
            }
        }
        world.nodes.retain(|n| n.id != id);
        if world.ai_context.iter().any(|b| b.node_id == id) {
            changes.push("Remove this node's story-wide AI context inclusion.".into());
        }
        world.ai_context.retain(|b| b.node_id != id);
        world.source_annotations.retain(|a| a.node_id != id);
        world
            .edges
            .retain(|e| e.source_id != id && e.target_id != id);
        // Clear only typed references, never matching words in free text.
        for node in &mut world.nodes {
            for (component_id, values) in &mut node.components {
                let definition = world
                    .component_types
                    .iter()
                    .find(|d| &d.id == component_id)
                    .unwrap();
                for field in &definition.fields {
                    if let Some(value) = values.get_mut(&field.key) {
                        if clear_reference(value, &field.kind, &id) {
                            changes.push(format!(
                                "Clear reference in {} / {} / {}.",
                                node.name, definition.name, field.label
                            ));
                        }
                    }
                }
            }
        }
        for definition in &mut world.component_types {
            for field in &mut definition.fields {
                if clear_reference(&mut field.default_value, &field.kind, &id) {
                    changes.push(format!(
                        "Clear default reference in {} / {}.",
                        definition.name, field.label
                    ));
                }
            }
        }
        for edge in &mut world.edges {
            let definition = world
                .relation_types
                .iter()
                .find(|d| d.id == edge.type_id)
                .unwrap();
            for field in &definition.fields {
                if let Some(value) = edge.properties.get_mut(&field.key) {
                    if clear_reference(value, &field.kind, &id) {
                        changes.push(format!(
                            "Clear connection reference: {} / {}.",
                            definition.name, field.label
                        ));
                    }
                }
            }
        }
        for definition in &mut world.relation_types {
            for field in &mut definition.fields {
                if clear_reference(&mut field.default_value, &field.kind, &id) {
                    changes.push(format!(
                        "Clear default reference in {} / {}.",
                        definition.name, field.label
                    ));
                }
            }
        }
    }
    Ok(())
}

// Paste is a new, self-contained group. Only typed references inside the copy
// are remapped; prose, original records, citations and AI bindings are not copied.
pub(crate) fn paste_nodes(
    world: &mut World,
    changes: &mut Vec<String>,
    mut nodes: Vec<WorldNode>,
    mut edges: Vec<WorldEdge>,
    rename: bool,
) -> Result<BTreeMap<String, String>, String> {
    if nodes.is_empty()
        || world.nodes.len() + nodes.len() > 5000
        || world.edges.len() + edges.len() > 20000
    {
        return Err("Choose a nonempty group within the world's node/connection limits.".into());
    }
    let mut mapping = BTreeMap::new();
    for node in &nodes {
        crate::validate_id(&node.id)?;
        crate::validate_name(&node.name)?;
        if mapping
            .insert(node.id.clone(), uuid::Uuid::new_v4().to_string())
            .is_some()
        {
            return Err("The copied group contains duplicate node IDs.".into());
        }
    }
    let mut edge_ids = BTreeSet::new();
    for edge in &edges {
        crate::validate_id(&edge.id)?;
        if !edge_ids.insert(&edge.id)
            || !mapping.contains_key(&edge.source_id)
            || !mapping.contains_key(&edge.target_id)
        {
            return Err(
                "Copied connections must be unique and stay within the copied group.".into(),
            );
        }
    }
    for node in &mut nodes {
        node.id = mapping[&node.id].clone();
        // Keep names recognizable, within the native name bound.
        if rename {
            node.name = format!("{} (copy)", node.name.chars().take(190).collect::<String>());
        }
        for (id, values) in &mut node.components {
            let definition = world
                .component_types
                .iter()
                .find(|d| &d.id == id)
                .ok_or("A copied extension is no longer defined.")?;
            for field in &definition.fields {
                if let Some(value) = values.get_mut(&field.key) {
                    remap_reference(value, &field.kind, &mapping);
                }
            }
        }
        changes.push(format!("Create node: {}.", node.name));
    }
    for edge in &mut edges {
        edge.id = uuid::Uuid::new_v4().to_string();
        edge.source_id = mapping[&edge.source_id].clone();
        edge.target_id = mapping[&edge.target_id].clone();
        let definition = world
            .relation_types
            .iter()
            .find(|d| d.id == edge.type_id)
            .ok_or("A copied relationship is no longer defined.")?;
        for field in &definition.fields {
            if let Some(value) = edge.properties.get_mut(&field.key) {
                remap_reference(value, &field.kind, &mapping);
            }
        }
    }
    world.nodes.extend(nodes);
    world.edges.extend(edges);
    Ok(mapping)
}

pub(crate) fn remap_reference(
    value: &mut PropertyValue,
    kind: &FieldKind,
    mapping: &BTreeMap<String, String>,
) {
    match (kind, &mut *value) {
        (FieldKind::NodeRef, PropertyValue::Text(id)) => {
            *value = mapping
                .get(id)
                .cloned()
                .map(PropertyValue::Text)
                .unwrap_or(PropertyValue::Null);
        }
        (FieldKind::NodeRefs, PropertyValue::NodeRefs(ids)) => {
            *ids = ids
                .iter()
                .filter_map(|id| mapping.get(id).cloned())
                .collect();
        }
        _ => (),
    }
}

fn clear_reference(value: &mut PropertyValue, kind: &FieldKind, id: &str) -> bool {
    match (kind, &mut *value) {
        (FieldKind::NodeRef, PropertyValue::Text(target)) if target == id => {
            *value = PropertyValue::Null;
            true
        }
        (FieldKind::NodeRefs, PropertyValue::NodeRefs(targets)) => {
            let before = targets.len();
            targets.retain(|target| target != id);
            before != targets.len()
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests;
