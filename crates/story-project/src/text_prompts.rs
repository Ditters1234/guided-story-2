//! Editable system messages shared by the text tools. Context/schema assembly
//! and native output validation remain owned by each task's existing builder.
use crate::LlmConnection;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::BTreeMap;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum TextPromptTask {
    World,
    Passage,
    Wording,
    Effects,
    Memory,
    ImagePrompt,
}
impl TextPromptTask {
    pub fn default_text(self) -> &'static str {
        match self {
            Self::World => include_str!("prompts/world.txt"),
            Self::Passage => include_str!("prompts/passage.txt"),
            Self::Wording => include_str!("prompts/wording.txt"),
            Self::Effects => include_str!("prompts/effects.txt"),
            Self::Memory => include_str!("prompts/memory.txt"),
            Self::ImagePrompt => include_str!("prompts/image-prompt.txt"),
        }
        .trim_end()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromptOverrides {
    pub version: u32,
    pub overrides: BTreeMap<TextPromptTask, String>,
}
impl PromptOverrides {
    pub fn is_default(&self) -> bool {
        self.version == 0 && self.overrides.is_empty()
    }
    pub fn validate(&self) -> Result<(), String> {
        let encoded_size = serde_json::to_vec(self).map_err(|e| e.to_string())?.len();
        if self
            .overrides
            .values()
            .any(|s| s.trim().is_empty() || s.len() > 32768)
            || self.overrides.values().map(String::len).sum::<usize>() > 98304
            || encoded_size > 196608
        {
            return Err("Prompts need nonempty instructions, at most 32 KB each and 96 KB in total, within the saved-data limit. Restore default to remove an override.".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextPromptTemplate {
    pub id: TextPromptTask,
    pub label: String,
    pub description: String,
    pub default_text: String,
    pub text: String,
    pub customized: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PromptSettingsView {
    pub version: u32,
    pub templates: Vec<TextPromptTemplate>,
}
impl LlmConnection {
    pub fn system_prompt(&self, task: TextPromptTask) -> String {
        self.prompts
            .overrides
            .get(&task)
            .map_or_else(|| task.default_text().into(), Clone::clone)
    }
    pub fn prompt_settings_view(&self) -> PromptSettingsView {
        let tasks = [
            (
                TextPromptTask::World,
                "World creation & rerolls",
                "Creates individual records or connected groups and revises selected fields.",
            ),
            (
                TextPromptTask::Passage,
                "Passage writing",
                "Writes the first passage and later passages from the scene and accepted history.",
            ),
            (
                TextPromptTask::Wording,
                "Wording edits",
                "Polishes accepted prose while preserving its events.",
            ),
            (
                TextPromptTask::Effects,
                "World-change extraction",
                "Proposes changes and continuity notes from the current passage.",
            ),
            (
                TextPromptTask::Memory,
                "Chapter memory",
                "Summarizes accepted chapter events with source quotes.",
            ),
            (
                TextPromptTask::ImagePrompt,
                "Image-prompt assistance",
                "Proposes shot notes and image prompts from captured appearance and clothing.",
            ),
        ];
        PromptSettingsView {
            version: self.prompts.version,
            templates: tasks
                .into_iter()
                .map(|(id, label, description)| TextPromptTemplate {
                    id,
                    label: label.into(),
                    description: description.into(),
                    default_text: id.default_text().into(),
                    text: self.system_prompt(id),
                    customized: self.prompts.overrides.contains_key(&id),
                })
                .collect(),
        }
    }
}
