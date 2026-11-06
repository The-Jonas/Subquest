//! Glue between OCR and translation: every recognized line becomes a
//! translation streamed to the UI, with the previous lines as context and a
//! cache for repeats. Every line is translated to the end (lines can arrive
//! faster than the API answers, e.g. in songs). A new OCR reading that is just
//! the newest line read slightly differently reuses that line's translation
//! instead of starting another request.
//!
//! Two layers race for every new line:
//!   - the quick translator (Google / DeepL) answers in a few hundred ms and is
//!     shown first, so a line the game shows for a second still gets translated;
//!   - the main translator (the LLM) answers better and replaces it when done.
//!     If its first word is late, a second identical request is sent and the
//!     first one to answer wins: that cuts the slow spikes of busy models.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use futures_util::future::{join, select, Either};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::watch;

use crate::settings::{Settings, TranslatorSettings};
use crate::translate::cache::TranslationCache;
use crate::translate::openai_compat::OpenAiCompatTranslator;
use crate::translate::quick::QuickTranslator;
use crate::translate::similar;
use crate::translate::{ContextLine, TranslateError, TranslationRequest};
use crate::AppState;

const CONTEXT_LINES: usize = 6; // previous lines sent along: enough for tone, cheap on tokens
const CACHE_CAPACITY: usize = 20_000; // a long game's worth of lines; ~2 MB on disk
const HEDGE_AFTER: Duration = Duration::from_millis(1500); // no first word by now: ask again in parallel
const WARM_EVERY: Duration = Duration::from_secs(20); // keeps the HTTPS connections open during a session
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
    cache_path: PathBuf, // where the cache is saved between runs
    pub context: Mutex<VecDeque<ContextLine>>, // reset when a session starts
    recent: Mutex<HashMap<u32, Recent>>,       // per area: newest line with its own translation
}

impl TranslationState {
    /// State with a shared HTTP client and the cache saved by earlier runs.
    pub fn new(data_dir: &std::path::Path) -> Self {
        let cache_path = data_dir.join("translation-cache.json");
        Self {
            http: reqwest::Client::new(),
            cache: Mutex::new(TranslationCache::load(&cache_path, CACHE_CAPACITY)),
            cache_path,
            context: Mutex::new(VecDeque::new()),
            recent: Mutex::new(HashMap::new()),
        }
    }

    /// Saves the cache to disk (only if it changed). Failing is not serious: the
    /// lines are just translated again next time.
    pub fn save_cache(&self) {
        if let Err(e) = self.cache.lock().unwrap().save(&self.cache_path) {
            log::warn(&format!("could not save the translation cache: {e}"));
        }
    }

    /// Number of translations kept, and the cache file's size in bytes.
    pub fn cache_usage(&self) -> (usize, u64) {
        let count = self.cache.lock().unwrap().count();
        let bytes = std::fs::metadata(&self.cache_path).map(|m| m.len()).unwrap_or(0);
        (count, bytes)
    }

    /// Forgets every saved translation, in memory and on disk.
    pub fn clear_cache(&self) {
        self.cache.lock().unwrap().clear();
        self.save_cache();
    }

    /// Forgets the previous game's lines; the cache stays (same lines, same answer).
    pub fn reset_context(&self) {
        self.context.lock().unwrap().clear();
        self.recent.lock().unwrap().clear();
    }

