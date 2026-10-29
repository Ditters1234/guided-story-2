//! Reviewed media is metadata, never a change to world facts or accepted prose.
use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use specta::Type;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Cursor, Read},
    path::Path,
};
mod client;
mod guidance;
mod prompts;
mod settings;
pub use client::*;
pub use guidance::*;
pub use prompts::*;
pub use settings::*;
const MAX_IMAGE: usize = 8 * 1024 * 1024;
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ImageMoment {
    Before,
    After,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MediaTarget {
    pub node_id: Option<String>,
    pub passage_id: Option<String>,
    pub moment: ImageMoment,
    pub subject_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guidance_source: Option<ImageGuidanceSource>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShotFact {
    pub node_id: String,
    pub name: String,
    pub appearance: String,
    pub outfit: Vec<String>,
    pub fingerprint: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MediaSource {
    pub world: RecordRef,
    pub passage: Option<RecordRef>,
    pub facts: Vec<ShotFact>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_context: Option<ImageContextCapture>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum MediaStage {
    Draft,
    Submitting,
    Unknown,
    Queued,
    Ready,
    Failed,
    Accepted,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageFile {
    pub id: String,
    pub sha256: String,
    pub extension: String,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MediaSubmission {
    pub endpoint: String,
    pub timeout_seconds: u32,
    pub profile: ImageWorkflowProfile,
    #[specta(type = WorkflowJson)]
    pub graph: Value,
    pub prompt_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MediaDraft {
    pub schema_version: u32,
    pub id: String,
    pub story_id: String,
    pub version: u32,
    pub target: MediaTarget,
    pub source: MediaSource,
    pub request: ImageWorkflowRequest,
    pub stage: MediaStage,
    pub submission: Option<MediaSubmission>,
    pub image: Option<ImageFile>,
    pub error: String,
    pub imported: bool,
    #[serde(default)]
    pub prompt_origin: Option<ImagePromptOrigin>,
    #[serde(default)]
    pub prompt_receipts: BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MediaAsset {
    pub schema_version: u32,
    pub id: String,
    pub target: MediaTarget,
    pub source: MediaSource,
    pub request: ImageWorkflowRequest,
    pub submission: Option<MediaSubmission>,
    pub image: ImageFile,
    pub imported: bool,
    #[serde(default)]
    pub prompt_origin: Option<ImagePromptOrigin>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MediaEntry {
    pub id: String,
    pub label: String,
    pub stage: MediaStage,
    pub applicable: bool,
    pub error: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MediaView {
    pub draft: MediaDraft,
    pub stale: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub guidance_stale: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preparation: Option<ImagePreparationPreview>,
}

fn shot_fact(world: &World, id: &str) -> Result<ShotFact, String> {
    let node = world
        .nodes
        .iter()
        .find(|n| n.id == id)
        .ok_or("A selected image subject is not in this source state.")?;
    let mut worn: Vec<_> = world
        .edges
        .iter()
        .filter(|e| e.type_id == "core.worn-by" && e.target_id == id)
        .filter_map(|e| {
            world
                .nodes
                .iter()
                .find(|n| n.id == e.source_id)
                .map(|n| (n, e))
        })
        .collect();
    let layer = |node: &WorldNode| match node
        .components
        .get("core.wearable")
        .and_then(|c| c.get("layer"))
    {
        Some(PropertyValue::Number(n)) => *n,
        _ => 0.0,
    };
    worn.sort_by(|a, b| layer(a.0).total_cmp(&layer(b.0)).then(a.0.id.cmp(&b.0.id)));
    let covered_regions: Vec<String> = worn
        .iter()
        .filter_map(|(node, _)| {
            node.components
                .get("core.wearable")
                .and_then(|v| v.get("regions"))
        })
        .filter_map(|v| {
            if let PropertyValue::Text(s) = v {
                Some(s)
            } else {
                None
            }
        })
        .flat_map(|s| {
            s.split([',', ';'])
                .map(|part| part.trim().to_lowercase())
                .filter(|s| !s.is_empty())
        })
        .collect();
    let appearance = visual_summary(world, node, &covered_regions);
    let outfit: Vec<String> = worn
        .iter()
        .map(|(n, e)| {
            format!(
                "{}: {}{}{}",
                n.name,
                visual_summary(world, n, &[]),
                n.components
                    .get("core.wearable")
                    .map(|details| format!(
                        " (coverage: {}; layer {})",
                        match details.get("regions") {
                            Some(PropertyValue::Text(s)) => s.as_str(),
                            _ => "unspecified",
                        },
                        layer(n)
                    ))
                    .unwrap_or_default(),
                {
                    let details = visual_link_summary(world, e);
                    if details.is_empty() {
                        String::new()
                    } else {
                        format!(" ({details})")
                    }
                }
            )
        })
        .collect();
    let fingerprint = hash(
        &serde_json::to_vec(&(
            &node.name,
            &appearance,
            &outfit,
            worn.iter()
                .map(|(n, e)| {
                    (
                        &n.id,
                        &n.name,
                        &n.visual_description,
                        n.components.get("core.wearable"),
                        &e.description,
                    )
                })
                .collect::<Vec<_>>(),
        ))
        .map_err(|e| e.to_string())?,
    );
    Ok(ShotFact {
        node_id: id.into(),
        name: node.name.clone(),
        appearance,
        outfit,
        fingerprint,
    })
}
fn validate_file(file: &ImageFile) -> Result<(), String> {
    validate_id(&file.id)?;
    if !["png", "jpg", "webp"].contains(&file.extension.as_str())
        || file.sha256.len() != 64
        || !file.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        || file.width == 0
        || file.height == 0
        || file.width > 4096
        || file.height > 4096
    {
        return Err("Invalid saved image metadata.".into());
    }
    Ok(())
}
fn decode_image(bytes: &[u8]) -> Result<(image::DynamicImage, &'static str), String> {
    if bytes.len() > MAX_IMAGE {
        return Err("Images must be at most 8 MB.".into());
    }
    let format = image::guess_format(bytes).map_err(|_| "Choose a PNG, JPEG or WebP image.")?;
    let ext = match format {
        image::ImageFormat::Png => "png",
        image::ImageFormat::Jpeg => "jpg",
        image::ImageFormat::WebP => "webp",
        _ => return Err("Choose a PNG, JPEG or WebP image.".into()),
    };
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    Ok((
        reader
            .decode()
            .map_err(|_| "The image is invalid or exceeds the 4096-pixel decode limit.")?,
        ext,
    ))
}
pub(crate) fn verify_backup_image(bytes: &[u8], file: &ImageFile) -> Result<(), String> {
    validate_file(file)?;
    if hash(bytes) != file.sha256 {
        return Err("Backup image checksum mismatch.".into());
    }
    let (image, extension) = decode_image(bytes)?;
    if image.width() != file.width || image.height() != file.height || extension != file.extension {
        return Err("Backup image dimensions or format do not match its metadata.".into());
    }
    Ok(())
}
impl Project {
    fn media_path(&self, id: &str) -> Result<std::path::PathBuf, String> {
        validate_id(id)?;
        Ok(self
            .root_path()
            .join("media-drafts")
            .join(format!("{id}.json")))
    }
    fn write_media(&self, d: &MediaDraft) -> Result<(), String> {
        fs::create_dir_all(self.root_path().join("media-drafts")).map_err(|e| e.to_string())?;
        let bytes = serde_json::to_vec_pretty(d).map_err(|e| e.to_string())?;
        if bytes.len() > 2_000_000 {
            return Err("Image candidate exceeds 2 MB.".into());
        }
        crate::store::atomic_write(&self.media_path(&d.id)?, &bytes)
    }
    pub fn media_draft(&self, id: &str) -> Result<MediaDraft, String> {
        let mut d: MediaDraft =
            serde_json::from_slice(&crate::store::read_bytes(&self.media_path(id)?)?)
                .map_err(|e| format!("Invalid image candidate; data kept: {e}"))?;
        if d.id != id || d.schema_version != 1 || d.story_id != self.view()?.story_id {
            return Err("Image candidate belongs to another story/version.".into());
        }
        self.validate_image_capture(&d.target, &d.source)?;
        if self.story_index()?.media.contains_key(id) && d.stage != MediaStage::Accepted {
            d.stage = MediaStage::Accepted;
            d.version += 1;
            self.write_media(&d)?;
        }
        Ok(d)
    }
    fn resolve_media(&self, target: &MediaTarget) -> Result<MediaSource, String> {
        let mut source = self.resolve_media_facts(target)?;
        source.image_context = self.capture_image_context(target, &source)?;
        Ok(source)
    }
    fn resolve_media_facts(&self, target: &MediaTarget) -> Result<MediaSource, String> {
        if target.node_id.is_some() == target.passage_id.is_some() || target.subject_ids.len() > 4 {
            return Err(
                "Choose a world node or accepted passage and up to four image subjects.".into(),
            );
        }
        let (world_ref, passage) = if let Some(id) = &target.passage_id {
            let index = self.story_index()?;
            let entry = index
                .passages
                .iter()
                .find(|p| &p.id == id)
                .ok_or("Choose an accepted passage.")?;
            let record: AcceptedPassage = self.read_record("passages", &entry.record)?;
            (
                if target.moment == ImageMoment::Before {
                    record.before_world
                } else {
                    record.after_world
                },
                Some(entry.record.clone()),
            )
        } else {
            (self.view()?.world_ref, None)
        };
        let world: World = self.read_record("worlds", &world_ref)?;
        let ids = if let Some(id) = &target.node_id {
            if !target.subject_ids.is_empty() {
                return Err("A node image uses only its selected node.".into());
            }
            vec![id.clone()]
        } else {
            target.subject_ids.clone()
        };
        let mut seen = BTreeSet::new();
        let facts = ids
            .iter()
            .map(|id| {
                if !seen.insert(id) {
                    return Err("Image subjects must be distinct.".into());
                }
                shot_fact(&world, id)
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(MediaSource {
            image_context: None,
            world: world_ref,
            passage,
            facts,
        })
    }
    fn media_applicable(&self, target: &MediaTarget, source: &MediaSource) -> bool {
        self.resolve_media_facts(target)
            .is_ok_and(|current| current.passage == source.passage && current.facts == source.facts)
    }
    pub fn create_media(&self, id: &str, target: MediaTarget) -> Result<MediaView, String> {
        if self.media_path(id)?.exists() {
            let d = self.media_draft(id)?;
            if d.target != target {
                return Err("Image candidate ID reused for another target.".into());
            }
            return self.media_view(id);
        }
        if self
            .list_media(None)?
            .iter()
            .filter(|e| e.stage != MediaStage::Accepted)
            .count()
            >= 100
        {
            return Err("Image candidate limit reached (100).".into());
        }
        let source = self.resolve_media(&target)?;
        let mut prompt = source
            .facts
            .iter()
            .map(|f| {
                format!(
                    "{}. {}{}",
                    f.name,
                    f.appearance,
                    if f.outfit.is_empty() {
                        String::new()
                    } else {
                        format!(" Wearing: {}.", f.outfit.join("; "))
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        if !source.facts.is_empty() {
            prompt.push_str("\nRespect the described clothing and coverage. Show only details visible in the chosen framing.");
        }
        let prompt_error = if prompt.chars().count() > 16000 {
            prompt.clear();
            "Captured appearance is too long for a default prompt. Write a shorter prompt; the source facts are kept.".into()
        } else {
            String::new()
        };
        let d = MediaDraft {
            schema_version: 1,
            id: id.into(),
            story_id: self.view()?.story_id,
            version: 1,
            target,
            source,
            request: ImageWorkflowRequest {
                prompt,
                ..Default::default()
            },
            stage: MediaStage::Draft,
            submission: None,
            image: None,
            error: prompt_error,
            imported: false,
            prompt_origin: None,
            prompt_receipts: BTreeMap::new(),
        };
        self.write_media(&d)?;
        self.media_view(id)
    }
    pub fn media_view(&self, id: &str) -> Result<MediaView, String> {
        let draft = self.media_draft(id)?;
        let stale = !self.media_applicable(&draft.target, &draft.source);
        let guidance_stale = self
            .image_guidance_current(&draft.target, &draft.source)
            .map_or(true, |current| !current);
        let preparation = Some(image_preparation(&draft.source, &draft.request));
        Ok(MediaView {
            draft,
            stale,
            guidance_stale,
            preparation,
        })
    }
    pub fn save_media_request(
        &self,
        id: &str,
        version: u32,
        request: ImageWorkflowRequest,
    ) -> Result<MediaView, String> {
        let mut d = self.media_draft(id)?;
        if d.version == version + 1 && d.request == request {
            return self.media_view(id);
        }
        if d.version != version || d.stage != MediaStage::Draft {
            return Err(
                "Only an unsent image candidate can be edited. Create a new candidate to reroll."
                    .into(),
            );
        }
        if request.prompt.len() > 64000
            || request.negative.as_ref().is_some_and(|p| p.len() > 64000)
        {
            return Err("Image prompts exceed their size limit.".into());
        }
        d.request = request;
        d.error.clear();
        d.version += 1;
        self.write_media(&d)?;
        self.media_view(id)
    }
    pub fn prepare_media(
        &self,
        id: &str,
        version: u32,
        settings: &ImageConnection,
    ) -> Result<MediaSubmission, String> {
        let d = self.media_draft(id)?;
        if d.version != version || d.stage != MediaStage::Draft {
            return Err("Save a new unsent candidate before submitting.".into());
        }
        if !self.media_applicable(&d.target, &d.source) {
            return Err("The source appearance or passage changed. Create a new image candidate; the old one was kept.".into());
        }
        if !self.image_guidance_current(&d.target, &d.source)? {
            return Err("Image guidance changed. Refresh guidance or create a new shot; your manual prompt and controls are retained.".into());
        }
        let preview = image_preparation(&d.source, &d.request);
        if !preview.error.is_empty() {
            return Err(preview.error);
        }
        let mut effective_request = d.request.clone();
        effective_request.prompt = preview.prompt;
        settings.validate()?;
        Ok(MediaSubmission {
            endpoint: settings.base_url.clone(),
            timeout_seconds: settings.timeout_seconds,
            profile: settings.profile.clone(),
            graph: settings.profile.prepare(&effective_request)?,
            prompt_id: None,
        })
    }
    pub fn begin_media_submission(
        &self,
        id: &str,
        version: u32,
        submission: MediaSubmission,
    ) -> Result<(), String> {
        let d = self.media_draft(id)?;
        let settings = ImageConnection {
            schema_version: 1,
            version: 0,
            base_url: submission.endpoint.clone(),
            timeout_seconds: submission.timeout_seconds,
            profile: submission.profile.clone(),
        };
        let expected = self.prepare_media(id, version, &settings)?;
        if expected != submission {
            return Err("The prepared image workflow changed.".into());
        }
        let mut d = d;
        d.submission = Some(submission);
        d.stage = MediaStage::Submitting;
        d.error.clear();
        d.version += 1;
        self.write_media(&d)
    }
    pub fn finish_media_submission(
        &self,
        id: &str,
        outcome: &ComfySubmission,
    ) -> Result<(), String> {
        let mut d = self.media_draft(id)?;
        if d.stage != MediaStage::Submitting && d.stage != MediaStage::Unknown {
            if matches!(outcome,ComfySubmission::Queued(prompt) if d.submission.as_ref().and_then(|s|s.prompt_id.as_ref())==Some(prompt))
            {
                return Ok(());
            }
            if matches!(outcome,ComfySubmission::Rejected(message) if d.stage==MediaStage::Failed&&&d.error==message)
                || matches!(outcome,ComfySubmission::Unknown(_) if d.submission.as_ref().and_then(|s|s.prompt_id.as_ref()).is_some())
            {
                return Ok(());
            }
            return Err("This image candidate is no longer submitting.".into());
        }
        match outcome {
            ComfySubmission::Queued(prompt) => {
                validate_remote_id(prompt)?;
                d.submission
                    .as_mut()
                    .ok_or("Missing submitted workflow.")?
                    .prompt_id = Some(prompt.clone());
                d.stage = MediaStage::Queued;
                d.error.clear();
            }
            ComfySubmission::Rejected(message) => {
                d.stage = MediaStage::Failed;
                d.error = message.clone();
            }
            ComfySubmission::Unknown(message) => {
                d.stage = MediaStage::Unknown;
                d.error = message.clone();
            }
        }
        d.version += 1;
        self.write_media(&d)
    }
    pub fn attach_media_prompt(
        &self,
        id: &str,
        version: u32,
        prompt_id: String,
    ) -> Result<MediaView, String> {
        validate_remote_id(&prompt_id)?;
        let mut d = self.media_draft(id)?;
        if d.version != version || d.stage != MediaStage::Unknown {
            return Err("Only an uncertain submission can attach a recovered prompt ID.".into());
        }
        d.submission
            .as_mut()
            .ok_or("Missing submitted workflow.")?
            .prompt_id = Some(prompt_id);
        d.stage = MediaStage::Queued;
        d.error.clear();
        d.version += 1;
        self.write_media(&d)?;
        self.media_view(id)
    }
    fn save_image_bytes(&self, bytes: &[u8]) -> Result<ImageFile, String> {
        let (image, ext) = decode_image(bytes)?;
        let file = ImageFile {
            id: uuid::Uuid::new_v4().to_string(),
            sha256: hash(bytes),
            extension: ext.into(),
            width: image.width(),
            height: image.height(),
        };
        let dir = self.root_path().join("images");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        crate::store::atomic_write(&dir.join(format!("{}.{}", file.id, file.extension)), bytes)?;
        Ok(file)
    }
    fn read_image_bytes(&self, file: &ImageFile) -> Result<Vec<u8>, String> {
        validate_file(file)?;
        let bytes = crate::store::read_bytes(
            &self
                .root_path()
                .join("images")
                .join(format!("{}.{}", file.id, file.extension)),
        )?;
        if bytes.len() > MAX_IMAGE || hash(&bytes) != file.sha256 {
            return Err("Saved image is missing, changed or incomplete; files were kept.".into());
        }
        Ok(bytes)
    }
    pub fn finish_media_image(
        &self,
        id: &str,
        version: u32,
        result: &ComfyResult,
    ) -> Result<MediaView, String> {
        let mut d = self.media_draft(id)?;
        if d.version > version {
            let already_saved = match result {
                ComfyResult::Pending => matches!(
                    d.stage,
                    MediaStage::Queued
                        | MediaStage::Ready
                        | MediaStage::Accepted
                        | MediaStage::Failed
                ),
                ComfyResult::Failed(message) => {
                    d.stage == MediaStage::Failed && &d.error == message
                }
                ComfyResult::Image(bytes) => {
                    matches!(d.stage, MediaStage::Ready | MediaStage::Accepted)
                        && d.image
                            .as_ref()
                            .is_some_and(|file| file.sha256 == hash(bytes))
                }
            };
            if already_saved {
                return self.media_view(id);
            }
        }
        if d.version != version || d.stage != MediaStage::Queued {
            return Err("This image candidate changed; reload its saved status.".into());
        }
        match result {
            ComfyResult::Pending => (),
            ComfyResult::Failed(message) => {
                d.stage = MediaStage::Failed;
                d.error = message.clone();
            }
            ComfyResult::Image(bytes) => {
                d.image = Some(self.save_image_bytes(bytes)?);
                d.stage = MediaStage::Ready;
                d.error.clear();
            }
        }
        d.version += 1;
        self.write_media(&d)?;
        self.media_view(id)
    }
    pub fn import_media_image(
        &self,
        id: &str,
        version: u32,
        path: &Path,
    ) -> Result<MediaView, String> {
        let mut d = self.media_draft(id)?;
        if d.version != version || d.stage != MediaStage::Draft {
            return Err("Import into a new unsent candidate.".into());
        }
        let mut bytes = vec![];
        fs::File::open(path)
            .map_err(|_| "The selected image could not be opened.")?
            .take((MAX_IMAGE + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        d.image = Some(self.save_image_bytes(&bytes)?);
        d.imported = true;
        d.stage = MediaStage::Ready;
        d.version += 1;
        self.write_media(&d)?;
        self.media_view(id)
    }
    pub fn media_asset(&self, id: &str) -> Result<MediaAsset, String> {
        let index = self.story_index()?;
        let reference = index
            .media
            .get(id)
            .ok_or("This image has not been accepted.")?;
        let asset: MediaAsset = self.read_record("media", reference)?;
        if asset.schema_version != 1 || asset.id != id {
            return Err("Invalid accepted image metadata.".into());
        }
        validate_file(&asset.image)?;
        self.validate_image_capture(&asset.target, &asset.source)?;
        Ok(asset)
    }
    pub fn media_preview(&self, id: &str, accepted: bool) -> Result<Vec<u8>, String> {
        self.media_thumbnail(id, accepted, 768)
    }
    pub fn media_thumbnail(&self, id: &str, accepted: bool, size: u32) -> Result<Vec<u8>, String> {
        if !(32..=768).contains(&size) {
            return Err("Image preview size must be 32–768 pixels.".into());
        }
        let file = if accepted {
            self.media_asset(id)?.image
        } else {
            self.media_draft(id)?
                .image
                .ok_or("No image is available yet.")?
        };
        let bytes = self.read_image_bytes(&file)?;
        let (image, _) = decode_image(&bytes)?;
        let mut output = Cursor::new(vec![]);
        image
            .thumbnail(size, size)
            .write_to(&mut output, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        Ok(output.into_inner())
    }
    pub fn accept_media(&self, id: &str, version: u32) -> Result<CommitResult, String> {
        let mut d = self.media_draft(id)?;
        let mut index = self.story_index()?;
        let current = self.view()?;
        if index.media.contains_key(id) {
            return Ok(CommitResult {
                committed_revision: self
                    .committed_request_revision(id)?
                    .unwrap_or(current.revision),
                project: current,
                replayed: true,
            });
        }
        if d.version != version || d.stage != MediaStage::Ready {
            return Err("Review the finished image before accepting.".into());
        }
        let image = d.image.clone().ok_or("Missing image file.")?;
        self.read_image_bytes(&image)?;
        let asset = MediaAsset {
            schema_version: 1,
            id: d.id.clone(),
            target: d.target.clone(),
            source: d.source.clone(),
            request: d.request.clone(),
            submission: d.submission.clone(),
            image,
            imported: d.imported,
            prompt_origin: d.prompt_origin.clone(),
        };
        index
            .media
            .insert(id.into(), self.write_record("media", &asset)?);
        if let Some(node) = &d.target.node_id {
            index.portraits.insert(node.clone(), id.into());
        }
        let result = self.commit_story(
            current.revision,
            id.into(),
            current.title,
            current.world,
            index,
        )?;
        d.stage = MediaStage::Accepted;
        d.version += 1;
        self.write_media(&d)?;
        Ok(result)
    }
    pub fn clear_portrait(
        &self,
        node_id: &str,
        request_id: String,
    ) -> Result<CommitResult, String> {
        validate_id(node_id)?;
        let current = self.view()?;
        let mut index = self.story_index()?;
        if index.portraits.remove(node_id).is_none() {
            return Ok(CommitResult {
                committed_revision: current.revision,
                project: current,
                replayed: true,
            });
        }
        self.commit_story(
            current.revision,
            request_id,
            current.title,
            current.world,
            index,
        )
    }
    pub fn select_portrait(
        &self,
        node_id: &str,
        asset_id: &str,
        request_id: String,
    ) -> Result<CommitResult, String> {
        let asset = self.media_asset(asset_id)?;
        if asset.target.node_id.as_deref() != Some(node_id) {
            return Err("Choose an image for this world node.".into());
        }
        let current = self.view()?;
        let mut index = self.story_index()?;
        index.portraits.insert(node_id.into(), asset_id.into());
        self.commit_story(
            current.revision,
            request_id,
            current.title,
            current.world,
            index,
        )
    }
    pub fn selected_portrait(
        &self,
        node_id: &str,
        world: &World,
    ) -> Result<Option<MediaEntry>, String> {
        let index = self.story_index()?;
        let Some(id) = index.portraits.get(node_id) else {
            return Ok(None);
        };
        match self.media_asset(id) {
            Ok(asset) => {
                let applicable =
                    shot_fact(world, node_id).is_ok_and(|f| asset.source.facts.first() == Some(&f));
                Ok(Some(MediaEntry {
                    id: id.clone(),
                    label: "Selected image".into(),
                    stage: MediaStage::Accepted,
                    applicable,
                    error: String::new(),
                }))
            }
            Err(error) => Ok(Some(MediaEntry {
                id: id.clone(),
                label: "Unavailable image".into(),
                stage: MediaStage::Accepted,
                applicable: false,
                error,
            })),
        }
    }
    pub fn node_images(&self, world: &World) -> Result<BTreeMap<String, MediaEntry>, String> {
        let index = self.story_index()?;
        let mut result = BTreeMap::new();
        for node in index.portraits.keys() {
            if let Some(entry) = self.selected_portrait(node, world)? {
                result.insert(node.clone(), entry);
            }
        }
        Ok(result)
    }
    pub fn list_media(&self, target: Option<&MediaTarget>) -> Result<Vec<MediaEntry>, String> {
        let matches = |t: &MediaTarget| {
            target.is_none_or(|filter| {
                t.node_id == filter.node_id && t.passage_id == filter.passage_id
            })
        };
        let index = self.story_index()?;
        let mut result = vec![];
        for id in index.media.keys() {
            match self.media_asset(id) {
                Ok(asset) if matches(&asset.target) => result.push(MediaEntry {
                    id: id.clone(),
                    label: if asset.imported {
                        "Imported image"
                    } else {
                        "Generated image"
                    }
                    .into(),
                    stage: MediaStage::Accepted,
                    applicable: self.media_applicable(&asset.target, &asset.source),
                    error: String::new(),
                }),
                Err(error) => result.push(MediaEntry {
                    id: id.clone(),
                    label: "Unavailable image".into(),
                    stage: MediaStage::Accepted,
                    applicable: false,
                    error,
                }),
                _ => (),
            }
        }
        let dir = self.root_path().join("media-drafts");
        if dir.exists() {
            for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
                let path = entry.map_err(|e| e.to_string())?.path();
                if path.extension().is_some_and(|e| e == "json") {
                    let id = path
                        .file_stem()
                        .and_then(|v| v.to_str())
                        .ok_or("Invalid image candidate filename.")?;
                    if index.media.contains_key(id) {
                        continue;
                    }
                    let d = match self.media_draft(id) {
                        Ok(d) => d,
                        Err(error) => {
                            result.push(MediaEntry {
                                id: id.into(),
                                label: "Unavailable candidate".into(),
                                stage: MediaStage::Failed,
                                applicable: false,
                                error,
                            });
                            continue;
                        }
                    };
                    if matches(&d.target) {
                        result.push(MediaEntry {
                            id: d.id,
                            label: "Image candidate".into(),
                            stage: d.stage,
                            applicable: self.media_applicable(&d.target, &d.source),
                            error: d.error,
                        });
                    }
                }
            }
        }
        result.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(result)
    }
    pub fn interrupt_media(&self) -> Result<(), String> {
        let dir = self.root_path().join("media-drafts");
        if !dir.exists() {
            return Ok(());
        }
        let mut errors = vec![];
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let result = (|| {
                let path = entry.map_err(|e| e.to_string())?.path();
                if path.extension().is_none_or(|e| e != "json") {
                    return Ok(());
                }
                let mut d = self.media_draft(
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .ok_or("Invalid image candidate filename.")?,
                )?;
                if d.stage == MediaStage::Submitting {
                    d.stage = MediaStage::Unknown;
                    d.error="Submission outcome is uncertain. Recover its ComfyUI prompt ID; no request was resent.".into();
                    d.version += 1;
                    self.write_media(&d)?;
                }
                Ok::<(), String>(())
            })();
            if let Err(e) = result {
                if errors.len() < 5 {
                    errors.push(e);
                }
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join(" "))
        }
    }
    pub(crate) fn copy_media_for_fork(
        &self,
        fork: &Project,
        source: &StoryIndex,
        kept: &mut StoryIndex,
    ) -> Result<(), String> {
        let mut worlds = BTreeMap::new();
        let mut passages = BTreeMap::new();
        for entry in &kept.passages {
            let record: AcceptedPassage = fork.read_record("passages", &entry.record)?;
            worlds.insert(record.before_world.sha256.clone(), record.before_world);
            worlds.insert(record.after_world.sha256.clone(), record.after_world);
            let original = source
                .passages
                .iter()
                .find(|p| p.id == entry.id)
                .ok_or("Missing fork source passage.")?;
            passages.insert(
                entry.id.clone(),
                (original.record.clone(), entry.record.clone()),
            );
        }
        for id in source.media.keys() {
            let mut asset = self.media_asset(id)?;
            let Some(world) = worlds.get(&asset.source.world.sha256) else {
                continue;
            };
            if let Some(passage_id) = &asset.target.passage_id {
                let Some((original, replacement)) = passages.get(passage_id) else {
                    continue;
                };
                if asset.source.passage.as_ref() != Some(original) {
                    continue;
                }
                asset.source.passage = Some(replacement.clone());
            }
            if let Some(context) = &mut asset.source.image_context {
                let Some(reference) = worlds.get(&context.world.sha256) else {
                    continue;
                };
                context.world = reference.clone();
            }
            asset.source.world = world.clone();
            let bytes = self.read_image_bytes(&asset.image)?;
            asset.image = fork.save_image_bytes(&bytes)?;
            kept.media
                .insert(id.clone(), fork.write_record("media", &asset)?);
        }
        let boundary = fork.view()?.world;
        for (node, id) in &source.portraits {
            if kept.media.contains_key(id) {
                let asset: MediaAsset = fork.read_record("media", &kept.media[id])?;
                if shot_fact(&boundary, node).is_ok_and(|f| asset.source.facts.first() == Some(&f))
                {
                    kept.portraits.insert(node.clone(), id.clone());
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
