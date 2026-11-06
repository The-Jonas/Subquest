//! JSON files that carry their format version, so any older file can be read
//! by every later Subquest.
//!
//! Each file has a `"version"` number (missing = 0, the formats from before
//! versioning). Reading upgrades it one step at a time through a list of
//! migrations: migration `n` turns a version `n` file into version `n + 1`, and
//! the current version is the length of the list. The migrations work on plain
//! JSON, so the structs only describe the current format and old fields never
//! need to linger in them.
//!
//! Nothing a user saved is lost silently:
//! - a damaged file is copied to `<file>.corrupt` before it is replaced;
//! - a file from a newer Subquest (after going back to an older version) is
//!   read as far as this version understands it, and copied to
//!   `<file>.v<N>.bak` before this version writes over it.

use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

/// Turns a file of one version into the next.
pub type Migration = fn(&mut Value);

/// Version written in a file (0 when it has none).
fn version_of(value: &Value) -> usize {
    value.get("version").and_then(Value::as_u64).unwrap_or(0) as usize
}

/// `<file><suffix>`, e.g. "profile.json" -> "profile.json.corrupt".
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Copies a file aside once (an existing copy is kept: it is the oldest one).
fn keep_copy(path: &Path, suffix: &str) {
    let copy = with_suffix(path, suffix);
    if !copy.exists() {
        let _ = std::fs::copy(path, copy);
    }
}

/// Upgrades a parsed file to the current version (the length of `migrations`).
/// Files from a newer version are left as they are.
pub fn upgrade(mut value: Value, migrations: &[Migration]) -> Value {
    let from = version_of(&value);
    if from > migrations.len() {
        return value;
    }
    for migrate in &migrations[from..] {
        migrate(&mut value);
    }
    if let Value::Object(map) = &mut value {
        map.insert("version".into(), migrations.len().into());
    }
    value
}

/// Reads a file, upgrading older formats. None when it is missing or damaged
/// (a damaged one is copied aside first).
pub fn read<T: DeserializeOwned>(path: &Path, migrations: &[Migration]) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    let parsed = serde_json::from_str::<Value>(&text)
        .ok()
        .filter(Value::is_object)
        .and_then(|value| serde_json::from_value(upgrade(value, migrations)).ok());
    if parsed.is_none() {
        keep_copy(path, ".corrupt");
    }
    parsed
}

/// Writes `data` with the current version number. Written to a temporary file
/// first and then moved over the old one, so a crash mid-write never leaves
/// half a file.
pub fn write<T: Serialize>(path: &Path, data: &T, migrations: &[Migration]) -> Result<(), String> {
    let mut value = serde_json::to_value(data).map_err(|e| e.to_string())?;
    if let Value::Object(map) = &mut value {
        map.insert("version".into(), migrations.len().into());
    }
    // A file from a newer Subquest is kept before this version overwrites it.
    let existing = std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .map(|v| version_of(&v));
    if let Some(newer) = existing.filter(|&v| v > migrations.len()) {
        keep_copy(path, &format!(".v{newer}.bak"));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    let temp = with_suffix(path, ".tmp");
    std::fs::write(&temp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, path).map_err(|e| e.to_string())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::json;

    /// A new empty folder for one test.
    pub fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("subquest-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
    #[serde(default)]
    struct Note {
        text: String,
        size: u32,
    }

    // v0 called the field "body"; v1 added "size".
    fn rename_body(v: &mut Value) {
        if let Some(body) = v.as_object_mut().and_then(|m| m.remove("body")) {
            v["text"] = body;
        }
    }
    fn add_size(v: &mut Value) {
        v["size"] = 12.into();
    }
    const MIGRATIONS: &[Migration] = &[rename_body, add_size];

    #[test]
    fn old_files_go_through_every_step() {
        let up = upgrade(json!({ "body": "hi" }), MIGRATIONS);
        assert_eq!(up, json!({ "text": "hi", "size": 12, "version": 2 }));
        let from_v1 = upgrade(json!({ "version": 1, "text": "hi" }), MIGRATIONS);
        assert_eq!(from_v1["size"], 12);
    }

    #[test]
    fn current_files_are_left_alone() {
        let v = json!({ "version": 2, "text": "hi", "size": 30 });
        assert_eq!(upgrade(v.clone(), MIGRATIONS), v);
    }

    #[test]
    fn round_trip_writes_the_version() {
        let dir = temp_dir("versioned-round-trip");
        let path = dir.join("note.json");
        let note = Note { text: "hi".into(), size: 30 };
        write(&path, &note, MIGRATIONS).unwrap();
        assert!(std::fs::read_to_string(&path).unwrap().contains("\"version\": 2"));
        assert_eq!(read::<Note>(&path, MIGRATIONS), Some(note));
        assert!(!with_suffix(&path, ".tmp").exists());
    }

    #[test]
    fn a_damaged_file_is_kept_aside() {
        let dir = temp_dir("versioned-damaged");
        let path = dir.join("note.json");
        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(read::<Note>(&path, MIGRATIONS), None);
        assert_eq!(std::fs::read_to_string(dir.join("note.json.corrupt")).unwrap(), "{ not json");
    }

    #[test]
    fn a_newer_file_is_read_and_kept_before_being_overwritten() {
        let dir = temp_dir("versioned-newer");
        let path = dir.join("note.json");
        let newer = r#"{ "version": 7, "text": "from the future", "size": 9, "glow": true }"#;
        std::fs::write(&path, newer).unwrap();
        let note: Note = read(&path, MIGRATIONS).unwrap();
        assert_eq!(note.text, "from the future");
        write(&path, &note, MIGRATIONS).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("note.json.v7.bak")).unwrap(), newer);
    }

    #[test]
    fn a_missing_file_is_none() {
        let dir = temp_dir("versioned-missing");
        assert_eq!(read::<Note>(&dir.join("nope.json"), MIGRATIONS), None);
        assert!(!dir.join("nope.json.corrupt").exists());
    }
}
