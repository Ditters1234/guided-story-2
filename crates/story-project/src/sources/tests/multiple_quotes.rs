use super::*;

#[test]
fn multiple_event_quotes_keep_effect_indexes_wording_evidence_backups_and_fork_boundaries() {
    let (dir, p) = fixture();
    let legacy = event_passage(&p);
    let legacy_reading = p.read_passage(&legacy).unwrap();
    let legacy_anchor = legacy_reading.passage.text_anchor.clone().unwrap();
    let legacy_path = p
        .root_path()
        .join("source-anchors")
        .join(format!("{}.json", legacy_anchor.id));
    let legacy_bytes = std::fs::read(&legacy_path).unwrap();
    let legacy_json: serde_json::Value = serde_json::from_slice(&legacy_bytes).unwrap();
    assert!(legacy_json["effectQuotes"][0].is_string());
    let first = "The pump failed beside the workshop.";
    let last = "Nora secured the broken valve.";
    let prose = format!(
        "{first} {} {last}",
        "Rain rattled against the workshop roof. ".repeat(25)
    );
    let mut effects = event_effects(&p, "Pump failure and valve secured", first);
    let PassageEffect::PutNode { node, evidence } = &mut effects.changes[0] else {
        panic!()
    };
    node.components.get_mut("detail.event").unwrap().insert(
        "occurrence".into(),
        PropertyValue::Text(format!("{first} {last}")),
    );
    *evidence = EffectEvidence::Quotes(vec![first.into(), last.into()]);
    let pid = accept(&p, edits(&p, &prose, effects));
    let original = p.read_passage(&pid).unwrap();
    let current = p.view().unwrap();
    let event = current
        .world
        .nodes
        .iter()
        .find(|n| n.name == "Pump failure and valve secured")
        .unwrap()
        .clone();
    let citations = p.node_sources(&current.world, &event.id).unwrap();
    assert_eq!(citations.len(), 2);
    let mut quotes = vec![];
    for citation in &citations {
        let SourceOrigin::Passage {
            quote,
            effect_index,
            ..
        } = &citation.annotation.origin
        else {
            panic!()
        };
        assert_eq!(*effect_index, Some(0));
        quotes.push(quote.as_str());
        assert_eq!(
            p.read_node_source(&current.world, &citation.annotation.id)
                .unwrap()
                .prose,
            prose
        );
    }
    assert!(quotes.contains(&first) && quotes.contains(&last));
    let citation_request = |effect_index| SourceEdit::AddPassage {
        node_id: current.world.nodes[1].id.clone(),
        passage_id: pid.clone(),
        version: original.record.clone(),
        quote: last.into(),
        effect_index: Some(effect_index),
    };
    assert!(p
        .preview_source_edit(&current.world, citation_request(0))
        .is_ok());
    assert!(p
        .preview_source_edit(&current.world, citation_request(1))
        .is_err());
    let revision = id();
    p.begin_passage_revision(
        &revision,
        &pid,
        original.record.clone(),
        PassageRevisionKind::Wording,
    )
    .unwrap();
    let view = p.passage_view(&revision, &settings()).unwrap();
    let mut edit = view.edits;
    edit.prose = format!("{first} The broken valve was secured by Nora.");
    p.save_passage(&revision, view.version, edit).unwrap();
    p.validate_passage(&revision, view.version + 1, &settings())
        .unwrap();
    let view = p.passage_view(&revision, &settings()).unwrap();
    p.accept_passage(&revision, view.version, &settings())
        .unwrap();
    let revised = p.read_passage(&pid).unwrap();
    assert_eq!(
        revised.passage.evidence_source,
        Some(original.record.clone())
    );
    assert_eq!(revised.evidence_prose, Some(prose.clone()));
    assert_eq!(revised.before, original.before);
    assert_eq!(revised.after, original.after);
    let current = p.view().unwrap();
    assert!(p
        .node_sources(&current.world, &event.id)
        .unwrap()
        .iter()
        .all(|c| c.earlier_wording));
    let backup = p
        .export_backup(dir.path(), &id(), current.revision)
        .unwrap();
    let restored = crate::restore_backup(
        Path::new(&backup.folder_path),
        dir.path(),
        &id(),
        &backup.fingerprint,
    )
    .unwrap();
    let copy = Project::open(Path::new(&restored.folder_path)).unwrap();
    assert_eq!(
        copy.read_passage(&pid).unwrap().evidence_prose,
        Some(prose.clone())
    );
    assert_eq!(
        copy.node_sources(&copy.view().unwrap().world, &event.id)
            .unwrap()
            .len(),
        2
    );
    // A later source must not be carried into the earlier fork.
    let future_prose = "A storm damaged the workshop roof.";
    let future = accept(
        &p,
        edits(
            &p,
            future_prose,
            event_effects(&p, "Future storm", future_prose),
        ),
    );
    let future_anchor = p
        .read_passage(&future)
        .unwrap()
        .passage
        .text_anchor
        .unwrap();
    let fork = p
        .fork_after_passage(&pid, dir.path(), "Earlier multi-quote boundary".into())
        .unwrap();
    assert_eq!(
        fork.read_passage(&pid).unwrap().evidence_prose,
        Some(prose.clone())
    );
    assert!(!fork
        .root_path()
        .join("source-anchors")
        .join(format!("{}.json", future_anchor.id))
        .exists());
    let fork_world = fork.view().unwrap().world;
    for citation in fork.node_sources(&fork_world, &event.id).unwrap() {
        let reading = fork
            .read_node_source(&fork_world, &citation.annotation.id)
            .unwrap();
        assert_eq!(reading.prose, prose);
        assert!(reading.quote == first || reading.quote == last);
    }
    assert_eq!(std::fs::read(&legacy_path).unwrap(), legacy_bytes);
    assert_eq!(
        std::fs::read(
            copy.root_path()
                .join("source-anchors")
                .join(format!("{}.json", legacy_anchor.id))
        )
        .unwrap(),
        legacy_bytes
    );
    assert_eq!(
        std::fs::read(
            fork.root_path()
                .join("source-anchors")
                .join(format!("{}.json", legacy_anchor.id))
        )
        .unwrap(),
        legacy_bytes
    );
}

#[test]
fn event_source_quota_reserves_every_excerpt_before_publication() {
    let (_dir, p) = fixture();
    let mut w = p.view().unwrap().world;
    let event = node(&w, "core.event", "Existing event");
    w.nodes.push(event.clone());
    for i in 0..31 {
        w.source_annotations.push(SourceAnnotation {
            id: id(),
            node_id: event.id.clone(),
            subject_fingerprint: node_source_fingerprint(&event),
            origin: SourceOrigin::Manual {
                note: format!("Author source {i}"),
            },
        });
    }
    w.validate().unwrap();
    let quotes = vec![
        "The pump failed beside the workshop.".into(),
        "Nora secured the broken valve.".into(),
    ];
    let effects = PassageEffects {
        changes: vec![PassageEffect::PutNode {
            node: event.clone(),
            evidence: EffectEvidence::Quotes(quotes),
        }],
        ..Default::default()
    };
    let issues = crate::story_effects::effect_record_issues(&w, &w, &effects);
    assert!(issues[0]
        .iter()
        .any(|issue| issue.contains("space for its source citation")));
    let one = PassageEffects {
        changes: vec![PassageEffect::PutNode {
            node: event,
            evidence: "The pump failed beside the workshop.".into(),
        }],
        ..Default::default()
    };
    assert!(!crate::story_effects::effect_record_issues(&w, &w, &one)[0]
        .iter()
        .any(|issue| issue.contains("space for its source citation")));
}
