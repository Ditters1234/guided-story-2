use super::*;
use crate::{ComponentDefinition, FieldDefinition, Project, WorldNode};
use std::collections::BTreeMap;
use uuid::Uuid;

fn id() -> String {
    Uuid::new_v4().to_string()
}
fn node(world: &mut World, preset: &str, name: &str) -> String {
    let components = world
        .node_types
        .iter()
        .find(|d| d.id == preset)
        .unwrap()
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
    let id = id();
    world.nodes.push(WorldNode {
        id: id.clone(),
        type_id: preset.into(),
        name: name.into(),
        description: String::new(),
        visual_description: String::new(),
        components,
    });
    id
}
fn link(world: &mut World, kind: &str, source: &str, target: &str) -> WorldEdge {
    let edge = WorldEdge {
        properties: Default::default(),
        id: id(),
        type_id: kind.into(),
        source_id: source.into(),
        target_id: target.into(),
        description: String::new(),
    };
    world.edges.push(edge.clone());
    edge
}
fn fixture() -> (World, String, String, String, String) {
    let mut w = World::default();
    let place = node(&mut w, "core.place", "House");
    let person = node(&mut w, "core.character", "Adult gardener");
    let bag = node(&mut w, "core.item", "Bag");
    let coat = node(&mut w, "core.item", "Coat");
    w.nodes
        .iter_mut()
        .find(|n| n.id == bag)
        .unwrap()
        .components
        .insert("core.container".into(), BTreeMap::new());
    let defaults = w
        .component_types
        .iter()
        .find(|d| d.id == "core.wearable")
        .unwrap()
        .fields
        .iter()
        .map(|f| (f.key.clone(), f.default_value.clone()))
        .collect();
    w.nodes
        .iter_mut()
        .find(|n| n.id == coat)
        .unwrap()
        .components
        .insert("core.wearable".into(), defaults);
    link(&mut w, "core.present-at", &person, &place);
    link(&mut w, "core.stored-in", &bag, &person);

    link(&mut w, "core.worn-by", &coat, &person);
    w.validate().unwrap();
    (w, place, person, bag, coat)
}
#[test]
fn transfer_ends_wearing_without_mutating_the_source_or_changing_link_identity() {
    let (w, _, person, bag, coat) = fixture();
    let before = w.clone();
    let mut edge = w
        .edges
        .iter()
        .find(|e| e.source_id == coat && e.type_id == "core.worn-by")
        .unwrap()
        .clone();
    edge.type_id = "core.stored-in".into();
    edge.target_id = bag.clone();
    let p = preview_world_edit(&w, WorldEdit::PutLink { link: edge.clone() }).unwrap();
    assert_eq!(w, before);
    assert!(p.world.edges.contains(&edge));
    assert!(!p
        .world
        .edges
        .iter()
        .any(|e| e.source_id == coat && e.type_id == "core.worn-by"));
    assert!(p
        .world
        .edges
        .iter()
        .any(|e| e.source_id == bag && e.target_id == person));
    assert!(p.changes.iter().any(|c| c.contains("worn by")));
}
#[test]
fn wearing_is_one_placement_and_taking_off_changes_it_to_carried() {
    let (w, _, person, bag, coat) = fixture();
    let old = w
        .edges
        .iter()
        .find(|e| e.source_id == coat && e.type_id == "core.worn-by")
        .unwrap();
    let mut moved = old.clone();
    moved.type_id = "core.stored-in".into();
    moved.target_id = bag;
    let w = preview_world_edit(&w, WorldEdit::PutLink { link: moved })
        .unwrap()
        .world;
    let mut next = w.clone();
    let wear = link(&mut next, "core.worn-by", &coat, &person);
    let w = preview_world_edit(&w, WorldEdit::PutLink { link: wear.clone() })
        .unwrap()
        .world;
    assert!(w
        .edges
        .iter()
        .any(|e| e.id == wear.id && e.target_id == person));
    assert_eq!(w.edges.iter().filter(|e| e.source_id == coat).count(), 1);
    let carried = WorldEdge {
        properties: Default::default(),
        type_id: "core.stored-in".into(),
        ..wear.clone()
    };
    let p = preview_world_edit(&w, WorldEdit::PutLink { link: carried }).unwrap();
    assert!(p
        .world
        .edges
        .iter()
        .any(|e| e.id == wear.id && e.target_id == person && e.type_id == "core.stored-in"));
    assert_eq!(
        p.world.edges.iter().filter(|e| e.source_id == coat).count(),
        1
    );
    let detached = preview_world_edit(&p.world, WorldEdit::RemoveLink { id: wear.id }).unwrap();
    assert!(!detached.world.edges.iter().any(|e| e.source_id == coat));
}
#[test]
fn removal_keeps_children_and_clears_only_typed_references() {
    let (mut w, place, person, bag, coat) = fixture();
    let child = node(&mut w, "core.item", "Seeds");
    link(&mut w, "core.stored-in", &child, &bag);
    w.component_types.push(ComponentDefinition {
        id: "custom.memory".into(),
        name: "Memory".into(),
        description: String::new(),
        fields: vec![
            FieldDefinition {
                key: "one".into(),
                label: "One".into(),
                kind: FieldKind::NodeRef,
                required: true,
                default_value: PropertyValue::Text(person.clone()),
                metadata: None,
            },
            FieldDefinition {
                key: "many".into(),
                label: "Many".into(),
                kind: FieldKind::NodeRefs,
                required: false,
                default_value: PropertyValue::NodeRefs(vec![]),
                metadata: None,
            },
            FieldDefinition {
                key: "text".into(),
                label: "Text".into(),
                kind: FieldKind::Text,
                required: false,
                default_value: PropertyValue::Text(String::new()),
                metadata: None,
            },
        ],
    });
    w.nodes
        .iter_mut()
        .find(|n| n.id == place)
        .unwrap()
        .components
        .insert(
            "custom.memory".into(),
            BTreeMap::from([
                ("one".into(), PropertyValue::Text(person.clone())),
                (
                    "many".into(),
                    PropertyValue::NodeRefs(vec![person.clone(), coat.clone()]),
                ),
                ("text".into(), PropertyValue::Text(person.clone())),
            ]),
        );
    let p = preview_world_edit(&w, WorldEdit::RemoveNode { id: person.clone() }).unwrap();
    assert_eq!(p.world.nodes.len(), w.nodes.len() - 1);
    assert!(p
        .world
        .edges
        .iter()
        .any(|e| e.source_id == child && e.target_id == bag));
    assert!(!p
        .world
        .edges
        .iter()
        .any(|e| e.source_id == bag || e.source_id == coat));
    let props = &p
        .world
        .nodes
        .iter()
        .find(|n| n.id == place)
        .unwrap()
        .components["custom.memory"];
    assert_eq!(props["one"], PropertyValue::Null);
    assert_eq!(props["many"], PropertyValue::NodeRefs(vec![coat]));
    assert_eq!(props["text"], PropertyValue::Text(person));
    assert_eq!(
        p.world.component_types.last().unwrap().fields[0].default_value,
        PropertyValue::Null
    );
}
#[test]
fn cycles_invalid_endpoints_and_duplicates_are_rejected_atomically() {
    let (mut w, place, person, bag, coat) = fixture();
    let room = node(&mut w, "core.place", "Room");
    link(&mut w, "core.inside", &room, &place);
    let existing = link(&mut w, "core.related", &person, &place);
    let before = w.clone();
    let mut scratch = w.clone();
    for edge in [
        link(&mut scratch, "core.inside", &place, &room),
        link(&mut scratch, "core.stored-in", &coat, &coat),
        link(&mut scratch, "core.present-at", &person, &bag),
        existing,
    ] {
        let mut edge = edge;
        edge.id = id();
        assert!(preview_world_edit(&w, WorldEdit::PutLink { link: edge }).is_err());
    }
    assert_eq!(w, before);
}

