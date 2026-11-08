use super::*;

fn id() -> String {
    Uuid::new_v4().to_string()
}
fn fixture() -> (tempfile::TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let p = Project::create(
        &dir.path().join("story"),
        "Fictional cleanup test".into(),
        World::default(),
        id(),
    )
    .unwrap();
    (dir, p)
}
fn temps(p: &Project) -> Vec<PathBuf> {
    let mut paths = vec![];
    let mut folders = vec![p.root.clone()];
    folders.extend(backup::FOLDERS.iter().map(|folder| p.root.join(folder)));
    for folder in folders {
        if folder.is_dir() {
            paths.extend(
                fs::read_dir(folder)
                    .unwrap()
                    .map(|entry| entry.unwrap().path())
                    .filter(|path| path.extension().is_some_and(|e| e == "tmp")),
            );
        }
    }
    paths
}

#[test]
fn cleanup_removes_only_owned_temporary_files_in_managed_project_folders() {
    let (dir, p) = fixture();
    let before = p.view().unwrap();
    let head = fs::read(p.root.join("head.json")).unwrap();
    let world = fs::read(p.record_path("worlds", &before.world_ref).unwrap()).unwrap();
    let mut disposable = vec![];
    for folder in std::iter::once("").chain(backup::FOLDERS.iter().copied()) {
        let folder = p.root.join(folder);
        fs::create_dir_all(&folder).unwrap();
        disposable
            .push(write_temp(&folder.join("pending.json"), b"partial temporary save").unwrap());
    }
    let mut retained = vec![];
    for name in [
        ".notes.tmp".to_string(),
        format!("{}.tmp", id()),
        format!(".{}.json", id()),
        "saved.json".into(),
    ] {
        let path = p.root.join(name);
        fs::write(&path, b"keep").unwrap();
        retained.push(path);
    }
    for folder in [
        p.root.join("unrelated"),
        dir.path().join(".backup.incomplete"),
    ] {
        fs::create_dir(&folder).unwrap();
        let path = folder.join(format!(".{}.tmp", id()));
        fs::write(&path, b"keep").unwrap();
        retained.push(path);
    }
    let directory = p.root.join(format!(".{}.tmp", id()));
    fs::create_dir(&directory).unwrap();
    let nested = directory.join("draft.json");
    fs::write(&nested, b"keep").unwrap();
    retained.push(nested);
    p.cleanup_temporary_writes().unwrap();
    p.cleanup_temporary_writes().unwrap();
    assert!(disposable.iter().all(|path| !path.exists()));
    assert!(retained
        .iter()
        .all(|path| fs::read(path).unwrap() == b"keep"));
    assert_eq!(fs::read(p.root.join("head.json")).unwrap(), head);
    assert_eq!(
        fs::read(p.record_path("worlds", &before.world_ref).unwrap()).unwrap(),
        world
    );
    assert_eq!(p.view().unwrap().revision, before.revision);
}

#[test]
fn interrupted_temporary_writes_and_cleanup_resume_without_changing_saved_story() {
    for phase in ["writeTemp", "cleanup"] {
        let (dir, p) = fixture();
        let before = p.view().unwrap();
        drop(p);
        let root = dir.path().join("story");
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "store::cleanup::tests::cleanup_crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("GS2_TEMP_ROOT", &root)
            .env("GS2_TEMP_PHASE", phase)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(75));
        let p = Project::open(&root).unwrap();
        assert_eq!(temps(&p).len(), if phase == "writeTemp" { 3 } else { 2 });
        p.cleanup_temporary_writes().unwrap();
        assert!(temps(&p).is_empty());
        assert_eq!(p.view().unwrap().world_ref, before.world_ref);
        assert_eq!(p.view().unwrap().revision, before.revision);
    }
}

#[test]
#[ignore = "isolated temporary-write/cleanup crash probe"]
fn cleanup_crash_child() {
    let Ok(root) = std::env::var("GS2_TEMP_ROOT") else {
        return;
    };
    let p = Project::open(Path::new(&root)).unwrap();
    for folder in ["", "worlds", "passage-drafts"] {
        let folder = p.root.join(folder);
        fs::create_dir_all(&folder).unwrap();
        write_temp(&folder.join("pending.json"), b"flushed but unpublished").unwrap();
    }
    if std::env::var("GS2_TEMP_PHASE").unwrap() == "writeTemp" {
        std::process::exit(75);
    }
    p.cleanup_temporary_writes_with_hook(&|_| std::process::exit(75))
        .unwrap();
    panic!("cleanup boundary not reached");
}

#[test]
#[cfg(windows)]
fn locked_temporary_file_is_reported_and_other_folders_still_clean_up() {
    use std::os::windows::fs::OpenOptionsExt;
    let (_dir, p) = fixture();
    let locked = write_temp(&p.root.join("head.json"), b"unsaved temporary head").unwrap();
    let other = write_temp(
        &p.root.join("worlds").join("pending.json"),
        b"unsaved world",
    )
    .unwrap();
    let reader = OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&locked)
        .unwrap();
    assert!(p
        .cleanup_temporary_writes()
        .unwrap_err()
        .contains("temporary save"));
    assert!(locked.exists());
    assert!(!other.exists());
    drop(reader);
    p.cleanup_temporary_writes().unwrap();
    assert!(!locked.exists());
    assert_eq!(p.view().unwrap().revision, 0);
}

#[test]
fn invalid_story_is_preserved_before_explicit_recovery() {
    let (_dir, p) = fixture();
    let temp = write_temp(&p.root.join("head.json"), b"unpublished head").unwrap();
    fs::write(p.root.join("head.json"), b"{broken").unwrap();
    assert!(p.cleanup_temporary_writes().is_err());
    assert!(temp.exists());
}

#[test]
fn unchanged_world_snapshots_are_shared_by_new_commits_without_losing_receipts() {
    let (dir, p) = fixture();
    let before = p.view().unwrap();
    let bytes = fs::read(p.record_path("worlds", &before.world_ref).unwrap()).unwrap();
    let first_id = id();
    p.commit(
        0,
        first_id.clone(),
        "Renamed story".into(),
        before.world.clone(),
    )
    .unwrap();
    p.commit_story(
        1,
        id(),
        "Renamed story".into(),
        before.world.clone(),
        crate::StoryIndex::default(),
    )
    .unwrap();
    assert_eq!(p.view().unwrap().world_ref, before.world_ref);
    assert_eq!(fs::read_dir(p.root.join("worlds")).unwrap().count(), 1);
    assert!(
        p.commit(0, first_id, "Renamed story".into(), before.world.clone())
            .unwrap()
            .replayed
    );
    let mut changed = before.world;
    changed.node_types[0].name = "Custom place name".into();
    p.commit(2, id(), "Renamed story".into(), changed.clone())
        .unwrap();
    let changed_ref = p.view().unwrap().world_ref;
    assert_ne!(changed_ref, before.world_ref);
    p.commit(3, id(), "Another title".into(), changed).unwrap();
    assert_eq!(p.view().unwrap().world_ref, changed_ref);
    assert_eq!(fs::read_dir(p.root.join("worlds")).unwrap().count(), 2);
    assert_eq!(
        fs::read(p.record_path("worlds", &before.world_ref).unwrap()).unwrap(),
        bytes
    );
    let report = p.export_backup(dir.path(), &id(), 4).unwrap();
    crate::inspect_backup(Path::new(&report.folder_path)).unwrap();
}
