//! What Subquest remembers about each game, so it starts ready next time.
//!
//! Every game has its own folder, `<app data>/games/<game>/`:
//! - `profile.json`: the screen areas;
//! - `glossary.json`: terms with a fixed translation (see glossary.rs);
//! - `styles.json` and `prints/`: the text styles (see styles.rs).
//!
//! Keeping a game's data together (and apart from settings.json, which holds
//! the API keys) leaves room for what later phases add per game (glossary,
//! rebuilt backgrounds), and lets a game folder be shared as it is.
//! Files are versioned (see versioned.rs) so older ones are always readable.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;
use crate::glossary::GlossaryEntry;
use crate::versioned::{self, Migration};

/// A game's profile: how its screen is read.
/// A game can have several areas (dialogue, song lyrics, names...), each with
/// its own look, because games use different fonts and colors for each.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct GameProfile {
    pub title: String,        // window title when it was saved, for display
    pub areas: Vec<GameArea>, // marked rectangles, or one "screen" area for the whole window
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

/// profile.json on disk: the profile plus the game it belongs to (the folder
/// name is a file-safe version of the key, which can't always be turned back).
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileFile {
    #[serde(default)]
    key: String,
    #[serde(flatten)]
    profile: GameProfile,
}

// --- Format history of profile.json ---
// v0: profiles inside settings.json. The very first ones had one area as
//     "areaMode" + "region" at the top, and called the in-place look "game".
// v1: one file per game, with "key" and "version".

/// v0 -> v1: one-area profiles become an areas list; "game" style -> "inPlace".
fn v0_areas_list(v: &mut Value) {
    let Some(map) = v.as_object_mut() else { return };
    let mode = map.remove("areaMode");
    let region = map.remove("region");
    let has_areas = map.get("areas").and_then(Value::as_array).is_some_and(|a| !a.is_empty());
    if !has_areas && (mode.is_some() || region.is_some()) {
        let mode = mode.filter(Value::is_string).unwrap_or_else(|| "dialogue".into());
        map.insert("areas".into(), serde_json::json!([{ "mode": mode, "region": region }]));
    }
    if let Some(areas) = map.get_mut("areas").and_then(Value::as_array_mut) {
        for area in areas {
            if area.get("style").and_then(Value::as_str) == Some("game") {
                area["style"] = "inPlace".into();
            }
        }
    }
}

pub const PROFILE_MIGRATIONS: &[Migration] = &[v0_areas_list];

/// glossary.json on disk.
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct GlossaryFile {
    entries: Vec<GlossaryEntry>,
}

// --- Format history of glossary.json ---
// v0: the first one ("entries": source, target, keep). No migrations yet.
const GLOSSARY_MIGRATIONS: &[Migration] = &[];

/// File-safe version of a game key ("hades2.exe" stays, odd characters become "_").
pub fn file_key(key: &str) -> String {
    key.chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' })
        .collect()
}

/// Folder holding every game's folder.
pub fn games_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("games")
}

/// Folder of one game.
pub fn game_dir(data_dir: &Path, key: &str) -> PathBuf {
    games_dir(data_dir).join(file_key(key))
}

/// All the profiles, kept in memory (there are few and they are small) and
/// written to disk on every change.
pub struct ProfileStore {
    data_dir: PathBuf,
    profiles: Mutex<BTreeMap<String, GameProfile>>,
    glossaries: Mutex<BTreeMap<String, Vec<GlossaryEntry>>>, // read on first use: every line needs it
}

impl ProfileStore {
    /// Reads every game folder that has a profile.
    pub fn load(data_dir: &Path) -> Self {
        let mut profiles = BTreeMap::new();
        for entry in std::fs::read_dir(games_dir(data_dir)).into_iter().flatten().flatten() {
            let path = entry.path().join("profile.json");
            if let Some(file) = versioned::read::<ProfileFile>(&path, PROFILE_MIGRATIONS) {
                let key = if file.key.is_empty() { entry.file_name().to_string_lossy().into_owned() } else { file.key };
                profiles.insert(key, file.profile);
            }
        }
        Self {
            data_dir: data_dir.to_path_buf(),
            profiles: Mutex::new(profiles),
            glossaries: Mutex::new(BTreeMap::new()),
        }
    }