#[test]
fn custom_physical_links_use_the_same_transfer_and_cycle_rules() {
    let (mut w, place, _, bag, coat) = fixture();
    w.relation_types.push(crate::RelationDefinition {
        description: String::new(),
        fields: vec![],
        id: "custom.attached".into(),
        name: "Attached".into(),
        label: "attached to".into(),
        source_capability: None,
        target_capability: None,
        physical_placement: true,
        single_target: true,
    });
    let mut scratch = w.clone();
    let edge = link(&mut scratch, "custom.attached", &bag, &place);
    let w = preview_world_edit(&w, WorldEdit::PutLink { link: edge })
        .unwrap()
        .world;
    assert_eq!(w.edges.iter().filter(|e| e.source_id == bag).count(), 1);
    let edge = link(&mut scratch, "custom.attached", &coat, &bag);
    let w = preview_world_edit(&w, WorldEdit::PutLink { link: edge })
        .unwrap()
        .world;
    assert!(!w
        .edges
        .iter()
        .any(|e| e.source_id == coat && e.type_id == "core.worn-by"));
    let edge = link(&mut scratch, "custom.attached", &place, &coat);
    assert!(preview_world_edit(&w, WorldEdit::PutLink { link: edge }).is_err());
}

#[test]
fn invalid_schema_defaults_and_oversized_connection_notes_are_rejected() {
    let (mut w, _, _, _, _) = fixture();
    w.component_types.push(ComponentDefinition {
        id: "custom.invalid".into(),
        name: "Invalid".into(),
        description: String::new(),
        fields: vec![FieldDefinition {
            key: "number".into(),
            label: "Number".into(),
            kind: FieldKind::Number,
            required: false,
            default_value: PropertyValue::Text("wrong kind".into()),
            metadata: None,
        }],
    });
    assert!(w.validate().unwrap_err().contains("Invalid default"));
    w.component_types.pop();
    w.edges[0].description = "x".repeat(16001);
    assert!(w
        .validate()
        .unwrap_err()
        .contains("description is too long"));
}
#[test]
fn reconnecting_source_releases_old_wearer_and_preserves_unrelated_links() {
    let (mut w, place, person, _, coat) = fixture();
    let other = node(&mut w, "core.item", "Other item");
    let mut edge = w
        .edges
        .iter()
        .find(|e| e.source_id == coat && e.type_id == "core.worn-by")
        .unwrap()
        .clone();
    edge.source_id = other.clone();
    edge.type_id = "core.stored-in".into();
    edge.target_id = place;
    let p = preview_world_edit(&w, WorldEdit::PutLink { link: edge }).unwrap();
    assert!(!p.world.edges.iter().any(|e| e.source_id == coat));
    assert!(p.world.edges.iter().any(|e| e.source_id == other));
    assert!(p.world.edges.iter().any(|e| e.source_id == person));
}
#[test]
fn dependent_capabilities_cannot_be_removed_until_links_are_resolved() {
    let (w, _, _, bag, _) = fixture();
    let mut w = w;
    let child = node(&mut w, "core.item", "Seeds");
    link(&mut w, "core.stored-in", &child, &bag);
    assert!(preview_world_edit(
        &w,
        WorldEdit::RemoveCapability {
            id: bag.clone(),
            capability: "core.container".into()
        }
    )
    .is_err());
    let w = preview_world_edit(
        &w,
        WorldEdit::RemoveLink {
            id: w.edges.last().unwrap().id.clone(),
        },
    )
    .unwrap()
    .world;
    assert!(preview_world_edit(
        &w,
        WorldEdit::RemoveCapability {
            id: bag,
            capability: "core.container".into()
        }
    )
    .is_ok());
}
#[test]
fn structural_edit_commits_once_and_old_snapshot_survives_reopen() {
    let (w, _, person, _, _) = fixture();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("story");
    let project = Project::create(&path, "Fictional".into(), w.clone(), id()).unwrap();
    let old = project.view().unwrap();
    let next = preview_world_edit(&w, WorldEdit::RemoveNode { id: person })
        .unwrap()
        .world;
    let request = id();
    project
        .commit(0, request.clone(), "Fictional".into(), next.clone())
        .unwrap();
    assert!(
        project
            .commit(0, request, "Fictional".into(), next.clone())
            .unwrap()
            .replayed
    );
    let bytes = std::fs::read(
        path.join("worlds")
            .join(format!("{}.json", old.world_ref.id)),
    )
    .unwrap();
    assert_eq!(serde_json::from_slice::<World>(&bytes).unwrap(), w);
    drop(project);
    assert_eq!(Project::open(&path).unwrap().view().unwrap().world, next);
}

