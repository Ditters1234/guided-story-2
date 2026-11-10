use super::*;
use uuid::Uuid;

fn id() -> String {
    Uuid::new_v4().to_string()
}
fn node(world: &mut World, type_id: &str, name: &str) -> String {
    let preset = world.node_types.iter().find(|d| d.id == type_id).unwrap();
    let components = preset
        .components
        .iter()
        .map(|id| {
            (
                id.clone(),
                crate::default_properties(
                    world.component_types.iter().find(|d| &d.id == id).unwrap(),
                ),
            )
        })
        .collect();
    let id = id();
    world.nodes.push(WorldNode {
        id: id.clone(),
        type_id: type_id.into(),
        name: name.into(),
        description: String::new(),
        visual_description: String::new(),
        components,
    });
    id
}
fn fixture() -> (World, Vec<String>) {
    let mut w = World::default();
    let room = node(&mut w, "core.place", "Workshop");
    let person = node(&mut w, "core.character", "Fictional adult");
    let bag = node(&mut w, "core.item", "Bag");
    w.edges.push(WorldEdge {
        id: id(),
        type_id: "core.present-at".into(),
        source_id: person.clone(),
        target_id: room,
        description: String::new(),
        properties: BTreeMap::new(),
    });
    w.edges.push(WorldEdge {
        id: id(),
        type_id: "core.stored-in".into(),
        source_id: bag.clone(),
        target_id: person.clone(),
        description: "Carried".into(),
        properties: BTreeMap::new(),
    });
    (w, vec![person, bag])
}
fn exported() -> LibraryEntry {
    let (w, ids) = fixture();
    export_library_entry(
        &w,
        ids,
        BTreeMap::new(),
        "Traveller with bag".into(),
        "Reusable fictional group".into(),
    )
    .unwrap()
    .entry
}

#[test]
fn selection_export_is_standalone_scoped_and_keeps_internal_links() {
    let (mut w, ids) = fixture();
    let before = w.clone();
    w.nodes[0].description = "UNSELECTED PRIVATE DETAIL".into();
    let result = export_library_entry(
        &w,
        ids.clone(),
        BTreeMap::from([(ids[0].clone(), Position { x: 100., y: 50. })]),
        "Traveller".into(),
        String::new(),
    )
    .unwrap();
    let text = serde_json::to_string(&result.entry).unwrap();
    assert!(!text.contains("UNSELECTED PRIVATE DETAIL"));
    assert_eq!(result.entry.world.nodes.len(), 2);
    assert_eq!(result.entry.world.edges.len(), 1);
    assert!(result.notices.iter().any(|n| n.contains("Omit 1")));
    assert_eq!(w.nodes.len(), before.nodes.len());
    let empty = World::default();
    let inserted = preview_world_edit(
        &empty,
        WorldEdit::InsertLibrary {
            entry: result.entry.clone(),
        },
    )
    .unwrap()
    .world;
    assert_eq!(inserted.nodes.len(), 2);
    assert_eq!(inserted.nodes[0].name, "Fictional adult");
    assert!(inserted.nodes.iter().all(|n| !ids.contains(&n.id)));
    let second = preview_world_edit(
        &inserted,
        WorldEdit::InsertLibrary {
            entry: result.entry,
        },
    )
    .unwrap()
    .world;
    assert_eq!(second.nodes.len(), 4);
    assert_eq!(second.edges.len(), 2);
    second.validate().unwrap();
}

