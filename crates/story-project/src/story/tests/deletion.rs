use super::*;

fn archived_files(p: &Project) -> BTreeMap<std::path::PathBuf, Vec<u8>> {
    let folder = p.root_path().join("passage-draft-versions");
    if !folder.exists() {
        return BTreeMap::new();
    }
    fs::read_dir(folder)
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let bytes = fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect()
}

#[test]
fn unchanged_passage_autosaves_keep_review_and_do_not_archive_duplicate_contents() {
    let (_dir, p) = fixture();
    let key = id();
    p.save_passage(&key, 0, edits(&p, "Mara waited beside the window."))
        .unwrap();
    p.validate_passage(&key, 1, &settings()).unwrap();
    let view = p.passage_view(&key, &settings()).unwrap();
    assert!(view.can_accept);
    let bytes = fs::read(p.passage_path(&key).unwrap()).unwrap();
    for _ in 0..3 {
        p.save_passage(&key, view.version, view.edits.clone())
            .unwrap();
    }
    assert_eq!(fs::read(p.passage_path(&key).unwrap()).unwrap(), bytes);
    assert!(archived_files(&p).is_empty());
    assert!(p.passage_view(&key, &settings()).unwrap().can_accept);
    let mut changed = view.edits;
    changed.prose = "Jon opened the window.".into();
    p.save_passage(&key, view.version, changed).unwrap();
    assert_eq!(archived_files(&p).len(), 1);
    assert!(!p.passage_view(&key, &settings()).unwrap().can_accept);
}

#[test]
fn deleting_passage_removes_its_autosaves_and_preserves_history_and_clones() {
    let (dir, p) = fixture();
    let accepted = id();
    p.save_passage(&accepted, 0, edits(&p, "Mara waited."))
        .unwrap();
    p.save_passage(&accepted, 1, edits(&p, "Mara waited beside the window."))
        .unwrap();
    p.validate_passage(&accepted, 2, &settings()).unwrap();
    let ready = p.passage_view(&accepted, &settings()).unwrap();
    p.accept_passage(&accepted, ready.version, &settings())
        .unwrap();
    let reading = p.read_passage(&accepted).unwrap();
    let before = p.view().unwrap();
    let mut canonical = BTreeMap::new();
    for folder in [
        "worlds",
        "revisions",
        "stories",
        "passages",
        "source-anchors",
        "backups",
    ] {
        for entry in fs::read_dir(p.root_path().join(folder)).unwrap() {
            let path = entry.unwrap().path();
            canonical.insert(path.clone(), fs::read(path).unwrap());
        }
    }
    let key = id();
    for version in 0..4 {
        p.save_passage(
            &key,
            version,
            edits(&p, &format!("Discarded draft wording {version}.")),
        )
        .unwrap();
    }
    let clone = id();
    p.clone_passage_draft(&key, 4, &clone).unwrap();
    p.save_passage(&clone, 1, edits(&p, "Keep this independent candidate."))
        .unwrap();
    let archives = archived_files(&p);
    let retained: BTreeMap<_, _> = archives
        .iter()
        .filter(|(_, bytes)| serde_json::from_slice::<PassageDraft>(bytes).unwrap().id != key)
        .map(|(path, bytes)| (path.clone(), bytes.clone()))
        .collect();
    assert_eq!(archives.len() - retained.len(), 3);
    assert!(!retained.is_empty());
    p.delete_passage_draft(&key, 4).unwrap();
    assert_eq!(archived_files(&p), retained);
    assert_eq!(p.read_passage(&accepted).unwrap().record, reading.record);
    assert_eq!(p.view().unwrap().revision, before.revision);
    assert_eq!(p.view().unwrap().world_ref, before.world_ref);
    for (path, bytes) in canonical {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
    assert_eq!(
        p.passage_view(&clone, &settings()).unwrap().edits.prose,
        "Keep this independent candidate."
    );
    let backup = p.export_backup(dir.path(), &id(), before.revision).unwrap();
    let reviewed = inspect_backup(Path::new(&backup.folder_path)).unwrap();
    let restored = restore_backup(
        Path::new(&backup.folder_path),
        dir.path(),
        &id(),
        &reviewed.fingerprint,
    )
    .unwrap();
    let copy = Project::open(Path::new(&restored.folder_path)).unwrap();
    assert_eq!(archived_files(&copy).len(), retained.len());
    assert!(copy
        .save_passage(&key, 0, edits(&copy, "Delayed save"))
        .is_err());
}

#[test]
fn deletion_cleanup_resumes_after_process_exit_at_each_boundary() {
    for phase in ["beforeMarker", "afterMarker", "afterRemove"] {
        let (dir, p) = fixture();
        let key = id();
        for version in 0..4 {
            p.save_passage(
                &key,
                version,
                edits(&p, &format!("Draft version {version}.")),
            )
            .unwrap();
        }
        let before = p.view().unwrap();
        drop(p);
        let root = dir.path().join("story");
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "story::tests::deletion::cleanup_crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("GS2_CLEANUP_ROOT", &root)
            .env("GS2_CLEANUP_ID", &key)
            .env("GS2_CLEANUP_PHASE", phase)
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(74));
        let p = Project::open(&root).unwrap();
        assert_eq!(
            archived_files(&p).len(),
            if phase == "afterRemove" { 2 } else { 3 }
        );
        assert_eq!(
            p.list_passage_drafts().unwrap().len(),
            usize::from(phase == "beforeMarker")
        );
        if phase != "beforeMarker" {
            assert!(p.save_passage(&key, 4, edits(&p, "Late write")).is_err());
        }
        // Opening in the app runs this recovery pass; it also cleans old markers
        // written by versions that retained all abandoned autosaves.
        p.interrupt_passages().unwrap();
        if phase == "beforeMarker" {
            assert_eq!(archived_files(&p).len(), 3);
        } else {
            assert!(archived_files(&p).is_empty());
        }
        p.delete_passage_draft(&key, 4).unwrap();
        assert!(archived_files(&p).is_empty());
        assert_eq!(p.view().unwrap().revision, before.revision);
        assert_eq!(p.view().unwrap().world_ref, before.world_ref);
    }
}

