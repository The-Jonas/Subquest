//! Backend for any API compatible with the OpenAI format
//! (`POST {base_url}/chat/completions`).
//!
//! One implementation covers many services by changing only `base_url` and `model`:
//! OpenAI, Google Gemini (compatible endpoint), OpenRouter, Ollama and
//! llama.cpp server / LM Studio running locally.

use std::time::Duration;

use futures_util::StreamExt;
use serde_json::{json, Value};

use super::sse::{SseEvent, SseParser};

const RETRY_DELAYS_MS: [u64; 2] = [400, 1200]; // pauses before the 2nd and 3rd attempts

/// Overloaded (503) or rate-limited (429): the request was fine, the timing wasn't.
fn is_transient(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::SERVICE_UNAVAILABLE
        || status == reqwest::StatusCode::TOO_MANY_REQUESTS
}

/// Async pause on Tauri's runtime (tokio), without blocking the thread.
async fn tokio_sleep(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}
use super::{language_name, TranslateError, TranslationRequest};

pub struct OpenAiCompatTranslator {
    pub client: reqwest::Client,
    pub base_url: String, // e.g. "https://generativelanguage.googleapis.com/v1beta/openai/"
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
            src = language_name(&request.source_lang),
            dst = language_name(&request.target_lang),
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

    /// Joins the base URL and a path without doubling or dropping the slash.
    fn endpoint(&self, path: &str) -> String {
        format!("{}/{}", self.base_url.trim_end_matches('/'), path)
    }

    /// Adds the bearer token when there is one (local servers don't need it).
    fn authorized(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if self.api_key.trim().is_empty() {
            request
        } else {
            request.bearer_auth(self.api_key.trim())
        }
    }

    /// Checks the settings before going to the network, naming what is missing.
    fn validate(&self) -> Result<(), TranslateError> {
        if self.base_url.trim().is_empty() {
            return Err(TranslateError::NotConfigured("API address".into()));
        }
        if self.model.trim().is_empty() {
            return Err(TranslateError::NotConfigured("model".into()));
        }
        Ok(())
    }

    /// Turns a non-2xx response into an error with the start of its body:
    /// Busy for overload/rate limits (worth retrying), Provider for the rest.
    async fn provider_error(response: reqwest::Response) -> TranslateError {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let snippet: String = body.chars().take(300).collect();
        let message = format!("HTTP {status}: {snippet}");
        if is_transient(status) {
            TranslateError::Busy(message)
        } else {
            TranslateError::Provider(message)
        }
    }

    /// Sends the chat request, retrying overload/rate-limit answers a couple of
    /// times with a short pause: those spikes usually pass in under a second.
    /// Only the connection is retried; once the stream starts, it is used as is.
    async fn send_with_retry(&self, body: &Value) -> Result<reqwest::Response, TranslateError> {
        let mut attempt = 0;
        loop {
            let response = self
                .authorized(self.client.post(self.endpoint("chat/completions")))
                .json(body)
                .timeout(Duration::from_secs(30))
                .send()
                .await
                .map_err(|e| TranslateError::Network(e.to_string()))?;
            if response.status().is_success() {
                return Ok(response);
            }
            if !is_transient(response.status()) || attempt >= RETRY_DELAYS_MS.len() {
                return Err(Self::provider_error(response).await);
            }
            tokio_sleep(RETRY_DELAYS_MS[attempt]).await;
            attempt += 1;
        }
    }

