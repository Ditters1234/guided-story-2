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
    /// Call only while native project operations are idle. The held project lock
    /// excludes other app instances. Published records never refer to .tmp files.
    pub fn cleanup_temporary_writes(&self) -> Result<(), String> {
        self.cleanup_temporary_writes_with_hook(&|_| Ok(()))
    }

    fn cleanup_temporary_writes_with_hook(
        &self,
        hook: &impl Fn(&str) -> Result<(), String>,
    ) -> Result<(), String> {
        // Do not alter an invalid story before the author chooses recovery.
        self.view()?;
        let mut folders = vec![self.root.clone()];
        folders.extend(backup::FOLDERS.iter().map(|folder| self.root.join(folder)));
        let mut errors = vec![];
        for folder in folders {
            let result = (|| {
                if !folder.try_exists().map_err(|e| e.to_string())? {
                    return Ok(());
                }
                if !ordinary(&folder, true)? {
                    return Err(format!(
                        "Temporary cleanup skipped a linked or unsupported folder: {}",
                        folder.display()
                    ));
                }
                for entry in fs::read_dir(&folder).map_err(|e| e.to_string())? {
                    let path = entry.map_err(|e| e.to_string())?.path();
                    let owned_name = path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .and_then(|name| name.strip_prefix('.'))
                        .and_then(|name| name.strip_suffix(".tmp"))
                        .is_some_and(|id| validate_id(id).is_ok());
                    if owned_name && ordinary(&path, false)? {
                        fs::remove_file(&path).map_err(|e| {
                            format!("Could not remove temporary save {}: {e}", path.display())
                        })?;
                        hook("afterRemove")?;
                    }
                }
                Ok::<(), String>(())
            })();
            if let Err(error) = result {
                errors.push(error);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("\n"))
        }
    }
}

#[cfg(test)]
mod tests;
