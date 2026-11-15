//! Read existing authoring nodes for a draft. No world mutations or node creation.
use super::*;

pub(super) const BRIEF_FIELDS: &[&str] = &[
    "chapterDirection",
    "passageDirection",
    "stopAfter",
    "alreadyCovered",
];
const NARRATION_FIELDS: &[&str] = &["viewpointId", "knowledgePerspective"];
pub(crate) const NARRATION_LINKS: &[&str] = &["core.narration-pov", "core.narration-focus"];

pub(crate) fn retired_writing_field(capability: &str, key: &str) -> bool {
    capability == "core.narration-style" && key == "viewpointId"
}

pub(crate) fn strip_retired_writing_values(node: &mut WorldNode) {
    for (capability, values) in &mut node.components {
        values.retain(|key, _| !retired_writing_field(capability, key));
    }
}

pub(crate) fn bundled_writing_relationships() -> Result<Vec<RelationDefinition>, String> {
    crate::model::bundled_json(
        "writing-input-relations.json",
        include_str!("../../../../schema/writing-input-relations.json"),
    )
}

pub(crate) fn validate_writing_relationships(world: &World) -> Result<(), String> {
    for expected in bundled_writing_relationships()? {
        if let Some(saved) = world.relation_types.iter().find(|d| d.id == expected.id) {
            if saved.source_capability != expected.source_capability
                || saved.target_capability != expected.target_capability
                || saved.physical_placement
                || !saved.single_target
            {
                return Err("Narration POV/focus links require their declared source, target and nonphysical single-target contract.".into());
            }
        }
    }
    Ok(())
}

pub(crate) fn bundled_writing_fields() -> Result<BTreeMap<String, Vec<FieldDefinition>>, String> {
    crate::model::bundled_json(
        "writing-input-fields.json",
        include_str!("../../../../schema/writing-input-fields.json"),
    )
}

pub(crate) fn install_bundled_writing_fields(
    world: &mut World,
    only: Option<&str>,
) -> Result<(), String> {
    let mut definitions = bundled_writing_fields()?;
    if let Some(capability) = only {
        if !definitions.contains_key(capability) {
            return Err(format!("Missing bundled writing fields for {capability}."));
        }
        definitions.retain(|id, _| id == capability);
    }
    for (capability, mut fields) in definitions {
        if fields
            .iter()
            .any(|f| f.required || !crate::story_effects::planning_only(f))
        {
            return Err("Writing-input fields must be optional author-planning fields.".into());
        }
        let saved = &mut world
            .component_types
            .iter_mut()
            .find(|d| d.id == capability)
            .ok_or("Writing-input fields need an existing capability.")?
            .fields;
        fields.retain(|field| !saved.contains(field));
        saved.extend(fields);
    }
    Ok(())
}

pub fn validate_bundled_writing_fields() -> Result<(), String> {
    let mut world = World::bundled()?;
    install_bundled_writing_fields(&mut world, None)?;
    world.relation_types.extend(bundled_writing_relationships()?);
    world.validate()
}

fn source<'a>(world: &'a World, id: &str, capability: &str) -> Result<&'a WorldNode, String> {
    world.nodes.iter().find(|n| n.id == id && n.components.contains_key(capability))
        .ok_or_else(|| format!("The selected writing source {id} is missing or no longer has {capability}. Select a source in World."))
}

fn text(node: &WorldNode, capability: &str, field: &str) -> Result<String, String> {
    match node.components[capability].get(field) {
        None | Some(PropertyValue::Null) => Ok(String::new()),
        Some(PropertyValue::Text(value)) => Ok(value.clone()),
        _ => Err(format!("{} needs a text value for {field}.", node.name)),
    }
}

fn passage_key(brief: &Option<String>, direction: &str, stop: &str) -> Result<Option<String>, String> {
    if direction.trim().is_empty() && stop.trim().is_empty() {
        return Ok(None);
    }
    story_hash(&(brief, direction, stop)).map(Some)
}

pub(super) fn brief_passage_key(world: &World, plan: &PassagePlan) -> Result<Option<String>, String> {
    let Some(id) = &plan.brief_node_id else { return Ok(None) };
    let node = source(world, id, "detail.brief")?;
    passage_key(&plan.brief_node_id, &text(node, "detail.brief", "passageDirection")?, &text(node, "detail.brief", "stopAfter")?)
}

pub(super) fn linked_target(world: &World, narration_id: &str, role: &str) -> Option<String> {
    world
        .edges
        .iter()
        .find(|e| e.source_id == narration_id && e.type_id == role)
        .map(|e| e.target_id.clone())
}

pub(super) fn retain_former_viewpoint(world: &World, plan: &mut PassagePlan, id: &str) {
    if plan.participant_ids.len() < 12
        && !plan.participant_ids.iter().any(|participant| participant == id)
        && !plan.reference_ids.iter().any(|reference| reference == id)
        && world
            .nodes
            .iter()
            .any(|n| n.id == id && n.components.contains_key("core.character"))
    {
        plan.participant_ids.insert(0, id.into());
    }
}

