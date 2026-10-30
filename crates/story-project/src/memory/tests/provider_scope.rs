use super::*;

fn item(world: &World, id: &str, name: &str) -> WorldNode {
    let preset = world
        .node_types
        .iter()
        .find(|p| p.id == "core.item")
        .unwrap();
    WorldNode {
        id: id.into(),
        type_id: preset.id.clone(),
        name: name.into(),
        description: String::new(),
        visual_description: String::new(),
        components: preset
            .components
            .iter()
            .map(|id| {
                (
                    id.clone(),
                    default_properties(world.component_types.iter().find(|d| &d.id == id).unwrap()),
                )
            })
            .collect(),
    }
}
fn accept_effects(p: &Project, prose: &str, effects: PassageEffects) -> String {
    let world = p.view().unwrap().world;
    let mut plan = p.next_passage_plan().unwrap();
    plan.location_id = Some(world.nodes[0].id.clone());
    plan.viewpoint_id = Some(world.nodes[1].id.clone());
    let key = id();
    p.save_passage(
        &key,
        0,
        PassageEdits {
            plan,
            prose: prose.into(),
            effects_text: serde_json::to_string(&effects).unwrap(),
            close_chapter: true,
            approved_effects: vec![],
        },
    )
    .unwrap();
    p.validate_passage(&key, 1, &settings()).unwrap();
    let approved = p.approve_passage_for_test(&key, &settings()).unwrap();
    p.accept_passage(&key, approved.version, &settings())
        .unwrap();
    key
}

#[test]
fn newly_introduced_effect_subjects_use_canonical_source_ids_and_survive_later_world_edits() {
    let (_dir, p) = fixture();
    let world = p.view().unwrap().world;
    let prose = "Mara lifted a brass lantern from the bench.";
    let introduced = item(&world, "new:lantern", "Brass lantern");
    let passage = accept_effects(
        &p,
        prose,
        PassageEffects {
            changes: vec![
                PassageEffect::PutNode {
                    node: introduced,
                    evidence: prose.into(),
                },
                PassageEffect::PutLink {
                    link: WorldEdge {
                        id: "new:lantern-carried".into(),
                        type_id: "core.stored-in".into(),
                        source_id: "new:lantern".into(),
                        target_id: world.nodes[1].id.clone(),
                        description: String::new(),
                        properties: BTreeMap::new(),
                    },
                    evidence: prose.into(),
                },
            ],
            notes: vec![],
            suggest_chapter_end: false,
        },
    );
    let mut current = p.view().unwrap();
    let canonical = current
        .world
        .nodes
        .iter()
        .find(|n| n.name == "Brass lantern")
        .unwrap()
        .id
        .clone();
    assert_ne!(canonical, "new:lantern");
    current
        .world
        .nodes
        .iter_mut()
        .find(|n| n.id == canonical)
        .unwrap()
        .name = "A name from later history".into();
    let future = id();
    current
        .world
        .nodes
        .push(item(&current.world, &future, "Future apparatus"));
    p.commit(current.revision, id(), current.title, current.world)
        .unwrap();
    let chapter = p.story_index().unwrap().chapters[0].id.clone();
    let draft = id();
    p.create_memory_draft(&draft, &chapter).unwrap();
    let prepared = p
        .start_memory(&draft, 1, p.view().unwrap().revision, &settings())
        .unwrap();
    let input: Value = serde_json::from_str(&prepared.user).unwrap();
    let nodes = input["nodes"].as_array().unwrap();
    assert!(nodes
        .iter()
        .any(|n| n["id"] == canonical && n["name"] == "Brass lantern"));
    assert!(!prepared.user.contains("A name from later history"));
    assert!(!prepared.user.contains("Future apparatus"));
    assert!(!prepared.user.contains("Private profile detail"));
    let point_schema = &prepared.schema["properties"]["points"]["items"]["properties"];
    assert_eq!(point_schema["passageId"]["enum"], json!([passage]));
    assert!(point_schema["nodeIds"]["items"]["enum"]
        .as_array()
        .unwrap()
        .contains(&json!(canonical)));
    assert!(!point_schema["nodeIds"]["items"]["enum"]
        .as_array()
        .unwrap()
        .contains(&json!(future)));
    p.finish_memory(
        &draft,
        Ok(serde_json::to_string(&MemoryOutput {
            points: vec![MemoryPoint {
                text: "Mara took the brass lantern.".into(),
                passage_id: passage,
                evidence: prose.into(),
                node_ids: vec![canonical],
            }],
            no_lasting_facts: false,
        })
        .unwrap()),
        false,
        false,
    )
    .unwrap();
    assert!(p.memory_view(&draft).unwrap().can_apply);
}

#[test]
fn link_only_changes_and_detachments_include_their_unplanned_item_subjects() {
    for detach in [false, true] {
        let (_dir, p) = fixture();
        let mut current = p.view().unwrap();
        let carried = item(&current.world, &id(), "Travel lantern");
        let subject = carried.id.clone();
        let link = WorldEdge {
            id: id(),
            type_id: "core.stored-in".into(),
            source_id: subject.clone(),
            target_id: current.world.nodes[1].id.clone(),
            description: String::new(),
            properties: BTreeMap::new(),
        };
        current.world.nodes.push(carried);
        current.world.edges.push(link.clone());
        p.commit(current.revision, id(), current.title, current.world)
            .unwrap();
        let prose = if detach {
            "Mara detached the travel lantern from her belt."
        } else {
            "Mara set the travel lantern on the glasshouse floor."
        };
        let change = if detach {
            PassageEffect::RemoveLink {
                id: link.id,
                evidence: prose.into(),
            }
        } else {
            PassageEffect::PutLink {
                link: WorldEdge {
                    target_id: p.view().unwrap().world.nodes[0].id.clone(),
                    ..link
                },
                evidence: prose.into(),
            }
        };
        accept_effects(
            &p,
            prose,
            PassageEffects {
                changes: vec![change],
                notes: vec![],
                suggest_chapter_end: false,
            },
        );
        let chapter = p.story_index().unwrap().chapters[0].id.clone();
        let draft = id();
        p.create_memory_draft(&draft, &chapter).unwrap();
        assert!(p
            .memory_view(&draft)
            .unwrap()
            .nodes
            .iter()
            .any(|n| n.id == subject));
    }
}
