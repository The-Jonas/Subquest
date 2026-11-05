//! Subquest — Rust side of the app: state, commands called by the UI and the
//! bridge to the C++ core.

mod core_ffi;
mod error;
mod fonts;
mod overlay;
mod settings;
mod translate;
mod translation;

use std::ffi::{c_char, c_void, CStr};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::Serialize;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, Emitter, Manager, State};

use core_ffi::{Core, SqRect, WindowEntry};
use error::AppError;
use settings::{Settings, TranslatorSettings};
use translation::TranslationState;

/// Channel the preview reaches the UI through, as raw bytes (no JSON).
type PreviewChannel = Channel<InvokeResponseBody>;

/// State shared across commands.
struct AppState {
    core: Result<Core, String>, // Err = DLL missing or incompatible (the app opens anyway)
    settings: Mutex<Settings>,
    config_dir: PathBuf,
    data_dir: PathBuf, // app data folder (imported fonts live here)
    session_user_data: Mutex<Option<*mut AppHandle>>, // AppHandle lent to the core during a session
    preview_user_data: Mutex<Option<*mut PreviewChannel>>, // channel lent to the core during the preview
    translation: TranslationState,
    overlay_running: Mutex<Option<Arc<AtomicBool>>>, // stops the loop that follows the game window
    active_profile: Mutex<Option<String>>, // profile key of the game being translated (the overlay styles itself from it)
}

// SAFETY: the pointers are only created/freed while the Mutex is held, and
// AppHandle and Channel are Send + Sync.
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

/// Core status, so the UI can show a clear notice when the DLL is missing.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CoreStatus {
    loaded: bool,
    api_version: Option<u32>,
    error: Option<String>,
}

/// Text recognized by OCR, sent to the UI (and the overlay) as an event.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OcrTextEvent {
    text: String,
    frame_id: u64,
    ocr_ms: f64,        // recognition only
    total_ms: f64,      // from frame capture until the text was ready
    box_color: String,  // dialogue box color sampled from the game, "#rrggbb"
    text_color: String, // text color sampled from the game, "#rrggbb"
    frame_width: i32,   // captured area size: line boxes are in these pixels
    frame_height: i32,
    lines: Vec<OcrLineEvent>,
}

/// One recognized line with its place and colors, for drawing its translation in place.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OcrLineEvent {
    text: String,
    rect: [i32; 4], // [x, y, width, height] inside the captured area
    box_color: String,
    text_color: String,
}

/// Formats 0xRRGGBB as a CSS color.
fn css_color(rgb: u32) -> String {
    format!("#{:06x}", rgb & 0xFF_FFFF)
}

/// Copies a C string into an owned String (lossy on invalid UTF-8).
unsafe fn owned(text: *const c_char) -> String {
    CStr::from_ptr(text).to_string_lossy().into_owned()
}

/// Returns the loaded core or the `coreMissing` error.
fn core(state: &AppState) -> Result<&Core, AppError> {
    state
        .core
        .as_ref()
        .map_err(|e| AppError::core_missing(e.clone()))
}

/// Reports whether the core DLL was loaded.
#[tauri::command]
fn core_status(state: State<'_, AppState>) -> CoreStatus {
    match &state.core {
        Ok(c) => CoreStatus {
            loaded: true,
            api_version: Some(c.api_version()),
            error: None,
        },
        Err(e) => CoreStatus {
            loaded: false,
            api_version: None,
            error: Some(e.clone()),
        },
    }
}

/// Lists open windows so the user can pick the game.
#[tauri::command]
fn list_windows(state: State<'_, AppState>) -> Result<Vec<WindowEntry>, AppError> {
    core(&state)?.list_windows()
}

/// Callback called by the core (OCR thread): forwards the text to the UI and
/// hands it to the translator on the async runtime, so the OCR thread never waits
/// on the network. The overlay window listens to the same events.
unsafe extern "C" fn on_ocr_text(event: *const core_ffi::SqOcrEvent, user: *mut c_void) {
    let app = &*(user as *const AppHandle);
    let e = &*event;
    let raw_lines = if e.lines.is_null() || e.line_count == 0 {
        &[][..]
    } else {
        std::slice::from_raw_parts(e.lines, e.line_count as usize)
    };
    let lines = raw_lines
        .iter()
        .map(|l| OcrLineEvent {
            text: owned(l.utf8_text),
            rect: [l.rect.x, l.rect.y, l.rect.width, l.rect.height],
            box_color: css_color(l.background_rgb),
            text_color: css_color(l.text_rgb),
        })
        .collect();
    let text = owned(e.utf8_text);
    let _ = app.emit(
        "ocr-text",
        OcrTextEvent {
            text: text.clone(),
            frame_id: e.frame_id,
            ocr_ms: e.ocr_ms,
            total_ms: e.total_ms,
            box_color: css_color(e.background_rgb),
            text_color: css_color(e.text_rgb),
            frame_width: e.frame_width,
            frame_height: e.frame_height,
            lines,
        },
    );
    let app = app.clone();
    tauri::async_runtime::spawn(translation::translate_line(app, text, e.frame_id));
}

