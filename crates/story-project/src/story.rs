//! Passage drafts and immutable reading history. Canon changes only at acceptance.
use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use specta::Type;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
mod revisions;
pub use revisions::*;
mod review;
pub use review::*;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PassagePlan {
    pub chapter_title: String,
    pub chapter_plan: String,
    pub title: String,
    pub direction: String,
    pub viewpoint_id: Option<String>,
    pub location_id: Option<String>,
    pub participant_ids: Vec<String>,
    pub reference_ids: Vec<String>,
    pub target_words: u32,
}
impl Default for PassagePlan {
    fn default() -> Self {
        Self {
            chapter_title: "Chapter 1".into(),
            chapter_plan: String::new(),
            title: "Passage 1".into(),
            direction: String::new(),
            viewpoint_id: None,
            location_id: None,
            participant_ids: vec![],
            reference_ids: vec![],
            target_words: 450,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PassageEdits {
    pub plan: PassagePlan,
    pub prose: String,
    pub effects_text: String,
    pub close_chapter: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub approved_effects: Vec<String>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PassageStage {
    Draft,
    Narrating,
    NeedsCheck,
    Checking,
    Ready,
    Failed,
    Interrupted,
    Cancelled,
    Accepted,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Chapter {
    pub id: String,
    pub title: String,
    pub plan: String,
    pub closed: bool,
    pub passage_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PassageEntry {
    pub id: String,
    pub chapter_id: String,
    pub title: String,
    pub record: RecordRef,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoryIndex {
    pub schema_version: u32,
    pub chapters: Vec<Chapter>,
    pub passages: Vec<PassageEntry>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub memories: BTreeMap<String, RecordRef>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub media: BTreeMap<String, RecordRef>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub portraits: BTreeMap<String, String>,
}
impl Default for StoryIndex {
    fn default() -> Self {
        Self {
            schema_version: 1,
            chapters: vec![],
            passages: vec![],
            memories: BTreeMap::new(),
            media: BTreeMap::new(),
            portraits: BTreeMap::new(),
        }
    }
}
impl StoryIndex {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 || self.passages.len() > 5000 || self.chapters.len() > 1000 {
            return Err("Unsupported or oversized story history.".into());
        }
        let mut ids = BTreeSet::new();
        let mut ordered = vec![];
        for (i, ch) in self.chapters.iter().enumerate() {
            validate_id(&ch.id)?;
            validate_name(&ch.title)?;
            if !ids.insert(ch.id.clone())
                || ch.passage_ids.is_empty()
                || ch.plan.len() > 16000
                || (!ch.closed && i + 1 != self.chapters.len())
            {
                return Err("Invalid chapter history.".into());
            }
            ordered.extend(ch.passage_ids.iter());
        }
        if ordered != self.passages.iter().map(|p| &p.id).collect::<Vec<_>>() {
            return Err("Chapter contents do not match passage order.".into());
        }
        for p in &self.passages {
            validate_id(&p.id)?;
            validate_name(&p.title)?;
            validate_id(&p.record.id)?;
            validate_id(&p.chapter_id)?;
            if p.record.sha256.len() != 64
                || !p.record.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err("Invalid passage record checksum.".into());
            }
            if !ids.insert(p.id.clone())
                || !self
                    .chapters
                    .iter()
                    .any(|ch| ch.id == p.chapter_id && ch.passage_ids.contains(&p.id))
            {
                return Err("Invalid passage index.".into());
            }
        }
        for (chapter, reference) in &self.memories {
            if !self.chapters.iter().any(|c| &c.id == chapter)
                || reference.sha256.len() != 64
                || !reference.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err("Invalid chapter memory reference.".into());
            }
            validate_id(&reference.id)?;
        }
        if self.media.len() > 2000 {
            return Err("Accepted image limit reached (2,000).".into());
        }
        for (id, reference) in &self.media {
            validate_id(id)?;
            validate_id(&reference.id)?;
            if reference.sha256.len() != 64
                || !reference.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err("Invalid media reference.".into());
            }
        }
        for (node, asset) in &self.portraits {
            validate_id(node)?;
            if !self.media.contains_key(asset) {
                return Err("Selected portrait is not in the accepted media index.".into());
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContinuityNote {
    pub text: String,
    pub evidence: String,
    pub node_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcceptedPassage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_version: Option<RecordRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision_kind: Option<PassageRevisionKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_source: Option<RecordRef>,
    #[serde(default)]
    pub narration_provider: String,
    #[serde(default)]
    pub effects_provider: String,
    pub schema_version: u32,
    pub id: String,
    pub source_story_id: String,
    pub chapter_id: String,
    pub plan: PassagePlan,
    pub prose: String,
    pub effects: PassageEffects,
    pub close_chapter: bool,
    pub before_world: RecordRef,
    pub after_world: RecordRef,
    pub narration_source: Option<RecordRef>,
    pub source_fingerprint: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PassageReading {
    pub record: RecordRef,
    pub replacement_error: String,
    pub evidence_prose: Option<String>,
    pub passage: AcceptedPassage,
    pub before: World,
    pub after: World,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PassageSummary {
    pub revision_kind: Option<PassageRevisionKind>,
    pub id: String,
    pub title: String,
    pub stage: PassageStage,
    pub version: u32,
    pub created_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PassageView {
    pub effects_review: EffectsReview,
    pub memory: MemoryContext,
    pub revision: Option<PassageRevisionView>,
    pub narration_provider: String,
    pub effects_provider: String,
    pub id: String,
    pub version: u32,
    pub stage: PassageStage,
    pub edits: PassageEdits,
    pub error: String,
    pub stale: bool,
    pub changes: Vec<String>,
    pub can_accept: bool,
    pub suggested_chapter_end: bool,
    pub context_nodes: Vec<String>,
    pub recent_passages: Vec<String>,
    pub original_prose: String,
    pub original_effects: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PassageAcceptance {
    pub commit: CommitResult,
    pub passage_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PassageSource {
    world: RecordRef,
    fingerprint: String,
    #[serde(default = "default_memory_budget")]
    memory_budget: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AcceptanceAttempt {
    revision: u32,
    title: String,
    world: RecordRef,
    index: RecordRef,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PassageDraft {
    #[serde(default)]
    revision: Option<PassageRevision>,
    #[serde(default)]
    cloned_from: Option<(String, u32)>,
    #[serde(default)]
    narration_provider: String,
    #[serde(default)]
    effects_provider: String,
    schema_version: u32,
    id: String,
    story_id: String,
    version: u32,
    created_at: u64,
    stage: PassageStage,
    edits: PassageEdits,
    source: Option<PassageSource>,
    narration_source: Option<RecordRef>,
    checked: String,
    error: String,
    original_prose: String,
    original_effects: String,
    allocations: BTreeMap<String, String>,
    attempt: Option<AcceptanceAttempt>,
    incomplete: bool,
}
pub struct PreparedPassage {
    pub system: String,
    pub user: String,
    pub schema: Option<Value>,
    pub output_tokens: u32,
    pub demo: String,
}
pub(crate) fn story_hash<T: Serialize>(value: &T) -> Result<String, String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).map_err(|e| e.to_string())?)
    ))
}
fn validate_edits(edits: &PassageEdits) -> Result<(), String> {
    if edits.approved_effects.len() > 92
        || edits
            .approved_effects
            .iter()
            .any(|key| key.len() != 64 || !key.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return Err("Invalid effect review approvals.".into());
    }
    let p = &edits.plan;
    if p.chapter_title.chars().count() > 200 || p.title.chars().count() > 200 {
        return Err("Passage/chapter titles exceed 200 characters.".into());
    }
    if p.chapter_plan.chars().count() > 4000
        || p.direction.chars().count() > 8000
        || p.target_words > 10000
        || p.participant_ids.len() > 12
        || p.reference_ids.len() > 12
        || edits.prose.len() > 100000
        || edits.effects_text.len() > 500000
    {
        return Err(
            "Narrow the plan, prose or selected references to their supported limits.".into(),
        );
    }
    for id in p
        .viewpoint_id
        .iter()
        .chain(p.location_id.iter())
        .chain(&p.participant_ids)
        .chain(&p.reference_ids)
    {
        validate_id(id)?;
    }
    Ok(())
}
pub(crate) fn scene_world(world: &World, plan: &PassagePlan) -> Result<World, String> {
    world.validate()?;
    let mut ids: BTreeSet<String> = plan
        .viewpoint_id
        .iter()
        .chain(plan.location_id.iter())
        .chain(&plan.participant_ids)
        .chain(&plan.reference_ids)
        .cloned()
        .collect();
    for id in &ids {
        if !world.nodes.iter().any(|n| &n.id == id) {
            return Err("A selected scene resource no longer exists.".into());
        }
    }
    let actors: BTreeSet<_> = plan
        .viewpoint_id
        .iter()
        .chain(&plan.participant_ids)
        .collect();
    for e in &world.edges {
        if (e.type_id == "core.stored-in" || e.type_id == "core.worn-by")
            && actors.contains(&e.target_id)
        {
            ids.insert(e.source_id.clone());
        }
    }
    let physical: BTreeSet<_> = world
        .relation_types
        .iter()
        .filter(|r| r.physical_placement)
        .map(|r| &r.id)
        .collect();
    loop {
        let before = ids.len();
        for e in &world.edges {
            if ids.contains(&e.source_id) && physical.contains(&e.type_id) {
                ids.insert(e.target_id.clone());
            }
        }
        if ids.len() == before {
            break;
        }
        if ids.len() > 80 {
            return Err(
                "The scene requires too many placement ancestors; narrow its resources.".into(),
            );
        }
    }
    if ids.len() > 80 {
        return Err(
            "Select fewer scene resources (limit 80 including inventory and ancestors).".into(),
        );
    }
    let mut context = world.clone();
    context.ai_context.clear();
    context.nodes.retain(|n| ids.contains(&n.id));
    context
        .edges
        .retain(|e| ids.contains(&e.source_id) && ids.contains(&e.target_id));
    Ok(context)
}
impl Project {
    pub fn story_index(&self) -> Result<StoryIndex, String> {
        let v = self.view()?;
        let index = if let Some(r) = v.story_ref {
            self.read_record("stories", &r)?
        } else {
            StoryIndex::default()
        };
        index.validate()?;
        Ok(index)
    }
    pub fn read_passage(&self, id: &str) -> Result<PassageReading, String> {
        validate_id(id)?;
        let index = self.story_index()?;
        let entry = index
            .passages
            .iter()
            .find(|p| p.id == id)
            .ok_or("That passage is not in this story.")?;
        let passage: AcceptedPassage = self.read_record("passages", &entry.record)?;
        if passage.schema_version != 1
            || passage.id != entry.id
            || passage.chapter_id != entry.chapter_id
        {
            return Err("Invalid accepted passage record.".into());
        }
        let before: World = self.read_record("worlds", &passage.before_world)?;
        let after: World = self.read_record("worlds", &passage.after_world)?;
        before.validate()?;
        after.validate()?;
        let evidence_prose = if let Some(source) = &passage.evidence_source {
            let anchor: AcceptedPassage = self.read_record("passages", source)?;
            if anchor.id != passage.id
                || anchor.before_world.sha256 != passage.before_world.sha256
                || anchor.after_world.sha256 != passage.after_world.sha256
                || !effects_have_current_quotes(&passage.effects, &anchor.prose)
            {
                return Err("Passage evidence does not match its retained events.".into());
            }
            Some(anchor.prose)
        } else {
            None
        };
        let current = self.view()?;
        let replacement_error = if index.passages.last().is_none_or(|p| p.id != id) {
            "Earlier event changes need a fork.".into()
        } else if current.world_ref.sha256 != passage.after_world.sha256 {
            "The world was edited after this passage. Fork after it to preserve those edits in the original before replacing events in the copy.".into()
        } else {
            String::new()
        };
        Ok(PassageReading {
            record: entry.record.clone(),
            replacement_error,
            evidence_prose,
            passage,
            before,
            after,
        })
    }
    pub fn next_passage_plan(&self) -> Result<PassagePlan, String> {
        let index = self.story_index()?;
        let active = index.chapters.last().filter(|ch| !ch.closed);
        Ok(PassagePlan {
            chapter_title: active
                .map(|c| c.title.clone())
                .unwrap_or_else(|| format!("Chapter {}", index.chapters.len() + 1)),
            chapter_plan: active.map(|c| c.plan.clone()).unwrap_or_default(),
            title: format!("Passage {}", index.passages.len() + 1),
            ..Default::default()
        })
    }
    fn passage_path(&self, id: &str) -> Result<std::path::PathBuf, String> {
        validate_id(id)?;
        Ok(self
            .root_path()
            .join("passage-drafts")
            .join(format!("{id}.json")))
    }
    fn write_passage_draft(&self, draft: &PassageDraft) -> Result<(), String> {
        fs::create_dir_all(self.root_path().join("passage-drafts")).map_err(|e| e.to_string())?;
        let bytes = serde_json::to_vec_pretty(draft).map_err(|e| e.to_string())?;
        if bytes.len() > 2_000_000 {
            return Err("Passage draft exceeds 2 MB.".into());
        }
        crate::store::atomic_write(&self.passage_path(&draft.id)?, &bytes)
    }
    fn read_passage_draft(&self, id: &str) -> Result<PassageDraft, String> {
        let mut d: PassageDraft =
            serde_json::from_slice(&crate::store::read_bytes(&self.passage_path(id)?)?)
                .map_err(|e| format!("Invalid passage draft; it was kept: {e}"))?;
        if d.schema_version != 1 || d.id != id || d.story_id != self.view()?.story_id {
            return Err("The passage draft belongs to another story/version.".into());
        }
        if d.stage != PassageStage::Accepted && self.passage_committed(&d)?.is_some() {
            d.stage = PassageStage::Accepted;
            d.version += 1;
            d.error.clear();
            self.write_passage_draft(&d)?;
        }
        Ok(d)
    }
    fn archive_passage(&self, d: &PassageDraft) -> Result<(), String> {
        self.write_record("passage-draft-versions", d).map(|_| ())
    }
    pub fn list_passage_drafts(&self) -> Result<Vec<PassageSummary>, String> {
        let path = self.root_path().join("passage-drafts");
        if !path.exists() {
            return Ok(vec![]);
        }
        let mut list = vec![];
        let accepted: BTreeSet<_> = self
            .story_index()?
            .passages
            .into_iter()
            .map(|p| p.id)
            .collect();
        for e in fs::read_dir(path).map_err(|e| e.to_string())? {
            let path = e.map_err(|e| e.to_string())?.path();
            if path.extension().is_some_and(|e| e == "json") {
                let id = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or("Invalid passage draft filename.")?;
                if accepted.contains(id) {
                    continue;
                }
                let d = self.read_passage_draft(id)?;
                if d.stage == PassageStage::Accepted {
                    continue;
                }
                list.push(PassageSummary {
                    revision_kind: d.revision.as_ref().map(|r| r.kind),
                    id: d.id,
                    title: d.edits.plan.title,
                    stage: d.stage,
                    version: d.version,
                    created_at: d.created_at.to_string(),
                });
                if list.len() > 200 {
                    return Err("The prototype supports 200 passage drafts per story.".into());
                }
            }
        }
        list.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| b.id.cmp(&a.id))
        });
        Ok(list)
    }
    pub fn save_passage(&self, id: &str, expected: u32, edits: PassageEdits) -> Result<(), String> {
        validate_edits(&edits)?;
        let path = self.passage_path(id)?;
        let mut d = if path.exists() {
            let d = self.read_passage_draft(id)?;
            if d.version == expected + 1 && d.edits == edits {
                return Ok(());
            }
            if d.version != expected
                || matches!(
                    d.stage,
                    PassageStage::Narrating | PassageStage::Checking | PassageStage::Accepted
                )
            {
                return Err("This passage changed or is running/accepted. Reload or create a new candidate.".into());
            }
            d
        } else {
            if expected != 0 || self.list_passage_drafts()?.len() >= 200 {
                return Err("Passage draft limit reached or version changed.".into());
            }
            PassageDraft {
                revision: None,
                cloned_from: None,
                narration_provider: String::new(),
                effects_provider: String::new(),
                schema_version: 1,
                id: id.into(),
                story_id: self.view()?.story_id,
                version: 0,
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|_| "Clock unavailable.")?
                    .as_millis() as u64,
                stage: PassageStage::Draft,
                edits: edits.clone(),
                source: None,
                narration_source: None,
                checked: String::new(),
                error: String::new(),
                original_prose: String::new(),
                original_effects: String::new(),
                allocations: BTreeMap::new(),
                attempt: None,
                incomplete: false,
            }
        };
        if d.edits.plan != edits.plan
            || d.edits.prose != edits.prose
            || d.edits.effects_text != edits.effects_text
        {
            d.checked.clear();
            d.stage = if edits.prose.trim().is_empty() {
                PassageStage::Draft
            } else {
                PassageStage::NeedsCheck
            };
            d.attempt = None;
        }
        if d.edits.close_chapter != edits.close_chapter
            || d.edits.approved_effects != edits.approved_effects
        {
            d.attempt = None;
        }
        if d.version > 0 {
            self.archive_passage(&d)?;
        }
        d.edits = edits;
        d.version += 1;
        d.error.clear();
        self.write_passage_draft(&d)
    }
    fn story_input(
        &self,
        world: &World,
        plan: &PassagePlan,
        settings: &LlmConnection,
        memory_budget: u32,
    ) -> Result<(World, Value, String, Vec<String>), String> {
        let index = self.story_index()?;
        self.story_input_at(world, plan, settings, &index, memory_budget)
    }
    fn story_input_at(
        &self,
        world: &World,
        plan: &PassagePlan,
        settings: &LlmConnection,
        index: &StoryIndex,
        memory_budget: u32,
    ) -> Result<(World, Value, String, Vec<String>), String> {
        let context = scene_world(world, plan)?;
        let mut recent = vec![];
        let mut ids = vec![];
        for entry in index
            .passages
            .iter()
            .rev()
            .take(2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            let passage: AcceptedPassage = self.read_record("passages", &entry.record)?;
            ids.push(entry.id.clone());
            recent.push(json!({"id":entry.id,"version":entry.record,"title":passage.plan.title,"prose":passage.prose,"notes":passage.effects.notes,"evidenceFromEarlierWording":passage.evidence_source.is_some()}));
        }
        let current_chapter = index
            .chapters
            .last()
            .map(|ch| json!({"id":ch.id,"title":ch.title,"plan":ch.plan,"closed":ch.closed}));
        let memory = self.memory_context(index, plan, &context, memory_budget)?;
        let input = json!({"storyTitle":self.view()?.title,"plan":plan,"currentChapter":current_chapter,"records":context.nodes,"links":context.edges,"recentPassages":recent,"earlierMemory":memory});
        let mut fingerprint = story_hash(&(
            "passage-source-v4-optional-direction",
            &input,
            &context.node_types,
            &context.component_types,
            &context.relation_types,
            settings,
        ))?;
        if let Some(included) = story_ai_context_stamp(world)? {
            fingerprint = story_hash(&(&fingerprint, included))?;
        }
        Ok((context, input, fingerprint, ids))
    }
    pub fn start_passage(
        &self,
        id: &str,
        expected: u32,
        revision: u32,
        checking: bool,
        settings: &LlmConnection,
    ) -> Result<PreparedPassage, String> {
        settings.ready()?;
        let mut d = self.read_passage_draft(id)?;
        validate_name(&d.edits.plan.chapter_title)?;
        validate_name(&d.edits.plan.title)?;
        if !(100..=1500).contains(&d.edits.plan.target_words) {
            return Err("Choose a target of 100–1,500 words.".into());
        }
        let current = self.view()?;
        if d.version != expected
            || current.revision != revision
            || matches!(
                d.stage,
                PassageStage::Narrating | PassageStage::Checking | PassageStage::Accepted
            )
        {
            return Err("Save the latest world and passage before starting.".into());
        }
        if checking && d.edits.prose.trim().is_empty() {
            return Err("Write or generate prose before checking its effects.".into());
        }
        let wording = d
            .revision
            .as_ref()
            .is_some_and(|r| r.kind == PassageRevisionKind::Wording);
        if !checking && wording && d.edits.plan.direction.trim().is_empty() {
            return Err("Add wording direction before generating a polish candidate.".into());
        }
        if checking && wording {
            return Err("Wording revisions keep their accepted events. Validate wording after reviewing the text instead.".into());
        }
        let (context, source_world, mut input, mut fingerprint, _) =
            self.draft_input_budget(&d, settings, 0)?;
        let task = if checking {
            TextPromptTask::Effects
        } else if wording {
            TextPromptTask::Wording
        } else {
            TextPromptTask::Passage
        };
        let mut system = settings.system_prompt(task);
        if checking {
            input["prose"] = json!(d.edits.prose);
            input["nodeTypes"] = json!(context.node_types);
            input["capabilities"] = json!(context.component_types);
            input["relationships"] = json!(context.relation_types);
        }
        let source_world_data: World = self.read_record("worlds", &source_world)?;
        let included = resolve_ai_context(&source_world_data, task)?;
        append_ai_context(&included, &mut system, &mut input);
        let schema = checking.then(effects_schema);
        if settings.structured_output != StructuredOutput::Schema && checking {
            input["responseSchema"] = schema.clone().unwrap();
        }
        let user = serde_json::to_string(&input).map_err(|e| e.to_string())?;
        let extra = if settings.structured_output == StructuredOutput::Schema {
            schema.as_ref().map_or(0, |s| s.to_string().len())
        } else {
            0
        };
        let output_tokens = if checking {
            settings.max_tokens
        } else {
            settings.max_tokens.min(d.edits.plan.target_words * 3 + 512)
        };
        let available_bytes = (settings.context_tokens.saturating_sub(output_tokens + 256)
            as usize)
            .saturating_mul(3);
        let memory_budget = available_bytes
            .saturating_sub(system.len() + user.len() + extra + 128)
            .min(default_memory_budget() as usize) as u32;
        if memory_budget > 0 {
            let (_, _, expanded, expanded_fingerprint, _) =
                self.draft_input_budget(&d, settings, memory_budget)?;
            for (key, value) in expanded.as_object().ok_or("Invalid passage context.")? {
                input[key] = value.clone();
            }
            fingerprint = expanded_fingerprint;
        }
        let user = serde_json::to_string(&input).map_err(|e| e.to_string())?;
        if (system.len() + user.len() + extra) / 3 + 256 + output_tokens as usize
            > settings.context_tokens as usize
        {
            return Err(format!("Required scene context and output exceed the configured budget. Included AI context: {}. Shorten included nodes, narrow references or shorten the passage target; no required content was dropped.",included.iter().map(|s|s.name.as_str()).collect::<Vec<_>>().join(", ")));
        }
        self.archive_passage(&d)?;
        d.source = Some(PassageSource {
            world: source_world.clone(),
            fingerprint,
            memory_budget,
        });
        if !checking {
            d.narration_source = Some(source_world);
        }
        d.checked.clear();
        d.error.clear();
        d.attempt = None;
        d.incomplete = false;
        d.stage = if checking {
            PassageStage::Checking
        } else {
            PassageStage::Narrating
        };
        let label = if settings.provider == TextProvider::Demo {
            "Offline demo · no model request".into()
        } else {
            settings.model.clone()
        };
        if checking {
            d.effects_provider = label;
        } else {
            d.narration_provider = label;
        }
        d.version += 1;
        self.write_passage_draft(&d)?;
        let demo = if checking {
            serde_json::to_string(&PassageEffects::default()).unwrap()
        } else if wording {
            input["originalProse"]
                .as_str()
                .unwrap_or_default()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            format!("The room fell quiet. {} paused to consider what came next.\n\n{}\n\nFor the moment, nothing in the world had changed.",context.nodes.iter().find(|n|Some(&n.id)==d.edits.plan.viewpoint_id.as_ref()).map(|n|n.name.as_str()).unwrap_or("The visitor"),d.edits.plan.direction)
        };
        Ok(PreparedPassage {
            system,
            user,
            schema,
            output_tokens,
            demo,
        })
    }
    pub fn finish_passage(
        &self,
        id: &str,
        checking: bool,
        result: Result<String, String>,
        cancelled: bool,
        incomplete: bool,
        settings: &LlmConnection,
    ) -> Result<(), String> {
        let mut d = self.read_passage_draft(id)?;
        if d.stage
            != if checking {
                PassageStage::Checking
            } else {
                PassageStage::Narrating
            }
        {
            return Err("The passage request is no longer running.".into());
        }
        match result {
            Ok(text) => {
                if text.len() > if checking { 500000 } else { 100000 } {
                    return Err("Passage response exceeds its saved limit.".into());
                }
                if checking {
                    d.edits.approved_effects.clear();
                    d.original_effects = text.clone();
                    d.edits.effects_text = text;
                } else {
                    d.edits.approved_effects.clear();
                    d.original_prose = text.clone();
                    d.edits.prose = text;
                    if d.revision
                        .as_ref()
                        .is_none_or(|r| r.kind != PassageRevisionKind::Wording)
                    {
                        d.edits.effects_text = empty_effects();
                    }
                }
                d.incomplete = incomplete;
                d.stage = PassageStage::NeedsCheck;
                d.error = if incomplete {
                    "The response was cut off. Its retained text requires editing/rechecking."
                        .into()
                } else {
                    String::new()
                };
            }
            Err(error) => {
                d.error = error;
                d.stage = if cancelled {
                    PassageStage::Cancelled
                } else {
                    PassageStage::Failed
                };
            }
        }
        d.version += 1;
        self.write_passage_draft(&d)?;
        if checking && d.stage == PassageStage::NeedsCheck && !incomplete {
            let matching = self
                .draft_input(&d, settings)
                .is_ok_and(|(_, _, _, fp, _)| {
                    d.source.as_ref().is_some_and(|s| s.fingerprint == fp)
                });
            let result = if matching {
                self.validate_passage(id, d.version, settings)
            } else {
                Err("The source changed during checking. Recheck against the current world before accepting.".into())
            };
            if let Err(error) = result {
                let mut d = self.read_passage_draft(id)?;
                d.error = error;
                d.version += 1;
                self.write_passage_draft(&d)?;
            }
        }
        Ok(())
    }
    pub fn validate_passage(
        &self,
        id: &str,
        expected: u32,
        settings: &LlmConnection,
    ) -> Result<(), String> {
        let mut d = self.read_passage_draft(id)?;
        validate_name(&d.edits.plan.chapter_title)?;
        validate_name(&d.edits.plan.title)?;
        if !(100..=1500).contains(&d.edits.plan.target_words) {
            return Err("Choose a target of 100–1,500 words.".into());
        }
        if d.version != expected
            || matches!(
                d.stage,
                PassageStage::Narrating | PassageStage::Checking | PassageStage::Accepted
            )
        {
            return Err("Save/reload the passage before validating.".into());
        }
        if d.edits.prose.trim().is_empty() {
            return Err("A passage needs prose before acceptance.".into());
        }
        let (context, source_world, _, fingerprint, _) = self.draft_input(&d, settings)?;
        if d.revision
            .as_ref()
            .is_some_and(|r| r.kind == PassageRevisionKind::Wording)
        {
            self.wording_record(&d)?;
        } else {
            let world: World = self.read_record("worlds", &source_world)?;
            let effects: PassageEffects = serde_json::from_str(&d.edits.effects_text)
                .map_err(|e| format!("Effects JSON needs editing: {e}"))?;
            preview_passage_effects(
                &world,
                &context,
                &d.edits.prose,
                &effects,
                &mut d.allocations,
            )?;
        }
        d.source = Some(PassageSource {
            world: source_world,
            fingerprint: fingerprint.clone(),
            memory_budget: d
                .source
                .as_ref()
                .map_or(default_memory_budget(), |s| s.memory_budget),
        });
        d.checked = story_hash(&(
            &d.edits.plan,
            &d.edits.prose,
            &d.edits.effects_text,
            &fingerprint,
        ))?;
        d.stage = PassageStage::Ready;
        d.incomplete = false;
        d.error.clear();
        d.attempt = None;
        d.version += 1;
        self.write_passage_draft(&d)
    }
    pub fn passage_view(&self, id: &str, settings: &LlmConnection) -> Result<PassageView, String> {
        let mut d = self.read_passage_draft(id)?;
        let current = self.view()?;
        let revision = self.revision_view(&d)?;
        let wording = d
            .revision
            .as_ref()
            .is_some_and(|r| r.kind == PassageRevisionKind::Wording);
        let prepared = self.draft_input(&d, settings);
        if d.revision.is_some() && d.stage != PassageStage::Accepted {
            if let Err(error) = &prepared {
                d.error = error.clone();
            }
        }
        let stale = d.source.as_ref().is_some_and(|source| {
            prepared
                .as_ref()
                .map_or(true, |(_, _, _, fp, _)| fp != &source.fingerprint)
        });
        let mut changes = vec![];
        let mut suggested = false;
        let mut review_issues = vec![];
        if wording {
            changes.push("Wording only: accepted events, chapter membership and world snapshots stay unchanged.".into());
        } else if !d.edits.prose.trim().is_empty() {
            match serde_json::from_str::<PassageEffects>(&d.edits.effects_text)
                .map_err(|e| format!("Effects JSON needs editing: {e}"))
                .and_then(|effects| {
                    suggested = effects.suggest_chapter_end;
                    let source = if let Some(s) = &d.source {
                        self.read_record("worlds", &s.world)?
                    } else {
                        if let Some(revision) = &revision {
                            revision.world.clone()
                        } else {
                            current.world.clone()
                        }
                    };
                    let context = scene_world(&source, &d.edits.plan)?;
                    review_issues = effect_record_issues(&source, &context, &effects);
                    preview_passage_effects(
                        &source,
                        &context,
                        &d.edits.prose,
                        &effects,
                        &mut d.allocations,
                    )
                }) {
                Ok((_, labels)) => changes = labels,
                Err(error) => {
                    if d.error.is_empty() {
                        d.error = error;
                    }
                }
            }
        }
        let matching = prepared.as_ref().is_ok_and(|(_, _, _, fp, _)| {
            story_hash(&(&d.edits.plan, &d.edits.prose, &d.edits.effects_text, fp))
                .is_ok_and(|hash| hash == d.checked)
        });
        let checked = d.stage == PassageStage::Ready && matching && !stale && d.error.is_empty();
        let effects_review = if wording {
            EffectsReview {
                checked,
                entries: vec![],
            }
        } else {
            review::effect_review(
                &d.edits,
                prepared
                    .as_ref()
                    .map(|(ctx, _, _, fp, _)| (ctx, fp.as_str()))
                    .ok(),
                checked,
                &review_issues,
            )?
        };
        let approved = effects_review.entries.iter().all(|entry| entry.approved);
        let memory = prepared
            .as_ref()
            .ok()
            .and_then(|(_, _, input, _, _)| {
                serde_json::from_value(input["earlierMemory"].clone()).ok()
            })
            .unwrap_or_default();
        let (context_nodes, recent_passages) = prepared
            .map(|(ctx, _, _, _, recent)| (ctx.nodes.into_iter().map(|n| n.name).collect(), recent))
            .unwrap_or_default();
        Ok(PassageView {
            effects_review,
            memory,
            revision,
            narration_provider: d.narration_provider,
            effects_provider: d.effects_provider,
            id: d.id,
            version: d.version,
            stage: d.stage,
            edits: d.edits,
            error: d.error.clone(),
            stale,
            changes,
            can_accept: checked && approved,
            suggested_chapter_end: suggested,
            context_nodes,
            recent_passages,
            original_prose: d.original_prose,
            original_effects: d.original_effects,
        })
    }
    pub fn accept_passage(
        &self,
        id: &str,
        expected: u32,
        settings: &LlmConnection,
    ) -> Result<PassageAcceptance, String> {
        let mut d = self.read_passage_draft(id)?;
        let current = self.view()?;
        let mut index = self.story_index()?;
        if let Some(revision) = self.passage_committed(&d)? {
            return Ok(PassageAcceptance {
                commit: CommitResult {
                    project: current,
                    committed_revision: revision,
                    replayed: true,
                },
                passage_id: d
                    .revision
                    .as_ref()
                    .map_or_else(|| id.into(), |r| r.passage_id.clone()),
            });
        }
        if d.version != expected {
            return Err(
                "The exact passage/effects/source must be checked before acceptance.".into(),
            );
        }
        let review = self.passage_view(id, settings)?;
        if !review.can_accept {
            if review.effects_review.checked
                && review.effects_review.entries.iter().any(|e| !e.approved)
            {
                return Err("Approve each included world change and continuity note before accepting this passage.".into());
            }
            return Err(
                "The exact passage/effects/source must be checked before acceptance.".into(),
            );
        }
        if d.revision.is_some() {
            return self.accept_revision(&mut d, current);
        }
        let mut effects: PassageEffects =
            serde_json::from_str(&d.edits.effects_text).map_err(|e| e.to_string())?;
        let context = scene_world(&current.world, &d.edits.plan)?;
        let (world, _) = preview_passage_effects(
            &current.world,
            &context,
            &d.edits.prose,
            &effects,
            &mut d.allocations,
        )?;
        for note in &mut effects.notes {
            for id in &mut note.node_ids {
                if let Some(resolved) = d.allocations.get(id) {
                    *id = resolved.clone();
                }
            }
        }
        let active = index.chapters.last_mut().filter(|ch| !ch.closed);
        let chapter_id = if let Some(ch) = active {
            ch.title = d.edits.plan.chapter_title.clone();
            ch.plan = d.edits.plan.chapter_plan.clone();
            ch.closed = d.edits.close_chapter;
            ch.passage_ids.push(id.into());
            ch.id.clone()
        } else {
            let ch = Chapter {
                id: uuid::Uuid::new_v4().to_string(),
                title: d.edits.plan.chapter_title.clone(),
                plan: d.edits.plan.chapter_plan.clone(),
                closed: d.edits.close_chapter,
                passage_ids: vec![id.into()],
            };
            let id = ch.id.clone();
            index.chapters.push(ch);
            id
        };
        let (world, index) = if let Some(attempt) = &d.attempt {
            if attempt.revision == current.revision {
                (
                    self.read_record("worlds", &attempt.world)?,
                    self.read_record("stories", &attempt.index)?,
                )
            } else {
                d.attempt = None;
                (world, index)
            }
        } else {
            (world, index)
        };
        let (world, index) = if d.attempt.is_some() {
            (world, index)
        } else {
            let after = self.write_record("worlds", &world)?;
            let passage = AcceptedPassage {
                previous_version: None,
                revision_kind: None,
                evidence_source: None,
                narration_provider: d.narration_provider.clone(),
                effects_provider: d.effects_provider.clone(),
                schema_version: 1,
                id: id.into(),
                source_story_id: current.story_id.clone(),
                chapter_id: chapter_id.clone(),
                plan: d.edits.plan.clone(),
                prose: d.edits.prose.clone(),
                effects,
                close_chapter: d.edits.close_chapter,
                before_world: current.world_ref.clone(),
                after_world: after.clone(),
                narration_source: d.narration_source.clone(),
                source_fingerprint: d
                    .source
                    .as_ref()
                    .ok_or("Missing checked source.")?
                    .fingerprint
                    .clone(),
            };
            let record = self.write_record("passages", &passage)?;
            let mut index = index;
            index.passages.push(PassageEntry {
                id: id.into(),
                chapter_id,
                title: d.edits.plan.title.clone(),
                record,
            });
            index.validate()?;
            let index_ref = self.write_record("stories", &index)?;
            d.attempt = Some(AcceptanceAttempt {
                revision: current.revision,
                title: current.title.clone(),
                world: after,
                index: index_ref,
            });
            d.version += 1;
            self.write_passage_draft(&d)?;
            (world, index)
        };
        let commit = self.commit_story(current.revision, id.into(), current.title, world, index)?;
        d.stage = PassageStage::Accepted;
        d.version += 1;
        self.write_passage_draft(&d)?;
        Ok(PassageAcceptance {
            commit,
            passage_id: id.into(),
        })
    }
    pub fn interrupt_passages(&self) -> Result<(), String> {
        let path = self.root_path().join("passage-drafts");
        if !path.exists() {
            return Ok(());
        }
        let accepted: BTreeSet<_> = self
            .story_index()?
            .passages
            .into_iter()
            .map(|p| p.id)
            .collect();
        let mut errors = vec![];
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let result = (|| {
                let path = entry.map_err(|e| e.to_string())?.path();
                if !path.extension().is_some_and(|e| e == "json") {
                    return Ok(());
                }
                let id = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or("Invalid passage draft filename.")?;
                if accepted.contains(id) {
                    return Ok(());
                }
                let mut d = self.read_passage_draft(id)?;
                if matches!(d.stage, PassageStage::Narrating | PassageStage::Checking) {
                    d.stage = PassageStage::Interrupted;
                    d.checked.clear();
                    d.error="The request was interrupted and was not resent. Retained prose and effects were kept.".into();
                    d.version += 1;
                    self.write_passage_draft(&d)?;
                }
                Ok::<(), String>(())
            })();
            if let Err(error) = result {
                if errors.len() < 5 {
                    errors.push(error);
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join(" "))
        }
    }
    pub fn fork_after_passage(
        &self,
        id: &str,
        parent: &Path,
        title: String,
    ) -> Result<Project, String> {
        validate_name(&title)?;
        let parent = parent.canonicalize().map_err(|e| e.to_string())?;
        if !parent.is_dir() {
            return Err("Choose a destination parent folder.".into());
        }
        let index = self.story_index()?;
        let end = index
            .passages
            .iter()
            .position(|p| p.id == id)
            .ok_or("Choose an accepted passage to fork after.")?;
        let boundary = self.read_passage(id)?.after;
        let operation = uuid::Uuid::new_v4().to_string();
        let staging = parent.join(format!(".guided-story-fork-{operation}.incomplete"));
        let destination = parent.join(format!("guided-story-{}", &operation[..8]));
        if !staging.starts_with(&parent) || !destination.starts_with(&parent) {
            return Err("Invalid fork destination.".into());
        }
        let fork = Project::create(
            &staging,
            title.clone(),
            boundary.clone(),
            uuid::Uuid::new_v4().to_string(),
        )?;
        let mut kept = StoryIndex::default();
        for entry in index.passages.iter().take(end + 1) {
            let record =
                self.copy_passage_for_fork(&fork, self.read_passage(&entry.id)?.passage)?;
            if kept
                .chapters
                .last()
                .is_none_or(|ch| ch.id != record.chapter_id)
            {
                kept.chapters.push(Chapter {
                    id: record.chapter_id.clone(),
                    title: record.plan.chapter_title.clone(),
                    plan: record.plan.chapter_plan.clone(),
                    closed: false,
                    passage_ids: vec![],
                });
            }
            let ch = kept.chapters.last_mut().unwrap();
            ch.title = record.plan.chapter_title.clone();
            ch.plan = record.plan.chapter_plan.clone();
            ch.closed = record.close_chapter;
            ch.passage_ids.push(record.id.clone());
            kept.passages.push(PassageEntry {
                id: record.id.clone(),
                chapter_id: record.chapter_id.clone(),
                title: record.plan.title.clone(),
                record: fork.write_record("passages", &record)?,
            });
        }
        self.copy_memories_for_fork(&fork, &index, &mut kept)?;
        self.copy_media_for_fork(&fork, &index, &mut kept)?;
        fork.commit_story(0, uuid::Uuid::new_v4().to_string(), title, boundary, kept)?;
        drop(fork);
        fs::rename(&staging, &destination)
            .map_err(|e| format!("Fork remains in its incomplete staging folder: {e}"))?;
        Project::open(&destination)
    }
}

pub fn empty_effects() -> String {
    "{\"changes\":[],\"notes\":[],\"suggestChapterEnd\":false}".into()
}

#[cfg(test)]
mod tests;
