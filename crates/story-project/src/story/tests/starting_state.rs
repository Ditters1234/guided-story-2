use super::*;

fn homestead() -> (tempfile::TempDir, Project, PassagePlan) {
    let dir = tempfile::tempdir().unwrap();
    let mut w = World::default();
    let mut make = |kind: &str, id: &str, name: &str| {
        let mut n = node(&World::default(), kind, name);
        n.id = id.into();
        w.nodes.push(n);
    };
    for (id, name) in [
        ("homestead", "Thorne Homestead"),
        ("house", "Main House"),
        ("yard", "Yard"),
        ("barn", "Barn"),
        ("remote", "Unselected Mill"),
    ] {
        make("core.place", id, name);
    }
    make("core.character", "silas", "Silas Thorne");
    make("core.character", "elara", "Elara Vane");
    for (kind, source, target) in [
        ("core.inside", "house", "homestead"),
        ("core.inside", "yard", "homestead"),
        ("core.inside", "barn", "homestead"),
        ("core.present-at", "silas", "house"),
        ("core.present-at", "elara", "yard"),
    ] {
        w.edges.push(WorldEdge {
            id: format!("{source}-placement"),
            type_id: kind.into(),
            source_id: source.into(),
            target_id: target.into(),
            description: String::new(),
            properties: BTreeMap::new(),
        });
    }
    // IDs need the same native UUID contract as actual worlds.
    let mapping: BTreeMap<_, _> = w.nodes.iter().map(|n| (n.id.clone(), id())).collect();
    for n in &mut w.nodes {
        n.id = mapping[&n.id].clone();
    }
    for e in &mut w.edges {
        e.id = id();
        e.source_id = mapping[&e.source_id].clone();
        e.target_id = mapping[&e.target_id].clone();
    }
    let p = Project::create(
        &dir.path().join("story"),
        "Fictional separated cast".into(),
        w,
        id(),
    )
    .unwrap();
    let plan = PassagePlan {
        viewpoint_id: Some(mapping["silas"].clone()),
        participant_ids: vec![mapping["elara"].clone()],
        location_id: Some(mapping["barn"].clone()),
        target_words: 140,
        ..p.next_passage_plan().unwrap()
    };
    (dir, p, plan)
}
fn person<'a>(state: &'a Value, name: &str) -> &'a Value {
    state["people"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == name)
        .unwrap()
}

#[test]
fn separated_people_keep_named_actual_starts_in_narration_and_extraction() {
    let (_dir, p, plan) = homestead();
    let before = p.view().unwrap();
    let key = id();
    let mut edit = edits(&p, "");
    edit.plan = plan;
    p.save_passage(&key, 0, edit).unwrap();
    let preview = p.passage_view(&key, &settings()).unwrap().scene_context;
    assert!(preview.error.is_empty());
    assert!(preview
        .resources
        .iter()
        .any(|record| record.name == "Yard" && record.role == SceneResourceRole::ContainingPlace));
    let generated = p.start_passage(&key, 1, 0, false, &settings()).unwrap();
    let request: Value = serde_json::from_str(&generated.user).unwrap();
    assert_eq!(
        preview
            .resources
            .iter()
            .map(|record| (&record.node_id, json!(record.role)))
            .collect::<BTreeMap<_, _>>(),
        request["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|record| {
                let id = preview
                    .resources
                    .iter()
                    .find(|entry| Some(entry.node_id.as_str()) == record["id"].as_str())
                    .unwrap();
                (&id.node_id, record["sceneRole"].clone())
            })
            .collect()
    );
    let state = &request["startingState"];
    assert!(request["plan"].get("locationId").is_none());
    assert_eq!(
        request["plan"]["intendedLocationId"],
        state["intendedSceneFocus"]["id"]
    );
    assert_eq!(state["openingViewpoint"]["place"]["name"], "Main House");
    assert!(state["openingFacts"]
        .as_array()
        .unwrap()
        .contains(&json!("Elara Vane starts in Yard.")));
    assert_eq!(state["intendedSceneFocus"]["name"], "Barn");
    for (who, place) in [("Silas Thorne", "Main House"), ("Elara Vane", "Yard")] {
        assert_eq!(person(state, who)["currentPlace"]["name"], place);
        assert_eq!(person(state, who)["relativeToFocus"], "elsewhere");
    }
    assert!(!generated.user.contains("Unselected Mill"));
    let elara = request["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["name"] == "Elara Vane")
        .unwrap();
    assert_eq!(elara["startingPosition"]["currentPlace"]["name"], "Yard");
    assert_eq!(state["physicalLinks"].as_array().unwrap().len(), 4);
    assert_eq!(p.view().unwrap().world, before.world);
    let prose="Silas left the Main House, crossed the Yard and entered the Barn. Elara followed him from the Yard into the Barn.";
    p.finish_passage(&key, false, Ok(prose.into()), false, false, &settings())
        .unwrap();
    let view = p.passage_view(&key, &settings()).unwrap();
    let check = p
        .start_passage(&key, view.version, 0, true, &settings())
        .unwrap();
    let extracted: Value = serde_json::from_str(&check.user).unwrap();
    assert_eq!(extracted["startingState"], *state);
    assert_eq!(p.view().unwrap().world, before.world);
}

