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
    if request.creation_type_ids.is_empty() && request.group_type_id.is_none() && request.bundle {
        return world.clone();
    }
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
    let mut ids: BTreeSet<_> = model
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
    if ids.contains("core.item") {
        ids.extend([
            "core.container".into(),
            "core.wearable".into(),
            "detail.wearable".into(),
        ]);
    }
    if ids.contains("core.character") {
        ids.insert("detail.anatomy".into());
    }
    model.component_types.retain(|d| ids.contains(&d.id));
    model.relation_types.retain(|r| {
        r.source_capability
            .as_ref()
            .is_none_or(|id| ids.contains(id))
    });
    model
}

pub(super) fn wire_capabilities(world: &World) -> Value {
    json!(world.component_types.iter().map(|d| {
        json!({"id":d.id,"name":d.name,"description":d.description,"fields":d.fields.iter().map(|f| {
            let mut v=json!({"key":f.key,"label":f.label,"kind":f.kind});
            if f.required {v["required"]=json!(true);}
            if f.default_value!=PropertyValue::Null {v["defaultValue"]=json!(f.default_value);}
            if let Some(m)=&f.metadata {
                if let Some(help)=&m.help {v["help"]=json!(help);}
                if !m.uses.is_empty(){v["uses"]=json!(m.uses);}
                if !m.choices.is_empty(){v["choices"]=json!(m.choices);}
                if let Some(c)=&m.reference_capability{v["referenceCapability"]=json!(c);}
                if !m.covered_by.is_empty(){v["coveredBy"]=json!(m.covered_by);}
            }
            v
        }).collect::<Vec<_>>()})
    }).collect::<Vec<_>>())
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
    json!({"outputTokens":tokens,"suggestedMaxNewNodes":(tokens.saturating_sub(512)/node_cost).clamp(1,20),"nodeLimitIsAdvisory":true,"priority":"Cover the requested distinct places/people/items and necessary links first; develop their useful structured details. Reduce optional extras and repeated text before collapsing requested entities. Never fill every optional field with null or boilerplate."})
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
    if !places.is_empty() {
        v["placeStructure"] = json!({"placeTypeIds":places,"rule":"Represent distinct buildings, grounds and useful scene locations as separate linked places. A homestead containing a house, barn and cultivated field needs a parent homestead and child house, barn and field, not one paragraph containing all four. Add important interiors when requested or needed for scenes; do not turn every object or corner into a place. In a group, split only requested locations; do not invent another entire world.","containment":world.relation_types.iter().find(|r|r.physical_placement&&r.source_capability.as_deref()==Some("core.place")&&r.target_capability.as_deref()==Some("core.place")).map(|r|json!({"typeId":r.id,"source":"new:house","target":"new:homestead"}))});
    }
    if is_world {
        v["foundationTypeIds"] = json!(foundation_types(world));
        v["narration"] = json!({"requested":request.include_narration==Some(true),"presetIds":world.node_types.iter().filter(|t|t.components.iter().any(|c|c=="core.narration-style")).map(|t|&t.id).collect::<Vec<_>>(),"rule":"When requested, create a narration-style node with person, tense and several concrete instructions suited to this premise: viewpoint discipline, tone, pace, dialogue and descriptive focus. Respect explicit user choices; otherwise third person and past tense are a starting point. Apply will include the proposed style for passage writing/wording. This is guidance, not an accomplished event."});
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
