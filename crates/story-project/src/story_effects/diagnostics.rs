use super::*;

// Shared by native validation and the review cards. Never infer a missing ID's
// intended identity or repair a proposal without an explicit author edit.
pub(crate) fn effect_record_issues(
    source: &World,
    context: &World,
    effects: &PassageEffects,
) -> Vec<Vec<String>> {
    if effects.changes.len() > 80 {
        return vec![];
    }
    let mut issues = vec![vec![]; effects.changes.len()];
    let scoped: BTreeSet<_> = context.nodes.iter().map(|n| n.id.as_str()).collect();
    let mut allowed = scoped.clone();
    let mut nodes: BTreeMap<_, _> = source.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    for change in &effects.changes {
        if let PassageEffect::PutNode { node, .. } = change {
            if alias(&node.id) {
                allowed.insert(&node.id);
            }
            nodes.insert(&node.id, node);
        }
    }
    let mut records: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    let mut placements: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    let mut single_targets: BTreeMap<(&str, &str), Vec<usize>> = BTreeMap::new();
    for (i, change) in effects.changes.iter().enumerate() {
        let record_id = match change {
            PassageEffect::PutNode { node, .. } => node.id.as_str(),
            PassageEffect::PutLink { link, .. } => link.id.as_str(),
            PassageEffect::RemoveLink { id, .. } => id.as_str(),
        };
        records.entry(record_id).or_default().push(i);
        match change {
            PassageEffect::PutNode { node, evidence } => {
                let old = source.nodes.iter().find(|n| n.id == node.id);
                if (authoring_only_components(source, node.components.keys())
                    || old.is_some_and(|n| authoring_only_components(source, n.components.keys())))
                    && old.is_none_or(|n| n != node)
                {
                    issues[i].push("Author-only presentation/planning profiles are edited through world tools, not passage effects. Remove this proposal; the prose is retained.".into());
                }
                for def in source.component_types.iter().filter(|d| {
                    node.components.contains_key(&d.id)
                        || old.is_some_and(|n| n.components.contains_key(&d.id))
                }) {
                    for field in def.fields.iter().filter(|f| !planning_only(f)) {
                        if old
                            .and_then(|n| n.components.get(&def.id))
                            .and_then(|values| values.get(&field.key))
                            .is_some_and(populated)
                            && node
                                .components
                                .get(&def.id)
                                .is_none_or(|values| !values.contains_key(&field.key))
                        {
                            issues[i].push(format!("{} / {} was omitted from this complete record. Restore its saved value in Edit, or explicitly clear it with passage evidence if the fact ended. Unrelated facts must be preserved.", def.name, field.label));
                        }
                    }
                    for field in def.fields.iter().filter(|f| planning_only(f)) {
                        let before = old
                            .and_then(|n| n.components.get(&def.id))
                            .and_then(|v| v.get(&field.key))
                            .unwrap_or(&PropertyValue::Null);
                        let after = node
                            .components
                            .get(&def.id)
                            .and_then(|v| v.get(&field.key))
                            .unwrap_or(&PropertyValue::Null);
                        let changed = if old.is_some() {
                            before != after
                        } else {
                            after != &PropertyValue::Null && after != &field.default_value
                        };
                        if changed {
                            issues[i].push(format!("{} / {} is author planning guidance. Preserve its saved/default value or remove this proposal; use world tools to revise it.",def.name,field.label));
                        }
                    }
                }
                let event_citations: usize = effects
                    .changes
                    .iter()
                    .map(|e| match e {
                        PassageEffect::PutNode { node, evidence }
                            if node.components.contains_key("detail.event") =>
                        {
                            evidence.quotes().len()
                        }
                        _ => 0,
                    })
                    .sum();
                if node.components.contains_key("detail.event")
                    && (source
                        .source_annotations
                        .iter()
                        .filter(|a| a.node_id == node.id)
                        .count()
                        + evidence.quotes().len()
                        > 32
                        || source.source_annotations.len() + event_citations > 10000)
                {
                    issues[i].push("This Event needs space for its source citation. Remove an older citation from the current node, or remove this Event proposal while retaining the other passage effects.".into());
                }
                if !alias(&node.id)
                    && (validate_id(&node.id).is_err() || !scoped.contains(node.id.as_str()))
                {
                    issues[i].push("This node is outside the supplied scene or has an invalid new-record reference. Remove this proposal or repair its record reference in Advanced JSON.".into());
                }
            }
            PassageEffect::PutLink { link, .. } => {
                let old = source.edges.iter().find(|e| e.id == link.id);
                if old.is_some_and(|old| {
                    old.type_id != link.type_id && has_planning_link_values(source, old)
                }) {
                    issues[i].push("This connection has author planning guidance. Preserve its relationship type; use world tools to revise it.".into());
                }
                for displaced in displaced_links(source, link) {
                    if !scoped.contains(displaced.source_id.as_str())
                        || !scoped.contains(displaced.target_id.as_str())
                    {
                        issues[i].push(format!("This proposal would replace {} outside the supplied scene. Add its previous target as a scene reference before validating, or remove this proposal.", link_label(source, displaced)));
                    }
                }
                if !alias(&link.id) {
                    if let Some(old) = source.edges.iter().find(|e| e.id == link.id) {
                        if !scoped.contains(old.source_id.as_str())
                            || !scoped.contains(old.target_id.as_str())
                        {
                            issues[i].push("The connection to update is outside this scene. Choose a supplied connection or Create new connection in Edit.".into());
                        }
                    } else {
                        issues[i].push("The referenced connection does not exist. In Edit, choose Create new connection or an existing connection to update.".into());
                    }
                }
                if link.source_id == link.target_id {
                    issues[i].push("Source and target must be different nodes.".into());
                }
                let definition = source.relation_types.iter().find(|r| r.id == link.type_id);
                if definition.is_none() {
                    issues[i].push("Choose a saved relationship type in Edit.".into());
                }
                if let Some(definition) = definition {
                    for key in link
                        .properties
                        .keys()
                        .filter(|key| !definition.fields.iter().any(|f| &f.key == *key))
                    {
                        issues[i].push(format!("Unknown relationship field {key}. Remove it in Edit or choose the intended relationship type."));
                    }
                    for field in &definition.fields {
                        if !planning_only(field)
                            && old
                                .filter(|e| e.type_id == link.type_id)
                                .and_then(|e| e.properties.get(&field.key))
                                .is_some_and(populated)
                            && !link.properties.contains_key(&field.key)
                        {
                            issues[i].push(format!("{} / {} was omitted from this complete connection. Restore its saved value in Edit, or explicitly clear it with passage evidence if the fact ended. Unrelated facts must be preserved.", definition.name, field.label));
                        }
                        if planning_only(field) {
                            let saved = old.filter(|e| e.type_id == link.type_id);
                            let before = saved
                                .and_then(|e| e.properties.get(&field.key))
                                .unwrap_or(&PropertyValue::Null);
                            let after = link
                                .properties
                                .get(&field.key)
                                .unwrap_or(&PropertyValue::Null);
                            let changed = if saved.is_some() {
                                before != after
                            } else {
                                after != &PropertyValue::Null && after != &field.default_value
                            };
                            if changed {
                                issues[i].push(format!("{} / {} is author planning guidance. Preserve its saved/default value or remove this proposal; use world tools to revise it.", definition.name, field.label));
                            }
                        }
                        match link.properties.get(&field.key) {
                            None if field.required => {
                                issues[i].push(format!("Fill in {} in Edit.", field.label))
                            }
                            Some(value) if !crate::model::field_matches(value, field, &nodes) => {
                                issues[i].push(format!("Correct {} in Edit.", field.label))
                            }
                            _ => (),
                        }
                    }
                    let previous = source
                        .edges
                        .iter()
                        .find(|e| e.id == link.id)
                        .map(|e| &e.properties);
                    for field in &definition.fields {
                        if previous.and_then(|p| p.get(&field.key))
                            != link.properties.get(&field.key)
                            && crate::projection::property_references(
                                std::slice::from_ref(field),
                                &link.properties,
                            )
                            .iter()
                            .any(|id| !allowed.contains(id.as_str()))
                        {
                            issues[i].push(format!("{} references a node outside this scene. Choose a supplied node in Edit.",field.label));
                        }
                    }
                }
                for (label, id, capability) in [
                    (
                        "Source",
                        &link.source_id,
                        definition.and_then(|r| r.source_capability.as_ref()),
                    ),
                    (
                        "Target",
                        &link.target_id,
                        definition.and_then(|r| r.target_capability.as_ref()),
                    ),
                ] {
                    if !allowed.contains(id.as_str()) {
                        issues[i].push(format!("{label} is outside the supplied scene. Choose a scene node or a proposed new node in Edit."));
                    } else if let (Some(node), Some(capability)) =
                        (nodes.get(id.as_str()), capability)
                    {
                        if !crate::model::capability(node, capability) {
                            let name = source
                                .component_types
                                .iter()
                                .find(|c| &c.id == capability)
                                .map_or(capability.as_str(), |c| c.name.as_str());
                            issues[i].push(format!("{label} “{}” needs the {name} capability for this relationship. Choose a compatible relationship or endpoint in Edit.", node.name));
                        }
                    }
                }
                if definition.is_some_and(|r| r.physical_placement) {
                    placements.entry(&link.source_id).or_default().push(i);
                } else if definition.is_some_and(|r| r.single_target) {
                    single_targets
                        .entry((&link.type_id, &link.source_id))
                        .or_default()
                        .push(i);
                }
            }
            PassageEffect::RemoveLink { id, .. } => {
                if let Some(old) = source.edges.iter().find(|e| &e.id == id) {
                    if !scoped.contains(old.source_id.as_str())
                        || !scoped.contains(old.target_id.as_str())
                    {
                        issues[i].push("The connection to remove is outside this scene. Choose a supplied connection in Edit or remove this proposal.".into());
                    }
                } else {
                    issues[i].push("The connection to remove no longer exists. Choose an existing connection in Edit or remove this proposal.".into());
                }
            }
        }
    }
    for indices in records.values().filter(|v| v.len() > 1) {
        for &i in indices {
            issues[i].push("Another proposal changes this same record. Keep one update/removal for it, or choose Create new connection for a separate connection.".into());
        }
    }
    for indices in placements.values().filter(|v| v.len() > 1) {
        for &i in indices {
            issues[i].push("Multiple physical placements are proposed for this node. Keep only its final location, worn, carried or stored placement.".into());
        }
    }
    for indices in single_targets.values().filter(|v| v.len() > 1) {
        for &i in indices {
            issues[i].push("Multiple targets are proposed for this single-target relationship and source. Keep one final connection before validating.".into());
        }
    }
    issues
}