    fn write(&self, key: &str, profile: &GameProfile) -> Result<(), AppError> {
        let file = ProfileFile { key: key.to_string(), profile: profile.clone() };
        let path = game_dir(&self.data_dir, key).join("profile.json");
        versioned::write(&path, &file, PROFILE_MIGRATIONS).map_err(AppError::other)
    }

    /// Every profile, by game key.
    pub fn all(&self) -> BTreeMap<String, GameProfile> {
        self.profiles.lock().unwrap().clone()
    }

    /// One game's profile, if it has one.
    pub fn get(&self, key: &str) -> Option<GameProfile> {
        self.profiles.lock().unwrap().get(key).cloned()
    }

    /// Replaces a game's profile and saves it.
    pub fn save(&self, key: &str, profile: GameProfile) -> Result<(), AppError> {
        self.write(key, &profile)?;
        self.profiles.lock().unwrap().insert(key.to_string(), profile);
        Ok(())
    }

    /// Changes a game's profile (a new one if it has none) and saves it.
    pub fn update(&self, key: &str, change: impl FnOnce(&mut GameProfile)) -> Result<(), AppError> {
        let mut profiles = self.profiles.lock().unwrap();
        let mut profile = profiles.get(key).cloned().unwrap_or_default();
        change(&mut profile);
        self.write(key, &profile)?;
        profiles.insert(key.to_string(), profile);
        Ok(())
    }

    /// A game's glossary (empty when it has none).
    pub fn glossary(&self, key: &str) -> Vec<GlossaryEntry> {
        let mut glossaries = self.glossaries.lock().unwrap();
        glossaries
            .entry(key.to_string())
            .or_insert_with(|| {
                let path = game_dir(&self.data_dir, key).join("glossary.json");
                versioned::read::<GlossaryFile>(&path, GLOSSARY_MIGRATIONS).unwrap_or_default().entries
            })
            .clone()
    }

    /// Replaces a game's glossary and saves it.
    pub fn save_glossary(&self, key: &str, entries: Vec<GlossaryEntry>) -> Result<(), AppError> {
        let path = game_dir(&self.data_dir, key).join("glossary.json");
        let file = GlossaryFile { entries };
        versioned::write(&path, &file, GLOSSARY_MIGRATIONS).map_err(AppError::other)?;
        self.glossaries.lock().unwrap().insert(key.to_string(), file.entries);
        Ok(())
    }

    /// Forgets a game: its whole folder (profile, glossary, styles and prints) goes.
    pub fn forget(&self, key: &str) {
        self.profiles.lock().unwrap().remove(key);
        self.glossaries.lock().unwrap().remove(key);
        let _ = std::fs::remove_dir_all(game_dir(&self.data_dir, key));
    }
}

