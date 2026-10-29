//! Explicit local-provider check, using only a fresh fictional project.
use super::*;
use std::{path::Path, time::Instant};

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
