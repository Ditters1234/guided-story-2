use super::*;
use crate::*;

#[test]
fn prompt_edits_roundtrip_with_replay_conflicts_restore_and_independent_connection_saves() {
    let dir = tempfile::tempdir().unwrap();
    let original = load_connection(dir.path()).unwrap().settings;
    let mut overrides = BTreeMap::from([(
        TextPromptTask::Passage,
        "Write in spare, concrete language.".into(),
    )]);
    let saved = save_prompt_templates(dir.path(), 0, overrides.clone()).unwrap();
    assert_eq!(saved.version, 1);
    assert_eq!(
        load_connection(dir.path()).unwrap().settings.version,
        original.version
    );
    assert_eq!(
        save_prompt_templates(dir.path(), 0, overrides.clone())
            .unwrap()
            .version,
        1
    );
    let mut old_transport_form = original.clone();
    old_transport_form.temperature = 0.45;
    let transport = save_connection(dir.path(), 0, old_transport_form, ApiKeyUpdate::Keep).unwrap();
    assert_eq!(transport.settings.prompts.overrides, overrides);
    assert_eq!(transport.settings.prompts.version, 1);
    overrides.insert(
        TextPromptTask::Memory,
        "Keep concise source-linked memory.".into(),
    );
    assert!(save_prompt_templates(dir.path(), 0, overrides.clone()).is_err());
    save_prompt_templates(dir.path(), 1, overrides).unwrap();
    let restored = save_prompt_templates(
        dir.path(),
        2,
        BTreeMap::from([(
            TextPromptTask::Passage,
            TextPromptTask::Passage.default_text().into(),
        )]),
    )
    .unwrap();
    assert!(restored
        .templates
        .iter()
        .all(|t| !t.customized && t.text == t.default_text));
    let connection = load_connection(dir.path()).unwrap().settings;
    assert_eq!(connection.temperature, 0.45);
    assert_eq!(connection.version, 1);
    assert_eq!(connection.prompts.version, 3);
    assert_eq!(connection.credential_ref, original.credential_ref);
}

#[test]
fn legacy_settings_keep_their_serialized_shape_and_prompt_edits_are_bounded() {
    let original = LlmConnection::default();
    let json = serde_json::to_value(&original).unwrap();
    assert!(json.get("prompts").is_none());
    assert_eq!(
        serde_json::from_value::<LlmConnection>(json).unwrap(),
        original
    );
    let dir = tempfile::tempdir().unwrap();
    for invalid in ["".into(), "  \n".into(), "x".repeat(32769)] {
        assert!(save_prompt_templates(
            dir.path(),
            0,
            BTreeMap::from([(TextPromptTask::World, invalid)])
        )
        .is_err());
    }
    assert!(!dir.path().join("llm-connection.json").exists());
    let templates = original.prompt_settings_view().templates;
    assert_eq!(templates.len(), 6);
    for t in &templates {
        assert_eq!(t.text, original.system_prompt(t.id));
        assert!(!t.text.is_empty());
    }
    assert!(!serde_json::to_string(&original.prompt_settings_view())
        .unwrap()
        .contains("credentialRef"));
    let max = "é".repeat(16384);
    save_prompt_templates(
        dir.path(),
        0,
        BTreeMap::from([(TextPromptTask::World, max.clone())]),
    )
    .unwrap();
    assert_eq!(
        load_connection(dir.path())
            .unwrap()
            .settings
            .system_prompt(TextPromptTask::World),
        max
    );
    let saved_bytes = std::fs::read(dir.path().join("llm-connection.json")).unwrap();
    assert!(save_prompt_templates(
        dir.path(),
        1,
        BTreeMap::from([
            (TextPromptTask::World, "\u{1}".repeat(32768)),
            (TextPromptTask::Memory, "\u{1}".repeat(32768))
        ])
    )
    .is_err());
    assert_eq!(
        std::fs::read(dir.path().join("llm-connection.json")).unwrap(),
        saved_bytes
    );
}

#[cfg(windows)]
#[test]
fn prompt_saves_preserve_the_encrypted_api_key_without_exposing_it_to_the_editor() {
    let dir = tempfile::tempdir().unwrap();
    let initial = save_connection(
        dir.path(),
        0,
        LlmConnection::default(),
        ApiKeyUpdate::Replace("fictional-prompt-test-key".into()),
    )
    .unwrap();
    let prompts = save_prompt_templates(
        dir.path(),
        0,
        BTreeMap::from([(
            TextPromptTask::World,
            "Develop concrete world records.".into(),
        )]),
    )
    .unwrap();
    let next = load_connection(dir.path()).unwrap();
    assert_eq!(
        initial.settings.credential_ref,
        next.settings.credential_ref
    );
    assert_eq!(
        connection_api_key(dir.path(), &next.settings)
            .unwrap()
            .as_deref(),
        Some("fictional-prompt-test-key")
    );
    let encoded = serde_json::to_string(&prompts).unwrap();
    assert!(!encoded.contains("fictional-prompt-test-key") && !encoded.contains("credentialRef"));
}

#[test]
fn world_generation_uses_the_exact_override_and_still_carries_schema_and_scoped_context() {
    let mut settings = LlmConnection {
        provider: TextProvider::Demo,
        ..Default::default()
    };
    let world = World::default();
    let request = AuthoringRequest {
        link_target_ids: vec![],
        include_narration: None,
        creation_type_ids: vec![],
        link_fields: vec![],
        mode: AuthoringMode::Create,
        direction: "A fictional observatory.".into(),
        type_id: Some("core.place".into()),
        bundle: false,
        creation_scope: None,
        group_type_id: None,
        target_ids: vec![],
        context_ids: vec![],
        fields: vec![],
    };
    let normal = prepare_authoring(&world, &request, &settings).unwrap();
    settings.prompts.overrides.insert(
        TextPromptTask::World,
        "Use a concise, specific world-building style.".into(),
    );
    let custom = prepare_authoring(&world, &request, &settings).unwrap();
    assert_eq!(
        custom.system,
        "Use a concise, specific world-building style."
    );
    assert_ne!(custom.fingerprint, normal.fingerprint);
    assert_eq!(custom.user, normal.user);
    assert_eq!(custom.response_schema, normal.response_schema);
    assert!(custom.user.contains("responseSchema"));
    settings
        .prompts
        .overrides
        .insert(TextPromptTask::World, "x".repeat(32768));
    settings.context_tokens = 2048;
    settings.max_tokens = 1024;
    assert!(prepare_authoring(&world, &request, &settings).is_err());
}
