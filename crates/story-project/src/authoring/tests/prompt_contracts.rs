use super::*;
use crate::authoring::prompt_contract;

fn revision(world: &World) -> AuthoringRequest {
    AuthoringRequest {
        mode: AuthoringMode::Revise,
        type_id: None,
        target_ids: vec![world.nodes[0].id.clone()],
        context_ids: vec![world.nodes[1].id.clone()],
        fields: vec![EditableField {
            node_id: world.nodes[0].id.clone(),
            component_id: Some("detail.intent".into()),
            key: "objective".into(),
        }],
        link_target_ids: vec![world.edges[0].id.clone()],
        link_fields: vec![EditableLinkField {
            link_id: world.edges[0].id.clone(),
            property: None,
        }],
        ..request()
    }
}

fn source() -> World {
    let mut world = World::default();
    world.nodes = vec![
        proposed_node(&world, "core.character", &id(), "Mara"),
        proposed_node(&world, "core.place", &id(), "Glasshouse"),
    ];
    world.edges.push(WorldEdge {
        id: id(),
        type_id: "core.present-at".into(),
        source_id: world.nodes[0].id.clone(),
        target_id: world.nodes[1].id.clone(),
        description: String::new(),
        properties: BTreeMap::new(),
    });
    world
}

#[test]
fn each_operation_has_explicit_permissions_and_its_own_output_contract_in_all_transports() {
    let world = source();
    let requests = [
        ("world", group_request()),
        (
            "group",
            AuthoringRequest {
                creation_scope: Some(CreationScope::Group),
                ..group_request()
            },
        ),
        ("node", request()),
        (
            "node",
            AuthoringRequest {
                bundle: true,
                type_id: Some("core.character".into()),
                ..request()
            },
        ),
        ("revision", revision(&world)),
        (
            "revision",
            AuthoringRequest {
                target_ids: vec![],
                fields: vec![],
                ..revision(&world)
            },
        ),
    ];
    for (kind, request) in requests {
        for mode in [
            StructuredOutput::Json,
            StructuredOutput::Prompt,
            StructuredOutput::Schema,
        ] {
            let mut connection = settings();
            connection.structured_output = mode;
            let prepared = prepare_authoring(&world, &request, &connection).unwrap();
            let input: Value = serde_json::from_str(&prepared.user).unwrap();
            assert_eq!(input["operation"]["kind"], kind);
            assert_eq!(
                prepared.system,
                connection.system_prompt(TextPromptTask::World)
            );
            if kind == "revision" {
                assert!(input.get("creationGuide").is_none());
                let permissions = input["recordPermissions"]["nodes"].as_array().unwrap();
                let place = permissions
                    .iter()
                    .find(|p| p["id"] == world.nodes[1].id)
                    .unwrap();
                assert_eq!(place["role"], "reference");
                let link = &input["recordPermissions"]["links"][0];
                assert_eq!(link["role"], "editable");
                assert_eq!(link["fields"], json!([null]));
                if !request.target_ids.is_empty() {
                    let person = permissions
                        .iter()
                        .find(|p| p["id"] == world.nodes[0].id)
                        .unwrap();
                    assert_eq!(person["role"], "editable");
                    assert_eq!(
                        person["fields"],
                        json!([{"componentId":"detail.intent","key":"objective"}])
                    );
                }
            }
        }
    }
}

#[test]
fn preflight_rejects_changed_scope_missing_sources_and_mismatched_submitted_schema() {
    let world = source();
    let request = revision(&world);
    let prepared = prepare_authoring(&world, &request, &settings()).unwrap();
    let input: Value = serde_json::from_str(&prepared.user).unwrap();
    let check = |packet: &Value, schema: &Value| {
        prompt_contract::lint(
            &prepared.context,
            &prepared.context,
            &request,
            packet,
            schema,
            &crate::authoring::references::scope(
                &world,
                &prepared.context,
                &prepared.context,
                &request,
                None,
            ),
        )
    };
    for change in [
        "permission",
        "source",
        "request",
        "schema",
        "protectedField",
    ] {
        let mut changed = input.clone();
        match change {
            "permission" => changed["recordPermissions"]["nodes"][1]["role"] = json!("editable"),
            "source" => {
                changed["records"].as_array_mut().unwrap().pop();
            }
            "request" => changed["request"]["fields"][0]["key"] = json!("name"),
            "schema" => {
                changed["responseSchema"]["properties"]["nodes"]["items"]["anyOf"][0]
                    ["properties"]["id"]["const"] = json!(world.nodes[1].id)
            }
            _ => {
                changed["responseSchema"]["properties"]["nodes"]["items"]["anyOf"][0]
                    ["properties"]["name"] = json!({"type":"string"})
            }
        }
        let error = check(&changed, &prepared.response_schema).unwrap_err();
        assert!(error.contains("not sent"), "{change}: {error}");
    }
    let mut schema = prepared.response_schema.clone();
    schema["properties"]["edges"]["maxItems"] = json!(2);
    assert!(check(&input, &schema).is_err());
}

#[test]
fn preflight_rejects_invalid_example_fields_and_preserves_saved_custom_instructions() {
    let world = World::default();
    let request = group_request();
    let mut connection = settings();
    connection.prompts.overrides.insert(
        TextPromptTask::World,
        "Custom saved world instructions.".into(),
    );
    let prepared = prepare_authoring(&world, &request, &connection).unwrap();
    assert_eq!(prepared.system, "Custom saved world instructions.");
    let input: Value = serde_json::from_str(&prepared.user).unwrap();
    assert_eq!(input["operation"]["kind"], "world");
    let mut bad_field = input.clone();
    bad_field["clothingExample"]["records"][0]["components"]["detail.design"]["inventedField"] =
        json!("Not declared");
    assert!(prompt_contract::lint(
        &prepared.context,
        &world,
        &request,
        &bad_field,
        &prepared.response_schema,
        &crate::authoring::references::scope(&world, &world, &prepared.context, &request, None),
    )
    .is_err());
    let mut bad_type = input;
    bad_type["clothingExample"]["records"][0]["typeId"] = json!("unavailable.type");
    assert!(prompt_contract::lint(
        &prepared.context,
        &world,
        &request,
        &bad_type,
        &prepared.response_schema,
        &crate::authoring::references::scope(&world, &world, &prepared.context, &request, None),
    )
    .is_err());
}
