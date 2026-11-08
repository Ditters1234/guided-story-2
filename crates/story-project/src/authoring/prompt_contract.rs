//! Operation-specific guidance and checks over the actual submitted request.
use super::*;

pub(super) fn operation(request: &AuthoringRequest) -> Value {
    let (kind, task) = if request.mode == AuthoringMode::Revise {
        ("revision", "Develop only checked fields on the editable records. Blank checked fields and blank direction are valid. Return every selected node/link, preserving identity, type, capabilities, link endpoints and unchecked values. Target context bindings describe intended use and do not govern their own rewrite.")
    } else if request.type_id.is_some() {
        ("node", "Create the requested typed root using new:root. Include related records only when bundle is enabled. Develop useful specifics for this node's intended purpose.")
    } else if request.creation_scope == Some(CreationScope::Group) {
        ("group", "Create only the requested additions. Reuse supplied existing identities as read-only link targets. Do not invent a cast or setting to hold an item group. Empty additions are valid.")
    } else {
        ("world", "Develop the premise into a connected starting foundation with useful places, people and belongings. Preserve the author's explicit choices, scale and tone. Give the opening cast immediate goals and actual starting places.")
    };
    let output = if request.mode == AuthoringMode::Create && request.bundle {
        if creation::typed_bundle(request) {
            "root, preset-keyed nodes arrays, edges"
        } else {
            "preset-keyed nodes arrays, edges"
        }
    } else {
        "flat nodes array, edges array"
    };
    json!({"kind":kind,"task":task,"output":output})
}

pub(super) fn permissions(context: &World, request: &AuthoringRequest) -> Value {
    let nodes: Vec<_> = context
        .nodes
        .iter()
        .map(|node| {
            let editable = request.target_ids.contains(&node.id);
            let mut entry = json!({"id":node.id,"role":if editable{"editable"}else{"reference"}});
            if editable {
                entry["fields"] = json!(request
                    .fields
                    .iter()
                    .filter(|f| f.node_id == node.id)
                    .map(|f| json!({"componentId":f.component_id,"key":f.key}))
                    .collect::<Vec<_>>());
            }
            entry
        })
        .collect();
    let links: Vec<_> = context
        .edges
        .iter()
        .map(|link| {
            let editable = request.link_target_ids.contains(&link.id);
            let mut entry = json!({"id":link.id,"role":if editable{"editable"}else{"reference"}});
            if editable {
                entry["fields"] = json!(request
                    .link_fields
                    .iter()
                    .filter(|f| f.link_id == link.id)
                    .map(|f| &f.property)
                    .collect::<Vec<_>>());
            }
            entry
        })
        .collect();
    json!({"nodes":nodes,"links":links,"rule":"Only listed editable fields may change. All other records, values and indexed existingWorld entries are read-only. A null link field selects connection notes. Reference permission never authorizes existing-record changes."})
}

fn failure(detail: &str) -> String {
    format!("AI request assembly failed its preflight check: {detail}. The request was not sent.")
}

fn variants(schema: &Value) -> Vec<&Value> {
    schema["anyOf"]
        .as_array()
        .map_or_else(|| vec![schema], |v| v.iter().collect())
}

pub(super) fn lint(
    context: &World,
    model: &World,
    request: &AuthoringRequest,
    input: &Value,
    schema: &Value,
    references: &crate::prompt_context::ReferenceScope,
) -> Result<(), String> {
    if input["operation"] != operation(request)
        || input["recordPermissions"] != permissions(context, request)
    {
        return Err(failure(
            "operation or record permissions disagree with the selected request",
        ));
    }
    let mut declared: AuthoringRequest = serde_json::from_value(input["request"].clone())
        .map_err(|_| failure("invalid serialized request"))?;
    declared.direction.clone_from(&request.direction);
    if declared != *request {
        return Err(failure(
            "serialized request permissions disagree with the native request",
        ));
    }
    let records = input["records"]
        .as_array()
        .ok_or_else(|| failure("missing source records"))?;
    if records.len() != context.nodes.len() || input["links"] != json!(context.edges) {
        return Err(failure(
            "source context does not match the captured boundary",
        ));
    }
    for node in &context.nodes {
        let supplied: Vec<_> = records.iter().filter(|r| r["id"] == node.id).collect();
        let expected = if request.target_ids.contains(&node.id) {
            json!(node)
        } else {
            crate::prompt_context::reference_node(context, node)
        };
        if supplied.len() != 1 || *supplied[0] != expected {
            return Err(failure("a source record is missing, duplicated or changed"));
        }
    }
    if request.mode == AuthoringMode::Revise {
        if input.get("creationGuide").is_some() || input.get("clothingExample").is_some() {
            return Err(failure("revision includes creation guidance"));
        }
    }
    for output_schema in std::iter::once(schema).chain(input.get("responseSchema")) {
        lint_output(model, request, input, output_schema, references)?;
    }
    let mut reference_schema = output_schema(model, request);
    let used = references.authoring(&mut reference_schema, model);
    if !used.is_empty() {
        let guidance = references.guidance(
            &used,
            request.mode == AuthoringMode::Create && input.get("responseSchema").is_some(),
        );
        if input["referenceChoices"] != guidance {
            return Err(failure("reference hints disagree with the captured scope"));
        }
    } else if input.get("referenceChoices").is_some() {
        return Err(failure(
            "reference hints are present without editable reference fields",
        ));
    }
    if let Some(example) = input.get("clothingExample") {
        lint_clothing(model, example)?;
    }
    Ok(())
}

