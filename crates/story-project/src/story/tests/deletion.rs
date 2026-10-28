use super::*;

#[test]
fn passage_draft_deletion_survives_reopen_and_does_not_resurrect_or_change_history() {
    let (dir, p) = fixture();
    let before = p.view().unwrap();
    let key = id();
    let edit = edits(&p, "An unaccepted passage to discard.");
    p.save_passage(&key, 0, edit.clone()).unwrap();
    assert!(p.delete_passage_draft(&key, 0).is_err());
    p.delete_passage_draft(&key, 1).unwrap();
    p.delete_passage_draft(&key, 1).unwrap();
    assert!(p.list_passage_drafts().unwrap().is_empty());
    assert!(p
        .passage_view(&key, &settings())
        .unwrap_err()
        .contains("deleted"));
    for version in [0, 1, 2] {
        assert!(p.save_passage(&key, version, edit.clone()).is_err());
    }
    assert!(p.clone_passage_draft(&key, 2, &id()).is_err());
    assert!(p
        .start_passage(&key, 2, before.revision, false, &settings())
        .is_err());
    assert!(p.validate_passage(&key, 2, &settings()).is_err());
    assert!(p.accept_passage(&key, 2, &settings()).is_err());
    let marker = p.read_passage_draft_record(&key).unwrap();
    assert!(
        marker.edits.prose.is_empty()
            && marker.original_prose.is_empty()
            && marker.source.is_none()
    );
    assert_eq!(p.view().unwrap().world_ref, before.world_ref);
    assert_eq!(p.view().unwrap().revision, before.revision);
    assert!(p.story_index().unwrap().passages.is_empty());
    drop(p);
    let p = Project::open(&dir.path().join("story")).unwrap();
    p.interrupt_passages().unwrap();
    assert!(p.list_passage_drafts().unwrap().is_empty());
    p.delete_passage_draft(&key, 1).unwrap();
    p.save_passage(&id(), 0, edit).unwrap();
}

#[test]
fn deleting_revisions_and_rerolls_preserves_accepted_sources_and_other_candidates() {
    let (_dir, p) = fixture();
    let key = id();
    p.save_passage(&key, 0, edits(&p, "The room fell quiet."))
        .unwrap();
    p.validate_passage(&key, 1, &settings()).unwrap();
    let ready = p.passage_view(&key, &settings()).unwrap();
    p.accept_passage(&key, ready.version, &settings()).unwrap();
    let reading = p.read_passage(&key).unwrap();
    let before = p.view().unwrap();
    let accepted = p.read_passage_draft(&key).unwrap();
    assert!(p.delete_passage_draft(&key, accepted.version).is_err());
    // Receipt reconciliation must also protect a committed result with stale status.
    let mut uncertain = accepted.clone();
    uncertain.stage = PassageStage::Ready;
    p.write_passage_draft(&uncertain).unwrap();
    assert!(p.delete_passage_draft(&key, uncertain.version).is_err());
    for kind in [PassageRevisionKind::Wording, PassageRevisionKind::Events] {
        let revision = id();
        p.begin_passage_revision(&revision, &key, reading.record.clone(), kind)
            .unwrap();
        let candidate = id();
        p.clone_passage_draft(&revision, 1, &candidate).unwrap();
        p.delete_passage_draft(&revision, 1).unwrap();
        assert!(p
            .begin_passage_revision(&revision, &key, reading.record.clone(), kind)
            .is_err());
        assert!(p
            .clone_passage_draft(&key, accepted.version, &revision)
            .is_err());
        assert_eq!(
            p.passage_view(&candidate, &settings())
                .unwrap()
                .revision
                .unwrap()
                .kind,
            kind
        );
        p.delete_passage_draft(&candidate, 1).unwrap();
    }
    assert_eq!(p.read_passage(&key).unwrap().record, reading.record);
    assert_eq!(p.view().unwrap().world_ref, before.world_ref);
    assert_eq!(p.view().unwrap().revision, before.revision);
}

#[test]
fn running_passage_drafts_are_protected_until_stopped() {
    let (_dir, p) = fixture();
    for checking in [false, true] {
        let key = id();
        p.save_passage(
            &key,
            0,
            edits(&p, if checking { "A manual passage." } else { "" }),
        )
        .unwrap();
        p.start_passage(&key, 1, p.view().unwrap().revision, checking, &settings())
            .unwrap();
        assert!(p.delete_passage_draft(&key, 2).is_err());
        p.finish_passage(
            &key,
            checking,
            Err("Stopped fixture".into()),
            true,
            false,
            &settings(),
        )
        .unwrap();
        let stopped = p.passage_view(&key, &settings()).unwrap();
        p.delete_passage_draft(&key, stopped.version).unwrap();
    }
}