#[test]
#[ignore = "isolated draft archive cleanup process-exit probe"]
fn cleanup_crash_child() {
    let Ok(root) = std::env::var("GS2_CLEANUP_ROOT") else {
        return;
    };
    let p = Project::open(Path::new(&root)).unwrap();
    let key = std::env::var("GS2_CLEANUP_ID").unwrap();
    let phase = std::env::var("GS2_CLEANUP_PHASE").unwrap();
    p.delete_passage_draft_with_hook(&key, 4, &|at| {
        if at == phase {
            std::process::exit(74);
        }
        Ok(())
    })
    .unwrap();
    panic!("The cleanup boundary was not reached.");
}

#[test]
#[cfg(windows)]
fn failed_cleanup_preserves_marker_and_retry_finishes_without_recreating_draft() {
    use std::os::windows::fs::OpenOptionsExt;
    let (_dir, p) = fixture();
    let key = id();
    p.save_passage(&key, 0, edits(&p, "Original draft."))
        .unwrap();
    p.save_passage(&key, 1, edits(&p, "Edited draft.")).unwrap();
    let path = archived_files(&p).into_keys().next().unwrap();
    // A reader without delete sharing reproduces a real Windows cleanup failure.
    let reader = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(path)
        .unwrap();
    let error = p.delete_passage_draft(&key, 2).unwrap_err();
    assert!(error.contains("deletion is saved") && error.contains("cleanup is incomplete"));
    assert_eq!(archived_files(&p).len(), 1);
    assert!(p.list_passage_drafts().unwrap().is_empty());
    assert!(p.save_passage(&key, 0, edits(&p, "Delayed draft")).is_err());
    drop(reader);
    p.delete_passage_draft(&key, 2).unwrap();
    assert!(archived_files(&p).is_empty());
}

#[test]
fn cleanup_preserves_archives_without_proven_disposable_ownership() {
    let (_dir, p) = fixture();
    let key = id();
    p.save_passage(&key, 0, edits(&p, "Original draft."))
        .unwrap();
    let draft = p.read_passage_draft(&key).unwrap();
    let original = serde_json::to_value(&draft).unwrap();
    for (field, value) in [
        ("storyId", json!(id())),
        ("schemaVersion", json!(99)),
        ("stage", json!("accepted")),
        ("unknownField", json!("preserve")),
    ] {
        let mut value_draft = original.clone();
        value_draft[field] = value;
        p.write_record("passage-draft-versions", &value_draft)
            .unwrap();
    }
    let damaged = p
        .root_path()
        .join("passage-draft-versions")
        .join(format!("{}.json", id()));
    fs::write(damaged, "{broken").unwrap();
    let retained = archived_files(&p);
    p.save_passage(&key, 1, edits(&p, "Edited draft.")).unwrap();
    assert_eq!(archived_files(&p).len(), retained.len() + 1);
    p.delete_passage_draft(&key, 2).unwrap();
    assert_eq!(archived_files(&p), retained);
}

