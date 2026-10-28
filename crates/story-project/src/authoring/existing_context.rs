//! Bounded, read-only discovery for additions to an established world.
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExistingPlacement {
    pub type_id: String,
    pub target_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExistingNodeSummary {
    pub id: String,
    pub name: String,
    pub type_id: String,
    pub capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<ExistingPlacement>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExistingAuthoringContext {
    pub catalog: Vec<ExistingNodeSummary>,
    pub detailed_node_ids: Vec<String>,
    pub total_nodes: u32,
    pub omitted_nodes: u32,
    pub omitted_details: u32,
    pub omitted_links: u32,
}
fn words(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|w| {
            w.chars().count() >= 3
                && ![
                    "the", "and", "for", "with", "from", "this", "that", "some", "create", "group",
                    "new", "add", "only", "items", "nodes",
                ]
                .contains(&w.as_str())
        })
        .collect()
}
pub(super) fn group_context(
    world: &World,
    request: &AuthoringRequest,
    budget: usize,
) -> Result<(ExistingAuthoringContext, Value), String> {
    let budget = budget.min(24_000);
    if budget < 768 {
        return Err("No room to check existing world nodes. Narrow explicit references/required instructions or increase the context allowance.".into());
    }
    let explicit: BTreeSet<_> = request.context_ids.iter().cloned().collect();
    let query = words(&request.direction);
    let desired_capabilities = world
        .node_types
        .iter()
        .find(|t| Some(&t.id) == request.group_type_id.as_ref())
        .map(|t| t.components.as_slice())
        .unwrap_or_default();
    let mut scores = BTreeMap::new();
    for node in &world.nodes {
        let name = words(&node.name);
        let text = format!(
            "{} {}",
            node.description.chars().take(1200).collect::<String>(),
            node.visual_description
                .chars()
                .take(600)
                .collect::<String>()
        );
        let score = if explicit.contains(&node.id) {
            100_000
        } else {
            0
        } + query.intersection(&name).count() as u32 * 400
            + query.intersection(&words(&text)).count().min(10) as u32 * 4
            + if request.group_type_id.as_ref() == Some(&node.type_id) {
                15
            } else {
                0
            }
            + desired_capabilities
                .iter()
                .filter(|id| node.components.contains_key(*id))
                .count() as u32
                * 5;
        scores.insert(node.id.clone(), score);
    }
    let physical: BTreeSet<_> = world
        .relation_types
        .iter()
        .filter(|r| r.physical_placement)
        .map(|r| r.id.as_str())
        .collect();
    let placements: BTreeMap<_, _> = world
        .edges
        .iter()
        .filter(|e| physical.contains(e.type_id.as_str()))
        .map(|e| (e.source_id.as_str(), e))
        .collect();
    // Follow at most three physical hops from named/selected nodes. A holder's
    // items, worn clothing, containers and contents use the same generic links.
    let mut frontier: BTreeSet<_> = scores
        .iter()
        .filter(|(_, s)| **s >= 400)
        .map(|(id, _)| id.clone())
        .collect();
    let mut visited = frontier.clone();
    for depth in 0..3 {
        let mut next = BTreeSet::new();
        for edge in world
            .edges
            .iter()
            .filter(|e| physical.contains(e.type_id.as_str()))
        {
            for (a, b) in [
                (&edge.source_id, &edge.target_id),
                (&edge.target_id, &edge.source_id),
            ] {
                if frontier.contains(a) && visited.insert(b.clone()) {
                    let score = scores.get_mut(b).ok_or("Missing linked world node.")?;
                    *score = (*score).max(240 - depth * 40);
                    next.insert(b.clone());
                }
            }
        }
        frontier = next;
    }
    let mut ranked: Vec<_> = world.nodes.iter().collect();
    ranked.sort_by(|a, b| {
        scores[&b.id]
            .cmp(&scores[&a.id])
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.id.cmp(&b.id))
    });
    let mut catalog = vec![];
    let mut catalog_bytes = 0;
    for node in &ranked {
        let entry = ExistingNodeSummary {
            id: node.id.clone(),
            name: node.name.clone(),
            type_id: node.type_id.clone(),
            capabilities: node.components.keys().cloned().collect(),
            placement: placements.get(node.id.as_str()).map(|e| ExistingPlacement {
                type_id: e.type_id.clone(),
                target_id: e.target_id.clone(),
            }),
        };
        let size = serde_json::to_vec(&entry).map_err(|e| e.to_string())?.len() + 1;
        if catalog.len() < 100 && catalog_bytes + size <= (budget - 512) * 45 / 100 {
            catalog_bytes += size;
            catalog.push(entry);
        }
    }
    if catalog.is_empty() && !world.nodes.is_empty() {
        return Err("No existing node fits the world-context allowance. Narrow required context or increase the context allowance.".into());
    }
    let indexed: BTreeSet<_> = catalog.iter().map(|n| n.id.as_str()).collect();
    let relevant = |node: &&WorldNode| {
        !explicit.contains(&node.id) && (scores[&node.id] > 0 || world.nodes.len() <= 12)
    };
    let relevant_count = ranked.iter().copied().filter(relevant).count();
    let mut details = vec![];
    let mut bytes = catalog_bytes + 512;
    for node in ranked
        .iter()
        .copied()
        .filter(relevant)
        .filter(|n| indexed.contains(n.id.as_str()))
    {
        let size = serde_json::to_vec(node).map_err(|e| e.to_string())?.len() + 1;
        if details.len() < 12 && bytes + size <= budget {
            bytes += size;
            details.push(node);
        }
    }
    let detailed_node_ids: Vec<_> = explicit
        .iter()
        .cloned()
        .chain(details.iter().map(|n| n.id.clone()))
        .collect();
    let detailed: BTreeSet<_> = detailed_node_ids.iter().map(String::as_str).collect();
    let mut links = vec![];
    let mut omitted_links = 0;
    for edge in world.edges.iter().filter(|e| {
        detailed.contains(e.source_id.as_str()) && detailed.contains(e.target_id.as_str())
    }) {
        if explicit.contains(&edge.source_id) && explicit.contains(&edge.target_id) {
            continue;
        }
        let entry = json!({"id":edge.id,"typeId":edge.type_id,"sourceId":edge.source_id,"targetId":edge.target_id});
        let size = entry.to_string().len() + 1;
        if links.len() < 100 && bytes + size <= budget {
            bytes += size;
            links.push(entry);
        } else {
            omitted_links += 1;
        }
    }
    let context = ExistingAuthoringContext {
        omitted_nodes: (world.nodes.len() - catalog.len()) as u32,
        omitted_details: (relevant_count - details.len()) as u32,
        total_nodes: world.nodes.len() as u32,
        catalog,
        detailed_node_ids,
        omitted_links,
    };
    let payload = json!({"catalog":context.catalog,"details":details,"links":links,"coverage":{"totalNodes":context.total_nodes,"indexedNodes":context.catalog.len(),"fullRecords":context.detailed_node_ids.len(),"omittedNodes":context.omitted_nodes,"omittedDetails":context.omitted_details,"omittedLinks":context.omitted_links}});
    if payload.to_string().len() > budget {
        return Err("Existing world context exceeded its allowance. Narrow required context or increase the context allowance.".into());
    }
    Ok((context, payload))
}
