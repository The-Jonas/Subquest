//! Quick translators: plain machine translation that answers in a few hundred
//! milliseconds. They don't replace the LLM (no context, no OCR fixing, flat
//! tone); they fill the gap while it thinks, so a line the game shows for one
//! second still gets a translation on screen.
//!
//! - Google: the free endpoint the browser extension uses. No key, but it isn't
//!   an official API and Google may change or limit it.
//! - DeepL: official API, free plan with 500k characters a month (needs a key).

use std::time::Duration;

use serde_json::Value;

use super::TranslateError;

const TIMEOUT: Duration = Duration::from_secs(4); // after this the quick answer is no longer quick
const GOOGLE_URL: &str = "https://translate.googleapis.com/translate_a/single";
const DEEPL_FREE_URL: &str = "https://api-free.deepl.com/v2/translate";
const DEEPL_PRO_URL: &str = "https://api.deepl.com/v2/translate";

/// Which quick translator to use; built from the user's settings.
pub enum QuickTranslator {
    Google,
    DeepL { api_key: String },
}

impl QuickTranslator {
    /// The configured quick translator, or None when the layer is turned off.
    pub fn from_settings(engine: &str, deepl_key: &str) -> Option<Self> {
        match engine {
            "google" => Some(Self::Google),
            "deepl" if !deepl_key.trim().is_empty() => Some(Self::DeepL {
                api_key: deepl_key.trim().to_string(),
            }),
            _ => None,
        }
    }

    /// Translates one piece of text (line breaks are kept).
    pub async fn translate(
        &self,
        http: &reqwest::Client,
        text: &str,
        source: &str,
        target: &str,
    ) -> Result<String, TranslateError> {
        match self {
            Self::Google => google(http, text, source, target).await,
            Self::DeepL { api_key } => deepl(http, api_key, text, source, target).await,
        }
    }
}

/// Google's language codes for ours.
fn google_lang(tag: &str) -> &str {
    match tag {
        "pt-BR" => "pt",
        "zh-Hans" => "zh-CN",
        other => other,
    }
}

/// Free Google endpoint: GET with the text in the query; the answer is a JSON
/// array whose first item lists the translated pieces.
async fn google(http: &reqwest::Client, text: &str, source: &str, target: &str) -> Result<String, TranslateError> {
    let response = http
        .get(GOOGLE_URL)
        .query(&[
            ("client", "gtx"),
            ("sl", google_lang(source)),
            ("tl", google_lang(target)),
            ("dt", "t"),
            ("q", text),
        ])
        .timeout(TIMEOUT)
        .send()
        .await
        .map_err(|e| TranslateError::Network(e.to_string()))?;
    if !response.status().is_success() {
        return Err(TranslateError::Provider(format!("Google: HTTP {}", response.status())));
    }
    let body: Value = response
        .json()
        .await
        .map_err(|e| TranslateError::Provider(e.to_string()))?;
    parse_google(&body)
}

/// Joins the translated pieces of a Google answer: [[["piece", "source", ...], ...], ...].
fn parse_google(body: &Value) -> Result<String, TranslateError> {
    let pieces = body
        .get(0)
        .and_then(Value::as_array)
        .ok_or_else(|| TranslateError::Provider(format!("Google: unexpected answer {body}")))?;
    let text: String = pieces
        .iter()
        .filter_map(|p| p.get(0).and_then(Value::as_str))
        .collect();
    Ok(text.trim().to_string())
}

/// DeepL's language codes: the target needs the variant for Portuguese.
fn deepl_lang(tag: &str, is_target: bool) -> &'static str {
    match tag {
        "pt-BR" if is_target => "PT-BR",
        "pt-BR" => "PT",
        "zh-Hans" => "ZH",
        "ja" => "JA",
        "ko" => "KO",
        "es" => "ES",
        _ if is_target => "EN-US",
        _ => "EN",
    }
}

/// Official DeepL API. Free keys end in ":fx" and use their own address.
async fn deepl(
    http: &reqwest::Client,
    api_key: &str,
    text: &str,
    source: &str,
    target: &str,
) -> Result<String, TranslateError> {
    let url = if api_key.ends_with(":fx") { DEEPL_FREE_URL } else { DEEPL_PRO_URL };
    let response = http
        .post(url)
        .header("Authorization", format!("DeepL-Auth-Key {api_key}"))
        .form(&[
            ("text", text),
            ("source_lang", deepl_lang(source, false)),
            ("target_lang", deepl_lang(target, true)),
            ("preserve_formatting", "1"),
        ])
        .timeout(TIMEOUT)
        .send()
        .await
        .map_err(|e| TranslateError::Network(e.to_string()))?;
    if !response.status().is_success() {
        return Err(TranslateError::Provider(format!("DeepL: HTTP {}", response.status())));
    }
    let body: Value = response
        .json()
        .await
        .map_err(|e| TranslateError::Provider(e.to_string()))?;
    body["translations"][0]["text"]
        .as_str()
        .map(|t| t.trim().to_string())
        .ok_or_else(|| TranslateError::Provider(format!("DeepL: unexpected answer {body}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn joins_google_pieces_and_keeps_line_breaks() {
        let body = json!([[["Todos são um\n", "All are one\n", null], ["PAREM", "CEASE", null]], null, "en"]);
        assert_eq!(parse_google(&body).unwrap(), "Todos são um\nPAREM");
    }

    #[test]
    fn deepl_needs_a_key_google_does_not() {
        assert!(QuickTranslator::from_settings("google", "").is_some());
        assert!(QuickTranslator::from_settings("deepl", "").is_none());
        assert!(QuickTranslator::from_settings("deepl", "abc:fx").is_some());
        assert!(QuickTranslator::from_settings("", "abc").is_none());
    }

    #[test]
    fn languages_map_to_each_service() {
        assert_eq!(google_lang("pt-BR"), "pt");
        assert_eq!(deepl_lang("pt-BR", true), "PT-BR");
        assert_eq!(deepl_lang("en", false), "EN");
    }
}
