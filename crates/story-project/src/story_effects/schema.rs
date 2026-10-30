//! Concrete provider contracts over the captured scene. Canonical validation and
//! evidence/approval rules stay in the native effects pipeline.
use super::*;

fn field_descriptor(field: &FieldDefinition) -> Value {
    let mut value = json!({"key":field.key,"label":field.label,"kind":field.kind});
    if field.required {
        value["required"] = json!(true);
    }
    if field.default_value != PropertyValue::Null {
        value["defaultValue"] = json!(field.default_value);
    }
    if let Some(meta) = &field.metadata {
        if let Some(help) = &meta.help {
            value["help"] = json!(help);
        }
        if !meta.uses.is_empty() {
            value["uses"] = json!(meta.uses);
        }
        if let Some(stability) = &meta.stability {
            value["stability"] = json!(stability);
        }
        if !meta.choices.is_empty() {
            value["choices"] = json!(meta.choices);
        }
        if let Some(capability) = &meta.reference_capability {
            value["referenceCapability"] = json!(capability);
        }
        if !meta.covered_by.is_empty() {
            value["coveredBy"] = json!(meta.covered_by);
        }
    }
    value
}

pub(super) fn capabilities(world: &World) -> Value {
    json!(world.component_types.iter().map(|definition|json!({"id":definition.id,"name":definition.name,"description":definition.description,"fields":definition.fields.iter().map(field_descriptor).collect::<Vec<_>>()})).collect::<Vec<_>>())
}
pub(super) fn relationships(world: &World) -> Value {
    json!(world.relation_types.iter().map(|definition|{
        let mut value=json!({"id":definition.id,"name":definition.name,"label":definition.label,"physicalPlacement":definition.physical_placement,"singleTarget":definition.single_target});
        if let Some(capability)=&definition.source_capability {value["sourceCapability"]=json!(capability);}
        if let Some(capability)=&definition.target_capability {value["targetCapability"]=json!(capability);}
        if !definition.fields.is_empty(){value["fields"]=json!(definition.fields.iter().map(field_descriptor).collect::<Vec<_>>());}
        value
    }).collect::<Vec<_>>())
}

/// Descriptive help is already in the field catalogue/default extraction prompt.
/// The provider grammar retains all semantic constraints and field names.
fn compact_schema(value: &mut Value) {
    match value {
        Value::Object(object) => {
            if object.get("description").is_some_and(Value::is_string) {
                object.remove("description");
            }
            for child in object.values_mut() {
                compact_schema(child);
            }
            if object.len() == 1 {
                if let Some(options) = object.get("anyOf").and_then(Value::as_array) {
                    if options.len() == 2
                        && options[1] == json!({"type":"null"})
                        && options[0]
                            .get("type")
                            .is_some_and(|v| v.is_string() || v.is_array())
                    {
                        let mut simple = options[0].clone();
                        let mut types = match &simple["type"] {
                            Value::String(t) => vec![json!(t)],
                            Value::Array(types) => types.clone(),
                            _ => vec![],
                        };
                        if !types.contains(&Value::String("null".into())) {
                            types.push(json!("null"));
                        }
                        simple["type"] = json!(types);
                        if let Some(choices) = simple.get_mut("enum").and_then(Value::as_array_mut)
                        {
                            if !choices.contains(&Value::Null) {
                                choices.push(Value::Null);
                            }
                        }
                        *value = simple;
                    }
                }
            }
        }
        Value::Array(values) => {
            for child in values {
                compact_schema(child);
            }
        }
        _ => {}
    }
}

fn reference(ids: impl IntoIterator<Item = String>, new_alias: bool) -> Option<Value> {
    let ids: BTreeSet<_> = ids.into_iter().collect();
    let mut choices = vec![];
    if !ids.is_empty() {
        choices.push(json!({"type":"string","enum":ids}));
    }
    if new_alias {
        choices.push(json!({"type":"string","pattern":"^new:[A-Za-z0-9_-]{1,60}$"}));
    }
    match choices.len() {
        0 => None,
        1 => choices.pop(),
        _ => Some(json!({"anyOf":choices})),
    }
}

fn endpoint_reference(world: &World, required: Option<&str>) -> Value {
    reference(
        world
            .nodes
            .iter()
            .filter(|node| required.is_none_or(|id| crate::model::capability(node, id)))
            .map(|node| node.id.clone()),
        true,
    )
    .unwrap()
}

fn components(world: &World, required: &[String], old: Option<&WorldNode>) -> Value {
    let mut allowed: BTreeSet<_> = required.iter().cloned().collect();
    if old.is_none() {
        if allowed.contains("core.item") {
            allowed.extend([
                "core.container".into(),
                "core.wearable".into(),
                "detail.wearable".into(),
            ]);
        }
        if allowed.contains("core.character") {
            allowed.insert("detail.anatomy".into());
        }
    }
    let properties: serde_json::Map<_, _> = world.component_types.iter().filter(|d| allowed.contains(&d.id)).map(|d| {
        let mut required_fields = vec![];
        let fields: serde_json::Map<_, _> = d.fields.iter().filter_map(|f| {
            let previous = old.and_then(|n| n.components.get(&d.id)).and_then(|values| values.get(&f.key));
            let schema = if planning_only(f) {
                if old.is_some() && previous.is_none() { return None; }
                json!({"const":previous.unwrap_or(&f.default_value)})
            } else { crate::authoring::field_schema(f) };
            if f.required || (planning_only(f) && previous.is_some()) { required_fields.push(f.key.clone()); }
            Some((f.key.clone(), schema))
        }).collect();
        (d.id.clone(), json!({"type":"object","properties":fields,"required":required_fields,"additionalProperties":false}))
    }).collect();
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}

