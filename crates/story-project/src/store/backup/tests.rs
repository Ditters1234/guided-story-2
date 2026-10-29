use super::*;
fn fingerprint(source: &Path) -> String {
    digest(&encode(&load_backup(source).unwrap()).unwrap())
}
fn restore_backup(source: &Path, parent: &Path, id: &str) -> Result<BackupReport, String> {
    super::restore_backup(source, parent, id, &fingerprint(source))
}
use std::{
    io::Cursor,
    process::{Command, Stdio},
};
fn id() -> String {
    Uuid::new_v4().to_string()
}
fn rename_fixture(source: &Path, destination: &Path) {
    for attempt in 0..40 {
        match fs::rename(source, destination) {
            Ok(()) => return,
            Err(e) if attempt < 39 && matches!(e.raw_os_error(), Some(5 | 32 | 33)) => {
                std::thread::sleep(std::time::Duration::from_millis(50))
            }
            Err(e) => panic!("Fixture could not be moved: {e}"),
        }
    }
}
fn text() -> LlmConnection {
    LlmConnection {
        provider: TextProvider::Demo,
        ..Default::default()
    }
}
fn fixture() -> (tempfile::TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let mut world = World::default();
    let mut preset = world
        .node_types
        .iter()
        .find(|t| t.id == "core.place")
        .unwrap()
        .clone();
    preset.id = "custom.greenhouse".into();
    preset.name = "Greenhouse".into();
    world.node_types.push(preset.clone());
    let components = preset
        .components
        .iter()
        .map(|id| {
            let d = world.component_types.iter().find(|c| &c.id == id).unwrap();
            (
                id.clone(),
                d.fields
                    .iter()
                    .map(|f| (f.key.clone(), f.default_value.clone()))
                    .collect(),
            )
        })
        .collect();
    world.nodes.push(WorldNode {
        id: id(),
        type_id: preset.id,
        name: "Glasshouse".into(),
        description: "Fictional planning note".into(),
        visual_description: "A sunlit glasshouse".into(),
        components,
    });
    let p = Project::create(
        &dir.path().join("source"),
        "Fictional backup test".into(),
        world,
        id(),
    )
    .unwrap();
    (dir, p)
}
fn passage(p: &Project) -> String {
    let key = id();
    let mut plan = p.next_passage_plan().unwrap();
    plan.direction = "A quiet morning.".into();
    plan.location_id = Some(p.view().unwrap().world.nodes[0].id.clone());
    p.save_passage(
        &key,
        0,
        PassageEdits {
            plan,
            prose: "Sunlight filled the Glasshouse.".into(),
            effects_text: empty_effects(),
            close_chapter: true,
            approved_effects: vec![],
        },
    )
    .unwrap();
    p.validate_passage(&key, 1, &text()).unwrap();
    p.accept_passage(
        &key,
        p.passage_view(&key, &text()).unwrap().version,
        &text(),
    )
    .unwrap();
    key
}
fn target(p: &Project) -> MediaTarget {
    MediaTarget {
        node_id: Some(p.view().unwrap().world.nodes[0].id.clone()),
        passage_id: None,
        moment: ImageMoment::After,
        subject_ids: vec![],
    }
}
fn png() -> Vec<u8> {
    let mut bytes = Cursor::new(vec![]);
    image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
        12,
        16,
        image::Rgb([60, 120, 180]),
    ))
    .write_to(&mut bytes, image::ImageFormat::Png)
    .unwrap();
    bytes.into_inner()
}
fn enriched(p: &Project, dir: &Path) -> (String, String, String) {
    let passage = passage(p);
    let revision = id();
    p.begin_passage_revision(
        &revision,
        &passage,
        p.read_passage(&passage).unwrap().record,
        PassageRevisionKind::Wording,
    )
    .unwrap();
    let mut edits = p.passage_view(&revision, &text()).unwrap().edits;
    edits.prose = "Warm sunlight filled the Glasshouse.".into();
    p.save_passage(&revision, 1, edits).unwrap();
    p.validate_passage(&revision, 2, &text()).unwrap();
    p.accept_passage(
        &revision,
        p.passage_view(&revision, &text()).unwrap().version,
        &text(),
    )
    .unwrap();
    let chapter = p.story_index().unwrap().chapters[0].id.clone();
    let memory = id();
    p.create_memory_draft(&memory, &chapter).unwrap();
    let mut edits = p.memory_view(&memory).unwrap().edits;
    edits.output.no_lasting_facts = true;
    p.save_memory_draft(&memory, 1, edits).unwrap();
    p.apply_memory(&memory, 2).unwrap();
    let asset = id();
    p.create_media(&asset, target(p)).unwrap();
    let file = dir.join("synthetic.png");
    fs::write(&file, png()).unwrap();
    let ready = p.import_media_image(&asset, 1, &file).unwrap();
    p.accept_media(&asset, ready.draft.version).unwrap();
    let pending = id();
    p.create_media(&pending, target(p)).unwrap();
    let suggestion = id();
    p.create_image_prompt(
        &suggestion,
        &pending,
        1,
        &text(),
        &ImageConnection::default(),
    )
    .unwrap();
    let mut edits = p
        .image_prompt_view(&suggestion, &text(), &ImageConnection::default())
        .unwrap()
        .edits;
    edits.direction = "Keep the window visible.".into();
    edits.output.prompt = "A sunlit glasshouse, the window in frame.".into();
    p.save_image_prompt(&suggestion, 1, edits).unwrap();
    p.apply_image_prompt(&suggestion, 2, &text(), &ImageConnection::default())
        .unwrap();
    let queued = id();
    p.create_media(&queued, target(p)).unwrap();
    let submission = p
        .prepare_media(&queued, 1, &ImageConnection::default())
        .unwrap();
    p.begin_media_submission(&queued, 1, submission).unwrap();
    p.finish_media_submission(
        &queued,
        &ComfySubmission::Queued("fictional-prompt-id".into()),
    )
    .unwrap();
    let world_proposal = id();
    p.save_authoring_draft(
        &world_proposal,
        0,
        AuthoringRequest {
            mode: AuthoringMode::Create,
            direction: "A fictional courtyard".into(),
            type_id: Some("core.place".into()),
            bundle: false,
            creation_scope: None,
            group_type_id: None,
            target_ids: vec![],
            context_ids: vec![],
            fields: vec![],
        },
    )
    .unwrap();
    p.save_workspace(0, Workspace::default()).unwrap();
    let view = p.view().unwrap();
    let mut invalid = view.world;
    invalid.nodes[0].name = "".into();
    p.save_draft(EditorDraft {
        schema_version: super::super::SCHEMA,
        source_revision: view.revision,
        title: view.title,
        world: invalid,
    })
    .unwrap();
    (passage, asset, queued)
}
fn reseal(root: &Path) {
    let mut m = load_backup(root).unwrap();
    m.entries = inventory(&root.join("project"), true).unwrap().0;
    atomic_write(&root.join("backup.json"), &encode(&m).unwrap()).unwrap();
}