fn lint_output(
    model: &World,
    request: &AuthoringRequest,
    input: &Value,
    schema: &Value,
    references: &crate::prompt_context::ReferenceScope,
) -> Result<(), String> {
    if request.mode == AuthoringMode::Revise {
        // Reuse native protection to verify every unchecked value, endpoint and
        // actual capability in the submitted contract.
        let mut protected = output_schema(model, request);
        references.authoring(&mut protected, model);
        creation::strip_schema_help(&mut protected);
        if *schema != protected {
            return Err(failure(
                "revision output disagrees with native field protection",
            ));
        }
    } else {
        let mut original = schema.clone();
        creation::strip_schema_help(&mut original);
        let mut bound = original.clone();
        references.authoring(&mut bound, model);
        creation::strip_schema_help(&mut bound);
        if bound != original {
            return Err(failure(
                "reference field choices disagree with the captured scope",
            ));
        }
        if input["allowedNewTypeIds"]
            != json!(model.node_types.iter().map(|t| &t.id).collect::<Vec<_>>())
        {
            return Err(failure(
                "creation type permissions disagree with the scoped schema",
            ));
        }
        let nodes = &schema["properties"]["nodes"];
        let offered: BTreeSet<_> = if request.bundle {
            if nodes["type"] != "object" {
                return Err(failure("bundled creation requires preset groups"));
            }
            nodes["properties"]
                .as_object()
                .ok_or_else(|| failure("missing preset groups"))?
                .keys()
                .map(String::as_str)
                .collect()
        } else {
            if nodes["type"] != "array" {
                return Err(failure("single-node creation requires a flat array"));
            }
            variants(&nodes["items"])
                .into_iter()
                .flat_map(|v| {
                    let type_id = &v["properties"]["typeId"];
                    type_id["const"].as_str().into_iter().chain(
                        type_id["enum"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(Value::as_str),
                    )
                })
                .collect()
        };
        if offered != model.node_types.iter().map(|t| t.id.as_str()).collect() {
            return Err(failure("output presets disagree with creation permissions"));
        }
        if request.bundle {
            let required: BTreeSet<_> = nodes["required"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            if required != offered
                || nodes["properties"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .any(|(id, group)| {
                        group["items"]["properties"]["typeId"]["const"].as_str()
                            != Some(id.as_str())
                    })
            {
                return Err(failure(
                    "preset group requirements disagree with their record types",
                ));
            }
        }
        if creation::typed_bundle(request) {
            let root = &schema["properties"]["root"]["properties"];
            if root["id"]["const"] != "new:root"
                || root["typeId"]["const"] != json!(request.type_id)
                || !schema["required"]
                    .as_array()
                    .is_some_and(|v| v.contains(&json!("root")))
            {
                return Err(failure(
                    "typed-root output disagrees with the requested preset",
                ));
            }
        }
    }
    Ok(())
}

fn lint_clothing(model: &World, example: &Value) -> Result<(), String> {
    let records = example["records"]
        .as_array()
        .ok_or_else(|| failure("clothing example must be a record fragment"))?;
    let mut ids = BTreeSet::new();
    for value in records {
        let mut node = value.clone();
        node.as_object_mut()
            .ok_or_else(|| failure("invalid example record"))?
            .remove("capabilityFlags");
        let node: WorldNode =
            serde_json::from_value(node).map_err(|_| failure("invalid example node shape"))?;
        let preset = model
            .node_types
            .iter()
            .find(|p| p.id == node.type_id)
            .ok_or_else(|| failure("example uses an unavailable preset"))?;
        let offered = creation::offered_components(model, preset);
        if !alias(&node.id)
            || !ids.insert(node.id)
            || preset
                .components
                .iter()
                .any(|id| !node.components.contains_key(id))
            || node.components.keys().any(|id| !offered.contains(id))
        {
            return Err(failure(
                "example has duplicate IDs or omits a required capability",
            ));
        }
        for (id, fields) in &node.components {
            let definition = model
                .component_types
                .iter()
                .find(|d| &d.id == id)
                .ok_or_else(|| failure("example uses an undeclared capability"))?;
            if fields
                .keys()
                .any(|key| !definition.fields.iter().any(|f| &f.key == key))
            {
                return Err(failure("example uses an undeclared field"));
            }
        }
        let markers = creation::optional_markers(model, preset);
        let flags = value.get("capabilityFlags").and_then(Value::as_object);
        if markers.iter().any(|id| {
            flags
                .and_then(|f| f.get(id))
                .and_then(Value::as_bool)
                .is_none()
        }) || flags.is_some_and(|f| f.keys().any(|id| !markers.contains(id)))
        {
            return Err(failure("example omits a capability choice"));
        }
    }
    let edges: Vec<WorldEdge> = serde_json::from_value(example["edges"].clone())
        .map_err(|_| failure("invalid example links"))?;
    let mut placed = BTreeSet::new();
    for edge in edges {
        if !model.relation_types.iter().any(|r| r.id == edge.type_id)
            || !ids.contains(&edge.source_id)
            || edge.target_id != example["wearerReference"].as_str().unwrap_or_default()
            || !placed.insert(edge.source_id)
        {
            return Err(failure(
                "example links use unavailable types, IDs or conflicting placements",
            ));
        }
    }
    if placed.len() != ids.len() {
        return Err(failure("example omits a garment placement"));
    }
    Ok(())
}
