//! User settings, saved as JSON in the app's config folder
//! (`%APPDATA%\Subquest\settings.json`). Never in the repository.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::versioned::{self, Migration};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub ui_language: String,     // UI language: "pt-BR", "en", "es"
    pub ui_theme: String,        // color theme of Subquest's windows: "graphite", "night", "moss", "plum"
    pub source_language: String, // game language
    pub target_language: String, // translation language
    pub ocr_engine: String,      // "windows" | "paddle"
    pub translator: TranslatorSettings,
    pub quick: QuickSettings, // second translation layer: fills the gap while the LLM answers
    pub shortcuts: ShortcutSettings, // keys that work inside the game during a session
    pub overlay: OverlaySettings,
    pub tutorial_done: bool, // false => opens the tutorial on first run

    // Game profiles lived here before they got their own folders (profiles.rs).
    // Read only to move them there; never written back.
    #[serde(rename = "profiles", skip_serializing)]
    pub old_profiles: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct TranslatorSettings {
    pub kind: String, // "openai-compatible" | "deepl"
    pub base_url: String,
    pub model: String,
    pub vision_model: String, // model that looks at images (font detection); empty = `model`
    // TODO(phase 3): move the key to Windows Credential Manager.
    pub api_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct OverlaySettings {
    pub font_size: u32,          // classic box font size in px; 0 = automatic
    pub background_opacity: f32, // 0..1
    pub show_original: bool,     // show the original text above the translation
    pub look: String,            // "inPlace" (drawn over each original line) or "classic" (subtitle box)
}

/// Global shortcuts, written like "Ctrl+Shift+A" (empty = no shortcut).
/// Registered only while a game is being translated, so they don't steal
/// these keys from other programs the rest of the time.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ShortcutSettings {
    pub area: String,    // mark a new area
    pub style: String,   // take a style print
    pub pause: String,   // pause / resume translating
    pub hide: String,    // hide / show the translations
    pub toolbar: String, // show / hide the in-game toolbar
    pub edit: String,    // edit the marked areas (move, resize, rename, remove)
    pub term: String,    // click a word in the game to add it to the glossary
}

impl Default for ShortcutSettings {
    /// Ctrl+Shift plus the action's initial (B for the bar).
    fn default() -> Self {
        Self {
            area: "Ctrl+Shift+A".into(),
            style: "Ctrl+Shift+S".into(),
            pause: "Ctrl+Shift+T".into(),
            hide: "Ctrl+Shift+H".into(),
            toolbar: "Ctrl+Shift+B".into(),
            edit: "Ctrl+Shift+E".into(),
            term: "Ctrl+Shift+G".into(),
        }
    }
}

/// The quick translation layer: a plain machine translator that answers in a
/// few hundred milliseconds and is shown until the main (LLM) translation arrives.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct QuickSettings {
    pub engine: String,    // "" (off) | "google" | "deepl"
    pub deepl_key: String, // only for "deepl"
}

impl Default for QuickSettings {
    /// On, with Google: free and needs no key, so lines show fast from the first run.
    fn default() -> Self {
        Self {
            engine: "google".into(),
            deepl_key: String::new(),
        }
    }
}

impl Default for Settings {
    /// Defaults for the most common case: English game, translated to PT-BR.
    fn default() -> Self {
        Self {
            ui_language: "pt-BR".into(),
            ui_theme: "graphite".into(),
            source_language: "en".into(),
            target_language: "pt-BR".into(),
            ocr_engine: "windows".into(),
            translator: TranslatorSettings::default(),
            quick: QuickSettings::default(),
            shortcuts: ShortcutSettings::default(),
            overlay: OverlaySettings::default(),
            tutorial_done: false,
            old_profiles: BTreeMap::new(),
        }
    }
}

impl Default for TranslatorSettings {
    /// Starts pointing at Gemini: a free API key from Google AI Studio is the
    /// easiest way for most players to get good translations.
    fn default() -> Self {
        Self {
            kind: "openai-compatible".into(),
            base_url: "https://generativelanguage.googleapis.com/v1beta/openai/".into(),
            model: String::new(),
            vision_model: String::new(),
            api_key: String::new(),
        }
    }
}

impl Default for OverlaySettings {
    /// Large font and a background dark enough to read over any scene.
    fn default() -> Self {
        Self {
            font_size: 0, // 0 = automatic: the size of the game's own letters
            background_opacity: 0.78,
            show_original: false,
            look: "inPlace".into(),
        }
    }
}

/// Path of the settings file inside the app's config folder.
pub fn settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join("settings.json")
}

// Format history of settings.json (see versioned.rs). No changes yet that
// need more than serde's defaults for new fields; the file is versioned so
// later ones can be migrated, and so it is protected from damage and from
// being overwritten by an older Subquest.
const SETTINGS_MIGRATIONS: &[Migration] = &[];

/// Reads the settings; falls back to defaults if the file is missing or
/// damaged (a damaged one is copied aside first).
pub fn load(config_dir: &Path) -> Settings {
    versioned::read(&settings_path(config_dir), SETTINGS_MIGRATIONS).unwrap_or_default()
}

/// Writes the settings, creating the folder if needed.
pub fn save(config_dir: &Path, settings: &Settings) -> Result<(), String> {
    versioned::write(&settings_path(config_dir), settings, SETTINGS_MIGRATIONS)
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
        assert_eq!(s.overlay.font_size, 0); // automatic
        assert_eq!(s.overlay.look, "inPlace");
    }

    #[test]
    fn old_profiles_are_read_but_never_written() {
        let json = r#"{ "profiles": { "hades2.exe": { "title": "Hades II" } } }"#;
        let s: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(s.old_profiles["hades2.exe"]["title"], "Hades II");
        assert!(!serde_json::to_string(&s).unwrap().contains("profiles"));
    }
}
