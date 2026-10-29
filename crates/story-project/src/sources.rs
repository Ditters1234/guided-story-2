//! Typed provenance over shared nodes. Citations never execute passage effects.
use crate::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use specta::Type;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SourceOrigin {
    Manual {
        note: String,
    },
    Passage {
        passage_id: String,
        text_version_id: String,
        event_id: String,
        anchor: RecordRef,
        quote: String,
        effect_index: Option<u32>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceAnnotation {
    pub id: String,
    pub node_id: String,
    pub subject_fingerprint: String,
    pub origin: SourceOrigin,
}
/// A minimal immutable text/evidence anchor, without world pointers or revision history.
/// Its independent identity avoids a passage -> after-world -> passage checksum cycle.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceAnchor {
    pub schema_version: u32,
    pub passage_id: String,
    pub text_version_id: String,
    pub event_id: String,
    pub title: String,
    pub prose: String,
    pub effect_quotes: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SourceEdit {
    AddManual {
        node_id: String,
        note: String,
    },
    AddPassage {
        node_id: String,
        passage_id: String,
        version: RecordRef,
        quote: String,
        effect_index: Option<u32>,
    },
    Remove {
        id: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NodeSourceView {
    pub annotation: SourceAnnotation,
    pub title: String,
    pub earlier_wording: bool,
    pub edited_since_source: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceReading {
    pub title: String,
    pub prose: String,
    pub quote: String,
    pub passage_id: String,
    pub earlier_wording: bool,
}
fn fingerprint(value: &impl Serialize) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).expect("serializable source"))
    )
}
fn hash_valid(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
pub(crate) fn node_source_fingerprint(node: &WorldNode) -> String {
    fingerprint(node)
}
pub(crate) fn new_event_identity() -> String {
    fingerprint(&uuid::Uuid::new_v4().to_string())
}
pub(crate) fn passage_event_identity(p: &AcceptedPassage) -> String {
    p.event_id.clone().unwrap_or_else(|| {
        fingerprint(&(
            "legacy-event-identity-v1",
            &p.id,
            &p.before_world.sha256,
            &p.after_world.sha256,
        ))
    })
}
pub(crate) fn text_version_identity(p: &AcceptedPassage, r: &RecordRef) -> String {
    p.text_version_id.clone().unwrap_or_else(|| r.id.clone())
}
pub(crate) fn effect_quotes(effects: &PassageEffects) -> Vec<String> {
    effects
        .changes
        .iter()
        .map(|c| match c {
            PassageEffect::PutNode { evidence, .. }
            | PassageEffect::PutLink { evidence, .. }
            | PassageEffect::RemoveLink { evidence, .. } => evidence.clone(),
        })
        .collect()
}
fn exact_quote(prose: &str, quote: &str) -> bool {
    let normalize = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    let q = normalize(quote);
    (8..=600).contains(&q.chars().count()) && normalize(prose).contains(&q)
}
impl SourceAnchor {
    pub(crate) fn validate(&self) -> Result<(), String> {
        validate_id(&self.passage_id)?;
        validate_id(&self.text_version_id)?;
        validate_name(&self.title)?;
        if self.schema_version != 1
            || !hash_valid(&self.event_id)
            || self.prose.len() > 100000
            || self.effect_quotes.len() > 80
            || self.effect_quotes.iter().any(|q| {
                !(8..=600).contains(
                    &q.split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .chars()
                        .count(),
                )
            })
        {
            return Err("Invalid passage source anchor.".into());
        }
        Ok(())
    }
}
pub(crate) fn validate_annotation_shapes(world: &World) -> Result<(), String> {
    if world.source_annotations.len() > 10000 {
        return Err("Source annotation limit reached (10,000).".into());
    }
    let mut ids = BTreeSet::new();
    let mut counts = BTreeMap::<&str, usize>::new();
    for a in &world.source_annotations {
        validate_id(&a.id)?;
        validate_id(&a.node_id)?;
        *counts.entry(&a.node_id).or_default() += 1;
        if !ids.insert(&a.id)
            || !world.nodes.iter().any(|n| n.id == a.node_id)
            || !hash_valid(&a.subject_fingerprint)
        {
            return Err("Invalid source annotation target, identity or fingerprint.".into());
        }
        match &a.origin {
            SourceOrigin::Manual { note } => {
                if note.trim().is_empty() || note.chars().count() > 2000 {
                    return Err("An author source needs 1–2,000 characters.".into());
                }
            }
            SourceOrigin::Passage {
                passage_id,
                text_version_id,
                event_id,
                anchor,
                quote,
                effect_index,
            } => {
                validate_id(passage_id)?;
                validate_id(text_version_id)?;
                validate_id(&anchor.id)?;
                if !hash_valid(event_id)
                    || !hash_valid(&anchor.sha256)
                    || !(8..=600).contains(
                        &quote
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .chars()
                            .count(),
                    )
                    || effect_index.is_some_and(|i| i >= 80)
                {
                    return Err("Invalid passage source annotation.".into());
                }
            }
        }
    }
    if counts.values().any(|n| *n > 32) {
        return Err("A node supports at most 32 source annotations. Remove an unneeded citation before adding another.".into());
    }
    Ok(())
}
/// Authored Event additions are backstory; this metadata never simulates their consequences.
pub(crate) fn ensure_manual_event_sources(world: &mut World) {
    for node in &world.nodes {
        if node.components.contains_key("detail.event")
            && !world
                .source_annotations
                .iter()
                .any(|a| a.node_id == node.id)
        {
            world.source_annotations.push(SourceAnnotation {
                id: node.id.clone(),
                node_id: node.id.clone(),
                subject_fingerprint: node_source_fingerprint(node),
                origin: SourceOrigin::Manual {
                    note: "Author-established event / backstory.".into(),
                },
            });
        }
    }
}
pub(crate) fn scope_sources(world: &mut World) {
    let ids: BTreeSet<_> = world.nodes.iter().map(|n| n.id.as_str()).collect();
    world
        .source_annotations
        .retain(|a| ids.contains(a.node_id.as_str()));
}
pub(crate) fn validate_annotation_records_using(
    world: &World,
    read_anchor: impl Fn(&RecordRef) -> Result<SourceAnchor, String>,
) -> Result<(), String> {
    let mut anchors = BTreeMap::new();
    for a in &world.source_annotations {
        if let SourceOrigin::Passage {
            passage_id,
            text_version_id,
            event_id,
            anchor,
            quote,
            effect_index,
        } = &a.origin
        {
            let key = (&anchor.id, &anchor.sha256);
            if !anchors.contains_key(&key) {
                let data: SourceAnchor = read_anchor(anchor)?;
                data.validate()?;
                anchors.insert(key, data);
            }
            let data = &anchors[&key];
            if data.passage_id != *passage_id
                || data.text_version_id != *text_version_id
                || data.event_id != *event_id
                || !exact_quote(&data.prose, quote)
                || effect_index.is_some_and(|i| {
                    data.effect_quotes
                        .get(i as usize)
                        .is_none_or(|q| !exact_quote(q, quote))
                })
            {
                return Err(
                    "A source citation does not match its exact retained text/effect anchor."
                        .into(),
                );
            }
        }
    }
    Ok(())
}
pub(crate) fn validate_world_sources_using(
    world: &World,
    index: &StoryIndex,
    read_anchor: impl Fn(&RecordRef) -> Result<SourceAnchor, String>,
    read_passage: impl Fn(&RecordRef) -> Result<AcceptedPassage, String>,
) -> Result<(), String> {
    validate_annotation_records_using(world, &read_anchor)?;
    let mut passages = BTreeMap::new();
    for a in &world.source_annotations {
        if let SourceOrigin::Passage {
            passage_id,
            event_id,
            text_version_id,
            anchor,
            ..
        } = &a.origin
        {
            let entry = index
                .passages
                .iter()
                .find(|e| &e.id == passage_id)
                .ok_or("A source citation refers to unavailable or future story history.")?;
            if !passages.contains_key(passage_id) {
                let p: AcceptedPassage = read_passage(&entry.record)?;
                passages.insert(passage_id, p);
            }
            let p = &passages[passage_id];
            if p.id != *passage_id || passage_event_identity(p) != *event_id {
                return Err(
                    "A source citation belongs to superseded or unavailable events.".into(),
                );
            }
            if text_version_identity(p, &entry.record) == *text_version_id {
                if p.text_anchor
                    .as_ref()
                    .is_some_and(|r| r.sha256 != anchor.sha256)
                {
                    return Err("The citation is not the registered accepted text anchor.".into());
                }
                let data: SourceAnchor = read_anchor(anchor)?;
                if data.prose != p.prose || data.effect_quotes != effect_quotes(&p.effects) {
                    return Err("A source anchor does not match the accepted text version.".into());
                }
            } else if p
                .earlier_text_anchors
                .get(text_version_id)
                .is_none_or(|r| r.sha256 != anchor.sha256)
            {
                return Err(
                    "The earlier wording citation is not retained by this passage version.".into(),
                );
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_passage_anchors_using(
    p: &AcceptedPassage,
    r: &RecordRef,
    read_anchor: impl Fn(&RecordRef) -> Result<SourceAnchor, String>,
) -> Result<(), String> {
    if p.text_version_id.is_some() != p.event_id.is_some()
        || p.text_version_id.is_some() != p.text_anchor.is_some()
    {
        return Err("Incomplete accepted text-source identity.".into());
    }
    if let Some(id) = &p.text_version_id {
        validate_id(id)?;
    }
    if p.event_id.as_ref().is_some_and(|id| !hash_valid(id)) {
        return Err("Invalid accepted event identity.".into());
    }
    if p.earlier_text_anchors.len() > 512 {
        return Err("The passage has too many retained text anchors. Fork before adding more wording revisions.".into());
    }
    if let Some(reference) = &p.text_anchor {
        let a: SourceAnchor = read_anchor(reference)?;
        a.validate()?;
        if a.passage_id != p.id
            || a.text_version_id != text_version_identity(p, r)
            || a.event_id != passage_event_identity(p)
            || a.prose != p.prose
            || a.effect_quotes != effect_quotes(&p.effects)
        {
            return Err("The passage text anchor does not match its accepted version.".into());
        }
    }
    for (version, reference) in &p.earlier_text_anchors {
        validate_id(version)?;
        let a: SourceAnchor = read_anchor(reference)?;
        a.validate()?;
        if a.passage_id != p.id
            || &a.text_version_id != version
            || a.event_id != passage_event_identity(p)
        {
            return Err(
                "An earlier text anchor belongs to another passage or superseded events.".into(),
            );
        }
    }
    Ok(())
}

impl Project {
    pub(crate) fn validate_annotation_records(&self, world: &World) -> Result<(), String> {
        validate_annotation_records_using(world, |r| self.read_record("source-anchors", r))
    }
    pub(crate) fn validate_world_sources(
        &self,
        world: &World,
        index: &StoryIndex,
    ) -> Result<(), String> {
        validate_world_sources_using(
            world,
            index,
            |r| self.read_record("source-anchors", r),
            |r| self.read_record("passages", r),
        )
    }
    pub(crate) fn capture_source_anchor(
        &self,
        p: &AcceptedPassage,
        version: &RecordRef,
    ) -> Result<(SourceAnchor, RecordRef), String> {
        let anchor = SourceAnchor {
            schema_version: 1,
            passage_id: p.id.clone(),
            text_version_id: text_version_identity(p, version),
            event_id: passage_event_identity(p),
            title: p.plan.title.clone(),
            prose: p.prose.clone(),
            effect_quotes: effect_quotes(&p.effects),
        };
        anchor.validate()?;
        let reference = if let Some(reference) = &p.text_anchor {
            let retained: SourceAnchor = self.read_record("source-anchors", reference)?;
            if serde_json::to_vec(&retained).unwrap() != serde_json::to_vec(&anchor).unwrap() {
                return Err("The text source does not match its registered anchor.".into());
            }
            reference.clone()
        } else {
            self.write_record("source-anchors", &anchor)?
        };
        Ok((anchor, reference))
    }
    pub fn preview_source_edit(
        &self,
        world: &World,
        edit: SourceEdit,
    ) -> Result<WorldEditPreview, String> {
        world.validate()?;
        let index = self.story_index()?;
        self.validate_world_sources(world, &index)?;
        let mut next = world.clone();
        let mut changes = vec![];
        match edit {
            SourceEdit::Remove { id } => {
                let existing = next
                    .source_annotations
                    .iter()
                    .find(|a| a.id == id)
                    .ok_or("This source annotation no longer exists.")?;
                let is_event = next
                    .nodes
                    .iter()
                    .find(|n| n.id == existing.node_id)
                    .is_some_and(|n| n.components.contains_key("detail.event"));
                let sole = is_event
                    && next
                        .source_annotations
                        .iter()
                        .filter(|a| a.node_id == existing.node_id)
                        .count()
                        == 1;
                if sole && matches!(existing.origin, SourceOrigin::Manual { .. }) {
                    return Err("This Event retains an author source. Add a replacement source before removing it, or remove the Event node.".into());
                }
                if sole {
                    changes.push("Keep this Event as author-established backstory; no passage citation remains.".into());
                }
                let len = next.source_annotations.len();
                next.source_annotations.retain(|a| a.id != id);
                if len == next.source_annotations.len() {
                    return Err("This source annotation no longer exists.".into());
                }
                changes.push(
                    "Remove this citation. The node and established story remain intact.".into(),
                );
            }
            SourceEdit::AddManual { node_id, note } => {
                let node = world
                    .nodes
                    .iter()
                    .find(|n| n.id == node_id)
                    .ok_or("The source target no longer exists.")?;
                next.source_annotations.push(SourceAnnotation {
                    id: uuid::Uuid::new_v4().to_string(),
                    node_id,
                    subject_fingerprint: node_source_fingerprint(node),
                    origin: SourceOrigin::Manual { note },
                });
                changes.push("Record an author-supplied source for this node; no passage or effects are replayed.".into());
            }
            SourceEdit::AddPassage {
                node_id,
                passage_id,
                version,
                quote,
                effect_index,
            } => {
                let node = world
                    .nodes
                    .iter()
                    .find(|n| n.id == node_id)
                    .ok_or("The source target no longer exists.")?;
                let reading = self.read_passage(&passage_id)?;
                if reading.record != version {
                    return Err(
                        "This passage was revised. Reload its source before attaching a citation."
                            .into(),
                    );
                }
                let (anchor, reference) = self.capture_source_anchor(&reading.passage, &version)?;
                if !exact_quote(&anchor.prose, &quote)
                    || effect_index.is_some_and(|i| {
                        anchor
                            .effect_quotes
                            .get(i as usize)
                            .is_none_or(|q| !exact_quote(q, &quote))
                    })
                {
                    return Err("Choose one continuous 8–600 character quote from the selected passage/effect.".into());
                }
                next.source_annotations.push(SourceAnnotation {
                    id: uuid::Uuid::new_v4().to_string(),
                    node_id,
                    subject_fingerprint: node_source_fingerprint(node),
                    origin: SourceOrigin::Passage {
                        passage_id,
                        text_version_id: anchor.text_version_id,
                        event_id: anchor.event_id,
                        anchor: reference,
                        quote,
                        effect_index,
                    },
                });
                changes.push("Attach this exact accepted text version as a citation. Node facts and historical snapshots are unchanged.".into());
            }
        }
        ensure_manual_event_sources(&mut next);
        next.validate()?;
        self.validate_world_sources(&next, &index)?;
        Ok(WorldEditPreview {
            world: next,
            changes,
        })
    }
    pub fn node_sources(
        &self,
        world: &World,
        node_id: &str,
    ) -> Result<Vec<NodeSourceView>, String> {
        world.validate()?;
        let index = self.story_index()?;
        self.validate_world_sources(world, &index)?;
        world
            .source_annotations
            .iter()
            .filter(|a| a.node_id == node_id)
            .map(|a| {
                let (title, earlier) = match &a.origin {
                    SourceOrigin::Manual { .. } => ("Author source".into(), false),
                    SourceOrigin::Passage {
                        passage_id,
                        text_version_id,
                        anchor,
                        event_id,
                        ..
                    } => {
                        let data: SourceAnchor = self.read_record("source-anchors", anchor)?;
                        let entry = index
                            .passages
                            .iter()
                            .find(|p| &p.id == passage_id)
                            .ok_or("The cited passage is unavailable in this story.")?;
                        let current: AcceptedPassage =
                            self.read_record("passages", &entry.record)?;
                        if passage_event_identity(&current) != *event_id {
                            return Err("The citation belongs to superseded events.".into());
                        }
                        (
                            data.title,
                            text_version_identity(&current, &entry.record) != *text_version_id,
                        )
                    }
                };
                Ok(NodeSourceView {
                    annotation: a.clone(),
                    title,
                    earlier_wording: earlier,
                    edited_since_source: world
                        .nodes
                        .iter()
                        .find(|n| n.id == a.node_id)
                        .is_none_or(|n| node_source_fingerprint(n) != a.subject_fingerprint),
                })
            })
            .collect()
    }
    pub fn read_node_source(&self, world: &World, id: &str) -> Result<SourceReading, String> {
        let a = world
            .source_annotations
            .iter()
            .find(|a| a.id == id)
            .ok_or("This citation is not in the selected world state.")?;
        let info = self
            .node_sources(world, &a.node_id)?
            .into_iter()
            .find(|v| v.annotation.id == id)
            .ok_or("Missing citation.")?;
        let SourceOrigin::Passage {
            anchor,
            quote,
            passage_id,
            ..
        } = &a.origin
        else {
            return Err("This is an author source, without passage text.".into());
        };
        let data: SourceAnchor = self.read_record("source-anchors", anchor)?;
        Ok(SourceReading {
            title: data.title,
            prose: data.prose,
            quote: quote.clone(),
            passage_id: passage_id.clone(),
            earlier_wording: info.earlier_wording,
        })
    }
    pub(crate) fn write_text_anchor(
        &self,
        passage_id: &str,
        version: &str,
        event_id: &str,
        title: &str,
        prose: &str,
        effects: &PassageEffects,
    ) -> Result<RecordRef, String> {
        let anchor = SourceAnchor {
            schema_version: 1,
            passage_id: passage_id.into(),
            text_version_id: version.into(),
            event_id: event_id.into(),
            title: title.into(),
            prose: prose.into(),
            effect_quotes: effect_quotes(effects),
        };
        anchor.validate()?;
        self.write_record("source-anchors", &anchor)
    }
    pub(crate) fn validate_passage_anchors(
        &self,
        p: &AcceptedPassage,
        r: &RecordRef,
    ) -> Result<(), String> {
        validate_passage_anchors_using(p, r, |r| self.read_record("source-anchors", r))
    }
    /// Called only after every effect is reviewed, immediately before atomic publication.
    pub(crate) fn annotate_passage_events(
        &self,
        world: &mut World,
        effects: &PassageEffects,
        allocations: &BTreeMap<String, String>,
        reference: &RecordRef,
    ) -> Result<(), String> {
        let anchor: SourceAnchor = self.read_record("source-anchors", reference)?;
        anchor.validate()?;
        let targets: Vec<_> = effects
            .changes
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let PassageEffect::PutNode { node, evidence } = e else {
                    return None;
                };
                node.components.contains_key("detail.event").then(|| {
                    (
                        i,
                        allocations.get(&node.id).unwrap_or(&node.id).clone(),
                        evidence.clone(),
                    )
                })
            })
            .collect();
        if targets.is_empty() {
            return Ok(());
        }
        for (index, id, quote) in targets {
            let node = world
                .nodes
                .iter()
                .find(|n| n.id == id)
                .ok_or("The accepted event target is missing.")?;
            world.source_annotations.push(SourceAnnotation {
                id: uuid::Uuid::new_v4().to_string(),
                node_id: id,
                subject_fingerprint: node_source_fingerprint(node),
                origin: SourceOrigin::Passage {
                    passage_id: anchor.passage_id.clone(),
                    text_version_id: anchor.text_version_id.clone(),
                    event_id: anchor.event_id.clone(),
                    anchor: reference.clone(),
                    quote,
                    effect_index: Some(index as u32),
                },
            });
        }
        world.validate()
    }
    pub(crate) fn copy_text_anchor(
        &self,
        fork: &Project,
        anchor: &RecordRef,
    ) -> Result<(), String> {
        let destination = fork
            .root_path()
            .join("source-anchors")
            .join(format!("{}.json", anchor.id));
        if destination.exists() {
            let retained: SourceAnchor = fork.read_record("source-anchors", anchor)?;
            retained.validate()?;
            return Ok(());
        }
        let data: SourceAnchor = self.read_record("source-anchors", anchor)?;
        data.validate()?;
        let dir = fork.root_path().join("source-anchors");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!("{}.json", anchor.id));
        let bytes = crate::store::read_bytes(
            &self
                .root_path()
                .join("source-anchors")
                .join(format!("{}.json", anchor.id)),
        )?;
        if path.exists() {
            let _: SourceAnchor = fork.read_record("source-anchors", anchor)?;
        } else {
            crate::store::atomic_write(&path, &bytes)?;
        }
        Ok(())
    }
    pub(crate) fn copy_source_anchors(&self, fork: &Project, world: &World) -> Result<(), String> {
        self.validate_annotation_records(world)?;
        for a in &world.source_annotations {
            if let SourceOrigin::Passage { anchor, .. } = &a.origin {
                self.copy_text_anchor(fork, anchor)?;
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests;
