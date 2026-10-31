//! A compact, named account of the captured physical start, separate from plans.
use super::*;

pub(super) fn starting_state(world: &World, plan: &PassagePlan) -> Value {
    let nodes: BTreeMap<_, _> = world
        .nodes
        .iter()
        .filter(|n| !crate::knowledge::is_claim(n))
        .map(|n| (n.id.as_str(), n))
        .collect();
    let physical: BTreeMap<_, _> = world
        .relation_types
        .iter()
        .filter(|r| r.physical_placement)
        .map(|r| (r.id.as_str(), r))
        .collect();
    let placements: BTreeMap<_, _> = world
        .edges
        .iter()
        .filter(|e| {
            physical.contains_key(e.type_id.as_str())
                && nodes.contains_key(e.source_id.as_str())
                && nodes.contains_key(e.target_id.as_str())
        })
        .map(|e| (e.source_id.as_str(), e))
        .collect();
    let named = |id: &str| nodes.get(id).map(|n| json!({"id":n.id,"name":n.name}));
    let focus = plan
        .location_id
        .as_deref()
        .filter(|id| nodes.contains_key(*id));
    let mut used_links = BTreeSet::new();
    let people: Vec<_> = nodes.values().filter(|n| n.components.contains_key("core.character")).map(|n| {
        let mut current = n.id.as_str();
        let mut visited = BTreeSet::new();
        let mut place = None;
        let mut within_focus = false;
        while visited.insert(current) {
            let Some(edge) = placements.get(current) else { break; };
            used_links.insert(edge.id.as_str());
            current = edge.target_id.as_str();
            let target = nodes[current];
            if place.is_none() && target.components.contains_key("core.place") { place = Some(current); }
            if Some(current) == focus { within_focus = true; }
        }
        let status = if focus.is_none() { "unspecifiedFocus" }
            else if place == focus { "atFocus" }
            else if within_focus { "withinFocusArea" }
            else if place.is_some() { "elsewhere" }
            else { "unknown" };
        let role = if plan.viewpoint_id.as_ref() == Some(&n.id) { "viewpoint" }
            else if plan.participant_ids.contains(&n.id) { "participant" }
            else if plan.reference_ids.contains(&n.id) { "reference" }
            else { "placementContext" };
        json!({"id":n.id,"name":n.name,"selectedAs":role,"currentPlace":place.and_then(named),"relativeToFocus":status})
    }).collect();
    let links: Vec<_> = world.edges.iter().filter(|e| used_links.contains(e.id.as_str())).map(|e| {
        json!({"sourceId":e.source_id,"sourceName":nodes[e.source_id.as_str()].name,
            "relationship":physical[e.type_id.as_str()].label,"targetId":e.target_id,"targetName":nodes[e.target_id.as_str()].name})
    }).collect();
    let opening = people
        .iter()
        .find(|p| p["selectedAs"] == "viewpoint")
        .map(|p| json!({"person":p["name"],"place":p["currentPlace"]}));
    let facts: Vec<_> = people
        .iter()
        .filter(|p| p["selectedAs"] == "viewpoint" || p["selectedAs"] == "participant")
        .map(|p| {
            if let Some(place) = p["currentPlace"]["name"].as_str() {
                format!(
                    "{} starts in {}.",
                    p["name"].as_str().unwrap_or("Selected person"),
                    place
                )
            } else {
                format!(
                    "{} has no established starting place.",
                    p["name"].as_str().unwrap_or("Selected person")
                )
            }
        })
        .collect();
    json!({"intendedSceneFocus":focus.and_then(named),"openingViewpoint":opening,"openingFacts":facts,"people":people,"physicalLinks":links})
}