fn node(
    world: &World,
    type_id: &str,
    id: Value,
    required: &[String],
    old: Option<&WorldNode>,
) -> Value {
    json!({"type":"object","properties":{
        "id":id,"typeId":{"const":type_id},"name":{"type":"string","minLength":1,"maxLength":200},
        "description":{"type":"string","maxLength":16000},"visualDescription":{"type":"string","maxLength":16000},
        "components":components(world,required,old)
    },"required":["id","typeId","name","description","visualDescription","components"],"additionalProperties":false})
}

fn nodes(world: &World) -> Vec<Value> {
    // Records with the same actual capabilities and protected planning values
    // share a branch so several same-preset participants do not duplicate their
    // whole field schema. Their ID enum still permits only captured records.
    let mut groups: BTreeMap<String, (&WorldNode, Vec<String>)> = BTreeMap::new();
    for old in world
        .nodes
        .iter()
        .filter(|n| !authoring_only_components(world, n.components.keys()))
    {
        let planning: Vec<_> = world
            .component_types
            .iter()
            .filter(|d| old.components.contains_key(&d.id))
            .flat_map(|d| {
                d.fields
                    .iter()
                    .filter(|f| planning_only(f))
                    .map(|f| (&d.id, &f.key, old.components[&d.id].get(&f.key)))
            })
            .collect();
        let key = json!([
            old.type_id,
            old.components.keys().collect::<Vec<_>>(),
            planning
        ])
        .to_string();
        groups
            .entry(key)
            .or_insert_with(|| (old, vec![]))
            .1
            .push(old.id.clone());
    }
    let mut choices: Vec<_> = groups
        .into_values()
        .map(|(old, ids)| {
            node(
                world,
                &old.type_id,
                reference(ids, false).unwrap(),
                &old.components.keys().cloned().collect::<Vec<_>>(),
                Some(old),
            )
        })
        .collect();
    for preset in world
        .node_types
        .iter()
        .filter(|t| !authoring_only_components(world, t.components.iter()))
    {
        choices.push(node(
            world,
            &preset.id,
            reference([], true).unwrap(),
            &preset.components,
            None,
        ));
    }
    choices
}

pub(super) fn effects_schema(world: &World) -> Value {
    let quote = json!({"type":"string","minLength":8,"maxLength":600,"description":"Copy one continuous excerpt verbatim from the current supplied prose. Preserve words, capitalization and punctuation; no inserted ellipses, joined excerpts or earlier-passage quotes."});
    let nodes = nodes(world);
    let put_nodes = (!nodes.is_empty()).then(|| json!({"type":"object","properties":{"kind":{"const":"putNode"},"node":{"anyOf":nodes},"evidence":quote},"required":["kind","node","evidence"],"additionalProperties":false}));
    let node_ids = reference(world.nodes.iter().map(|n| n.id.clone()), true).unwrap();
    let scoped: BTreeSet<_> = world.nodes.iter().map(|n| n.id.as_str()).collect();
    let link_ids: Vec<_> = world
        .edges
        .iter()
        .filter(|e| scoped.contains(e.source_id.as_str()) && scoped.contains(e.target_id.as_str()))
        .map(|e| e.id.clone())
        .collect();
    let mut links = crate::authoring::relationship_schema(world);
    let put_links = if let Some(variants) = links.get_mut("anyOf").and_then(Value::as_array_mut) {
        for v in variants {
            let relation = v["properties"]["typeId"]["const"]
                .as_str()
                .and_then(|id| world.relation_types.iter().find(|r| r.id == id));
            v["properties"]["id"] = reference(link_ids.clone(), true).unwrap();
            v["properties"]["sourceId"] =
                endpoint_reference(world, relation.and_then(|r| r.source_capability.as_deref()));
            v["properties"]["targetId"] =
                endpoint_reference(world, relation.and_then(|r| r.target_capability.as_deref()));
        }
        Some(
            json!({"type":"object","properties":{"kind":{"const":"putLink"},"link":links,"evidence":quote},"required":["kind","link","evidence"],"additionalProperties":false}),
        )
    } else {
        None
    };
    let remove_links = reference(link_ids, false).map(|ids|json!({"type":"object","properties":{"kind":{"const":"removeLink"},"id":ids,"evidence":quote},"required":["kind","id","evidence"],"additionalProperties":false}));
    let mut schema = json!({"type":"object","properties":{
        "changes":{"type":"object","properties":{
            "putNodes":operation_array(put_nodes),
            "putLinks":operation_array(put_links),
            "removeLinks":operation_array(remove_links)
        },"required":["putNodes","putLinks","removeLinks"],"additionalProperties":false},
        "notes":{"type":"array","maxItems":12,"items":{"type":"object","properties":{"text":{"type":"string","maxLength":2000},"evidence":quote,"nodeIds":{"type":"array","maxItems":20,"items":node_ids}},"required":["text","evidence","nodeIds"],"additionalProperties":false}},
        "suggestChapterEnd":{"type":"boolean"}
    },"required":["changes","notes","suggestChapterEnd"],"additionalProperties":false});
    compact_schema(&mut schema);
    schema
}

fn operation_array(items: Option<Value>) -> Value {
    json!({"type":"array","maxItems":if items.is_some(){80}else{0},"items":items.unwrap_or_else(||json!({"type":"object","properties":{},"additionalProperties":false}))})
}

#[cfg(test)]
mod tests;