#[test]
fn library_file_roundtrips_replays_and_detects_changed_or_invalid_files() {
    let temp = tempfile::tempdir().unwrap();
    let entry = exported();
    let request = id();
    let name = save_library_entry(temp.path(), &request, &entry).unwrap();
    assert_eq!(
        save_library_entry(temp.path(), &request, &entry).unwrap(),
        name
    );
    let listing = list_library(temp.path()).unwrap();
    assert_eq!(listing.files.len(), 1);
    let file = &listing.files[0];
    assert!(file.error.is_none());
    assert_eq!(
        read_library_entry(temp.path(), &name, &file.sha256)
            .unwrap()
            .name,
        entry.name
    );
    let path = Path::new(&listing.folder_path).join(&name);
    let original = fs::read(&path).unwrap();
    let mut different = entry.clone();
    different.name = "Different".into();
    assert!(save_library_entry(temp.path(), &request, &different).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    fs::write(&path, serde_json::to_vec_pretty(&different).unwrap()).unwrap();
    assert!(read_library_entry(temp.path(), &name, &file.sha256)
        .unwrap_err()
        .contains("changed"));
    fs::write(
        Path::new(&listing.folder_path).join("broken.json"),
        b"{broken",
    )
    .unwrap();
    let refreshed = list_library(temp.path()).unwrap();
    assert_eq!(refreshed.files.len(), 2);
    assert_eq!(
        refreshed.files.iter().filter(|f| f.error.is_some()).count(),
        1
    );
    assert!(read_library_entry(temp.path(), "../outside.json", "").is_err());
}

#[test]
fn interrupted_library_publication_never_exposes_partial_json_or_overwrites() {
    for boundary in ["before-publish", "after-publish"] {
        let temp = tempfile::tempdir().unwrap();
        let request = id();
        let entry = exported();
        assert!(save_with_hook(
            temp.path(),
            &request,
            &entry,
            |stage| if stage == boundary {
                Err("Interrupted".into())
            } else {
                Ok(())
            }
        )
        .is_err());
        let listing = list_library(temp.path()).unwrap();
        assert_eq!(
            listing.files.len(),
            usize::from(boundary == "after-publish")
        );
        let name = save_library_entry(temp.path(), &request, &entry).unwrap();
        let listing = list_library(temp.path()).unwrap();
        assert_eq!(listing.files.len(), 1);
        assert_eq!(listing.files[0].file_name, name);
        assert!(listing.files[0].error.is_none());
    }
}

#[test]
fn library_folder_choice_is_device_local_and_survives_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("config");
    let folder = temp.path().join("shared-library");
    fs::create_dir(&folder).unwrap();
    let listing = choose_library_folder(&config, folder.to_str().unwrap()).unwrap();
    assert_eq!(
        listing.folder_path,
        list_library(&config).unwrap().folder_path
    );
    save_library_entry(&config, &id(), &exported()).unwrap();
    assert_eq!(list_library(&config).unwrap().files.len(), 1);
    assert!(!folder.join("node-library.json").exists());
}

#[test]
fn schema_conflicts_and_story_metadata_are_rejected_without_mutation() {
    let entry = exported();
    let mut target = World::default();
    target
        .node_types
        .iter_mut()
        .find(|d| d.id == entry.world.nodes[0].type_id)
        .unwrap()
        .description = "Different saved definition".into();
    let before = target.clone();
    assert!(preview_world_edit(
        &target,
        WorldEdit::InsertLibrary {
            entry: entry.clone()
        }
    )
    .unwrap_err()
    .contains("schema conflict"));
    assert_eq!(target, before);
    let mut invalid = entry;
    invalid.schema_version = 2;
    assert!(invalid.validate().is_err());
    invalid.schema_version = 1;
    invalid.world.ai_context.push(AiContextBinding {
        node_id: invalid.world.nodes[0].id.clone(),
        enabled: true,
        role: AiContextRole::Instructions,
        tasks: vec![TextPromptTask::Passage],
        fields: vec![AiContextField::Description],
        order: 0,
        image_preparation: false,
    });
    assert!(invalid.validate().is_err());
}

