//! An explicit revision bundle combines protected edits with scoped additions.
//! Both reuse the ordinary authoring validators and publish one reviewed world.
use super::*;

pub(super) fn enabled(request: &AuthoringRequest) -> bool {
    request.mode == AuthoringMode::Revise && request.bundle
}

pub(super) fn guide(world: &World, request: &AuthoringRequest) -> Value {
    let additions = additions_request(world, request);
    let model = creation::schema_world(world, &additions);
    let mut guide = creation::guide(&model, &additions);
    guide["limits"] = json!({"newNodes":8,"newConnections":32,"totalNodesIncludingRevisions":20});
    guide
}

pub(super) fn additions_request(world: &World, request: &AuthoringRequest) -> AuthoringRequest {
    let mut next = request.clone();
    next.mode = AuthoringMode::Create;
    next.type_id = None;
    next.creation_scope = Some(CreationScope::Group);
    next.group_type_id = None;
    next.include_narration = None;
    next.context_ids = request
        .context_ids
        .iter()
        .chain(&request.target_ids)
        .cloned()
        .collect();
    for edge in world
        .edges
        .iter()
        .filter(|e| request.link_target_ids.contains(&e.id))
    {
        next.context_ids
            .extend([edge.source_id.clone(), edge.target_id.clone()]);
    }
    next.context_ids.sort();
    next.context_ids.dedup();
    next.target_ids.clear();
    next.fields.clear();
    next.link_target_ids.clear();
    next.link_fields.clear();
    next
}

pub(super) fn materialize(
    world: &World,
    request: &AuthoringRequest,
    text: &str,
) -> Result<String, String> {
    let mut value: Value =
        serde_json::from_str(text).map_err(|e| format!("Candidate JSON needs editing: {e}"))?;
    let Some(additions) = value.as_object_mut().and_then(|v| v.remove("additions")) else {
        return Ok(text.into());
    };
    let new_text = creation::materialize_text(
        world,
        &additions_request(world, request),
        &additions.to_string(),
    )?;
    let added: NodeCandidate = serde_json::from_str(&new_text)
        .map_err(|e| format!("Related additions need editing: {e}"))?;
    let mut existing: NodeCandidate =
        serde_json::from_value(value).map_err(|e| format!("Revision targets need editing: {e}"))?;
    existing.nodes.extend(added.nodes);
    existing.edges.extend(added.edges);
    serde_json::to_string(&existing).map_err(|e| e.to_string())
}

pub(super) fn candidate(
    source: &World,
    record: &mut AuthoringRecord,
    candidate: NodeCandidate,
) -> Result<(World, Vec<String>), String> {
    let (nodes, new_nodes): (Vec<_>, Vec<_>) =
        candidate.nodes.into_iter().partition(|n| !alias(&n.id));
    let (edges, new_edges): (Vec<_>, Vec<_>) =
        candidate.edges.into_iter().partition(|e| !alias(&e.id));
    if new_nodes.len() > 8 || new_edges.len() > 32 {
        return Err(
            "Narrow related additions to at most 8 new nodes and 32 new connections.".into(),
        );
    }
    // Additions can link from new nodes to the supplied selection/references;
    // they cannot move or replace existing placements, records or links.
    let mut additions = record.clone();
    additions.request = additions_request(source, &record.request);
    additions.candidate_text = serde_json::to_string(&NodeCandidate {
        nodes: new_nodes,
        edges: new_edges,
    })
    .map_err(|e| e.to_string())?;
    let (expanded, mut changes) = candidate_world(source, &mut additions)?;
    record.allocations = additions.allocations.clone();
    let mut revision = record.clone();
    revision.request.bundle = false;
    revision.request.creation_type_ids.clear();
    // Scoped new aliases are legal references in checked existing fields. The
    // original UUID targets and every unchecked value remain protected.
    revision.request.context_ids.extend(
        record
            .allocations
            .keys()
            .filter(|id| {
                expanded
                    .nodes
                    .iter()
                    .any(|n| Some(&n.id) == record.allocations.get(*id))
            })
            .cloned(),
    );
    revision.candidate_text =
        serde_json::to_string(&NodeCandidate { nodes, edges }).map_err(|e| e.to_string())?;
    let (world, updates) = candidate_world(&expanded, &mut revision)?;
    changes.extend(updates);
    Ok((world, changes))
}
