//! Per-game glossary: names, places and terms that must always come out the
//! same way (or never be translated at all).
//!
//! Nothing here needs to know what a word *is*: the OCR text is plain text, and
//! a term applies to a line when the line contains it. Only the terms found in
//! a line go to the translator, so the prompt stays short and lines without
//! any term keep their cached translations.

use serde::{Deserialize, Serialize};

/// One entry of a game's glossary.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct GlossaryEntry {
    pub source: String, // as the game writes it, e.g. "Pale King"
    pub target: String, // fixed translation, e.g. "Rei Pálido" (ignored when `keep`)
    pub keep: bool,     // never translated: names like "Wesnoth" stay as they are
}

impl GlossaryEntry {
    /// What the term must become in the translation.
    fn translation(&self) -> &str {
        if self.keep || self.target.trim().is_empty() {
            self.source.trim()
        } else {
            self.target.trim()
        }
    }
}

/// Lowercase text with every run of whitespace turned into one space, so a
/// term still matches when the game wrapped it across two lines.
fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Letters that make words with their neighbors (Latin, Greek, Cyrillic...).
/// Scripts written without spaces (Japanese, Chinese) can't have word edges,
/// so a term there matches anywhere.
fn joins_words(c: char) -> bool {
    c.is_alphanumeric() && (c as u32) < 0x2E80
}

/// Whether `term` appears in `text` as a whole word ("King" doesn't match "Kingdom").
fn contains_term(text: &str, term: &str) -> bool {
    let mut from = 0;
    while let Some(at) = text[from..].find(term) {
        let start = from + at;
        let end = start + term.len();
        let before = text[..start].chars().next_back();
        let after = text[end..].chars().next();
        let first = term.chars().next();
        let last = term.chars().next_back();
        let edge_ok = |edge: Option<char>, side: Option<char>| {
            !(edge.is_some_and(joins_words) && side.is_some_and(joins_words))
        };
        if edge_ok(first, before) && edge_ok(last, after) {
            return true;
        }
        from = start + term.chars().next().map_or(1, char::len_utf8);
    }
    false
}

/// Entries found in a line, as (term, translation) pairs for the translator.
/// Case and line breaks don't matter.
pub fn matching(entries: &[GlossaryEntry], text: &str) -> Vec<(String, String)> {
    let text = normalize(text);
    let mut found: Vec<(String, String)> = Vec::new();
    for entry in entries {
        let term = normalize(&entry.source);
        if term.is_empty() || found.iter().any(|(s, _)| normalize(s) == term) {
            continue;
        }
        if contains_term(&text, &term) {
            found.push((entry.source.trim().to_string(), entry.translation().to_string()));
        }
    }
    found
}

/// Short fingerprint of the terms used for a line, part of its cache key: a
/// line translated with other glossary terms is translated again.
pub fn signature(terms: &[(String, String)]) -> String {
    terms.iter().map(|(s, t)| format!("{s}={t}")).collect::<Vec<_>>().join(";")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(source: &str, target: &str, keep: bool) -> GlossaryEntry {
        GlossaryEntry { source: source.into(), target: target.into(), keep }
    }

    #[test]
    fn finds_terms_whatever_the_case_and_line_breaks() {
        let entries = [entry("Pale King", "Rei Pálido", false)];
        let found = matching(&entries, "The PALE\nking awaits.");
        assert_eq!(found, vec![("Pale King".to_string(), "Rei Pálido".to_string())]);
    }

    #[test]
    fn matches_whole_words_only() {
        let entries = [entry("King", "Rei", false)];
        assert!(matching(&entries, "The Kingdom falls").is_empty());
        assert_eq!(matching(&entries, "Long live the King!").len(), 1);
    }

    #[test]
    fn kept_names_translate_to_themselves() {
        let entries = [entry("Wesnoth", "", true), entry("Konrad", "Conrado", true)];
        let found = matching(&entries, "Konrad must defend Wesnoth.");
        assert_eq!(found[0], ("Wesnoth".to_string(), "Wesnoth".to_string()));
        assert_eq!(found[1], ("Konrad".to_string(), "Konrad".to_string()));
    }

    #[test]
    fn japanese_terms_match_inside_sentences() {
        let entries = [entry("勇者", "Herói", false)];
        assert_eq!(matching(&entries, "勇者よ、目覚めなさい").len(), 1);
    }

    #[test]
    fn empty_and_repeated_entries_are_skipped() {
        let entries = [entry("", "x", false), entry("Hollow", "Vazio", false), entry("hollow", "Oco", false)];
        assert_eq!(matching(&entries, "Hollow Knight"), vec![("Hollow".to_string(), "Vazio".to_string())]);
    }

    #[test]
    fn lines_without_terms_have_no_signature() {
        assert_eq!(signature(&[]), "");
        assert_eq!(signature(&[("A".into(), "B".into())]), "A=B");
    }
}