#[test]
fn pasted_group_is_independent_and_remaps_typed_node_and_link_references() {
    let (mut w, place, person, bag, coat) = fixture();
    let fields = vec![
        FieldDefinition {
            key: "one".into(),
            label: "One".into(),
            kind: FieldKind::NodeRef,
            required: false,
            default_value: PropertyValue::Null,
            metadata: None,
        },
        FieldDefinition {
            key: "many".into(),
            label: "Many".into(),
            kind: FieldKind::NodeRefs,
            required: false,
            default_value: PropertyValue::NodeRefs(vec![]),
            metadata: None,
        },
        FieldDefinition {
            key: "text".into(),
            label: "Text".into(),
            kind: FieldKind::Text,
            required: false,
            default_value: PropertyValue::Text(String::new()),
            metadata: None,
        },
    ];
    w.component_types.push(ComponentDefinition {
        id: "custom.refs".into(),
        name: "References".into(),
        description: String::new(),
        fields: fields.clone(),
    });
    let values = BTreeMap::from([
        ("one".into(), PropertyValue::Text(person.clone())),
        (
            "many".into(),
            PropertyValue::NodeRefs(vec![person.clone(), place.clone()]),
        ),
        ("text".into(), PropertyValue::Text(person.clone())),
    ]);
    w.nodes
        .iter_mut()
        .find(|n| n.id == bag)
        .unwrap()
        .components
        .insert("custom.refs".into(), values.clone());
    w.relation_types
        .iter_mut()
        .find(|d| d.id == "core.stored-in")
        .unwrap()
        .fields = fields;
    w.edges
        .iter_mut()
        .find(|e| e.source_id == bag)
        .unwrap()
        .properties = values;
    w.validate().unwrap();
    let original = w.clone();
    let ids = BTreeSet::from([person.clone(), bag.clone(), coat.clone()]);
    let nodes: Vec<_> = w
        .nodes
        .iter()
        .filter(|n| ids.contains(&n.id))
        .cloned()
        .collect();
    let edges: Vec<_> = w
        .edges
        .iter()
        .filter(|e| ids.contains(&e.source_id) && ids.contains(&e.target_id))
        .cloned()
        .collect();
    let edit = WorldEdit::PasteNodes { nodes, edges };
    let pasted = preview_world_edit(&w, edit.clone()).unwrap().world;
    assert_eq!(w, original);
    assert_eq!(pasted.nodes.len(), w.nodes.len() + 3);
    assert_eq!(pasted.edges.len(), w.edges.len() + 2);
    assert_eq!(&pasted.nodes[..w.nodes.len()], w.nodes.as_slice());
    let new_person = pasted
        .nodes
        .iter()
        .find(|n| n.name == "Adult gardener (copy)")
        .unwrap();
    let new_bag = pasted
        .nodes
        .iter()
        .find(|n| n.name == "Bag (copy)")
        .unwrap();
    let props = &new_bag.components["custom.refs"];
    assert_eq!(props["one"], PropertyValue::Text(new_person.id.clone()));
    assert_eq!(
        props["many"],
        PropertyValue::NodeRefs(vec![new_person.id.clone()])
    );
    assert_eq!(props["text"], PropertyValue::Text(person));
    let edge = pasted
        .edges
        .iter()
        .find(|e| e.source_id == new_bag.id)
        .unwrap();
    assert_eq!(edge.target_id, new_person.id);
    assert_eq!(&edge.properties, props);
    assert!(!pasted.edges.iter().any(|e| e.source_id == new_person.id));
    let twice = preview_world_edit(&pasted, edit).unwrap().world;
    assert_eq!(twice.nodes.len(), w.nodes.len() + 6);
    twice.validate().unwrap();
}