/// Starts translating the chosen window, in the chosen region.
#[tauri::command]
async fn start_session(
    app: AppHandle,
    state: State<'_, AppState>,
    window_handle: u64,
    region: Option<(i32, i32, i32, i32)>,
    profile_key: String,
) -> Result<(), AppError> {
    let core = core(&state)?;
    *state.active_profile.lock().unwrap() = Some(profile_key.clone());
    let _ = app.emit("session-started", profile_key); // the overlay loads this game's look
    let language = state.settings.lock().unwrap().source_language.clone();
    state.translation.reset_context();
    let (x, y, width, height) = region.unwrap_or_default();

    let area = SqRect {
        x,
        y,
        width,
        height,
    };

    let mut slot = state.session_user_data.lock().unwrap();
    let user = Box::into_raw(Box::new(app.clone()));
    let result = core.start(
        window_handle,
        area,
        &language,
        on_ocr_text,
        user as *mut c_void,
    );
    match result {
        Ok(()) => *slot = Some(user),
        // SAFETY: the core refused the session, so it didn't keep the pointer.
        Err(_) => drop(unsafe { Box::from_raw(user) }),
    }
    result?;
    drop(slot);

    // In-place translations need the overlay over the whole captured area: the
    // marked area, or the whole window when reading the whole screen.
    let in_place = state
        .active_profile
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|key| state.settings.lock().unwrap().profiles.get(key).cloned())
        .and_then(|p| p.areas.first().map(|a| a.style != "classic"))
        .unwrap_or(true);

    // The session works even if the overlay fails; the translations still show in the app.
    if let Err(e) = overlay::open(&app, window_handle, region.map(|_| area), in_place) {
        eprintln!("[subquest] overlay: {}", e.detail);
    }
    Ok(())
}

/// Stops the session and takes back the AppHandle lent to the core.
#[tauri::command]
async fn stop_session(app: AppHandle, state: State<'_, AppState>) -> Result<(), AppError> {
    overlay::close(&app);
    let result = core(&state)?.stop();
    if let Some(user) = state.session_user_data.lock().unwrap().take() {
        // SAFETY: sq_stop already waited for the OCR thread; nobody uses the pointer anymore.
        drop(unsafe { Box::from_raw(user) });
    }
    result
}

/// Preview callback (capture thread): packs [width, height, source width,
/// source height] as little-endian u32 + the RGBA pixels and sends them through
/// the channel. JS reads this straight into an ImageData, no JSON involved.
unsafe extern "C" fn on_preview_frame(
    rgba: *const u8,
    width: i32,
    height: i32,
    source_width: i32,
    source_height: i32,
    user: *mut c_void,
) {
    let channel = &*(user as *const PreviewChannel);
    let len = width as usize * height as usize * 4;
    let mut payload = Vec::with_capacity(16 + len);
    for value in [width, height, source_width, source_height] {
        payload.extend_from_slice(&(value as u32).to_le_bytes());
    }
    payload.extend_from_slice(std::slice::from_raw_parts(rgba, len));
    let _ = channel.send(InvokeResponseBody::Raw(payload));
}

/// Stops the preview (if any) and frees the channel lent to the core.
fn release_preview(core: &Core, slot: &mut Option<*mut PreviewChannel>) {
    if let Some(user) = slot.take() {
        let _ = core.preview_stop();
        // SAFETY: sq_preview_stop waited for capture to end; nobody uses the pointer anymore.
        drop(unsafe { Box::from_raw(user) });
    }
}

// Session and preview commands are `async` on purpose: that way they run on a
// runtime thread instead of the main (window) thread. Stopping capture waits
// for threads to finish, and that must never freeze the UI.

/// Starts sending the live window preview through the given channel.
#[tauri::command]
async fn start_preview(
    state: State<'_, AppState>,
    window_handle: u64,
    max_width: i32,
    channel: PreviewChannel,
) -> Result<(), AppError> {
    let core = core(&state)?;
    let mut slot = state.preview_user_data.lock().unwrap();
    release_preview(core, &mut slot); // only one preview at a time

    let user = Box::into_raw(Box::new(channel));
    match core.preview_start(
        window_handle,
        max_width,
        on_preview_frame,
        user as *mut c_void,
    ) {
        Ok(()) => {
            *slot = Some(user);
            Ok(())
        }
        Err(e) => {
            // SAFETY: the core refused, so it didn't keep the pointer.
            drop(unsafe { Box::from_raw(user) });
            Err(e)
        }
    }
}

