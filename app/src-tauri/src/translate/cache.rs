//! Cache de traduções: a mesma fala com o mesmo tradutor e idioma sai na hora.
//!
//! Por enquanto fica em memória. TODO(fase 3): persistir em SQLite por jogo,
//! para que o cache sobreviva entre sessões.

use std::collections::{HashMap, VecDeque};

/// Cache com limite de itens; quando enche, descarta o mais antigo (FIFO).
pub struct TranslationCache {
    entries: HashMap<String, String>,
    order: VecDeque<String>,
    capacity: usize,
}

impl TranslationCache {
    /// Cria um cache que guarda no máximo `capacity` traduções.
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            capacity: capacity.max(1),
        }
    }

    /// Monta a chave: tradutor + idiomas + texto normalizado (espaços colapsados).
    pub fn key(translator: &str, source: &str, target: &str, text: &str) -> String {
        let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
        format!("{translator}\u{1f}{source}\u{1f}{target}\u{1f}{normalized}")
    }

    /// Busca uma tradução já feita.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    /// Guarda uma tradução, descartando a mais antiga se o cache estiver cheio.
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

    /// Quantidade de traduções guardadas.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn espacos_diferentes_geram_a_mesma_chave() {
        let a = TranslationCache::key("llm", "en", "pt-BR", "Hello,   traveler!\n");
        let b = TranslationCache::key("llm", "en", "pt-BR", "Hello, traveler!");
        assert_eq!(a, b);
    }

    #[test]
    fn descarta_o_mais_antigo_quando_enche() {
        let mut cache = TranslationCache::new(2);
        cache.insert("a".into(), "1".into());
        cache.insert("b".into(), "2".into());
        cache.insert("c".into(), "3".into());
        assert_eq!(cache.get("a"), None);
        assert_eq!(cache.get("c"), Some("3"));
        assert_eq!(cache.len(), 2);
    }
}
