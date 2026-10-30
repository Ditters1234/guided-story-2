use super::*;

#[test]
fn interrupted_draft_deletion_and_backup_restore_keep_durable_markers() {
    for phase in ["before", "after"] {
        let (dir, p) = create();
        let root = dir.path().join("story");
        let world_draft = id();
        let passage_draft = id();
        p.save_authoring_draft(&world_draft, 0, request()).unwrap();
        p.save_passage(
            &passage_draft,
            0,
            PassageEdits {
                plan: PassagePlan::default(),
                prose: "Fictional unsaved canon.".into(),
                effects_text: r#"{"changes":[],"notes":[],"suggestChapterEnd":false}"#.into(),
                close_chapter: false,
                approved_effects: vec![],
            },
        )
        .unwrap();
        let before = p.view().unwrap();
        drop(p);
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "authoring::tests::deletion::draft_delete_crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("GS2_DELETE_FIXTURE", &root)
            .env("GS2_DELETE_WORLD", &world_draft)
            .env("GS2_DELETE_PASSAGE", &passage_draft)
            .env("GS2_DELETE_PHASE", phase)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::inherit())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(73));
        let p = Project::open(&root).unwrap();
        assert_eq!(
            p.authoring_list().unwrap().len(),
            usize::from(phase == "before")
        );
        assert_eq!(
            p.list_passage_drafts().unwrap().len(),
            usize::from(phase == "before")
        );
        p.delete_authoring_draft(&world_draft, 1).unwrap();
        p.delete_passage_draft(&passage_draft, 1).unwrap();
        assert_eq!(p.view().unwrap().revision, before.revision);
        assert_eq!(p.view().unwrap().world_ref, before.world_ref);
        let backup = p.export_backup(dir.path(), &id(), before.revision).unwrap();
        let source = std::path::Path::new(&backup.folder_path);
        let reviewed = inspect_backup(source).unwrap();
        let restored = restore_backup(source, dir.path(), &id(), &reviewed.fingerprint).unwrap();
        let copy = Project::open(std::path::Path::new(&restored.folder_path)).unwrap();
        copy.interrupt_authoring().unwrap();
        copy.interrupt_passages().unwrap();
        assert!(
            copy.authoring_list().unwrap().is_empty()
                && copy.list_passage_drafts().unwrap().is_empty()
        );
        assert!(copy
            .save_authoring_draft(&world_draft, 0, request())
            .is_err());
        assert_eq!(copy.view().unwrap().world_ref, before.world_ref);
    }
}

#[test]
#[ignore = "only invoked as an isolated draft deletion crash probe"]
fn draft_delete_crash_child() {
    let Ok(root) = std::env::var("GS2_DELETE_FIXTURE") else {
        return;
    };
    let p = Project::open(std::path::Path::new(&root)).unwrap();
    if std::env::var("GS2_DELETE_PHASE").unwrap() == "after" {
        p.delete_authoring_draft(&std::env::var("GS2_DELETE_WORLD").unwrap(), 1)
            .unwrap();
        p.delete_passage_draft(&std::env::var("GS2_DELETE_PASSAGE").unwrap(), 1)
            .unwrap();
    }
    std::process::exit(73);
}

#[test]
fn deleting_world_drafts_is_versioned_durable_and_blocks_delayed_writes() {
    let (dir, p) = create();
    let before = p.view().unwrap();
    let key = id();
    let req = request();
    p.save_authoring_draft(&key, 0, req.clone()).unwrap();
    assert!(p.delete_authoring_draft(&key, 0).is_err());
    assert_eq!(p.authoring_list().unwrap().len(), 1);
    p.delete_authoring_draft(&key, 1).unwrap();
    p.delete_authoring_draft(&key, 1).unwrap(); // Lost acknowledgement retry.
    assert!(p.authoring_list().unwrap().is_empty());
    assert!(p
        .authoring_view(&key, &settings())
        .unwrap_err()
        .contains("deleted"));
    for expected in [0, 1, 2] {
        assert!(p.save_authoring_draft(&key, expected, req.clone()).is_err());
    }
    assert!(p
        .start_authoring(&key, 2, before.revision, &settings())
        .is_err());
    assert!(p
        .finish_authoring(&key, Ok("late result".into()), false, false)
        .is_err());
    assert!(p
        .save_authoring_candidate(&key, 2, "late edit".into())
        .is_err());
    assert!(p.apply_authoring(&key, 2, &settings()).is_err());
    let marker = p.read_authoring_record(&key).unwrap();
    assert!(marker.request.direction.is_empty() && marker.candidate_text.is_empty());
    let after = p.view().unwrap();
    assert_eq!(after.revision, before.revision);
    assert_eq!(after.world_ref, before.world_ref);
    drop(p);
    let reopened = Project::open(&dir.path().join("story")).unwrap();
    reopened.interrupt_authoring().unwrap();
    assert!(reopened.authoring_list().unwrap().is_empty());
    reopened.delete_authoring_draft(&key, 1).unwrap();
    reopened.save_authoring_draft(&id(), 0, req).unwrap();
}

#[test]
fn world_draft_deletion_allows_stopped_candidates_and_protects_applied_receipts() {
    let (_dir, p) = create();
    for apply in [false, true] {
        let key = id();
        p.save_authoring_draft(&key, 0, request()).unwrap();
        let before = p.view().unwrap();
        p.start_authoring(&key, 1, before.revision, &settings())
            .unwrap();
        assert!(p.delete_authoring_draft(&key, 2).is_err());
        p.finish_authoring(
            &key,
            Ok(demo_authoring(&before.world, &request()).unwrap()),
            false,
            false,
        )
        .unwrap();
        if apply {
            let accepted = p.apply_authoring(&key, 3, &settings()).unwrap();
            // Simulate publication succeeding before the final proposal status write.
            let mut record = p.read_authoring_record(&key).unwrap();
            record.status = AuthoringStatus::Review;
            p.write_authoring(&record).unwrap();
            assert!(p.delete_authoring_draft(&key, record.version).is_err());
            assert!(
                p.apply_authoring(&key, accepted.proposal.version, &settings())
                    .unwrap()
                    .commit
                    .replayed
            );
            assert_eq!(
                p.view().unwrap().world_ref,
                accepted.commit.project.world_ref
            );
        } else {
            p.delete_authoring_draft(&key, 3).unwrap();
            let marker = p.read_authoring_record(&key).unwrap();
            assert!(
                marker.raw_response.is_empty()
                    && marker.candidate_text.is_empty()
                    && marker.source_world_ref.is_none()
            );
        }
    }
    for cancelled in [false, true] {
        let key = id();
        p.save_authoring_draft(&key, 0, request()).unwrap();
        p.start_authoring(&key, 1, p.view().unwrap().revision, &settings())
            .unwrap();
        p.finish_authoring(&key, Err("Stopped fixture".into()), cancelled, false)
            .unwrap();
        p.delete_authoring_draft(&key, 3).unwrap();
    }
}
