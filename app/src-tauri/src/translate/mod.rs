//! Tradução: interface comum + backends + cache.
//!
//! Todo backend implementa `Translator`. O resto do app não sabe se a tradução
//! vem do DeepL, de uma API de LLM na nuvem ou de um modelo local.

pub mod cache;
pub mod openai_compat;

use serde::{Deserialize, Serialize};

/// Uma fala anterior já traduzida, usada como contexto para o LLM.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ContextLine {
    pub source: String,
    pub translation: String,
}

/// Tudo o que um backend precisa para traduzir uma fala.
#[derive(Clone, Debug)]
pub struct TranslationRequest {
    pub text: String,
    pub source_lang: String,             // "en", "ja", ...
    pub target_lang: String,             // "pt-BR", ...
    pub context: Vec<ContextLine>,       // últimas falas, da mais antiga para a mais nova
    pub glossary: Vec<(String, String)>, // termo original -> tradução fixa (nomes, golpes, lugares)
}

#[derive(Debug, PartialEq)]
pub enum TranslateError {
    NotConfigured(String), // falta chave, URL ou modelo
    Network(String),       // falha de conexão / timeout
    Provider(String),      // a API respondeu com erro
    NotImplemented,        // backend ainda em stub
}

impl std::fmt::Display for TranslateError {
    /// Mensagem curta, pensada para aparecer na interface.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranslateError::NotConfigured(what) => write!(f, "falta configurar: {what}"),
            TranslateError::Network(e) => write!(f, "erro de conexão: {e}"),
            TranslateError::Provider(e) => write!(f, "o serviço de tradução recusou: {e}"),
            TranslateError::NotImplemented => write!(f, "tradutor ainda não implementado"),
        }
    }
}

/// Contrato de qualquer backend de tradução.
pub trait Translator: Send + Sync {
    fn id(&self) -> &'static str; // identificador estável, usado no cache e nas configurações
    fn translate(&self, request: &TranslationRequest) -> Result<String, TranslateError>; // traduz uma fala
}