#[test]
fn roundtrip_preserves_history_custom_nodes_memory_images_and_drafts_without_source() {
    let (dir, p) = fixture();
    let (passage, asset, queued) = enriched(&p, dir.path());
    let head = fs::read(p.root.join("head.json")).unwrap();
    fs::write(
        p.root.join("connection.json"),
        "fictional-secret-exclude-me",
    )
    .unwrap();
    fs::write(p.root.join(".leftover.tmp"), "partial").unwrap();
    fs::write(
        p.root
            .join("backups")
            .join(format!("damaged-{}.json", id())),
        "not JSON: retained recovery bytes",
    )
    .unwrap();
    let backup = p
        .export_backup(dir.path(), &id(), p.view().unwrap().revision)
        .unwrap();
    assert_eq!(backup.omitted_files, 2);
    let backup = PathBuf::from(backup.folder_path);
    assert!(!backup.join("project/connection.json").exists());
    assert!(!backup.join("project/.project.lock").exists());
    assert_eq!(head, fs::read(p.root.join("head.json")).unwrap());
    let source = p.root.clone();
    drop(p);
    rename_fixture(&source, &dir.path().join("source-unavailable"));
    let restored = restore_backup(&backup, dir.path(), &id()).unwrap();
    let restored = Project::open(Path::new(&restored.folder_path)).unwrap();
    assert_eq!(head, fs::read(restored.root.join("head.json")).unwrap());
    assert_eq!(
        restored.read_passage(&passage).unwrap().passage.prose,
        "Warm sunlight filled the Glasshouse."
    );
    assert_eq!(
        restored.passage_versions(&passage).unwrap().versions.len(),
        1
    );
    assert!(!restored.media_preview(&asset, true).unwrap().is_empty());
    assert_eq!(
        restored.media_draft(&queued).unwrap().stage,
        MediaStage::Queued
    );
    assert_eq!(
        restored
            .media_draft(&queued)
            .unwrap()
            .submission
            .unwrap()
            .prompt_id
            .as_deref(),
        Some("fictional-prompt-id")
    );
    assert!(restored
        .load_draft()
        .unwrap()
        .unwrap()
        .world
        .validate()
        .is_err());
    assert_eq!(restored.load_workspace().unwrap().version, 1);
    assert_eq!(restored.story_index().unwrap().memories.len(), 1);
    assert_eq!(restored.authoring_list().unwrap().len(), 1);
    let suggestions = fs::read_dir(restored.root.join("image-prompt-drafts"))
        .unwrap()
        .count();
    assert_eq!(suggestions, 1);
}
#[test]
fn export_and_restore_replays_preserve_newer_work() {
    let (dir, p) = fixture();
    let export = id();
    let backup = p.export_backup(dir.path(), &export, 0).unwrap();
    p.commit(0, id(), "Later source work".into(), p.view().unwrap().world)
        .unwrap();
    assert_eq!(p.export_backup(dir.path(), &export, 0).unwrap().revision, 0);
    assert!(p.export_backup(dir.path(), &export, 1).is_err());
    let restore = id();
    let restored = restore_backup(Path::new(&backup.folder_path), dir.path(), &restore).unwrap();
    let copy = Project::open(Path::new(&restored.folder_path)).unwrap();
    copy.commit(
        0,
        id(),
        "Later restored work".into(),
        copy.view().unwrap().world,
    )
    .unwrap();
    let replay = restore_backup(Path::new(&backup.folder_path), dir.path(), &restore).unwrap();
    assert_eq!(replay.folder_path, restored.folder_path);
    assert_eq!(copy.view().unwrap().title, "Later restored work");
}
#[test]
fn restore_requires_the_exact_reviewed_backup_and_indexes_match_record_identities() {
    let (dir, p) = fixture();
    let backup = p.export_backup(dir.path(), &id(), 0).unwrap();
    let path = Path::new(&backup.folder_path);
    let mut m = load_backup(path).unwrap();
    m.omitted_files += 1;
    fs::write(path.join("backup.json"), encode(&m).unwrap()).unwrap();
    let op = id();
    assert!(
        super::restore_backup(path, dir.path(), &op, &backup.fingerprint)
            .unwrap_err()
            .contains("changed")
    );
    assert!(!dir
        .path()
        .join(format!("guided-story-restored-{op}"))
        .exists());
    let reviewed = inspect_backup(path).unwrap();
    super::restore_backup(path, dir.path(), &op, &reviewed.fingerprint).unwrap();
    passage(&p);
    let mut index = p.story_index().unwrap();
    let wrong = id();
    index.passages[0].id = wrong.clone();
    index.chapters[0].passage_ids[0] = wrong;
    p.write_record("stories", &index).unwrap();
    assert!(p
        .export_backup(dir.path(), &id(), 1)
        .unwrap_err()
        .contains("identity"));
}
#[test]
fn corrupted_missing_or_extra_backup_files_never_publish_a_restore() {
    for change in ["changed", "missing", "extra"] {
        let (dir, p) = fixture();
        let backup = p.export_backup(dir.path(), &id(), 0).unwrap();
        let backup = PathBuf::from(backup.folder_path);
        let world = backup
            .join("project/worlds")
            .join(format!("{}.json", p.view().unwrap().world_ref.id));
        match change {
            "changed" => fs::write(world, "{}").unwrap(),
            "missing" => fs::remove_file(world).unwrap(),
            _ => fs::write(backup.join("project/unexpected.json"), "{}").unwrap(),
        };
        let op = id();
        assert!(restore_backup(&backup, dir.path(), &op).is_err());
        assert!(!dir
            .path()
            .join(format!("guided-story-restored-{op}"))
            .exists());
        assert_eq!(p.view().unwrap().revision, 0);
    }
}
#[test]
fn resealed_invalid_world_and_foreign_draft_still_fail_domain_validation() {
    for change in ["world", "foreign"] {
        let (dir, p) = fixture();
        passage(&p);
        let backup = PathBuf::from(p.export_backup(dir.path(), &id(), 1).unwrap().folder_path);
        if change == "world" {
            let path = backup
                .join("project/worlds")
                .join(format!("{}.json", p.view().unwrap().world_ref.id));
            let mut w: World = decode(&fs::read(&path).unwrap()).unwrap();
            w.nodes[0].name.clear();
            fs::write(path, encode(&w).unwrap()).unwrap();
        } else {
            let path = fs::read_dir(backup.join("project/passage-drafts"))
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            let mut v: Value = decode(&fs::read(&path).unwrap()).unwrap();
            v["storyId"] = Value::String(id());
            fs::write(path, encode(&v).unwrap()).unwrap();
        }
        reseal(&backup);
        assert!(inspect_backup(&backup).is_err());
    }
}
#[test]
fn missing_earlier_versions_or_images_block_export() {
    for remove in ["old-version", "image"] {
        let (dir, p) = fixture();
        let (passage, asset, _) = enriched(&p, dir.path());
        if remove == "old-version" {
            let previous = p
                .read_passage(&passage)
                .unwrap()
                .passage
                .previous_version
                .unwrap();
            fs::remove_file(
                p.root
                    .join("passages")
                    .join(format!("{}.json", previous.id)),
            )
            .unwrap();
        } else {
            let f = p.media_asset(&asset).unwrap().image;
            fs::remove_file(
                p.root
                    .join("images")
                    .join(format!("{}.{}", f.id, f.extension)),
            )
            .unwrap();
        }
        let op = id();
        assert!(p
            .export_backup(dir.path(), &op, p.view().unwrap().revision)
            .is_err());
        assert!(!dir
            .path()
            .join(format!("guided-story-backup-{op}"))
            .exists());
    }
}
#[test]
fn unsafe_duplicate_unsupported_and_oversized_manifest_entries_are_rejected() {
    for bad in [
        "../head.json",
        "C:/outside.json",
        "worlds/../head.json",
        "worlds\\outside.json",
        "worlds/CON.json",
    ] {
        assert!(!allowed(bad));
    }
    for change in ["path", "duplicate", "size", "schema"] {
        let (dir, p) = fixture();
        let backup = PathBuf::from(p.export_backup(dir.path(), &id(), 0).unwrap().folder_path);
        let mut m = load_backup(&backup).unwrap();
        match change {
            "path" => m.entries[0].path = "../outside.json".into(),
            "duplicate" => m.entries.push(m.entries[0].clone()),
            "size" => m.entries[0].bytes = MAX_FILE_BYTES + 1,
            _ => m.schema_version = 2,
        };
        fs::write(backup.join("backup.json"), encode(&m).unwrap()).unwrap();
        assert!(inspect_backup(&backup).is_err());
        assert!(!dir.path().join("outside.json").exists());
    }
}
#[test]
fn nested_destinations_existing_folders_and_busy_transfer_are_preserved() {
    let (dir, p) = fixture();
    assert!(p.export_backup(p.root_path(), &id(), 0).is_err());
    let op = id();
    let occupied = dir.path().join(format!("guided-story-backup-{op}"));
    fs::create_dir(&occupied).unwrap();
    fs::write(occupied.join("keep.txt"), "keep").unwrap();
    assert!(p.export_backup(dir.path(), &op, 0).is_err());
    assert_eq!(
        fs::read_to_string(occupied.join("keep.txt")).unwrap(),
        "keep"
    );
    let lock = transfer_lock(dir.path()).unwrap();
    assert!(p.export_backup(dir.path(), &id(), 0).is_err());
    drop(lock);
    let backup = p.export_backup(dir.path(), &id(), 0).unwrap();
    assert!(restore_backup(
        Path::new(&backup.folder_path),
        Path::new(&backup.folder_path),
        &id()
    )
    .is_err());
    let op = id();
    let occupied = dir.path().join(format!("guided-story-restored-{op}"));
    fs::create_dir(&occupied).unwrap();
    assert!(restore_backup(Path::new(&backup.folder_path), dir.path(), &op).is_err());
    assert!(occupied.is_dir());
}
#[test]
fn incomplete_copy_mismatch_and_source_changes_do_not_publish() {
    let (dir, p) = fixture();
    let op = id();
    let result = p.export_backup_with_hook(dir.path(), &op, 0, |point| {
        if point == "copied" {
            Err("interrupted".into())
        } else {
            Ok(())
        }
    });
    assert!(result.is_err());
    let staging = dir
        .path()
        .join(format!(".guided-story-backup-{op}.incomplete"));
    fs::write(staging.join("project/head.json"), "changed").unwrap();
    assert!(p.export_backup(dir.path(), &op, 0).is_err());
    assert!(!dir
        .path()
        .join(format!("guided-story-backup-{op}"))
        .exists());
    assert_eq!(p.view().unwrap().revision, 0);
}
#[test]
fn failed_after_publication_retries_a_complete_export_and_restore() {
    let (dir, p) = fixture();
    let op = id();
    assert!(p
        .export_backup_with_hook(dir.path(), &op, 0, |point| if point == "published" {
            Err("reply lost".into())
        } else {
            Ok(())
        })
        .is_err());
    let backup = p.export_backup(dir.path(), &op, 0).unwrap();
    let restore = id();
    assert!(restore_with_hook(
        Path::new(&backup.folder_path),
        dir.path(),
        &restore,
        &fingerprint(Path::new(&backup.folder_path)),
        |point| if point == "published" {
            Err("reply lost".into())
        } else {
            Ok(())
        }
    )
    .is_err());
    let r = restore_backup(Path::new(&backup.folder_path), dir.path(), &restore).unwrap();
    assert!(Project::open(Path::new(&r.folder_path)).is_ok());
}
#[test]
fn resumed_atomic_write_remnants_are_removed_and_unlisted_backup_temps_are_rejected() {
    let (dir, p) = fixture();
    let op = id();
    assert!(p
        .export_backup_with_hook(dir.path(), &op, 0, |point| if point == "copied" {
            Err("interrupted".into())
        } else {
            Ok(())
        })
        .is_err());
    let staging = dir
        .path()
        .join(format!(".guided-story-backup-{op}.incomplete"));
    let temp = staging
        .join("project/worlds")
        .join(format!(".{}.tmp", id()));
    fs::write(&temp, "partial write").unwrap();
    let backup = p.export_backup(dir.path(), &op, 0).unwrap();
    assert!(!temp.exists());
    let root = Path::new(&backup.folder_path);
    assert!(!root
        .join("project/worlds")
        .join(temp.file_name().unwrap())
        .exists());
    fs::write(
        root.join("project").join(format!(".{}.tmp", id())),
        "unlisted",
    )
    .unwrap();
    assert!(inspect_backup(root).is_err());
}
#[test]
fn real_process_exit_at_copy_validation_and_publication_is_retryable() {
    for kind in ["backup", "restore"] {
        for point in ["copied", "validated", "published"] {
            let (dir, p) = fixture();
            let source = if kind == "backup" {
                p.folder_path()
            } else {
                p.export_backup(dir.path(), &id(), 0).unwrap().folder_path
            };
            drop(p);
            let op = id();
            let child = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "store::backup::tests::backup_crash_child",
                    "--ignored",
                    "--nocapture",
                ])
                .env("STORY_BACKUP_CHILD_SOURCE", &source)
                .env("STORY_BACKUP_CHILD_PARENT", dir.path())
                .env("STORY_BACKUP_CHILD_ID", &op)
                .env("STORY_BACKUP_CHILD_KIND", kind)
                .env("STORY_BACKUP_CHILD_POINT", point)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap();
            assert_eq!(child.code(), Some(77), "{kind}/{point}");
            let destination = dir.path().join(format!(
                "guided-story-{}-{op}",
                if kind == "backup" {
                    "backup"
                } else {
                    "restored"
                }
            ));
            assert_eq!(destination.exists(), point == "published");
            let r = if kind == "backup" {
                Project::open(Path::new(&source))
                    .unwrap()
                    .export_backup(dir.path(), &op, 0)
                    .unwrap()
            } else {
                restore_backup(Path::new(&source), dir.path(), &op).unwrap()
            };
            if kind == "backup" {
                inspect_backup(Path::new(&r.folder_path)).unwrap();
            } else {
                Project::open(Path::new(&r.folder_path)).unwrap();
            }
        }
    }
}
#[test]
#[ignore = "child process interruption helper"]
fn backup_crash_child() {
    let Ok(source) = std::env::var("STORY_BACKUP_CHILD_SOURCE") else {
        return;
    };
    let parent = std::env::var("STORY_BACKUP_CHILD_PARENT").unwrap();
    let op = std::env::var("STORY_BACKUP_CHILD_ID").unwrap();
    let point = std::env::var("STORY_BACKUP_CHILD_POINT").unwrap();
    let hook = |at: &str| {
        if at == point {
            std::process::exit(77);
        }
        Ok(())
    };
    if std::env::var("STORY_BACKUP_CHILD_KIND").unwrap() == "backup" {
        Project::open(Path::new(&source))
            .unwrap()
            .export_backup_with_hook(Path::new(&parent), &op, 0, hook)
            .unwrap();
    } else {
        restore_with_hook(
            Path::new(&source),
            Path::new(&parent),
            &op,
            &fingerprint(Path::new(&source)),
            hook,
        )
        .unwrap();
    }
}
