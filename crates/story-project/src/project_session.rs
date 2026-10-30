//! Device-local startup choice, separate from portable stories and API settings.
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::Read,
    path::Path,
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectSession {
    schema_version: u32,
    last_project_path: String,
}

fn validate_path(path: &str) -> Result<(), String> {
    if path.chars().count() > 32767 || path.contains('\0') || !Path::new(path).is_absolute() {
        return Err("The last story needs a valid absolute folder path.".into());
    }
    Ok(())
}
fn lock(root: &Path) -> Result<File, String> {
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("project-session.lock"))
        .map_err(|e| e.to_string())?;
    lock.try_lock_exclusive()
        .map_err(|_| "The last-story setting is in use by another app instance. Retry shortly.")?;
    Ok(lock)
}

pub fn last_project_path(root: &Path) -> Result<Option<String>, String> {
    let _lock = lock(root)?;
    let file = match File::open(root.join("project-session.json")) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.to_string()),
    };
    let mut bytes = vec![];
    file.take(256 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 256 * 1024 {
        return Err("The last-story setting is too large. The file has been preserved.".into());
    }
    let session: ProjectSession = serde_json::from_slice(&bytes).map_err(|e| {
        format!("Cannot read project-session.json: {e}. The file has been preserved.")
    })?;
    if session.schema_version != 1 {
        return Err("Unsupported last-story setting version. The file has been preserved.".into());
    }
    validate_path(&session.last_project_path)?;
    Ok(Some(session.last_project_path))
}

/// Called only after a project has successfully opened/been created. Failure is
/// reported as a warning on that open project, never as a failed story operation.
pub fn remember_project(root: &Path, folder_path: &str) -> Result<(), String> {
    validate_path(folder_path)?;
    let _lock = lock(root)?;
    let session = ProjectSession {
        schema_version: 1,
        last_project_path: folder_path.into(),
    };
    let bytes = serde_json::to_vec_pretty(&session).map_err(|e| e.to_string())?;
    crate::store::atomic_write(&root.join("project-session.json"), &bytes)
}

pub fn startup_project_path(root: &Path, explicit: Option<&str>) -> Result<Option<String>, String> {
    match explicit {
        Some(path) if path.trim().is_empty() => {
            Err("Choose a project folder after --project.".into())
        }
        Some(path) => Ok(Some(path.into())),
        None => last_project_path(root),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn last_successful_folder_roundtrips_without_changing_story_or_other_settings() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("device");
        let story = dir.path().join("story");
        let project = crate::Project::create(
            &story,
            "Fictional story".into(),
            crate::World::default(),
            uuid::Uuid::new_v4().to_string(),
        )
        .unwrap();
        let before = project.view().unwrap();
        assert!(last_project_path(&config).unwrap().is_none());
        fs::write(config.join("preferences.json"), b"unchanged appearance").unwrap();
        remember_project(&config, &project.folder_path()).unwrap();
        assert_eq!(
            last_project_path(&config).unwrap(),
            Some(project.folder_path())
        );
        assert!(!story.join("project-session.json").exists());
        assert_eq!(
            fs::read(config.join("preferences.json")).unwrap(),
            b"unchanged appearance"
        );
        assert_eq!(project.view().unwrap().world_ref, before.world_ref);
        let another = dir.path().join("another-story");
        remember_project(&config, another.to_str().unwrap()).unwrap();
        assert_eq!(
            last_project_path(&config).unwrap().unwrap(),
            another.to_str().unwrap()
        );
    }
    #[test]
    fn explicit_launch_path_overrides_remembered_or_unreadable_state() {
        let dir = tempfile::tempdir().unwrap();
        let remembered = dir.path().join("last-story");
        let explicit = dir.path().join("requested-story");
        remember_project(dir.path(), remembered.to_str().unwrap()).unwrap();
        assert_eq!(
            startup_project_path(dir.path(), None).unwrap().unwrap(),
            remembered.to_str().unwrap()
        );
        fs::write(dir.path().join("project-session.json"), b"{broken").unwrap();
        assert_eq!(
            startup_project_path(dir.path(), Some(explicit.to_str().unwrap()))
                .unwrap()
                .unwrap(),
            explicit.to_str().unwrap()
        );
        assert!(startup_project_path(dir.path(), Some("")).is_err());
        assert!(startup_project_path(dir.path(), None).is_err());
        assert_eq!(
            fs::read(dir.path().join("project-session.json")).unwrap(),
            b"{broken"
        );
    }
    #[test]
    fn invalid_metadata_and_concurrent_writes_are_bounded_and_preserved() {
        let dir = tempfile::tempdir().unwrap();
        assert!(remember_project(dir.path(), "relative/story").is_err());
        for bytes in [
            br#"{"schemaVersion":1,"lastProjectPath":"relative/story"}"#.to_vec(),
            br#"{"schemaVersion":9,"lastProjectPath":"unused"}"#.to_vec(),
            vec![b'x'; 256 * 1024 + 1],
        ] {
            fs::write(dir.path().join("project-session.json"), &bytes).unwrap();
            assert!(last_project_path(dir.path()).is_err());
            assert_eq!(
                fs::read(dir.path().join("project-session.json")).unwrap(),
                bytes
            );
        }
        let _held = lock(dir.path()).unwrap();
        assert!(last_project_path(dir.path()).is_err());
        assert!(remember_project(dir.path(), dir.path().to_str().unwrap()).is_err());
    }
}
