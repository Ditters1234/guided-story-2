use super::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PassageRevisionKind {
    Wording,
    Events,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PassageRevision {
    pub kind: PassageRevisionKind,
    pub passage_id: String,
    pub source: RecordRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PassageRevisionView {
    pub kind: PassageRevisionKind,
    pub passage_id: String,
    pub source: RecordRef,
    pub prose: String,
    pub world: World,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EarlierPassageVersion {
    pub record: RecordRef,
    pub prose: String,
    pub revision_kind: Option<PassageRevisionKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PassageVersions {
    pub versions: Vec<EarlierPassageVersion>,
    pub more: bool,
}

impl Project {
    pub(super) fn passage_committed(&self, draft: &PassageDraft) -> Result<Option<u32>, String> {
        let Some(attempt) = &draft.attempt else {
            return Ok(None);
        };
        self.committed_story_request(
            &draft.id,
            &attempt.title,
            &self.read_record("worlds", &attempt.world)?,
            &self.read_record("stories", &attempt.index)?,
        )
    }

    fn revision_base(
        &self,
        revision: &PassageRevision,
        current: bool,
    ) -> Result<AcceptedPassage, String> {
        let base: AcceptedPassage = self.read_record("passages", &revision.source)?;
        if base.schema_version != 1 || base.id != revision.passage_id {
            return Err("Invalid passage revision source.".into());
        }
        if current {
            let reading = self.read_passage(&revision.passage_id)?;
            if reading.record != revision.source {
                return Err("This passage was revised. Keep this draft and start a revision from its current version.".into());
            }
            if revision.kind == PassageRevisionKind::Events && !reading.replacement_error.is_empty()
            {
                return Err(reading.replacement_error);
            }
        }
        Ok(base)
    }

    pub fn begin_passage_revision(
        &self,
        id: &str,
        passage_id: &str,
        source: RecordRef,
        kind: PassageRevisionKind,
    ) -> Result<(), String> {
        validate_id(id)?;
        validate_id(passage_id)?;
        let revision = PassageRevision {
            kind,
            passage_id: passage_id.into(),
            source,
        };
        if self.passage_path(id)?.exists() {
            if self.read_passage_draft(id)?.revision.as_ref() == Some(&revision) {
                return Ok(());
            }
            return Err("This draft ID already has a different purpose.".into());
        }
        let base = self.revision_base(&revision, true)?;
        let mut plan = base.plan;
        if kind == PassageRevisionKind::Wording {
            plan.direction.clear();
        }
        let effects_text = if kind == PassageRevisionKind::Wording {
            serde_json::to_string(&base.effects).map_err(|e| e.to_string())?
        } else {
            empty_effects()
        };
        let mut draft = self.new_passage_draft(
            id,
            PassageEdits {
                plan,
                prose: base.prose,
                effects_text,
                close_chapter: base.close_chapter,
            },
        )?;
        draft.revision = Some(revision);
        self.write_passage_draft(&draft)
    }

    fn new_passage_draft(&self, id: &str, edits: PassageEdits) -> Result<PassageDraft, String> {
        validate_id(id)?;
        validate_edits(&edits)?;
        if self.list_passage_drafts()?.len() >= 200 {
            return Err("Passage draft limit reached.".into());
        }
        Ok(PassageDraft {
            revision: None,
            cloned_from: None,
            narration_provider: String::new(),
            effects_provider: String::new(),
            schema_version: 1,
            id: id.into(),
            story_id: self.view()?.story_id,
            version: 1,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| "Clock unavailable.")?
                .as_millis() as u64,
            stage: if edits.prose.trim().is_empty() {
                PassageStage::Draft
            } else {
                PassageStage::NeedsCheck
            },
            edits,
            source: None,
            narration_source: None,
            checked: String::new(),
            error: String::new(),
            original_prose: String::new(),
            original_effects: String::new(),
            allocations: BTreeMap::new(),
            attempt: None,
            incomplete: false,
        })
    }

    pub fn clone_passage_draft(
        &self,
        source_id: &str,
        expected: u32,
        id: &str,
    ) -> Result<(), String> {
        validate_id(source_id)?;
        validate_id(id)?;
        if self.passage_path(id)?.exists() {
            if self.read_passage_draft(id)?.cloned_from == Some((source_id.into(), expected)) {
                return Ok(());
            }
            return Err("This candidate ID was already used for another draft.".into());
        }
        let source = self.read_passage_draft(source_id)?;
        if source.version != expected
            || matches!(
                source.stage,
                PassageStage::Narrating | PassageStage::Checking
            )
        {
            return Err("Save/finish the source candidate before rerolling.".into());
        }
        let mut edits = source.edits;
        edits.prose.clear();
        if source
            .revision
            .as_ref()
            .is_none_or(|r| r.kind != PassageRevisionKind::Wording)
        {
            edits.effects_text = empty_effects();
        }
        let mut draft = self.new_passage_draft(id, edits)?;
        draft.revision = source.revision;
        draft.cloned_from = Some((source_id.into(), expected));
        self.write_passage_draft(&draft)
    }

    fn history_before(&self, passage_id: &str) -> Result<StoryIndex, String> {
        let mut index = self.story_index()?;
        let position = index
            .passages
            .iter()
            .position(|p| p.id == passage_id)
            .ok_or("The revision target is no longer in this story.")?;
        index.passages.truncate(position);
        let retained: BTreeSet<_> = index.passages.iter().map(|p| &p.id).collect();
        for chapter in &mut index.chapters {
            chapter.passage_ids.retain(|id| retained.contains(id));
        }
        index.chapters.retain(|ch| !ch.passage_ids.is_empty());
        index
            .memories
            .retain(|id, _| index.chapters.iter().any(|ch| &ch.id == id));
        if let (Some(chapter), Some(entry)) = (index.chapters.last_mut(), index.passages.last()) {
            let last: AcceptedPassage = self.read_record("passages", &entry.record)?;
            chapter.title = last.plan.chapter_title;
            chapter.plan = last.plan.chapter_plan;
            chapter.closed = last.close_chapter;
        }
        index.validate()?;
        Ok(index)
    }

    pub(super) fn draft_input(
        &self,
        draft: &PassageDraft,
        settings: &LlmConnection,
    ) -> Result<(World, RecordRef, Value, String, Vec<String>), String> {
        self.draft_input_budget(
            draft,
            settings,
            draft
                .source
                .as_ref()
                .map_or(default_memory_budget(), |s| s.memory_budget),
        )
    }
    pub(super) fn draft_input_budget(
        &self,
        draft: &PassageDraft,
        settings: &LlmConnection,
        memory_budget: u32,
    ) -> Result<(World, RecordRef, Value, String, Vec<String>), String> {
        if let Some(revision) = &draft.revision {
            let base = self.revision_base(revision, true)?;
            if revision.kind == PassageRevisionKind::Wording {
                let mut permitted = base.plan.clone();
                permitted.direction = draft.edits.plan.direction.clone();
                if draft.edits.plan != permitted
                    || draft.edits.close_chapter != base.close_chapter
                    || serde_json::from_str::<Value>(&draft.edits.effects_text).ok()
                        != Some(serde_json::to_value(&base.effects).map_err(|e| e.to_string())?)
                {
                    return Err("Wording revisions preserve the accepted plan, chapter closing and effects. Use event replacement to change them.".into());
                }
            }
            let world: World = self.read_record("worlds", &base.before_world)?;
            let history = self.history_before(&revision.passage_id)?;
            let (context, mut input, fingerprint, recent) =
                self.story_input_at(&world, &draft.edits.plan, settings, &history, memory_budget)?;
            if revision.kind == PassageRevisionKind::Wording {
                input["originalProse"] = json!(base.prose);
                input["acceptedEffects"] = json!(base.effects);
            }
            let fingerprint = story_hash(&(fingerprint, revision))?;
            Ok((context, base.before_world, input, fingerprint, recent))
        } else {
            let current = self.view()?;
            let (context, input, fingerprint, recent) =
                self.story_input(&current.world, &draft.edits.plan, settings, memory_budget)?;
            Ok((context, current.world_ref, input, fingerprint, recent))
        }
    }

    pub(super) fn revision_view(
        &self,
        draft: &PassageDraft,
    ) -> Result<Option<PassageRevisionView>, String> {
        draft
            .revision
            .as_ref()
            .map(|revision| {
                let base = self.revision_base(revision, false)?;
                let world: World = self.read_record("worlds", &base.before_world)?;
                world.validate()?;
                Ok(PassageRevisionView {
                    kind: revision.kind,
                    passage_id: revision.passage_id.clone(),
                    source: revision.source.clone(),
                    prose: base.prose,
                    world,
                })
            })
            .transpose()
    }

    pub(super) fn wording_record(&self, draft: &PassageDraft) -> Result<AcceptedPassage, String> {
        let revision = draft.revision.as_ref().ok_or("Missing wording source.")?;
        let mut record = self.revision_base(revision, true)?;
        if record.prose == draft.edits.prose {
            return Err("Edit the wording before accepting a new version.".into());
        }
        if effects_have_current_quotes(&record.effects, &draft.edits.prose) {
            record.evidence_source = None;
        } else if record.evidence_source.is_none() {
            record.evidence_source = Some(revision.source.clone());
        }
        record.prose = draft.edits.prose.clone();
        record.previous_version = Some(revision.source.clone());
        record.revision_kind = Some(PassageRevisionKind::Wording);
        record.source_story_id = self.view()?.story_id;
        record.narration_provider = draft.narration_provider.clone();
        record.narration_source = draft.narration_source.clone();
        if let Some(source) = &draft.source {
            record.source_fingerprint = source.fingerprint.clone();
        }
        Ok(record)
    }

    pub(super) fn accept_revision(
        &self,
        draft: &mut PassageDraft,
        current: ProjectView,
    ) -> Result<PassageAcceptance, String> {
        let revision = draft.revision.clone().ok_or("Missing revision source.")?;
        let mut index = self.story_index()?;
        let base = self.revision_base(&revision, true)?;
        let (world, index) = if let Some(attempt) = draft
            .attempt
            .as_ref()
            .filter(|a| a.revision == current.revision)
        {
            (
                self.read_record("worlds", &attempt.world)?,
                self.read_record("stories", &attempt.index)?,
            )
        } else {
            let (world, record) = if revision.kind == PassageRevisionKind::Wording {
                (current.world.clone(), self.wording_record(draft)?)
            } else {
                let before: World = self.read_record("worlds", &base.before_world)?;
                let context = scene_world(&before, &draft.edits.plan)?;
                let mut effects: PassageEffects =
                    serde_json::from_str(&draft.edits.effects_text).map_err(|e| e.to_string())?;
                let (world, _) = preview_passage_effects(
                    &before,
                    &context,
                    &draft.edits.prose,
                    &effects,
                    &mut draft.allocations,
                )?;
                for note in &mut effects.notes {
                    for id in &mut note.node_ids {
                        if let Some(resolved) = draft.allocations.get(id) {
                            *id = resolved.clone();
                        }
                    }
                }
                let after = self.write_record("worlds", &world)?;
                let record = AcceptedPassage {
                    previous_version: Some(revision.source.clone()),
                    revision_kind: Some(PassageRevisionKind::Events),
                    evidence_source: None,
                    narration_provider: draft.narration_provider.clone(),
                    effects_provider: draft.effects_provider.clone(),
                    schema_version: 1,
                    id: base.id.clone(),
                    source_story_id: current.story_id.clone(),
                    chapter_id: base.chapter_id.clone(),
                    plan: draft.edits.plan.clone(),
                    prose: draft.edits.prose.clone(),
                    effects,
                    close_chapter: draft.edits.close_chapter,
                    before_world: base.before_world,
                    after_world: after,
                    narration_source: draft.narration_source.clone(),
                    source_fingerprint: draft
                        .source
                        .as_ref()
                        .ok_or("Missing checked source.")?
                        .fingerprint
                        .clone(),
                };
                (world, record)
            };
            let entry = index
                .passages
                .iter_mut()
                .find(|p| p.id == revision.passage_id)
                .ok_or("Revision target missing.")?;
            entry.title = record.plan.title.clone();
            entry.record = self.write_record("passages", &record)?;
            if revision.kind == PassageRevisionKind::Events {
                let chapter = index
                    .chapters
                    .last_mut()
                    .ok_or("Missing replacement chapter.")?;
                chapter.title = record.plan.chapter_title;
                chapter.plan = record.plan.chapter_plan;
                chapter.closed = record.close_chapter;
            }
            index.validate()?;
            let world_ref = self.write_record("worlds", &world)?;
            let index_ref = self.write_record("stories", &index)?;
            draft.attempt = Some(AcceptanceAttempt {
                revision: current.revision,
                title: current.title.clone(),
                world: world_ref,
                index: index_ref,
            });
            draft.version += 1;
            self.write_passage_draft(draft)?;
            (world, index)
        };
        let commit = self.commit_story(
            current.revision,
            draft.id.clone(),
            current.title,
            world,
            index,
        )?;
        draft.stage = PassageStage::Accepted;
        draft.version += 1;
        self.write_passage_draft(draft)?;
        Ok(PassageAcceptance {
            commit,
            passage_id: revision.passage_id,
        })
    }

    pub fn passage_versions(&self, id: &str) -> Result<PassageVersions, String> {
        let current = self.read_passage(id)?;
        let mut next = current.passage.previous_version;
        let mut versions = vec![];
        let mut seen = BTreeSet::new();
        while let Some(reference) = next {
            if versions.len() == 20 {
                return Ok(PassageVersions {
                    versions,
                    more: true,
                });
            }
            if !seen.insert(reference.id.clone()) {
                return Err("Passage version history contains a cycle.".into());
            }
            let record: AcceptedPassage = self.read_record("passages", &reference)?;
            if record.schema_version != 1 || record.id != id {
                return Err("Invalid earlier passage version.".into());
            }
            next = record.previous_version;
            versions.push(EarlierPassageVersion {
                record: reference,
                prose: record.prose,
                revision_kind: record.revision_kind,
            });
        }
        Ok(PassageVersions {
            versions,
            more: false,
        })
    }

    /// A fork starts a fresh revision history. Only an evidence anchor for the retained
    /// events may cross that boundary; superseded alternate events are not copied.
    pub(super) fn copy_passage_for_fork(
        &self,
        fork: &Project,
        mut record: AcceptedPassage,
    ) -> Result<AcceptedPassage, String> {
        if let Some(source) = &record.evidence_source {
            let mut anchor: AcceptedPassage = self.read_record("passages", source)?;
            if anchor.id != record.id
                || anchor.before_world.sha256 != record.before_world.sha256
                || anchor.after_world.sha256 != record.after_world.sha256
                || !effects_have_current_quotes(&record.effects, &anchor.prose)
            {
                return Err(
                    "The retained wording evidence anchor does not match its events.".into(),
                );
            }
            anchor.previous_version = None;
            anchor.evidence_source = None;
            self.copy_passage_worlds(fork, &mut anchor)?;
            record.evidence_source = Some(fork.write_record("passages", &anchor)?);
        }
        record.previous_version = None;
        self.copy_passage_worlds(fork, &mut record)?;
        Ok(record)
    }

    fn copy_passage_worlds(
        &self,
        fork: &Project,
        record: &mut AcceptedPassage,
    ) -> Result<(), String> {
        for reference in [&mut record.before_world, &mut record.after_world] {
            let world: World = self.read_record("worlds", reference)?;
            world.validate()?;
            *reference = fork.write_record("worlds", &world)?;
        }
        if let Some(source) = &record.narration_source {
            let world: World = self.read_record("worlds", source)?;
            world.validate()?;
            record.narration_source = Some(fork.write_record("worlds", &world)?);
        }
        Ok(())
    }
}

pub(super) fn effects_have_current_quotes(effects: &PassageEffects, prose: &str) -> bool {
    let prose = prose.split_whitespace().collect::<Vec<_>>().join(" ");
    effects
        .changes
        .iter()
        .map(|c| match c {
            PassageEffect::PutNode { evidence, .. }
            | PassageEffect::PutLink { evidence, .. }
            | PassageEffect::RemoveLink { evidence, .. } => evidence,
        })
        .chain(effects.notes.iter().map(|n| &n.evidence))
        .all(|quote| prose.contains(&quote.split_whitespace().collect::<Vec<_>>().join(" ")))
}