    /// Decides whether a line needs its own translation or is the newest line
    /// of the same area read again with small OCR differences. Runs on the OCR
    /// thread, so it's quick.
    pub fn plan(&self, text: &str, frame_id: u64, area: u32) -> Plan {
        let normalized = similar::normalize(text);
        let mut recent = self.recent.lock().unwrap();
        if let Some(r) = recent.get(&area) {
            if similar::same_line(&r.normalized, &normalized) {
                return Plan::Reuse {
                    of: r.frame_id,
                    result: r.result.clone(),
                };
            }
        }
        let (sender, result) = watch::channel(None);
        recent.insert(
            area,
            Recent {
                normalized,
                frame_id,
                result,
            },
        );
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
    quick: bool,                // text from the quick layer (the main one is late or failed)
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
    let emit = |text: &str, done: bool, cached: bool, quick: bool, error_kind: Option<String>| {
        let _ = app.emit(
            "translation",
            TranslationEvent {
                frame_id,
                text: text.to_string(),
                done,
                cached,
                quick,
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
                emit(&full, true, true, false, None);
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
        emit(&cached, true, true, false, None);
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
    // Quick layer: shown as soon as it answers, unless the LLM already started writing.
    let shown = Mutex::new(Shown::Nothing);
    let quick = QuickTranslator::from_settings(&settings.quick.engine, &settings.quick.deepl_key);
    let quick_line = async {
        let quick = quick.as_ref()?;
        let text = quick
            .translate(&translation.http, &text, &request.source_lang, &request.target_lang)
            .await
            .map_err(|e| log::warn(&format!("quick translation failed: {e}")))
            .ok()?;
        let first = {
            let mut shown = shown.lock().unwrap();
            let first = *shown == Shown::Nothing;
            if first {
                *shown = Shown::Quick;
            }
            first
        };
        if first {
            emit(&text, false, false, true, None);
        }
        Some(text)
    };

    // Main layer: streams, except while the quick text is up (a half-written
    // line replacing a whole one would be a step back; it swaps in when done).
    let on_partial = |partial: &str| {
        let streaming = {
            let mut shown = shown.lock().unwrap();
            if *shown == Shown::Nothing {
                *shown = Shown::Main;
            }
            *shown == Shown::Main
        };
        if streaming {
            emit(partial, false, false, false, None);
        }
    };
    let model = backend(&translation.http, &settings.translator);
    let (quick_text, result) = join(quick_line, hedged(&model, &request, &on_partial)).await;

    match result {
        Ok(full) => {
            translation.cache.lock().unwrap().insert(key, full.clone());
            publish.send_replace(Some(full.clone()));
            emit(&full, true, false, false, None);
            remember(translation, text, full);
        }
        Err(e) => {
            log::warn(&format!("translation failed: {e}"));
            if let Some(quick_text) = quick_text {
                // The quick translation stays as the final one (not cached: the LLM may do better later).
                publish.send_replace(Some(quick_text.clone()));
                emit(&quick_text, true, false, true, None);
                remember(translation, text, quick_text);
            } else {
                // Dropping `publish` here releases any re-readings that were waiting.
                emit("", true, false, false, Some(e.kind().to_string()));
            }
        }
    }
}

/// Which layer's text is on screen for a line.
#[derive(Clone, Copy, PartialEq)]
enum Shown {
    Nothing,
    Quick,
    Main,
}

/// Errors worth a second try in parallel (the request itself was fine).
fn worth_hedging(error: &TranslateError) -> bool {
    matches!(error, TranslateError::Busy(_) | TranslateError::Network(_))
}

/// Translates with the main model, hedging slow answers: if no word arrived
/// after HEDGE_AFTER, the same request is sent again and the first attempt to
/// write a word keeps the line (the other cancels itself). Only one attempt
/// ever reaches `on_partial`.
async fn hedged(
    model: &OpenAiCompatTranslator,
    request: &TranslationRequest,
    on_partial: &(dyn Fn(&str) + Sync),
) -> Result<String, TranslateError> {
    let winner = AtomicU8::new(0); // 0 = nobody wrote yet, else the attempt's id
    let attempt = |id: u8| {
        let winner = &winner;
        async move {
            model
                .translate(request, |partial| {
                    let owner = match winner.compare_exchange(0, id, Ordering::SeqCst, Ordering::SeqCst) {
                        Ok(_) => id,
                        Err(current) => current,
                    };
                    if owner != id {
                        return false; // the other attempt already writes this line
                    }
                    on_partial(partial);
                    true
                })
                .await
        }
    };
    let first = attempt(1);
    let second = async {
        tokio::time::sleep(HEDGE_AFTER).await;
        if winner.load(Ordering::SeqCst) != 0 {
            return Err(TranslateError::Cancelled); // the first one is already writing
        }
        attempt(2).await
    };
    futures_util::pin_mut!(first, second);

    match select(first, second).await {
        Either::Left((Ok(text), _)) | Either::Right((Ok(text), _)) => Ok(text),
        Either::Left((Err(e), second)) => {
            if !worth_hedging(&e) {
                return Err(e); // wrong key, model, settings: asking again won't help
            }
            match second.await {
                Ok(text) => Ok(text),
                Err(TranslateError::Cancelled) => Err(e),
                Err(other) => Err(other),
            }
        }
        Either::Right((Err(e), first)) => match first.await {
            Ok(text) => Ok(text),
            Err(TranslateError::Cancelled) => Err(e),
            Err(other) => Err(other),
        },
    }
}

/// Keeps the connections to the translators open while a session runs: after
/// a quiet minute, the next line would pay a new HTTPS handshake (200-400 ms).
/// Also opens them right away, so the first line of a session doesn't pay it.
pub async fn keep_warm(app: AppHandle, window_handle: u64) {
    loop {
        let state = app.state::<AppState>();
        if *state.session_window.lock().unwrap() != Some(window_handle) {
            return; // the session ended (or another one started)
        }
        let settings: Settings = state.settings.lock().unwrap().clone();
        let http = &state.translation.http;
        let main = backend(http, &settings.translator);
        let quick = QuickTranslator::from_settings(&settings.quick.engine, &settings.quick.deepl_key);
        let quick_ping = async {
            if let Some(QuickTranslator::Google) = quick {
                // DeepL counts characters, so only the free Google endpoint is pinged.
                let _ = QuickTranslator::Google.translate(http, "ok", "en", &settings.target_language).await;
            }
        };
        let _ = join(main.list_models(), quick_ping).await;
        tokio::time::sleep(WARM_EVERY).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// A server whose first connection hangs (a busy model) and whose second
    /// answers at once, each in its own thread.
    fn slow_then_fast_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for attempt in 0..2 {
                let (mut socket, _) = listener.accept().unwrap();
                std::thread::spawn(move || {
                    let mut buf = vec![0u8; 16384];
                    let _ = socket.read(&mut buf);
                    if attempt == 0 {
                        std::thread::sleep(Duration::from_secs(6));
                    }
                    let body = format!(
                        "data: {{\"choices\":[{{\"delta\":{{\"content\":\"resposta {attempt}\"}}}}]}}\n\ndata: [DONE]\n\n"
                    );
                    let head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n";
                    let _ = socket.write_all(format!("{head}{body}").as_bytes());
                });
            }
        });
        format!("http://{addr}/v1/")
    }

    #[test]
    fn a_hung_request_is_overtaken_by_the_second_one() {
        let model = OpenAiCompatTranslator {
            client: reqwest::Client::new(),
            base_url: slow_then_fast_server(),
            api_key: String::new(),
            model: "m".into(),
        };
        let request = TranslationRequest {
            text: "hi".into(),
            source_lang: "en".into(),
            target_lang: "pt-BR".into(),
            context: Vec::new(),
            glossary: Vec::new(),
        };
        let seen = Mutex::new(Vec::new());
        let on_partial = |p: &str| seen.lock().unwrap().push(p.to_string());
        let started = Instant::now();
        let text = tauri::async_runtime::block_on(hedged(&model, &request, &on_partial)).unwrap();
        assert_eq!(text, "resposta 1");
        assert!(started.elapsed() < Duration::from_secs(4), "should not wait for the hung one");
        assert_eq!(*seen.lock().unwrap(), vec!["resposta 1"]); // only the winner wrote
    }
}
