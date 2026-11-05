//! Translation cache: the same line with the same translator and language comes back instantly.
//!
//! In memory for now. TODO(phase 3): persist to SQLite per game so the cache
//! survives between sessions.

use std::collections::{HashMap, VecDeque};

/// Size-limited cache; when full, drops the oldest entry (FIFO).
pub struct TranslationCache {
    entries: HashMap<String, String>,
    order: VecDeque<String>,
    capacity: usize,
}

impl TranslationCache {
    /// Creates a cache that holds at most `capacity` translations.
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            capacity: capacity.max(1),
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

    /// Number of stored translations.
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
