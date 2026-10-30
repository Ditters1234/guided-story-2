//! Explicit local-provider check, using only a fresh fictional project.
use super::*;
use std::{path::Path, time::Instant};

#[test]
#[ignore = "explicitly save a compatibility profile after reviewing a real fictional probe"]
fn apply_verified_local_profile() {
    if std::env::var("STORY_CREATION_PROFILE_APPLY").as_deref() != Ok("json-schema-direct-off") {
        return;
    }
    let config = std::env::var("STORY_CREATION_QUALITY_CONFIG").unwrap();
    let proof = std::env::var("STORY_CREATION_PROFILE_PROOF").unwrap();
    let root = Path::new(&proof);
    let profile: Value =
        serde_json::from_slice(&fs::read(root.join("profile.json")).unwrap()).unwrap();
    assert_eq!(profile["structuredOutput"], "schema");
    assert_eq!(profile["thinking"], "directOff");
    let connection = load_connection(Path::new(&config)).unwrap().settings;
    let endpoint = url::Url::parse(&connection.base_url).unwrap();
    assert!(match endpoint.host() {
        Some(url::Host::Domain(s)) => s.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    });
    let p = Project::open(&root.join("project")).unwrap();
    let w = p.view().unwrap().world;
    assert_eq!(
        w.nodes
            .iter()
            .filter(|n| n.components.contains_key("core.character"))
            .count(),
        2
    );
    assert!(
        w.nodes
            .iter()
            .filter(|n| n.components.contains_key("core.place"))
            .count()
            >= 4
    );
    assert!(!resolve_ai_context(&w, TextPromptTask::Passage)
        .unwrap()
        .is_empty());
    for person in w
        .nodes
        .iter()
        .filter(|n| n.components.contains_key("core.character"))
    {
        let garments: Vec<_> = w
            .edges
            .iter()
            .filter(|e| e.type_id == "core.worn-by" && e.target_id == person.id)
            .filter_map(|e| w.nodes.iter().find(|n| n.id == e.source_id))
            .collect();
        assert!(garments.len() >= 2);
        let names = garments
            .iter()
            .map(|n| n.name.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            names.contains("trouser")
                || names.contains("pants")
                || names.contains("dress")
                || names.contains("skirt")
        );
        assert!(names.contains("boot") || names.contains("shoe"));
    }
    assert!(p
        .authoring_list()
        .unwrap()
        .iter()
        .any(|r| r.status == AuthoringStatus::Applied && r.provider_label == connection.model));
    let key_before = connection_api_key(Path::new(&config), &connection).unwrap();
    let backup = root.join("connection-before-profile.private.json");
    assert!(!backup.exists());
    fs::write(&backup, serde_json::to_vec_pretty(&connection).unwrap()).unwrap();
    let mut next = connection.clone();
    next.structured_output = StructuredOutput::Schema;
    next.thinking = ThinkingControl::DirectOff;
    let after = save_connection(
        Path::new(&config),
        connection.version,
        next.clone(),
        ApiKeyUpdate::Keep,
    )
    .unwrap();
    next.version = after.settings.version;
    assert!(after.settings == next);
    assert!(connection_api_key(Path::new(&config), &after.settings).unwrap() == key_before);
    let receipt = json!({"structuredOutput":after.settings.structured_output,"thinking":after.settings.thinking,"version":after.settings.version,"preservedKey":true,"preservedPrompts":true,"preservedEndpointModelSamplingBudgets":true});
    fs::write(
        root.join("profile-save-receipt.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .unwrap();
    println!("{receipt}");
}

#[tokio::test]
#[ignore = "explicit bounded local-provider transport check"]
async fn local_provider_transport_check() {
    let Ok(config) = std::env::var("STORY_CREATION_QUALITY_CONFIG") else {
        return;
    };
    let output =
        std::env::var("STORY_CREATION_QUALITY_ROOT").expect("Choose a new isolated folder");
    let root = Path::new(&output);
    fs::create_dir(root).unwrap();
    let mut connection = load_connection(Path::new(&config)).unwrap().settings;
    let endpoint = url::Url::parse(&connection.base_url).unwrap();
    assert!(match endpoint.host() {
        Some(url::Host::Domain(s)) => s.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    });
    connection.timeout_seconds = 30;
    let key = connection_api_key(Path::new(&config), &connection).unwrap();
    let started = Instant::now();
    let models = discover_models(&connection, key.as_deref()).await;
    fs::write(root.join("catalogue.json"),json!({"seconds":started.elapsed().as_secs_f64(),"models":models.as_ref().ok(),"error":models.as_ref().err()}).to_string()).unwrap();
    assert!(models.unwrap().contains(&connection.model));
    let started = Instant::now();
    let result=complete_text(&connection,key.as_deref(),"Return a JSON object only.","Return {\"ok\":true}.",Some(&json!({"type":"object","properties":{"ok":{"type":"boolean"}},"required":["ok"],"additionalProperties":false})),32).await;
    fs::write(root.join("ping.json"),json!({"seconds":started.elapsed().as_secs_f64(),"text":result.as_ref().ok().map(|r|&r.text),"incomplete":result.as_ref().ok().map(|r|r.incomplete),"error":result.as_ref().err()}).to_string()).unwrap();
    let result = result.unwrap();
    println!(
        "{}",
        json!({"seconds":started.elapsed().as_secs_f64(),"text":result.text,"incomplete":result.incomplete})
    );
    assert_eq!(
        serde_json::from_str::<Value>(&result.text).unwrap(),
        json!({"ok":true})
    );
}

#[tokio::test]
#[ignore = "explicit two-request fictional local-model quality check"]
async fn developed_world_and_group_quality() {
    let Ok(config) = std::env::var("STORY_CREATION_QUALITY_CONFIG") else {
        return;
    };
    let output =
        std::env::var("STORY_CREATION_QUALITY_ROOT").expect("Choose a new isolated folder");
    let root = Path::new(&output);
    fs::create_dir(root).unwrap();
    let mut connection = load_connection(Path::new(&config)).unwrap().settings;
    assert_eq!(connection.provider, TextProvider::Compatible);
    let endpoint = url::Url::parse(&connection.base_url).unwrap();
    assert!(
        match endpoint.host() {
            Some(url::Host::Domain(s)) => s.eq_ignore_ascii_case("localhost"),
            Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
            _ => false,
        },
        "Only the configured local provider is permitted"
    );
    connection.max_tokens = connection.max_tokens.min(4096);
    connection.timeout_seconds = connection.timeout_seconds.min(300);
    if std::env::var_os("STORY_CREATION_QUALITY_DIRECT_OFF").is_some() {
        connection.thinking = ThinkingControl::DirectOff;
    }
    if std::env::var_os("STORY_CREATION_QUALITY_SCHEMA").is_some() {
        connection.structured_output = StructuredOutput::Schema;
    }
    let key = connection_api_key(Path::new(&config), &connection).unwrap();
    let p = Project::create(
        &root.join("project"),
        "Fictional developed world quality check".into(),
        World::default(),
        id(),
    )
    .unwrap();
    let mut request=AuthoringRequest{creation_scope:Some(CreationScope::World),include_narration:Some(true),direction:"An older man and a younger adult woman share an isolated forested homestead with a house, a barn and cultivated fields. It is late autumn, the water pump is unreliable, and they disagree about leaving before winter. Develop a usable starting world, giving both adults their own reasons, ways of speaking and practical clothing.".into(),..group_request()};
    if let Ok(direction) = std::env::var("STORY_CREATION_QUALITY_DIRECTION") {
        request.direction = direction;
    }
    request.creation_type_ids = creation::foundation_types(&p.view().unwrap().world);
    let world_id = id();
    fs::write(root.join("profile.json"),json!({"thinking":connection.thinking,"structuredOutput":connection.structured_output,"maxTokens":connection.max_tokens,"timeoutSeconds":connection.timeout_seconds}).to_string()).unwrap();
    p.save_authoring_draft(&world_id, 0, request).unwrap();
    let prepared = p.start_authoring(&world_id, 1, 0, &connection).unwrap();
    fs::write(root.join("world-request.json"),json!({"system":prepared.system,"user":serde_json::from_str::<Value>(&prepared.user).unwrap(),"estimatedInputTokens":prepared.estimated_input_tokens,"outputTokens":connection.max_tokens}).to_string()).unwrap();
    let started = Instant::now();
    let result = complete_text(
        &connection,
        key.as_deref(),
        &prepared.system,
        &prepared.user,
        Some(&prepared.response_schema),
        connection.max_tokens,
    )
    .await;
    let response = match result {
        Ok(r) => r,
        Err(error) => {
            fs::write(root.join("world-provider-error.txt"), &error).unwrap();
            p.finish_authoring(&world_id, Err(error.clone()), false, false)
                .unwrap();
            panic!("Local world request failed; no retries are performed: {error}");
        }
    };
    fs::write(root.join("world-response.json"), &response.text).unwrap();
    p.finish_authoring(&world_id, Ok(response.text), false, response.incomplete)
        .unwrap();
    let review = p.authoring_view(&world_id, &connection).unwrap();
    fs::write(root.join("world-review.json"),json!({"error":review.error,"warnings":review.warnings,"seconds":started.elapsed().as_secs_f64()}).to_string()).unwrap();
    assert!(
        review.error.is_empty(),
        "World response retained for review: {}",
        review.error
    );
    let world = p
        .apply_authoring(&world_id, review.version, &connection)
        .unwrap()
        .commit
        .project
        .world;
    let places: Vec<_> = world
        .nodes
        .iter()
        .filter(|n| n.components.contains_key("core.place"))
        .collect();
    let people: Vec<_> = world
        .nodes
        .iter()
        .filter(|n| n.components.contains_key("core.character"))
        .collect();
    let detail = |n: &&WorldNode| json!({"name":n.name,"description":n.description,"visualDescription":n.visual_description,"components":n.components});
    let world_summary = json!({"calls":1,"seconds":started.elapsed().as_secs_f64(),"nodes":world.nodes.len(),"places":places.iter().map(detail).collect::<Vec<_>>(),"people":people.iter().map(detail).collect::<Vec<_>>(),"links":world.edges,"narration":world.nodes.iter().filter(|n|n.components.contains_key("core.narration-style")).collect::<Vec<_>>(),"aiContext":world.ai_context});
    fs::write(
        root.join("world-evaluation.json"),
        serde_json::to_vec_pretty(&world_summary).unwrap(),
    )
    .unwrap();
    assert!(
        places.len() >= 4,
        "Expected separate site, house, barn and field"
    );
    assert_eq!(people.len(), 2);
    for person in &people {
        let worn = world
            .edges
            .iter()
            .filter(|e| e.type_id == "core.worn-by" && e.target_id == person.id)
            .count();
        assert!(
            worn >= 2,
            "{} needs a usable item-based outfit, got {worn} worn items",
            person.name
        );
        let garments: Vec<_> = world
            .edges
            .iter()
            .filter(|e| e.type_id == "core.worn-by" && e.target_id == person.id)
            .filter_map(|e| world.nodes.iter().find(|n| n.id == e.source_id))
            .collect();
        let names = garments
            .iter()
            .map(|n| n.name.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");
        let regions = garments
            .iter()
            .filter_map(|n| n.components.get("core.wearable"))
            .filter_map(|v| v.get("regions"))
            .filter_map(|v| match v {
                PropertyValue::Text(t) => Some(t.to_lowercase()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            regions.contains("torso") || regions.contains("upper") || regions.contains("chest"),
            "{} lacks upper-body coverage: {names}",
            person.name
        );
        assert!(
            names.contains("trouser")
                || names.contains("pants")
                || names.contains("dress")
                || names.contains("skirt")
                || regions.contains("hips")
                || regions.contains("waist"),
            "{} lacks lower-body clothing: {names}",
            person.name
        );
        assert!(
            names.contains("boot") || names.contains("shoe") || regions.contains("feet"),
            "{} lacks footwear: {names}",
            person.name
        );
        let capture = p
            .create_media(
                &id(),
                MediaTarget {
                    guidance_source: None,
                    node_id: Some(person.id.clone()),
                    passage_id: None,
                    moment: ImageMoment::After,
                    subject_ids: vec![],
                },
            )
            .unwrap();
        assert_eq!(capture.draft.source.facts[0].outfit.len(), worn);
    }
    assert!(world
        .ai_context
        .iter()
        .any(|b| b.tasks.contains(&TextPromptTask::Passage)));
    assert!(
        world
            .edges
            .iter()
            .filter(|e| e.type_id == "core.inside")
            .count()
            >= 3
    );
    if std::env::var_os("STORY_CREATION_QUALITY_WORLD_ONLY").is_some() {
        println!(
            "{}",
            json!({"calls":1,"nodes":world.nodes.len(),"places":places.len(),"characters":people.len(),"evaluationFolder":output})
        );
        return;
    }
    let barn = places
        .iter()
        .find(|n| n.name.to_lowercase().contains("barn"))
        .expect("Expected a separate barn node");
    let group_id = id();
    let group=AuthoringRequest{creation_scope:Some(CreationScope::Group),include_narration:None,group_type_id:Some("core.item".into()),context_ids:vec![barn.id.clone()],direction:format!("Create three useful items for repairing the unreliable water pump at {}. Avoid equipment already present. Give each a concrete function, practical limitation, recognisable design/material and condition. Store them in the existing barn or a supplied container. Do not add people or places.",barn.name),..group_request()};
    connection.max_tokens = connection.max_tokens.min(2048);
    p.save_authoring_draft(&group_id, 0, group).unwrap();
    let prepared = p
        .start_authoring(&group_id, 1, p.view().unwrap().revision, &connection)
        .unwrap();
    fs::write(root.join("group-request.json"),json!({"system":prepared.system,"user":serde_json::from_str::<Value>(&prepared.user).unwrap(),"estimatedInputTokens":prepared.estimated_input_tokens,"outputTokens":connection.max_tokens}).to_string()).unwrap();
    let started = Instant::now();
    let response = complete_text(
        &connection,
        key.as_deref(),
        &prepared.system,
        &prepared.user,
        Some(&prepared.response_schema),
        connection.max_tokens,
    )
    .await
    .expect("Local group request failed; no retries are performed");
    fs::write(root.join("group-response.json"), &response.text).unwrap();
    p.finish_authoring(&group_id, Ok(response.text), false, response.incomplete)
        .unwrap();
    let review = p.authoring_view(&group_id, &connection).unwrap();
    fs::write(root.join("group-review.json"),json!({"error":review.error,"warnings":review.warnings,"seconds":started.elapsed().as_secs_f64()}).to_string()).unwrap();
    assert!(
        review.error.is_empty(),
        "Group response retained for review: {}",
        review.error
    );
    let candidate: NodeCandidate = serde_json::from_str(&review.candidate_text).unwrap();
    assert_eq!(candidate.nodes.len(), 3);
    assert!(candidate.nodes.iter().all(|n| n.type_id == "core.item"));
    let before = world.clone();
    let after = p
        .apply_authoring(&group_id, review.version, &connection)
        .unwrap()
        .commit
        .project
        .world;
    for old in before.nodes {
        assert!(after.nodes.contains(&old));
    }
    for old in before.edges {
        assert!(after.edges.contains(&old));
    }
    println!(
        "{}",
        json!({"calls":2,"worldNodes":world.nodes.len(),"places":places.len(),"characters":people.len(),"groupItems":candidate.nodes.len(),"groupSeconds":started.elapsed().as_secs_f64(),"evaluationFolder":output})
    );
}

#[tokio::test]
#[ignore = "explicit local-model quality probe; never part of ordinary tests"]
async fn named_people_and_item_outfits_from_a_homestead_idea() {
    let Ok(config) = std::env::var("STORY_AUTHORING_QUALITY_CONFIG") else {
        return;
    };
    let output = std::env::var("STORY_AUTHORING_QUALITY_ROOT")
        .expect("Choose a new isolated output folder.");
    let root = Path::new(&output);
    fs::create_dir(root).expect("Use a new output folder; previous probes are never replaced.");
    let mut connection = load_connection(Path::new(&config)).unwrap().settings;
    assert_eq!(connection.provider, TextProvider::Compatible);
    let endpoint = url::Url::parse(&connection.base_url).unwrap();
    assert!(
        match endpoint.host() {
            Some(url::Host::Domain(host)) => host.eq_ignore_ascii_case("localhost"),
            Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
            None => false,
        },
        "This probe only permits the configured loopback provider."
    );
    connection.max_tokens = connection.max_tokens.min(4096);
    connection.timeout_seconds = connection.timeout_seconds.min(180);
    let key = connection_api_key(Path::new(&config), &connection).unwrap();
    let p = Project::create(
        &root.join("project"),
        "Fictional homestead quality probe".into(),
        World::default(),
        id(),
    )
    .unwrap();
    let request = AuthoringRequest {
        link_target_ids: vec![], link_fields: vec![], direction: "An older man and a younger woman at an isolated forested homestead. Develop this into the starting world for a story.".into(), ..group_request() };
    let proposal = id();
    p.save_authoring_draft(&proposal, 0, request).unwrap();
    let prepared = p.start_authoring(&proposal, 1, 0, &connection).unwrap();
    fs::write(root.join("request.json"), json!({"system":prepared.system,"user":serde_json::from_str::<Value>(&prepared.user).unwrap(),"estimatedInputTokens":prepared.estimated_input_tokens,"outputTokens":connection.max_tokens}).to_string()).unwrap();
    let started = Instant::now();
    let response = complete_text(
        &connection,
        key.as_deref(),
        &prepared.system,
        &prepared.user,
        Some(&prepared.response_schema),
        connection.max_tokens,
    )
    .await;
    match response {
        Ok(response) => {
            fs::write(root.join("response.json"), &response.text).unwrap();
            p.finish_authoring(&proposal, Ok(response.text), false, response.incomplete)
                .unwrap();
        }
        Err(error) => {
            fs::write(root.join("provider-error.txt"), &error).unwrap();
            p.finish_authoring(&proposal, Err(error.clone()), false, false)
                .unwrap();
            panic!("Local quality probe failed: {error}");
        }
    }
    let view = p.authoring_view(&proposal, &connection).unwrap();
    fs::write(root.join("review.json"), json!({"error":view.error,"warnings":view.warnings,"elapsedSeconds":started.elapsed().as_secs_f64()}).to_string()).unwrap();
    assert!(
        view.error.is_empty(),
        "Retained local response needs review: {}",
        view.error
    );
    let world = p
        .apply_authoring(&proposal, view.version, &connection)
        .unwrap()
        .commit
        .project
        .world;
    let characters: Vec<_> = world
        .nodes
        .iter()
        .filter(|n| n.components.contains_key("core.character"))
        .collect();
    let mut people = vec![];
    for person in &characters {
        let worn: Vec<_> = world
            .edges
            .iter()
            .filter(|e| e.type_id == "core.worn-by" && e.target_id == person.id)
            .filter_map(|e| world.nodes.iter().find(|n| n.id == e.source_id))
            .collect();
        let image = p
            .create_media(
                &id(),
                MediaTarget {
                    guidance_source: None,
                    node_id: Some(person.id.clone()),
                    passage_id: None,
                    moment: ImageMoment::After,
                    subject_ids: vec![],
                },
            )
            .unwrap();
        people.push(json!({"name":person.name,"storyNotes":person.description,"appearance":person.visual_description,"outfit":worn.iter().map(|n|json!({"name":n.name,"appearance":n.visual_description})).collect::<Vec<_>>(),"imageOutfitCount":image.draft.source.facts[0].outfit.len()}));
    }
    let summary = json!({"calls":1,"elapsedSeconds":started.elapsed().as_secs_f64(),"nodeCount":world.nodes.len(),"people":people,"warnings":view.warnings});
    fs::write(
        root.join("summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string(&summary).unwrap());
    assert_eq!(
        characters.len(),
        2,
        "The starting premise calls for two people."
    );
    for person in &characters {
        assert!(
            ![
                "older man",
                "elder man",
                "old man",
                "younger woman",
                "young woman",
                "man",
                "woman"
            ]
            .contains(&person.name.to_lowercase().as_str()),
            "Role label used as name: {}",
            person.name
        );
        assert!(person.description.len() > 30 && person.visual_description.len() > 20);
        let worn = world
            .edges
            .iter()
            .filter(|e| e.type_id == "core.worn-by" && e.target_id == person.id)
            .count();
        assert!(worn > 0, "No worn item nodes for {}", person.name);
        assert_eq!(
            people.iter().find(|p| p["name"] == person.name).unwrap()["imageOutfitCount"],
            worn
        );
    }
}
