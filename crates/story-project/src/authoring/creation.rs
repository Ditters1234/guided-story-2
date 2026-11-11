//! Compact task guidance for useful connected records, independent of preset names.
use super::*;
pub(super) fn foundation_types(world: &World) -> Vec<String> {
    world
        .node_types
        .iter()
        .filter(|t| {
            t.components.iter().any(|c| {
                [
                    "core.place",
                    "core.character",
                    "core.item",
                    "core.narration-style",
                    "detail.rule",
                    "detail.brief",
                ]
                .contains(&c.as_str())
            })
        })
        .map(|t| t.id.clone())
        .collect()
}
pub(super) fn schema_world(world: &World, request: &AuthoringRequest) -> World {
    if request.mode != AuthoringMode::Create {
        return world.clone();
    }
    let unrestricted_catalogue =
        request.creation_type_ids.is_empty() && request.group_type_id.is_none() && request.bundle;
    let mut model = world.clone();
    model.node_types.retain(|t| {
        if let Some(id) = &request.group_type_id {
            &t.id == id
        } else if !request.creation_type_ids.is_empty() {
            request.creation_type_ids.contains(&t.id)
        } else {
            request.bundle || request.type_id.as_ref() == Some(&t.id)
        }
    });
    // Existing references need descriptors, but never become permissible new
    // relationship sources. Derive source capabilities only from offered nodes.
    let mut source_ids: BTreeSet<_> = model
        .node_types
        .iter()
        .flat_map(|t| t.components.iter().cloned())
        .collect();
    if source_ids.contains("core.item") {
        source_ids.extend([
            "core.container".into(),
            "core.wearable".into(),
            "detail.wearable".into(),
        ]);
    }
    if source_ids.contains("core.container") {
        // The native capability resolver treats every container as a holder.
        source_ids.insert("core.holder".into());
    }
    if source_ids.contains("core.character") {
        source_ids.insert("detail.anatomy".into());
    }
    let mut descriptor_ids = source_ids.clone();
    descriptor_ids.extend(
        world
            .nodes
            .iter()
            .flat_map(|n| n.components.keys().cloned()),
    );
    if !unrestricted_catalogue {
        model
            .component_types
            .retain(|d| descriptor_ids.contains(&d.id));
    }
    model.relation_types.retain(|r| {
        r.source_capability
            .as_ref()
            .is_none_or(|id| source_ids.contains(id))
    });
    model
}

