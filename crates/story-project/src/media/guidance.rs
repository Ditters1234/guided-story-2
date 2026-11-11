//! Explicit image presentation context, separate from physical appearance facts.
use super::*;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ImageGuidanceSource {
    Source,
    Current,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageContextCapture {
    pub world: RecordRef,
    pub sections: Vec<AiContextSection>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImagePreparationPreview {
    pub sections: Vec<AiContextSection>,
    pub prompt: String,
    pub error: String,
}

fn image_sections(world: &World) -> Result<Vec<AiContextSection>, String> {
    crate::ai_context::resolve_context_sections(world, |b| b.image_preparation, true)
}
fn validate_sections(sections: &[AiContextSection]) -> Result<(), String> {
    for s in sections {
        if s.content.trim().is_empty() {
            return Err(format!("Included image guidance “{}” has no selected content. Fill its fields, disable its inclusion or select different content before generation.",s.name));
        }
    }
    Ok(())
}
fn render_sections(sections: &[AiContextSection]) -> String {
    let mut parts = vec![];
    for section in sections {
        let role = if section.role == AiContextRole::Instructions {
            "Image presentation instructions"
        } else {
            "Included image reference facts"
        };
        parts.push(format!("{role} [{}]:\n{}", section.name, section.content));
    }
    parts.join("\n\n")
}
pub fn preview_image_context(world: &World) -> Result<AiContextPreview, String> {
    world.validate()?;
    let sections = image_sections(world)?;
    validate_sections(&sections)?;
    Ok(AiContextPreview {
        system: render_sections(&sections),
        references: sections
            .iter()
            .filter(|s| s.role == AiContextRole::Reference)
            .cloned()
            .collect(),
        sections,
    })
}
pub(crate) fn image_preparation(
    source: &MediaSource,
    request: &ImageWorkflowRequest,
) -> ImagePreparationPreview {
    let sections = source
        .image_context
        .as_ref()
        .map(|c| c.sections.clone())
        .unwrap_or_default();
    let guidance = render_sections(&sections);
    let prompt = if guidance.is_empty() {
        request.prompt.clone()
    } else {
        format!(
            "{guidance}\n\nImage prompt (preserve captured identity, outfit and coverage):\n{}",
            request.prompt
        )
    };
    let error = if let Err(error) = validate_sections(&sections) {
        error
    } else if prompt.chars().count() > 16000 {
        format!("Image guidance ({}) and the image prompt exceed 16,000 characters. Shorten the included content or prompt; no required guidance was dropped.",sections.iter().map(|s|s.name.as_str()).collect::<Vec<_>>().join(", "))
    } else {
        String::new()
    };
    ImagePreparationPreview {
        sections,
        prompt,
        error,
    }
}
pub(crate) fn validate_image_context_capture(
    capture: &ImageContextCapture,
    read_world: impl Fn(&RecordRef) -> Result<World, String>,
) -> Result<(), String> {
    let world = read_world(&capture.world)?;
    world.validate()?;
    if image_sections(&world)? != capture.sections {
        return Err("Captured image guidance does not match its saved source world.".into());
    }
    Ok(())
}
pub(crate) fn validate_image_guidance_boundary(
    target: &MediaTarget,
    source: &MediaSource,
) -> Result<(), String> {
    if target.passage_id.is_some()
        && target.guidance_source != Some(ImageGuidanceSource::Current)
        && source
            .image_context
            .as_ref()
            .is_some_and(|c| c.world != source.world)
    {
        return Err("Saved-passage image guidance must use that captured passage world. Current guidance requires an explicit current-world selection.".into());
    }
    Ok(())
}
impl Project {
    pub(crate) fn validate_image_capture(
        &self,
        target: &MediaTarget,
        source: &MediaSource,
    ) -> Result<(), String> {
        validate_image_guidance_boundary(target, source)?;
        if let Some(capture) = &source.image_context {
            validate_image_context_capture(capture, |r| self.read_record("worlds", r))?;
        }
        Ok(())
    }
    pub fn preview_image_preparation(
        &self,
        id: &str,
        accepted: bool,
    ) -> Result<ImagePreparationPreview, String> {
        if accepted {
            let asset = self.media_asset(id)?;
            Ok(image_preparation(&asset.source, &asset.request))
        } else {
            let draft = self.media_draft(id)?;
            Ok(image_preparation(&draft.source, &draft.request))
        }
    }
    pub(crate) fn image_context_world(
        &self,
        target: &MediaTarget,
        source: &MediaSource,
    ) -> Result<(RecordRef, World), String> {
        if target.node_id.is_some() || target.guidance_source == Some(ImageGuidanceSource::Current)
        {
            let current = self.view()?;
            Ok((current.world_ref, current.world))
        } else {
            Ok((
                source.world.clone(),
                self.read_record("worlds", &source.world)?,
            ))
        }
    }
    pub(crate) fn capture_image_context(
        &self,
        target: &MediaTarget,
        source: &MediaSource,
    ) -> Result<Option<ImageContextCapture>, String> {
        let (world_ref, world) = self.image_context_world(target, source)?;
        let sections = image_sections(&world)?;
        Ok((!sections.is_empty()).then_some(ImageContextCapture {
            world: world_ref,
            sections,
        }))
    }
    pub(crate) fn image_guidance_current(
        &self,
        target: &MediaTarget,
        source: &MediaSource,
    ) -> Result<bool, String> {
        let current = self.capture_image_context(target, source)?;
        Ok(current.as_ref().map(|c| &c.sections)
            == source.image_context.as_ref().map(|c| &c.sections))
    }
    pub fn refresh_image_guidance(&self, id: &str, version: u32) -> Result<MediaView, String> {
        let mut draft = self.media_draft(id)?;
        if draft.stage != MediaStage::Draft {
            return Err("Only an unsent image candidate can refresh guidance. Submitted jobs keep their exact original input.".into());
        }
        if !self.media_applicable(&draft.target, &draft.source) {
            return Err("Source appearance or passage changed. Create a new shot instead of rebinding its facts.".into());
        }
        let context = self.capture_image_context(&draft.target, &draft.source)?;
        let unchanged = context.as_ref().map(|c| &c.sections)
            == draft.source.image_context.as_ref().map(|c| &c.sections);
        if draft.version != version {
            if draft.version == version + 1 && unchanged {
                return self.media_view(id);
            }
            return Err("The image candidate changed. Reload before refreshing guidance.".into());
        }
        if unchanged {
            return self.media_view(id);
        }
        draft.source.image_context = context;
        draft.version += 1;
        self.write_media(&draft)?;
        self.media_view(id)
    }
}
#[cfg(test)]
mod tests;
