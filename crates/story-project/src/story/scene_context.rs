use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SceneResourceRole {
    Focus,
    Viewpoint,
    Participant,
    Reference,
    Inventory,
    ContainingPlace,
    Placement,
    Claim,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SceneResource {
    pub node_id: String,
    pub name: String,
    pub role: SceneResourceRole,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SceneContextPreview {
    pub resources: Vec<SceneResource>,
    pub guidance: Vec<AiContextSection>,
    pub error: String,
}

pub(super) fn inventory_ids<'a>(world: &'a World, plan: &PassagePlan) -> BTreeSet<&'a String> {
    let actors: BTreeSet<_> = plan
        .viewpoint_id
        .iter()
        .chain(&plan.participant_ids)
        .chain(plan.reference_ids.iter().filter(|id| {
            world
                .nodes
                .iter()
                .any(|node| &node.id == *id && node.components.contains_key("core.character"))
        }))
        .collect();
    world
        .edges
        .iter()
        .filter(|edge| {
            matches!(edge.type_id.as_str(), "core.worn-by" | "core.stored-in")
                && actors.contains(&edge.target_id)
        })
        .map(|edge| &edge.source_id)
        .collect()
}

fn role(node: &WorldNode, plan: &PassagePlan, inventory: &BTreeSet<&String>) -> SceneResourceRole {
    use SceneResourceRole::*;
    if crate::knowledge::is_claim(node) {
        Claim
    } else if plan.location_id.as_ref() == Some(&node.id) {
        Focus
    } else if plan.viewpoint_id.as_ref() == Some(&node.id) {
        Viewpoint
    } else if plan.participant_ids.contains(&node.id) {
        Participant
    } else if plan.reference_ids.contains(&node.id) {
        Reference
    } else if inventory.contains(&node.id) {
        Inventory
    } else if node.components.contains_key("core.place") {
        ContainingPlace
    } else {
        Placement
    }
}

pub(super) fn validate_cast(
    world: &World,
    context: &World,
    plan: &PassagePlan,
    settings: &LlmConnection,
    guidance: &[AiContextSection],
) -> Result<(), String> {
    let mut texts = vec![plan.chapter_plan.as_str(), plan.direction.as_str()];
    texts.extend(
        settings
            .prompts
            .overrides
            .get(&TextPromptTask::Passage)
            .map(String::as_str),
    );
    for section in guidance {
        texts.extend([section.name.as_str(), section.content.as_str()]);
    }
    cast_scope::validate_named_cast_scope(world, context, &texts)
}

pub(super) fn preview(
    world: &World,
    context: &World,
    input: &Value,
    plan: &PassagePlan,
    settings: &LlmConnection,
    task: TextPromptTask,
) -> SceneContextPreview {
    let included_claims: BTreeSet<_> = input["situatedKnowledge"]["claims"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|claim| claim["nodeId"].as_str())
        .collect();
    let inventory = inventory_ids(context, plan);
    let mut resources: Vec<_> = context
        .nodes
        .iter()
        .filter(|node| {
            !crate::knowledge::is_claim(node) || included_claims.contains(node.id.as_str())
        })
        .map(|node| SceneResource {
            node_id: node.id.clone(),
            name: node.name.clone(),
            role: role(node, plan, &inventory),
        })
        .collect();
    resources.sort_by(|a, b| (&a.role, &a.name, &a.node_id).cmp(&(&b.role, &b.name, &b.node_id)));
    let (guidance, error) = match crate::knowledge::resolve_narrative_context(world, task, plan) {
        Ok(guidance) => {
            let error = if task == TextPromptTask::Passage {
                validate_cast(world, context, plan, settings, &guidance)
                    .err()
                    .unwrap_or_default()
            } else {
                String::new()
            };
            (guidance, error)
        }
        Err(error) => (vec![], error),
    };
    SceneContextPreview {
        resources,
        guidance,
        error,
    }
}

pub(super) fn project_records(context: &World, plan: &PassagePlan, input: &mut Value) {
    crate::knowledge::project_records(context, input);
    let inventory = inventory_ids(context, plan);
    if let Some(records) = input["records"].as_array_mut() {
        for record in records {
            if let Some(node) = context
                .nodes
                .iter()
                .find(|node| record["id"].as_str() == Some(&node.id))
            {
                record["sceneRole"] = json!(role(node, plan, &inventory));
            }
        }
    }
}
