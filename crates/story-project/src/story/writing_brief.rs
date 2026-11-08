use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WritingBrief {
    pub chapter_title: String,
    pub chapter_status: String,
    pub accepted_passages: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chapter_id: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub chapter_remaining: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub already_covered: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub passage_direction: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stop_after: String,
}

pub(super) fn assemble(plan: &PassagePlan, index: &StoryIndex) -> WritingBrief {
    let chapter = index
        .chapters
        .last()
        .filter(|ch| !ch.closed && !plan.start_new_chapter);
    WritingBrief {
        chapter_title: plan.chapter_title.clone(),
        chapter_status: if chapter.is_some() {
            "continuing".into()
        } else {
            "opening".into()
        },
        accepted_passages: chapter.map_or(0, |ch| ch.passage_ids.len() as u32),
        chapter_id: chapter.map(|ch| ch.id.clone()),
        chapter_remaining: plan.chapter_plan.clone(),
        already_covered: plan.chapter_progress.clone(),
        passage_direction: plan.direction.clone(),
        stop_after: plan.stop_after.clone(),
    }
}

pub(super) fn project(input: &mut Value, settings: &LlmConnection, task: TextPromptTask) {
    if settings.prompts.overrides.contains_key(&task) {
        // Keep established input paths for saved custom prompts, using only
        // the current draft's chapter direction rather than an earlier plan.
        let brief = &input["writingBrief"];
        input["currentChapter"] = json!({"id":brief["chapterId"],"title":brief["chapterTitle"],
            "plan":brief["chapterRemaining"],"closed":false});
    } else if let Some(plan) = input["plan"].as_object_mut() {
        for key in ["chapterPlan", "chapterProgress", "direction", "stopAfter"] {
            plan.remove(key);
        }
    }
}
