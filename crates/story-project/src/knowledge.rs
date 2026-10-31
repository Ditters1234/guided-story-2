//! Situated claims remain shared nodes. Author assessment never implies belief.
use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use specta::Type;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum KnowledgePerspective {
    #[default]
    Viewpoint,
    Author,
}
pub(crate) fn default_perspective(p: &KnowledgePerspective) -> bool {
    *p == KnowledgePerspective::Viewpoint
}
pub(crate) fn is_zero(n: &u32) -> bool {
    *n == 0
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClaimHolder {
    pub node_id: String,
    pub name: String,
    pub stance: String,
    pub acquisition: String,
    pub source_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClaimContext {
    pub node_id: String,
    pub name: String,
    pub proposition: String,
    pub notes: String,
    pub details: Vec<ClaimDetail>,
    pub author_assessment: String,
    pub reader_disclosed: bool,
    pub viewpoint_stance: Option<String>,
    pub holders: Vec<ClaimHolder>,
    pub sources: Vec<SourceAnnotation>,
    pub edited_since_source: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClaimDetail {
    pub capability: String,
    pub label: String,
    pub value: PropertyValue,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClaimPreview {
    pub node_id: String,
    pub name: String,
    pub included: bool,
    pub reason: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgePreview {
    pub included: Vec<ClaimContext>,
    pub decisions: Vec<ClaimPreview>,
    pub omitted_by_limit: u32,
}
pub(crate) fn is_claim(n: &WorldNode) -> bool {
    n.components.contains_key("core.claim")
}
fn value<'a>(n: &'a WorldNode, key: &str) -> &'a str {
    match n.components.get("core.claim").and_then(|v| v.get(key)) {
        Some(PropertyValue::Text(s)) => s,
        _ => "",
    }
}
fn edge_text<'a>(e: &'a WorldEdge, key: &str) -> &'a str {
    match e.properties.get(key) {
        Some(PropertyValue::Text(s)) => s,
        _ => "",
    }
}
pub(crate) fn validate_knowledge(world: &World) -> Result<(), String> {
    // Optional contracts never install themselves in historical schemas.
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Contracts {
        component_types: Vec<ComponentDefinition>,
        relation_types: Vec<RelationDefinition>,
    }
    let catalog: Contracts =
        serde_json::from_str(include_str!("../../../schema/knowledge-core.json"))
            .expect("Invalid pinned knowledge contracts");
    if let Some(def) = world.component_types.iter().find(|d| d.id == "core.claim") {
        if Some(def)
            != catalog
                .component_types
                .iter()
                .find(|d| d.id == "core.claim")
        {
            return Err("The Claim capability contract changed. Use a custom extension for additional fields.".into());
        }
    }
    if let Some(def) = world
        .relation_types
        .iter()
        .find(|d| d.id == "core.holds-claim")
    {
        if Some(def)
            != catalog
                .relation_types
                .iter()
                .find(|d| d.id == "core.holds-claim")
        {
            return Err("The held-by claim relationship contract changed. Use a custom relationship for other behavior.".into());
        }
    }
    let mut pairs = BTreeSet::new();
    for edge in world
        .edges
        .iter()
        .filter(|e| e.type_id == "core.holds-claim")
    {
        if !pairs.insert((&edge.source_id, &edge.target_id)) {
            return Err(
                "Keep one stance per character and claim; edit the existing held-by connection."
                    .into(),
            );
        }
        if !["believes", "doubts", "rejects"].contains(&edge_text(edge, "stance")) {
            return Err("Choose a character stance for this claim.".into());
        }
    }
    for node in world.nodes.iter().filter(|n| is_claim(n)) {
        if !["established", "unconfirmed", "disproved"].contains(&value(node, "status"))
            || !["withheld", "narrator", "revealed"].contains(&value(node, "disclosure"))
        {
            return Err(format!(
                "{} needs an author assessment and narration access.",
                node.name
            ));
        }
    }
    Ok(())
}
pub(crate) fn claim_allowed(world: &World, node: &WorldNode, plan: &PassagePlan) -> bool {
    match value(node, "disclosure") {
        "revealed" => true,
        "narrator" => {
            plan.knowledge_perspective == KnowledgePerspective::Author
                || plan.viewpoint_id.as_ref().is_some_and(|id| {
                    world.edges.iter().any(|e| {
                        e.type_id == "core.holds-claim"
                            && e.source_id == node.id
                            && &e.target_id == id
                    })
                })
        }
        _ => false,
    }
}
pub(crate) fn excluded_claims(world: &World, plan: &PassagePlan) -> BTreeSet<String> {
    world
        .nodes
        .iter()
        .filter(|n| is_claim(n) && !claim_allowed(world, n, plan))
        .map(|n| n.id.clone())
        .collect()
}
pub(crate) fn excluded_claim_passages(world: &World, plan: &PassagePlan) -> BTreeSet<String> {
    let excluded = excluded_claims(world, plan);
    world
        .source_annotations
        .iter()
        .filter(|a| excluded.contains(&a.node_id))
        .filter_map(|a| match &a.origin {
            SourceOrigin::Passage { passage_id, .. } => Some(passage_id.clone()),
            _ => None,
        })
        .collect()
}
pub(crate) fn relevant_claims(
    world: &World,
    plan: &PassagePlan,
    ids: &BTreeSet<String>,
) -> Vec<String> {
    let mut claims:Vec<_>=world.nodes.iter().filter(|n|is_claim(n)).filter_map(|n| {
        let selected=ids.contains(&n.id);
        let held=world.edges.iter().any(|e|e.type_id=="core.holds-claim"&&e.source_id==n.id&&ids.contains(&e.target_id));
        let subjects=matches!(n.components.get("core.claim").and_then(|v|v.get("subjects")),Some(PropertyValue::NodeRefs(refs)) if refs.iter().any(|id|ids.contains(id)));
        (selected||held||subjects).then_some((if plan.reference_ids.contains(&n.id){0}else if held{1}else{2},n.id.clone()))
    }).collect();
    claims.sort();
    claims.into_iter().map(|(_, id)| id).collect()
}
pub fn preview_knowledge(world: &World, plan: &PassagePlan) -> Result<KnowledgePreview, String> {
    world.validate()?;
    let ids = plan
        .viewpoint_id
        .iter()
        .chain(plan.location_id.iter())
        .chain(&plan.participant_ids)
        .chain(&plan.reference_ids)
        .cloned()
        .collect();
    let claims = relevant_claims(world, plan, &ids);
    let mut preview = KnowledgePreview {
        omitted_by_limit: claims.len().saturating_sub(20) as u32,
        ..Default::default()
    };
    for id in claims.into_iter().take(20) {
        let n = world.nodes.iter().find(|n| n.id == id).unwrap();
        let allowed = claim_allowed(world, n, plan);
        let holders: Vec<_> = world
            .edges
            .iter()
            .filter(|e| {
                e.type_id == "core.holds-claim" && e.source_id == id && ids.contains(&e.target_id)
            })
            .filter_map(|e| {
                world
                    .nodes
                    .iter()
                    .find(|n| n.id == e.target_id)
                    .map(|n| ClaimHolder {
                        node_id: n.id.clone(),
                        name: n.name.clone(),
                        stance: edge_text(e, "stance").into(),
                        acquisition: edge_text(e, "acquisition").into(),
                        source_id: match e.properties.get("source") {
                            Some(PropertyValue::Text(s)) => Some(s.clone()),
                            _ => None,
                        },
                    })
            })
            .collect();
        let viewpoint_stance = holders
            .iter()
            .find(|h| Some(&h.node_id) == plan.viewpoint_id.as_ref())
            .map(|h| h.stance.clone());
        let reason = if !allowed {
            if value(n, "disclosure") == "withheld" {
                "Withheld from prose context"
            } else {
                "No viewpoint stance; author perspective required"
            }
        } else if viewpoint_stance.is_some() {
            "Viewpoint stance; reader disclosure remains separate"
        } else {
            "Narrator information; do not grant character knowledge"
        };
        preview.decisions.push(ClaimPreview {
            node_id: id.clone(),
            name: n.name.clone(),
            included: allowed,
            reason: reason.into(),
        });
        if allowed {
            let sources: Vec<_> = world
                .source_annotations
                .iter()
                .filter(|a| a.node_id == id)
                .cloned()
                .collect();
            let details = world
                .component_types
                .iter()
                .filter_map(|d| n.components.get(&d.id).map(|v| (d, v)))
                .flat_map(|(d, v)| {
                    d.fields
                        .iter()
                        .filter(|f| f.used_for(FieldUse::Story))
                        .filter_map(|f| {
                            v.get(&f.key)
                                .filter(|p| **p != PropertyValue::Null)
                                .map(|p| ClaimDetail {
                                    capability: d.name.clone(),
                                    label: f.label.clone(),
                                    value: p.clone(),
                                })
                        })
                })
                .collect();
            preview.included.push(ClaimContext {
                node_id: id,
                name: n.name.clone(),
                proposition: value(n, "proposition").into(),
                notes: n.description.clone(),
                details,
                author_assessment: value(n, "status").into(),
                reader_disclosed: value(n, "disclosure") == "revealed",
                viewpoint_stance,
                holders,
                edited_since_source: sources
                    .iter()
                    .any(|a| a.subject_fingerprint != crate::sources::node_source_fingerprint(n)),
                sources,
            });
        }
    }
    let mut included = serde_json::to_value(&preview.included).map_err(|e| e.to_string())?;
    scrub_references(&mut included, &excluded_claims(world, plan));
    preview.included = serde_json::from_value(included).map_err(|e| e.to_string())?;
    Ok(preview)
}
pub(crate) fn resolve_narrative_context(
    world: &World,
    task: TextPromptTask,
    plan: &PassagePlan,
) -> Result<Vec<AiContextSection>, String> {
    let excluded = excluded_claims(world, plan);
    if world
        .ai_context
        .iter()
        .any(|b| b.enabled && b.tasks.contains(&task) && excluded.contains(&b.node_id))
    {
        return Err("Required AI context includes a claim omitted by this narration perspective. Change its inclusion, access or scene perspective; required context was not silently dropped.".into());
    }
    resolve_ai_context(world, task)
}
pub(crate) fn mentions_excluded(
    value: &impl Serialize,
    excluded: &BTreeSet<String>,
) -> Result<bool, String> {
    fn contains(v: &serde_json::Value, ids: &BTreeSet<String>) -> bool {
        match v {
            serde_json::Value::String(s) => ids.contains(s),
            serde_json::Value::Array(a) => a.iter().any(|v| contains(v, ids)),
            serde_json::Value::Object(m) => m.values().any(|v| contains(v, ids)),
            _ => false,
        }
    }
    Ok(contains(
        &serde_json::to_value(value).map_err(|e| e.to_string())?,
        excluded,
    ))
}
pub(crate) fn scrub_references(value: &mut serde_json::Value, excluded: &BTreeSet<String>) {
    match value {
        serde_json::Value::String(s) if excluded.contains(s) => *value = serde_json::Value::Null,
        serde_json::Value::Array(a) => {
            a.retain(|v| !matches!(v,serde_json::Value::String(s) if excluded.contains(s)));
            for v in a {
                scrub_references(v, excluded);
            }
        }
        serde_json::Value::Object(m) => {
            for v in m.values_mut() {
                scrub_references(v, excluded);
            }
        }
        _ => (),
    }
}
pub(crate) fn project_records(context: &World, input: &mut serde_json::Value) {
    let claim_ids: BTreeSet<_> = context
        .nodes
        .iter()
        .filter(|n| is_claim(n))
        .map(|n| &n.id)
        .collect();
    input["records"] = json!(context
        .nodes
        .iter()
        .filter(|n| !is_claim(n))
        .map(|n| narrative_record(context, n))
        .collect::<Vec<_>>());
    input["links"] = json!(context
        .edges
        .iter()
        .filter(|e| !claim_ids.contains(&e.source_id) && !claim_ids.contains(&e.target_id))
        .map(|e| narrative_link(context, e))
        .collect::<Vec<_>>());
    input["sources"] = json!(context
        .source_annotations
        .iter()
        .filter(|a| !claim_ids.contains(&a.node_id))
        .collect::<Vec<_>>());
    let starts = input["startingState"]["people"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if let Some(records) = input["records"].as_array_mut() {
        for record in records {
            if let Some(start) = starts.iter().find(|p| p["id"] == record["id"]) {
                record["startingPosition"] = json!({"currentPlace":start["currentPlace"],"relativeToIntendedFocus":start["relativeToFocus"]});
            }
        }
    }
}
#[cfg(test)]
mod tests;
