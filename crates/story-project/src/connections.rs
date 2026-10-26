use crate::store::atomic_write;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::{
    fs::{self, File, OpenOptions},
    io::Read,
    path::Path,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TextProvider {
    Compatible,
    Demo,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum StructuredOutput {
    Json,
    Schema,
    Prompt,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TokenParameter {
    MaxTokens,
    MaxCompletionTokens,
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ThinkingControl {
    #[default]
    Provider,
    ChatTemplateOff,
    DirectOff,
}
fn default_top_p() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LlmConnection {
    pub schema_version: u32,
    pub version: u32,
    pub provider: TextProvider,
    pub base_url: String,
    pub model: String,
    pub temperature: f64,
    #[serde(default = "default_top_p")]
    pub top_p: f64,
    #[serde(default)]
    pub top_k: Option<u32>,
    #[serde(default)]
    pub min_p: Option<f64>,
    #[serde(default)]
    pub thinking: ThinkingControl,
    pub max_tokens: u32,
    pub context_tokens: u32,
    pub timeout_seconds: u32,
    pub structured_output: StructuredOutput,
    pub token_parameter: TokenParameter,
    pub credential_ref: Option<String>,
}
impl Default for LlmConnection {
    fn default() -> Self {
        Self {
            schema_version: 1,
            version: 0,
            provider: TextProvider::Compatible,
            base_url: "http://127.0.0.1:8888/v1".into(),
            model: String::new(),
            temperature: 0.7,
            top_p: 0.8,
            top_k: Some(20),
            min_p: Some(0.0),
            thinking: ThinkingControl::ChatTemplateOff,
            max_tokens: 4096,
            context_tokens: 32000,
            timeout_seconds: 300,
            structured_output: StructuredOutput::Json,
            token_parameter: TokenParameter::MaxTokens,
            credential_ref: None,
        }
    }
}
impl LlmConnection {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("Unsupported LLM connection settings.".into());
        }
        let url = url::Url::parse(&self.base_url)
            .map_err(|_| "Enter a complete HTTP(S) API base URL.")?;
        if !["http", "https"].contains(&url.scheme())
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || self.base_url.len() > 2000
        {
            return Err(
                "Use an HTTP(S) API URL without embedded credentials, query or fragment.".into(),
            );
        }
        if self.model.len() > 200
            || !self.temperature.is_finite()
            || !(0.0..=2.0).contains(&self.temperature)
            || !self.top_p.is_finite()
            || !(0.01..=1.0).contains(&self.top_p)
            || self.top_k.is_some_and(|v| !(1..=1000).contains(&v))
            || self
                .min_p
                .is_some_and(|v| !v.is_finite() || !(0.0..=1.0).contains(&v))
            || !(64..=32768).contains(&self.max_tokens)
            || !(2048..=262144).contains(&self.context_tokens)
            || self.max_tokens + 1024 >= self.context_tokens
            || !(10..=600).contains(&self.timeout_seconds)
        {
            return Err("Check the model, temperature, token budgets and timeout.".into());
        }
        if let Some(id) = &self.credential_ref {
            crate::validate_id(id)?;
        }
        Ok(())
    }
    pub fn ready(&self) -> Result<(), String> {
        self.validate()?;
        if self.provider == TextProvider::Compatible && self.model.trim().is_empty() {
            return Err(
                "Set a model in Settings → LLM, or select the offline demo provider.".into(),
            );
        }
        Ok(())
    }
    pub fn endpoint(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        if base.ends_with("/chat/completions") {
            base.into()
        } else {
            format!("{base}/chat/completions")
        }
    }
    pub fn models_endpoint(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        let base = base.strip_suffix("/chat/completions").unwrap_or(base);
        format!("{base}/models")
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionTest {
    pub models: Vec<String>,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionView {
    pub settings: LlmConnection,
    pub has_api_key: bool,
}
#[derive(Clone, Serialize, Deserialize, Type)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum ApiKeyUpdate {
    Keep,
    Replace(String),
    Clear,
}

fn lock(root: &Path) -> Result<File, String> {
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("connections.lock"))
        .map_err(|e| e.to_string())?;
    file.try_lock_exclusive()
        .map_err(|_| "Connection settings are busy. Retry shortly.")?;
    Ok(file)
}
fn read_connection(root: &Path) -> Result<LlmConnection, String> {
    let file = match File::open(root.join("llm-connection.json")) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(LlmConnection::default()),
        Err(e) => return Err(e.to_string()),
    };
    let mut bytes = vec![];
    file.take(16385)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 16384 {
        return Err("LLM connection settings are too large.".into());
    }
    let settings: LlmConnection = serde_json::from_slice(&bytes).map_err(|_| {
        "Cannot read LLM connection settings. The existing file has been preserved."
    })?;
    settings.validate()?;
    Ok(settings)
}
pub fn load_connection(root: &Path) -> Result<ConnectionView, String> {
    let _lock = lock(root)?;
    let settings = read_connection(root)?;
    let has_api_key = settings.credential_ref.is_some();
    Ok(ConnectionView {
        settings,
        has_api_key,
    })
}
pub fn save_connection(
    root: &Path,
    expected: u32,
    mut settings: LlmConnection,
    key: ApiKeyUpdate,
) -> Result<ConnectionView, String> {
    settings.validate()?;
    let _lock = lock(root)?;
    let current = read_connection(root)?;
    if current.version != expected {
        return Err("Connection settings changed. Reload before saving.".into());
    }
    let previous_credential = current.credential_ref.clone();
    if matches!(&key, ApiKeyUpdate::Keep) {
        let mut unchanged = settings.clone();
        unchanged.version = current.version;
        unchanged.credential_ref = current.credential_ref.clone();
        if unchanged == current {
            return Ok(ConnectionView {
                has_api_key: current.credential_ref.is_some(),
                settings: current,
            });
        }
    }
    settings.credential_ref = match key {
        ApiKeyUpdate::Keep => current.credential_ref,
        ApiKeyUpdate::Clear => None,
        ApiKeyUpdate::Replace(value) => {
            if value.is_empty() || value.len() > 8192 || value.contains(['\r', '\n']) {
                return Err("Enter a nonempty API key without line breaks.".into());
            }
            let id = uuid::Uuid::new_v4().to_string();
            let sealed = protect(value.as_bytes(), false)?;
            fs::create_dir_all(root.join("credentials")).map_err(|e| e.to_string())?;
            atomic_write(
                &root.join("credentials").join(format!("{id}.dpapi")),
                &sealed,
            )?;
            Some(id)
        }
    };
    settings.version = expected
        .checked_add(1)
        .ok_or("Connection version limit reached.")?;
    atomic_write(
        &root.join("llm-connection.json"),
        &serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?,
    )?;
    if previous_credential != settings.credential_ref {
        if let Some(id) = previous_credential {
            let _ = fs::remove_file(root.join("credentials").join(format!("{id}.dpapi")));
        }
    }
    Ok(ConnectionView {
        has_api_key: settings.credential_ref.is_some(),
        settings,
    })
}
pub fn connection_api_key(root: &Path, settings: &LlmConnection) -> Result<Option<String>, String> {
    let Some(id) = &settings.credential_ref else {
        return Ok(None);
    };
    crate::validate_id(id)?;
    let file = File::open(root.join("credentials").join(format!("{id}.dpapi")))
        .map_err(|_| "The saved API key is unavailable. Re-enter it in Settings → LLM.")?;
    let mut sealed = vec![];
    file.take(32769)
        .read_to_end(&mut sealed)
        .map_err(|_| "Could not read the saved API key.")?;
    if sealed.len() > 32768 {
        return Err("The saved credential is invalid.".into());
    }
    String::from_utf8(protect(&sealed, true)?)
        .map(Some)
        .map_err(|_| "The saved credential is invalid.".into())
}
#[cfg(windows)]
fn protect(bytes: &[u8], decrypt: bool) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };
    let input = CRYPT_INTEGER_BLOB {
        cbData: bytes.len() as u32,
        pbData: bytes.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // DPAPI scopes secrets to this Windows user. The output is copied before LocalFree.
    let ok = unsafe {
        if decrypt {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptProtectData(
                &input,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
    };
    if ok == 0 {
        return Err("Windows could not protect/unprotect the API key for this user.".into());
    }
    let result =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        LocalFree(output.pbData as *mut _);
    };
    Ok(result)
}
#[cfg(not(windows))]
fn protect(_bytes: &[u8], _decrypt: bool) -> Result<Vec<u8>, String> {
    Err("API-key storage is currently supported on Windows only.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn qwen_defaults_and_legacy_sampling_are_explicit_and_preserve_saved_choices() {
        let defaults = LlmConnection::default();
        assert_eq!(defaults.top_k, Some(20));
        assert_eq!(defaults.thinking, ThinkingControl::ChatTemplateOff);
        let mut legacy = serde_json::to_value(&defaults).unwrap();
        for field in ["topP", "topK", "minP", "thinking"] {
            legacy.as_object_mut().unwrap().remove(field);
        }
        legacy["temperature"] = serde_json::json!(0.9);
        let saved: LlmConnection = serde_json::from_value(legacy).unwrap();
        assert_eq!(saved.temperature, 0.9);
        assert_eq!(saved.top_p, 1.0);
        assert_eq!(saved.top_k, None);
        assert_eq!(saved.min_p, None);
        assert_eq!(saved.thinking, ThinkingControl::Provider);
        saved.validate().unwrap();
        let mut invalid = defaults;
        invalid.top_k = Some(0);
        assert!(invalid.validate().is_err());
        invalid.top_k = None;
        invalid.min_p = Some(f64::NAN);
        assert!(invalid.validate().is_err());
    }
    #[test]
    fn connection_settings_stay_separate_and_reject_stale_or_unsafe_urls() {
        let dir = tempfile::tempdir().unwrap();
        let mut settings = LlmConnection::default();
        settings.provider = TextProvider::Demo;
        let saved = save_connection(dir.path(), 0, settings, ApiKeyUpdate::Keep).unwrap();
        assert_eq!(saved.settings.version, 1);
        assert!(
            save_connection(dir.path(), 0, saved.settings.clone(), ApiKeyUpdate::Clear).is_err()
        );
        let mut bad = saved.settings;
        bad.base_url = "http://name:secret@example.com/v1".into();
        assert!(bad.validate().is_err());
        assert!(!dir.path().join("head.json").exists());
    }
    #[cfg(windows)]
    #[test]
    fn api_keys_are_encrypted_and_not_returned_in_views() {
        let dir = tempfile::tempdir().unwrap();
        let saved = save_connection(
            dir.path(),
            0,
            LlmConnection::default(),
            ApiKeyUpdate::Replace("fictional-fixture-key".into()),
        )
        .unwrap();
        assert!(saved.has_api_key);
        assert!(!serde_json::to_string(&saved)
            .unwrap()
            .contains("fictional-fixture-key"));
        assert!(!String::from_utf8_lossy(
            &fs::read(dir.path().join("llm-connection.json")).unwrap()
        )
        .contains("fictional-fixture-key"));
        assert_eq!(
            connection_api_key(dir.path(), &saved.settings)
                .unwrap()
                .unwrap(),
            "fictional-fixture-key"
        );
        assert_eq!(
            save_connection(dir.path(), 1, saved.settings, ApiKeyUpdate::Clear)
                .unwrap()
                .has_api_key,
            false
        );
    }
}