/// JSON/prompt-mode servers need an output shape, not a second copy of every
/// field contract. Schema mode still receives the full generated JSON Schema;
/// the complete saved schema always owns native candidate validation.
pub(super) fn prompt_schema(world: &World, request: &AuthoringRequest) -> Value {
    let mut schema = json!({"type":"object","required":["nodes","edges"],"additionalProperties":false,"properties":{
        "nodes":{"type":"array","minItems":if request.creation_scope==Some(CreationScope::Group){0}else{1},"maxItems":if request.bundle{20}else{1},"items":{"type":"object","required":["id","typeId","name","description","visualDescription","components"],"additionalProperties":false,"properties":{
            "id":if !request.bundle&&request.type_id.is_some(){json!({"const":"new:root"})}else{json!({"type":"string"})},"typeId":{"enum":world.node_types.iter().map(|t|&t.id).collect::<Vec<_>>()},"name":{"type":"string"},"description":{"type":"string"},"visualDescription":{"type":"string"},"components":{"type":"object","propertyNames":{"enum":world.component_types.iter().map(|c|&c.id).collect::<Vec<_>>()},"additionalProperties":{"type":"object"},"description":"Use exact capability and field keys from capabilities. Include every capability listed by the chosen nodeType; unused optional fields may be omitted and unused preset capabilities may be empty objects. Required fields, kinds and choices are defined in capabilities."}
        }}},
        "edges":{"type":"array","maxItems":80,"items":{"type":"object","required":["id","typeId","sourceId","targetId","description"],"additionalProperties":false,"properties":{"id":{"type":"string"},"typeId":{"enum":world.relation_types.iter().map(|r|&r.id).collect::<Vec<_>>()},"sourceId":{"type":"string"},"targetId":{"type":"string"},"description":{"type":"string"},"properties":{"type":"object","description":"Only declared relationship field keys, kinds and required values from relationships."}}}}
    }});
    schema["properties"]["nodes"]["items"]["properties"]["id"] =
        if !request.bundle && request.type_id.is_some() {
            json!({"const":"new:root"})
        } else {
            new_alias_schema()
        };
    schema["properties"]["edges"]["items"]["properties"]["id"] = new_alias_schema();
    schema["properties"]["edges"]["items"]["properties"]["sourceId"] = new_alias_schema();
    let markers: BTreeSet<_> = world
        .node_types
        .iter()
        .flat_map(|t| optional_markers(world, t))
        .collect();
    if !markers.is_empty() {
        schema["properties"]["nodes"]["items"]["properties"]["capabilityFlags"] = json!({"type":"object","propertyNames":{"enum":markers},"additionalProperties":{"type":"boolean"},"description":"For each offered optional marker in creationGuide.capabilityChoices for this type, choose true to attach it or false to omit it. Do not repeat that marker in components."});
    }
    if typed_bundle(request) {
        let mut root = schema["properties"]["nodes"]["items"].clone();
        root["properties"]["id"] = json!({"const":"new:root"});
        root["properties"]["typeId"] = json!({"const":request.type_id});
        root["properties"]["components"]["required"] = json!(world
            .node_types
            .iter()
            .find(|t| Some(&t.id) == request.type_id.as_ref())
            .map(|t| &t.components));
        root["properties"]
            .as_object_mut()
            .unwrap()
            .remove("capabilityFlags");
        if let Some(preset) = world
            .node_types
            .iter()
            .find(|t| Some(&t.id) == request.type_id.as_ref())
        {
            add_marker_flags(&mut root, &optional_markers(world, preset));
        }
        add_root_property(&mut schema, root);
    }
    if request.bundle {
        let template = schema["properties"]["nodes"]["items"].clone();
        let variants: Vec<_> = world
            .node_types
            .iter()
            .map(|preset| {
                let mut node = template.clone();
                node["properties"]["typeId"] = json!({"const":preset.id});
                node["properties"]["components"]["required"] = json!(preset.components);
                node["properties"]["components"]["propertyNames"] =
                    json!({"enum":offered_components(world,preset)});
                node["properties"]["components"]
                    .as_object_mut()
                    .unwrap()
                    .remove("description");
                node["properties"]
                    .as_object_mut()
                    .unwrap()
                    .remove("capabilityFlags");
                add_marker_flags(&mut node, &optional_markers(world, preset));
                node
            })
            .collect();
        group_related_nodes(&mut schema, variants);
    }
    schema
}
pub(super) fn new_alias_schema() -> Value {
    json!({"type":"string","pattern":"^new:[A-Za-z0-9_-]{1,40}$"})
}
pub(super) fn typed_bundle(request: &AuthoringRequest) -> bool {
    request.mode == AuthoringMode::Create && request.type_id.is_some() && request.bundle
}
pub(super) fn optional_markers(world: &World, preset: &NodeTypeDefinition) -> Vec<String> {
    let allowed = offered_components(world, preset);
    world
        .component_types
        .iter()
        .filter(|d| {
            allowed.contains(&d.id) && d.fields.is_empty() && !preset.components.contains(&d.id)
        })
        .map(|d| d.id.clone())
        .collect()
}
pub(crate) fn offered_components(world: &World, preset: &NodeTypeDefinition) -> Vec<String> {
    let mut allowed = BTreeSet::new();
    allowed.extend(preset.components.iter().map(String::as_str));
    if preset.components.iter().any(|id| id == "core.item") {
        allowed.extend(["core.container", "core.wearable", "detail.wearable"]);
    }
    if preset.components.iter().any(|id| id == "core.character") {
        allowed.insert("detail.anatomy");
    }
    world
        .component_types
        .iter()
        .filter(|d| allowed.contains(d.id.as_str()))
        .map(|d| d.id.clone())
        .collect()
}
pub(super) fn add_marker_flags(schema: &mut Value, markers: &[String]) {
    if markers.is_empty() {
        return;
    }
    let properties: serde_json::Map<_, _> = markers
        .iter()
        .map(|id| (id.clone(), json!({"type":"boolean"})))
        .collect();
    schema["properties"]["capabilityFlags"] = json!({"type":"object","properties":properties,"required":markers,"additionalProperties":false});
    schema["required"]
        .as_array_mut()
        .unwrap()
        .push(json!("capabilityFlags"));
    if let Some(properties) = schema["properties"]["components"]
        .get_mut("properties")
        .and_then(Value::as_object_mut)
    {
        for id in markers {
            properties.remove(id);
        }
    }
}
pub(super) fn add_root_property(schema: &mut Value, root: Value) {
    schema["properties"]["root"] = root;
    schema["required"] = json!(["root", "nodes", "edges"]);
    schema["properties"]["nodes"]["minItems"] = json!(0);
    schema["properties"]["nodes"]["maxItems"] = json!(19);
}
/// Choosing a type key before writing its record avoids an ambiguous component
/// union in constrained decoders. This is a provider contract, not persistence.
pub(super) fn group_related_nodes(schema: &mut Value, variants: Vec<Value>) {
    let limit = if schema["properties"].get("root").is_some() {
        19
    } else {
        20
    };
    let groups: serde_json::Map<_, _> = variants
        .into_iter()
        .filter_map(|node| {
            let id = node["properties"]["typeId"]["const"].as_str()?.to_owned();
            Some((id, json!({"type":"array","maxItems":limit,"items":node})))
        })
        .collect();
    let required: Vec<_> = groups.keys().cloned().collect();
    schema["properties"]["nodes"] = json!({"type":"object","properties":groups,"required":required,"additionalProperties":false});
}