/// Stops the live preview.
#[tauri::command]
async fn stop_preview(state: State<'_, AppState>) -> Result<(), AppError> {
    let core = core(&state)?;
    release_preview(core, &mut state.preview_user_data.lock().unwrap());
    Ok(())
}

/// Result of the "Test" button in Settings.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TranslatorTest {
    text: String,
    elapsed_ms: f64,
}

/// Translates a fixed sample line with the given (possibly unsaved) settings,
/// so the user can check the key, model and latency before playing.
#[tauri::command]
async fn test_translator(
    state: State<'_, AppState>,
    translator: TranslatorSettings,
    source_language: String,
    target_language: String,
) -> Result<TranslatorTest, AppError> {
    let request = translate::TranslationRequest {
        text: "The bridge is out. You'll have to find another way across.".into(),
        source_lang: source_language,
        target_lang: target_language,
        context: Vec::new(),
        glossary: Vec::new(),
    };
    let started = Instant::now();
    let text = translation::backend(&state.translation.http, &translator)
        .translate(&request, |_| true)
        .await
        .map_err(AppError::from_translate)?;
    Ok(TranslatorTest {
        text,
        elapsed_ms: started.elapsed().as_secs_f64() * 1000.0,
    })
}

/// Lists the models the configured service offers.
#[tauri::command]
async fn list_models(
    state: State<'_, AppState>,
    translator: TranslatorSettings,
) -> Result<Vec<String>, AppError> {
    translation::backend(&state.translation.http, &translator)
        .list_models()
        .await
        .map_err(AppError::from_translate)
}

/// Imports a font sent as raw bytes; the original file name comes in the
/// `x-file-name` header (URL-encoded by the UI, since headers are ASCII-only).
#[tauri::command]
fn import_font(
    state: State<'_, AppState>,
    request: tauri::ipc::Request<'_>,
) -> Result<fonts::FontEntry, AppError> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(AppError::other("expected the font file as raw bytes"));
    };
    let name = request
        .headers()
        .get("x-file-name")
        .and_then(|v| v.to_str().ok())
        .map(percent_decode)
        .ok_or_else(|| AppError::other("missing file name"))?;
    fonts::import(&state.data_dir, &name, bytes)
}

/// Decodes "%20"-style escapes (enough for file names sent by encodeURIComponent).
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&text[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Lists the imported fonts.
#[tauri::command]
fn list_fonts(state: State<'_, AppState>) -> Vec<fonts::FontEntry> {
    fonts::list(&state.data_dir)
}

/// Sends an imported font's bytes to the UI (raw, no JSON).
#[tauri::command]
fn read_font(state: State<'_, AppState>, file: String) -> Result<tauri::ipc::Response, AppError> {
    fonts::read(&state.data_dir, &file).map(tauri::ipc::Response::new)
}

/// Deletes an imported font.
#[tauri::command]
fn remove_font(state: State<'_, AppState>, file: String) -> Result<(), AppError> {
    fonts::remove(&state.data_dir, &file)
}

/// Profile key of the game being translated, so the overlay can apply its font and style.
#[tauri::command]
fn active_profile(state: State<'_, AppState>) -> Option<String> {
    state.active_profile.lock().unwrap().clone()
}

/// Returns the current settings.
#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

/// Replaces and saves the settings.
#[tauri::command]
fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<(), AppError> {
    settings::save(&state.config_dir, &settings).map_err(AppError::other)?;
    *state.settings.lock().unwrap() = settings.clone();
    let _ = app.emit("settings-changed", settings); // the overlay restyles itself live
    Ok(())
}

/// Builds and runs the app: loads the core and settings and registers the commands.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let data_dir = app.path().app_data_dir()?;
            let resource_dir = app.path().resource_dir().ok();

            let mut errors = Vec::new();
            let mut loaded = None;
            for path in core_ffi::candidate_paths(resource_dir) {
                match Core::load(&path) {
                    Ok(c) => {
                        loaded = Some(c);
                        break;
                    }
                    Err(e) => errors.push(e),
                }
            }
            let core = loaded.ok_or_else(|| errors.join("\n"));

            app.manage(AppState {
                core,
                settings: Mutex::new(settings::load(&config_dir)),
                config_dir,
                data_dir,
                session_user_data: Mutex::new(None),
                preview_user_data: Mutex::new(None),
                translation: TranslationState::new(),
                overlay_running: Mutex::new(None),
                active_profile: Mutex::new(None),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            core_status,
            list_windows,
            start_session,
            stop_session,
            start_preview,
            stop_preview,
            test_translator,
            list_models,
            active_profile,
            import_font,
            list_fonts,
            read_font,
            remove_font,
            get_settings,
            save_settings
        ])
        .run(tauri::generate_context!())
        .expect("failed to start Subquest");
}
