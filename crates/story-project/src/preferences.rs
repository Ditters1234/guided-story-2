//! App-wide appearance preferences; never part of a story snapshot.
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
pub enum ThemeMode {
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeColors {
    pub background: String,
    pub surface: String,
    pub surface_alt: String,
    pub text: String,
    pub muted: String,
    pub border: String,
    pub accent: String,
    pub accent_text: String,
    pub selection: String,
    pub canvas: String,
    pub canvas_dots: String,
    pub connection: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeDefinition {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub mode: ThemeMode,
    pub colors: ThemeColors,
}
impl ThemeDefinition {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("Unsupported theme format.".into());
        }
        let uuid = self
            .id
            .strip_prefix("custom.")
            .ok_or("A custom theme needs its own ID.")?;
        crate::validate_id(uuid)?;
        if self.name.trim().is_empty() || self.name.chars().count() > 60 {
            return Err("Theme names need 1–60 characters.".into());
        }
        let c = &self.colors;
        for color in [
            &c.background,
            &c.surface,
            &c.surface_alt,
            &c.text,
            &c.muted,
            &c.border,
            &c.accent,
            &c.accent_text,
            &c.selection,
            &c.canvas,
            &c.canvas_dots,
            &c.connection,
        ] {
            if color.len() != 7
                || !color.starts_with('#')
                || !color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
            {
                return Err("Theme colors must use #RRGGBB hex values.".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preferences {
    pub schema_version: u32,
    pub version: u32,
    pub theme_id: String,
    pub text_scale: u16,
    pub reduced_motion: bool,
    pub custom_themes: Vec<ThemeDefinition>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            version: 0,
            theme_id: "system".into(),
            text_scale: 100,
            reduced_motion: false,
            custom_themes: vec![],
        }
    }
}
impl Preferences {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("Unsupported preferences version.".into());
        }
        if !(85..=140).contains(&self.text_scale) {
            return Err("Text size must be between 85% and 140%.".into());
        }
        if self.custom_themes.len() > 12 {
            return Err("Keep at most 12 custom themes.".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for theme in &self.custom_themes {
            theme.validate()?;
            if !ids.insert(&theme.id) {
                return Err("Duplicate custom theme ID.".into());
            }
        }
        let builtins: Vec<ThemeDefinition> =
            serde_json::from_str(include_str!("../../../schema/themes.json"))
                .expect("Invalid built-in themes");
        if self.theme_id != "system"
            && !builtins.iter().any(|t| t.id == self.theme_id)
            && !self.custom_themes.iter().any(|t| t.id == self.theme_id)
        {
            return Err("The selected theme does not exist.".into());
        }
        Ok(())
    }
}

fn lock(root: &Path) -> Result<File, String> {
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("preferences.lock"))
        .map_err(|e| e.to_string())?;
    lock.try_lock_exclusive()
        .map_err(|_| "Settings are being saved by another app instance. Retry shortly.")?;
    Ok(lock)
}
fn read(root: &Path) -> Result<Preferences, String> {
    let path = root.join("preferences.json");
    let file = match File::open(&path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Preferences::default()),
        Err(e) => return Err(e.to_string()),
    };
    let mut bytes = Vec::new();
    file.take(128 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 128 * 1024 {
        return Err("Preferences file is too large.".into());
    }
    let prefs: Preferences = serde_json::from_slice(&bytes)
        .map_err(|e| format!("Cannot read preferences.json: {e}. The file has been preserved."))?;
    prefs.validate()?;
    Ok(prefs)
}
pub fn load_preferences(root: &Path) -> Result<Preferences, String> {
    let _lock = lock(root)?;
    read(root)
}
pub fn save_preferences(
    root: &Path,
    expected_version: u32,
    mut next: Preferences,
) -> Result<Preferences, String> {
    next.validate()?;
    let _lock = lock(root)?;
    let current = read(root)?;
    next.version = expected_version
        .checked_add(1)
        .ok_or("Preferences version limit reached.")?;
    // Repeating a save after an uncertain response is safe.
    if current == next {
        return Ok(current);
    }
    if current.version != expected_version {
        return Err(
            "Settings changed in another app instance. Reload settings before saving.".into(),
        );
    }
    let bytes = serde_json::to_vec_pretty(&next).map_err(|e| e.to_string())?;
    crate::store::atomic_write(&root.join("preferences.json"), &bytes)?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_roundtrip_outside_projects_with_stale_write_and_replay_protection() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            load_preferences(dir.path()).unwrap(),
            Preferences::default()
        );
        let mut prefs = Preferences::default();
        prefs.theme_id = "dark".into();
        prefs.text_scale = 120;
        let saved = save_preferences(dir.path(), 0, prefs.clone()).unwrap();
        assert_eq!(saved.version, 1);
        assert_eq!(save_preferences(dir.path(), 0, prefs).unwrap(), saved);
        assert!(save_preferences(dir.path(), 0, Preferences::default()).is_err());
        assert_eq!(load_preferences(dir.path()).unwrap(), saved);
        assert!(!dir.path().join("head.json").exists());
    }
    #[test]
    fn corrupt_settings_are_preserved_and_never_replaced_with_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("preferences.json");
        fs::write(&path, b"{broken").unwrap();
        assert!(load_preferences(dir.path()).is_err());
        assert!(save_preferences(dir.path(), 0, Preferences::default()).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"{broken");
        let oversized = vec![b' '; 128 * 1024 + 1];
        fs::write(&path, &oversized).unwrap();
        assert!(load_preferences(dir.path())
            .unwrap_err()
            .contains("too large"));
        assert_eq!(fs::read(&path).unwrap(), oversized);
    }
    #[test]
    fn custom_themes_are_portable_data_and_reject_css_injection_and_invalid_settings() {
        let mut theme: ThemeDefinition = serde_json::from_str::<Vec<ThemeDefinition>>(
            include_str!("../../../schema/themes.json"),
        )
        .unwrap()
        .remove(0);
        theme.id = format!("custom.{}", uuid::Uuid::new_v4());
        let mut prefs = Preferences::default();
        prefs.theme_id = theme.id.clone();
        prefs.custom_themes.push(theme.clone());
        prefs.validate().unwrap();
        let dir = tempfile::tempdir().unwrap();
        save_preferences(dir.path(), 0, prefs.clone()).unwrap();
        assert_eq!(
            load_preferences(dir.path()).unwrap().custom_themes,
            vec![theme]
        );
        prefs.custom_themes[0].colors.surface = "url(https://example.com)".into();
        assert!(prefs.validate().is_err());
        prefs.custom_themes.clear();
        prefs.theme_id = "missing".into();
        assert!(prefs.validate().is_err());
        prefs.theme_id = "light".into();
        prefs.text_scale = 500;
        assert!(prefs.validate().is_err());
        prefs.text_scale = 100;
        prefs.schema_version = 2;
        assert!(prefs.validate().is_err());
    }
}
