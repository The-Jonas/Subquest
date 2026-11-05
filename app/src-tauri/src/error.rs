//! Erro que vai para a interface: um `kind` estável (a interface traduz) e um
//! `detail` técnico (vai para o console, útil para depurar).

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub kind: &'static str, // chave em `errors.<kind>` nos arquivos de idioma
    pub detail: String,
}

impl AppError {
    /// A DLL do núcleo não foi carregada.
    pub fn core_missing(detail: impl Into<String>) -> Self {
        Self {
            kind: "coreMissing",
            detail: detail.into(),
        }
    }

    /// Converte um `SqStatus` do núcleo (ver core_api.h) no `kind` correspondente.
    pub fn from_status(status: i32, detail: impl Into<String>) -> Self {
        let kind = match status {
            1 => "invalidArg",
            2 => "alreadyRunning",
            3 => "notRunning",
            4 => "platform",
            5 => "notImplemented",
            _ => "unknown",
        };
        Self {
            kind,
            detail: detail.into(),
        }
    }

    /// Qualquer outra falha do lado Rust (arquivo, conversão de texto...).
    pub fn other(detail: impl Into<String>) -> Self {
        Self {
            kind: "unknown",
            detail: detail.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_do_nucleo_vira_kind() {
        assert_eq!(AppError::from_status(5, "x").kind, "notImplemented");
        assert_eq!(AppError::from_status(4, "x").kind, "platform");
        assert_eq!(AppError::from_status(99, "x").kind, "unknown");
    }
}
