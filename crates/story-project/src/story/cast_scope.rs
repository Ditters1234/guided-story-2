//! Named cast references must be explicitly supplied, not recalled as edit scope.
use crate::{World, WorldNode};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
struct TrieNode {
    children: BTreeMap<char, usize>,
    people: BTreeSet<String>,
}
struct Names {
    nodes: Vec<TrieNode>,
}
impl Names {
    fn new() -> Self {
        Self {
            nodes: vec![TrieNode::default()],
        }
    }
    fn insert(&mut self, text: &str, id: &str) {
        let mut cursor = 0;
        for c in text.chars() {
            cursor = if let Some(next) = self.nodes[cursor].children.get(&c) {
                *next
            } else {
                let next = self.nodes.len();
                self.nodes.push(TrieNode::default());
                self.nodes[cursor].children.insert(c, next);
                next
            };
        }
        self.nodes[cursor].people.insert(id.into());
    }
    fn longest(&self, text: &[char], start: usize) -> Option<(usize, &BTreeSet<String>)> {
        let mut cursor = 0;
        let mut found = None;
        for (offset, c) in text[start..].iter().enumerate() {
            let Some(next) = self.nodes[cursor].children.get(c) else {
                break;
            };
            cursor = *next;
            let end = start + offset + 1;
            if !self.nodes[cursor].people.is_empty() && end_boundary(text, end) {
                found = Some((end, &self.nodes[cursor].people));
            }
        }
        found
    }
}

fn normalized(text: &str) -> String {
    let mut result = String::new();
    let mut space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            space = !result.is_empty();
        } else {
            if space {
                result.push(' ');
            }
            result.push(match c {
                '’' | '‘' => '\'',
                '‐' | '‑' => '-',
                _ => c,
            });
            space = false;
        }
    }
    result
}
fn folded(text: &str) -> String {
    text.chars().flat_map(char::to_lowercase).collect()
}
fn word(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
        // Combining accents continue a word; they are not name boundaries.
        || matches!(c, '\u{0300}'..='\u{036f}' | '\u{1ab0}'..='\u{1aff}' | '\u{1dc0}'..='\u{1dff}' | '\u{20d0}'..='\u{20ff}' | '\u{fe20}'..='\u{fe2f}')
}
fn start_boundary(text: &[char], start: usize) -> bool {
    start == 0
        || (!word(text[start - 1])
            && !(text[start - 1] == '\'' && start > 1 && word(text[start - 2])))
}
fn end_boundary(text: &[char], end: usize) -> bool {
    let Some(&next) = text.get(end) else {
        return true;
    };
    if next == '\'' {
        match text.get(end + 1) {
            None => true,
            Some('s' | 'S') => text.get(end + 2).is_none_or(|c| !word(*c)),
            Some(c) => !word(*c),
        }
    } else {
        !word(next)
    }
}

