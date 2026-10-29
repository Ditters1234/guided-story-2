use super::*;
use std::process::{Command, Stdio};
use tempfile::TempDir;
mod wearing_compatibility;

fn request() -> String {
    Uuid::new_v4().to_string()
}
fn create() -> (TempDir, Project) {
    let dir = tempfile::tempdir().unwrap();
    let project = Project::create(
        &dir.path().join("story"),
        "Test story".into(),
        World::default(),
        request(),
    )
    .unwrap();
    (dir, project)
}

#[test]
fn saved_world_is_immutable_and_stale_writes_are_rejected() {
    let (_dir, project) = create();
    let before = project.view().unwrap();
    let after = project
        .commit(0, request(), "New title".into(), World::default())
        .unwrap();
    assert_eq!(after.project.revision, 1);
    assert_eq!(before.title, "Test story");
    assert!(project
        .record_path("worlds", &before.world_ref)
        .unwrap()
        .exists());
    assert!(project
        .commit(0, request(), "Stale".into(), World::default())
        .is_err());
    assert_eq!(project.view().unwrap().title, "New title");
}

#[test]
fn duplicate_requests_commit_once_even_after_another_save() {
    let (_dir, project) = create();
    let id = request();
    project
        .commit(0, id.clone(), "First".into(), World::default())
        .unwrap();
    project
        .commit(1, request(), "Second".into(), World::default())
        .unwrap();
    let replay = project
        .commit(0, id.clone(), "First".into(), World::default())
        .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.committed_revision, 1);
    assert_eq!(replay.project.revision, 2);
    assert_eq!(replay.project.title, "Second");
    assert!(project
        .commit(2, id, "Different request".into(), World::default())
        .is_err());
}

#[test]
fn os_lock_blocks_competing_open_and_releases_on_drop() {
    let (_dir, project) = create();
    let path = project.root.clone();
    assert!(Project::open(&path).is_err());
    drop(project);
    assert!(Project::open(&path).is_ok());
}

#[test]
fn invalid_saved_layout_is_reported_and_cannot_be_overwritten_silently() {
    let (_dir, project) = create();
    let id = request();
    let mut workspace = Workspace::default();
    workspace
        .positions
        .insert(id.clone(), Position { x: 1e8, y: 0.0 });
    assert!(project.save_workspace(0, workspace.clone()).is_err());
    let path = project.root.join("workspace.json");
    fs::write(&path, serde_json::to_vec(&workspace).unwrap()).unwrap();
    let before = fs::read(&path).unwrap();
    assert!(project
        .load_workspace()
        .unwrap_err()
        .contains("Invalid map position"));
    assert!(project.save_workspace(0, Workspace::default()).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(project.view().unwrap().revision, 0);
}

#[test]
fn moved_folder_opens_without_old_absolute_paths() {
    let (dir, project) = create();
    let story_id = project.view().unwrap().story_id;
    let root = project.root.clone();
    drop(project);
    let moved = dir.path().join("moved-story");
    fs::rename(root, &moved).unwrap();
    assert_eq!(
        Project::open(&moved).unwrap().view().unwrap().story_id,
        story_id
    );
}

#[test]
fn corrupt_head_never_defaults_and_recovery_is_explicit() {
    let (_dir, project) = create();
    project
        .commit(0, request(), "Later title".into(), World::default())
        .unwrap();
    let root = project.root.clone();
    fs::write(root.join("head.json"), b"{broken").unwrap();
    drop(project);
    assert!(Project::open(&root).is_err());
    assert_eq!(fs::read(root.join("head.json")).unwrap(), b"{broken");
    let recovered = Project::recover(&root).unwrap();
    assert_eq!(recovered.view().unwrap().title, "Test story");
}

#[test]
fn modified_world_checksum_is_rejected_without_replacing_data() {
    let (_dir, project) = create();
    let view = project.view().unwrap();
    fs::write(
        project.record_path("worlds", &view.world_ref).unwrap(),
        b"{}",
    )
    .unwrap();
    assert!(project.view().is_err());
    assert!(project
        .commit(0, request(), "Overwrite".into(), World::default())
        .is_err());
}

#[test]
fn workspace_is_independent_and_uses_its_own_version() {
    let (_dir, project) = create();
    let workspace = project.save_workspace(0, Workspace::default()).unwrap();
    assert_eq!(workspace.version, 1);
    assert_eq!(project.view().unwrap().revision, 0);
    assert!(project.save_workspace(0, workspace).is_err());
}

#[test]
fn failed_publication_keeps_old_head() {
    let (_dir, project) = create();
    let old_head = read_bytes(&project.root.join("head.json")).unwrap();
    let result =
        project.commit_with_hook(0, request(), "Unsaved".into(), World::default(), |stage| {
            if stage == "before-head" {
                Err("Injected publication failure".into())
            } else {
                Ok(())
            }
        });
    assert!(result.is_err());
    assert_eq!(
        read_bytes(&project.root.join("head.json")).unwrap(),
        old_head
    );
    assert_eq!(project.view().unwrap().revision, 0);
}

#[test]
fn malformed_reference_cannot_escape_project_folder() {
    let (_dir, project) = create();
    let reference = RecordRef {
        id: "../../outside".into(),
        sha256: "0".repeat(64),
    };
    assert!(project.record_path("worlds", &reference).is_err());
}

#[test]
fn interruption_at_each_commit_boundary_leaves_one_complete_revision() {
    for stage in [
        "before-world",
        "after-world",
        "after-manifest",
        "before-head",
        "after-head",
    ] {
        let (_dir, project) = create();
        let root = project.root.clone();
        drop(project);
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "store::tests::crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("STORY_CRASH_ROOT", &root)
            .env("STORY_CRASH_STAGE", stage)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(73), "child did not reach {stage}");
        let project = Project::open(&root).unwrap();
        let published = stage == "after-head";
        assert_eq!(project.view().unwrap().revision, u32::from(published));
        if published {
            let replay = project
                .commit(
                    0,
                    "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb".into(),
                    "Revised story".into(),
                    World::default(),
                )
                .unwrap();
            assert!(replay.replayed);
            assert_eq!(replay.project.revision, 1);
        }
    }
}

