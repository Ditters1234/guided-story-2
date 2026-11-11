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