/// Inspect only the text actually selected for this task. No records are added
/// to context, no placement is inferred, and References need not be participants.
pub(super) fn validate_named_cast_scope(
    world: &World,
    context: &World,
    texts: &[&str],
) -> Result<(), String> {
    let people: Vec<&WorldNode> = world
        .nodes
        .iter()
        .filter(|n| n.components.contains_key("core.character") && !crate::knowledge::is_claim(n))
        .collect();
    let supplied: BTreeSet<_> = context.nodes.iter().map(|n| n.id.as_str()).collect();
    if people
        .iter()
        .all(|person| supplied.contains(person.id.as_str()))
    {
        return Ok(());
    }
    let by_id: BTreeMap<_, _> = people
        .iter()
        .map(|n| (n.id.as_str(), n.name.as_str()))
        .collect();
    let mut full = Names::new();
    let mut first_index: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut first_names = vec![];
    for person in people {
        let name = normalized(&person.name);
        if name.split_whitespace().count() > 1 && name.chars().any(char::is_alphanumeric) {
            full.insert(&folded(&name), &person.id);
        }
        if let Some(first) = name
            .split_whitespace()
            .next()
            .filter(|first| first.chars().filter(|c| c.is_alphabetic()).count() >= 4)
        {
            first_index
                .entry(folded(first))
                .or_default()
                .insert(person.id.clone());
            first_names.push((first.to_owned(), person.id.clone()));
        }
    }
    let mut first = Names::new();
    for (name, id) in first_names {
        if first_index[&folded(&name)].len() == 1 {
            first.insert(&name, &id);
        }
    }
    let mut missing = BTreeSet::new();
    for text in texts {
        let original: Vec<_> = normalized(text).chars().collect();
        let mut lower = vec![];
        let mut positions = vec![];
        for (index, c) in original.iter().enumerate() {
            for lower_c in c.to_lowercase() {
                lower.push(lower_c);
                positions.push(index);
            }
        }
        let mut full_spans = vec![];
        let mut cursor = 0;
        while cursor < lower.len() {
            if start_boundary(&lower, cursor) {
                if let Some((end, ids)) = full.longest(&lower, cursor) {
                    // A recognized longer/scoped or ambiguous full name must
                    // not be reinterpreted as a shorter name or first alias.
                    full_spans.push((positions[cursor], positions[end - 1] + 1));
                    if ids.len() == 1 {
                        let id = ids.first().unwrap();
                        if !supplied.contains(id.as_str()) {
                            missing.insert(by_id[id.as_str()].to_owned());
                        }
                    }
                    cursor = end;
                    continue;
                }
            }
            cursor += 1;
        }
        for start in 0..original.len() {
            if !start_boundary(&original, start) {
                continue;
            }
            if let Some((end, ids)) = first.longest(&original, start) {
                let next_span = full_spans.partition_point(|(_, b)| *b <= start);
                if full_spans.get(next_span).is_some_and(|(a, _)| *a < end) {
                    continue;
                }
                let id = ids.first().unwrap();
                if !supplied.contains(id.as_str()) {
                    missing.insert(by_id[id.as_str()].to_owned());
                }
            }
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Scene settings is missing context for {}. Add them as Viewpoint, Participants or References, then try again.",
            missing.into_iter().collect::<Vec<_>>().join(", ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cast(entries: &[(&str, &str)]) -> World {
        let mut world = World::default();
        world.nodes = entries
            .iter()
            .map(|(id, name)| WorldNode {
                id: (*id).into(),
                type_id: "custom.traveller".into(),
                name: (*name).into(),
                description: String::new(),
                visual_description: String::new(),
                components: BTreeMap::from([("core.character".into(), BTreeMap::new())]),
            })
            .collect();
        world
    }
    fn scope(world: &World, ids: &[&str]) -> World {
        let mut scoped = world.clone();
        scoped.nodes.retain(|n| ids.contains(&n.id.as_str()));
        scoped
    }
    #[test]
    fn named_cast_requires_explicit_context_but_references_need_no_participation() {
        let world = cast(&[("silas", "Silas Thorne"), ("elara", "Elara Vane")]);
        let before = serde_json::to_value(&world).unwrap();
        let text = "Keep Silas’s clothing practical. Describe Elara's voice.";
        let error = validate_named_cast_scope(&world, &scope(&world, &[]), &[text]).unwrap_err();
        assert!(error.contains("Silas Thorne") && error.contains("Elara Vane"));
        assert!(error.contains("Viewpoint, Participants or References"));
        assert!(
            validate_named_cast_scope(&world, &scope(&world, &["silas", "elara"]), &[text]).is_ok()
        );
        assert_eq!(serde_json::to_value(&world).unwrap(), before);
    }
    #[test]
    fn full_names_ignore_case_and_whitespace_first_names_require_exact_case() {
        let world = cast(&[
            ("elise", "Élise Moreau"),
            ("silas", "Silas Thorne"),
            ("tom", "Tom Li"),
            ("darcy", "D'Arcy Reed"),
        ]);
        let empty = scope(&world, &[]);
        for text in [
            "ÉLISE\nMOREAU enters.",
            "silas thorne’s coat",
            "D’Arcy Reed waited.",
            "Tom Li",
        ] {
            assert!(
                validate_named_cast_scope(&world, &empty, &[text]).is_err(),
                "{text}"
            );
        }
        for text in [
            "silas waits.",
            "Tom waits.",
            "Silasette",
            "préSilas",
            "Silas_2",
            "silas thornes",
            "D'ArcyReed",
            "D'Arcy-Else",
            "Élise\u{0301}tte",
        ] {
            assert!(
                validate_named_cast_scope(&world, &empty, &[text]).is_ok(),
                "{text}"
            );
        }
        assert!(validate_named_cast_scope(&world, &empty, &["‘Silas’ waits."]).is_err());
    }
    #[test]
    fn ambiguous_names_and_known_longer_names_never_guess_an_outside_identity() {
        let world = cast(&[
            ("one", "Silas Thorne"),
            ("two", "Silas Reed"),
            ("junior", "Silas Thorne Junior"),
            ("surname", "Thorne Avery"),
            ("same1", "Robin Vale"),
            ("same2", "ROBIN VALE"),
        ]);
        let selected = scope(&world, &["junior"]);
        assert!(validate_named_cast_scope(
            &world,
            &selected,
            &["Silas waits. Robin Vale waits. Silas Thorne Junior waits."]
        )
        .is_ok());
        assert!(
            validate_named_cast_scope(&world, &selected, &["Silas Thorne waits."])
                .unwrap_err()
                .contains("Silas Thorne")
        );
        assert!(validate_named_cast_scope(
            &world,
            &scope(&world, &["one"]),
            &["Silas Thorne waits."]
        )
        .is_ok());
    }
    #[test]
    fn claim_bearing_people_and_noncharacters_do_not_create_recall_scope() {
        let mut world = cast(&[
            ("public", "Silas Thorne"),
            ("hidden", "Hidden Person"),
            ("place", "Silent Meadow"),
        ]);
        world.nodes[1]
            .components
            .insert("core.claim".into(), BTreeMap::new());
        world.nodes[2].components.clear();
        assert!(validate_named_cast_scope(
            &world,
            &scope(&world, &[]),
            &["Hidden Person and Silent Meadow."]
        )
        .is_ok());
        assert!(validate_named_cast_scope(&world, &scope(&world, &[]), &["Silas Thorne"]).is_err());
    }
}
