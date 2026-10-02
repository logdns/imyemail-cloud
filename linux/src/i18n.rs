//! Local UI copy and appearance preferences. Mail content is never translated.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, sync::OnceLock};

pub const CODES: [&str; 6] = ["en", "zh-TW", "zh-CN", "ja", "fr", "es"];
pub const NAMES: [&str; 6] = ["English", "繁體中文", "简体中文", "日本語", "Français", "Español"];
#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Preferences {
    pub language: String,
    pub theme: String,
    pub allow_remote_images: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self { language: "en".into(), theme: "system".into(), allow_remote_images: false }
    }
}
fn path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
        })
        .join("email.imy.cloud/appearance.json")
}
pub fn load() -> Preferences {
    let mut value: Preferences = std::fs::read(path())
        .ok()
        .and_then(|v| serde_json::from_slice(&v).ok())
        .unwrap_or_default();
    if !CODES.contains(&value.language.as_str()) {
        value.language = "en".into();
    }
    if !["system", "light", "dark"].contains(&value.theme.as_str()) {
        value.theme = "system".into();
    }
    value
}
pub fn save(value: &Preferences) -> Result<(), String> {
    let destination = path();
    std::fs::create_dir_all(destination.parent().unwrap()).map_err(|e| e.to_string())?;
    let temporary = destination.with_extension("tmp");
    std::fs::write(&temporary, serde_json::to_vec(value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    std::fs::rename(temporary, destination).map_err(|e| e.to_string())
}
pub fn language() -> &'static str {
    static LANGUAGE: OnceLock<String> = OnceLock::new();
    LANGUAGE.get_or_init(|| load().language)
}
pub fn t(text: &str) -> &str {
    translate(text, language())
}
pub fn translate<'a>(text: &'a str, language: &str) -> &'a str {
    static TABLES: OnceLock<BTreeMap<String, BTreeMap<String, String>>> = OnceLock::new();
    let tables = TABLES.get_or_init(|| {
        serde_json::from_str(include_str!("../data/locales.json"))
            .expect("validated bundled localization")
    });
    tables.get(language).unwrap_or(&tables["en"]).get(text).map_or(text, String::as_str)
}

/// Substitute runtime values only after translating the UI template.
pub fn format(key: &str, values: &[String]) -> String {
    let mut rest = t(key);
    let mut result = String::new();
    while let Some(start) = rest.find("XPH") {
        result.push_str(&rest[..start]);
        let token = &rest[start + 3..];
        if let Some(end) = token.find('X')
            && let Some(value) =
                token[..end].parse::<usize>().ok().and_then(|index| values.get(index))
        {
            result.push_str(value);
            rest = &token[end + 1..];
            continue;
        }
        result.push_str("XPH");
        rest = token;
    }
    result.push_str(rest);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn offline_catalogs_and_fallback() {
        assert_eq!(Preferences::default().language, "en");
        assert!(!Preferences::default().allow_remote_images);
        for (code, expected) in
            CODES.iter().zip(["Settings", "設定", "设置", "設定", "Paramètres", "Configuración"])
        {
            assert_eq!(translate("设置", code), expected);
        }
        assert_eq!(translate("设置", "unknown"), "Settings");
        assert_eq!(translate("A customer subject 你好", "en"), "A customer subject 你好");
    }
}
