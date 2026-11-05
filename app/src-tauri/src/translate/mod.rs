//! Translation: common interface + backends + cache.
//!
//! Every backend implements `Translator`. The rest of the app doesn't know whether
//! the translation comes from DeepL, a cloud LLM API or a local model.

pub mod cache;
pub mod openai_compat;

use serde::{Deserialize, Serialize};

/// A previous, already translated line, used as context for the LLM.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ContextLine {
    pub source: String,
    pub translation: String,
}

/// Everything a backend needs to translate one line.
#[derive(Clone, Debug)]
pub struct TranslationRequest {
    pub text: String,
    pub source_lang: String,             // "en", "ja", ...
    pub target_lang: String,             // "pt-BR", ...
    pub context: Vec<ContextLine>,       // latest lines, oldest first
    pub glossary: Vec<(String, String)>, // source term -> fixed translation (names, moves, places)
}

#[derive(Debug, PartialEq)]
pub enum TranslateError {
    NotConfigured(String), // missing key, URL or model
    Network(String),       // connection failure / timeout
    Provider(String),      // the API answered with an error
    NotImplemented,        // backend is still a stub
}

impl std::fmt::Display for TranslateError {
    /// Short message, meant for logs and the UI detail.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranslateError::NotConfigured(what) => write!(f, "not configured: {what}"),
            TranslateError::Network(e) => write!(f, "connection error: {e}"),
            TranslateError::Provider(e) => write!(f, "the translation service refused: {e}"),
            TranslateError::NotImplemented => write!(f, "translator not implemented yet"),
        }
    }
}

/// Contract for any translation backend.
pub trait Translator: Send + Sync {
    fn id(&self) -> &'static str; // stable identifier, used by the cache and settings
    fn translate(&self, request: &TranslationRequest) -> Result<String, TranslateError>; // translates one line
}
