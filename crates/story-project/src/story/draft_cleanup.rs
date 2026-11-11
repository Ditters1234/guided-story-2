use super::*;

fn ordinary(path: &Path, directory: bool) -> Result<bool, String> {
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    let linked = {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    };
    #[cfg(not(windows))]
    let linked = meta.file_type().is_symlink();
    Ok(!linked
        && if directory {
            meta.is_dir()
        } else {
            meta.is_file()
        })
}

impl Project {
    /// Only disposable draft archives are removed. No canonical record refers to
    /// this folder; clones contain their own full draft, not an archive reference.
    /// The durable deletion marker must be published before calling this method.
    pub(super) fn purge_deleted_passage_versions(
        &self,
        deleted: &BTreeSet<String>,
        hook: &impl Fn(&str) -> Result<(), String>,
    ) -> Result<(), String> {
        let folder = self.root_path().join("passage-draft-versions");
        if deleted.is_empty() || !folder.try_exists().map_err(|e| e.to_string())? {
            return Ok(());
        }
        let result = (|| {
            if !ordinary(&folder, true)? {
                return Err("Draft archive folder is not an ordinary directory.".into());
            }
            let story_id = self.view()?.story_id;
            for entry in fs::read_dir(&folder).map_err(|e| e.to_string())? {
                let path = entry.map_err(|e| e.to_string())?.path();
                if path.extension().is_none_or(|ext| ext != "json") {
                    continue;
                }
                let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                    continue;
                };
                if validate_id(stem).is_err() || !ordinary(&path, false)? {
                    continue;
                }
                // Unrecognized/damaged files cannot prove ownership. Preserve them.
                let bytes = crate::store::read_bytes(&path)?;
                let Ok(draft) = serde_json::from_slice::<PassageDraft>(&bytes) else {
                    continue;
                };
                if draft.schema_version == 1
                    && draft.story_id == story_id
                    && draft.version > 0
                    && draft.stage != PassageStage::Accepted
                    && deleted.contains(&draft.id)
                {
                    fs::remove_file(&path).map_err(|e| e.to_string())?;
                    hook("afterRemove")?;
                }
            }
            Ok::<(), String>(())
        })();
        result.map_err(|error| format!(
            "Draft deletion is saved, but archived draft cleanup is incomplete. Reopen the story or retry deletion: {error}"
        ))
    }
}