/// Moves data from before game folders into them, once:
/// - profiles that lived inside settings.json (`old_profiles`);
/// - `styles/<game>.json` and the prints in `styles/<game>/`.
///
/// Nothing already in a game folder is overwritten. Returns true when every
/// old profile was written, so settings.json can be saved without them.
pub fn move_old_data(data_dir: &Path, old_profiles: &BTreeMap<String, Value>) -> bool {
    let mut all_written = true;
    for (key, value) in old_profiles {
        let path = game_dir(data_dir, key).join("profile.json");
        if path.exists() {
            continue;
        }
        let upgraded = versioned::upgrade(value.clone(), PROFILE_MIGRATIONS);
        let written = serde_json::from_value::<GameProfile>(upgraded)
            .map_err(|e| e.to_string())
            .and_then(|profile| {
                let file = ProfileFile { key: key.clone(), profile };
                versioned::write(&path, &file, PROFILE_MIGRATIONS)
            });
        all_written &= written.is_ok();
    }

    let old_styles = data_dir.join("styles");
    for entry in std::fs::read_dir(&old_styles).into_iter().flatten().flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let (game, target) = if path.is_dir() {
            (name.clone(), "prints".to_string())
        } else if let Some(game) = name.strip_suffix(".json") {
            (game.to_string(), "styles.json".to_string())
        } else {
            continue;
        };
        let dest = games_dir(data_dir).join(&game).join(target);
        if dest.exists() || std::fs::create_dir_all(games_dir(data_dir).join(&game)).is_err() {
            continue;
        }
        let _ = std::fs::rename(&path, &dest);
    }
    let _ = std::fs::remove_dir(&old_styles); // only goes when everything moved

    all_written
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::versioned::tests::temp_dir;
    use serde_json::json;

    #[test]
    fn first_format_profiles_become_one_area() {
        let old = json!({ "title": "Hades II", "areaMode": "dialogue", "region": [1, 2, 3, 4] });
        let profile: GameProfile = serde_json::from_value(versioned::upgrade(old, PROFILE_MIGRATIONS)).unwrap();
        assert_eq!(profile.areas.len(), 1);
        assert_eq!(profile.areas[0].region, Some([1, 2, 3, 4]));
        assert_eq!(profile.areas[0].style, "inPlace");
    }

    #[test]
    fn the_old_game_style_becomes_in_place() {
        let old = json!({ "areas": [{ "style": "game" }, { "style": "classic" }] });
        let profile: GameProfile = serde_json::from_value(versioned::upgrade(old, PROFILE_MIGRATIONS)).unwrap();
        assert_eq!(profile.areas[0].style, "inPlace");
        assert_eq!(profile.areas[1].style, "classic");
    }

    #[test]
    fn profiles_are_saved_and_read_back_by_key() {
        let dir = temp_dir("profiles-round-trip");
        let store = ProfileStore::load(&dir);
        let profile = GameProfile {
            title: "Hades II".into(),
            areas: vec![GameArea { region: Some([120, 840, 1680, 200]), ..GameArea::default() }],
        };
        store.save("hades2.exe", profile.clone()).unwrap();
        store.save("odd name?.exe", GameProfile::default()).unwrap();

        let again = ProfileStore::load(&dir);
        assert_eq!(again.get("hades2.exe"), Some(profile));
        assert!(again.get("odd name?.exe").is_some()); // the real key, not the folder name
        let text = std::fs::read_to_string(dir.join("games/hades2.exe/profile.json")).unwrap();
        assert!(text.contains("\"version\": 1"));
    }

    #[test]
    fn glossaries_are_saved_in_the_game_folder() {
        let dir = temp_dir("profiles-glossary");
        let store = ProfileStore::load(&dir);
        assert!(store.glossary("hades2.exe").is_empty());
        let entries = vec![GlossaryEntry { source: "Melinoë".into(), target: String::new(), keep: true }];
        store.save_glossary("hades2.exe", entries.clone()).unwrap();
        assert_eq!(ProfileStore::load(&dir).glossary("hades2.exe"), entries);
        let text = std::fs::read_to_string(dir.join("games/hades2.exe/glossary.json")).unwrap();
        assert!(text.contains("\"version\": 0"));
    }

    #[test]
    fn forgetting_a_game_removes_its_folder() {
        let dir = temp_dir("profiles-forget");
        let store = ProfileStore::load(&dir);
        store.update("hades2.exe", |p| p.title = "Hades II".into()).unwrap();
        std::fs::create_dir_all(dir.join("games/hades2.exe/prints")).unwrap();
        store.forget("hades2.exe");
        assert!(store.get("hades2.exe").is_none());
        assert!(!dir.join("games/hades2.exe").exists());
    }

    #[test]
    fn old_data_moves_into_game_folders() {
        let dir = temp_dir("profiles-move");
        std::fs::create_dir_all(dir.join("styles/hades2.exe")).unwrap();
        std::fs::write(dir.join("styles/hades2.exe.json"), r#"{ "styles": [] }"#).unwrap();
        std::fs::write(dir.join("styles/hades2.exe/s1.png"), b"png").unwrap();
        let old = BTreeMap::from([("hades2.exe".to_string(), json!({ "title": "Hades II", "region": [1, 2, 3, 4] }))]);

        assert!(move_old_data(&dir, &old));
        let store = ProfileStore::load(&dir);
        assert_eq!(store.get("hades2.exe").unwrap().areas[0].region, Some([1, 2, 3, 4]));
        assert!(dir.join("games/hades2.exe/styles.json").exists());
        assert!(dir.join("games/hades2.exe/prints/s1.png").exists());
        assert!(!dir.join("styles").exists());
    }

    #[test]
    fn moving_again_overwrites_nothing() {
        let dir = temp_dir("profiles-move-twice");
        let store = ProfileStore::load(&dir);
        store.update("hades2.exe", |p| p.title = "New".into()).unwrap();
        let old = BTreeMap::from([("hades2.exe".to_string(), json!({ "title": "Old" }))]);
        assert!(move_old_data(&dir, &old));
        assert_eq!(ProfileStore::load(&dir).get("hades2.exe").unwrap().title, "New");
    }
}
