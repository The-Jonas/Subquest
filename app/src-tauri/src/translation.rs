//! Glue between OCR and translation: every recognized line becomes a
//! translation streamed to the UI, with the previous lines as context and a
//! cache for repeats. Every line is translated to the end (lines can arrive
//! faster than the API answers, e.g. in songs). A new OCR reading that is just
//! the newest line read slightly differently reuses that line's translation
//! instead of starting another request.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::watch;

use crate::settings::{Settings, TranslatorSettings};
use crate::translate::cache::TranslationCache;
use crate::translate::openai_compat::OpenAiCompatTranslator;
use crate::translate::similar;
use crate::translate::{ContextLine, TranslateError, TranslationRequest};
use crate::AppState;

const CONTEXT_LINES: usize = 6; // previous lines sent along: enough for tone, cheap on tokens
const CACHE_CAPACITY: usize = 2000;
const REUSE_WAIT: Duration = Duration::from_secs(20); // a re-reading waits this long for the original

/// Final translation of a line, shared with its re-readings (None while it streams).
type Shared = watch::Receiver<Option<String>>;

/// The newest line that got its own translation.
struct Recent {
    normalized: String,
    frame_id: u64,
    result: Shared,
}

/// What to do with a line that just arrived from OCR.
pub enum Plan {
    /// A new line: translate it and publish the result here.
    New(watch::Sender<Option<String>>),
    /// The newest line read again: wait for its translation (`of` is its frame id).
    Reuse { of: u64, result: Shared },
}

/// Translation state that lives as long as the app.
pub struct TranslationState {
    pub http: reqwest::Client, // one client, so connections get reused (lower latency)
    pub cache: Mutex<TranslationCache>,
    pub context: Mutex<VecDeque<ContextLine>>, // reset when a session starts
    recent: Mutex<Option<Recent>>,             // newest line with its own translation
}

impl TranslationState {
    /// Empty state with a shared HTTP client.
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            cache: Mutex::new(TranslationCache::new(CACHE_CAPACITY)),
            context: Mutex::new(VecDeque::new()),
            recent: Mutex::new(None),
        }
    }

    /// Forgets the previous game's lines; the cache stays (same lines, same answer).
    pub fn reset_context(&self) {
        self.context.lock().unwrap().clear();
        *self.recent.lock().unwrap() = None;
    }

    /// Decides whether a line needs its own translation or is the newest line
    /// read again with small OCR differences. Runs on the OCR thread, so it's quick.
    pub fn plan(&self, text: &str, frame_id: u64) -> Plan {
        let normalized = similar::normalize(text);
        let mut recent = self.recent.lock().unwrap();
        if let Some(r) = recent.as_ref() {
            if similar::same_line(&r.normalized, &normalized) {
                return Plan::Reuse {
                    of: r.frame_id,
                    result: r.result.clone(),
                };
            }
        }
        let (sender, result) = watch::channel(None);
        *recent = Some(Recent {
            normalized,
            frame_id,
            result,
        });
        Plan::New(sender)
    }
}

/// Builds the backend from the user's translator settings.
pub fn backend(http: &reqwest::Client, settings: &TranslatorSettings) -> OpenAiCompatTranslator {
    OpenAiCompatTranslator {
        client: http.clone(),
        base_url: settings.base_url.clone(),
        api_key: settings.api_key.clone(),
        model: settings.model.clone(),
    }
}

/// Event sent to the UI while a line is being translated.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TranslationEvent {
    frame_id: u64,
    text: String, // translation so far (or final)
    done: bool,
    cached: bool,
    elapsed_ms: f64,            // since the OCR text arrived
    error_kind: Option<String>, // set when the translation failed
}

/// Translates one OCR line and streams the result to the UI. Runs on the async
/// runtime, never on the OCR thread, so OCR keeps its pace while the network works.
pub async fn translate_line(app: AppHandle, text: String, frame_id: u64, plan: Plan) {
    let state = app.state::<AppState>();
    let translation = &state.translation;
    let started = Instant::now();
    let settings: Settings = state.settings.lock().unwrap().clone();

    // Emits an event for this line, filling in the fields every event shares.
    let emit = |text: &str, done: bool, cached: bool, error_kind: Option<String>| {
        let _ = app.emit(
            "translation",
            TranslationEvent {
                frame_id,
                text: text.to_string(),
                done,
                cached,
                elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
                error_kind,
            },
        );
    };

    let publish = match plan {
        Plan::New(sender) => sender,
        Plan::Reuse { mut result, .. } => {
            // Same line read again: no request, just its translation once it's ready.
            let ready = tokio::time::timeout(REUSE_WAIT, result.wait_for(Option::is_some)).await;
            if let Ok(Ok(full)) = ready {
                let full = full.clone().unwrap_or_default();
                emit(&full, true, true, None);
            }
            return;
        }
    };

    let key = TranslationCache::key(
        &settings.translator.model,
        &settings.source_language,
        &settings.target_language,
        &text,
    );
    let hit = translation
        .cache
        .lock()
        .unwrap()
        .get(&key)
        .map(str::to_owned);
    if let Some(cached) = hit {
        publish.send_replace(Some(cached.clone()));
        emit(&cached, true, true, None);
        remember(translation, text, cached);
        return;
    }

    let request = TranslationRequest {
        text: text.clone(),
        source_lang: settings.source_language.clone(),
        target_lang: settings.target_language.clone(),
        context: translation
            .context
            .lock()
            .unwrap()
            .iter()
            .cloned()
            .collect(),
        glossary: Vec::new(), // TODO(phase 3): per-game glossary from the profile
    };
    let result = backend(&translation.http, &settings.translator)
        .translate(&request, |partial| {
            emit(partial, false, false, None);
            true // never cancel: older lines still belong in the history
        })
        .await;

    match result {
        Ok(full) => {
            translation.cache.lock().unwrap().insert(key, full.clone());
            publish.send_replace(Some(full.clone()));
            emit(&full, true, false, None);
            remember(translation, text, full);
        }
        Err(TranslateError::Cancelled) => {} // not used by this flow (we never cancel)
        Err(e) => {
            // Dropping `publish` here releases any re-readings that were waiting.
            log::warn(&format!("translation failed: {e}"));
            emit("", true, false, Some(e.kind().to_string()));
        }
    }
}

/// Appends a finished line to the context, keeping only the latest few.
fn remember(translation: &TranslationState, source: String, translated: String) {
    let mut context = translation.context.lock().unwrap();
    context.push_back(ContextLine {
        source,
        translation: translated,
    });
    while context.len() > CONTEXT_LINES {
        context.pop_front();
    }
}

/// Tiny logging helper: stderr is enough until we need a real logger.
mod log {
    /// Prints a warning to stderr (visible in the `tauri dev` terminal).
    pub fn warn(message: &str) {
        eprintln!("[subquest] {message}");
    }
}