/// Small foundation responses develop a few useful details per capability.
/// This limits only the provider contract; manual proposals retain native limits.
pub(super) fn limit_foundation_fields(
    schema: &mut Value,
    world: &World,
    request: &AuthoringRequest,
    tokens: u32,
) {
    if request.mode != AuthoringMode::Create
        || !request.bundle
        || request.type_id.is_some()
        || request.creation_scope == Some(CreationScope::Group)
        || tokens > 4096
    {
        return;
    }
    fn limit_text(schema: &mut Value, limit: u64) {
        let Some(object) = schema.as_object_mut() else {
            return;
        };
        let string = object.get("type").is_some_and(|kind| {
            kind == "string"
                || kind
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind == "string"))
        });
        // Arbitrary patterns can require a minimum longer than the budget.
        // Keep those branches intact rather than make their grammar impossible.
        if string && !object.contains_key("pattern") {
            let minimum = object.get("minLength").and_then(Value::as_u64).unwrap_or(0);
            let fixed = object
                .get("const")
                .and_then(Value::as_str)
                .map(|value| value.chars().count() as u64)
                .unwrap_or(0);
            let choices = object
                .get("enum")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(|value| value.chars().count() as u64)
                .max()
                .unwrap_or(0);
            let bound = limit.max(minimum).max(fixed).max(choices);
            let previous = object
                .get("maxLength")
                .and_then(Value::as_u64)
                .unwrap_or(bound);
            object.insert("maxLength".into(), json!(previous.min(bound)));
        }
        for keyword in ["anyOf", "oneOf", "allOf"] {
            if let Some(branches) = object.get_mut(keyword).and_then(Value::as_array_mut) {
                for branch in branches {
                    limit_text(branch, limit);
                }
            }
        }
    }
    let Some(groups) = schema["properties"]["nodes"]["properties"].as_object_mut() else {
        return;
    };
    for group in groups.values_mut() {
        let node = &mut group["items"];
        for (key, bound) in [
            ("name", 120),
            ("description", 160),
            ("visualDescription", 160),
        ] {
            if let Some(field) = node["properties"].get_mut(key) {
                limit_text(field, bound);
            }
        }
        let Some(components) = node["properties"]["components"]["properties"].as_object_mut()
        else {
            continue;
        };
        for (id, component) in components {
            let Some(definition) = world.component_types.iter().find(|d| &d.id == id) else {
                continue;
            };
            let retained = definition
                .fields
                .iter()
                .filter(|field| field.required || field.default_value != PropertyValue::Null)
                .count();
            let maximum = (retained + 1).max(2).min(definition.fields.len());
            component["maxProperties"] = json!(maximum);
            for field in &definition.fields {
                if matches!(field.kind, FieldKind::Text | FieldKind::LongText)
                    && !crate::story_effects::planning_only(field)
                {
                    if let Some(value) = component["properties"].get_mut(&field.key) {
                        limit_text(value, 120);
                    }
                }
            }
        }
    }
}

