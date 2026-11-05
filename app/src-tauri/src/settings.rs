//! User settings, saved as JSON in the app's config folder
//! (`%APPDATA%\io.github.the-jonas.subquest\settings.json`). Never in the repository.

use std::collections::BTreeMap;
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
    pub profiles: BTreeMap<String, GameProfile>, // key: lowercase executable name
}

/// What Subquest remembers about each game, so it starts ready next time.
/// A game can have several areas (dialogue, song lyrics, names...), each with
/// its own look, because games use different fonts and colors for each.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct GameProfile {
    pub title: String,        // window title when it was saved, for display
    pub areas: Vec<GameArea>, // the UI uses the first one for now

    // Fields from the first profile format (one area, no look). Read only to
    // migrate old settings.json files; never written back.
    #[serde(skip_serializing)]
    area_mode: Option<String>,
    #[serde(skip_serializing)]
    region: Option<[i32; 4]>,
}

/// One area of the game screen that Subquest reads and covers.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct GameArea {
    pub name: String,             // shown in the UI, e.g. "Dialogue"
    pub mode: String,             // "dialogue" (marked rectangle) | "screen" (whole window)
    pub region: Option<[i32; 4]>, // [x, y, width, height] in capture pixels
    pub font: String, // overlay font: a family name or "imported:<file>"; empty = default
    pub style: String, // "inPlace" (each line redrawn where it is, in the game's colors) | "classic" (subtitle box)
    pub caps: String,  // letter case the game uses: "" (as translated) | "upper" | "smallCaps"
}

impl Default for GameArea {
    /// A dialogue area drawn in place, like part of the game: the most common case.
    fn default() -> Self {
        Self {
            name: String::new(),
            mode: "dialogue".into(),
            region: None,
            font: String::new(),
            style: "inPlace".into(),
            caps: String::new(),
        }
    }
}

impl GameProfile {
    /// Upgrades older formats: the first one (area_mode + region at the top)
    /// becomes one area, and the "game" style became "inPlace".
    fn migrate(&mut self) {
        for area in &mut self.areas {
            if area.style == "game" {
                area.style = "inPlace".into();
            }
        }
        if self.areas.is_empty() && (self.area_mode.is_some() || self.region.is_some()) {
            self.areas.push(GameArea {
                mode: self.area_mode.take().unwrap_or_else(|| "dialogue".into()),
                region: self.region.take(),
                ..GameArea::default()
            });
        }
    }
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
            profiles: BTreeMap::new(),
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
/// Old profile formats are upgraded on the way in.
pub fn load(config_dir: &Path) -> Settings {
    let mut settings: Settings = std::fs::read_to_string(settings_path(config_dir))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    settings
        .profiles
        .values_mut()
        .for_each(GameProfile::migrate);
    settings
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
        assert!(s.profiles.is_empty());
    }

    #[test]
    fn profiles_round_trip_through_json() {
        let mut s = Settings::default();
        s.profiles.insert(
            "hades2.exe".into(),
            GameProfile {
                title: "Hades II".into(),
                areas: vec![GameArea {
                    name: "Dialogue".into(),
                    region: Some([120, 840, 1680, 200]),
                    font: "Cinzel Variable".into(),
                    ..GameArea::default()
                }],
                ..GameProfile::default()
            },
        );
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"style\":\"inPlace\""));
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn first_format_profiles_become_one_area() {
        let json = r#"{ "profiles": { "hades2.exe": {
            "title": "Hades II", "areaMode": "dialogue", "region": [1, 2, 3, 4] } } }"#;
        let mut s: Settings = serde_json::from_str(json).unwrap();
        s.profiles.values_mut().for_each(GameProfile::migrate);
        let p = &s.profiles["hades2.exe"];
        assert_eq!(p.areas.len(), 1);
        assert_eq!(p.areas[0].region, Some([1, 2, 3, 4]));
        assert_eq!(p.areas[0].style, "inPlace");
        assert!(!serde_json::to_string(p).unwrap().contains("areaMode"));
    }
}
