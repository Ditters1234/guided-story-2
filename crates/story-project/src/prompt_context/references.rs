use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// IDs grant reference permission only. The source world is used for capability
/// checks and previous values; it is never serialized by this projection.
pub(crate) struct ReferenceScope<'a> {
    source: &'a World,
    ids: BTreeSet<String>,
    new_capabilities: Option<BTreeSet<String>>,
    new_alias: Value,
}

impl<'a> ReferenceScope<'a> {
    pub(crate) fn new(
        source: &'a World,
        ids: BTreeSet<String>,
        new_model: Option<&World>,
        new_alias: Value,
    ) -> Self {
        let new_capabilities = new_model.map(|model| {
            let mut capabilities: BTreeSet<_> = model
                .node_types
                .iter()
                .flat_map(|t| crate::authoring::creation::offered_components(model, t))
                .collect();
            if capabilities.contains("core.container") {
                capabilities.insert("core.holder".into());
            }
            capabilities
        });
        Self {
            source,
            ids,
            new_capabilities,
            new_alias,
        }
    }

    fn choices(&self, capability: Option<&str>) -> (Vec<String>, bool) {
        let ids = self
            .source
            .nodes
            .iter()
            .filter(|n| {
                self.ids.contains(&n.id)
                    && capability.is_none_or(|cap| crate::model::capability(n, cap))
            })
            .map(|n| n.id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let new_alias = self
            .new_capabilities
            .as_ref()
            .is_some_and(|caps| capability.is_none_or(|cap| caps.contains(cap)));
        (ids, new_alias)
    }

    fn field(&self, field: &FieldDefinition, previous: &[&PropertyValue]) -> Value {
        let capability = field
            .metadata
            .as_ref()
            .and_then(|m| m.reference_capability.as_deref());
        let (ids, new_alias) = self.choices(capability);
        let mut choices = vec![];
        if !ids.is_empty() {
            choices.push(json!({"type":"string","enum":ids}));
        }
        if new_alias {
            choices.push(self.new_alias.clone());
        }
        let has_targets = !choices.is_empty();
        let target = alternatives(choices);
        let mut values = vec![];
        match field.kind {
            FieldKind::NodeRef => {
                if has_targets {
                    values.push(target);
                }
                values.push(json!({"type":"null"}));
            }
            FieldKind::NodeRefs => {
                values.push(if has_targets {
                    json!({"type":"array","items":target,"maxItems":500})
                } else {
                    json!({"type":"array","items":{"type":"string"},"maxItems":0})
                });
                if !field.required {
                    values.push(json!({"type":"null"}));
                }
            }
            _ => unreachable!("reference projection only binds node reference fields"),
        }
        // Native scope permits the exact old value. For lists this must remain
        // a whole-list alternative, never permission to reuse individual old IDs.
        for value in previous {
            let outside = match value {
                PropertyValue::Text(id) => !ids.contains(id),
                PropertyValue::NodeRefs(old) => old.iter().any(|id| !ids.contains(id)),
                _ => false,
            };
            let constant = json!({"const":value});
            if outside && !values.contains(&constant) {
                values.push(constant);
            }
        }
        alternatives(values)
    }

    fn properties(
        &self,
        schema: &mut Value,
        fields: &[FieldDefinition],
        previous: &[&BTreeMap<String, PropertyValue>],
        used: &mut BTreeSet<Option<String>>,
    ) {
        for field in fields
            .iter()
            .filter(|f| matches!(f.kind, FieldKind::NodeRef | FieldKind::NodeRefs))
        {
            let Some(output) = schema
                .get_mut("properties")
                .and_then(|p| p.get_mut(&field.key))
            else {
                continue;
            };
            // Protected fields already carry their exact const/null contract.
            if output.get("const").is_some() || *output == json!({"type":"null"}) {
                continue;
            }
            let old = previous
                .iter()
                .filter_map(|p| p.get(&field.key))
                .collect::<Vec<_>>();
            *output = self.field(field, &old);
            used.insert(
                field
                    .metadata
                    .as_ref()
                    .and_then(|m| m.reference_capability.clone()),
            );
        }
    }

    pub(crate) fn node(
        &self,
        schema: &mut Value,
        model: &World,
        used: &mut BTreeSet<Option<String>>,
    ) {
        let ids = schema_ids(&schema["properties"]["id"]);
        let previous: Vec<_> = self
            .source
            .nodes
            .iter()
            .filter(|n| ids.contains(&n.id))
            .collect();
        let Some(components) = schema
            .get_mut("properties")
            .and_then(|p| p.get_mut("components"))
            .and_then(|c| c.get_mut("properties"))
            .and_then(Value::as_object_mut)
        else {
            return;
        };
        for (id, component) in components {
            if let Some(definition) = model.component_types.iter().find(|d| &d.id == id) {
                let values = previous
                    .iter()
                    .filter_map(|n| n.components.get(id))
                    .collect::<Vec<_>>();
                self.properties(component, &definition.fields, &values, used);
            }
        }
    }

    pub(crate) fn link(
        &self,
        schema: &mut Value,
        model: &World,
        used: &mut BTreeSet<Option<String>>,
    ) {
        let ids = schema_ids(&schema["properties"]["id"]);
        let previous = self
            .source
            .edges
            .iter()
            .filter(|e| ids.contains(&e.id))
            .map(|e| &e.properties)
            .collect::<Vec<_>>();
        let relation = schema["properties"]["typeId"]["const"]
            .as_str()
            .and_then(|id| model.relation_types.iter().find(|r| r.id == id));
        if let Some(relation) = relation {
            self.properties(
                &mut schema["properties"]["properties"],
                &relation.fields,
                &previous,
                used,
            );
        }
    }

    pub(crate) fn authoring(&self, schema: &mut Value, model: &World) -> BTreeSet<Option<String>> {
        let mut used = BTreeSet::new();
        let nodes = &mut schema["properties"]["nodes"];
        if nodes["type"] == "object" {
            if let Some(groups) = nodes["properties"].as_object_mut() {
                for group in groups.values_mut() {
                    variants(&mut group["items"], &mut |node| {
                        self.node(node, model, &mut used)
                    });
                }
            }
        } else {
            variants(&mut nodes["items"], &mut |node| {
                self.node(node, model, &mut used)
            });
        }
        if let Some(root) = schema["properties"].get_mut("root") {
            self.node(root, model, &mut used);
        }
        variants(&mut schema["properties"]["edges"]["items"], &mut |link| {
            self.link(link, model, &mut used)
        });
        used
    }

    pub(crate) fn guidance(&self, used: &BTreeSet<Option<String>>, include_pools: bool) -> Value {
        let mut value = json!({"rule":"Reference fields use only supplied IDs matching referenceCapability, or declared new aliases for compatible proposed nodes. References grant no edit permission. An old reference/list may remain verbatim; its hidden IDs cannot be copied into a changed value."});
        if include_pools {
            value["pools"] = json!(used
                .iter()
                .map(|cap| {
                    let (ids, new_alias) = self.choices(cap.as_deref());
                    let mut pool = json!({"referenceCapability":cap,"existingIds":ids});
                    if new_alias {
                        if let Some(pattern) = self.new_alias.get("pattern") {
                            pool["newAliasPattern"] = pattern.clone();
                        } else if let Some(id) = self.new_alias.get("const") {
                            pool["newAliasId"] = id.clone();
                        }
                    }
                    pool
                })
                .collect::<Vec<_>>());
        }
        value
    }
}

fn alternatives(mut choices: Vec<Value>) -> Value {
    if choices.len() == 1 {
        choices.pop().unwrap()
    } else {
        json!({"anyOf":choices})
    }
}

fn schema_ids(schema: &Value) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    if let Some(id) = schema["const"].as_str() {
        ids.insert(id.into());
    }
    ids.extend(
        schema["enum"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned),
    );
    for option in schema["anyOf"].as_array().into_iter().flatten() {
        ids.extend(schema_ids(option));
    }
    ids
}

fn variants(schema: &mut Value, visit: &mut impl FnMut(&mut Value)) {
    if let Some(choices) = schema.get_mut("anyOf").and_then(Value::as_array_mut) {
        for choice in choices {
            visit(choice);
        }
    } else {
        visit(schema);
    }
}
