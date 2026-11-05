//! Backend para qualquer API compatível com o formato da OpenAI
//! (`POST {base_url}/chat/completions`).
//!
//! Uma implementação cobre vários serviços, mudando só `base_url` e `model`:
//! OpenAI, Google Gemini (endpoint compatível), OpenRouter, Ollama e
//! llama.cpp server / LM Studio rodando localmente.

use super::{TranslateError, TranslationRequest, Translator};

pub struct OpenAiCompatTranslator {
    pub base_url: String, // ex.: "http://localhost:11434/v1" (Ollama)
    pub api_key: String,  // vazio para servidores locais
    pub model: String,
}

impl OpenAiCompatTranslator {
    /// Monta o prompt de sistema: papel, idiomas, glossário e regras de saída.
    /// Pedimos explicitamente para corrigir erros prováveis de OCR, que é onde
    /// tradutores tradicionais (e o MORT) mais erram.
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

    /// Monta as mensagens: falas anteriores como pares user/assistant (contexto)
    /// e, por último, a fala atual.
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
    /// Identificador usado no cache e nas configurações.
    fn id(&self) -> &'static str {
        "openai-compatible"
    }

    /// TODO(fase 1): enviar `messages()` com reqwest (stream: true) e repassar
    /// os pedaços para o overlay conforme chegam; temperature baixa (0.2–0.3).
    fn translate(&self, _request: &TranslationRequest) -> Result<String, TranslateError> {
        if self.base_url.trim().is_empty() {
            return Err(TranslateError::NotConfigured("endereço da API".into()));
        }
        if self.model.trim().is_empty() {
            return Err(TranslateError::NotConfigured("modelo".into()));
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
    fn prompt_inclui_glossario_e_idiomas() {
        let prompt = OpenAiCompatTranslator::system_prompt(&request());
        assert!(prompt.contains("from en to pt-BR"));
        assert!(prompt.contains("Pale King => Rei Pálido"));
    }

    #[test]
    fn contexto_vem_antes_da_fala_atual() {
        let msgs = OpenAiCompatTranslator::messages(&request());
        assert_eq!(msgs.len(), 4);
        assert_eq!(msgs[1], ("user", "Hello.".to_string()));
        assert_eq!(msgs[2], ("assistant", "Olá.".to_string()));
        assert_eq!(msgs[3].1, "Where is the Pale King?");
    }

    #[test]
    fn sem_endereco_avisa_o_que_falta() {
        let t = OpenAiCompatTranslator {
            base_url: "".into(),
            api_key: "".into(),
            model: "x".into(),
        };
        assert_eq!(
            t.translate(&request()),
            Err(TranslateError::NotConfigured("endereço da API".into()))
        );
    }
}