#[test]
#[ignore = "only invoked as an isolated crash probe"]
fn crash_child() {
    let Ok(root) = std::env::var("STORY_CRASH_ROOT") else {
        return;
    };
    let stage = std::env::var("STORY_CRASH_STAGE").unwrap();
    let project = Project::open(Path::new(&root)).unwrap();
    project
        .commit_with_hook(
            0,
            "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb".into(),
            "Revised story".into(),
            World::default(),
            |current| {
                if stage == current {
                    std::process::exit(73);
                }
                Ok(())
            },
        )
        .unwrap();
    panic!("probe never interrupted publication");
}

fn node(world: &World, preset: &str, name: &str) -> WorldNode {
    let definition = world.node_types.iter().find(|d| d.id == preset).unwrap();
    let components = definition
        .components
        .iter()
        .map(|id| {
            let definition = world.component_types.iter().find(|d| &d.id == id).unwrap();
            (
                id.clone(),
                definition
                    .fields
                    .iter()
                    .map(|f| (f.key.clone(), f.default_value.clone()))
                    .collect(),
            )
        })
        .collect();
    WorldNode {
        id: request(),
        type_id: preset.into(),
        name: name.into(),
        description: "".into(),
        visual_description: "".into(),
        components,
    }
}
fn edge(kind: &str, source: &str, target: &str) -> WorldEdge {
    WorldEdge {
        properties: Default::default(),
        id: request(),
        type_id: kind.into(),
        source_id: source.into(),
        target_id: target.into(),
        description: "".into(),
    }
}

#[test]
fn cycles_and_duplicate_entities_are_rejected() {
    let mut world = World::default();
    let first = node(&world, "core.place", "Room");
    let second = node(&world, "core.place", "House");
    world.edges = vec![
        edge("core.inside", &first.id, &second.id),
        edge("core.inside", &second.id, &first.id),
    ];
    world.nodes = vec![first, second];
    assert!(world.validate().is_err());
    world.edges.clear();
    world.nodes.push(world.nodes[0].clone());
    assert!(world.validate().is_err());
}

#[test]
fn container_is_a_capability_and_wearing_is_exclusive_physical_placement() {
    let mut world = World::default();
    let person = node(&world, "core.character", "Mara");
    let mut bag = node(&world, "core.item", "Satchel");
    bag.components
        .insert("core.container".into(), BTreeMap::new());
    let mut coat = node(&world, "core.item", "Coat");
    coat.components.insert(
        "core.wearable".into(),
        BTreeMap::from([
            ("regions".into(), PropertyValue::Text("torso".into())),
            ("layer".into(), PropertyValue::Number(1.0)),
        ]),
    );
    world.edges = vec![
        edge("core.stored-in", &bag.id, &person.id),
        edge("core.stored-in", &coat.id, &bag.id),
        edge("core.worn-by", &coat.id, &person.id),
    ];
    world.nodes = vec![person.clone(), bag.clone(), coat.clone()];
    assert!(world.validate().is_err());
    world.edges.remove(1);
    assert!(world.validate().is_ok());
    world.edges[0].target_id = bag.id;
    assert!(world.validate().is_err());
}

#[test]
fn custom_node_types_fields_and_relations_round_trip_without_engine_changes() {
    let (_dir, project) = create();
    let mut world = World::default();
    world.component_types.push(ComponentDefinition {
        id: "custom.weather".into(),
        name: "Weather".into(),
        description: "".into(),
        fields: vec![FieldDefinition {
            key: "forecast".into(),
            label: "Forecast".into(),
            kind: FieldKind::Text,
            required: true,
            default_value: PropertyValue::Text("Unknown".into()),
            metadata: None,
        }],
    });
    world.node_types.push(NodeTypeDefinition {
        id: "custom.sky".into(),
        name: "Weather system".into(),
        description: "".into(),
        components: vec!["custom.weather".into()],
    });
    let storm = node(&world, "custom.sky", "The approaching storm");
    let place = node(&world, "core.place", "The valley");
    world.relation_types.push(RelationDefinition {
        fields: vec![],
        id: "custom.affects".into(),
        name: "Affects".into(),
        label: "affects".into(),
        source_capability: None,
        target_capability: None,
        physical_placement: false,
        single_target: false,
    });
    world
        .edges
        .push(edge("custom.affects", &storm.id, &place.id));
    world.nodes = vec![storm, place];
    let result = project
        .commit(0, request(), "Custom world".into(), world.clone())
        .unwrap();
    assert_eq!(result.project.world, world);
    assert_eq!(project.view().unwrap().world, world);
}

