//! Translation: request types, the streaming backend, the cache and the
//! per-session state (context of previous lines).
//!
//! There is a single backend for now (OpenAI-compatible, which covers Gemini,
//! OpenAI, OpenRouter, Ollama and LM Studio). When a second one arrives
//! (e.g. DeepL), an enum over the backends goes here.

pub mod cache;
pub mod openai_compat;
pub mod sse;

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
    Busy(String),          // overloaded (503) or rate-limited (429), even after retrying
    Cancelled,             // a newer line arrived; this one is no longer worth finishing
}

impl TranslateError {
    /// Stable key for the UI (`errors.<kind>` in the locale files).
    pub fn kind(&self) -> &'static str {
        match self {
            TranslateError::NotConfigured(_) => "translatorNotConfigured",
            TranslateError::Network(_) => "translatorNetwork",
            TranslateError::Provider(_) => "translatorProvider",
            TranslateError::Busy(_) => "translatorBusy",
            TranslateError::Cancelled => "cancelled",
        }
    }
}

impl std::fmt::Display for TranslateError {
    /// Short message, meant for logs and the UI detail.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranslateError::NotConfigured(what) => write!(f, "not configured: {what}"),
            TranslateError::Network(e) => write!(f, "connection error: {e}"),
            TranslateError::Provider(e) => write!(f, "the translation service refused: {e}"),
            TranslateError::Busy(e) => write!(f, "the translation service is busy: {e}"),
            TranslateError::Cancelled => write!(f, "cancelled by a newer line"),
        }
    }
}

/// English name of a language tag, so the prompt says "Brazilian Portuguese"
/// instead of a bare "pt-BR" (models follow plain names more reliably).
pub fn language_name(tag: &str) -> &str {
    match tag {
        "en" => "English",
        "ja" => "Japanese",
        "zh-Hans" => "Simplified Chinese",
        "ko" => "Korean",
        "pt-BR" => "Brazilian Portuguese",
        "es" => "Spanish",
        other => other,
    }
}