pub(super) fn resolve(world: &World, plan: &PassagePlan) -> Result<PassagePlan, String> {
    let mut next = plan.clone();
    next.viewpoint_id = None;
    if let Some(id) = &plan.narration_node_id {
        let node = source(world, id, "core.narration-style")?;
        next.viewpoint_id = linked_target(world, id, NARRATION_LINKS[0]);
        next.location_id = linked_target(world, id, NARRATION_LINKS[1]);
        next.knowledge_perspective =
            match text(node, "core.narration-style", "knowledgePerspective")?.as_str() {
                "" | "Viewpoint / reader" => KnowledgePerspective::Viewpoint,
                "Author / omniscient" => KnowledgePerspective::Author,
                _ => {
                    return Err(format!(
                        "{} has an unsupported narration perspective.",
                        node.name
                    ))
                }
            };
    } else if let Some(id) = &plan.viewpoint_id {
        // Keep a former manual POV in the cast of an older open draft where there is room.
        retain_former_viewpoint(world, &mut next, id);
    }
    next.saved_draft_direction = None;
    if let Some(id) = &plan.brief_node_id {
        let node = source(world, id, "detail.brief")?;
        let authored = plan.saved_draft_direction.clone().unwrap_or_else(|| SavedDraftDirection::from_plan(plan));
        let chapter = text(node, "detail.brief", "chapterDirection")?;
        let progress = text(node, "detail.brief", "alreadyCovered")?;
        let direction = text(node, "detail.brief", "passageDirection")?;
        let stop = text(node, "detail.brief", "stopAfter")?;
        let key = passage_key(&plan.brief_node_id, &direction, &stop)?;
        let consumed = key.is_some() && key == plan.consumed_brief_passage;
        let writing_value = |value: String, fallback: &str, consumed: bool| {
            if value.trim().is_empty() {
                fallback.to_owned()
            } else if consumed {
                String::new()
            } else {
                value
            }
        };
        if [&chapter, &progress, &direction, &stop].iter().any(|value| !value.trim().is_empty()) {
            next.saved_draft_direction = Some(authored.clone());
        }
        next.chapter_plan = writing_value(chapter, &authored.chapter_plan, false);
        next.chapter_progress = writing_value(progress, &authored.chapter_progress, false);
        next.direction = writing_value(direction, &authored.direction, consumed);
        next.stop_after = writing_value(stop, &authored.stop_after, consumed);
    }
    next.participant_ids
        .retain(|id| Some(id) != next.viewpoint_id.as_ref());
    if let Some(id) = &next.viewpoint_id {
        next.reference_ids.retain(|r| r != id);
    }
    validate_edits(&PassageEdits {
        plan: next.clone(),
        prose: String::new(),
        effects_text: empty_effects(),
        close_chapter: false,
        revision_direction: String::new(),
        approved_effects: vec![],
    })?;
    Ok(next)
}

pub(super) fn stamp(world: &World, plan: &PassagePlan) -> Result<Option<String>, String> {
    let mut sources = vec![];
    for (id, capability) in [
        (&plan.narration_node_id, "core.narration-style"),
        (&plan.brief_node_id, "detail.brief"),
    ] {
        if let Some(id) = id {
            let node = source(world, id, capability)?;
            let mut projected = node.clone();
            strip_retired_writing_values(&mut projected);
            sources.push((&node.id, &node.name, &node.description, projected.components));
        }
    }
    if sources.is_empty() {
        Ok(None)
    } else {
        let links: Vec<_> = world
            .edges
            .iter()
            .filter(|e| {
                plan.narration_node_id.as_ref() == Some(&e.source_id)
                    && NARRATION_LINKS.contains(&e.type_id.as_str())
            })
            .collect();
        story_hash(&(sources, links)).map(Some)
    }
}

/// Reuse the context resolver and its budgets/claim rules. Selected instructions
/// appear once; fields consumed by the structured brief/plan are not repeated.
pub(super) fn guidance(
    world: &World,
    plan: &PassagePlan,
    task: TextPromptTask,
) -> Result<Vec<AiContextSection>, String> {
    if plan.narration_node_id.is_none() && plan.brief_node_id.is_none() {
        return crate::knowledge::resolve_narrative_context(world, task, plan);
    }
    let mut scoped = world.clone();
    let excluded = crate::knowledge::excluded_claims(world, plan);
    for (id, capability, consumed) in [
        (
            &plan.narration_node_id,
            "core.narration-style",
            NARRATION_FIELDS,
        ),
        (&plan.brief_node_id, "detail.brief", BRIEF_FIELDS),
    ] {
        let Some(id) = id else { continue };
        if excluded.contains(id) {
            return Err("A selected writing source is excluded by this narration perspective. Change its access or choose another source; no required content was dropped.".into());
        }
        source(world, id, capability)?;
        let node = scoped.nodes.iter_mut().find(|n| &n.id == id).unwrap();
        let values = node.components.get_mut(capability).unwrap();
        let had_writing = consumed.iter().any(|key| {
            values
                .get(*key)
                .is_some_and(crate::story_effects::populated)
        });
        for key in consumed {
            values.remove(*key);
        }
        let content = crate::ai_context::content(
            &scoped,
            scoped.nodes.iter().find(|n| &n.id == id).unwrap(),
            &[AiContextField::Description, AiContextField::Components],
            false,
        )?;
        if content.trim().is_empty() {
            if had_writing {
                scoped.ai_context.retain(|b| &b.node_id != id);
            }
            continue;
        }
        let order = scoped
            .ai_context
            .iter()
            .find(|b| &b.node_id == id)
            .map_or(0, |b| b.order);
        scoped.ai_context.retain(|b| &b.node_id != id);
        scoped.ai_context.push(AiContextBinding {
            node_id: id.clone(),
            enabled: true,
            role: AiContextRole::Instructions,
            tasks: vec![task],
            fields: vec![AiContextField::Description, AiContextField::Components],
            order,
            image_preparation: false,
        });
    }
    crate::knowledge::resolve_narrative_context(&scoped, task, plan)
}

impl Project {
    pub(super) fn resolved_writing_plan(
        &self,
        draft: &PassageDraft,
    ) -> Result<PassagePlan, String> {
        if draft.revision.is_some() || draft.stage == PassageStage::Accepted {
            // Accepted passages and their revisions use the captured plan.
            Ok(draft.edits.plan.clone())
        } else {
            resolve(&self.view()?.world, &draft.edits.plan)
        }
    }
}
