//! Translation cache: the same line with the same translator and language comes back instantly.
//!
//! Kept in memory while the app runs and saved to a JSON file when a session
//! stops and when the app closes, so menus, repeated lines and songs heard
//! before come back instantly next time too.

use std::collections::{HashMap, VecDeque};
use std::path::Path;

/// Size-limited cache; when full, drops the oldest entry (FIFO).
pub struct TranslationCache {
    entries: HashMap<String, String>,
    order: VecDeque<String>,
    capacity: usize,
    dirty: bool, // changed since the last load/save
}

impl TranslationCache {
    /// Creates a cache that holds at most `capacity` translations.
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            capacity: capacity.max(1),
            dirty: false,
        }
    }

    /// Builds the key: translator + languages + normalized text (collapsed whitespace).
    pub fn key(translator: &str, source: &str, target: &str, text: &str) -> String {
        let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
        format!("{translator}\u{1f}{source}\u{1f}{target}\u{1f}{normalized}")
    }

    /// Looks up an existing translation.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    /// Stores a translation, dropping the oldest one if the cache is full.
    pub fn insert(&mut self, key: String, translation: String) {
        self.dirty = true;
        if self.entries.contains_key(&key) {
            self.entries.insert(key, translation);
            return;
        }
        if self.order.len() >= self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            }
        }
        self.order.push_back(key.clone());
        self.entries.insert(key, translation);
    }

    /// Loads a cache saved by `save`; a missing or broken file gives an empty cache.
    pub fn load(path: &Path, capacity: usize) -> Self {
        let mut cache = Self::new(capacity);
        let entries: Vec<(String, String)> = std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        for (key, translation) in entries {
            cache.insert(key, translation);
        }
        cache.dirty = false;
        cache
    }

    /// Writes the cache (oldest first, so loading keeps the order) when it changed.
    pub fn save(&mut self, path: &Path) -> std::io::Result<()> {
        if !self.dirty {
            return Ok(());
        }
        let entries: Vec<(&String, &String)> = self
            .order
            .iter()
            .filter_map(|key| self.entries.get(key).map(|value| (key, value)))
            .collect();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("tmp"); // write then rename: a crash never leaves half a file
        std::fs::write(&tmp, serde_json::to_vec(&entries)?)?;
        std::fs::rename(&tmp, path)?;
        self.dirty = false;
        Ok(())
    }

    /// Number of stored translations.
    /// Forgets every translation (the file is rewritten empty on the next save).
    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.dirty = true;
    }

    /// Number of stored translations.
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn different_whitespace_gives_same_key() {
        let a = TranslationCache::key("llm", "en", "pt-BR", "Hello,   traveler!\n");
        let b = TranslationCache::key("llm", "en", "pt-BR", "Hello, traveler!");
        assert_eq!(a, b);
    }

    #[test]
    fn survives_a_save_and_load() {
        let path = std::env::temp_dir().join(format!("sq-cache-test-{}.json", std::process::id()));
        let mut cache = TranslationCache::new(10);
        cache.insert("a".into(), "um".into());
        cache.insert("b".into(), "dois".into());
        cache.save(&path).unwrap();
        let loaded = TranslationCache::load(&path, 10);
        assert_eq!(loaded.get("b"), Some("dois"));
        assert_eq!(loaded.len(), 2);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn drops_oldest_when_full() {
        let mut cache = TranslationCache::new(2);
        cache.insert("a".into(), "1".into());
        cache.insert("b".into(), "2".into());
        cache.insert("c".into(), "3".into());
        assert_eq!(cache.get("a"), None);
        assert_eq!(cache.get("c"), Some("3"));
        assert_eq!(cache.len(), 2);
    }
}
