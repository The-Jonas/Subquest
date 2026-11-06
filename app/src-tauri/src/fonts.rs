//! Fonts imported by the user, kept in the app's own data folder
//! (`%APPDATA%\Subquest\fonts`). Nothing is installed in
//! Windows, so no admin rights are needed and removing the app removes them.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::AppError;

const ALLOWED: [&str; 4] = ["ttf", "otf", "woff", "woff2"];
const MAX_BYTES: usize = 20 * 1024 * 1024; // font files are small; anything bigger is a mistake

/// An imported font, as the UI lists it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontEntry {
    pub file: String, // file name inside the fonts folder (also its id)
    pub name: String, // display name: the file name without extension
}

/// The fonts folder inside the app data folder.
pub fn folder(data_dir: &Path) -> PathBuf {
    data_dir.join("fonts")
}

/// Keeps only letters, digits, spaces, dashes and dots, so a file name can't
/// escape the fonts folder ("..\..\x") or contain odd characters.
pub fn safe_file_name(name: &str) -> Option<String> {
    // Split on both separators ourselves: Path only knows the current OS's one.
    let base = name.rsplit(['/', '\\']).next()?.to_string();
    let cleaned: String = base
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.'))
        .collect();
    let ext = Path::new(&cleaned)
        .extension()?
        .to_string_lossy()
        .to_lowercase();
    if !ALLOWED.contains(&ext.as_str()) || cleaned.starts_with('.') {
        return None;
    }
    Some(cleaned)
}

/// Display name of a font file: "Cinzel-Regular.ttf" -> "Cinzel-Regular".
fn display_name(file: &str) -> String {
    Path::new(file)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| file.to_string())
}

/// Saves the bytes of a font file. The first bytes are checked against the
/// known font signatures, so a renamed image or text file is refused.
pub fn import(data_dir: &Path, original_name: &str, bytes: &[u8]) -> Result<FontEntry, AppError> {
    let file = safe_file_name(original_name).ok_or_else(|| {
        AppError::other("unsupported font file (use .ttf, .otf, .woff or .woff2)")
    })?;
    if bytes.len() > MAX_BYTES {
        return Err(AppError::other("font file is too large"));
    }
    if !looks_like_font(bytes) {
        return Err(AppError::other("the file is not a font"));
    }
    let dir = folder(data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| AppError::other(e.to_string()))?;
    std::fs::write(dir.join(&file), bytes).map_err(|e| AppError::other(e.to_string()))?;
    Ok(FontEntry {
        name: display_name(&file),
        file,
    })
}

/// TrueType (00 01 00 00 / "true"), OpenType ("OTTO"), WOFF ("wOFF"), WOFF2 ("wOF2").
fn looks_like_font(bytes: &[u8]) -> bool {
    matches!(
        bytes.get(..4),
        Some([0x00, 0x01, 0x00, 0x00])
            | Some(b"true")
            | Some(b"OTTO")
            | Some(b"wOFF")
            | Some(b"wOF2")
    )
}

/// Lists the imported fonts, sorted by name.
pub fn list(data_dir: &Path) -> Vec<FontEntry> {
    let Ok(entries) = std::fs::read_dir(folder(data_dir)) else {
        return Vec::new();
    };
    let mut fonts: Vec<FontEntry> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| safe_file_name(&e.file_name().to_string_lossy()))
        .map(|file| FontEntry {
            name: display_name(&file),
            file,
        })
        .collect();
    fonts.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    fonts
}

/// Reads an imported font's bytes (the UI turns them into a FontFace).
pub fn read(data_dir: &Path, file: &str) -> Result<Vec<u8>, AppError> {
    let file = safe_file_name(file).ok_or_else(|| AppError::other("invalid font name"))?;
    std::fs::read(folder(data_dir).join(file)).map_err(|e| AppError::other(e.to_string()))
}

/// Deletes an imported font.
pub fn remove(data_dir: &Path, file: &str) -> Result<(), AppError> {
    let file = safe_file_name(file).ok_or_else(|| AppError::other("invalid font name"))?;
    std::fs::remove_file(folder(data_dir).join(file)).map_err(|e| AppError::other(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_cannot_escape_the_folder() {
        assert_eq!(safe_file_name("..\\..\\evil.ttf"), Some("evil.ttf".into()));
        assert_eq!(safe_file_name("../../evil.ttf"), Some("evil.ttf".into()));
        assert_eq!(safe_file_name("notes.txt"), None);
        assert_eq!(
            safe_file_name("Cinzel-Regular.TTF"),
            Some("Cinzel-Regular.TTF".into())
        );
    }

    #[test]
    fn import_list_read_remove_round_trip() {
        let dir = std::env::temp_dir().join(format!("subquest-fonts-{}", std::process::id()));
        let fake_ttf = [0x00, 0x01, 0x00, 0x00, 1, 2, 3];
        let entry = import(&dir, "My Font.ttf", &fake_ttf).unwrap();
        assert_eq!(entry.name, "My Font");
        assert_eq!(list(&dir).len(), 1);
        assert_eq!(read(&dir, "My Font.ttf").unwrap(), fake_ttf);
        remove(&dir, "My Font.ttf").unwrap();
        assert!(list(&dir).is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn refuses_files_that_are_not_fonts() {
        let dir = std::env::temp_dir().join("subquest-fonts-bad");
        assert!(import(&dir, "fake.ttf", b"<html>").is_err());
    }
}
