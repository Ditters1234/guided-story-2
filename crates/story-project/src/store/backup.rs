//! Consistent, checksummed folder backups. Publication never replaces an existing folder.
use super::*;
use crate::*;
use serde_json::Value;
use specta::Type;
use std::collections::BTreeSet;

const MAX_FILES: usize = 25_000;
const MAX_BYTES: u64 = u32::MAX as u64;
const FOLDERS: &[&str] = &[
    "revisions",
    "worlds",
    "stories",
    "passages",
    "memories",
    "media",
    "images",
    "backups",
    "proposals",
    "passage-drafts",
    "passage-draft-versions",
    "memory-drafts",
    "memory-draft-versions",
    "media-drafts",
    "image-prompt-drafts",
    "image-prompt-versions",
];
const RECORDS: &[&str] = &[
    "revisions",
    "worlds",
    "stories",
    "passages",
    "memories",
    "media",
    "passage-draft-versions",
    "memory-draft-versions",
    "image-prompt-versions",
];

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupReport {
    pub fingerprint: String,
    pub folder_path: String,
    pub title: String,
    pub story_id: String,
    pub revision: u32,
    pub files: u32,
    pub bytes: u32,
    pub omitted_files: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Entry {
    path: String,
    bytes: u64,
    sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BackupManifest {
    format: String,
    schema_version: u32,
    operation_id: String,
    story_id: String,
    title: String,
    revision: u32,
    omitted_files: u32,
    entries: Vec<Entry>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Transfer {
    schema_version: u32,
    kind: String,
    operation_id: String,
    input: String,
}

fn plain(path: &Path, directory: bool) -> Result<(), String> {
    let meta = fs::symlink_metadata(path)
        .map_err(|e| format!("Could not inspect {}: {e}", path.display()))?;
    #[cfg(windows)]
    let linked = {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    };
    #[cfg(not(windows))]
    let linked = meta.file_type().is_symlink();
    if linked
        || if directory {
            !meta.is_dir()
        } else {
            !meta.is_file()
        }
    {
        return Err(format!(
            "Backups require ordinary files and folders; linked or unsupported path: {}",
            path.display()
        ));
    }
    Ok(())
}
fn allowed(path: &str) -> bool {
    if ["head.json", "workspace.json", "editor-draft.json"].contains(&path) {
        return true;
    }
    let parts = path.split('/').collect::<Vec<_>>();
    if parts.len() != 2 || !FOLDERS.contains(&parts[0]) {
        return false;
    }
    let Some((stem, ext)) = parts[1].rsplit_once('.') else {
        return false;
    };
    if parts[0] == "backups" {
        let stem = stem
            .strip_prefix("damaged-")
            .or_else(|| stem.strip_prefix("draft-"))
            .unwrap_or(stem);
        return ext == "json" && validate_id(stem).is_ok();
    }
    validate_id(stem).is_ok()
        && if parts[0] == "images" {
            ["png", "jpg", "webp"].contains(&ext)
        } else {
            ext == "json"
        }
}
fn read(root: &Path, relative: &str) -> Result<Vec<u8>, String> {
    if !allowed(relative) {
        return Err("Unsupported or unsafe backup entry path.".into());
    }
    plain(root, true)?;
    if let Some((folder, _)) = relative.split_once('/') {
        plain(&root.join(folder), true)?;
    }
    let path = root.join(relative);
    plain(&path, false)?;
    read_bytes(&path)
}
fn inventory(root: &Path, strict: bool) -> Result<(Vec<Entry>, u32), String> {
    plain(root, true)?;
    let mut paths = vec![];
    let mut omitted = 0u32;
    for item in fs::read_dir(root).map_err(|e| e.to_string())? {
        let path = item.map_err(|e| e.to_string())?.path();
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or("Unsupported project filename.")?
            .to_string();
        let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        // Never follow junctions/symlinks, including ones in otherwise excluded locations.
        plain(&path, meta.is_dir())?;
        if name == ".project.lock" || name == ".restore-receipt.json" {
            continue;
        }
        if name.starts_with('.') && name.ends_with(".tmp") {
            if strict {
                return Err("Unexpected temporary file in backup project.".into());
            }
            omitted += 1;
            continue;
        }
        if FOLDERS.contains(&name.as_str()) {
            plain(&path, true)?;
            for item in fs::read_dir(&path).map_err(|e| e.to_string())? {
                let child = item.map_err(|e| e.to_string())?.path();
                plain(&child, false)?;
                let file = child
                    .file_name()
                    .and_then(|s| s.to_str())
                    .ok_or("Unsupported project filename.")?;
                if file.starts_with('.') && file.ends_with(".tmp") {
                    if strict {
                        return Err("Unexpected temporary file in backup project.".into());
                    }
                    omitted += 1;
                    continue;
                }
                let relative = format!("{name}/{file}");
                if !allowed(&relative) {
                    return Err(format!(
                        "Unsupported project file: {relative}. Files were kept."
                    ));
                }
                paths.push(relative);
                if paths.len() > MAX_FILES {
                    return Err("Backup exceeds 25,000 files. No backup was published.".into());
                }
            }
        } else if allowed(&name) {
            paths.push(name);
        } else if strict {
            return Err(format!("Unexpected file in backup project: {name}"));
        } else {
            omitted += 1;
        }
    }
    if paths.len() > MAX_FILES {
        return Err("Backup exceeds 25,000 files.".into());
    }
    paths.sort();
    let mut entries = vec![];
    let mut total = 0u64;
    for path in paths {
        let bytes = read(root, &path)?;
        total += bytes.len() as u64;
        if total > MAX_BYTES {
            return Err("Backup exceeds 4 GiB. No backup was published.".into());
        }
        entries.push(Entry {
            path,
            bytes: bytes.len() as u64,
            sha256: digest(&bytes),
        });
    }
    Ok((entries, omitted))
}
fn references(
    value: &Value,
    context: &str,
    refs: &mut BTreeSet<(String, String, String)>,
    images: &mut BTreeMap<String, ImageFile>,
) -> Result<(), String> {
    match value {
        Value::Object(map) => {
            if map.len() == 2 && map.contains_key("id") && map.contains_key("sha256") {
                let reference: RecordRef =
                    serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
                validate_id(&reference.id)?;
                let folder = match context {
                    "world" | "worldRef" | "sourceWorldRef" | "beforeWorld" | "afterWorld"
                    | "narrationSource" => "worlds",
                    "storyRef" | "index" => "stories",
                    "record" | "previousVersion" | "evidenceSource" | "passage" | "source" => {
                        "passages"
                    }
                    "memoryReference" => "memories",
                    "mediaReference" => "media",
                    _ => return Err(format!("Unsupported backup reference field: {context}")),
                };
                refs.insert((folder.into(), reference.id, reference.sha256));
                if refs.len() > 1_000_000 {
                    return Err("Too many backup references.".into());
                }
            } else if map.contains_key("id")
                && map.contains_key("sha256")
                && map.contains_key("extension")
            {
                let file: ImageFile =
                    serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
                let path = format!("images/{}.{}", file.id, file.extension);
                if !allowed(&path) {
                    return Err("Invalid backup image path.".into());
                }
                if images.get(&path).is_some_and(|old| old != &file) {
                    return Err("Conflicting image metadata.".into());
                }
                images.insert(path, file);
            } else {
                for (key, item) in map {
                    if key != "graph" && key != "workflow" {
                        let key = match context {
                            "memories" => "memoryReference",
                            "media" => "mediaReference",
                            _ => key.as_str(),
                        };
                        references(item, key, refs, images)?;
                    }
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                references(value, context, refs, images)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn audit(root: &Path, entries: &[Entry]) -> Result<(Manifest, Head), String> {
    let head: Head = decode(&read(root, "head.json")?)?;
    if head.schema_version != SCHEMA {
        return Err("Unsupported project head schema.".into());
    }
    let mut records = BTreeSet::new();
    let mut refs = BTreeSet::from([(
        "revisions".into(),
        head.revision.id.clone(),
        head.revision.sha256.clone(),
    )]);
    let mut images = BTreeMap::new();
    let mut identities = BTreeMap::new();
    let mut associations = BTreeSet::new();
    let head_path = format!("revisions/{}.json", head.revision.id);
    let head_bytes = read(root, &head_path)?;
    if digest(&head_bytes) != head.revision.sha256 {
        return Err("Backup head checksum mismatch.".into());
    }
    let manifest: Manifest = decode(&head_bytes)?;
    validate_id(&manifest.story_id)?;
    validate_name(&manifest.title)?;
    for entry in entries {
        let Some((folder, name)) = entry.path.split_once('/') else {
            if entry.path == "workspace.json" {
                decode::<Workspace>(&read(root, &entry.path)?)?.validate()?;
            }
            if entry.path == "editor-draft.json" {
                let d: EditorDraft = decode(&read(root, &entry.path)?)?;
                if d.schema_version != SCHEMA {
                    return Err("Unsupported editor draft schema.".into());
                }
            }
            continue;
        };
        if folder == "images" || folder == "backups" {
            continue;
        }
        let bytes = read(root, &entry.path)?;
        let value: Value = decode(&bytes)?;
        if RECORDS.contains(&folder) {
            records.insert((
                folder.to_string(),
                name.trim_end_matches(".json").to_string(),
                entry.sha256.clone(),
            ));
        }
        if folder == "worlds" {
            decode::<World>(&bytes)?.validate()?;
        } else {
            let expected_schema = if folder == "revisions" {
                u64::from(SCHEMA)
            } else {
                1
            };
            if value.get("schemaVersion").and_then(Value::as_u64) != Some(expected_schema) {
                return Err(format!("Unsupported record schema: {}", entry.path));
            }
            if value
                .get("storyId")
                .and_then(Value::as_str)
                .is_some_and(|id| id != manifest.story_id)
            {
                return Err("Backup contains a draft from a different story.".into());
            }
            if [
                "proposals",
                "passage-drafts",
                "memory-drafts",
                "media-drafts",
                "image-prompt-drafts",
            ]
            .contains(&folder)
            {
                if value.get("id").and_then(Value::as_str) != Some(name.trim_end_matches(".json"))
                    || value.get("storyId").and_then(Value::as_str)
                        != Some(manifest.story_id.as_str())
                    || value
                        .get("version")
                        .and_then(Value::as_u64)
                        .is_none_or(|v| v == 0 || v > u32::MAX as u64)
                {
                    return Err("Invalid saved draft identity or version.".into());
                }
            }
            match folder {
                "revisions" => {
                    let m: Manifest = decode(&bytes)?;
                    validate_name(&m.title)?;
                    if m.story_id != manifest.story_id {
                        return Err("Mixed story revisions in backup.".into());
                    }
                }
                "stories" => {
                    let index: StoryIndex = decode(&bytes)?;
                    index.validate()?;
                    for p in index.passages {
                        associations.insert(("passages", p.record.id, p.id, p.chapter_id));
                    }
                    for (chapter, r) in index.memories {
                        associations.insert(("memories", r.id, String::new(), chapter));
                    }
                    for (id, r) in index.media {
                        associations.insert(("media", r.id, id, String::new()));
                    }
                }
                "passages" => {
                    let p: AcceptedPassage = decode(&bytes)?;
                    validate_id(&p.id)?;
                    validate_id(&p.chapter_id)?;
                    validate_id(&p.source_story_id)?;
                    identities.insert(
                        (
                            folder.to_string(),
                            name.trim_end_matches(".json").to_string(),
                        ),
                        (p.id, p.chapter_id),
                    );
                }
                "memories" => {
                    let m: ChapterMemory = decode(&bytes)?;
                    validate_id(&m.id)?;
                    validate_id(&m.chapter_id)?;
                    identities.insert(
                        (
                            folder.to_string(),
                            name.trim_end_matches(".json").to_string(),
                        ),
                        (String::new(), m.chapter_id),
                    );
                }
                "media" => {
                    let m: MediaAsset = decode(&bytes)?;
                    validate_id(&m.id)?;
                    identities.insert(
                        (
                            folder.to_string(),
                            name.trim_end_matches(".json").to_string(),
                        ),
                        (m.id, String::new()),
                    );
                }
                "media-drafts" => {
                    let m: MediaDraft = decode(&bytes)?;
                    validate_id(&m.id)?;
                }
                _ => {}
            }
        }
        references(&value, "", &mut refs, &mut images)?;
    }
    let any_records: BTreeSet<_> = records.iter().map(|r| (r.1.clone(), r.2.clone())).collect();
    for reference in refs {
        if !records.contains(&reference)
            && !(reference.0.is_empty()
                && any_records.contains(&(reference.1.clone(), reference.2.clone())))
        {
            return Err(format!(
                "Missing or changed referenced JSON record: {}/{}",
                reference.0, reference.1
            ));
        }
    }
    for (path, file) in images {
        crate::media::verify_backup_image(&read(root, &path)?, &file)?;
    }
    // Check associations that a generic hash reference alone cannot establish.
    for (folder, record, id, chapter) in associations {
        if identities.get(&(folder.to_string(), record)) != Some(&(id, chapter)) {
            return Err("Historical index identity mismatch.".into());
        }
    }
    Ok((manifest, head))
}
fn report(root: &Path, m: &BackupManifest) -> Result<BackupReport, String> {
    Ok(BackupReport {
        fingerprint: digest(&encode(m)?),
        folder_path: root.to_string_lossy().into_owned(),
        title: m.title.clone(),
        story_id: m.story_id.clone(),
        revision: m.revision,
        files: m.entries.len() as u32,
        bytes: m
            .entries
            .iter()
            .map(|e| e.bytes)
            .sum::<u64>()
            .try_into()
            .map_err(|_| "Backup exceeds its size limit.")?,
        omitted_files: m.omitted_files,
    })
}
fn load_backup(root: &Path) -> Result<BackupManifest, String> {
    plain(root, true)?;
    plain(&root.join("backup.json"), false)?;
    let m: BackupManifest = decode(&read_bytes(&root.join("backup.json"))?)?;
    if m.format != "guided-story-folder-backup"
        || m.schema_version != 1
        || m.entries.len() > MAX_FILES
    {
        return Err("Unsupported or oversized story backup.".into());
    }
    validate_id(&m.operation_id)?;
    validate_id(&m.story_id)?;
    validate_name(&m.title)?;
    let mut seen = BTreeSet::new();
    let mut bytes = 0u64;
    for e in &m.entries {
        if !allowed(&e.path)
            || !seen.insert(e.path.to_ascii_lowercase())
            || e.bytes > MAX_FILE_BYTES
            || e.sha256.len() != 64
            || !e.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("Unsafe, duplicate or invalid backup entry.".into());
        }
        bytes = bytes.checked_add(e.bytes).ok_or("Backup size overflow.")?;
    }
    if bytes > MAX_BYTES {
        return Err("Backup exceeds 4 GiB.".into());
    }
    Ok(m)
}
pub fn inspect_backup(root: &Path) -> Result<BackupReport, String> {
    let m = load_backup(root)?;
    let project = root.join("project");
    let (entries, _) = inventory(&project, true)?;
    if entries != m.entries {
        return Err(
            "Backup files are missing, changed or unexpected. No project was restored.".into(),
        );
    }
    let (saved, _) = audit(&project, &entries)?;
    if saved.story_id != m.story_id || saved.title != m.title || saved.revision != m.revision {
        return Err("Backup metadata does not match its project.".into());
    }
    report(root, &m)
}
fn parent_folder(parent: &Path, source: &Path) -> Result<PathBuf, String> {
    plain(parent, true)?;
    let parent = parent.canonicalize().map_err(|e| e.to_string())?;
    let source = source.canonicalize().map_err(|e| e.to_string())?;
    if parent.starts_with(&source) {
        return Err("Choose a destination outside the source project or backup.".into());
    }
    Ok(parent)
}
fn transfer_lock(parent: &Path) -> Result<File, String> {
    let path = parent.join(".guided-story-transfer.lock");
    if path.exists() {
        plain(&path, false)?;
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| e.to_string())?;
    FileExt::try_lock_exclusive(&file).map_err(|_| {
        "Another backup or restore is using this destination. Try again when it finishes."
    })?;
    Ok(file)
}
fn stage(parent: &Path, id: &str, kind: &str, input: &str) -> Result<PathBuf, String> {
    let path = parent.join(format!(".guided-story-{kind}-{id}.incomplete"));
    let receipt = Transfer {
        schema_version: 1,
        kind: kind.into(),
        operation_id: id.into(),
        input: input.into(),
    };
    if path.exists() {
        plain(&path, true)?;
        plain(&path.join("transfer.json"), false)?;
        let old: Transfer = decode(&read_bytes(&path.join("transfer.json"))?)?;
        if old.schema_version != 1
            || old.kind != kind
            || old.operation_id != id
            || old.input != input
        {
            return Err("Incomplete transfer belongs to different input; files were kept.".into());
        }
    } else {
        fs::create_dir(&path).map_err(|e| e.to_string())?;
        atomic_write(&path.join("transfer.json"), &encode(&receipt)?)?;
    }
    // Only our UUID-named atomic-write remnants may be removed from this owned staging tree.
    let mut folders = vec![path.clone(), path.join("project")];
    folders.extend(FOLDERS.iter().map(|f| path.join("project").join(f)));
    for folder in folders {
        if folder.exists() {
            plain(&folder, true)?;
            for entry in fs::read_dir(&folder).map_err(|e| e.to_string())? {
                let item = entry.map_err(|e| e.to_string())?.path();
                if let Some(name) = item.file_name().and_then(|s| s.to_str()) {
                    if name
                        .strip_prefix('.')
                        .and_then(|s| s.strip_suffix(".tmp"))
                        .is_some_and(|id| validate_id(id).is_ok())
                    {
                        plain(&item, false)?;
                        fs::remove_file(item).map_err(|e| e.to_string())?;
                    }
                }
            }
        }
    }
    Ok(path)
}
fn copy_entries(source: &Path, destination: &Path, entries: &[Entry]) -> Result<(), String> {
    if !destination.exists() {
        fs::create_dir(destination).map_err(|e| e.to_string())?;
    }
    plain(destination, true)?;
    // Empty structural folders are required by later commits, even when they had no files to list.
    for folder in FOLDERS {
        let path = destination.join(folder);
        if !path.exists() {
            fs::create_dir(&path).map_err(|e| e.to_string())?;
        }
        plain(&path, true)?;
    }
    for entry in entries {
        let bytes = read(source, &entry.path)?;
        if bytes.len() as u64 != entry.bytes || digest(&bytes) != entry.sha256 {
            return Err("Source changed during the copy. No destination was published.".into());
        }
        let path = destination.join(&entry.path);
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir(parent).map_err(|e| e.to_string())?;
            }
            plain(parent, true)?;
        }
        if path.exists() {
            plain(&path, false)?;
            if read_bytes(&path)? == bytes {
                continue;
            }
            return Err(
                "An incomplete copy changed; files were kept. Start a new transfer.".into(),
            );
        }
        let temp = write_temp(&path, &bytes)?;
        fs::rename(temp, path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn publish(stage: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists() {
        return Err("The destination already exists; it was not replaced.".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};
        let source: Vec<u16> = stage.as_os_str().encode_wide().chain(Some(0)).collect();
        let target: Vec<u16> = destination
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        // No replace flag: publication must fail even if an empty destination appeared meanwhile.
        if unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), MOVEFILE_WRITE_THROUGH) } == 0 {
            return Err(format!(
                "Copy remains in its incomplete folder: {}",
                std::io::Error::last_os_error()
            ));
        }
    }
    #[cfg(not(windows))]
    fs::rename(stage, destination)
        .map_err(|e| format!("Copy remains in its incomplete folder: {e}"))?;
    #[cfg(not(windows))]
    File::open(destination.parent().ok_or("Missing destination parent.")?)
        .and_then(|f| f.sync_all())
        .map_err(|e| e.to_string())?;
    Ok(())
}
impl Project {
    pub fn export_backup(
        &self,
        parent: &Path,
        id: &str,
        expected_revision: u32,
    ) -> Result<BackupReport, String> {
        self.export_backup_with_hook(parent, id, expected_revision, |_| Ok(()))
    }
    fn export_backup_with_hook(
        &self,
        parent: &Path,
        id: &str,
        expected_revision: u32,
        hook: impl Fn(&str) -> Result<(), String>,
    ) -> Result<BackupReport, String> {
        validate_id(id)?;
        let view = self.view()?;
        let parent = parent_folder(parent, self.root_path())?;
        let _lock = transfer_lock(&parent)?;
        let destination = parent.join(format!("guided-story-backup-{id}"));
        if destination.exists() {
            let old = load_backup(&destination)?;
            if old.operation_id != id
                || old.story_id != view.story_id
                || old.revision != expected_revision
            {
                return Err("Backup operation was reused for different content.".into());
            }
            return inspect_backup(&destination);
        }
        if view.revision != expected_revision {
            return Err("The story changed before backup. Save and start a new backup.".into());
        }
        let (entries, omitted) = inventory(self.root_path(), false)?;
        audit(self.root_path(), &entries)?;
        let m = BackupManifest {
            format: "guided-story-folder-backup".into(),
            schema_version: 1,
            operation_id: id.into(),
            story_id: view.story_id,
            title: view.title,
            revision: view.revision,
            omitted_files: omitted,
            entries,
        };
        let input = digest(&encode(&m)?);
        let staging = stage(&parent, id, "backup", &input)?;
        copy_entries(self.root_path(), &staging.join("project"), &m.entries)?;
        hook("copied")?;
        atomic_write(&staging.join("backup.json"), &encode(&m)?)?;
        inspect_backup(&staging)?;
        hook("validated")?;
        publish(&staging, &destination)?;
        hook("published")?;
        report(&destination, &m)
    }
}
pub fn restore_backup(
    source: &Path,
    parent: &Path,
    id: &str,
    fingerprint: &str,
) -> Result<BackupReport, String> {
    restore_with_hook(source, parent, id, fingerprint, |_| Ok(()))
}
fn restore_with_hook(
    source: &Path,
    parent: &Path,
    id: &str,
    fingerprint: &str,
    hook: impl Fn(&str) -> Result<(), String>,
) -> Result<BackupReport, String> {
    validate_id(id)?;
    let m = load_backup(source)?;
    let input = digest(&encode(&m)?);
    if input != fingerprint {
        return Err("The selected backup changed. Choose it again to review and validate its current contents.".into());
    }
    let parent = parent_folder(parent, source)?;
    let _lock = transfer_lock(&parent)?;
    let destination = parent.join(format!("guided-story-restored-{id}"));
    if destination.exists() {
        plain(&destination, true)?;
        plain(&destination.join(".restore-receipt.json"), false)?;
        let receipt: Transfer = decode(&read_bytes(&destination.join(".restore-receipt.json"))?)?;
        if receipt.schema_version != 1
            || receipt.kind != "restore"
            || receipt.operation_id != id
            || receipt.input != input
        {
            return Err("Existing destination belongs to different work; it was kept.".into());
        }
        return report(&destination, &m);
    }
    inspect_backup(source)?;
    let staging = stage(&parent, id, "restore", &input)?;
    let project = staging.join("project");
    copy_entries(&source.join("project"), &project, &m.entries)?;
    hook("copied")?;
    let (entries, _) = inventory(&project, true)?;
    if entries != m.entries {
        return Err("Restored copy differs from its manifest.".into());
    }
    audit(&project, &entries)?;
    let opened = Project::open(&project)?;
    opened.view()?;
    drop(opened);
    let receipt = Transfer {
        schema_version: 1,
        kind: "restore".into(),
        operation_id: id.into(),
        input,
    };
    atomic_write(&project.join(".restore-receipt.json"), &encode(&receipt)?)?;
    hook("validated")?;
    publish(&project, &destination)?;
    hook("published")?;
    report(&destination, &m)
}

#[cfg(test)]
mod tests;
