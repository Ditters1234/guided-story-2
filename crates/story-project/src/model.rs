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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub properties: BTreeMap<String, PropertyValue>,
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
    LongText,
    Number,
    Boolean,
    NodeRef,
    NodeRefs,
    Choice,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FieldUse {
    Story,
    Visual,
    Planning,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FieldStability {
    Stable,
    State,
}

/// Declarative authoring/projection hints. These never grant edit permission.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FieldMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub uses: Vec<FieldUse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stability: Option<FieldStability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_capability: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub covered_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FieldDefinition {
    pub key: String,
    pub label: String,
    pub kind: FieldKind,
    pub required: bool,
    pub default_value: PropertyValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<FieldMetadata>,
}

impl FieldDefinition {
    pub fn used_for(&self, usage: FieldUse) -> bool {
        match &self.metadata {
            Some(meta) if !meta.uses.is_empty() => meta.uses.contains(&usage),
            _ => usage == FieldUse::Story,
        }
    }
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<FieldDefinition>,
}

#[derive(Debug, Clone, Serialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct World {
    pub nodes: Vec<WorldNode>,
    pub edges: Vec<WorldEdge>,
    pub node_types: Vec<NodeTypeDefinition>,
    pub component_types: Vec<ComponentDefinition>,
    pub relation_types: Vec<RelationDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ai_context: Vec<crate::AiContextBinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_annotations: Vec<crate::SourceAnnotation>,
}

/// Original field order/values for verifying immutable commit receipts. Never
/// use this representation to edit or validate an active world.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StoredWorld {
    nodes: Vec<WorldNode>,
    edges: Vec<WorldEdge>,
    node_types: Vec<NodeTypeDefinition>,
    component_types: Vec<ComponentDefinition>,
    relation_types: Vec<RelationDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    ai_context: Vec<crate::AiContextBinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    source_annotations: Vec<crate::SourceAnnotation>,
}

// Upgrade the in-memory interpretation only. Checksums are verified against the
// original bytes by storage; immutable records and historical references stay intact.
impl<'de> Deserialize<'de> for World {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let data = StoredWorld::deserialize(deserializer)?;
        let mut world = Self {
            nodes: data.nodes,
            edges: data.edges,
            node_types: data.node_types,
            component_types: data.component_types,
            relation_types: data.relation_types,
            ai_context: data.ai_context,
            source_annotations: data.source_annotations,
        };
        world
            .upgrade_legacy_wearing()
            .map_err(serde::de::Error::custom)?;
        Ok(world)
    }
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
pub(crate) fn capability(node: &WorldNode, id: &str) -> bool {
    node.components.contains_key(id)
        || (id == "core.holder" && node.components.contains_key("core.container"))
}
pub(crate) fn field_matches(
    value: &PropertyValue,
    field: &FieldDefinition,
    nodes: &BTreeMap<&str, &WorldNode>,
) -> bool {
    if matches!(value, PropertyValue::Null) && !field.required {
        return true;
    }
    let valid_ref = |id: &str| {
        nodes.get(id).is_some_and(|n| {
            field
                .metadata
                .as_ref()
                .and_then(|m| m.reference_capability.as_ref())
                .is_none_or(|c| capability(n, c))
        })
    };
    match field.kind {
        FieldKind::Text | FieldKind::LongText => {
            value.as_str().is_some_and(|s| s.chars().count() <= 16000)
        }
        FieldKind::Number => value
            .as_f64()
            .is_some_and(|n| n.is_finite() && n.abs() <= 9_007_199_254_740_991.0),
        FieldKind::Boolean => matches!(value, PropertyValue::Boolean(_)),
        FieldKind::NodeRef => {
            matches!(value, PropertyValue::Null) || value.as_str().is_some_and(valid_ref)
        }
        FieldKind::NodeRefs => {
            matches!(value, PropertyValue::NodeRefs(refs) if refs.len() <= 500 && refs.iter().all(|id| valid_ref(id)))
        }
        FieldKind::Choice => value.as_str().is_some_and(|value| {
            field
                .metadata
                .as_ref()
                .is_some_and(|m| m.choices.iter().any(|choice| choice == value))
        }),
    }
}

