use crate::model::*;
use fs2::FileExt;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

const SCHEMA: u32 = 1;
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Head {
    schema_version: u32,
    revision: RecordRef,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Receipt {
    fingerprint: String,
    revision: u32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    story_id: String,
    title: String,
    revision: u32,
    world_ref: RecordRef,
    receipts: BTreeMap<String, Receipt>,
}

/// The held OS lock grants this backend exclusive write access to the project.
pub struct Project {
    root: PathBuf,
    _lock: File,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(value).map_err(|e| format!("Could not encode project data: {e}"))
}
fn read_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|e| format!("Could not read {}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err("Project record exceeds the size limit.".into());
    }
    Ok(bytes)
}
fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    serde_json::from_slice(bytes)
        .map_err(|e| format!("Invalid project JSON; saved data has not been replaced: {e}"))
}

fn write_temp(path: &Path, bytes: &[u8]) -> Result<PathBuf, String> {
    let temp = path.with_file_name(format!(".{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        Ok(temp.clone())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let from: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // Both NUL-terminated buffers outlive the call and refer to same-directory files.
    if unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        return Err(format!(
            "Could not publish the save; previous revision is unchanged: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    fs::rename(source, destination).map_err(|e| e.to_string())?;
    if let Some(parent) = destination.parent() {
        File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp = write_temp(path, bytes)?;
    let result = replace_file(&temp, path);
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

impl Project {
    fn lock(root: &Path) -> Result<Self, String> {
        let root = root
            .canonicalize()
            .map_err(|e| format!("Could not open project folder: {e}"))?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join(".project.lock"))
            .map_err(|e| e.to_string())?;
        FileExt::try_lock_exclusive(&lock).map_err(|_| {
            "This project is already open in another window or application instance.".to_string()
        })?;
        Ok(Self { root, _lock: lock })
    }

    pub fn create(
        root: &Path,
        title: String,
        world: World,
        request_id: String,
    ) -> Result<Self, String> {
        validate_name(&title)?;
        validate_id(&request_id)?;
        world.validate()?;
        fs::create_dir(root).map_err(|e| {
            format!("Choose a new project folder; existing folders are never overwritten: {e}")
        })?;
        let project = Self::lock(root)?;
        for folder in ["revisions", "worlds", "backups"] {
            fs::create_dir(project.root.join(folder)).map_err(|e| e.to_string())?;
        }
        let fingerprint = Self::fingerprint(&title, &world)?;
        let receipts = BTreeMap::from([(
            request_id,
            Receipt {
                fingerprint,
                revision: 0,
            },
        )]);
        project.publish(
            Uuid::new_v4().to_string(),
            title,
            0,
            world,
            receipts,
            |_| Ok(()),
        )?;
        Ok(project)
    }

    pub fn open(root: &Path) -> Result<Self, String> {
        let project = Self::lock(root)?;
        project.view()?;
        Ok(project)
    }

    pub fn folder_path(&self) -> String {
        self.root.to_string_lossy().into_owned()
    }

    fn record_path(&self, folder: &str, reference: &RecordRef) -> Result<PathBuf, String> {
        validate_id(&reference.id)?;
        if reference.sha256.len() != 64 || !reference.sha256.bytes().all(|x| x.is_ascii_hexdigit())
        {
            return Err("Invalid project record checksum.".into());
        }
        Ok(self
            .root
            .join(folder)
            .join(format!("{}.json", reference.id)))
    }

    fn read_record<T: DeserializeOwned>(
        &self,
        folder: &str,
        reference: &RecordRef,
    ) -> Result<T, String> {
        let bytes = read_bytes(&self.record_path(folder, reference)?)?;
        if digest(&bytes) != reference.sha256 {
            return Err("A project record has changed or is incomplete. Use recovery; the original files have been kept.".into());
        }
        decode(&bytes)
    }

    fn read_head(&self) -> Result<Head, String> {
        decode(&read_bytes(&self.root.join("head.json"))?)
    }

    fn load(&self, head: &Head) -> Result<(Manifest, World), String> {
        if head.schema_version != SCHEMA {
            return Err("Unsupported project schema version.".into());
        }
        let manifest: Manifest = self.read_record("revisions", &head.revision)?;
        if manifest.schema_version != SCHEMA {
            return Err("Unsupported story schema version.".into());
        }
        validate_id(&manifest.story_id)?;
        validate_name(&manifest.title)?;
        let world: World = self.read_record("worlds", &manifest.world_ref)?;
        world.validate()?;
        Ok((manifest, world))
    }

    fn to_view(&self, manifest: Manifest, world: World) -> ProjectView {
        ProjectView {
            schema_version: SCHEMA,
            story_id: manifest.story_id,
            title: manifest.title,
            revision: manifest.revision,
            folder_path: self.folder_path(),
            world_ref: manifest.world_ref,
            world,
        }
    }

    pub fn view(&self) -> Result<ProjectView, String> {
        let (manifest, world) = self.load(&self.read_head()?)?;
        Ok(self.to_view(manifest, world))
    }

    fn fingerprint(title: &str, world: &World) -> Result<String, String> {
        Ok(digest(&encode(&(title, world))?))
    }

    pub fn commit(
        &self,
        expected_revision: u32,
        request_id: String,
        title: String,
        world: World,
    ) -> Result<CommitResult, String> {
        self.commit_with_hook(expected_revision, request_id, title, world, |_| Ok(()))
    }

    fn commit_with_hook(
        &self,
        expected_revision: u32,
        request_id: String,
        title: String,
        world: World,
        hook: impl Fn(&str) -> Result<(), String>,
    ) -> Result<CommitResult, String> {
        validate_id(&request_id)?;
        validate_name(&title)?;
        world.validate()?;
        let fingerprint = Self::fingerprint(&title, &world)?;
        let (mut manifest, current_world) = self.load(&self.read_head()?)?;
        if let Some(receipt) = manifest.receipts.get(&request_id) {
            if receipt.fingerprint != fingerprint {
                return Err("This request ID was already used for different changes.".into());
            }
            let committed_revision = receipt.revision;
            return Ok(CommitResult {
                project: self.to_view(manifest, current_world),
                committed_revision,
                replayed: true,
            });
        }
        if manifest.revision != expected_revision {
            return Err(
                "The project changed. Reload its saved state before applying these edits.".into(),
            );
        }
        if manifest.receipts.len() >= 10000 {
            return Err("This prototype has reached its save-receipt limit. Export a backup before continuing.".into());
        }
        let revision = manifest
            .revision
            .checked_add(1)
            .ok_or("Revision limit reached.")?;
        manifest.receipts.insert(
            request_id,
            Receipt {
                fingerprint,
                revision,
            },
        );
        self.publish(
            manifest.story_id,
            title,
            revision,
            world,
            manifest.receipts,
            hook,
        )?;
        Ok(CommitResult {
            project: self.view()?,
            committed_revision: revision,
            replayed: false,
        })
    }

    fn write_record<T: Serialize>(&self, folder: &str, value: &T) -> Result<RecordRef, String> {
        let bytes = encode(value)?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err("Project record exceeds its size limit.".into());
        }
        let reference = RecordRef {
            id: Uuid::new_v4().to_string(),
            sha256: digest(&bytes),
        };
        let path = self.record_path(folder, &reference)?;
        let temp = write_temp(&path, &bytes)?;
        fs::rename(&temp, &path).map_err(|e| e.to_string())?;
        Ok(reference)
    }

    fn publish(
        &self,
        story_id: String,
        title: String,
        revision: u32,
        world: World,
        receipts: BTreeMap<String, Receipt>,
        hook: impl Fn(&str) -> Result<(), String>,
    ) -> Result<(), String> {
        hook("before-world")?;
        let world_ref = self.write_record("worlds", &world)?;
        hook("after-world")?;
        let manifest = Manifest {
            schema_version: SCHEMA,
            story_id,
            title,
            revision,
            world_ref,
            receipts,
        };
        let next = Head {
            schema_version: SCHEMA,
            revision: self.write_record("revisions", &manifest)?,
        };
        hook("after-manifest")?;
        // Validate complete referents before publishing a pointer to them.
        self.load(&next)?;
        let head_path = self.root.join("head.json");
        if head_path.exists() {
            let previous = self.read_head()?;
            self.load(&previous)?;
            atomic_write(
                &self
                    .root
                    .join("backups")
                    .join(format!("{}.json", previous.revision.id)),
                &encode(&previous)?,
            )?;
        }
        hook("before-head")?;
        atomic_write(&head_path, &encode(&next)?)?;
        hook("after-head")?;
        Ok(())
    }

    /// Recovery is explicit. Normal open never replaces invalid JSON with defaults.
    pub fn recover(root: &Path) -> Result<Self, String> {
        let project = Self::lock(root)?;
        if project.view().is_ok() {
            return Ok(project);
        }
        let mut candidates = Vec::new();
        for entry in fs::read_dir(project.root.join("backups"))
            .map_err(|_| "No previous save is available for recovery.")?
        {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().is_some_and(|x| x == "json") {
                if let Ok(bytes) = read_bytes(&path) {
                    if let Ok(head) = decode::<Head>(&bytes) {
                        if let Ok((manifest, _)) = project.load(&head) {
                            candidates.push((manifest.revision, head));
                        }
                    }
                }
            }
        }
        let (_, head) = candidates
            .into_iter()
            .max_by_key(|(revision, _)| *revision)
            .ok_or(
                "No complete previous save is available. The project has been kept unchanged.",
            )?;
        let path = project.root.join("head.json");
        if let Ok(bytes) = read_bytes(&path) {
            atomic_write(
                &project
                    .root
                    .join("backups")
                    .join(format!("damaged-{}.json", Uuid::new_v4())),
                &bytes,
            )?;
        }
        atomic_write(&path, &encode(&head)?)?;
        project.view()?;
        Ok(project)
    }

    pub fn load_workspace(&self) -> Result<Workspace, String> {
        let path = self.root.join("workspace.json");
        if !path.exists() {
            return Ok(Workspace::default());
        }
        let workspace: Workspace = decode(&read_bytes(&path)?)?;
        workspace.validate()?;
        Ok(workspace)
    }

    pub fn load_draft(&self) -> Result<Option<EditorDraft>, String> {
        let path = self.root.join("editor-draft.json");
        if !path.exists() {
            return Ok(None);
        }
        let draft: EditorDraft = decode(&read_bytes(&path)?)?;
        if draft.schema_version != SCHEMA {
            return Err("Unsupported editor draft version.".into());
        }
        Ok(Some(draft))
    }

    pub fn save_draft(&self, draft: EditorDraft) -> Result<(), String> {
        if draft.schema_version != SCHEMA || draft.source_revision != self.view()?.revision {
            return Err("Draft source changed; existing draft has been kept.".into());
        }
        let bytes = encode(&draft)?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err("Editor draft exceeds its size limit.".into());
        }
        if let Some(previous) = self.load_draft()? {
            if previous.source_revision != draft.source_revision
                && (previous.world != draft.world || previous.title != draft.title)
            {
                atomic_write(
                    &self
                        .root
                        .join("backups")
                        .join(format!("draft-{}.json", Uuid::new_v4())),
                    &encode(&previous)?,
                )?;
            }
        }
        atomic_write(&self.root.join("editor-draft.json"), &bytes)
    }

    pub fn clear_matching_draft(&self, title: &str, world: &World) -> Result<(), String> {
        if self
            .load_draft()?
            .is_some_and(|draft| draft.title == title && draft.world == *world)
        {
            fs::remove_file(self.root.join("editor-draft.json")).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn save_workspace(
        &self,
        expected_version: u32,
        mut workspace: Workspace,
    ) -> Result<Workspace, String> {
        if self.load_workspace()?.version != expected_version {
            return Err("Workspace layout changed; reload it before saving.".into());
        }
        workspace.validate()?;
        workspace.version = expected_version
            .checked_add(1)
            .ok_or("Workspace version limit reached.")?;
        atomic_write(&self.root.join("workspace.json"), &encode(&workspace)?)?;
        Ok(workspace)
    }
}

#[cfg(test)]
mod tests;
