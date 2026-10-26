use super::*;

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryCoverage {
    pub older_passages: u32,
    pub closed_chapters: u32,
    pub current_summaries: u32,
    pub missing_summaries: u32,
    pub stale_summaries: u32,
    pub unavailable_summaries: u32,
    pub unrepresented_passages: u32,
    pub available_points: u32,
    pub selected_points: u32,
    pub omitted_points: u32,
    pub byte_budget: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RememberedPoint {
    pub text: String,
    pub passage_id: String,
    pub chapter_id: String,
    pub source: RecordRef,
    pub node_ids: Vec<String>,
    pub summary: Option<RecordRef>,
    pub reason: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryContext {
    pub points: Vec<RememberedPoint>,
    pub coverage: MemoryCoverage,
}

impl Project {
    fn memory_state(
        &self,
        index: &StoryIndex,
        chapter_id: &str,
    ) -> (MemoryStatus, Option<ChapterMemory>, String) {
        let Some(reference) = index.memories.get(chapter_id) else {
            return (MemoryStatus::Missing, None, String::new());
        };
        let record = match self.read_record::<ChapterMemory>("memories", reference) {
            Ok(record) => record,
            Err(error) => return (MemoryStatus::Unavailable, None, error),
        };
        if record.schema_version != 1
            || record.chapter_id != chapter_id
            || validate_id(&record.id).is_err()
            || validate_memory_shape(&record.output, &record.sources).is_err()
        {
            return (
                MemoryStatus::Unavailable,
                None,
                "Invalid saved chapter memory; it has not been replaced.".into(),
            );
        }
        match memory_sources(index, chapter_id) {
            Ok((title, sources)) if title == record.chapter_title && sources == record.sources => {
                (MemoryStatus::Current, Some(record), String::new())
            }
            _ => (
                MemoryStatus::Stale,
                None,
                "Chapter text or closing state changed. This summary is excluded from generation."
                    .into(),
            ),
        }
    }
    pub fn chapter_memory_status(&self) -> Result<Vec<ChapterMemoryStatus>, String> {
        let index = self.story_index()?;
        Ok(index
            .chapters
            .iter()
            .filter(|c| c.closed || index.memories.contains_key(&c.id))
            .map(|c| {
                let (status, record, error) = self.memory_state(&index, &c.id);
                ChapterMemoryStatus {
                    closed: c.closed,
                    chapter_id: c.id.clone(),
                    title: c.title.clone(),
                    status,
                    points: record.map_or(0, |r| r.output.points.len() as u32),
                    error,
                }
            })
            .collect())
    }
    pub(crate) fn memory_context(
        &self,
        index: &StoryIndex,
        plan: &PassagePlan,
        world: &World,
        budget: u32,
    ) -> Result<MemoryContext, String> {
        let cutoff = index.passages.len().saturating_sub(2);
        let older: BTreeSet<_> = index
            .passages
            .iter()
            .take(cutoff)
            .map(|p| p.id.clone())
            .collect();
        let positions: BTreeMap<_, _> = index
            .passages
            .iter()
            .enumerate()
            .map(|(i, p)| (p.id.clone(), i))
            .collect();
        let mut coverage = MemoryCoverage {
            older_passages: cutoff as u32,
            byte_budget: budget.min(default_memory_budget()),
            ..Default::default()
        };
        let mut represented = BTreeSet::new();
        let mut summarized = BTreeSet::new();
        let mut candidates = vec![];
        for chapter in &index.chapters {
            if !chapter.closed {
                continue;
            }
            coverage.closed_chapters += 1;
            let (status, record, _) = self.memory_state(index, &chapter.id);
            match status {
                MemoryStatus::Current => coverage.current_summaries += 1,
                MemoryStatus::Missing => coverage.missing_summaries += 1,
                MemoryStatus::Stale => coverage.stale_summaries += 1,
                MemoryStatus::Unavailable => coverage.unavailable_summaries += 1,
            }
            if let Some(record) = record {
                for source in &record.sources {
                    if older.contains(&source.passage_id) {
                        represented.insert(source.passage_id.clone());
                        summarized.insert(source.passage_id.clone());
                    }
                }
                if record.sources.iter().any(|s| older.contains(&s.passage_id)) {
                    for point in record.output.points {
                        let source = record
                            .sources
                            .iter()
                            .find(|s| s.passage_id == point.passage_id)
                            .ok_or("Memory source passage is missing.")?;
                        candidates.push(RememberedPoint {
                            text: point.text,
                            passage_id: point.passage_id,
                            chapter_id: chapter.id.clone(),
                            source: source.record.clone(),
                            node_ids: point.node_ids,
                            summary: index.memories.get(&chapter.id).cloned(),
                            reason: "Chapter summary".into(),
                        });
                    }
                }
            }
        }
        for entry in index
            .passages
            .iter()
            .take(cutoff)
            .filter(|p| !summarized.contains(&p.id))
        {
            let record: AcceptedPassage = self.read_record("passages", &entry.record)?;
            if !record.effects.notes.is_empty() {
                represented.insert(entry.id.clone());
            }
            for note in record.effects.notes {
                candidates.push(RememberedPoint {
                    text: note.text,
                    passage_id: entry.id.clone(),
                    chapter_id: entry.chapter_id.clone(),
                    source: entry.record.clone(),
                    node_ids: note.node_ids,
                    summary: None,
                    reason: "Accepted passage note".into(),
                });
            }
        }
        coverage.unrepresented_passages = older.difference(&represented).count() as u32;
        let score =
            |point: &RememberedPoint| {
                let mut value = 0;
                for id in &point.node_ids {
                    value += if plan.reference_ids.contains(id) {
                        40
                    } else if plan.viewpoint_id.as_ref() == Some(id) {
                        25
                    } else if plan.participant_ids.contains(id) {
                        15
                    } else if plan.location_id.as_ref() == Some(id) {
                        10
                    } else if world.nodes.iter().any(|n| &n.id == id) {
                        3
                    } else {
                        0
                    };
                }
                if point.node_ids.is_empty() {
                    let text = point.text.to_lowercase();
                    if world.nodes.iter().any(|n| {
                        n.name.chars().count() >= 3 && text.contains(&n.name.to_lowercase())
                    }) {
                        value += 2;
                    }
                }
                value
            };
        candidates.sort_by(|a, b| {
            score(b)
                .cmp(&score(a))
                .then(
                    positions
                        .get(&b.passage_id)
                        .cmp(&positions.get(&a.passage_id)),
                )
                .then(a.passage_id.cmp(&b.passage_id))
                .then(a.text.cmp(&b.text))
        });
        let mut deduplicated = BTreeSet::new();
        candidates.retain(|p| deduplicated.insert((p.passage_id.clone(), p.text.clone())));
        coverage.available_points = candidates.len() as u32;
        let mut points = vec![];
        let mut bytes = 2usize;
        for mut point in candidates {
            if score(&point) > 0 {
                point.reason.push_str(" · selected scene resources");
            }
            let size = serde_json::to_vec(&point).map_err(|e| e.to_string())?.len() + 1;
            if points.len() < 12 && bytes + size <= coverage.byte_budget as usize {
                bytes += size;
                points.push(point);
            }
        }
        coverage.selected_points = points.len() as u32;
        coverage.omitted_points = coverage.available_points - coverage.selected_points;
        Ok(MemoryContext { points, coverage })
    }
    pub(crate) fn copy_memories_for_fork(
        &self,
        fork: &Project,
        source: &StoryIndex,
        kept: &mut StoryIndex,
    ) -> Result<(), String> {
        for chapter in &kept.chapters {
            if !chapter.closed {
                continue;
            }
            let (status, record, _) = self.memory_state(source, &chapter.id);
            if status != MemoryStatus::Current {
                continue;
            }
            let Some(mut record) = record else { continue };
            let Ok((title, sources)) = memory_sources(kept, &chapter.id) else {
                continue;
            };
            if record
                .sources
                .iter()
                .map(|s| &s.passage_id)
                .collect::<Vec<_>>()
                != sources.iter().map(|s| &s.passage_id).collect::<Vec<_>>()
            {
                continue;
            }
            record.chapter_title = title;
            record.sources = sources;
            kept.memories
                .insert(chapter.id.clone(), fork.write_record("memories", &record)?);
        }
        Ok(())
    }
}