    /// Streams the translation. `on_partial` gets the text accumulated so far after
    /// every chunk and returns false to cancel (a newer line made this one stale).
    /// Low temperature keeps translations stable; on Gemini, reasoning is kept low
    /// because thinking adds latency and doesn't help short dialogue lines.
    pub async fn translate(
        &self,
        request: &TranslationRequest,
        mut on_partial: impl FnMut(&str) -> bool,
    ) -> Result<String, TranslateError> {
        self.validate()?;
        let messages: Vec<Value> = Self::messages(request)
            .into_iter()
            .map(|(role, content)| json!({ "role": role, "content": content }))
            .collect();
        let mut body = json!({
            "model": self.model.trim(),
            "messages": messages,
            "stream": true,
            "temperature": 0.2,
        });
        if self.base_url.contains("generativelanguage.googleapis.com") {
            body["reasoning_effort"] = json!("low");
        }

        let response = self.send_with_retry(&body).await?;

        let mut parser = SseParser::default();
        let mut text = String::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| TranslateError::Network(e.to_string()))?;
            for event in parser.push(&String::from_utf8_lossy(&chunk)) {
                match event {
                    SseEvent::Delta(piece) => {
                        text.push_str(&piece);
                        if !on_partial(&text) {
                            return Err(TranslateError::Cancelled);
                        }
                    }
                    SseEvent::Done => return Ok(text.trim().to_string()),
                }
            }
        }
        Ok(text.trim().to_string())
    }

    /// Lists the model ids the service offers (`GET {base_url}/models`), so the
    /// user picks from real names instead of guessing them.
    pub async fn list_models(&self) -> Result<Vec<String>, TranslateError> {
        if self.base_url.trim().is_empty() {
            return Err(TranslateError::NotConfigured("API address".into()));
        }
        let response = self
            .authorized(self.client.get(self.endpoint("models")))
            .timeout(Duration::from_secs(15))
            .send()
            .await
            .map_err(|e| TranslateError::Network(e.to_string()))?;
        if !response.status().is_success() {
            return Err(Self::provider_error(response).await);
        }
        let json: Value = response
            .json()
            .await
            .map_err(|e| TranslateError::Provider(e.to_string()))?;
        let mut models: Vec<String> = json["data"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|m| m["id"].as_str())
                    // Gemini returns "models/gemini-..."; the chat endpoint wants the bare id
                    .map(|id| id.trim_start_matches("models/").to_string())
                    .collect()
            })
            .unwrap_or_default();
        models.sort();
        Ok(models)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::translate::ContextLine;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

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

    fn translator(base_url: &str) -> OpenAiCompatTranslator {
        OpenAiCompatTranslator {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            api_key: "test-key".into(),
            model: "test-model".into(),
        }
    }

    /// Serves exactly one HTTP request with `body` (sent in small pieces, like a
    /// real stream) and reports back the raw request it received.
    fn fake_server(
        content_type: &'static str,
        body: &'static str,
    ) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut buf = vec![0u8; 16384];
            let n = socket.read(&mut buf).unwrap();
            tx.send(String::from_utf8_lossy(&buf[..n]).into_owned())
                .unwrap();
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nConnection: close\r\n\r\n"
            );
            socket.write_all(head.as_bytes()).unwrap();
            for piece in body.as_bytes().chunks(17) {
                socket.write_all(piece).unwrap();
                socket.flush().unwrap();
            }
        });
        (format!("http://{addr}/v1/"), rx)
    }

    #[test]
    fn prompt_includes_glossary_and_languages() {
        let prompt = OpenAiCompatTranslator::system_prompt(&request());
        assert!(prompt.contains("from English to Brazilian Portuguese"));
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
    fn missing_model_reports_what_is_missing() {
        let mut t = translator("http://localhost:1/v1");
        t.model = " ".into();
        let result = tauri::async_runtime::block_on(t.translate(&request(), |_| true));
        assert_eq!(result, Err(TranslateError::NotConfigured("model".into())));
    }

    #[test]
    fn streams_partials_and_sends_auth() {
        let (url, requests) = fake_server(
            "text/event-stream",
            "data: {\"choices\":[{\"delta\":{\"content\":\"Onde está \"}}]}\n\n\
             data: {\"choices\":[{\"delta\":{\"content\":\"o Rei Pálido?\"}}]}\n\n\
             data: [DONE]\n\n",
        );
        let mut partials = Vec::new();
        let result = tauri::async_runtime::block_on(translator(&url).translate(&request(), |p| {
            partials.push(p.to_string());
            true
        }));
        assert_eq!(result, Ok("Onde está o Rei Pálido?".into()));
        assert_eq!(partials, vec!["Onde está ", "Onde está o Rei Pálido?"]);

        let raw = requests.recv().unwrap();
        assert!(raw.starts_with("POST /v1/chat/completions"));
        assert!(raw
            .to_lowercase()
            .contains("authorization: bearer test-key"));
    }

    /// Like fake_server, but answers the first request with a 503 and the second normally.
    fn flaky_server(body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for attempt in 0..2 {
                let (mut socket, _) = listener.accept().unwrap();
                let mut buf = vec![0u8; 16384];
                let _ = socket.read(&mut buf).unwrap();
                let response = if attempt == 0 {
                    "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 4\r\nConnection: close\r\n\r\nbusy".to_string()
                } else {
                    format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n{body}")
                };
                socket.write_all(response.as_bytes()).unwrap();
            }
        });
        format!("http://{addr}/v1/")
    }

    #[test]
    fn retries_once_when_the_model_is_busy() {
        let url = flaky_server(
            "data: {\"choices\":[{\"delta\":{\"content\":\"Oi\"}}]}\n\ndata: [DONE]\n\n",
        );
        let result =
            tauri::async_runtime::block_on(translator(&url).translate(&request(), |_| true));
        assert_eq!(result, Ok("Oi".into()));
    }

    #[test]
    fn cancels_when_asked() {
        let (url, _requests) = fake_server(
            "text/event-stream",
            "data: {\"choices\":[{\"delta\":{\"content\":\"Onde\"}}]}\n\n\
             data: {\"choices\":[{\"delta\":{\"content\":\" está\"}}]}\n\n",
        );
        let result =
            tauri::async_runtime::block_on(translator(&url).translate(&request(), |_| false));
        assert_eq!(result, Err(TranslateError::Cancelled));
    }

    #[test]
    fn lists_models_without_the_models_prefix() {
        let (url, _requests) = fake_server(
            "application/json",
            "{\"data\":[{\"id\":\"models/gemini-b-flash\"},{\"id\":\"models/gemini-a-pro\"}]}",
        );
        let models = tauri::async_runtime::block_on(translator(&url).list_models()).unwrap();
        assert_eq!(models, vec!["gemini-a-pro", "gemini-b-flash"]);
    }
}
