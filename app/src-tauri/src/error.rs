//! Error sent to the UI: a stable `kind` (the UI translates it) and a technical
//! `detail` (goes to the console, useful for debugging).

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub kind: &'static str, // key in `errors.<kind>` in the locale files
    pub detail: String,
}

impl AppError {
    /// The core DLL was not loaded.
    pub fn core_missing(detail: impl Into<String>) -> Self {
        Self {
            kind: "coreMissing",
            detail: detail.into(),
        }
    }

    /// Maps a core `SqStatus` (see core_api.h) to the matching `kind`.
    pub fn from_status(status: i32, detail: impl Into<String>) -> Self {
        let kind = match status {
            1 => "invalidArg",
            2 => "alreadyRunning",
            3 => "notRunning",
            4 => "platform",
            5 => "notImplemented",
            6 => "ocrLanguage",
            _ => "unknown",
        };
        Self {
            kind,
            detail: detail.into(),
        }
    }

    /// Any other failure on the Rust side (file, text conversion...).
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
    fn core_status_maps_to_kind() {
        assert_eq!(AppError::from_status(5, "x").kind, "notImplemented");
        assert_eq!(AppError::from_status(4, "x").kind, "platform");
        assert_eq!(AppError::from_status(6, "x").kind, "ocrLanguage");
        assert_eq!(AppError::from_status(99, "x").kind, "unknown");
    }
}
