use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use specta::Type;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PassageEffect {
    PutNode { node: WorldNode, evidence: String },
    PutLink { link: WorldEdge, evidence: String },
    RemoveLink { id: String, evidence: String },
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PassageEffects {
    pub changes: Vec<PassageEffect>,
    pub notes: Vec<ContinuityNote>,
    pub suggest_chapter_end: bool,
}
fn alias(id: &str) -> bool {
    id.strip_prefix("new:").is_some_and(|s| {
        !s.is_empty()
            && s.len() <= 60
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    })
}
fn evidence(prose: &str, quote: &str) -> Result<(), String> {
    let quote = quote.split_whitespace().collect::<Vec<_>>().join(" ");
    let prose = prose.split_whitespace().collect::<Vec<_>>().join(" ");
    if quote.chars().count() < 8 || quote.chars().count() > 600 || !prose.contains(&quote) {
        return Err(
            "Each effect/note needs an exact 8–600 character evidence quote from this prose."
                .into(),
        );
    }
    Ok(())
}
fn link_label(world: &World, edge: &WorldEdge) -> String {
    let name = |id: &str| {
        world
            .nodes
            .iter()
            .find(|n| n.id == id)
            .map_or("Unknown node", |n| n.name.as_str())
    };
    let relation = world
        .relation_types
        .iter()
        .find(|r| r.id == edge.type_id)
        .map_or(edge.type_id.as_str(), |r| r.label.as_str());
    format!(
        "{} → {} → {}",
        name(&edge.source_id),
        relation,
        name(&edge.target_id)
    )
}
pub(crate) fn preview_passage_effects(
    source: &World,
    context: &World,
    prose: &str,
    effects: &PassageEffects,
    allocations: &mut BTreeMap<String, String>,
) -> Result<(World, Vec<String>), String> {
    if effects.changes.len() > 80 || effects.notes.len() > 12 {
        return Err("Too many proposed effects or continuity notes.".into());
    }
    let scoped: BTreeSet<_> = context.nodes.iter().map(|n| n.id.clone()).collect();
    let mut allowed = scoped.clone();
    let mut seen = BTreeSet::new();
    let mut new_count = 0;
    for change in &effects.changes {
        let (id, quote) = match change {
            PassageEffect::PutNode { node, evidence } => (node.id.as_str(), evidence),
            PassageEffect::PutLink { link, evidence } => (link.id.as_str(), evidence),
            PassageEffect::RemoveLink { id, evidence } => (id.as_str(), evidence),
        };
        evidence(prose, quote)?;
        if !seen.insert(id.to_string()) {
            return Err("Duplicate effect record ID; combine its changes in one effect.".into());
        }
        if let PassageEffect::PutNode { node, .. } = change {
            if alias(&node.id) {
                new_count += 1;
                allowed.insert(node.id.clone());
                allocations
                    .entry(node.id.clone())
                    .or_insert_with(|| uuid::Uuid::new_v4().to_string());
            } else {
                validate_id(&node.id)?;
                if !scoped.contains(&node.id) {
                    return Err("A node effect is outside the selected scene context.".into());
                }
            }
        }
        if let PassageEffect::PutLink { link, .. } = change {
            if alias(&link.id) {
                allocations
                    .entry(link.id.clone())
                    .or_insert_with(|| uuid::Uuid::new_v4().to_string());
            } else {
                validate_id(&link.id)?;
                let old = source
                    .edges
                    .iter()
                    .find(|e| e.id == link.id)
                    .ok_or("Existing link effect ID is missing.")?;
                if !scoped.contains(&old.source_id) || !scoped.contains(&old.target_id) {
                    return Err("A link effect is outside the selected scene context.".into());
                }
            }
        }
    }
    if new_count > 20 {
        return Err("A passage may introduce at most 20 new nodes.".into());
    }
    let aliases = allocations.clone();
    let resolve = |id: &str| aliases.get(id).cloned().unwrap_or_else(|| id.into());
    let mut world = source.clone();
    let mut labels = vec![];
    for change in &effects.changes {
        if let PassageEffect::PutNode { node, evidence } = change {
            let mut node = node.clone();
            let old = source.nodes.iter().find(|n| n.id == node.id);
            let preset = source
                .node_types
                .iter()
                .find(|t| t.id == node.type_id)
                .ok_or("Unknown node type in effects.")?;
            if let Some(old) = old {
                if old.type_id != node.type_id
                    || old.components.keys().collect::<Vec<_>>()
                        != node.components.keys().collect::<Vec<_>>()
                {
                    return Err(
                        "Passage effects cannot change an existing node's type/capability set."
                            .into(),
                    );
                }
            } else if preset
                .components
                .iter()
                .any(|c| !node.components.contains_key(c))
            {
                return Err("A newly introduced node is missing a preset capability.".into());
            }
            for (component, values) in &mut node.components {
                let definition = source
                    .component_types
                    .iter()
                    .find(|d| d.id == *component)
                    .ok_or("Unknown effect capability.")?;
                for field in &definition.fields {
                    if let Some(value) = values.get_mut(&field.key) {
                        let previous = old
                            .and_then(|n| n.components.get(component))
                            .and_then(|v| v.get(&field.key));
                        match (&field.kind, &mut *value) {
                            (FieldKind::NodeRef, PropertyValue::Text(id)) => {
                                if previous != Some(&PropertyValue::Text(id.clone()))
                                    && !allowed.contains(id)
                                {
                                    return Err(
                                        "An effect reference is outside the supplied context."
                                            .into(),
                                    );
                                }
                                *id = resolve(id);
                            }
                            (FieldKind::NodeRefs, PropertyValue::NodeRefs(ids)) => {
                                if previous != Some(&PropertyValue::NodeRefs(ids.clone()))
                                    && ids.iter().any(|id| !allowed.contains(id))
                                {
                                    return Err(
                                        "An effect reference is outside the supplied context."
                                            .into(),
                                    );
                                }
                                for id in ids {
                                    *id = resolve(id);
                                }
                            }
                            _ => (),
                        }
                    }
                }
            }
            node.id = resolve(&node.id);
            if let Some(existing) = world.nodes.iter_mut().find(|n| n.id == node.id) {
                if existing != &node {
                    labels.push(format!("Update {} — “{}”", node.name, evidence));
                }
                *existing = node;
            } else {
                labels.push(format!("Introduce {} — “{}”", node.name, evidence));
                world.nodes.push(node);
            }
        }
    }
    let mut detached = BTreeSet::new();
    for change in &effects.changes {
        if let PassageEffect::RemoveLink { id, .. } = change {
            let old = source
                .edges
                .iter()
                .find(|e| &e.id == id)
                .ok_or("Removed link is missing.")?;
            if !scoped.contains(&old.source_id) || !scoped.contains(&old.target_id) {
                return Err("Removed link is outside the supplied context.".into());
            }
            if source
                .relation_types
                .iter()
                .any(|r| r.id == old.type_id && r.physical_placement)
            {
                detached.insert(old.source_id.clone());
            }
            world.edges.retain(|e| e.id != *id);
        }
    }
    let physical: BTreeSet<_> = source
        .relation_types
        .iter()
        .filter(|r| r.physical_placement)
        .map(|r| r.id.clone())
        .collect();
    let mut moved = detached;
    for change in &effects.changes {
        if let PassageEffect::PutLink { link, .. } = change {
            if !allowed.contains(&link.source_id) || !allowed.contains(&link.target_id) {
                return Err("Effect link endpoints are outside the supplied context.".into());
            }
            let definition = source
                .relation_types
                .iter()
                .find(|d| d.id == link.type_id)
                .ok_or("Unknown effect relationship.")?;
            let mut link = link.clone();
            link.id = resolve(&link.id);
            link.source_id = resolve(&link.source_id);
            link.target_id = resolve(&link.target_id);
            if let Some(old) = world
                .edges
                .iter()
                .find(|e| e.id == link.id && physical.contains(&e.type_id))
            {
                moved.insert(old.source_id.clone());
            }
            world.edges.retain(|e| {
                e.id != link.id
                    && !(e.source_id == link.source_id
                        && ((definition.physical_placement && physical.contains(&e.type_id))
                            || (definition.single_target && e.type_id == link.type_id)))
            });
            if definition.physical_placement {
                moved.insert(link.source_id.clone());
            }
            if link.type_id == "core.worn-by"
                && !world.edges.iter().any(|e| {
                    e.type_id == "core.stored-in"
                        && e.source_id == link.source_id
                        && e.target_id == link.target_id
                })
            {
                let prior = world
                    .edges
                    .iter()
                    .find(|e| physical.contains(&e.type_id) && e.source_id == link.source_id)
                    .map(|e| e.id.clone());
                world
                    .edges
                    .retain(|e| !(e.source_id == link.source_id && physical.contains(&e.type_id)));
                let key = format!("auto-held-{}", link.source_id);
                let id = prior.unwrap_or_else(|| {
                    allocations
                        .entry(key)
                        .or_insert_with(|| uuid::Uuid::new_v4().to_string())
                        .clone()
                });
                world.edges.push(WorldEdge {
                    id,
                    type_id: "core.stored-in".into(),
                    source_id: link.source_id.clone(),
                    target_id: link.target_id.clone(),
                    description: String::new(),
                });
                moved.insert(link.source_id.clone());
            }
            world.edges.push(link);
        }
    }
    let held: BTreeSet<_> = world
        .edges
        .iter()
        .filter(|e| e.type_id == "core.stored-in")
        .map(|e| (e.source_id.clone(), e.target_id.clone()))
        .collect();
    world.edges.retain(|e| {
        e.type_id != "core.worn-by"
            || !moved.contains(&e.source_id)
            || held.contains(&(e.source_id.clone(), e.target_id.clone()))
    });
    for edge in &source.edges {
        if !world.edges.iter().any(|e| e.id == edge.id) {
            labels.push(format!("Remove {}", link_label(source, edge)));
        }
    }
    for edge in &world.edges {
        if !source.edges.iter().any(|e| e == edge) {
            labels.push(format!("Set {}", link_label(&world, edge)));
        }
    }
    for note in &effects.notes {
        if note.text.trim().is_empty()
            || note.text.len() > 2000
            || note.node_ids.len() > 20
            || note.node_ids.iter().any(|id| !allowed.contains(id))
        {
            return Err("Invalid continuity note or out-of-context subject.".into());
        }
        evidence(prose, &note.evidence)?;
    }
    world.validate()?;
    Ok((world, labels))
}
pub(crate) fn effects_schema() -> Value {
    let node = json!({"type":"object","properties":{"id":{"type":"string"},"typeId":{"type":"string"},"name":{"type":"string"},"description":{"type":"string"},"visualDescription":{"type":"string"},"components":{"type":"object"}},"required":["id","typeId","name","description","visualDescription","components"],"additionalProperties":false});
    let link = json!({"type":"object","properties":{"id":{"type":"string"},"typeId":{"type":"string"},"sourceId":{"type":"string"},"targetId":{"type":"string"},"description":{"type":"string"}},"required":["id","typeId","sourceId","targetId","description"],"additionalProperties":false});
    json!({"type":"object","properties":{"changes":{"type":"array","maxItems":80,"items":{"anyOf":[
        {"type":"object","properties":{"kind":{"const":"putNode"},"node":node,"evidence":{"type":"string"}},"required":["kind","node","evidence"],"additionalProperties":false},
        {"type":"object","properties":{"kind":{"const":"putLink"},"link":link,"evidence":{"type":"string"}},"required":["kind","link","evidence"],"additionalProperties":false},
        {"type":"object","properties":{"kind":{"const":"removeLink"},"id":{"type":"string"},"evidence":{"type":"string"}},"required":["kind","id","evidence"],"additionalProperties":false}]}},"notes":{"type":"array","maxItems":12,"items":{"type":"object","properties":{"text":{"type":"string"},"evidence":{"type":"string"},"nodeIds":{"type":"array","items":{"type":"string"}}},"required":["text","evidence","nodeIds"],"additionalProperties":false}},"suggestChapterEnd":{"type":"boolean"}},"required":["changes","notes","suggestChapterEnd"],"additionalProperties":false})
}
