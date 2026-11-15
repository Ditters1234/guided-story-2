use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PassageHandoff {
    pub withheld: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passage_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<RecordRef>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub chapter_relation: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ending_excerpt: String,
}

/// An exact, bounded ending, never a model summary or a new story fact.
pub(super) fn ending_excerpt(prose: &str) -> &str {
    let prose = prose.trim_end();
    let mut start = prose.len().saturating_sub(1200);
    while !prose.is_char_boundary(start) {
        start += 1;
    }
    if start > 0 {
        // Avoid starting in the middle of a word when a boundary is available.
        if let Some((offset, ch)) = prose[start..]
            .char_indices()
            .find(|(_, c)| c.is_whitespace())
        {
            start += offset + ch.len_utf8();
        }
    }
    &prose[start..]
}

pub(super) fn handoff(index: &StoryIndex, recent: &[Value]) -> Option<PassageHandoff> {
    let latest = index.passages.last()?;
    // Never substitute an older permitted passage for a withheld latest one.
    let included = recent.iter().find(|p| p["id"].as_str() == Some(&latest.id));
    Some(match included {
        Some(p) => PassageHandoff {
            withheld: false,
            passage_id: Some(latest.id.clone()),
            source: Some(latest.record.clone()),
            title: p["title"].as_str().unwrap_or_default().into(),
            chapter_relation: p["chapterRelation"].as_str().unwrap_or_default().into(),
            ending_excerpt: ending_excerpt(p["prose"].as_str().unwrap_or_default()).into(),
        },
        None => PassageHandoff {
            withheld: true,
            passage_id: None,
            source: None,
            title: String::new(),
            chapter_relation: String::new(),
            ending_excerpt: String::new(),
        },
    })
}

/// Keep the exact ending in the handoff and the preceding prose and notes in
/// recent history. Work on the provider projection, leaving native recall and
/// repeated-passage detection on the complete accepted text.
pub(super) fn split_recent_handoff(input: &mut Value) {
    let handoff = &input["passageHandoff"];
    let Some((id, ending)) = handoff["passageId"]
        .as_str()
        .zip(handoff["endingExcerpt"].as_str())
    else {
        return;
    };
    let (id, ending) = (id.to_owned(), ending.to_owned());
    let Some(passage) = input["recentPassages"]
        .as_array_mut()
        .and_then(|passages| passages.iter_mut().find(|passage| passage["id"] == id))
    else {
        return;
    };
    if let Some(prefix) = passage["prose"]
        .as_str()
        .and_then(|prose| prose.trim_end().strip_suffix(&ending))
    {
        let prefix = prefix.trim_end();
        if prefix.is_empty() {
            passage.as_object_mut().unwrap().remove("prose");
        } else {
            passage["prose"] = json!(prefix);
        }
    }
}

pub(super) fn project_prompt_sources(input: &mut Value) {
    fn omit(value: &mut Value, keys: &[&str]) {
        if let Some(fields) = value.as_object_mut() {
            fields.retain(|key, _| !keys.contains(&key.as_str()));
        }
    }
    fn project_citations(sources: &mut Value) {
        for citation in sources.as_array_mut().into_iter().flatten() {
            omit(citation, &["id", "subjectFingerprint"]);
            if let Some(origin) = citation.get_mut("origin") {
                omit(
                    origin,
                    &["anchor", "textVersionId", "eventId", "effectIndex"],
                );
            }
        }
    }

    // Version pointers stay in the native context for review and staleness.
    for passage in input
        .get_mut("recentPassages")
        .and_then(Value::as_array_mut)
        .into_iter()
        .flatten()
    {
        omit(passage, &["version", "chapterId"]);
    }
    if let Some(handoff) = input.get_mut("passageHandoff") {
        omit(handoff, &["source"]);
    }
    if let Some(points) = input
        .pointer_mut("/earlierMemory/points")
        .and_then(Value::as_array_mut)
    {
        for point in points {
            omit(point, &["source", "summary", "chapterId"]);
        }
    }
    if let Some(sources) = input.get_mut("sources") {
        project_citations(sources);
    }
    for path in ["/records", "/situatedKnowledge/claims"] {
        if let Some(records) = input.pointer_mut(path).and_then(Value::as_array_mut) {
            for record in records {
                if let Some(sources) = record.get_mut("sources") {
                    project_citations(sources);
                }
            }
        }
    }
    if let Some(plan) = input.get_mut("plan") {
        omit(plan, &["consumedBriefPassage"]);
    }
}

/// Advisory exact-copy detection only; validation and acceptance are unchanged.
/// Inspect only history already allowed into this draft's narrative context.
pub(super) fn repeated_passages(prose: &str, input: &Value) -> Vec<String> {
    fn normalize(text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }
    let normalized = normalize(prose);
    if normalized.split_whitespace().count() < 16 {
        return vec![];
    }
    let paragraphs: Vec<_> = prose
        .replace("\r\n", "\n")
        .split("\n\n")
        .map(normalize)
        .filter(|p| p.split_whitespace().count() >= 40)
        .collect();
    input["recentPassages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let earlier = normalize(entry["prose"].as_str()?);
            (earlier == normalized || paragraphs.iter().any(|p| earlier.contains(p)))
                .then(|| entry["id"].as_str().map(String::from))
                .flatten()
        })
        .collect()
}
