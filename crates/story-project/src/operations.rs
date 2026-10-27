//! Pure structural edits: preview a complete valid result before publishing a revision.
use crate::{FieldKind, PropertyValue, World, WorldEdge};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum WorldEdit {
    PutLink { link: WorldEdge },
    RemoveLink { id: String },
    RemoveNode { id: String },
    RemoveCapability { id: String, capability: String },
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
        WorldEdit::RemoveNode { id } => {
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
                if edge.target_id == id && physical(&world, &edge.type_id) {
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
        if !source.edges.iter().any(|e| e.id == edge.id) {
            changes.push(format!("Add connection: {}.", label(&world, edge)));
        }
    }
    Ok(WorldEditPreview { world, changes })
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