#[test]
fn passage_draft_deletion_survives_reopen_and_does_not_resurrect_or_change_history() {
    let (dir, p) = fixture();
    let before = p.view().unwrap();
    let key = id();
    let edit = edits(&p, "An unaccepted passage to discard.");
    p.save_passage(&key, 0, edit.clone()).unwrap();
    assert!(p.delete_passage_draft(&key, 0).is_err());
    p.delete_passage_draft(&key, 1).unwrap();
    p.delete_passage_draft(&key, 1).unwrap();
    assert!(p.list_passage_drafts().unwrap().is_empty());
    assert!(p
        .passage_view(&key, &settings())
        .unwrap_err()
        .contains("deleted"));
    for version in [0, 1, 2] {
        assert!(p.save_passage(&key, version, edit.clone()).is_err());
    }
    assert!(p.clone_passage_draft(&key, 2, &id()).is_err());
    assert!(p
        .start_passage(&key, 2, before.revision, false, &settings())
        .is_err());
    assert!(p.validate_passage(&key, 2, &settings()).is_err());
    assert!(p.accept_passage(&key, 2, &settings()).is_err());
    let marker = p.read_passage_draft_record(&key).unwrap();
    assert!(
        marker.edits.prose.is_empty()
            && marker.original_prose.is_empty()
            && marker.source.is_none()
    );
    assert_eq!(p.view().unwrap().world_ref, before.world_ref);
    assert_eq!(p.view().unwrap().revision, before.revision);
    assert!(p.story_index().unwrap().passages.is_empty());
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    p.interrupt_passages().unwrap();
    assert!(p.list_passage_drafts().unwrap().is_empty());
    p.delete_passage_draft(&key, 1).unwrap();
    p.save_passage(&id(), 0, edit).unwrap();
}

#[test]
fn deleting_revisions_and_rerolls_preserves_accepted_sources_and_other_candidates() {
    let (_dir, p) = fixture();
    let key = id();
    p.save_passage(&key, 0, edits(&p, "The room fell quiet."))
        .unwrap();
    p.validate_passage(&key, 1, &settings()).unwrap();
    let ready = p.passage_view(&key, &settings()).unwrap();
    p.accept_passage(&key, ready.version, &settings()).unwrap();
    let reading = p.read_passage(&key).unwrap();
    let before = p.view().unwrap();
    let accepted = p.read_passage_draft(&key).unwrap();
    assert!(p.delete_passage_draft(&key, accepted.version).is_err());
    // Receipt reconciliation must also protect a committed result with stale status.
    let mut uncertain = accepted.clone();
    uncertain.stage = PassageStage::Ready;
    p.write_passage_draft(&uncertain).unwrap();
    assert!(p.delete_passage_draft(&key, uncertain.version).is_err());
    for kind in [PassageRevisionKind::Wording, PassageRevisionKind::Events] {
        let revision = id();
        p.begin_passage_revision(&revision, &key, reading.record.clone(), kind)
            .unwrap();
        let candidate = id();
        p.clone_passage_draft(&revision, 1, &candidate).unwrap();
        p.delete_passage_draft(&revision, 1).unwrap();
        assert!(p
            .begin_passage_revision(&revision, &key, reading.record.clone(), kind)
            .is_err());
        assert!(p
            .clone_passage_draft(&key, accepted.version, &revision)
            .is_err());
        assert_eq!(
            p.passage_view(&candidate, &settings())
                .unwrap()
                .revision
                .unwrap()
                .kind,
            kind
        );
        p.delete_passage_draft(&candidate, 1).unwrap();
    }
    assert_eq!(p.read_passage(&key).unwrap().record, reading.record);
    assert_eq!(p.view().unwrap().world_ref, before.world_ref);
    assert_eq!(p.view().unwrap().revision, before.revision);
}

#[test]
fn running_passage_drafts_are_protected_until_stopped() {
    let (_dir, p) = fixture();
    for checking in [false, true] {
        let key = id();
        p.save_passage(
            &key,
            0,
            edits(&p, if checking { "A manual passage." } else { "" }),
        )
        .unwrap();
        p.start_passage(&key, 1, p.view().unwrap().revision, checking, &settings())
            .unwrap();
        assert!(p.delete_passage_draft(&key, 2).is_err());
        p.finish_passage(
            &key,
            checking,
            Err("Stopped fixture".into()),
            true,
            false,
            &settings(),
        )
        .unwrap();
        let stopped = p.passage_view(&key, &settings()).unwrap();
        p.delete_passage_draft(&key, stopped.version).unwrap();
    }
}
