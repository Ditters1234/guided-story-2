use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(untagged)]
pub enum PropertyValue {
    Text(String),
    Number(f64),
    Boolean(bool),
    NodeRefs(Vec<String>),
    Null,
}
impl PropertyValue {
    fn as_str(&self) -> Option<&str> {
        if let Self::Text(s) = self {
            Some(s)
        } else {
            None
        }
    }
    fn as_f64(&self) -> Option<f64> {
        if let Self::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }
    fn as_u64(&self) -> Option<u64> {
        self.as_f64()
            .filter(|n| {
                n.is_finite() && *n >= 0.0 && *n <= 9_007_199_254_740_991.0 && n.fract() == 0.0
            })
            .map(|n| n as u64)
    }
}

/// All entities share one identity; capabilities and typed links supply behavior.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorldNode {
    pub id: String,
    pub type_id: String,
    pub name: String,
    pub description: String,
    pub visual_description: String,
    pub components: BTreeMap<String, BTreeMap<String, PropertyValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorldEdge {
    pub id: String,
    pub type_id: String,
    pub source_id: String,
    pub target_id: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NodeTypeDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub components: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum FieldKind {
    Text,
    Number,
    Boolean,
    NodeRef,
    NodeRefs,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FieldDefinition {
    pub key: String,
    pub label: String,
    pub kind: FieldKind,
    pub required: bool,
    pub default_value: PropertyValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComponentDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub fields: Vec<FieldDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RelationDefinition {
    pub id: String,
    pub name: String,
    pub label: String,
    pub source_capability: Option<String>,
    pub target_capability: Option<String>,
    pub physical_placement: bool,
    pub single_target: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct World {
    pub nodes: Vec<WorldNode>,
    pub edges: Vec<WorldEdge>,
    pub node_types: Vec<NodeTypeDefinition>,
    pub component_types: Vec<ComponentDefinition>,
    pub relation_types: Vec<RelationDefinition>,
}

impl Default for World {
    fn default() -> Self {
        serde_json::from_str(include_str!("../../../schema/world-presets.json"))
            .expect("Invalid built-in presets")
    }
}

pub fn validate_id(id: &str) -> Result<(), String> {
    Uuid::parse_str(id)
        .map(|_| ())
        .map_err(|_| "Invalid record ID.".into())
}
pub fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() || name.chars().count() > 200 {
        return Err("A name must contain 1–200 characters.".into());
    }
    Ok(())
}
fn capability(node: &WorldNode, id: &str) -> bool {
    node.components.contains_key(id)
        || (id == "core.holder" && node.components.contains_key("core.container"))
}
fn field_matches(value: &PropertyValue, kind: &FieldKind, ids: &BTreeSet<String>) -> bool {
    match kind {
        FieldKind::Text => value.as_str().is_some_and(|s| s.chars().count() <= 16000),
        FieldKind::Number => value
            .as_f64()
            .is_some_and(|n| n.is_finite() && n.abs() <= 9_007_199_254_740_991.0),
        FieldKind::Boolean => matches!(value, PropertyValue::Boolean(_)),
        FieldKind::NodeRef => {
            matches!(value, PropertyValue::Null)
                || value.as_str().is_some_and(|id| ids.contains(id))
        }
        FieldKind::NodeRefs => {
            matches!(value, PropertyValue::NodeRefs(refs) if refs.len() <= 500 && refs.iter().all(|id| ids.contains(id)))
        }
    }
}

impl World {
    pub fn validate(&self) -> Result<(), String> {
        // Extensions use their own IDs rather than weakening built-in behavior.
        let builtins = Self::default();
        for definition in &builtins.component_types {
            if !self.component_types.contains(definition) {
                return Err("A built-in capability definition is missing or changed. Add a custom capability instead.".into());
            }
        }
        for definition in &builtins.relation_types {
            if !self.relation_types.contains(definition) {
                return Err("A built-in relationship definition is missing or changed. Add a custom relationship instead.".into());
            }
        }
        if self.nodes.len() > 5000
            || self.edges.len() > 20000
            || self.node_types.len() > 200
            || self.component_types.len() > 200
            || self.relation_types.len() > 200
        {
            return Err("World exceeds the prototype's record/type limits.".into());
        }
        for (id, name) in self
            .node_types
            .iter()
            .map(|d| (&d.id, &d.name))
            .chain(self.component_types.iter().map(|d| (&d.id, &d.name)))
            .chain(self.relation_types.iter().map(|d| (&d.id, &d.name)))
        {
            validate_name(id)?;
            validate_name(name)?;
        }
        for ids in [
            self.node_types.iter().map(|d| &d.id).collect::<Vec<_>>(),
            self.component_types.iter().map(|d| &d.id).collect(),
            self.relation_types.iter().map(|d| &d.id).collect(),
        ] {
            let mut unique = BTreeSet::new();
            for id in ids {
                if !unique.insert(id) {
                    return Err("Duplicate schema definition ID.".into());
                }
            }
        }
        let components: BTreeMap<_, _> = self
            .component_types
            .iter()
            .map(|d| (d.id.as_str(), d))
            .collect();
        let types: BTreeSet<_> = self.node_types.iter().map(|d| d.id.as_str()).collect();
        let relations: BTreeMap<_, _> = self
            .relation_types
            .iter()
            .map(|d| (d.id.as_str(), d))
            .collect();
        for preset in &self.node_types {
            if preset
                .components
                .iter()
                .any(|id| !components.contains_key(id.as_str()))
            {
                return Err("A node preset references an unknown capability.".into());
            }
        }
        for definition in &self.component_types {
            if definition.fields.len() > 100 {
                return Err("A capability has too many fields.".into());
            }
            let mut fields = BTreeSet::new();
            for field in &definition.fields {
                validate_name(&field.key)?;
                validate_name(&field.label)?;
                if !fields.insert(&field.key) {
                    return Err("Duplicate capability field.".into());
                }
            }
        }
        for relation in &self.relation_types {
            validate_name(&relation.label)?;
            for id in [&relation.source_capability, &relation.target_capability]
                .into_iter()
                .flatten()
            {
                if !components.contains_key(id.as_str()) {
                    return Err("A relation references an unknown capability.".into());
                }
            }
        }
        let mut ids = BTreeSet::new();
        for node in &self.nodes {
            validate_id(&node.id)?;
            validate_name(&node.name)?;
            if !ids.insert(node.id.clone()) {
                return Err("Duplicate world node ID.".into());
            }
            if !types.contains(node.type_id.as_str()) {
                return Err("A node type has no saved definition.".into());
            }
            if node.description.chars().count() > 16000
                || node.visual_description.chars().count() > 16000
            {
                return Err("A node description is too long.".into());
            }
        }
        let nodes: BTreeMap<_, _> = self.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        for node in &self.nodes {
            for (id, values) in &node.components {
                let definition = components
                    .get(id.as_str())
                    .ok_or("A node capability has no saved definition.")?;
                if values
                    .keys()
                    .any(|key| !definition.fields.iter().any(|f| &f.key == key))
                {
                    return Err("A capability property has no field definition.".into());
                }
                for field in &definition.fields {
                    match values.get(&field.key) {
                        None if field.required => {
                            return Err(format!("{} needs a value for {}.", node.name, field.label))
                        }
                        Some(value) if !field_matches(value, &field.kind, &ids) => {
                            return Err(format!("Invalid {} on {}.", field.label, node.name))
                        }
                        _ => (),
                    }
                }
            }
            if let Some(item) = node.components.get("core.item") {
                let quantity = item
                    .get("quantity")
                    .and_then(PropertyValue::as_u64)
                    .filter(|n| *n <= 1_000_000)
                    .ok_or("Item quantity must be a nonnegative whole number.")?;
                validate_name(
                    item.get("unit")
                        .and_then(PropertyValue::as_str)
                        .ok_or("An item needs a unit.")?,
                )?;
                if capability(node, "core.container") && quantity != 1 {
                    return Err("A counted container must have quantity one.".into());
                }
            }
        }
        let mut edge_ids = BTreeSet::new();
        let mut physical: BTreeMap<&str, &str> = BTreeMap::new();
        let mut single = BTreeSet::new();
        let mut pairs = BTreeSet::new();
        for edge in &self.edges {
            validate_id(&edge.id)?;
            if ids.contains(&edge.id) || !edge_ids.insert(&edge.id) {
                return Err("Duplicate world edge ID.".into());
            }
            let relation = relations
                .get(edge.type_id.as_str())
                .ok_or("An edge type has no saved definition.")?;
            let source = nodes
                .get(edge.source_id.as_str())
                .ok_or("A relation source does not exist.")?;
            let target = nodes
                .get(edge.target_id.as_str())
                .ok_or("A relation target does not exist.")?;
            if source.id == target.id {
                return Err("A node cannot link to itself.".into());
            }
            if !pairs.insert((&edge.type_id, &edge.source_id, &edge.target_id)) {
                return Err("Duplicate world connection.".into());
            }
            if relation
                .source_capability
                .as_ref()
                .is_some_and(|id| !capability(source, id))
                || relation
                    .target_capability
                    .as_ref()
                    .is_some_and(|id| !capability(target, id))
            {
                return Err("A relation endpoint lacks its required capability.".into());
            }
            if relation.single_target && !single.insert((&edge.type_id, &edge.source_id)) {
                return Err("This relation allows only one target per node.".into());
            }
            if relation.physical_placement
                && physical.insert(&edge.source_id, &edge.target_id).is_some()
            {
                return Err("A node cannot occupy two physical holders at once.".into());
            }
        }
        for id in physical.keys() {
            let mut seen = BTreeSet::from([*id]);
            let mut next = physical.get(id);
            while let Some(parent) = next {
                if !seen.insert(*parent) {
                    return Err("Physical containment cannot form a cycle.".into());
                }
                next = physical.get(parent);
            }
        }
        for edge in self.edges.iter().filter(|e| e.type_id == "core.worn-by") {
            let item = nodes[edge.source_id.as_str()];
            if physical.get(edge.source_id.as_str()).copied() != Some(edge.target_id.as_str())
                || item
                    .components
                    .get("core.item")
                    .and_then(|v| v.get("quantity"))
                    .and_then(PropertyValue::as_u64)
                    != Some(1)
            {
                return Err(
                    "A worn item must be held directly by its wearer with quantity one.".into(),
                );
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordRef {
    pub id: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectView {
    pub schema_version: u32,
    pub story_id: String,
    pub title: String,
    pub revision: u32,
    pub folder_path: String,
    pub world_ref: RecordRef,
    pub world: World,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommitResult {
    pub project: ProjectView,
    pub committed_revision: u32,
    pub replayed: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Workspace {
    pub version: u32,
    pub expanded_ids: Vec<String>,
    pub selected_id: Option<String>,
    pub positions: BTreeMap<String, Position>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorDraft {
    pub schema_version: u32,
    pub source_revision: u32,
    pub title: String,
    pub world: World,
}