#[test]
fn custom_fields_links_and_instance_bound_defaults_remain_independent_on_repeated_import() {
    let (mut world, ids) = fixture();
    let person = ids[0].clone();
    let bag = ids[1].clone();
    let outside = world.nodes[0].id.clone();
    let fields = vec![
        FieldDefinition {
            key: "target".into(),
            label: "Target".into(),
            kind: FieldKind::NodeRef,
            required: true,
            default_value: PropertyValue::Text(person.clone()),
            metadata: None,
        },
        FieldDefinition {
            key: "others".into(),
            label: "Others".into(),
            kind: FieldKind::NodeRefs,
            required: false,
            default_value: PropertyValue::NodeRefs(vec![]),
            metadata: None,
        },
        FieldDefinition {
            key: "notes".into(),
            label: "Notes".into(),
            kind: FieldKind::Text,
            required: false,
            default_value: PropertyValue::Text(String::new()),
            metadata: None,
        },
    ];
    world.component_types.push(ComponentDefinition {
        id: "custom.traveller".into(),
        name: "Traveller details".into(),
        description: "Portable custom data".into(),
        fields: fields.clone(),
    });
    world
        .nodes
        .iter_mut()
        .find(|n| n.id == bag)
        .unwrap()
        .components
        .insert(
            "custom.traveller".into(),
            BTreeMap::from([
                ("target".into(), PropertyValue::Text(person.clone())),
                (
                    "others".into(),
                    PropertyValue::NodeRefs(vec![person.clone(), outside]),
                ),
                ("notes".into(), PropertyValue::Text(person.clone())),
            ]),
        );
    world.relation_types.push(RelationDefinition {
        id: "custom.attachment".into(),
        name: "Attachment".into(),
        label: "attachment".into(),
        source_capability: None,
        target_capability: None,
        physical_placement: false,
        single_target: false,
        fields,
        description: String::new(),
    });
    world.edges.push(WorldEdge {
        id: id(),
        type_id: "custom.attachment".into(),
        source_id: bag.clone(),
        target_id: person.clone(),
        description: "Custom rich link".into(),
        properties: BTreeMap::from([("target".into(), PropertyValue::Text(person.clone()))]),
    });
    world.validate().unwrap();
    let entry = export_library_entry(
        &world,
        ids.clone(),
        BTreeMap::new(),
        "Custom group".into(),
        String::new(),
    )
    .unwrap()
    .entry;
    assert_eq!(
        entry.world.nodes[1].components["custom.traveller"]["others"],
        PropertyValue::NodeRefs(vec![person.clone()])
    );
    let mut imported = World::default();
    for round in 0..2 {
        let original = imported.clone();
        imported = preview_world_edit(
            &imported,
            WorldEdit::InsertLibrary {
                entry: entry.clone(),
            },
        )
        .unwrap()
        .world;
        assert_eq!(
            &imported.nodes[..original.nodes.len()],
            original.nodes.as_slice()
        );
        let person = &imported.nodes[round * 2];
        let bag = &imported.nodes[round * 2 + 1];
        let (capability, properties) = bag
            .components
            .iter()
            .find(|(key, _)| key.starts_with("library."))
            .unwrap();
        assert_eq!(properties["target"], PropertyValue::Text(person.id.clone()));
        assert_eq!(properties["notes"], PropertyValue::Text(ids[0].clone()));
        let definition = imported
            .component_types
            .iter()
            .find(|d| &d.id == capability)
            .unwrap();
        assert_eq!(
            definition.fields[0].default_value,
            PropertyValue::Text(person.id.clone())
        );
        let link = imported
            .edges
            .iter()
            .find(|e| e.source_id == bag.id && e.description == "Custom rich link")
            .unwrap();
        assert_eq!(link.target_id, person.id);
        assert_eq!(
            link.properties["target"],
            PropertyValue::Text(person.id.clone())
        );
        imported.validate().unwrap();
    }
}

#[test]
fn imports_publish_once_preserve_history_and_reopen_without_library_files() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("story");
    let before = World::default();
    let project = Project::create(
        &path,
        "Fictional library check".into(),
        before.clone(),
        id(),
    )
    .unwrap();
    let original = project.view().unwrap();
    let world = preview_world_edit(&before, WorldEdit::InsertLibrary { entry: exported() })
        .unwrap()
        .world;
    let request = id();
    project
        .commit(
            0,
            request.clone(),
            "Fictional library check".into(),
            world.clone(),
        )
        .unwrap();
    assert!(
        project
            .commit(0, request, "Fictional library check".into(), world.clone())
            .unwrap()
            .replayed
    );
    drop(project);
    assert_eq!(Project::open(&path).unwrap().view().unwrap().world, world);
    let bytes = fs::read(
        path.join("worlds")
            .join(format!("{}.json", original.world_ref.id)),
    )
    .unwrap();
    assert_eq!(serde_json::from_slice::<World>(&bytes).unwrap(), before);
    assert!(choose_library_folder(temp.path(), path.to_str().unwrap()).is_err());
}

