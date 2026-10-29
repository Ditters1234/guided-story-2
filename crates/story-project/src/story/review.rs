use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum EffectReviewKind {
    Change,
    Note,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectReviewEntry {
    pub kind: EffectReviewKind,
    pub index: u32,
    pub key: String,
    pub approved: bool,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectsReview {
    pub checked: bool,
    pub entries: Vec<EffectReviewEntry>,
}

fn node_references(node: &WorldNode, world: &World) -> Vec<String> {
    let mut refs = vec![];
    for (id, values) in &node.components {
        if let Some(def) = world.component_types.iter().find(|d| &d.id == id) {
            for field in &def.fields {
                match (&field.kind, values.get(&field.key)) {
                    (FieldKind::NodeRef, Some(PropertyValue::Text(id))) => refs.push(id.clone()),
                    (FieldKind::NodeRefs, Some(PropertyValue::NodeRefs(ids))) => {
                        refs.extend(ids.clone())
                    }
                    _ => (),
                }
            }
        }
    }
    refs
}

fn dependencies<'a>(
    refs: Vec<String>,
    nodes: &BTreeMap<&str, &'a WorldNode>,
    world: &World,
) -> BTreeMap<String, &'a WorldNode> {
    let mut found = BTreeMap::new();
    let mut pending = refs;
    while let Some(id) = pending.pop() {
        if found.contains_key(&id) {
            continue;
        }
        if let Some(node) = nodes.get(id.as_str()) {
            found.insert(id, *node);
            pending.extend(node_references(node, world));
        }
    }
    found
}

pub(super) fn effect_review(
    edits: &PassageEdits,
    prepared: Option<(&World, &str)>,
    checked: bool,
    issues: &[Vec<String>],
) -> Result<EffectsReview, String> {
    let Some((world, source)) = prepared else {
        return Ok(EffectsReview {
            checked: false,
            entries: vec![],
        });
    };
    let Ok(effects) = serde_json::from_str::<PassageEffects>(&edits.effects_text) else {
        return Ok(EffectsReview {
            checked: false,
            entries: vec![],
        });
    };
    if effects.changes.len() > 80 || effects.notes.len() > 12 {
        return Ok(EffectsReview {
            checked: false,
            entries: vec![],
        });
    }
    let nodes: BTreeMap<_, _> = effects
        .changes
        .iter()
        .filter_map(|e| {
            if let PassageEffect::PutNode { node, .. } = e {
                Some((node.id.as_str(), node))
            } else {
                None
            }
        })
        .collect();
    let mut entries = vec![];
    let mut occurrences = BTreeMap::<String, u32>::new();
    let mut unique_key = |base: String| -> Result<String, String> {
        let count = occurrences.entry(base.clone()).or_default();
        let key = story_hash(&(&base, *count))?;
        *count += 1;
        Ok(key)
    };
    for (index, change) in effects.changes.iter().enumerate() {
        let refs = match change {
            PassageEffect::PutNode { node, .. } => node_references(node, world),
            PassageEffect::PutLink { link, .. } => {
                let mut refs = vec![link.source_id.clone(), link.target_id.clone()];
                if let Some(def) = world.relation_types.iter().find(|d| d.id == link.type_id) {
                    refs.extend(crate::projection::property_references(
                        &def.fields,
                        &link.properties,
                    ));
                }
                refs
            }
            PassageEffect::RemoveLink { .. } => vec![],
        };
        let key = unique_key(story_hash(&(
            "effect-review-v1",
            source,
            &edits.plan,
            &edits.prose,
            "change",
            change,
            dependencies(refs, &nodes, world),
        ))?)?;
        let approved = edits.approved_effects.contains(&key);
        entries.push(EffectReviewEntry {
            kind: EffectReviewKind::Change,
            index: index as u32,
            key,
            approved,
            issues: issues.get(index).cloned().unwrap_or_default(),
        });
    }
    for (index, note) in effects.notes.iter().enumerate() {
        let key = unique_key(story_hash(&(
            "effect-review-v1",
            source,
            &edits.plan,
            &edits.prose,
            "note",
            note,
            dependencies(note.node_ids.clone(), &nodes, world),
        ))?)?;
        let approved = edits.approved_effects.contains(&key);
        entries.push(EffectReviewEntry {
            kind: EffectReviewKind::Note,
            index: index as u32,
            key,
            approved,
            issues: vec![],
        });
    }
    Ok(EffectsReview { checked, entries })
}

#[cfg(test)]
impl Project {
    pub(crate) fn approve_passage_for_test(
        &self,
        id: &str,
        settings: &LlmConnection,
    ) -> Result<PassageView, String> {
        let view = self.passage_view(id, settings)?;
        if view.effects_review.entries.is_empty() {
            return Ok(view);
        }
        let mut edits = view.edits;
        edits.approved_effects = view
            .effects_review
            .entries
            .iter()
            .map(|e| e.key.clone())
            .collect();
        self.save_passage(id, view.version, edits)?;
        self.passage_view(id, settings)
    }
}
