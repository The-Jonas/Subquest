//! User settings, saved as JSON in the app's config folder
//! (`%APPDATA%\io.github.the-jonas.subquest\settings.json`). Never in the repository.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub ui_language: String,     // UI language: "pt-BR", "en", "es"
    pub source_language: String, // game language
    pub target_language: String, // translation language
    pub ocr_engine: String,      // "windows" | "paddle"
    pub translator: TranslatorSettings,
    pub overlay: OverlaySettings,
    pub tutorial_done: bool, // false => opens the tutorial on first run
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct TranslatorSettings {
    pub kind: String, // "openai-compatible" | "deepl"
    pub base_url: String,
    pub model: String,
    // TODO(phase 3): move the key to Windows Credential Manager.
    pub api_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct OverlaySettings {
    pub font_size: u32,
    pub background_opacity: f32, // 0..1
    pub show_original: bool,     // show the original text above the translation
}

impl Default for Settings {
    /// Defaults for the most common case: English game, translated to PT-BR.
    fn default() -> Self {
        Self {
            ui_language: "pt-BR".into(),
            source_language: "en".into(),
            target_language: "pt-BR".into(),
            ocr_engine: "windows".into(),
            translator: TranslatorSettings::default(),
            overlay: OverlaySettings::default(),
            tutorial_done: false,
        }
    }
}

impl Default for TranslatorSettings {
    /// Starts pointing at a local Ollama: works with no key at all.
    fn default() -> Self {
        Self {
            kind: "openai-compatible".into(),
            base_url: "http://localhost:11434/v1".into(),
            model: String::new(),
            api_key: String::new(),
        }
    }
}

impl Default for OverlaySettings {
    /// Large font and a background dark enough to read over any scene.
    fn default() -> Self {
        Self {
            font_size: 22,
            background_opacity: 0.78,
            show_original: false,
        }
    }
}

/// Path of the settings file inside the app's config folder.
pub fn settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join("settings.json")
}

/// Reads the settings; falls back to defaults if the file is missing or corrupt.
pub fn load(config_dir: &Path) -> Settings {
    std::fs::read_to_string(settings_path(config_dir))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Writes the settings, creating the folder if needed.
pub fn save(config_dir: &Path, settings: &Settings) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(settings_path(config_dir), text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_use_defaults() {
        // Simulates a settings.json from an older version, without the new sections.
        let s: Settings = serde_json::from_str(r#"{ "uiLanguage": "en" }"#).unwrap();
        assert_eq!(s.ui_language, "en");
        assert_eq!(s.target_language, "pt-BR");
        assert_eq!(s.overlay.font_size, 22);
    }
}