#[test]
fn examples_are_valid_runtime_files_with_independent_repeated_instances() {
    for bytes in [
        include_bytes!("../../../../examples/node-library/surveyor-and-belongings.json").as_slice(),
        include_bytes!("../../../../examples/node-library/workshop-corner.json").as_slice(),
    ] {
        let entry = decode_entry(bytes).unwrap();
        let world = World::default();
        let first = preview_world_edit(
            &world,
            WorldEdit::InsertLibrary {
                entry: entry.clone(),
            },
        )
        .unwrap()
        .world;
        let second = preview_world_edit(
            &first,
            WorldEdit::InsertLibrary {
                entry: entry.clone(),
            },
        )
        .unwrap()
        .world;
        assert_eq!(second.nodes.len(), entry.world.nodes.len() * 2);
        assert_eq!(second.edges.len(), entry.world.edges.len() * 2);
        second.validate().unwrap();
    }
}

#[test]
fn events_export_without_cross_story_citations_or_implicit_ai_bindings() {
    let mut world = World::default();
    let event = node(&mut world, "core.event", "A repaired pump");
    crate::sources::ensure_manual_event_sources(&mut world);
    world.ai_context.push(AiContextBinding {
        node_id: event.clone(),
        enabled: true,
        role: AiContextRole::Reference,
        tasks: vec![TextPromptTask::Passage],
        fields: vec![AiContextField::Description],
        order: 0,
        image_preparation: false,
    });
    let entry = export_library_entry(
        &world,
        vec![event.clone()],
        BTreeMap::new(),
        "Past repair".into(),
        String::new(),
    )
    .unwrap()
    .entry;
    assert!(entry.world.source_annotations.is_empty());
    assert!(entry.world.ai_context.is_empty());
    let imported = preview_world_edit(&World::default(), WorldEdit::InsertLibrary { entry })
        .unwrap()
        .world;
    assert!(imported.ai_context.is_empty());
    assert_eq!(imported.source_annotations.len(), 1);
    assert_ne!(imported.source_annotations[0].node_id, event);
    assert!(matches!(
        imported.source_annotations[0].origin,
        SourceOrigin::Manual { .. }
    ));
    assert!(imported.edges.is_empty());
}

#[test]
fn process_exit_before_or_after_library_publication_is_recoverable_without_duplicates() {
    use std::process::{Command, Stdio};
    for stage in ["before-publish", "after-publish"] {
        let temp = tempfile::tempdir().unwrap();
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "node_library::tests::library_crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("STORY_LIBRARY_TEST_ROOT", temp.path())
            .env("STORY_LIBRARY_TEST_STAGE", stage)
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(73));
        let listing = list_library(temp.path()).unwrap();
        assert_eq!(listing.files.len(), usize::from(stage == "after-publish"));
        let entry: LibraryEntry =
            serde_json::from_slice(&fs::read(temp.path().join("captured-entry.json")).unwrap())
                .unwrap();
        save_library_entry(temp.path(), "bb000000-0000-4000-8000-000000000001", &entry).unwrap();
        assert_eq!(list_library(temp.path()).unwrap().files.len(), 1);
    }
}

#[test]
#[ignore = "isolated library process-interruption helper"]
fn library_crash_child() {
    let Ok(root) = std::env::var("STORY_LIBRARY_TEST_ROOT") else {
        return;
    };
    let stage = std::env::var("STORY_LIBRARY_TEST_STAGE").unwrap();
    let root = Path::new(&root);
    let entry = exported();
    fs::write(
        root.join("captured-entry.json"),
        serde_json::to_vec(&entry).unwrap(),
    )
    .unwrap();
    save_with_hook(
        root,
        "bb000000-0000-4000-8000-000000000001",
        &entry,
        |boundary| {
            if boundary == stage {
                std::process::exit(73);
            }
            Ok(())
        },
    )
    .unwrap();
    panic!("Publication was not interrupted");
}
