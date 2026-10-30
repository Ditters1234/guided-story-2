use super::*;
use fs2::FileExt;
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageConnection {
    pub schema_version: u32,
    pub version: u32,
    pub base_url: String,
    pub timeout_seconds: u32,
    pub profile: ImageWorkflowProfile,
}
impl Default for ImageConnection {
    fn default() -> Self {
        Self {
            schema_version: 1,
            version: 0,
            base_url: "http://127.0.0.1:8188".into(),
            timeout_seconds: 60,
            profile: guided_story_image_workflow(true).expect("Invalid bundled image workflow"),
        }
    }
}
impl ImageConnection {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || !(5..=300).contains(&self.timeout_seconds)
            || self.base_url.len() > 2000
        {
            return Err("Check the image connection version and timeout (5–300 seconds).".into());
        }
        image_endpoint(&self.base_url, "")?;
        self.profile.prepare(&ImageWorkflowRequest {
            prompt: "Workflow validation".into(),
            ..Default::default()
        })?;
        if self.profile.id.len() > 100
            || self.profile.name.len() > 200
            || self.profile.family.len() > 100
        {
            return Err("Workflow profile labels are too long.".into());
        }
        Ok(())
    }
}
pub(crate) fn image_endpoint(base: &str, route: &str) -> Result<url::Url, String> {
    let mut url = url::Url::parse(base).map_err(|_| "Enter a complete HTTP(S) ComfyUI URL.")?;
    if !["http", "https"].contains(&url.scheme())
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("Use an HTTP(S) ComfyUI URL without credentials, query or fragment.".into());
    }
    url.set_path(&format!("{}/{}", url.path().trim_end_matches('/'), route));
    Ok(url)
}
fn read(root: &Path) -> Result<ImageConnection, String> {
    let path = root.join("image-connection.json");
    if !path.exists() {
        return Ok(ImageConnection::default());
    }
    let bytes = crate::store::read_bytes(&path)?;
    if bytes.len() > 1_100_000 {
        return Err("Image connection settings exceed their size limit.".into());
    }
    let settings: ImageConnection = serde_json::from_slice(&bytes)
        .map_err(|_| "Image connection settings are invalid; files were kept.")?;
    settings.validate()?;
    Ok(settings)
}
pub fn load_image_connection(root: &Path) -> Result<ImageConnection, String> {
    read(root)
}
pub fn save_image_connection(
    root: &Path,
    expected: u32,
    mut settings: ImageConnection,
) -> Result<ImageConnection, String> {
    settings.validate()?;
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join("image-connection.lock"))
        .map_err(|e| e.to_string())?;
    FileExt::try_lock_exclusive(&lock)
        .map_err(|_| "Image settings are being saved in another window.")?;
    let current = read(root)?;
    let mut comparable = settings.clone();
    comparable.version = current.version;
    if comparable == current {
        return Ok(current);
    }
    if current.version != expected {
        return Err("Image settings changed. Reload before saving.".into());
    }
    settings.version = current.version + 1;
    crate::store::atomic_write(
        &root.join("image-connection.json"),
        &serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?,
    )?;
    Ok(settings)
}
