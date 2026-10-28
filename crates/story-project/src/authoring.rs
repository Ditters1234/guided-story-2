//! Schema-driven world proposals. Provider output is untrusted data until explicitly applied.
use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use specta::Type;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AuthoringMode {
    Create,
    Revise,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum CreationScope {
    World,
    Group,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditableField {
    pub node_id: String,
    pub component_id: Option<String>,
    pub key: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthoringRequest {
    pub mode: AuthoringMode,
    pub direction: String,
    pub type_id: Option<String>,
    pub bundle: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creation_scope: Option<CreationScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_type_id: Option<String>,
    pub target_ids: Vec<String>,
    pub context_ids: Vec<String>,
    pub fields: Vec<EditableField>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NodeCandidate {
    pub nodes: Vec<WorldNode>,
    pub edges: Vec<WorldEdge>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AuthoringStatus {
    Draft,
    Running,
    Review,
    Failed,
    Cancelled,
    Interrupted,
    Applied,
    Discarded,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ApplyAttempt {
    expected_revision: u32,
    title: String,
    world_ref: RecordRef,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AuthoringRecord {
    #[serde(default)]
    created_at: u64,
    schema_version: u32,
    id: String,
    story_id: String,
    version: u32,
    status: AuthoringStatus,
    request: AuthoringRequest,
    source_world_ref: Option<RecordRef>,
    source_fingerprint: String,
    provider_label: String,
    raw_response: String,
    candidate_text: String,
    error: String,
    allocations: BTreeMap<String, String>,
    applied_revision: Option<u32>,
    apply_attempt: Option<ApplyAttempt>,
    incomplete: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthoringSummary {
    pub created_at: String,
    pub id: String,
    pub version: u32,
    pub status: AuthoringStatus,
    pub direction: String,
    pub provider_label: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthoringView {
    pub id: String,
    pub story_id: String,
    pub version: u32,
    pub status: AuthoringStatus,
    pub request: AuthoringRequest,
    pub provider_label: String,
    pub raw_response: String,
    pub candidate_text: String,
    pub error: String,
    pub stale: bool,
    pub changes: Vec<String>,
    pub warnings: Vec<String>,
    pub context: World,
    pub estimated_input_tokens: u32,
    pub created_node_ids: Vec<String>,
    pub applied_revision: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyAuthoringResult {
    pub commit: CommitResult,
    pub proposal: AuthoringView,
}
#[derive(Debug, Clone)]
pub struct PreparedAuthoring {
    pub system: String,
    pub user: String,
    pub response_schema: Value,
    pub fingerprint: String,
    pub estimated_input_tokens: u32,
    pub context: World,
}

fn hash<T: Serialize>(value: &T) -> Result<String, String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).map_err(|e| e.to_string())?)
    ))
}
fn bounded_request(request: &AuthoringRequest) -> Result<(), String> {
    if request.direction.chars().count() > 12000
        || request.target_ids.len() > 12
        || request.context_ids.len() > 12
        || request.fields.len() > 200
    {
        return Err(
            "Narrow the request: at most 12 targets/references and 200 editable fields.".into(),
        );
    }
    for id in request.target_ids.iter().chain(&request.context_ids) {
        validate_id(id)?;
    }
    Ok(())
}
pub fn authoring_fields(world: &World, targets: &[String]) -> Vec<EditableField> {
    let mut fields = vec![];
    for node in world.nodes.iter().filter(|n| targets.contains(&n.id)) {
        for key in ["name", "description", "visualDescription"] {
            fields.push(EditableField {
                node_id: node.id.clone(),
                component_id: None,
                key: key.into(),
            });
        }
        for id in node.components.keys() {
            if let Some(def) = world.component_types.iter().find(|d| &d.id == id) {
                for field in &def.fields {
                    fields.push(EditableField {
                        node_id: node.id.clone(),
                        component_id: Some(id.clone()),
                        key: field.key.clone(),
                    });
                }
            }
        }
    }
    fields
}
fn field_schema(kind: &FieldKind) -> Value {
    match kind {
        FieldKind::Text | FieldKind::LongText => json!({"type":"string","maxLength":16000}),
        FieldKind::Number => json!({"type":"number"}),
        FieldKind::Boolean => json!({"type":"boolean"}),
        FieldKind::NodeRef => json!({"type":["string","null"]}),
        FieldKind::NodeRefs => json!({"type":"array","items":{"type":"string"},"maxItems":500}),
    }
}
fn output_schema(world: &World, request: &AuthoringRequest) -> Value {
    let mut choices = vec![];
    let component_properties:serde_json::Map<String,Value>=world.component_types.iter().map(|d| {
        let fields:serde_json::Map<String,Value>=d.fields.iter().map(|f|(f.key.clone(),field_schema(&f.kind))).collect();
        (d.id.clone(),json!({"type":"object","properties":fields,"required":d.fields.iter().filter(|f|f.required).map(|f|&f.key).collect::<Vec<_>>(),"additionalProperties":false}))
    }).collect();
    let types: Vec<_> = world
        .node_types
        .iter()
        .filter(|d| {
            if request.mode == AuthoringMode::Revise {
                world
                    .nodes
                    .iter()
                    .any(|n| request.target_ids.contains(&n.id) && n.type_id == d.id)
            } else if let Some(type_id) = &request.group_type_id {
                &d.id == type_id
            } else {
                request.bundle || request.type_id.as_ref() == Some(&d.id)
            }
        })
        .collect();
    for preset in types {
        let character = preset.components.iter().any(|id| id == "core.character");
        let required = if request.mode == AuthoringMode::Create {
            preset.components.clone()
        } else {
            vec![]
        };
        choices.push(json!({"type":"object","properties":{
            "id":{"type":"string"},"typeId":{"const":preset.id},"name":{"type":"string","minLength":1,"maxLength":200,"description":if request.mode==AuthoringMode::Revise{"Preserve the existing name unless Name is editable and the author asks to change it."}else if character{"A personal name, unless the author explicitly wants an unnamed person. Age/role labels belong in descriptions, not as placeholder names."}else{"A specific name for this record."}},
            "description":{"type":"string","maxLength":16000},"visualDescription":{"type":"string","maxLength":16000},
            "components":{"allOf":[{"$ref":"#/$defs/components"}],"required":required}
        },"required":["id","typeId","name","description","visualDescription","components"],"additionalProperties":false}));
    }
    json!({"type":"object","$defs":{"components":{"type":"object","properties":component_properties,"additionalProperties":false}},"properties":{
        "nodes":{"type":"array","minItems":1,"maxItems":20,"items":{"anyOf":choices}},
        "edges":{"type":"array","maxItems":80,"items":{"type":"object","properties":{
            "id":{"type":"string"},"typeId":{"type":"string","enum":world.relation_types.iter().map(|r|&r.id).collect::<Vec<_>>()},"sourceId":{"type":"string"},"targetId":{"type":"string"},"description":{"type":"string","maxLength":16000}
        },"required":["id","typeId","sourceId","targetId","description"],"additionalProperties":false}}
    },"required":["nodes","edges"],"additionalProperties":false})
}
pub fn prepare_authoring(
    world: &World,
    request: &AuthoringRequest,
    settings: &LlmConnection,
) -> Result<PreparedAuthoring, String> {
    world.validate()?;
    bounded_request(request)?;
    settings.ready()?;
    if request.mode == AuthoringMode::Create && request.direction.trim().is_empty() {
        return Err("Describe what you want to create or change.".into());
    }
    let targets: BTreeSet<_> = request.target_ids.iter().cloned().collect();
    let context_ids: BTreeSet<_> = request
        .target_ids
        .iter()
        .chain(&request.context_ids)
        .cloned()
        .collect();
    for id in &context_ids {
        if !world.nodes.iter().any(|n| &n.id == id) {
            return Err("A selected target or reference no longer exists.".into());
        }
    }
    if request.mode == AuthoringMode::Revise {
        if targets.is_empty()
            || targets.len() != request.target_ids.len()
            || request.fields.is_empty()
            || request.bundle
            || request.creation_scope.is_some()
            || request.group_type_id.is_some()
        {
            return Err("Select existing nodes and the fields AI may change.".into());
        }
        let allowed = authoring_fields(world, &request.target_ids);
        let mut unique = BTreeSet::new();
        for field in &request.fields {
            if !allowed.contains(field) || !unique.insert(field) {
                return Err(
                    "An editable field is missing, duplicated or outside the selected targets."
                        .into(),
                );
            }
        }
    } else {
        if !request.target_ids.is_empty() || !request.fields.is_empty() {
            return Err("Creation cannot rewrite existing nodes.".into());
        }
        if (request.creation_scope.is_some() && (request.type_id.is_some() || !request.bundle))
            || (request.group_type_id.is_some()
                && request.creation_scope != Some(CreationScope::Group))
        {
            return Err("Choose world building, a group, or a typed node without mixing their scope controls.".into());
        }
        if let Some(id) = &request.group_type_id {
            if !world.node_types.iter().any(|d| &d.id == id) {
                return Err("Choose a saved node type for this group.".into());
            }
        }
        if let Some(id) = &request.type_id {
            if !world.node_types.iter().any(|d| &d.id == id) {
                return Err("Choose a saved node type.".into());
            }
        } else if !request.bundle {
            return Err("Choose a node type or enable a connected group.".into());
        }
    }
    let mut context = world.clone();
    context.ai_context.clear();
    context.nodes.retain(|n| context_ids.contains(&n.id));
    context
        .edges
        .retain(|e| context_ids.contains(&e.source_id) && context_ids.contains(&e.target_id));
    // Revisions cannot add types, capabilities or relationships. Send only the
    // schema needed to interpret the supplied targets/references, not the catalogue.
    if request.mode == AuthoringMode::Revise {
        let types: BTreeSet<_> = context.nodes.iter().map(|n| n.type_id.clone()).collect();
        let components: BTreeSet<_> = context
            .nodes
            .iter()
            .flat_map(|n| n.components.keys().cloned())
            .collect();
        let relations: BTreeSet<_> = context.edges.iter().map(|e| e.type_id.clone()).collect();
        context.node_types.retain(|d| types.contains(&d.id));
        context
            .component_types
            .retain(|d| components.contains(&d.id));
        context.relation_types.retain(|d| relations.contains(&d.id));
    }
    let response_schema = output_schema(&context, request);
    let mut system = settings.system_prompt(TextPromptTask::World);
    let mut effective_request = request.clone();
    if request.mode == AuthoringMode::Revise && request.direction.trim().is_empty() {
        effective_request.direction="Develop the checked fields from each node's name, type, existing facts and supplied context. Fill missing details and generate a fresh version of populated checked fields. Existing descriptions are not required. Keep identity, unchecked fields, capabilities and links unchanged. For notes, rules or instruction profiles, write useful content for their stated purpose; leave physical appearance empty when it is not relevant.".into();
    }
    // Include reusable type definitions; no unselected world records or implicit map expansion.
    let mut input = json!({"request":effective_request,"records":context.nodes,"links":context.edges,"nodeTypes":context.node_types,"capabilities":context.component_types,"relationships":context.relation_types,"responseBudget":{"outputTokens":settings.max_tokens,"suggestedMaxNewNodes":(settings.max_tokens.saturating_sub(512)/350).clamp(1,20)}});
    if request.mode == AuthoringMode::Create {
        input["creationIntent"] = json!(if request.type_id.is_some() {
            "node"
        } else if request.creation_scope == Some(CreationScope::Group) {
            "group"
        } else {
            "world"
        });
        input["allowedNewTypeIds"] = json!(request.group_type_id.as_ref().map(|id| vec![id]));
    }
    let character_root = request
        .type_id
        .as_ref()
        .and_then(|id| context.node_types.iter().find(|d| &d.id == id))
        .is_some_and(|d| d.components.iter().any(|id| id == "core.character"));
    if request.mode == AuthoringMode::Create
        && request.bundle
        && (input["creationIntent"] == "world" || character_root)
    {
        if let Some(example) = clothing_example(&context) {
            input["clothingExample"] = example;
        }
    }
    let mut context_source = world.clone();
    if request.mode == AuthoringMode::Revise {
        let bindings = world
            .ai_context
            .iter()
            .filter(|b| targets.contains(&b.node_id))
            .collect::<Vec<_>>();
        if !bindings.is_empty() {
            input["targetContextBindings"] = json!(bindings);
        }
        // A target is supplied in records as editable source, including when its
        // required context body is blank. It must not govern its own rewrite.
        context_source
            .ai_context
            .retain(|b| !targets.contains(&b.node_id));
    }
    let included = resolve_ai_context(&context_source, TextPromptTask::World)?;
    append_ai_context(&included, &mut system, &mut input);
    let schema_bytes = if settings.structured_output == StructuredOutput::Schema {
        response_schema.to_string().len()
    } else {
        input["responseSchema"] = response_schema.clone();
        0
    };
    let user = serde_json::to_string(&input).map_err(|e| e.to_string())?;
    let estimated_input_tokens = ((system.len() + user.len() + schema_bytes) / 3 + 256) as u32;
    if estimated_input_tokens + settings.max_tokens > settings.context_tokens {
        return Err(format!("Required context and output exceed the configured token budget. Included AI context: {}. Shorten included nodes or narrow references/schema; no required content was dropped.",included.iter().map(|s|s.name.as_str()).collect::<Vec<_>>().join(", ")));
    }
    let fingerprint = hash(&(
        "world-authoring-v4",
        &system,
        &user,
        &response_schema,
        settings,
    ))?;
    Ok(PreparedAuthoring {
        system,
        user,
        response_schema,
        fingerprint,
        estimated_input_tokens,
        context,
    })
}
fn clothing_example(world: &World) -> Option<Value> {
    let preset = world
        .node_types
        .iter()
        .find(|d| d.components.iter().any(|c| c == "core.item"))?;
    if !world
        .component_types
        .iter()
        .any(|d| d.id == "core.wearable")
        || !["core.stored-in", "core.worn-by"]
            .iter()
            .all(|id| world.relation_types.iter().any(|r| &r.id == id))
    {
        return None;
    }
    let mut components: BTreeMap<String, BTreeMap<String, PropertyValue>> = preset
        .components
        .iter()
        .map(|id| {
            let fields = world
                .component_types
                .iter()
                .find(|d| &d.id == id)
                .map(|d| {
                    d.fields
                        .iter()
                        .map(|f| (f.key.clone(), f.default_value.clone()))
                        .collect()
                })
                .unwrap_or_default();
            (id.clone(), fields)
        })
        .collect();
    let mut wearable: BTreeMap<_, _> = world
        .component_types
        .iter()
        .find(|d| d.id == "core.wearable")?
        .fields
        .iter()
        .map(|f| (f.key.clone(), f.default_value.clone()))
        .collect();
    wearable.insert("regions".into(), PropertyValue::Text("torso, arms".into()));
    wearable.insert("layer".into(), PropertyValue::Number(1.0));
    components.insert("core.wearable".into(), wearable);
    Some(
        json!({"garment":{"id":"new:shirt","typeId":preset.id,"name":"Linen work shirt","description":"A practical everyday shirt.","visualDescription":"Faded blue linen, long sleeves and wooden buttons.","components":components},"links":[{"id":"new:shirt-worn","typeId":"core.worn-by","sourceId":"new:shirt","targetId":"new:wearer","description":""}],"note":"new:wearer stands for a character created in this proposal or an explicitly supplied character reference; include real character records, not this placeholder."}),
    )
}
fn proposal_warnings(candidate: &NodeCandidate, request: &AuthoringRequest) -> Vec<String> {
    if request.mode != AuthoringMode::Create || !request.bundle {
        return vec![];
    }
    candidate.nodes.iter().filter(|n|n.components.contains_key("core.character")).filter(|n|!candidate.edges.iter().any(|e|e.type_id=="core.worn-by"&&e.target_id==n.id)).map(|n|{
        if request.group_type_id.is_some() {
            format!("{} has no linked worn items. Add separate clothing in an item group afterward, or use a mixed-type group. Keep this if intentional.",n.name)
        }else{
            format!("{} has no linked worn items. Add outfit items before Apply, or keep this if intentional. Clothing written only in a character description is not an item-based outfit.",n.name)
        }
    }).collect()
}
fn alias(id: &str) -> bool {
    id.strip_prefix("new:").is_some_and(|s| {
        !s.is_empty()
            && s.len() <= 40
            && s.bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    })
}
fn selected(request: &AuthoringRequest, id: &str, component: Option<&str>, key: &str) -> bool {
    request
        .fields
        .iter()
        .any(|f| f.node_id == id && f.component_id.as_deref() == component && f.key == key)
}
fn candidate_world(
    source: &World,
    record: &mut AuthoringRecord,
) -> Result<(World, Vec<String>), String> {
    let candidate: NodeCandidate = serde_json::from_str(&record.candidate_text)
        .map_err(|e| format!("Candidate JSON needs editing: {e}"))?;
    if candidate.nodes.is_empty() || candidate.nodes.len() > 20 || candidate.edges.len() > 80 {
        return Err("A proposal needs 1–20 nodes and at most 80 links.".into());
    }
    let request = &record.request;
    let create = request.mode == AuthoringMode::Create;
    let allowed_refs: BTreeSet<_> = request
        .context_ids
        .iter()
        .chain(&request.target_ids)
        .cloned()
        .chain(candidate.nodes.iter().map(|n| n.id.clone()))
        .collect();
    let mut seen = BTreeSet::new();
    let mut changes = vec![];
    let mut world = source.clone();
    if create {
        if !request.bundle && candidate.nodes.len() != 1 {
            return Err("This request permits one new node. Enable a connected group for related additions.".into());
        }
        if let Some(id) = &request.type_id {
            let root = candidate
                .nodes
                .iter()
                .find(|n| n.id == "new:root")
                .ok_or("Keep one new:root node for the requested type.")?;
            if &root.type_id != id {
                return Err("The root must keep the requested node type.".into());
            }
        }
    } else if !candidate.edges.is_empty() || candidate.nodes.len() != request.target_ids.len() {
        return Err("Revision must contain exactly the selected targets and no link edits.".into());
    }
    for node in &candidate.nodes {
        if !seen.insert(node.id.clone()) {
            return Err("Duplicate proposal node ID.".into());
        }
        if create {
            if request
                .group_type_id
                .as_ref()
                .is_some_and(|id| id != &node.type_id)
            {
                let label = source
                    .node_types
                    .iter()
                    .find(|t| Some(&t.id) == request.group_type_id.as_ref())
                    .map(|t| t.name.as_str())
                    .unwrap_or("selected");
                return Err(format!("This group permits only {label} nodes. Remove other node types or start a mixed-type group."));
            }
            if !alias(&node.id) {
                return Err("New nodes need unique new:<label> IDs.".into());
            }
            let preset = source
                .node_types
                .iter()
                .find(|d| d.id == node.type_id)
                .ok_or("The proposed node type is unknown.")?;
            if preset
                .components
                .iter()
                .any(|id| !node.components.contains_key(id))
            {
                return Err("A new node is missing a capability from its preset.".into());
            }
            record
                .allocations
                .entry(node.id.clone())
                .or_insert_with(|| uuid::Uuid::new_v4().to_string());
            changes.push(format!("Add {}.", node.name));
        } else {
            if !request.target_ids.contains(&node.id) {
                return Err("The proposal changes a node outside its selected scope.".into());
            }
            let old = source
                .nodes
                .iter()
                .find(|n| n.id == node.id)
                .ok_or("A revision target no longer exists.")?;
            if old.type_id != node.type_id
                || old.components.keys().collect::<Vec<_>>()
                    != node.components.keys().collect::<Vec<_>>()
            {
                return Err("Revision cannot add/remove capabilities or change node type. Add capabilities manually first.".into());
            }
            for (key, a, b) in [
                ("name", &old.name, &node.name),
                ("description", &old.description, &node.description),
                (
                    "visualDescription",
                    &old.visual_description,
                    &node.visual_description,
                ),
            ] {
                if a != b {
                    if !selected(request, &node.id, None, key) {
                        return Err(format!("{} is protected on {}.", key, old.name));
                    }
                    let label = match key {
                        "name" => "Name",
                        "description" => "Story notes",
                        _ => "Visible description",
                    };
                    changes.push(format!("Update {} / {label}.", old.name));
                }
            }
            for (component, values) in &node.components {
                let old_values = &old.components[component];
                let keys: BTreeSet<_> = values.keys().chain(old_values.keys()).collect();
                for key in keys {
                    if values.get(key) != old_values.get(key) {
                        if !selected(request, &node.id, Some(component), key) {
                            return Err(format!(
                                "{component} / {key} is protected on {}.",
                                old.name
                            ));
                        }
                        let definition = source.component_types.iter().find(|d| &d.id == component);
                        let component_label =
                            definition.map(|d| d.name.as_str()).unwrap_or(component);
                        let field_label = definition
                            .and_then(|d| d.fields.iter().find(|f| &f.key == key))
                            .map(|f| f.label.as_str())
                            .unwrap_or(key);
                        changes.push(format!(
                            "Update {} / {component_label} / {field_label}.",
                            old.name
                        ));
                    }
                }
            }
        }
    }
    for edge in &candidate.edges {
        if !alias(&edge.id)
            || !seen.insert(edge.id.clone())
            || !candidate.nodes.iter().any(|n| n.id == edge.source_id)
            || !allowed_refs.contains(&edge.target_id)
        {
            return Err("New links need unique new:<label> IDs, a new source and a supplied target/reference.".into());
        }
        record
            .allocations
            .entry(edge.id.clone())
            .or_insert_with(|| uuid::Uuid::new_v4().to_string());
    }
    let resolve = |id: &str| {
        record
            .allocations
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.into())
    };
    for mut node in candidate.nodes {
        let old = source.nodes.iter().find(|n| n.id == node.id);
        for (component, values) in &mut node.components {
            let def = source
                .component_types
                .iter()
                .find(|d| &d.id == component)
                .ok_or("Unknown capability in proposal.")?;
            for field in &def.fields {
                if let Some(value) = values.get_mut(&field.key) {
                    let previous = old
                        .and_then(|n| n.components.get(component))
                        .and_then(|v| v.get(&field.key));
                    match (&field.kind, &mut *value) {
                        (FieldKind::NodeRef, PropertyValue::Text(id)) => {
                            if previous != Some(&PropertyValue::Text(id.clone()))
                                && !allowed_refs.contains(id)
                            {
                                return Err(
                                    "A proposed reference is outside the selected context.".into(),
                                );
                            }
                            *id = resolve(id);
                        }
                        (FieldKind::NodeRefs, PropertyValue::NodeRefs(ids)) => {
                            if previous != Some(&PropertyValue::NodeRefs(ids.clone()))
                                && ids.iter().any(|id| !allowed_refs.contains(id))
                            {
                                return Err(
                                    "A proposed reference is outside the selected context.".into(),
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
        if create {
            world.nodes.push(node);
        } else {
            let target = world
                .nodes
                .iter_mut()
                .find(|n| n.id == node.id)
                .ok_or("Missing proposal target.")?;
            *target = node;
        }
    }
    for mut edge in candidate.edges {
        edge.id = resolve(&edge.id);
        edge.source_id = resolve(&edge.source_id);
        edge.target_id = resolve(&edge.target_id);
        changes.push(format!(
            "Connect {} → {}.",
            edge.type_id,
            world
                .nodes
                .iter()
                .find(|n| n.id == edge.target_id)
                .map(|n| n.name.as_str())
                .unwrap_or("reference")
        ));
        world.edges.push(edge);
    }
    world.validate()?;
    Ok((world, changes))
}

impl Project {
    fn authoring_path(&self, id: &str) -> Result<std::path::PathBuf, String> {
        validate_id(id)?;
        Ok(self
            .root_path()
            .join("proposals")
            .join(format!("{id}.json")))
    }
    fn write_authoring(&self, record: &AuthoringRecord) -> Result<(), String> {
        fs::create_dir_all(self.root_path().join("proposals")).map_err(|e| e.to_string())?;
        let bytes = serde_json::to_vec_pretty(record).map_err(|e| e.to_string())?;
        if bytes.len() > 16 * 1024 * 1024 {
            return Err("Proposal is too large to save.".into());
        }
        crate::store::atomic_write(&self.authoring_path(&record.id)?, &bytes)
    }
    fn read_authoring(&self, id: &str) -> Result<AuthoringRecord, String> {
        let record = self.read_authoring_record(id)?;
        if record.status == AuthoringStatus::Discarded {
            return Err("This AI draft was deleted.".into());
        }
        Ok(record)
    }
    fn read_authoring_record(&self, id: &str) -> Result<AuthoringRecord, String> {
        let mut record: AuthoringRecord =
            serde_json::from_slice(&crate::store::read_bytes(&self.authoring_path(id)?)?)
                .map_err(|e| format!("Invalid proposal file: {e}. It has been preserved."))?;
        if record.schema_version != 1 || record.id != id || record.story_id != self.view()?.story_id
        {
            return Err("The proposal does not belong to this story/version.".into());
        }
        if let Some(attempt) = &record.apply_attempt {
            if let Some(revision) =
                self.committed_revision_for(id, &attempt.title, &attempt.world_ref)?
            {
                if record.status != AuthoringStatus::Applied {
                    record.status = AuthoringStatus::Applied;
                    record.applied_revision = Some(revision);
                    record.version += 1;
                    self.write_authoring(&record)?;
                }
            }
        }
        Ok(record)
    }
    pub fn authoring_list(&self) -> Result<Vec<AuthoringSummary>, String> {
        let path = self.root_path().join("proposals");
        if !path.exists() {
            return Ok(vec![]);
        }
        let mut entries = vec![];
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.path().extension().is_some_and(|e| e == "json") {
                let id = entry
                    .path()
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or("Invalid proposal filename.")?
                    .to_string();
                let record = self.read_authoring_record(&id)?;
                if record.status == AuthoringStatus::Discarded {
                    continue;
                }
                entries.push(record);
                if entries.len() > 200 {
                    return Err(
                        "This prototype supports at most 200 saved AI proposals per story.".into(),
                    );
                }
            }
        }
        entries.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.id.cmp(&a.id))
        });
        Ok(entries
            .into_iter()
            .map(|r| AuthoringSummary {
                created_at: r.created_at.to_string(),
                id: r.id,
                version: r.version,
                status: r.status,
                direction: r.request.direction,
                provider_label: r.provider_label,
            })
            .collect())
    }
    pub fn delete_authoring_draft(&self, id: &str, expected: u32) -> Result<(), String> {
        let mut record = self.read_authoring_record(id)?;
        if record.status == AuthoringStatus::Discarded
            && record.version.checked_sub(1) == Some(expected)
        {
            return Ok(());
        }
        if matches!(
            record.status,
            AuthoringStatus::Running | AuthoringStatus::Applied | AuthoringStatus::Discarded
        ) {
            return Err("Only stopped, unapplied AI drafts can be deleted.".into());
        }
        if record.version != expected {
            return Err("This AI draft changed. Reload before deleting it.".into());
        }
        // One atomic replacement removes draft contents and retains an ID/version
        // tombstone so delayed saves and retries cannot recreate deleted work.
        record.status = AuthoringStatus::Discarded;
        record.version += 1;
        record.request = AuthoringRequest {
            mode: AuthoringMode::Create,
            direction: String::new(),
            type_id: None,
            bundle: false,
            creation_scope: None,
            group_type_id: None,
            target_ids: vec![],
            context_ids: vec![],
            fields: vec![],
        };
        record.source_world_ref = None;
        record.source_fingerprint.clear();
        record.provider_label.clear();
        record.raw_response.clear();
        record.candidate_text.clear();
        record.error.clear();
        record.allocations.clear();
        record.apply_attempt = None;
        record.incomplete = false;
        self.write_authoring(&record)
    }
    pub fn interrupt_authoring(&self) -> Result<(), String> {
        for summary in self.authoring_list()? {
            if summary.status == AuthoringStatus::Running {
                let mut record = self.read_authoring(&summary.id)?;
                record.status = AuthoringStatus::Interrupted;
                record.error =
                    "Generation was interrupted. It was not resent automatically.".into();
                record.version += 1;
                self.write_authoring(&record)?;
            }
        }
        Ok(())
    }
    pub fn save_authoring_draft(
        &self,
        id: &str,
        expected: u32,
        request: AuthoringRequest,
    ) -> Result<(), String> {
        bounded_request(&request)?;
        let path = self.authoring_path(id)?;
        let mut record = if path.exists() {
            let record = self.read_authoring(id)?;
            if record.status == AuthoringStatus::Draft
                && record.version == expected + 1
                && record.request == request
            {
                return Ok(());
            }
            if record.version != expected || record.status != AuthoringStatus::Draft {
                return Err("This proposal changed or already started. Open a new proposal to change its request.".into());
            }
            record
        } else {
            if expected != 0 || self.authoring_list()?.len() >= 200 {
                return Err("Proposal limit reached or draft version changed.".into());
            }
            AuthoringRecord {
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|_| "System clock is unavailable.")?
                    .as_millis() as u64,
                schema_version: 1,
                id: id.into(),
                story_id: self.view()?.story_id,
                version: 0,
                status: AuthoringStatus::Draft,
                request: request.clone(),
                source_world_ref: None,
                source_fingerprint: String::new(),
                provider_label: String::new(),
                raw_response: String::new(),
                candidate_text: String::new(),
                error: String::new(),
                allocations: BTreeMap::new(),
                applied_revision: None,
                apply_attempt: None,
                incomplete: false,
            }
        };
        record.request = request;
        record.version += 1;
        self.write_authoring(&record)
    }
    pub fn start_authoring(
        &self,
        id: &str,
        expected_version: u32,
        expected_revision: u32,
        settings: &LlmConnection,
    ) -> Result<PreparedAuthoring, String> {
        let mut record = self.read_authoring(id)?;
        let view = self.view()?;
        if record.status != AuthoringStatus::Draft
            || record.version != expected_version
            || view.revision != expected_revision
        {
            return Err("Save/reload the latest draft and world before generating.".into());
        }
        let prepared = prepare_authoring(&view.world, &record.request, settings)?;
        record.source_fingerprint = prepared.fingerprint.clone();
        record.source_world_ref = Some(view.world_ref);
        record.status = AuthoringStatus::Running;
        record.version += 1;
        record.provider_label = if settings.provider == TextProvider::Demo {
            "Offline demo · no model request".into()
        } else {
            settings.model.clone()
        };
        self.write_authoring(&record)?;
        Ok(prepared)
    }
    pub fn finish_authoring(
        &self,
        id: &str,
        response: Result<String, String>,
        cancelled: bool,
        incomplete: bool,
    ) -> Result<(), String> {
        let mut record = self.read_authoring(id)?;
        if record.status != AuthoringStatus::Running {
            return Err("The generation is no longer running.".into());
        }
        record.incomplete = incomplete;
        match response {
            Ok(text) => {
                if text.len() > 500000 {
                    return Err("Model output exceeds its saved-response limit.".into());
                }
                record.raw_response = text.clone();
                record.candidate_text = text;
                record.status = AuthoringStatus::Review;
                let source: World = self.read_record(
                    "worlds",
                    record
                        .source_world_ref
                        .as_ref()
                        .ok_or("Missing proposal source.")?,
                )?;
                record.error = candidate_world(&source, &mut record)
                    .err()
                    .unwrap_or_default();
            }
            Err(error) => {
                record.status = if cancelled {
                    AuthoringStatus::Cancelled
                } else {
                    AuthoringStatus::Failed
                };
                record.error = error;
            }
        }
        record.version += 1;
        self.write_authoring(&record)
    }
    pub fn save_authoring_candidate(
        &self,
        id: &str,
        expected: u32,
        text: String,
    ) -> Result<(), String> {
        if text.len() > 500000 {
            return Err("Candidate text exceeds its size limit.".into());
        }
        let mut record = self.read_authoring(id)?;
        if record.status == AuthoringStatus::Review
            && record.version == expected + 1
            && record.candidate_text == text
        {
            return Ok(());
        }
        if record.version != expected || record.status != AuthoringStatus::Review {
            return Err("The candidate changed. Reload before editing.".into());
        }
        if text != record.raw_response {
            record.incomplete = false;
        }
        record.candidate_text = text;
        record.apply_attempt = None;
        let source: World = self.read_record(
            "worlds",
            record
                .source_world_ref
                .as_ref()
                .ok_or("Missing proposal source.")?,
        )?;
        record.error = candidate_world(&source, &mut record)
            .err()
            .unwrap_or_default();
        record.version += 1;
        self.write_authoring(&record)
    }
    pub fn authoring_view(
        &self,
        id: &str,
        settings: &LlmConnection,
    ) -> Result<AuthoringView, String> {
        let mut record = self.read_authoring(id)?;
        let current = self.view()?;
        let source = if let Some(reference) = &record.source_world_ref {
            self.read_record("worlds", reference)?
        } else {
            current.world.clone()
        };
        let prepared = prepare_authoring(&source, &record.request, settings);
        let context = prepared
            .as_ref()
            .map(|p| p.context.clone())
            .unwrap_or_else(|_| {
                let mut context = source.clone();
                context.ai_context.clear();
                context.nodes.retain(|n| {
                    record.request.target_ids.contains(&n.id)
                        || record.request.context_ids.contains(&n.id)
                });
                context.edges.clear();
                context
            });
        let stale = !record.source_fingerprint.is_empty()
            && prepare_authoring(&current.world, &record.request, settings)
                .map_or(true, |p| p.fingerprint != record.source_fingerprint);
        let changes = if record.status == AuthoringStatus::Review {
            match candidate_world(&source, &mut record) {
                Ok((_, changes)) => changes,
                Err(error) => {
                    record.error = error;
                    vec![]
                }
            }
        } else {
            vec![]
        };
        if record.incomplete {
            record.error="Provider output was cut off. Edit and review the retained candidate before applying, or generate another candidate.".into();
        }
        let created_node_ids = if record.request.mode == AuthoringMode::Create {
            serde_json::from_str::<NodeCandidate>(&record.candidate_text)
                .ok()
                .map(|c| {
                    c.nodes
                        .iter()
                        .filter_map(|n| record.allocations.get(&n.id).cloned())
                        .collect()
                })
                .unwrap_or_default()
        } else {
            vec![]
        };
        let warnings = serde_json::from_str::<NodeCandidate>(&record.candidate_text)
            .map(|c| proposal_warnings(&c, &record.request))
            .unwrap_or_default();
        Ok(AuthoringView {
            id: record.id,
            story_id: record.story_id,
            version: record.version,
            status: record.status,
            request: record.request,
            provider_label: record.provider_label,
            raw_response: record.raw_response,
            candidate_text: record.candidate_text,
            error: record.error,
            stale,
            changes,
            warnings,
            context,
            estimated_input_tokens: prepared.map(|p| p.estimated_input_tokens).unwrap_or(0),
            created_node_ids,
            applied_revision: record.applied_revision,
        })
    }
    pub fn apply_authoring(
        &self,
        id: &str,
        expected: u32,
        settings: &LlmConnection,
    ) -> Result<ApplyAuthoringResult, String> {
        let mut record = self.read_authoring(id)?;
        if record.status == AuthoringStatus::Applied {
            return Ok(ApplyAuthoringResult {
                commit: CommitResult {
                    project: self.view()?,
                    committed_revision: record
                        .applied_revision
                        .ok_or("Missing applied revision.")?,
                    replayed: true,
                },
                proposal: self.authoring_view(id, settings)?,
            });
        }
        if record.version != expected
            || record.status != AuthoringStatus::Review
            || record.incomplete
        {
            return Err("The candidate changed, is incomplete or is not ready to apply.".into());
        }
        let current = self.view()?;
        let prepared = prepare_authoring(&current.world, &record.request, settings)?;
        if prepared.fingerprint != record.source_fingerprint {
            return Err("The proposal source changed. Create a new candidate from the latest world/settings.".into());
        }
        let (world, changes) = candidate_world(&current.world, &mut record)?;
        if changes.is_empty() {
            return Err("This candidate makes no changes.".into());
        }
        let world_ref = self.write_record("worlds", &world)?;
        record.apply_attempt = Some(ApplyAttempt {
            expected_revision: current.revision,
            title: current.title.clone(),
            world_ref,
        });
        record.version += 1;
        self.write_authoring(&record)?;
        let commit = self.commit(current.revision, id.into(), current.title, world)?;
        record.status = AuthoringStatus::Applied;
        record.applied_revision = Some(commit.committed_revision);
        record.version += 1;
        self.write_authoring(&record)?;
        Ok(ApplyAuthoringResult {
            commit,
            proposal: self.authoring_view(id, settings)?,
        })
    }
}

pub fn demo_authoring(world: &World, request: &AuthoringRequest) -> Result<String, String> {
    if request.mode == AuthoringMode::Create && request.creation_scope == Some(CreationScope::Group)
    {
        let preset = world
            .node_types
            .iter()
            .find(|d| {
                request.group_type_id.as_ref().map_or_else(
                    || d.components.iter().any(|id| id == "core.item"),
                    |id| id == &d.id,
                )
            })
            .or_else(|| world.node_types.first())
            .ok_or("No node preset is available.")?;
        let nodes = (1..=2)
            .map(|n| WorldNode {
                id: format!("new:group-{n}"),
                type_id: preset.id.clone(),
                name: format!("Demo {} {n}", preset.name),
                description: format!("Offline group example: {}", request.direction),
                visual_description: String::new(),
                components: preset
                    .components
                    .iter()
                    .map(|id| {
                        let fields = world
                            .component_types
                            .iter()
                            .find(|d| &d.id == id)
                            .map(|d| {
                                d.fields
                                    .iter()
                                    .map(|f| (f.key.clone(), f.default_value.clone()))
                                    .collect()
                            })
                            .unwrap_or_default();
                        (id.clone(), fields)
                    })
                    .collect(),
            })
            .collect();
        return serde_json::to_string_pretty(&NodeCandidate {
            nodes,
            edges: vec![WorldEdge {
                id: "new:group-link".into(),
                type_id: "core.related".into(),
                source_id: "new:group-1".into(),
                target_id: "new:group-2".into(),
                description: String::new(),
            }],
        })
        .map_err(|e| e.to_string());
    }
    let nodes = if request.mode == AuthoringMode::Create {
        let preset = world
            .node_types
            .iter()
            .find(|d| request.type_id.as_ref().is_none_or(|id| id == &d.id))
            .ok_or("No node preset is available.")?;
        let components = preset
            .components
            .iter()
            .map(|id| {
                let definition = world
                    .component_types
                    .iter()
                    .find(|d| &d.id == id)
                    .ok_or("Missing capability.")?;
                Ok((
                    id.clone(),
                    definition
                        .fields
                        .iter()
                        .map(|f| (f.key.clone(), f.default_value.clone()))
                        .collect(),
                ))
            })
            .collect::<Result<BTreeMap<_, _>, String>>()?;
        vec![WorldNode {
            id: "new:root".into(),
            type_id: preset.id.clone(),
            name: format!("Demo {}", preset.name),
            description: format!("Offline demo candidate: {}", request.direction),
            visual_description: String::new(),
            components,
        }]
    } else {
        let mut nodes: Vec<_> = world
            .nodes
            .iter()
            .filter(|n| request.target_ids.contains(&n.id))
            .cloned()
            .collect();
        for node in &mut nodes {
            let direction = if request.direction.trim().is_empty() {
                format!("Develop {}", node.name)
            } else {
                request.direction.clone()
            };
            for field in request.fields.iter().filter(|f| f.node_id == node.id) {
                if let Some(component) = &field.component_id {
                    let text_field = world
                        .component_types
                        .iter()
                        .find(|d| &d.id == component)
                        .and_then(|d| d.fields.iter().find(|f| f.key == field.key))
                        .is_some_and(|f| matches!(f.kind, FieldKind::Text | FieldKind::LongText));
                    if text_field {
                        if let Some(values) = node.components.get_mut(component) {
                            values.insert(
                                field.key.clone(),
                                PropertyValue::Text(format!("Demo variation: {direction}")),
                            );
                        }
                    }
                } else {
                    match field.key.as_str() {
                        "name" => {
                            node.name = format!("{} variation", node.name)
                                .chars()
                                .take(200)
                                .collect()
                        }
                        "description" => node.description = format!("Demo variation: {direction}"),
                        "visualDescription" => {
                            node.visual_description = format!("Demo visual variation: {direction}")
                        }
                        _ => (),
                    }
                }
            }
        }
        nodes
    };
    serde_json::to_string_pretty(&NodeCandidate {
        nodes,
        edges: vec![],
    })
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests;
