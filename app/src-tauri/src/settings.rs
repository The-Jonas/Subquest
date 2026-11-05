//! Configurações do usuário, salvas em JSON na pasta de config do app
//! (`%APPDATA%\io.github.the-jonas.subquest\settings.json`). Nunca no repositório.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub ui_language: String,     // idioma da interface: "pt-BR", "en", "es"
    pub source_language: String, // idioma do jogo
    pub target_language: String, // idioma da tradução
    pub ocr_engine: String,      // "windows" | "paddle"
    pub translator: TranslatorSettings,
    pub overlay: OverlaySettings,
    pub tutorial_done: bool, // false => abre o tutorial na primeira execução
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct TranslatorSettings {
    pub kind: String, // "openai-compatible" | "deepl"
    pub base_url: String,
    pub model: String,
    // TODO(fase 3): mover a chave para o Gerenciador de Credenciais do Windows.
    pub api_key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct OverlaySettings {
    pub font_size: u32,
    pub background_opacity: f32, // 0..1
    pub show_original: bool,     // mostrar o texto original acima da tradução
}

impl Default for Settings {
    /// Padrões pensados para o caso mais comum: jogo em inglês, tradução para PT-BR.
    fn default() -> Self {
        Self {
            ui_language: "pt-BR".into(),
            source_language: "en".into(),
            target_language: "pt-BR".into(),
            ocr_engine: "windows".into(),
            translator: TranslatorSettings::default(),
            overlay: OverlaySettings::default(),
            tutorial_done: false,
        }
    }
}

impl Default for TranslatorSettings {
    /// Começa apontando para um Ollama local: funciona sem chave nenhuma.
    fn default() -> Self {
        Self {
            kind: "openai-compatible".into(),
            base_url: "http://localhost:11434/v1".into(),
            model: String::new(),
            api_key: String::new(),
        }
    }
}

impl Default for OverlaySettings {
    /// Fonte grande e fundo escuro o bastante para ler sobre qualquer cenário.
    fn default() -> Self {
        Self {
            font_size: 22,
            background_opacity: 0.78,
            show_original: false,
        }
    }
}

/// Caminho do arquivo de configurações dentro da pasta de config do app.
pub fn settings_path(config_dir: &Path) -> PathBuf {
    config_dir.join("settings.json")
}

/// Lê as configurações; se o arquivo não existe ou está corrompido, volta aos padrões.
pub fn load(config_dir: &Path) -> Settings {
    std::fs::read_to_string(settings_path(config_dir))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Grava as configurações, criando a pasta se preciso.
pub fn save(config_dir: &Path, settings: &Settings) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(settings_path(config_dir), text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campos_faltando_usam_o_padrao() {
        // Simula um settings.json de uma versão antiga, sem as seções novas.
        let s: Settings = serde_json::from_str(r#"{ "uiLanguage": "en" }"#).unwrap();
        assert_eq!(s.ui_language, "en");
        assert_eq!(s.target_language, "pt-BR");
        assert_eq!(s.overlay.font_size, 22);
    }
}