impl World {
    fn upgrade_legacy_wearing(&mut self) -> Result<(), String> {
        let Some(index) = self
            .relation_types
            .iter()
            .position(|r| r.id == "core.worn-by" && !r.physical_placement)
        else {
            return Ok(());
        };
        let mut expected = Self::default()
            .relation_types
            .into_iter()
            .find(|r| r.id == "core.worn-by")
            .unwrap();
        expected.physical_placement = false;
        expected.fields = self.relation_types[index].fields.clone();
        if self.relation_types[index] != expected {
            return Ok(());
        }
        // Validate every edge before discarding redundancy, so a damaged legacy
        // placement cannot turn into an apparently valid save. Property drafts
        // may remain invalid and editable; quantity validation runs on commit.
        self.validate_connections(false)
            .map_err(|e| format!("Invalid legacy wearing links: {e}"))?;
        let mut redundant = BTreeSet::new();
        for i in 0..self.edges.len() {
            if self.edges[i].type_id != "core.worn-by" {
                continue;
            }
            let wear = &self.edges[i];
            let held = self
                .edges
                .iter()
                .find(|e| {
                    self.relation_types
                        .iter()
                        .any(|r| r.id == e.type_id && r.physical_placement)
                        && e.source_id == wear.source_id
                        && e.target_id == wear.target_id
                })
                .ok_or("Legacy wearing requires its original matching inventory link.")?;
            let held_notes = if held.type_id == "core.stored-in" {
                held.description.clone()
            } else {
                format!(
                    "Previous placement ({}): {}",
                    held.type_id, held.description
                )
            };
            let notes = if held_notes.is_empty() || held_notes == wear.description {
                wear.description.clone()
            } else if wear.description.is_empty() {
                held_notes
            } else {
                format!("{}\n\nInventory notes: {}", wear.description, held_notes)
            };
            if notes.chars().count() > 16000 {
                return Err("Legacy wearing notes exceed one connection's limit; merge the notes before upgrading. Original files are preserved.".into());
            }
            redundant.insert(held.id.clone());
            self.edges[i].description = notes;
        }
        self.edges.retain(|e| !redundant.contains(&e.id));
        self.relation_types[index].physical_placement = true;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        crate::knowledge::validate_knowledge(self)?;
        crate::ai_context::validate_ai_context(self)?;
        crate::sources::validate_annotation_shapes(self)?;
        // Extensions use their own IDs rather than weakening built-in behavior.
        let builtins: Self = serde_json::from_str(include_str!("../../../schema/world-core.json"))
            .expect("Invalid native world contracts");
        for definition in &builtins.component_types {
            if !self.component_types.contains(definition) {
                return Err("A built-in capability definition is missing or changed. Add a custom capability instead.".into());
            }
        }
        for definition in &builtins.relation_types {
            if !self.relation_types.iter().any(|saved| {
                let mut contract = saved.clone();
                contract.fields.clear();
                &contract == definition
            }) {
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
        for preset in &self.node_types {
            if preset.description.chars().count() > 4000 {
                return Err("Preset help is too long.".into());
            }
            if preset
                .components
                .iter()
                .enumerate()
                .any(|(i, id)| preset.components[..i].contains(id))
            {
                return Err("A preset cannot repeat an extension.".into());
            }
            if preset
                .components
                .iter()
                .any(|id| !components.contains_key(id.as_str()))
            {
                return Err("A node preset references an unknown capability.".into());
            }
        }
        for (name, help, definition_fields) in self
            .component_types
            .iter()
            .map(|d| (&d.name, d.description.as_str(), &d.fields))
            .chain(self.relation_types.iter().map(|d| (&d.name, "", &d.fields)))
        {
            if help.chars().count() > 4000 {
                return Err(format!("{name}: help is too long."));
            }
            if definition_fields.len() > 100 {
                return Err(format!("{name}: too many fields."));
            }
            let mut fields = BTreeSet::new();
            for field in definition_fields {
                validate_name(&field.key)?;
                validate_name(&field.label)?;
                if !fields.insert(&field.key) {
                    return Err(format!("Duplicate field on {name}."));
                }
                if let Some(meta) = &field.metadata {
                    if meta.help.as_ref().is_some_and(|s| s.chars().count() > 1000)
                        || meta.uses.len() > 3
                        || meta
                            .uses
                            .iter()
                            .enumerate()
                            .any(|(i, value)| meta.uses[..i].contains(value))
                        || meta.choices.len() > 40
                        || meta.covered_by.len() > 30
                        || meta
                            .covered_by
                            .iter()
                            .any(|v| v.trim().is_empty() || v.chars().count() > 100)
                    {
                        return Err(
                            "Field guidance exceeds its limits or repeats a task use.".into()
                        );
                    }
                    let mut choices = BTreeSet::new();
                    for choice in &meta.choices {
                        validate_name(choice)?;
                        if !choices.insert(choice) {
                            return Err("Field choices must be unique.".into());
                        }
                    }
                    if !matches!(field.kind, FieldKind::Choice) && !meta.choices.is_empty() {
                        return Err("Only a choice field may declare choices.".into());
                    }
                    if let Some(id) = &meta.reference_capability {
                        if !matches!(field.kind, FieldKind::NodeRef | FieldKind::NodeRefs)
                            || !components.contains_key(id.as_str())
                        {
                            return Err("A reference filter needs a reference field and a saved capability.".into());
                        }
                    }
                    if !meta.covered_by.is_empty() && !field.used_for(FieldUse::Visual) {
                        return Err("Coverage hints apply to visual fields only.".into());
                    }
                }
                if matches!(field.kind, FieldKind::Choice)
                    && field.metadata.as_ref().is_none_or(|m| m.choices.is_empty())
                {
                    return Err("A choice field needs at least one saved choice.".into());
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
        let node_index: BTreeMap<_, _> = self.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        for (name, fields) in self
            .component_types
            .iter()
            .map(|d| (&d.name, &d.fields))
            .chain(self.relation_types.iter().map(|d| (&d.name, &d.fields)))
        {
            for field in fields {
                if !field_matches(&field.default_value, field, &node_index) {
                    return Err(format!("Invalid default for {} / {}.", name, field.label));
                }
            }
        }
        for node in &self.nodes {
            if capability(node, "core.wearable") && !capability(node, "core.item") {
                return Err("A wearable extension requires the inventory item capability.".into());
            }
            for (id, values) in &node.components {
                let definition = components
                    .get(id.as_str())
                    .ok_or("A node capability has no saved definition.")?;
                if let Some(key) = values
                    .keys()
                    .find(|key| !definition.fields.iter().any(|f| &f.key == *key))
                {
                    return Err(format!(
                        "{} / {} has unknown field “{}”. Remove it or use a declared field.",
                        node.name, definition.name, key
                    ));
                }
                for field in &definition.fields {
                    match values.get(&field.key) {
                        None if field.required => {
                            return Err(format!("{} needs a value for {}.", node.name, field.label))
                        }
                        Some(value) if !field_matches(value, field, &node_index) => {
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
        self.validate_connections(true)
    }

    fn validate_connections(&self, check_quantities: bool) -> Result<(), String> {
        let ids: BTreeSet<_> = self.nodes.iter().map(|n| n.id.clone()).collect();
        let nodes: BTreeMap<_, _> = self.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        let relations: BTreeMap<_, _> = self
            .relation_types
            .iter()
            .map(|r| (r.id.as_str(), r))
            .collect();
        let mut edge_ids = BTreeSet::new();
        let mut physical: BTreeMap<&str, &str> = BTreeMap::new();
        let mut single = BTreeSet::new();
        let mut pairs = BTreeSet::new();
        for edge in &self.edges {
            validate_id(&edge.id)?;
            if edge.description.chars().count() > 16000 {
                return Err("A connection description is too long.".into());
            }
            if ids.contains(&edge.id) || !edge_ids.insert(&edge.id) {
                return Err("Duplicate world edge ID.".into());
            }
            let relation = relations
                .get(edge.type_id.as_str())
                .ok_or("An edge type has no saved definition.")?;
            if edge
                .properties
                .keys()
                .any(|key| !relation.fields.iter().any(|f| &f.key == key))
            {
                return Err(format!(
                    "{} has an undefined relationship field.",
                    relation.name
                ));
            }
            for field in &relation.fields {
                match edge.properties.get(&field.key) {
                    None if field.required => {
                        return Err(format!("{} needs {}.", relation.name, field.label))
                    }
                    Some(value) if !field_matches(value, field, &nodes) => {
                        return Err(format!("Invalid {} on {}.", field.label, relation.name))
                    }
                    _ => (),
                }
            }
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
                return Err(format!("A relation endpoint lacks its required capability: {} → {} ({}) needs {} on the source and {} on the target.",source.name,target.name,relation.name,relation.source_capability.as_deref().unwrap_or("no specific capability"),relation.target_capability.as_deref().unwrap_or("no specific capability")));
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
                || (check_quantities
                    && item
                        .components
                        .get("core.item")
                        .and_then(|v| v.get("quantity"))
                        .and_then(PropertyValue::as_u64)
                        != Some(1))
            {
                return Err(
                    "A worn item must have quantity one and a single physical placement at its wearer.".into(),
                );
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordRef {
    pub id: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectView {
    pub warnings: Vec<String>,
    pub schema_version: u32,
    pub story_id: String,
    pub title: String,
    pub revision: u32,
    pub folder_path: String,
    pub world_ref: RecordRef,
    pub story_ref: Option<RecordRef>,
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

impl Workspace {
    pub fn validate(&self) -> Result<(), String> {
        if self.positions.len() > 5000 || self.expanded_ids.len() > 5000 {
            return Err("Workspace layout is too large.".into());
        }
        for (id, position) in &self.positions {
            validate_id(id)?;
            if !position.x.is_finite()
                || !position.y.is_finite()
                || position.x.abs() > 1e7
                || position.y.abs() > 1e7
            {
                return Err("Invalid map position.".into());
            }
        }
        for id in &self.expanded_ids {
            validate_id(id)?;
        }
        if let Some(id) = &self.selected_id {
            validate_id(id)?;
        }
        Ok(())
    }
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