#[test]
fn invalid_custom_values_and_dangling_references_are_blocked() {
    let mut world = World::default();
    let mut item = node(&world, "core.item", "Lantern");
    item.components
        .get_mut("core.item")
        .unwrap()
        .insert("quantity".into(), PropertyValue::Text("many".into()));
    world.nodes.push(item);
    assert!(world.validate().is_err());
    world.nodes[0]
        .components
        .get_mut("core.item")
        .unwrap()
        .insert("quantity".into(), PropertyValue::Number(1.0));
    world
        .edges
        .push(edge("core.related", &world.nodes[0].id, &request()));
    assert!(world.validate().is_err());
}

#[test]
fn invalid_editor_drafts_are_preserved_without_changing_canon() {
    let (_dir, project) = create();
    let mut world = World::default();
    let mut item = node(&world, "core.item", "Draft");
    item.name.clear();
    world.nodes.push(item);
    project
        .save_draft(EditorDraft {
            schema_version: 2,
            source_revision: 0,
            title: "Test story".into(),
            world: world.clone(),
        })
        .unwrap();
    assert_eq!(project.load_draft().unwrap().unwrap().world, world);
    assert!(project
        .commit(0, request(), "Test story".into(), world)
        .is_err());
    assert!(project.view().unwrap().world.nodes.is_empty());
}

#[test]
fn custom_schema_cannot_disable_builtin_inventory_validation() {
    let mut world = World::default();
    world
        .relation_types
        .iter_mut()
        .find(|r| r.id == "core.stored-in")
        .unwrap()
        .physical_placement = false;
    assert!(world.validate().is_err());
}

#[test]
fn replacing_an_older_draft_archives_it_instead_of_silently_erasing_it() {
    let (_dir, project) = create();
    project
        .save_draft(EditorDraft {
            schema_version: 2,
            source_revision: 0,
            title: "Unfinished".into(),
            world: World::default(),
        })
        .unwrap();
    project
        .commit(0, request(), "Accepted".into(), World::default())
        .unwrap();
    let mut next = World::default();
    next.nodes.push(node(&next, "core.place", "New place"));
    project
        .save_draft(EditorDraft {
            schema_version: 2,
            source_revision: 1,
            title: "Accepted".into(),
            world: next,
        })
        .unwrap();
    assert!(fs::read_dir(project.root.join("backups"))
        .unwrap()
        .flatten()
        .any(|e| e.file_name().to_string_lossy().starts_with("draft-")));
}

#[cfg(windows)]
#[test]
fn windows_file_retry_is_bounded_and_does_not_repeat_other_failures() {
    let mut attempts = 0;
    let mut waited = Vec::new();
    let result = retry_windows_io(
        || {
            attempts += 1;
            if attempts < 3 {
                Err(std::io::Error::from_raw_os_error(32))
            } else {
                Ok(7)
            }
        },
        |delay| waited.push(delay.as_millis()),
    );
    assert_eq!(result.unwrap(), 7);
    assert_eq!(attempts, 3);
    assert_eq!(waited, vec![20, 50]);
    let mut attempts = 0;
    let mut waited = 0;
    let result: std::io::Result<()> = retry_windows_io(
        || {
            attempts += 1;
            Err(std::io::Error::from_raw_os_error(5))
        },
        |d| waited += d.as_millis(),
    );
    assert!(result.is_err());
    assert_eq!(attempts, 6);
    assert_eq!(waited, 770);
    let mut attempts = 0;
    let result: std::io::Result<()> = retry_windows_io(
        || {
            attempts += 1;
            Err(std::io::Error::from_raw_os_error(2))
        },
        |_| panic!("Not-found must not retry"),
    );
    assert!(result.is_err());
    assert_eq!(attempts, 1);
}
#[test]
fn immutable_publication_never_overwrites_existing_files_or_folders() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.json");
    let target = temp.path().join("target.json");
    std::fs::write(&source, b"source").unwrap();
    std::fs::write(&target, b"existing").unwrap();
    assert!(publish_new_path(&source, &target).is_err());
    assert_eq!(std::fs::read(&target).unwrap(), b"existing");
    assert!(source.exists());
    let source = temp.path().join("source-dir");
    let target = temp.path().join("target-dir");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir(&target).unwrap();
    assert!(publish_new_path(&source, &target).is_err());
    assert!(source.exists());
}
