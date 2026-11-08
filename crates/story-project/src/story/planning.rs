use super::*;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum PlanFillKind {
    Scene,
    Chapter,
}

impl PlanFillKind {
    pub fn review_text(self, reply: CompletionText) -> Result<String, String> {
        let text = reply.text.trim();
        let limit = match self {
            Self::Scene => 8000,
            Self::Chapter => 4000,
        };
        if reply.incomplete || text.is_empty() || text.chars().count() > limit {
            return Err("The plan suggestion was cut off, empty or too long. Try again or fill the direction manually.".into());
        }
        Ok(text.into())
    }
}

impl Project {
    pub fn check_plan_fill_source(
        &self,
        id: &str,
        version: u32,
        revision: u32,
    ) -> Result<(), String> {
        self.plan_fill_draft(id, version, revision).map(|_| ())
    }

    fn plan_fill_draft(
        &self,
        id: &str,
        version: u32,
        revision: u32,
    ) -> Result<PassageDraft, String> {
        let draft = self.read_passage_draft(id)?;
        if draft.version != version
            || self.view()?.revision != revision
            || matches!(
                draft.stage,
                PassageStage::Narrating | PassageStage::Checking | PassageStage::Accepted
            )
            || draft
                .revision
                .as_ref()
                .is_some_and(|r| r.kind == PassageRevisionKind::Wording)
        {
            return Err(
                "Open a current, editable scene draft before requesting a plan suggestion.".into(),
            );
        }
        Ok(draft)
    }

    pub fn prepare_plan_fill(
        &self,
        id: &str,
        version: u32,
        revision: u32,
        kind: PlanFillKind,
        settings: &LlmConnection,
    ) -> Result<PreparedPassage, String> {
        settings.ready()?;
        let draft = self.plan_fill_draft(id, version, revision)?;
        let plan = &draft.edits.plan;
        let (context, source, mut input, _, _) = self.draft_input_budget(&draft, settings, 0)?;
        let world: World = self.read_record("worlds", &source)?;
        let task = TextPromptTask::Planning;
        let included = crate::knowledge::resolve_narrative_context(&world, task, plan)?;
        let mut system = settings.system_prompt(task);
        input["planKind"] = json!(kind);
        // Optional history fits after the selected scene facts and required bindings.
        let mut recent = input["recentPassages"].take();
        let input_fields = input.as_object_mut().ok_or("Invalid planning context.")?;
        input_fields.remove("earlierMemory");
        input_fields.remove("recentPassages");
        append_ai_context(&included, &mut system, &mut input);
        scene_context::project_records(&context, plan, &mut input, true);
        let excluded = crate::knowledge::excluded_claims(&world, plan);
        crate::knowledge::scrub_references(&mut input, &excluded);
        crate::knowledge::scrub_references(&mut recent, &excluded);
        writing_brief::project(&mut input, settings, task);
        crate::prompt_context::compact_envelope(&mut input);
        let output_tokens = settings.max_tokens.min(768);
        let fits = |text: &str| {
            (system.len() + text.len()) / 3 + 256 + output_tokens as usize
                <= settings.context_tokens as usize
        };
        let mut user = input.to_string();
        if !fits(&user) {
            return Err("Required planning context exceeds the configured budget. Narrow scene references or shorten included guidance.".into());
        }
        let mut excerpts = vec![];
        for entry in recent.as_array().into_iter().flatten().rev() {
            let prose = entry["prose"].as_str().unwrap_or_default();
            let start = prose.char_indices().rev().nth(2999).map_or(0, |(i, _)| i);
            excerpts.insert(
                0,
                json!({"id":entry["id"],"version":entry["version"],"chapterRelation":entry["chapterRelation"],
                    "title":entry["title"],"endingExcerpt":&prose[start..]}),
            );
            input["recentPassages"] = json!(excerpts);
            let expanded = input.to_string();
            if !fits(&expanded) {
                break;
            }
            user = expanded;
        }
        let demo = match kind {
            PlanFillKind::Scene => "Give the viewpoint character a small decision tied to their present surroundings. End when they commit to a next step.",
            PlanFillKind::Chapter => "Establish the immediate problem, explore a consequence of the characters' first choice, and end on a decision that gives the next chapter direction.",
        };
        Ok(PreparedPassage {
            system,
            user,
            schema: None,
            output_tokens,
            demo: demo.into(),
        })
    }
}
