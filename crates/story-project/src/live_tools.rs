//! Explicit local-model acceptance probes. Isolated fictional data; no retries/settings writes.
use crate::*;
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::Path, time::Instant};

fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn node(w: &World, kind: &str, name: &str) -> WorldNode {
    let preset = w.node_types.iter().find(|t| t.id == kind).unwrap();
    WorldNode {
        id: id(),
        type_id: kind.into(),
        name: name.into(),
        description: String::new(),
        visual_description: String::new(),
        components: preset
            .components
            .iter()
            .map(|id| {
                let d = w.component_types.iter().find(|d| &d.id == id).unwrap();
                (
                    id.clone(),
                    d.fields
                        .iter()
                        .filter(|f| f.required || f.default_value != PropertyValue::Null)
                        .map(|f| (f.key.clone(), f.default_value.clone()))
                        .collect(),
                )
            })
            .collect(),
    }
}
fn text(n: &mut WorldNode, component: &str, key: &str, value: &str) {
    n.components
        .get_mut(component)
        .unwrap()
        .insert(key.into(), PropertyValue::Text(value.into()));
}
fn link(kind: &str, source: &str, target: &str) -> WorldEdge {
    WorldEdge {
        id: id(),
        type_id: kind.into(),
        source_id: source.into(),
        target_id: target.into(),
        description: String::new(),
        properties: BTreeMap::new(),
    }
}
struct Fixture {
    p: Project,
    room: String,
    nora: String,
    otis: String,
    key: String,
    style: String,
    custom: String,
}
fn fixture(root: &Path) -> Fixture {
    let mut w = World::default();
    let mut lodge = node(&w, "core.place", "Ridgeway Lodge");
    text(
        &mut lodge,
        "detail.place",
        "purpose",
        "Mountain repair station and shelter.",
    );
    let mut room = node(&w, "core.place", "Workshop");
    text(
        &mut room,
        "detail.place",
        "layout",
        "A scarred workbench beside a north window; a stove by the door.",
    );
    w.edges.push(link("core.inside", &room.id, &lodge.id));
    let mut nora = node(&w, "core.character", "Nora Hale");
    text(&mut nora, "detail.profile", "age", "42");
    text(&mut nora, "detail.profile", "occupation", "Repair mechanic");
    text(
        &mut nora,
        "detail.motivation",
        "wants",
        "Keep the lodge's generator working through the storm.",
    );
    text(
        &mut nora,
        "detail.voice",
        "style",
        "Dry, exact sentences; avoids promises she cannot keep.",
    );
    let mut otis = node(&w, "core.character", "Otis Reed");
    text(&mut otis, "detail.profile", "age", "67");
    text(&mut otis, "detail.profile", "occupation", "Lodge caretaker");
    text(
        &mut otis,
        "detail.voice",
        "style",
        "Patient questions, with understated jokes.",
    );
    w.edges.extend([
        link("core.present-at", &nora.id, &room.id),
        link("core.present-at", &otis.id, &room.id),
    ]);
    for person in [&nora, &otis] {
        for (name, regions) in [
            ("Wool shirt", "torso, arms"),
            ("Work trousers", "waist, hips, legs"),
            ("Leather boots", "feet"),
        ] {
            let mut g = node(
                &w,
                "core.wearable-item",
                &format!("{}'s {name}", person.name),
            );
            text(&mut g, "core.wearable", "regions", regions);
            text(&mut g, "detail.design", "colours", "Weathered brown");
            w.edges.push(link("core.worn-by", &g.id, &person.id));
            w.nodes.push(g);
        }
    }
    let key = node(&w, "core.item", "Brass key");
    w.edges.push(link("core.stored-in", &key.id, &nora.id));
    for name in ["Storm lantern", "Coil of rope"] {
        let n = node(&w, "core.item", name);
        w.edges.push(link("core.stored-in", &n.id, &room.id));
        w.nodes.push(n);
    }
    let mut style = node(&w, "core.narration-style", "Workshop narration");
    text(&mut style, "core.narration-style", "person", "Third person");
    text(&mut style, "core.narration-style", "tense", "Past tense");
    text(&mut style,"core.narration-style","instructions","Write close third person from Nora's viewpoint, in past tense throughout scene action. Use concrete repair details and restrained dialogue. Do not reveal Otis's private thoughts.");
    w.ai_context.push(AiContextBinding {
        node_id: style.id.clone(),
        enabled: true,
        role: AiContextRole::Instructions,
        tasks: vec![TextPromptTask::Passage, TextPromptTask::Wording],
        fields: vec![AiContextField::Components],
        order: 0,
        image_preparation: false,
    });
    w.component_types.push(ComponentDefinition {
        id: "custom.repair".into(),
        name: "Repair practice".into(),
        description: "A fictional technician's work habits.".into(),
        fields: [
            ("method", "Working method"),
            ("weakness", "Weakness under pressure"),
        ]
        .into_iter()
        .map(|(key, label)| FieldDefinition {
            key: key.into(),
            label: label.into(),
            kind: FieldKind::LongText,
            required: false,
            default_value: PropertyValue::Null,
            metadata: None,
        })
        .collect(),
    });
    w.node_types.push(NodeTypeDefinition {
        id: "custom.practice".into(),
        name: "Repair practice".into(),
        description: String::new(),
        components: vec!["custom.repair".into()],
    });
    let mut custom = node(&w, "custom.practice", "Nora's repair practice");
    custom.description =
        "Develop useful practical guidance for a cautious mechanic; keep this note unchanged."
            .into();
    let ids = (
        room.id.clone(),
        nora.id.clone(),
        otis.id.clone(),
        key.id.clone(),
        style.id.clone(),
        custom.id.clone(),
    );
    w.nodes
        .extend([lodge, room, nora, otis, key, style, custom]);
    let p = Project::create(
        &root.join("project"),
        "Fictional AI tools pass".into(),
        w,
        id(),
    )
    .unwrap();
    Fixture {
        p,
        room: ids.0,
        nora: ids.1,
        otis: ids.2,
        key: ids.3,
        style: ids.4,
        custom: ids.5,
    }
}
async fn call(
    root: &Path,
    name: &str,
    s: &LlmConnection,
    key: Option<&str>,
    system: &str,
    user: &str,
    schema: Option<&Value>,
    tokens: u32,
) -> Result<CompletionText, String> {
    println!("START {name}: output allowance {tokens}");
    fs::write(root.join(format!("{name}-request.json")),serde_json::to_vec_pretty(&json!({"system":system,"user":serde_json::from_str::<Value>(user).unwrap_or(json!(user)),"schema":schema,"outputTokens":tokens})).unwrap()).unwrap();
    let begin = Instant::now();
    let result = complete_text(s, key, system, user, schema, tokens).await;
    if let Ok(r) = &result {
        fs::write(root.join(format!("{name}-response.txt")), &r.text).unwrap();
    }
    let report = json!({"seconds":begin.elapsed().as_secs_f64(),"incomplete":result.as_ref().ok().map(|r|r.incomplete),"error":result.as_ref().err()});
    fs::write(
        root.join(format!("{name}-result.json")),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{name}: {report}");
    result
}
fn request() -> AuthoringRequest {
    AuthoringRequest {
        mode: AuthoringMode::Create,
        direction: String::new(),
        type_id: None,
        bundle: true,
        creation_scope: Some(CreationScope::Group),
        group_type_id: None,
        include_narration: None,
        creation_type_ids: vec![],
        target_ids: vec![],
        context_ids: vec![],
        fields: vec![],
        link_target_ids: vec![],
        link_fields: vec![],
    }
}
async fn author(
    root: &Path,
    f: &Fixture,
    s: &LlmConnection,
    key: Option<&str>,
    name: &str,
    request: AuthoringRequest,
) {
    let draft = id();
    f.p.save_authoring_draft(&draft, 0, request).unwrap();
    let prepared =
        f.p.start_authoring(&draft, 1, f.p.view().unwrap().revision, s)
            .unwrap();
    let result = call(
        root,
        name,
        s,
        key,
        &prepared.system,
        &prepared.user,
        Some(&prepared.response_schema),
        s.max_tokens,
    )
    .await;
    let incomplete = result.as_ref().is_ok_and(|r| r.incomplete);
    f.p.finish_authoring(&draft, result.map(|r| r.text), false, incomplete)
        .unwrap();
    let review = f.p.authoring_view(&draft, s).unwrap();
    fs::write(
        root.join(format!("{name}-review.json")),
        serde_json::to_vec_pretty(&review).unwrap(),
    )
    .unwrap();
    assert!(review.error.is_empty(), "{}", review.error);
    assert!(!review.stale);
    let before = f.p.view().unwrap().world;
    let applied = f.p.apply_authoring(&draft, review.version, s).unwrap();
    assert!(!applied.commit.replayed);
    assert_ne!(applied.commit.project.world, before);
}
fn edits(f: &Fixture, prose: &str) -> PassageEdits {
    let mut plan = f.p.next_passage_plan().unwrap();
    plan.location_id = Some(f.room.clone());
    plan.viewpoint_id = Some(f.nora.clone());
    plan.participant_ids = vec![f.otis.clone()];
    plan.target_words = 160;
    plan.title = "The workshop key".into();
    plan.direction="Nora hands the brass key she carries to Otis after a brief disagreement about the generator. Complete the handoff on the page. They stay in the workshop. No other possession changes, new characters or time jumps.".into();
    PassageEdits {
        plan,
        prose: prose.into(),
        effects_text: empty_effects(),
        close_chapter: false,
        approved_effects: vec![],
    }
}
fn manual_passage(f: &Fixture, s: &LlmConnection) -> String {
    let draft = id();
    let prose="Nora set her tools down on the bench. Otis waited beside the stove. Nora handed the brass key to Otis. He closed his fingers around it and nodded. Neither of them left the workshop.";
    let mut e = edits(f, prose);
    e.close_chapter = true;
    let placement =
        f.p.view()
            .unwrap()
            .world
            .edges
            .into_iter()
            .find(|l| l.source_id == f.key)
            .unwrap();
    e.effects_text=json!({"changes":[{"kind":"putLink","link":WorldEdge{target_id:f.otis.clone(),..placement},"evidence":"Nora handed the brass key to Otis."}],"notes":[],"suggestChapterEnd":false}).to_string();
    f.p.save_passage(&draft, 0, e).unwrap();
    f.p.validate_passage(&draft, 1, s).unwrap();
    let checked = f.p.passage_view(&draft, s).unwrap();
    assert!(checked.error.is_empty(), "{}", checked.error);
    let approved = f.p.approve_passage_for_test(&draft, s).unwrap();
    f.p.accept_passage(&draft, approved.version, s)
        .unwrap()
        .passage_id
}

#[tokio::test]
#[ignore = "explicit isolated real-provider workflow check"]
async fn real_ai_tools_pass() {
    let Ok(config) = std::env::var("STORY_AI_PASS_CONFIG") else {
        return;
    };
    let output = std::env::var("STORY_AI_PASS_ROOT").expect("Select a new isolated folder");
    let case = std::env::var("STORY_AI_PASS_CASE").unwrap();
    let root = Path::new(&output);
    fs::create_dir(root).unwrap();
    let mut s = load_connection(Path::new(&config)).unwrap().settings;
    assert_eq!(s.provider, TextProvider::Compatible);
    let endpoint = url::Url::parse(&s.base_url).unwrap();
    assert!(match endpoint.host() {
        Some(url::Host::Domain(n)) => n.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    });
    s.timeout_seconds = s.timeout_seconds.min(180);
    let key = connection_api_key(Path::new(&config), &s).unwrap();
    let f = fixture(root);
    let original = f.p.view().unwrap().world;
    match case.as_str() {
        "root-shape" => {
            s.max_tokens = s.max_tokens.min(128);
            let root_node = json!({"type":"object","properties":{"id":{"const":"new:root"},"typeId":{"const":"core.character"}},"required":["id","typeId"],"additionalProperties":false});
            let related = json!({"type":"object","properties":{"id":{"const":"new:item"},"typeId":{"const":"core.item"}},"required":["id","typeId"],"additionalProperties":false});
            let schema = json!({"type":"object","properties":{"nodes":{"type":"array","minItems":2,"maxItems":2,"prefixItems":[root_node],"items":related}},"required":["nodes"],"additionalProperties":false});
            let result = call(
                root,
                "root-tuple",
                &s,
                key.as_deref(),
                "Return only JSON matching the response schema.",
                "Return two nodes. Use new:unexpected for the first ID instead of new:root.",
                Some(&schema),
                128,
            )
            .await
            .unwrap();
            assert!(!result.incomplete);
            let value: Value = serde_json::from_str(&result.text).unwrap();
            assert_eq!(value["nodes"][0]["id"], "new:root");
            assert_eq!(value["nodes"][1]["id"], "new:item");
        }
        "revision" => {
            s.max_tokens = s.max_tokens.min(1000);
            let mut r = request();
            r.mode = AuthoringMode::Revise;
            r.bundle = false;
            r.creation_scope = None;
            r.target_ids = vec![f.custom.clone()];
            r.fields = ["method", "weakness"]
                .into_iter()
                .map(|k| EditableField {
                    node_id: f.custom.clone(),
                    component_id: Some("custom.repair".into()),
                    key: k.into(),
                })
                .collect();
            r.direction="Fill each checked field with one specific, usable sentence. A mechanic must improvise safe repairs during a winter power failure.".into();
            author(root, &f, &s, key.as_deref(), "custom-revision", r).await;
            let after = f.p.view().unwrap().world;
            let n = after.nodes.iter().find(|n| n.id == f.custom).unwrap();
            assert_eq!(n.name, "Nora's repair practice");
            assert_eq!(
                n.description,
                original
                    .nodes
                    .iter()
                    .find(|n| n.id == f.custom)
                    .unwrap()
                    .description
            );
            for k in ["method", "weakness"] {
                assert!(
                    matches!(n.components["custom.repair"].get(k),Some(PropertyValue::Text(t))if t.len()>15)
                );
            }
        }
        "character" => {
            let mut r = request();
            r.type_id = Some("core.character".into());
            r.creation_scope = None;
            r.context_ids = vec![f.room.clone()];
            r.direction="Create one adult travelling herbalist, Esme Vale, age 34, arriving at the workshop. Give her a distinct motive and voice, a complete practical worn outfit, a carried satchel with container capability, and three individually tracked remedies stored inside that satchel. Do not recreate existing people or places. Keep useful structured details concise.".into();
            author(root, &f, &s, key.as_deref(), "character-bundle", r).await;
            let w = f.p.view().unwrap().world;
            let person = w.nodes.iter().find(|n| n.name == "Esme Vale").unwrap();
            assert_eq!(
                w.nodes
                    .iter()
                    .filter(|n| n.components.contains_key("core.place"))
                    .count(),
                original
                    .nodes
                    .iter()
                    .filter(|n| n.components.contains_key("core.place"))
                    .count(),
                "The existing Workshop must be reused, not duplicated."
            );
            assert_eq!(
                w.nodes
                    .iter()
                    .filter(|n| n.components.contains_key("core.character"))
                    .count(),
                3
            );
            let bag = w
                .nodes
                .iter()
                .find(|n| {
                    n.components.contains_key("core.container")
                        && !original.nodes.iter().any(|old| old.id == n.id)
                })
                .unwrap();
            assert!(w
                .edges
                .iter()
                .any(|e| e.source_id == bag.id && e.target_id == person.id));
            assert_eq!(
                w.edges
                    .iter()
                    .filter(|e| e.type_id == "core.stored-in" && e.target_id == bag.id)
                    .count(),
                3
            );
            assert!(
                w.edges
                    .iter()
                    .filter(|e| e.type_id == "core.worn-by" && e.target_id == person.id)
                    .count()
                    >= 2
            );
            for n in &original.nodes {
                assert!(w.nodes.contains(n));
            }
            for e in &original.edges {
                assert!(w.edges.contains(e));
            }
        }
        "narration" => {
            s.max_tokens = s.max_tokens.min(1000);
            let mut r = request();
            r.mode = AuthoringMode::Revise;
            r.bundle = false;
            r.creation_scope = None;
            r.target_ids = vec![f.style.clone()];
            r.fields = vec![EditableField {
                node_id: f.style.clone(),
                component_id: Some("core.narration-style".into()),
                key: "instructions".into(),
            }];
            r.direction="Reroll the instructions for spare, sensory prose with dry dialogue. Preserve the profile's third-person viewpoint and past tense; do not introduce a conflicting tense or person.".into();
            author(root, &f, &s, key.as_deref(), "narration-reroll", r).await;
        }
        "group" => {
            s.max_tokens = s.max_tokens.min(1500);
            let mut r = request();
            r.group_type_id = Some("core.item".into());
            r.context_ids = vec![f.room.clone()];
            r.direction="Add two useful missing winter repair supplies to the existing workshop. A storm lantern and coil of rope are already present; do not duplicate either. Give each new item a concrete function, material and limitation. Store both in the existing workshop; do not create people or places.".into();
            author(root, &f, &s, key.as_deref(), "existing-group", r).await;
            let w = f.p.view().unwrap().world;
            assert_eq!(w.nodes.len(), original.nodes.len() + 2);
            for n in &original.nodes {
                assert!(w.nodes.contains(n));
            }
            for e in &original.edges {
                assert!(w.edges.contains(e));
            }
        }
        "story" => {
            s.max_tokens = s.max_tokens.min(1800);
            let draft = id();
            f.p.save_passage(&draft, 0, edits(&f, "")).unwrap();
            let prepared = f.p.start_passage(&draft, 1, 0, false, &s).unwrap();
            let result = call(
                root,
                "passage",
                &s,
                key.as_deref(),
                &prepared.system,
                &prepared.user,
                prepared.schema.as_ref(),
                prepared.output_tokens,
            )
            .await;
            let incomplete = result.as_ref().is_ok_and(|r| r.incomplete);
            f.p.finish_passage(&draft, false, result.map(|r| r.text), false, incomplete, &s)
                .unwrap();
            let view = f.p.passage_view(&draft, &s).unwrap();
            assert!(view.edits.prose.split_whitespace().count() >= 70);
            let prepared =
                f.p.start_passage(&draft, view.version, 0, true, &s)
                    .unwrap();
            let result = call(
                root,
                "effects",
                &s,
                key.as_deref(),
                &prepared.system,
                &prepared.user,
                prepared.schema.as_ref(),
                prepared.output_tokens,
            )
            .await;
            let incomplete = result.as_ref().is_ok_and(|r| r.incomplete);
            f.p.finish_passage(&draft, true, result.map(|r| r.text), false, incomplete, &s)
                .unwrap();
            let view = f.p.passage_view(&draft, &s).unwrap();
            fs::write(
                root.join("effects-review.json"),
                serde_json::to_vec_pretty(&view).unwrap(),
            )
            .unwrap();
            assert!(view.error.is_empty(), "{}", view.error);
            f.p.validate_passage(&draft, view.version, &s).unwrap();
            let checked = f.p.passage_view(&draft, &s).unwrap();
            assert!(checked.error.is_empty(), "{}", checked.error);
            let approved = f.p.approve_passage_for_test(&draft, &s).unwrap();
            let accepted = f.p.accept_passage(&draft, approved.version, &s).unwrap();
            let after = f.p.view().unwrap().world;
            assert!(after
                .edges
                .iter()
                .any(|e| e.source_id == f.key && e.target_id == f.otis));
            assert_eq!(f.p.story_index().unwrap().passages.len(), 1);
            fs::write(
                root.join("accepted.json"),
                serde_json::to_vec_pretty(&accepted).unwrap(),
            )
            .unwrap();
        }
        "memory-image" => {
            s.max_tokens = s.max_tokens.min(1000);
            let passage = manual_passage(&f, &s);
            let chapter = f.p.read_passage(&passage).unwrap().passage.chapter_id;
            let draft = id();
            f.p.create_memory_draft(&draft, &chapter).unwrap();
            let prepared =
                f.p.start_memory(&draft, 1, f.p.view().unwrap().revision, &s)
                    .unwrap();
            let result = call(
                root,
                "memory",
                &s,
                key.as_deref(),
                &prepared.system,
                &prepared.user,
                Some(&prepared.schema),
                prepared.output_tokens,
            )
            .await;
            let incomplete = result.as_ref().is_ok_and(|r| r.incomplete);
            f.p.finish_memory(&draft, result.map(|r| r.text), false, incomplete)
                .unwrap();
            let view = f.p.memory_view(&draft).unwrap();
            fs::write(
                root.join("memory-review.json"),
                serde_json::to_vec_pretty(&view).unwrap(),
            )
            .unwrap();
            assert!(view.can_apply, "{}", view.error);
            f.p.apply_memory(&draft, view.version).unwrap();
            let media = id();
            f.p.create_media(
                &media,
                MediaTarget {
                    guidance_source: None,
                    node_id: Some(f.nora.clone()),
                    passage_id: None,
                    moment: ImageMoment::After,
                    subject_ids: vec![],
                },
            )
            .unwrap();
            let image = ImageConnection::default();
            let draft = id();
            let view =
                f.p.create_image_prompt(
                    &draft,
                    &media,
                    f.p.media_draft(&media).unwrap().version,
                    &s,
                    &image,
                )
                .unwrap();
            let prepared =
                f.p.start_image_prompt(&draft, view.version, &s, &image)
                    .unwrap();
            let result = call(
                root,
                "image-prompt",
                &s,
                key.as_deref(),
                &prepared.system,
                &prepared.user,
                Some(&prepared.schema),
                prepared.output_tokens,
            )
            .await;
            let incomplete = result.as_ref().is_ok_and(|r| r.incomplete);
            f.p.finish_image_prompt(&draft, result.map(|r| r.text), false, incomplete)
                .unwrap();
            let view = f.p.image_prompt_view(&draft, &s, &image).unwrap();
            fs::write(
                root.join("image-prompt-review.json"),
                serde_json::to_vec_pretty(&view).unwrap(),
            )
            .unwrap();
            assert!(view.can_apply, "{}", view.error);
            f.p.apply_image_prompt(&draft, view.version, &s, &image)
                .unwrap();
            assert_eq!(f.p.story_index().unwrap().passages.len(), 1);
        }
        "wording" => {
            s.max_tokens = s.max_tokens.min(700);
            let passage = manual_passage(&f, &s);
            let reading = f.p.read_passage(&passage).unwrap();
            let draft = id();
            f.p.begin_passage_revision(
                &draft,
                &passage,
                reading.record,
                PassageRevisionKind::Wording,
            )
            .unwrap();
            let mut view = f.p.passage_view(&draft, &s).unwrap();
            view.edits.plan.direction="Smooth the prose and make the dialogue-free actions read naturally. Preserve every event, the completed key handoff, viewpoint and past tense.".into();
            f.p.save_passage(&draft, view.version, view.edits).unwrap();
            let view = f.p.passage_view(&draft, &s).unwrap();
            let prepared =
                f.p.start_passage(
                    &draft,
                    view.version,
                    f.p.view().unwrap().revision,
                    false,
                    &s,
                )
                .unwrap();
            let result = call(
                root,
                "wording",
                &s,
                key.as_deref(),
                &prepared.system,
                &prepared.user,
                prepared.schema.as_ref(),
                prepared.output_tokens,
            )
            .await;
            let incomplete = result.as_ref().is_ok_and(|r| r.incomplete);
            f.p.finish_passage(&draft, false, result.map(|r| r.text), false, incomplete, &s)
                .unwrap();
            let view = f.p.passage_view(&draft, &s).unwrap();
            f.p.validate_passage(&draft, view.version, &s).unwrap();
            let checked = f.p.passage_view(&draft, &s).unwrap();
            assert!(checked.error.is_empty(), "{}", checked.error);
            let approved = f.p.approve_passage_for_test(&draft, &s).unwrap();
            f.p.accept_passage(&draft, approved.version, &s).unwrap();
            assert_eq!(f.p.story_index().unwrap().passages.len(), 1);
            assert_eq!(f.p.read_passage(&passage).unwrap().after, reading.after);
        }
        _ => panic!("Unknown explicit probe case"),
    }
    fs::write(root.join("passed.json"),serde_json::to_vec_pretty(&json!({"case":case,"provider":s.model,"revision":f.p.view().unwrap().revision,"passed":true})).unwrap()).unwrap();
    println!("PASS {case}");
}