/// Provider-only root/marker declarations become ordinary reviewable records.
/// Never infer a capability from a name, description or relationship.
pub(super) fn materialize_text(
    world: &World,
    request: &AuthoringRequest,
    text: &str,
) -> Result<String, String> {
    if selection_revision::enabled(request) {
        return selection_revision::materialize(world, request, text);
    }
    if request.mode != AuthoringMode::Create {
        return Ok(text.into());
    }
    let mut value: Value =
        serde_json::from_str(text).map_err(|e| format!("Candidate JSON needs editing: {e}"))?;
    let rooted = typed_bundle(request) && value.get("root").is_some();
    let mut changed = rooted;
    if request.bundle && value.get("nodes").is_some_and(Value::is_object) {
        if typed_bundle(request) && !rooted {
            return Err("A grouped typed bundle needs its separate root record.".into());
        }
        let mut nodes = vec![];
        for (type_id, group) in value["nodes"].as_object().unwrap() {
            if !world.node_types.iter().any(|t| &t.id == type_id) {
                return Err(format!("Unknown related-node group {type_id}."));
            }
            let group = group
                .as_array()
                .ok_or("Each related-node type group must be an array.")?;
            for node in group {
                if node["typeId"].as_str() != Some(type_id.as_str()) {
                    return Err(format!(
                        "A related node does not match its {type_id} group."
                    ));
                }
                nodes.push(node.clone());
            }
        }
        value["nodes"] = json!(nodes);
        changed = true;
    }
    fn expand(world: &World, node: &mut Value) -> Result<bool, String> {
        let Some(flags) = node.get("capabilityFlags") else {
            return Ok(false);
        };
        let flags = flags
            .as_object()
            .ok_or("capabilityFlags must contain explicit true/false choices.")?
            .clone();
        let preset = world
            .node_types
            .iter()
            .find(|t| Some(t.id.as_str()) == node["typeId"].as_str())
            .ok_or("Capability choices need a known node type.")?;
        let markers = optional_markers(world, preset);
        let components = node
            .get_mut("components")
            .and_then(Value::as_object_mut)
            .ok_or("Capability choices need a components object.")?;
        for (id, enabled) in flags {
            if !markers.contains(&id) {
                return Err(format!(
                    "{id} is not an offered optional marker for this node type."
                ));
            }
            let enabled = enabled
                .as_bool()
                .ok_or("Capability choices must be true or false.")?;
            if enabled {
                if components.get(&id).is_some_and(|v| v != &json!({})) {
                    return Err(format!("The {id} marker must have no fields."));
                }
                components.insert(id, json!({}));
            } else if components.contains_key(&id) {
                return Err(format!(
                    "The {id} capability contradicts its false choice. Remove it or choose true."
                ));
            }
        }
        node.as_object_mut().unwrap().remove("capabilityFlags");
        Ok(true)
    }
    if rooted {
        changed |= expand(world, &mut value["root"])?;
    }
    if let Some(nodes) = value.get_mut("nodes").and_then(Value::as_array_mut) {
        for node in nodes {
            changed |= expand(world, node)?;
        }
    }
    if !changed {
        return Ok(text.into());
    }
    if !rooted {
        // Strict decoding retains the usual rejection of foreign fields.
        let candidate: NodeCandidate = serde_json::from_value(value)
            .map_err(|e| format!("Candidate JSON needs editing: {e}"))?;
        return serde_json::to_string_pretty(&candidate).map_err(|e| e.to_string());
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Packet {
        root: WorldNode,
        nodes: Vec<WorldNode>,
        edges: Vec<WorldEdge>,
    }
    let mut packet: Packet = serde_json::from_value(value)
        .map_err(|e| format!("Typed bundle needs a complete root, related nodes and edges: {e}"))?;
    if packet.root.id != "new:root" || Some(&packet.root.type_id) != request.type_id.as_ref() {
        return Err("The typed bundle root must use new:root and the requested node type.".into());
    }
    packet.nodes.insert(0, packet.root);
    serde_json::to_string_pretty(&NodeCandidate {
        nodes: packet.nodes,
        edges: packet.edges,
    })
    .map_err(|e| e.to_string())
}
pub(super) fn strip_schema_help(v: &mut Value) {
    match v {
        Value::Object(m) => {
            if m.get("type").is_some_and(|v| v.is_string() || v.is_array())
                || m.get("anyOf").is_some_and(Value::is_array)
                || m.get("allOf").is_some_and(Value::is_array)
            {
                m.remove("description");
            }
            for v in m.values_mut() {
                strip_schema_help(v);
            }
            if let Some(options) = m.get("anyOf").and_then(Value::as_array) {
                if options.len() == 2
                    && options[1] == json!({"type":"null"})
                    && options[0].get("type").is_some()
                {
                    let mut simple = options[0].clone();
                    let mut types = match &simple["type"] {
                        Value::String(s) => vec![json!(s)],
                        Value::Array(a) => a.clone(),
                        _ => vec![],
                    };
                    if !types.contains(&json!("null")) {
                        types.push(json!("null"));
                    }
                    simple["type"] = json!(types);
                    if let Some(values) = simple.get_mut("enum").and_then(Value::as_array_mut) {
                        if !values.contains(&Value::Null) {
                            values.push(Value::Null);
                        }
                    }
                    *v = simple;
                }
            }
        }
        Value::Array(a) => {
            for v in a {
                strip_schema_help(v);
            }
        }
        _ => (),
    }
}
pub(super) fn budget(world: &World, request: &AuthoringRequest, tokens: u32) -> Value {
    let typed = request.type_id.as_ref().or(request.group_type_id.as_ref());
    // Optional profile fields are a menu, not a bill for filling every field on
    // every support item. Mixed worlds need room for places, people AND inventory.
    let node_cost = typed
        .and_then(|id| world.node_types.iter().find(|t| &t.id == id))
        .map(|t| {
            let fields: Vec<_> = t
                .components
                .iter()
                .filter_map(|id| world.component_types.iter().find(|d| &d.id == id))
                .flat_map(|d| &d.fields)
                .collect();
            180 + fields.iter().filter(|f| f.required).count() as u32 * 20
                + (fields.len().min(8) as u32) * 10
        })
        .unwrap_or(240);
    let mut value = json!({"outputTokens":tokens,"suggestedMaxNewNodes":if request.mode==AuthoringMode::Create&&!request.bundle{1}else{(tokens.saturating_sub(512)/node_cost).clamp(1,20)},"nodeLimitIsAdvisory":true,"priority":"Finish all requested places, cast, inventory, narration and links within outputTokens. Use concise specifics and sparse optional fields; avoid repetition."});
    if request.mode == AuthoringMode::Create
        && request.type_id.is_none()
        && request.creation_scope != Some(CreationScope::Group)
    {
        value["foundationDetailAllowance"] = json!({"characterWords":80,"placeWords":40,"itemWords":20,"narrationWords":60,"instruction":"Approximate TOTAL words per record: specific motives, contradictions, voice, appearance; usable places; recognisable items. Preserve explicit choices. Finish cast, places, clothing and narration before extras; deepen details in later revisions."});
    }
    value
}
pub(super) fn placement_guide(world: &World) -> Value {
    let mut guide = json!({"rule":"For each new record, use its ID as sourceId of at most ONE physical edge. The source is the thing being placed, and targetId is its immediate holder or parent. A holder can be the target of many edges. Check all physical sourceIds before returning."});
    if world
        .relation_types
        .iter()
        .any(|relation| relation.id == "core.inside")
    {
        guide["insideExamples"] = json!([
            {"typeId":"core.inside","sourceId":"new:house","targetId":"new:homestead"},
            {"typeId":"core.inside","sourceId":"new:barn","targetId":"new:homestead"},
            {"typeId":"core.inside","sourceId":"new:yard","targetId":"new:homestead"}
        ]);
    }
    if world
        .relation_types
        .iter()
        .any(|relation| relation.id == "core.worn-by")
    {
        guide["clothing"] = json!("Each garment is a separate sourceId. Worn garments use one worn-by edge pointing to their wearer. Several different garments can point to the same wearer. A stored-in edge is only for an item that is carried or stored, never an additional link for an already worn garment.");
    }
    guide
}

pub(super) fn starting_situation_guide() -> Value {
    json!({
        "premise": "Match the premise's scale and tone. Preserve named people, places, relationships and constraints. For a short premise, develop one specific local situation; ordinary friction is enough.",
        "firstAction": "Give the main characters concrete immediate objectives and link their actual starting places with present-at when applicable. Show why their choices matter to each other. Keep outcomes open.",
        "usableWorld": "Give key places a feature or obstacle that matters to an immediate objective. When a usable or transferable object matters to that objective or the premise, make it a separate item with its condition and a storage or wearing link as appropriate. Keep fixed environmental features in place fields.",
        "connections": "Use a few meaningful links from the supplied catalogue, with concrete details when supported. For example, a courier needs to cross before the quay steps flood. The skiff key is stored with the ferryman, who owes the courier one crossing. Store the key, the access constraint and the obligation on the matching records and links. Adapt the pattern to the author's premise.",
        "budget": "Reserve space for premise-critical items and links alongside distinct scene places, cast, normal outfits and requested narration. If space is tight, trim decorative details and optional records first. Use only the offered types; carry necessary facts in the closest supported fields when a type is unavailable. Do not write the passage or present intended actions as completed."
    })
}

pub(super) fn guide(world: &World, request: &AuthoringRequest) -> Value {
    let is_world =
        request.type_id.is_none() && request.creation_scope != Some(CreationScope::Group);
    let allowed: Vec<_> = world
        .node_types
        .iter()
        .filter(|t| request.group_type_id.as_ref().is_none_or(|id| &t.id == id))
        .collect();
    let places: Vec<_> = allowed
        .iter()
        .filter(|t| t.components.iter().any(|c| c == "core.place"))
        .map(|t| &t.id)
        .collect();
    let mut v = json!({"intent":if is_world{"world foundation"}else{"only the requested node/group"},"detail":"Useful specifics, not generic summaries: important people need differing wants, pressures, behavior/voice and visible identity; places need distinct uses, layout/access and usable resources or constraints; items need function, material/design and relevant condition. Use the actual project field keys. Invent coherent specifics within the author's scope; do not substitute a list of adjectives for development.","scope":"Do not create unrelated characters, locations or guidance profiles for a requested item group. Reuse supplied existing targets. Do not duplicate established records."});
    v["people"] = json!("Give requested people personal names unless anonymity is intentional. Age/role labels belong in profile fields. Man/woman denotes an adult; younger does not imply a minor. Distinguish long-term motives from current intent. Preserve explicit appearance choices.");
    if request.bundle {
        v["nodeGroups"]=json!("Return every offered preset-keyed array, [] when unused. Each record must match its group type. At most 20 new records including any root. Every new alias referenced by a link must have exactly one created record. Existing supplied IDs are read-only targets, never copied additions. Use wearable presets for garments and item presets for supplies.");
    }
    let choices: BTreeMap<_, _> = allowed
        .iter()
        .filter_map(|t| {
            let markers = optional_markers(world, t);
            (!markers.is_empty()).then_some((&t.id, markers))
        })
        .collect();
    if !choices.is_empty() {
        v["capabilityChoices"] = json!(choices);
        v["capabilityChoiceRule"]=json!("Set every offered capabilityFlags marker explicitly: true attaches it, false omits it. A bag holding contents needs core.container:true. Flags stay outside components; required preset capabilities stay inside components.");
    }
    if typed_bundle(request) {
        v["typedRoot"] = json!({"id":"new:root","typeId":request.type_id,"rule":"Put the main record only in root, with at most 19 related records in nodes. Never duplicate root in a group. Every reference to the main record uses new:root."});
    }
    if request.bundle
        && world
            .node_types
            .iter()
            .any(|t| t.components.iter().any(|c| c == "core.item"))
        && world
            .component_types
            .iter()
            .any(|d| d.id == "core.container")
    {
        v["containers"] = json!({"requiredWhen":"An item holds other items through stored-in links.","mustInclude":{"components":{"core.item":{"quantity":1,"unit":"item"}},"capabilityFlags":{"core.container":true}},"rule":"Attach core.container via its offered flag, or components when preset-required. A name/description cannot supply this capability. Contents each link to the container; it has one placement at its owner/place. Ordinary items holding nothing need no container marker."});
    }
    if !places.is_empty() {
        v["placeStructure"] = json!({"placeTypeIds":places,"rule":"Represent distinct buildings, grounds and useful scene locations as separate linked places. A homestead containing a house, barn and cultivated field needs a parent homestead and child house, barn and field, not one paragraph containing all four. Add important interiors when requested or needed for scenes; do not turn every object or corner into a place. In a group, split only requested locations; do not invent another entire world.","containment":world.relation_types.iter().find(|r|r.physical_placement&&r.source_capability.as_deref()==Some("core.place")&&r.target_capability.as_deref()==Some("core.place")).map(|r|json!({"typeId":r.id,"source":"new:house","target":"new:homestead"}))});
    }
    if is_world {
        v["foundationTypeIds"] = json!(foundation_types(world));
        v["narration"] = json!({"requested":request.include_narration==Some(true),"presetIds":world.node_types.iter().filter(|t|t.components.iter().any(|c|c=="core.narration-style")).map(|t|&t.id).collect::<Vec<_>>(),"rule":"When requested, create a narration-style node with person, tense and several concrete instructions suited to this premise: viewpoint discipline, tone, pace, dialogue and descriptive focus. Respect explicit user choices; otherwise third person and past tense are a starting point. Apply will include the proposed style for passage writing/wording. This is guidance, not an accomplished event."});
    }
    if request.bundle
        && allowed
            .iter()
            .any(|t| t.components.iter().any(|c| c == "core.character"))
        && allowed
            .iter()
            .any(|t| t.components.iter().any(|c| c == "core.item"))
    {
        v["startingOutfits"] = json!({"preferredTypeIds":world.node_types.iter().filter(|t|t.components.iter().any(|c|c=="core.wearable")).map(|t|&t.id).collect::<Vec<_>>(),"rule":"For each normally clothed humanoid you are asked to create, provide separate items covering upper body, lower body and feet unless explicitly directed otherwise. Usually use shirt, trousers and shoes/boots; a one-piece garment can replace top and trousers. Shirt plus coat leaves the lower body uncovered. Prefer offered wearable presets; a plain item needs an explicit core.wearable component when no wearable preset is permitted. Reserve garment records and links alongside requested places and narration. Clothing prose cannot replace item records."});
    }
    v
}
pub(super) fn include_narration(
    world: &mut World,
    source: &World,
    request: &AuthoringRequest,
    changes: &mut Vec<String>,
) -> Result<(), String> {
    if request.include_narration != Some(true) {
        return Ok(());
    }
    if !world.nodes.iter().any(|n| {
        n.components.contains_key("core.narration-style")
            && !source.nodes.iter().any(|old| old.id == n.id)
    }) {
        return Err("Narration style was requested but is missing. Add a proposed Narration style node with instructions, or start a new candidate with narration creation turned off.".into());
    }
    let mut order = world
        .ai_context
        .iter()
        .map(|b| b.order)
        .max()
        .unwrap_or(-10);
    for node in world.nodes.iter().filter(|n| {
        n.components.contains_key("core.narration-style")
            && !source.nodes.iter().any(|old| old.id == n.id)
    }) {
        if !matches!(node.components["core.narration-style"].get("instructions"), Some(PropertyValue::Text(text)) if !text.trim().is_empty())
        {
            return Err(format!("{} needs narration instructions. Edit the proposed Narration style node; person and tense alone do not complete the requested guidance.",node.name));
        }
        order = (order + 10).min(10000);
        world.ai_context.push(AiContextBinding {
            node_id: node.id.clone(),
            enabled: true,
            role: AiContextRole::Instructions,
            tasks: vec![TextPromptTask::Passage, TextPromptTask::Wording],
            fields: vec![AiContextField::Description, AiContextField::Components],
            order,
            image_preparation: false,
        });
        changes.push(format!(
            "Include {} as instructions for passage writing and wording edits.",
            node.name
        ));
    }
    if world.ai_context.len() > 32 {
        return Err("Narration inclusion exceeds 32 context nodes. Remove an inclusion or start a world proposal without narration guidance.".into());
    }
    // Catch an empty proposed instruction source before publishing it.
    let mut check = world.clone();
    check
        .ai_context
        .retain(|b| !source.nodes.iter().any(|n| n.id == b.node_id));
    resolve_ai_context(&check, TextPromptTask::Passage)?;
    resolve_ai_context(&check, TextPromptTask::Wording)?;
    Ok(())
}

pub(super) fn demo_world(world: &World, request: &AuthoringRequest) -> Option<NodeCandidate> {
    if request.mode != AuthoringMode::Create
        || request.creation_scope != Some(CreationScope::World)
        || request.include_narration != Some(true)
    {
        return None;
    }
    let place = world
        .node_types
        .iter()
        .filter(|t| {
            request.creation_type_ids.is_empty() || request.creation_type_ids.contains(&t.id)
        })
        .find(|t| t.components.iter().any(|c| c == "core.place"))?;
    let style = world
        .node_types
        .iter()
        .filter(|t| {
            request.creation_type_ids.is_empty() || request.creation_type_ids.contains(&t.id)
        })
        .find(|t| t.components.iter().any(|c| c == "core.narration-style"))?;
    let make = |preset: &NodeTypeDefinition, id: &str, name: &str| WorldNode {
        id: id.into(),
        type_id: preset.id.clone(),
        name: name.into(),
        description: String::new(),
        visual_description: String::new(),
        components: preset
            .components
            .iter()
            .filter_map(|id| {
                world
                    .component_types
                    .iter()
                    .find(|d| &d.id == id)
                    .map(|d| (id.clone(), default_properties(d)))
            })
            .collect(),
    };
    let mut nodes: Vec<_> = [
        (
            "new:root",
            "Demo homestead",
            "A small working holding beside the forest.",
        ),
        (
            "new:house",
            "Demo house",
            "Shelter, meals and household work.",
        ),
        (
            "new:barn",
            "Demo barn",
            "Dry tool storage and repairs, with a draughty upper loft.",
        ),
        (
            "new:field",
            "Demo cultivated field",
            "Food growing on a sheltered strip beside the house.",
        ),
    ]
    .into_iter()
    .map(|(id, name, purpose)| {
        let mut n = make(place, id, name);
        n.description = purpose.into();
        n
    })
    .collect();
    let mut narration = make(style, "new:narration", "Demo narration style");
    if let Some(fields) = narration.components.get_mut("core.narration-style") {
        fields.insert("instructions".into(),PropertyValue::Text("Use close third person and past tense. Ground scenes in practical work and specific surroundings. Let differing priorities emerge through action and distinct dialogue. Keep viewpoint knowledge consistent; avoid summarising developments before they occur.".into()));
    }
    nodes.push(narration);
    let inside = world.relation_types.iter().find(|r| {
        r.physical_placement
            && r.source_capability.as_deref() == Some("core.place")
            && r.target_capability.as_deref() == Some("core.place")
    })?;
    let edges = ["house", "barn", "field"]
        .into_iter()
        .map(|id| WorldEdge {
            id: format!("new:{id}-inside"),
            type_id: inside.id.clone(),
            source_id: format!("new:{id}"),
            target_id: "new:root".into(),
            description: String::new(),
            properties: default_field_values(&inside.fields),
        })
        .collect();
    Some(NodeCandidate { nodes, edges })
}
