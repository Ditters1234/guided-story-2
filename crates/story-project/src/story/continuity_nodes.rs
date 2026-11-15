use super::*;

impl Project {
    pub(super) fn excluded_narrative_nodes(
        &self,
        world: &World,
        context: &World,
        plan: &PassagePlan,
        task: TextPromptTask,
    ) -> Result<BTreeSet<String>, String> {
        let mut excluded = crate::knowledge::excluded_claims(world, plan);
        if excluded.is_empty() {
            return Ok(excluded);
        }
        let required: BTreeSet<_> = world
            .ai_context
            .iter()
            .filter(|binding| binding.enabled && binding.tasks.contains(&task))
            .map(|binding| &binding.node_id)
            .chain(plan.narration_node_id.iter())
            .chain(plan.brief_node_id.iter())
            .collect();
        let notes: BTreeSet<_> = context
            .nodes
            .iter()
            .chain(
                world
                    .nodes
                    .iter()
                    .filter(|node| required.contains(&node.id)),
            )
            .filter(|node| node.type_id == "core.note")
            .map(|node| &node.id)
            .collect();
        let citations: Vec<_> = world
            .source_annotations
            .iter()
            .filter_map(|annotation| match &annotation.origin {
                SourceOrigin::Passage { passage_id, .. } if notes.contains(&annotation.node_id) => {
                    Some((&annotation.node_id, passage_id))
                }
                _ => None,
            })
            .collect();
        if citations.is_empty() {
            return Ok(excluded);
        }
        // Apply the same passage-level boundary used for recent history.
        let passages: BTreeSet<_> = citations.iter().map(|(_, passage)| *passage).collect();
        let mut omitted = crate::knowledge::excluded_claim_passages(world, plan);
        for entry in self
            .story_index()?
            .passages
            .iter()
            .filter(|entry| passages.contains(&entry.id))
        {
            let passage: AcceptedPassage = self.read_record("passages", &entry.record)?;
            if crate::knowledge::mentions_excluded(&passage.effects, &excluded)? {
                omitted.insert(entry.id.clone());
            }
        }
        for (note, passage) in citations {
            if omitted.contains(passage) {
                if required.contains(note) {
                    return Err("Required AI context includes a note from a passage excluded by claim access. Change the note's inclusion or the scene perspective.".into());
                }
                excluded.insert(note.clone());
            }
        }
        Ok(excluded)
    }
}

pub(super) fn preview_draft_effects(
    source: &World,
    context: &World,
    edits: &PassageEdits,
    effects: &PassageEffects,
    allocations: &mut BTreeMap<String, String>,
) -> Result<(World, Vec<String>, Vec<String>), String> {
    let (mut world, mut labels) =
        preview_passage_effects(source, context, &edits.prose, effects, allocations)?;
    let Some(brief_id) = edits
        .plan
        .brief_node_id
        .as_ref()
        .filter(|_| !effects.notes.is_empty())
    else {
        return Ok((world, labels, vec![]));
    };
    let brief = world
        .nodes
        .iter()
        .find(|node| node.id == *brief_id && node.components.contains_key("detail.brief"))
        .ok_or("Select an existing Story brief for the continuity updates.")?;
    let brief_name = brief.name.clone();
    let event_quotes: usize = effects
        .changes
        .iter()
        .map(|change| match change {
            PassageEffect::PutNode { node, evidence }
                if node.components.contains_key("detail.event") =>
            {
                evidence.quotes().len()
            }
            _ => 0,
        })
        .sum();
    if world.source_annotations.len() + event_quotes + effects.notes.len() > 10_000 {
        return Err("These continuity updates and events exceed the story's source citation limit. Remove a proposal or an older citation before validating.".into());
    }
    if !world
        .node_types
        .iter()
        .any(|preset| preset.id == "core.note")
    {
        let preset = World::bundled()?
            .node_types
            .into_iter()
            .find(|preset| preset.id == "core.note")
            .ok_or("The bundled Note preset is unavailable.")?;
        world.node_types.push(preset);
    }
    let related = world
        .relation_types
        .iter()
        .find(|relation| relation.id == "core.related")
        .ok_or("The related connection type is unavailable.")?;
    let properties = crate::default_field_values(&related.fields);
    let mut note_ids = Vec::with_capacity(effects.notes.len());
    for note in &effects.notes {
        let id = uuid::Uuid::new_v4().to_string();
        let text = note.text.split_whitespace().collect::<Vec<_>>().join(" ");
        let mut name: String = text.chars().take(72).collect();
        if name.len() < text.len() {
            name.push('…');
        }
        labels.push(format!("Add continuity update to {brief_name}: {name}"));
        world.nodes.push(WorldNode {
            id: id.clone(),
            type_id: "core.note".into(),
            name,
            description: note.text.clone(),
            components: BTreeMap::new(),
        });
        world.edges.push(WorldEdge {
            id: uuid::Uuid::new_v4().to_string(),
            type_id: "core.related".into(),
            source_id: id.clone(),
            target_id: brief_id.clone(),
            description: "Continuity update for this brief.".into(),
            properties: properties.clone(),
        });
        note_ids.push(id);
    }
    world.validate()?;
    Ok((world, labels, note_ids))
}