#[test]
fn invalid_paste_and_stale_selection_never_partially_mutate_the_world() {
    let (w, _, person, _, _) = fixture();
    let original = w.clone();
    let n = w.nodes.iter().find(|n| n.id == person).unwrap().clone();
    for edit in [
        WorldEdit::PasteNodes {
            nodes: vec![],
            edges: vec![],
        },
        WorldEdit::PasteNodes {
            nodes: vec![n.clone(), n.clone()],
            edges: vec![],
        },
        WorldEdit::PasteNodes {
            nodes: vec![n.clone()],
            edges: w.edges.clone(),
        },
        WorldEdit::RemoveSelection {
            node_ids: vec![],
            edge_ids: vec![],
        },
        WorldEdit::RemoveSelection {
            node_ids: vec![person.clone(), person.clone()],
            edge_ids: vec![],
        },
        WorldEdit::RemoveSelection {
            node_ids: vec![person.clone(), id()],
            edge_ids: vec![],
        },
        WorldEdit::RemoveSelection {
            node_ids: vec![person],
            edge_ids: vec![id()],
        },
    ] {
        assert!(preview_world_edit(&w, edit).is_err());
        assert_eq!(w, original);
    }
    let mut unnamed = n.clone();
    unnamed.name.clear();
    assert!(preview_world_edit(
        &w,
        WorldEdit::PasteNodes {
            nodes: vec![unnamed],
            edges: vec![]
        }
    )
    .is_err());
    let mut invalid = n;
    invalid
        .components
        .insert("custom.unknown".into(), BTreeMap::new());
    assert!(preview_world_edit(
        &w,
        WorldEdit::PasteNodes {
            nodes: vec![invalid],
            edges: vec![]
        }
    )
    .is_err());
    assert_eq!(w, original);
}