#[test]
fn generic_nested_placement_distinguishes_within_unknown_and_shared_parent() {
    let (_dir, p, mut plan) = homestead();
    let mut w = p.view().unwrap().world;
    let house = w
        .nodes
        .iter()
        .find(|n| n.name == "Main House")
        .unwrap()
        .id
        .clone();
    let barn = plan.location_id.clone().unwrap();
    let mut room = node(&w, "core.place", "Barn loft");
    let room_id = room.id.clone();
    room.description = "A room within the Barn.".into();
    w.nodes.push(room);
    let mut relation = w
        .relation_types
        .iter()
        .find(|r| r.id == "core.present-at")
        .unwrap()
        .clone();
    relation.id = "custom.stays-at".into();
    relation.label = "Stays at".into();
    w.relation_types.push(relation);
    let edge = w
        .edges
        .iter_mut()
        .find(|e| Some(&e.source_id) == plan.viewpoint_id.as_ref())
        .unwrap();
    edge.type_id = "custom.stays-at".into();
    edge.target_id = room_id.clone();
    w.edges.push(WorldEdge {
        id: id(),
        type_id: "core.inside".into(),
        source_id: room_id,
        target_id: barn,
        description: String::new(),
        properties: BTreeMap::new(),
    });
    w.validate().unwrap();
    let state =
        super::super::starting_state::starting_state(&scene_world(&w, &plan).unwrap(), &plan);
    assert_eq!(
        person(&state, "Silas Thorne")["currentPlace"]["name"],
        "Barn loft"
    );
    assert_eq!(
        person(&state, "Silas Thorne")["relativeToFocus"],
        "withinFocusArea"
    );
    assert_eq!(person(&state, "Elara Vane")["relativeToFocus"], "elsewhere");
    assert!(state["physicalLinks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|l| l["relationship"] == "Stays at"));
    w.edges
        .retain(|e| Some(&e.source_id) != plan.viewpoint_id.as_ref());
    let state =
        super::super::starting_state::starting_state(&scene_world(&w, &plan).unwrap(), &plan);
    assert!(person(&state, "Silas Thorne")["currentPlace"].is_null());
    assert_eq!(person(&state, "Silas Thorne")["relativeToFocus"], "unknown");
    plan.location_id = Some(house);
    let state =
        super::super::starting_state::starting_state(&scene_world(&w, &plan).unwrap(), &plan);
    assert_eq!(person(&state, "Elara Vane")["relativeToFocus"], "elsewhere");
}

#[test]
fn starting_summary_excludes_claim_names() {
    let (_dir, p, plan) = homestead();
    let mut w = p.view().unwrap().world;
    let mut hidden = node(&w, "core.place", "Hidden claim placement");
    hidden.components.insert(
        "core.claim".into(),
        default_properties(
            w.component_types
                .iter()
                .find(|d| d.id == "core.claim")
                .unwrap(),
        ),
    );
    let target = hidden.id.clone();
    w.nodes.push(hidden);
    w.edges
        .iter_mut()
        .find(|e| Some(&e.source_id) == plan.viewpoint_id.as_ref())
        .unwrap()
        .target_id = target;
    let summary = super::super::starting_state::starting_state(&w, &plan);
    assert!(!summary.to_string().contains("Hidden claim placement"));
    assert_eq!(
        person(&summary, "Silas Thorne")["relativeToFocus"],
        "unknown"
    );
}

#[test]
fn shared_deep_placement_paths_are_sent_once() {
    let (_dir, p, plan) = homestead();
    let mut w = p.view().unwrap().world;
    let house = w
        .nodes
        .iter()
        .find(|n| n.name == "Main House")
        .unwrap()
        .id
        .clone();
    let root = w
        .nodes
        .iter()
        .find(|n| n.name == "Thorne Homestead")
        .unwrap()
        .id
        .clone();
    w.edges.retain(|e| e.source_id != house);
    w.edges
        .iter_mut()
        .find(|e| Some(&e.source_id) == plan.participant_ids.first())
        .unwrap()
        .target_id = house.clone();
    let mut previous = house;
    for index in 0..12 {
        let n = node(&w, "core.place", &format!("Level {index}"));
        let next = n.id.clone();
        w.nodes.push(n);
        w.edges.push(WorldEdge {
            id: id(),
            type_id: "core.inside".into(),
            source_id: previous,
            target_id: next.clone(),
            description: String::new(),
            properties: BTreeMap::new(),
        });
        previous = next;
    }
    w.edges.push(WorldEdge {
        id: id(),
        type_id: "core.inside".into(),
        source_id: previous,
        target_id: root,
        description: String::new(),
        properties: BTreeMap::new(),
    });
    let summary =
        super::super::starting_state::starting_state(&scene_world(&w, &plan).unwrap(), &plan);
    assert_eq!(summary["physicalLinks"].as_array().unwrap().len(), 15);
    assert!(summary["people"]
        .as_array()
        .unwrap()
        .iter()
        .all(|p| p["relativeToFocus"] == "elsewhere"));
}

#[test]
fn empty_cast_named_by_guidance_blocks_before_job_start_and_explicit_references_supply_facts() {
    let (_dir, p, mut plan) = homestead();
    let original = p.view().unwrap();
    let mut w = original.world;
    let silas = plan.viewpoint_id.clone().unwrap();
    let elara = plan.participant_ids[0].clone();
    crate::ai_context::tests::note(
        &mut w,
        "Narration style",
        "Keep Silas's restrained manner and Elara’s quiet observations.",
        AiContextRole::Instructions,
        vec![TextPromptTask::Passage],
    );
    let jacket = node(&w, "core.wearable-item", "Canvas jacket");
    let jacket_id = jacket.id.clone();
    w.nodes.push(jacket);
    w.edges.push(WorldEdge {
        id: id(),
        type_id: "core.worn-by".into(),
        source_id: jacket_id,
        target_id: silas.clone(),
        description: String::new(),
        properties: BTreeMap::new(),
    });
    p.commit(0, id(), original.title, w).unwrap();
    let before = p.view().unwrap();
    plan.viewpoint_id = None;
    plan.participant_ids.clear();
    plan.reference_ids.clear();
    let key = id();
    let mut edit = edits(&p, "");
    edit.plan = plan;
    p.save_passage(&key, 0, edit).unwrap();
    let preview = p.passage_view(&key, &settings()).unwrap().scene_context;
    let error = p
        .start_passage(&key, 1, before.revision, false, &settings())
        .err()
        .expect("unscoped cast should block narration");
    assert_eq!(preview.error, error);
    assert_eq!(preview.guidance.len(), 1);
    assert!(
        error.contains("Silas Thorne") && error.contains("Elara Vane"),
        "{error}"
    );
    assert_eq!(p.read_passage_draft(&key).unwrap().version, 1);
    assert_eq!(
        p.read_passage_draft(&key).unwrap().stage,
        PassageStage::Draft
    );
    assert_eq!(p.view().unwrap().world, before.world);
    let mut view = p.passage_view(&key, &settings()).unwrap();
    view.edits.plan.reference_ids = vec![silas, elara];
    p.save_passage(&key, view.version, view.edits).unwrap();
    let ready = p.passage_view(&key, &settings()).unwrap();
    let prepared = p
        .start_passage(&key, ready.version, before.revision, false, &settings())
        .unwrap();
    let input: Value = serde_json::from_str(&prepared.user).unwrap();
    assert_eq!(
        input["startingState"]["people"].as_array().unwrap().len(),
        2
    );
    assert_eq!(
        input["startingState"]["openingFacts"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(input["records"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["name"] == "Canvas jacket"));
    assert_eq!(
        person(&input["startingState"], "Silas Thorne")["selectedAs"],
        "reference"
    );
    assert_eq!(
        person(&input["startingState"], "Silas Thorne")["currentPlace"]["name"],
        "Main House"
    );
    assert_eq!(
        person(&input["startingState"], "Elara Vane")["currentPlace"]["name"],
        "Yard"
    );
    assert_eq!(p.view().unwrap().world, before.world);
}

#[test]
fn setting_only_narration_is_allowed_without_named_cast() {
    let (_dir, p, mut plan) = homestead();
    plan.viewpoint_id = None;
    plan.participant_ids.clear();
    let key = id();
    let mut edit = edits(&p, "");
    edit.plan = plan;
    p.save_passage(&key, 0, edit).unwrap();
    let prepared = p.start_passage(&key, 1, 0, false, &settings()).unwrap();
    let input: Value = serde_json::from_str(&prepared.user).unwrap();
    assert!(input["startingState"]["people"]
        .as_array()
        .unwrap()
        .is_empty());
    p.finish_passage(
        &key,
        false,
        Ok("Rain rattled on the empty Barn's roof.".into()),
        false,
        false,
        &settings(),
    )
    .unwrap();
    assert_eq!(p.view().unwrap().revision, 0);
}
