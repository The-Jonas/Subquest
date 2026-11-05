//! Glue between OCR and translation: every recognized line becomes a
//! translation streamed to the UI, with the previous lines as context and a
//! cache for repeats. Every line is translated to the end (lines can arrive
//! faster than the API answers, e.g. in songs); `latest_line` tells which one
//! is current, so the overlay can show only the newest.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::settings::{Settings, TranslatorSettings};
use crate::translate::cache::TranslationCache;
use crate::translate::openai_compat::OpenAiCompatTranslator;
use crate::translate::{ContextLine, TranslateError, TranslationRequest};
use crate::AppState;

const CONTEXT_LINES: usize = 6; // previous lines sent along: enough for tone, cheap on tokens
const CACHE_CAPACITY: usize = 2000;

/// Translation state that lives as long as the app.
pub struct TranslationState {
    pub http: reqwest::Client, // one client, so connections get reused (lower latency)
    pub cache: Mutex<TranslationCache>,
    pub context: Mutex<VecDeque<ContextLine>>, // reset when a session starts
    pub latest_line: AtomicU64,                // frame id of the newest OCR line (for the overlay)
}

impl TranslationState {
    /// Empty state with a shared HTTP client.
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            cache: Mutex::new(TranslationCache::new(CACHE_CAPACITY)),
            context: Mutex::new(VecDeque::new()),
            latest_line: AtomicU64::new(0),
        }
    }

    /// Forgets the previous game's lines; the cache stays (same lines, same answer).
    pub fn reset_context(&self) {
        self.context.lock().unwrap().clear();
        self.latest_line.store(0, Ordering::SeqCst);
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
pub async fn translate_line(app: AppHandle, text: String, frame_id: u64) {
    let state = app.state::<AppState>();
    let translation = &state.translation;
    translation.latest_line.store(frame_id, Ordering::SeqCst);
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
            emit(&full, true, false, None);
            remember(translation, text, full);
        }
        Err(TranslateError::Cancelled) => {} // not used by this flow (we never cancel)
        Err(e) => {
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
