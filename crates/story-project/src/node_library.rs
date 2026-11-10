//! Portable, data-only reusable groups. Library files never own story state.
use crate::*;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use specta::Type;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

const MAX_BYTES: u64 = 16 * 1024 * 1024;
const MAX_SCAN_BYTES: u64 = 64 * 1024 * 1024;

pub fn validate_bundled_library_examples() -> Result<(), String> {
    for (name, bytes) in [
        (
            "surveyor-and-belongings.json",
            include_bytes!("../../../examples/node-library/surveyor-and-belongings.json")
                .as_slice(),
        ),
        (
            "workshop-corner.json",
            include_bytes!("../../../examples/node-library/workshop-corner.json").as_slice(),
        ),
    ] {
        let entry = decode_entry(bytes).map_err(|e| format!("Library example {name}: {e}"))?;
        preview_world_edit(&World::default(), WorldEdit::InsertLibrary { entry })
            .map_err(|e| format!("Library example {name}: {e}"))?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryEntry {
    pub schema_version: u32,
    pub name: String,
    pub description: String,
    pub world: World,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub positions: BTreeMap<String, Position>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryExport {
    pub entry: LibraryEntry,
    pub notices: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryFile {
    pub file_name: String,
    pub name: String,
    pub description: String,
    pub node_count: u32,
    pub connection_count: u32,
    pub sha256: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryListing {
    pub folder_path: String,
    pub files: Vec<LibraryFile>,
    pub warnings: Vec<String>,
}

impl LibraryEntry {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("Unsupported node-library format. Expected schemaVersion 1.".into());
        }
        validate_name(&self.name)?;
        if self.description.chars().count() > 4000 {
            return Err("Library description is too long.".into());
        }
        if self.world.nodes.is_empty() {
            return Err("A library entry needs at least one node.".into());
        }
        if !self.world.ai_context.is_empty() || !self.world.source_annotations.is_empty() {
            return Err("Library entries cannot include story AI bindings or source citations. Export a selection instead.".into());
        }
        self.world
            .validate()
            .map_err(|e| format!("Invalid library group: {e}"))?;
        let layout = Workspace {
            positions: self.positions.clone(),
            ..Workspace::default()
        };
        layout.validate()?;
        if self
            .positions
            .keys()
            .any(|id| !self.world.nodes.iter().any(|n| &n.id == id))
        {
            return Err("Library positions must refer to nodes in this entry.".into());
        }
        Ok(())
    }
}

/// Capture exactly the selection, including only its schema dependency closure.
pub fn export_library_entry(
    source: &World,
    ids: Vec<String>,
    positions: BTreeMap<String, Position>,
    name: String,
    description: String,
) -> Result<LibraryExport, String> {
    source.validate()?;
    let selected: BTreeSet<_> = ids.iter().cloned().collect();
    if selected.is_empty()
        || selected.len() != ids.len()
        || ids
            .iter()
            .any(|id| !source.nodes.iter().any(|n| &n.id == id))
    {
        return Err("Select available nodes to save to the library.".into());
    }
    let mut world = source.clone();
    world.nodes.retain(|n| selected.contains(&n.id));
    world
        .edges
        .retain(|e| selected.contains(&e.source_id) && selected.contains(&e.target_id));
    world.ai_context.clear();
    world.source_annotations.clear();
    let mut notices=vec!["Only selected nodes and their internal connections are included. Images, citations, AI bindings and pending work are excluded.".into()];
    let original_edges = source
        .edges
        .iter()
        .filter(|e| selected.contains(&e.source_id) || selected.contains(&e.target_id))
        .count();
    if original_edges > world.edges.len() {
        notices.push(format!(
            "Omit {} connections to unselected nodes.",
            original_edges - world.edges.len()
        ));
    }
    let map: BTreeMap<_, _> = selected.iter().map(|id| (id.clone(), id.clone())).collect();
    for node in &mut world.nodes {
        for (id, values) in &mut node.components {
            let definition = source.component_types.iter().find(|d| &d.id == id).unwrap();
            for field in &definition.fields {
                if let Some(value) = values.get_mut(&field.key) {
                    let before = value.clone();
                    crate::operations::remap_reference(value, &field.kind, &map);
                    if *value != before {
                        notices.push(format!(
                            "Clear outside reference: {} / {}.",
                            node.name, field.label
                        ));
                    }
                }
            }
        }
    }
    for edge in &mut world.edges {
        let definition = source
            .relation_types
            .iter()
            .find(|d| d.id == edge.type_id)
            .unwrap();
        for field in &definition.fields {
            if let Some(value) = edge.properties.get_mut(&field.key) {
                let before = value.clone();
                crate::operations::remap_reference(value, &field.kind, &map);
                if *value != before {
                    notices.push(format!(
                        "Clear outside connection reference: {} / {}.",
                        definition.name, field.label
                    ));
                }
            }
        }
    }
    // Include operational contracts even when this selection does not use them.
    let core: World = serde_json::from_str(include_str!("../../../schema/world-core.json"))
        .map_err(|e| e.to_string())?;
    let types: BTreeSet<_> = world.nodes.iter().map(|n| n.type_id.clone()).collect();
    let relations: BTreeSet<_> = core
        .relation_types
        .iter()
        .map(|d| d.id.clone())
        .chain(world.edges.iter().map(|e| e.type_id.clone()))
        .collect();
    world.node_types.retain(|d| types.contains(&d.id));
    world.relation_types.retain(|d| relations.contains(&d.id));
    for definition in &mut world.relation_types {
        if !world.edges.iter().any(|e| e.type_id == definition.id) {
            if let Some(contract) = core.relation_types.iter().find(|d| d.id == definition.id) {
                *definition = contract.clone();
            }
        }
    }
    let mut components: BTreeSet<_> = core
        .component_types
        .iter()
        .map(|d| d.id.clone())
        .chain(
            world
                .nodes
                .iter()
                .flat_map(|n| n.components.keys().cloned()),
        )
        .chain(world.node_types.iter().flat_map(|d| d.components.clone()))
        .chain(world.relation_types.iter().flat_map(|d| {
            [d.source_capability.clone(), d.target_capability.clone()]
                .into_iter()
                .flatten()
        }))
        .collect();
    loop {
        let before = components.len();
        let needed: Vec<_> = world
            .component_types
            .iter()
            .filter(|d| components.contains(&d.id))
            .flat_map(|d| &d.fields)
            .chain(world.relation_types.iter().flat_map(|d| &d.fields))
            .filter_map(|f| f.metadata.as_ref()?.reference_capability.clone())
            .collect();
        components.extend(needed);
        if before == components.len() {
            break;
        }
    }
    world.component_types.retain(|d| components.contains(&d.id));
    for field in world
        .component_types
        .iter_mut()
        .flat_map(|d| &mut d.fields)
        .chain(world.relation_types.iter_mut().flat_map(|d| &mut d.fields))
    {
        let before = field.default_value.clone();
        crate::operations::remap_reference(&mut field.default_value, &field.kind, &map);
        if before != field.default_value {
            notices.push(format!("Clear outside default reference: {}.", field.label));
        }
    }
    let positions = positions
        .into_iter()
        .filter(|(id, _)| selected.contains(id))
        .collect();
    let entry = LibraryEntry {
        schema_version: 1,
        name,
        description,
        world,
        positions,
    };
    entry.validate().map_err(|e|format!("Cannot save this selection: {e} Include required referenced nodes or revise their fields/defaults."))?;
    Ok(LibraryExport { entry, notices })
}

// Existing definitions are never replaced. Conflicts are explicit repairable errors.
pub(crate) fn insert_library(
    target: &mut World,
    changes: &mut Vec<String>,
    mut entry: LibraryEntry,
) -> Result<(), String> {
    entry.validate()?;
    // Defaults tied to an entry's nodes need independent definitions per insertion.
    // Propagate new capability IDs through reference filters, presets and links.
    let bound = |field: &FieldDefinition| {
        matches!(
            (&field.kind, &field.default_value),
            (FieldKind::NodeRef, PropertyValue::Text(_))
        ) || matches!((&field.kind,&field.default_value),(FieldKind::NodeRefs,PropertyValue::NodeRefs(ids)) if !ids.is_empty())
    };
    let mut component_ids = BTreeMap::<String, String>::new();
    loop {
        let before = component_ids.len();
        for definition in &entry.world.component_types {
            let depends = definition.fields.iter().any(|f| {
                bound(f)
                    || f.metadata
                        .as_ref()
                        .and_then(|m| m.reference_capability.as_ref())
                        .is_some_and(|id| component_ids.contains_key(id))
            });
            if depends && !component_ids.contains_key(&definition.id) {
                if definition.id.starts_with("core.") || definition.id.starts_with("detail.") {
                    return Err(format!("{} has an instance-specific schema default. Use an unset optional reference default or a custom extension ID for portable library entries.",definition.id));
                }
                component_ids.insert(
                    definition.id.clone(),
                    format!("library.{}", uuid::Uuid::new_v4()),
                );
            }
        }
        if before == component_ids.len() {
            break;
        }
    }
    let remap_fields = |fields: &mut Vec<FieldDefinition>| {
        for field in fields {
            if let Some(meta) = &mut field.metadata {
                if let Some(id) = &mut meta.reference_capability {
                    if let Some(next) = component_ids.get(id) {
                        *id = next.clone();
                    }
                }
            }
        }
    };
    for definition in &mut entry.world.component_types {
        if let Some(id) = component_ids.get(&definition.id) {
            definition.id = id.clone();
        }
        remap_fields(&mut definition.fields);
    }
    let mut type_ids = BTreeMap::new();
    for definition in &mut entry.world.node_types {
        if definition
            .components
            .iter()
            .any(|id| component_ids.contains_key(id))
        {
            let id = format!("library.{}", uuid::Uuid::new_v4());
            type_ids.insert(definition.id.clone(), id.clone());
            definition.id = id;
            for component in &mut definition.components {
                if let Some(id) = component_ids.get(component) {
                    *component = id.clone();
                }
            }
        }
    }
    let mut relation_ids = BTreeMap::new();
    for definition in &mut entry.world.relation_types {
        let depends = definition.fields.iter().any(|f| {
            bound(f)
                || f.metadata
                    .as_ref()
                    .and_then(|m| m.reference_capability.as_ref())
                    .is_some_and(|id| component_ids.contains_key(id))
        }) || [&definition.source_capability, &definition.target_capability]
            .into_iter()
            .flatten()
            .any(|id| component_ids.contains_key(id));
        if depends {
            if definition.id.starts_with("core.") {
                return Err(format!("{} needs an instance-specific relationship definition. Use unset optional reference defaults or a custom relationship ID in the library JSON.",definition.id));
            }
            let id = format!("library.{}", uuid::Uuid::new_v4());
            relation_ids.insert(definition.id.clone(), id.clone());
            definition.id = id;
            for capability in [
                &mut definition.source_capability,
                &mut definition.target_capability,
            ]
            .into_iter()
            .flatten()
            {
                if let Some(id) = component_ids.get(capability) {
                    *capability = id.clone();
                }
            }
            remap_fields(&mut definition.fields);
        }
    }
    for node in &mut entry.world.nodes {
        if let Some(id) = type_ids.get(&node.type_id) {
            node.type_id = id.clone();
        }
        node.components = std::mem::take(&mut node.components)
            .into_iter()
            .map(|(id, values)| (component_ids.get(&id).cloned().unwrap_or(id), values))
            .collect();
    }
    for edge in &mut entry.world.edges {
        if let Some(id) = relation_ids.get(&edge.type_id) {
            edge.type_id = id.clone();
        }
    }
    let old_components = target.component_types.len();
    let old_relations = target.relation_types.len();
    let core: World = serde_json::from_str(include_str!("../../../schema/world-core.json"))
        .map_err(|e| e.to_string())?;
    // Unused native definitions exist only to validate the standalone artifact.
    entry.world.relation_types.retain(|d| {
        !core.relation_types.iter().any(|c| c.id == d.id)
            || entry.world.edges.iter().any(|e| e.type_id == d.id)
            || !target.relation_types.iter().any(|old| old.id == d.id)
    });
    macro_rules! merge {($field:ident) => {for definition in &entry.world.$field {
        if let Some(existing)=target.$field.iter().find(|d|d.id==definition.id) {
            if existing!=definition {return Err(format!("Library schema conflict: {}. The story's definition is different and was not changed. Rename the conflicting definition and its uses in the library JSON, or use a compatible story.",definition.id));}
        } else {changes.push(format!("Add definition: {} ({}).",definition.name,definition.id));target.$field.push(definition.clone());}
    }}}
    merge!(component_types);
    merge!(node_types);
    merge!(relation_types);
    let mapping = crate::operations::paste_nodes(
        target,
        changes,
        entry.world.nodes,
        entry.world.edges,
        false,
    )?;
    for field in target.component_types[old_components..]
        .iter_mut()
        .flat_map(|d| &mut d.fields)
        .chain(
            target.relation_types[old_relations..]
                .iter_mut()
                .flat_map(|d| &mut d.fields),
        )
    {
        crate::operations::remap_reference(&mut field.default_value, &field.kind, &mapping);
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LibrarySettings {
    schema_version: u32,
    folder_path: String,
}

fn settings_lock(root: &Path) -> Result<File, String> {
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("node-library.lock"))
        .map_err(|e| e.to_string())?;
    file.try_lock_exclusive().map_err(|_| {
        "The node library is busy in another app instance. Retry shortly.".to_string()
    })?;
    Ok(file)
}
fn folder(root: &Path) -> Result<PathBuf, String> {
    let path = root.join("node-library.json");
    if !path.exists() {
        let default = root.join("node-library");
        fs::create_dir_all(&default).map_err(|e| e.to_string())?;
        return default.canonicalize().map_err(|e| e.to_string());
    }
    let settings: LibrarySettings = serde_json::from_slice(&read_limited(&path, 256 * 1024)?)
        .map_err(|e| format!("Cannot read node-library.json: {e}. The file was preserved."))?;
    if settings.schema_version != 1 || !Path::new(&settings.folder_path).is_absolute() {
        return Err("Invalid library folder setting. The file was preserved.".into());
    }
    let folder = PathBuf::from(settings.folder_path)
        .canonicalize()
        .map_err(|e| format!("Library folder unavailable: {e}. Choose a folder again."))?;
    if !folder.is_dir() {
        return Err("Choose a library folder.".into());
    }
    Ok(folder)
}
pub fn choose_library_folder(root: &Path, path: &str) -> Result<LibraryListing, String> {
    let _lock = settings_lock(root)?;
    let path = Path::new(path).canonicalize().map_err(|e| e.to_string())?;
    if !path.is_dir() {
        return Err("Choose an existing library folder.".into());
    }
    if path
        .ancestors()
        .any(|parent| parent.join(".project.lock").exists() && parent.join("head.json").exists())
    {
        return Err("Keep the reusable library outside story project folders.".into());
    }
    let settings = LibrarySettings {
        schema_version: 1,
        folder_path: path.to_string_lossy().into(),
    };
    crate::store::atomic_write(
        &root.join("node-library.json"),
        &serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?,
    )?;
    scan(&path)
}
pub fn list_library(root: &Path) -> Result<LibraryListing, String> {
    let _lock = settings_lock(root)?;
    scan(&folder(root)?)
}
fn read_limited(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let mut bytes = vec![];
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("Library JSON exceeds the file-size limit.".into());
    }
    Ok(bytes)
}
fn file_path(folder: &Path, name: &str) -> Result<PathBuf, String> {
    if name.is_empty()
        || name.len() > 240
        || name.starts_with('.')
        || name.contains(['/', '\\', ':'])
        || !name.to_lowercase().ends_with(".json")
        || name.chars().any(char::is_control)
    {
        return Err("Choose a JSON file directly inside the library folder.".into());
    }
    let path = folder.join(name);
    let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink()
        || !meta.is_file()
        || path.canonicalize().map_err(|e| e.to_string())?.parent() != Some(folder)
    {
        return Err(
            "Library entries must be regular JSON files inside the selected folder.".into(),
        );
    }
    Ok(path)
}
fn decode_entry(bytes: &[u8]) -> Result<LibraryEntry, String> {
    let entry: LibraryEntry =
        serde_json::from_slice(bytes).map_err(|e| format!("Invalid library JSON: {e}"))?;
    entry.validate()?;
    Ok(entry)
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn read_library_entry(
    root: &Path,
    name: &str,
    expected_sha256: &str,
) -> Result<LibraryEntry, String> {
    let _lock = settings_lock(root)?;
    let folder = folder(root)?;
    let bytes = read_limited(&file_path(&folder, name)?, MAX_BYTES)?;
    if sha(&bytes) != expected_sha256 {
        return Err(
            "This library file changed. Refresh the library and review the current entry.".into(),
        );
    }
    decode_entry(&bytes)
}
fn scan(folder: &Path) -> Result<LibraryListing, String> {
    let mut names = vec![];
    let mut warnings = vec![];
    for item in fs::read_dir(folder).map_err(|e| e.to_string())? {
        let item = item.map_err(|e| e.to_string())?;
        let Some(name) = item.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !name.starts_with('.') && name.to_lowercase().ends_with(".json") {
            names.push(name);
        }
        if names.len() > 500 {
            warnings.push(
                "Only the first 500 JSON entries are shown. Use a smaller library folder.".into(),
            );
            break;
        }
    }
    names.sort();
    names.truncate(500);
    let mut used = 0;
    let mut files = vec![];
    for name in names {
        let result = (|| {
            let path = file_path(folder, &name)?;
            let size = fs::metadata(&path).map_err(|e| e.to_string())?.len();
            if size > MAX_BYTES {
                return Err("Library JSON exceeds 16 MiB.".into());
            }
            if used + size > MAX_SCAN_BYTES {
                return Err(
                    "The 64 MiB library scan budget is full. Use a smaller library folder.".into(),
                );
            }
            used += size;
            let bytes = read_limited(&path, MAX_BYTES)?;
            let entry = decode_entry(&bytes)?;
            Ok((entry, sha(&bytes)))
        })();
        files.push(match result {
            Ok((entry, hash)) => LibraryFile {
                file_name: name,
                name: entry.name,
                description: entry.description,
                node_count: entry.world.nodes.len() as u32,
                connection_count: entry.world.edges.len() as u32,
                sha256: hash,
                error: None,
            },
            Err(error) => LibraryFile {
                file_name: name.clone(),
                name,
                description: String::new(),
                node_count: 0,
                connection_count: 0,
                sha256: String::new(),
                error: Some(error),
            },
        });
    }
    Ok(LibraryListing {
        folder_path: folder.to_string_lossy().into(),
        files,
        warnings,
    })
}
pub fn save_library_entry(
    root: &Path,
    request_id: &str,
    entry: &LibraryEntry,
) -> Result<String, String> {
    save_with_hook(root, request_id, entry, |_| Ok(()))
}
fn save_with_hook(
    root: &Path,
    request_id: &str,
    entry: &LibraryEntry,
    hook: impl Fn(&str) -> Result<(), String>,
) -> Result<String, String> {
    validate_id(request_id)?;
    entry.validate()?;
    let _lock = settings_lock(root)?;
    let folder = folder(root)?;
    let name = format!("{request_id}.json");
    let target = folder.join(&name);
    let bytes = serde_json::to_vec_pretty(entry).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Library JSON exceeds 16 MiB.".into());
    }
    if target.exists() {
        if read_limited(&file_path(&folder, &name)?, MAX_BYTES)? == bytes {
            return Ok(name);
        }
        return Err("This library save ID already contains different content. The original file was preserved.".into());
    }
    let stage = folder.join(format!(".{}.tmp", uuid::Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&stage)
        .map_err(|e| e.to_string())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    hook("before-publish")?;
    crate::store::publish_new_path(&stage, &target)?;
    hook("after-publish")?;
    Ok(name)
}

#[cfg(test)]
mod tests;
