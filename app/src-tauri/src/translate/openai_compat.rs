//! Backend for any API compatible with the OpenAI format
//! (`POST {base_url}/chat/completions`).
//!
//! One implementation covers many services by changing only `base_url` and `model`:
//! OpenAI, Google Gemini (compatible endpoint), OpenRouter, Ollama and
//! llama.cpp server / LM Studio rodando localmente.

use super::{TranslateError, TranslationRequest, Translator};

pub struct OpenAiCompatTranslator {
    pub base_url: String, // ex.: "http://localhost:11434/v1" (Ollama)
    pub api_key: String,  // empty for local servers
    pub model: String,
}

impl OpenAiCompatTranslator {
    /// Builds the system prompt: role, languages, glossary and output rules.
    /// We explicitly ask it to fix likely OCR mistakes, which is where
    /// traditional translators (and MORT) fail the most.
    pub fn system_prompt(request: &TranslationRequest) -> String {
        let mut prompt = format!(
            "You translate video game text from {src} to {dst}.\n\
             The input comes from OCR and may contain recognition errors \
             (e.g. 'l' read as 'I', missing spaces, stray symbols). Silently fix them.\n\
             Keep the tone, the character's voice and line breaks. \
             Answer with the translation only, no notes.",
            src = request.source_lang,
            dst = request.target_lang,
        );
        if !request.glossary.is_empty() {
            prompt.push_str("\nAlways translate these terms exactly as given:\n");
            for (term, translation) in &request.glossary {
                prompt.push_str(&format!("- {term} => {translation}\n"));
            }
        }
        prompt
    }

    /// Builds the messages: previous lines as user/assistant pairs (context)
    /// and, last, the current line.
    pub fn messages(request: &TranslationRequest) -> Vec<(&'static str, String)> {
        let mut messages = vec![("system", Self::system_prompt(request))];
        for line in &request.context {
            messages.push(("user", line.source.clone()));
            messages.push(("assistant", line.translation.clone()));
        }
        messages.push(("user", request.text.clone()));
        messages
    }
}

impl Translator for OpenAiCompatTranslator {
    /// Identifier used by the cache and the settings.
    fn id(&self) -> &'static str {
        "openai-compatible"
    }

    /// TODO(phase 1): send `messages()` with reqwest (stream: true) and forward
    /// chunks to the overlay as they arrive; low temperature (0.2–0.3).
    fn translate(&self, _request: &TranslationRequest) -> Result<String, TranslateError> {
        if self.base_url.trim().is_empty() {
            return Err(TranslateError::NotConfigured("API address".into()));
        }
        if self.model.trim().is_empty() {
            return Err(TranslateError::NotConfigured("model".into()));
        }
        Err(TranslateError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::translate::ContextLine;

    fn request() -> TranslationRequest {
        TranslationRequest {
            text: "Where is the Pale King?".into(),
            source_lang: "en".into(),
            target_lang: "pt-BR".into(),
            context: vec![ContextLine {
                source: "Hello.".into(),
                translation: "Olá.".into(),
            }],
            glossary: vec![("Pale King".into(), "Rei Pálido".into())],
        }
    }

    #[test]
    fn prompt_includes_glossary_and_languages() {
        let prompt = OpenAiCompatTranslator::system_prompt(&request());
        assert!(prompt.contains("from en to pt-BR"));
        assert!(prompt.contains("Pale King => Rei Pálido"));
    }

    #[test]
    fn context_comes_before_current_line() {
        let msgs = OpenAiCompatTranslator::messages(&request());
        assert_eq!(msgs.len(), 4);
        assert_eq!(msgs[1], ("user", "Hello.".to_string()));
        assert_eq!(msgs[2], ("assistant", "Olá.".to_string()));
        assert_eq!(msgs[3].1, "Where is the Pale King?");
    }

    #[test]
    fn missing_address_reports_what_is_missing() {
        let t = OpenAiCompatTranslator {
            base_url: "".into(),
            api_key: "".into(),
            model: "x".into(),
        };
        assert_eq!(
            t.translate(&request()),
            Err(TranslateError::NotConfigured("API address".into()))
        );
    }
}
