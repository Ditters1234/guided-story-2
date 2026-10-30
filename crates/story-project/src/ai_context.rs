//! Explicit story-wide prompt bindings. They do not grant world-edit scope or
//! traverse links; all source content remains ordinary versioned world nodes.
use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use specta::Type;
use std::collections::BTreeSet;
#[cfg(test)]
pub(crate) mod tests;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AiContextRole {
    Instructions,
    Reference,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum AiContextField {
    Description,
    VisualDescription,
    Components,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiContextBinding {
    pub node_id: String,
    pub enabled: bool,
    pub role: AiContextRole,
    pub tasks: Vec<TextPromptTask>,
    pub fields: Vec<AiContextField>,
    pub order: i32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub image_preparation: bool,
}
fn is_false(value: &bool) -> bool {
    !*value
}
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiContextSection {
    pub node_id: String,
    pub name: String,
    pub role: AiContextRole,
    pub order: i32,
    pub fields: Vec<AiContextField>,
    pub content: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiContextPreview {
    pub sections: Vec<AiContextSection>,
    pub system: String,
    pub references: Vec<AiContextSection>,
}

pub(crate) fn validate_ai_context(world: &World) -> Result<(), String> {
    if world.ai_context.len() > 32 {
        return Err("Keep at most 32 story-wide AI context bindings.".into());
    }
    let mut nodes = BTreeSet::new();
    for binding in &world.ai_context {
        if !world.nodes.iter().any(|n| n.id == binding.node_id) || !nodes.insert(&binding.node_id) {
            return Err("AI context includes a missing or duplicate node. Remove its inclusion before saving.".into());
        }
        if !(-10000..=10000).contains(&binding.order)
            || binding.tasks.iter().collect::<BTreeSet<_>>().len() != binding.tasks.len()
            || binding.fields.iter().collect::<BTreeSet<_>>().len() != binding.fields.len()
        {
            return Err(
                "AI context needs unique tools/fields and an order between -10,000 and 10,000."
                    .into(),
            );
        }
    }
    Ok(())
}
fn content(world: &World, node: &WorldNode, fields: &[AiContextField]) -> Result<String, String> {
    let mut parts = vec![];
    if fields.contains(&AiContextField::Description) && !node.description.trim().is_empty() {
        parts.push(node.description.clone());
    }
    if fields.contains(&AiContextField::VisualDescription)
        && !node.visual_description.trim().is_empty()
    {
        parts.push(format!("Visible description: {}", node.visual_description));
    }
    if fields.contains(&AiContextField::Components) {
        for (id, values) in &node.components {
            if let Some(definition) = world.component_types.iter().find(|d| &d.id == id) {
                for field in &definition.fields {
                    if let Some(value) = values.get(&field.key) {
                        if value != &PropertyValue::Null
                            && value != &PropertyValue::Text(String::new())
                        {
                            let value = if let PropertyValue::Text(text) = value {
                                text.clone()
                            } else {
                                serde_json::to_string(value).map_err(|e| e.to_string())?
                            };
                            parts.push(format!("{} / {}: {}", definition.name, field.label, value));
                        }
                    }
                }
            }
        }
    }
    Ok(parts.join("\n"))
}
pub fn resolve_ai_context(
    world: &World,
    task: TextPromptTask,
) -> Result<Vec<AiContextSection>, String> {
    if matches!(task,TextPromptTask::Passage|TextPromptTask::Wording) && world.ai_context.iter().any(|b|b.enabled&&b.tasks.contains(&task)&&world.nodes.iter().any(|n|n.id==b.node_id&&crate::knowledge::is_claim(n)&&matches!(n.components.get("core.claim").and_then(|v|v.get("disclosure")),Some(PropertyValue::Text(s)) if s=="withheld"))) {
        return Err("Required prose context includes a withheld claim. Change its inclusion or narration access; required context was not silently dropped.".into());
    }
    resolve_context_sections(
        world,
        |b| b.tasks.contains(&task) || (task == TextPromptTask::ImagePrompt && b.image_preparation),
        false,
    )
}
pub(crate) fn resolve_context_sections(
    world: &World,
    selected: impl Fn(&AiContextBinding) -> bool,
    allow_empty: bool,
) -> Result<Vec<AiContextSection>, String> {
    validate_ai_context(world)?;
    let mut sections = vec![];
    let mut bytes = 0usize;
    for binding in world.ai_context.iter().filter(|b| b.enabled && selected(b)) {
        let node = world
            .nodes
            .iter()
            .find(|n| n.id == binding.node_id)
            .ok_or("Included AI context node is missing.")?;
        let text = content(world, node, &binding.fields)?;
        if text.trim().is_empty() && !allow_empty {
            return Err(format!("Included AI context “{}” has no selected content. Add text, select its content fields, or disable its inclusion for this tool.",node.name));
        }
        bytes += text.len();
        if bytes > 131072 {
            return Err(format!("Required AI context exceeds 128 KB at “{}”. Shorten or disable included nodes; no required content was dropped.",node.name));
        }
        sections.push(AiContextSection {
            node_id: node.id.clone(),
            name: node.name.clone(),
            role: binding.role,
            order: binding.order,
            fields: binding.fields.clone(),
            content: text,
        });
    }
    sections.sort_by(|a, b| a.order.cmp(&b.order).then(a.node_id.cmp(&b.node_id)));
    Ok(sections)
}
pub(crate) fn append_ai_context(
    sections: &[AiContextSection],
    system: &mut String,
    input: &mut Value,
) {
    let references = sections
        .iter()
        .filter(|s| s.role == AiContextRole::Reference)
        .collect::<Vec<_>>();
    if !references.is_empty() {
        input["includedReferenceFacts"] = json!(references);
    }
    let instructions = sections
        .iter()
        .filter(|s| s.role == AiContextRole::Instructions)
        .collect::<Vec<_>>();
    if !instructions.is_empty() {
        system.push_str("\n\nStory instruction nodes (in authored order):");
        for section in instructions {
            system.push_str(&format!("\n\n[{}]\n{}", section.name, section.content));
        }
    }
}
pub fn preview_ai_context(
    world: &World,
    task: TextPromptTask,
    settings: &LlmConnection,
) -> Result<AiContextPreview, String> {
    world.validate()?;
    let sections = resolve_ai_context(world, task)?;
    let mut system = settings.system_prompt(task);
    let mut input = json!({});
    append_ai_context(&sections, &mut system, &mut input);
    let references = sections
        .iter()
        .filter(|s| s.role == AiContextRole::Reference)
        .cloned()
        .collect();
    Ok(AiContextPreview {
        sections,
        system,
        references,
    })
}
/// None preserves the old serialized fingerprint when there is no matching
/// active binding; captured sections retain selected source values exactly.
pub(crate) fn ai_context_capture(
    world: &World,
    task: TextPromptTask,
) -> Result<Option<Vec<AiContextSection>>, String> {
    let sections = resolve_ai_context(world, task)?;
    Ok((!sections.is_empty()).then_some(sections))
}
pub(crate) fn story_ai_context_stamp(world: &World) -> Result<Option<String>, String> {
    let mut source = vec![];
    for binding in world.ai_context.iter().filter(|b| {
        b.enabled
            && b.tasks.iter().any(|t| {
                matches!(
                    t,
                    TextPromptTask::Passage | TextPromptTask::Effects | TextPromptTask::Wording
                )
            })
    }) {
        let node = world
            .nodes
            .iter()
            .find(|n| n.id == binding.node_id)
            .ok_or("Included AI context node is missing.")?;
        let mut narrative_binding = binding.clone();
        narrative_binding.image_preparation = false;
        source.push((
            narrative_binding,
            &node.name,
            content(world, node, &binding.fields)?,
        ));
    }
    if source.is_empty() {
        Ok(None)
    } else {
        Ok(Some(crate::story::story_hash(&source)?))
    }
}
