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
        let mut draft = self.plan_fill_draft(id, version, revision)?;
        draft.edits.plan = self.resolved_writing_plan(&draft)?;
        let plan = &draft.edits.plan;
        let (context, source, mut input, _, _) = self.draft_input_budget(&draft, settings, 0)?;
        let world: World = self.read_record("worlds", &source)?;
        let task = TextPromptTask::Planning;
        let compact = !settings.prompts.overrides.contains_key(&task);
        let excluded = self.excluded_narrative_nodes(&world, &context, plan, task)?;
        let included = super::writing_inputs::guidance(&world, plan, task)?;
        let mut system = settings.system_prompt(task);
        input["planKind"] = json!(kind);
        // The exact latest ending is a required handoff. Older memory and
        // additional excerpts fit after scene facts, bindings and output.
        if compact {
            continuity::split_recent_handoff(&mut input);
        }
        let mut recent = input["recentPassages"].take();
        let input_fields = input.as_object_mut().ok_or("Invalid planning context.")?;
        input_fields.remove("recentPassages");
        if input["earlierMemory"]["coverage"]["olderPassages"].as_u64() == Some(0) {
            input.as_object_mut().unwrap().remove("earlierMemory");
        }
        append_ai_context(&included, &mut system, &mut input);
        scene_context::project_records(&context, plan, &mut input, true, &excluded, compact);
        crate::knowledge::scrub_references(&mut input, &excluded);
        crate::knowledge::scrub_references(&mut recent, &excluded);
        writing_brief::project(&mut input, settings, task);
        continuity::project_prompt_sources(&mut input);
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
        let available_bytes = (settings.context_tokens.saturating_sub(output_tokens + 256)
            as usize)
            .saturating_mul(3);
        let memory_budget = available_bytes
            .saturating_sub(system.len() + user.len() + 128)
            .min(default_memory_budget() as usize) as u32;
        if memory_budget > 0 && input.get("earlierMemory").is_some() {
            let (_, _, expanded, _, _) =
                self.draft_input_budget(&draft, settings, memory_budget)?;
            input["earlierMemory"] = expanded["earlierMemory"].clone();
            crate::knowledge::scrub_references(&mut input, &excluded);
            continuity::project_prompt_sources(&mut input);
            user = input.to_string();
        }
        let mut excerpts = vec![];
        for entry in recent.as_array().into_iter().flatten().rev() {
            let mut excerpt = json!({"id":entry["id"],"chapterRelation":entry["chapterRelation"],
                "title":entry["title"],"notes":entry["notes"],
                "evidenceFromEarlierWording":entry["evidenceFromEarlierWording"]});
            if let Some(prose) = entry["prose"].as_str() {
                let start = prose.char_indices().rev().nth(2999).map_or(0, |(i, _)| i);
                let field = if compact && input["passageHandoff"]["passageId"] == entry["id"] {
                    "precedingExcerpt"
                } else {
                    "endingExcerpt"
                };
                excerpt[field] = json!(&prose[start..]);
            }
            excerpts.insert(0, excerpt);
            input["recentPassages"] = json!(excerpts);
            let expanded = input.to_string();
            if !fits(&expanded) {
                break;
            }
            user = expanded;
        }
        if !fits(&user) {
            return Err("Required planning context exceeds the configured budget. Narrow scene references or shorten included guidance.".into());
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