#[test]
fn batch_delete_and_unlink_publish_once_and_preserve_original_snapshot() {
    let (w, _, person, bag, coat) = fixture();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("map-actions");
    let project = Project::create(&path, "Map actions".into(), w.clone(), id()).unwrap();
    let old = project.view().unwrap();
    let pasted = preview_world_edit(
        &w,
        WorldEdit::PasteNodes {
            nodes: w.nodes.clone(),
            edges: w.edges.clone(),
        },
    )
    .unwrap()
    .world;
    let request = id();
    project
        .commit(0, request.clone(), "Map actions".into(), pasted.clone())
        .unwrap();
    assert!(
        project
            .commit(0, request, "Map actions".into(), pasted.clone())
            .unwrap()
            .replayed
    );
    let edge = pasted.edges.iter().find(|e| e.source_id == coat).unwrap();
    let next = preview_world_edit(
        &pasted,
        WorldEdit::RemoveSelection {
            node_ids: vec![person.clone(), bag.clone()],
            edge_ids: vec![edge.id.clone()],
        },
    )
    .unwrap()
    .world;
    assert!(next.nodes.iter().any(|n| n.id == coat));
    assert!(!next.edges.iter().any(|e| e.source_id == coat));
    assert!(!next.nodes.iter().any(|n| n.id == person || n.id == bag));
    project
        .commit(1, id(), "Map actions".into(), next.clone())
        .unwrap();
    drop(project);
    assert_eq!(Project::open(&path).unwrap().view().unwrap().world, next);
    let bytes = std::fs::read(
        path.join("worlds")
            .join(format!("{}.json", old.world_ref.id)),
    )
    .unwrap();
    assert_eq!(serde_json::from_slice::<World>(&bytes).unwrap(), w);
}
