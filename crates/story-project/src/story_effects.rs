use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use specta::Type;
use std::collections::{BTreeMap, BTreeSet};
mod diagnostics;
mod schema;
pub(crate) use diagnostics::effect_record_issues;

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
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProviderEffectGroups {
    put_nodes: Vec<PassageEffect>,
    put_links: Vec<PassageEffect>,
    remove_links: Vec<PassageEffect>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProviderEffects {
    changes: ProviderEffectGroups,
    notes: Vec<ContinuityNote>,
    suggest_chapter_end: bool,
}
/// Provider operation groups avoid a mixed-kind array grammar. They are only a
/// transport shape: manual/retained flat text and native review stay unchanged.
pub(crate) fn materialize_effects_text(text: &str) -> Result<String, String> {
    let value: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if value.get("changes").is_some_and(Value::is_array) {
        return Ok(text.to_owned());
    }
    let grouped: ProviderEffects = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let ProviderEffectGroups {
        put_nodes,
        put_links,
        remove_links,
    } = grouped.changes;
    if put_nodes
        .iter()
        .any(|effect| !matches!(effect, PassageEffect::PutNode { .. }))
    {
        return Err("changes.putNodes may contain only putNode effects.".into());
    }
    if put_links
        .iter()
        .any(|effect| !matches!(effect, PassageEffect::PutLink { .. }))
    {
        return Err("changes.putLinks may contain only putLink effects.".into());
    }
    if remove_links
        .iter()
        .any(|effect| !matches!(effect, PassageEffect::RemoveLink { .. }))
    {
        return Err("changes.removeLinks may contain only removeLink effects.".into());
    }
    let effects = PassageEffects {
        changes: put_nodes
            .into_iter()
            .chain(put_links)
            .chain(remove_links)
            .collect(),
        notes: grouped.notes,
        suggest_chapter_end: grouped.suggest_chapter_end,
    };
    serde_json::to_string(&effects).map_err(|e| e.to_string())
}
pub(crate) fn planning_only(field: &FieldDefinition) -> bool {
    !field.used_for(FieldUse::Story) && !field.used_for(FieldUse::Visual)
}
pub(crate) fn authoring_only_components(
    world: &World,
    ids: impl Iterator<Item = impl AsRef<str>>,
) -> bool {
    let ids: BTreeSet<String> = ids.map(|id| id.as_ref().to_string()).collect();
    let operational = world
        .relation_types
        .iter()
        .filter(|r| r.physical_placement)
        .any(|r| {
            [r.source_capability.as_ref(), r.target_capability.as_ref()]
                .into_iter()
                .flatten()
                .any(|c| ids.contains(c))
        });
    let fields: Vec<_> = world
        .component_types
        .iter()
        .filter(|d| ids.contains(&d.id))
        .flat_map(|d| &d.fields)
        .collect();
    !operational && !fields.is_empty() && fields.iter().all(|f| planning_only(f))
}
/// The canonical schema stays complete; prose only receives creation definitions
/// for story/appearance records and declarations needed by supplied existing nodes.
pub(crate) fn prose_effect_schema(world: &World) -> World {
    let mut scoped = world.clone();
    scoped.node_types.retain(|t| {
        world.nodes.iter().any(|n| n.type_id == t.id)
            || !authoring_only_components(world, t.components.iter())
    });
    let mut needed: BTreeSet<_> = scoped
        .node_types
        .iter()
        .flat_map(|t| t.components.iter().cloned())
        .chain(
            world
                .nodes
                .iter()
                .flat_map(|n| n.components.keys().cloned()),
        )
        .collect();
    needed.extend(
        world
            .component_types
            .iter()
            .filter(|c| c.fields.is_empty() || c.fields.iter().any(|f| !planning_only(f)))
            .map(|c| c.id.clone()),
    );
    scoped.component_types.retain(|c| needed.contains(&c.id));
    scoped.relation_types.retain(|r| {
        [r.source_capability.as_ref(), r.target_capability.as_ref()]
            .into_iter()
            .flatten()
            .all(|id| needed.contains(id))
    });
    scoped
}
fn alias(id: &str) -> bool {
    id.strip_prefix("new:").is_some_and(|s| {
        !s.is_empty()
            && s.len() <= 60
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    })
}
fn evidence(normalized_prose: &str, quote: &str) -> Result<(), String> {
    let quote = quote.split_whitespace().collect::<Vec<_>>().join(" ");
    let count = quote.chars().count();
    if !(8..=600).contains(&count) {
        return Err(format!("quote has {count} characters; expected 8–600."));
    }
    if !normalized_prose.contains(&quote) {
        return Err(
            "quote does not appear as one continuous excerpt in the current passage.".into(),
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
    // Report every bad quote before allocating IDs or calculating world changes.
    let normalized_prose = prose.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut quote_errors = vec![];
    for (index, change) in effects.changes.iter().enumerate() {
        let quote = match change {
            PassageEffect::PutNode { evidence, .. }
            | PassageEffect::PutLink { evidence, .. }
            | PassageEffect::RemoveLink { evidence, .. } => evidence,
        };
        if let Err(reason) = evidence(&normalized_prose, quote) {
            quote_errors.push(format!("changes[{index}].evidence: {reason}"));
        }
    }
    for (index, note) in effects.notes.iter().enumerate() {
        if let Err(reason) = evidence(&normalized_prose, &note.evidence) {
            quote_errors.push(format!("notes[{index}].evidence: {reason}"));
        }
    }
    if !quote_errors.is_empty() {
        return Err(format!("Evidence quotes need correction:\n{}\nCopy one continuous excerpt per field, keeping its words, capitalization and punctuation. Do not insert ... or … to skip text or join separate excerpts. Whitespace differences are allowed. Edit these fields and validate again, or request a new world-change check; your passage is retained.", quote_errors.join("\n")));
    }
    let record_errors = effect_record_issues(source, context, effects)
        .into_iter()
        .enumerate()
        .flat_map(|(i, issues)| {
            issues
                .into_iter()
                .map(move |issue| format!("changes[{i}]: {issue}"))
        })
        .collect::<Vec<_>>();
    if !record_errors.is_empty() {
        return Err(format!(
            "Proposed changes need correction:\n{}",
            record_errors.join("\n")
        ));
    }
    let scoped: BTreeSet<_> = context.nodes.iter().map(|n| n.id.clone()).collect();
    let mut allowed = scoped.clone();
    let mut seen = BTreeSet::new();
    let mut new_count = 0;
    for change in &effects.changes {
        let id = match change {
            PassageEffect::PutNode { node, .. } => node.id.as_str(),
            PassageEffect::PutLink { link, .. } => link.id.as_str(),
            PassageEffect::RemoveLink { id, .. } => id.as_str(),
        };
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
            world.edges.retain(|e| e.id != *id);
        }
    }
    let physical: BTreeSet<_> = source
        .relation_types
        .iter()
        .filter(|r| r.physical_placement)
        .map(|r| r.id.clone())
        .collect();
    let mut placed = BTreeSet::new();
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
            crate::projection::resolve_property_references(
                &definition.fields,
                &mut link.properties,
                source
                    .edges
                    .iter()
                    .find(|e| e.id == link.id)
                    .map(|e| &e.properties),
                &allowed,
                &resolve,
            )?;
            link.id = resolve(&link.id);
            link.source_id = resolve(&link.source_id);
            link.target_id = resolve(&link.target_id);
            world.edges.retain(|e| {
                e.id != link.id
                    && !(e.source_id == link.source_id
                        && ((definition.physical_placement && physical.contains(&e.type_id))
                            || (definition.single_target && e.type_id == link.type_id)))
            });
            if definition.physical_placement && !placed.insert(link.source_id.clone()) {
                return Err("An effect proposes multiple physical placements for one node. Keep only its final worn, carried or stored placement.".into());
            }
            world.edges.push(link);
        }
    }
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
    }
    world.validate()?;
    Ok((world, labels))
}
pub(crate) fn effects_schema(world: &World) -> Value {
    schema::effects_schema(world)
}
pub(crate) fn effects_capabilities(world: &World) -> Value {
    schema::capabilities(world)
}
pub(crate) fn effects_relationships(world: &World) -> Value {
    schema::relationships(world)
}
