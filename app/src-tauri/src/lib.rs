//! Subquest — Rust side of the app: state, commands called by the UI and the
//! bridge to the C++ core.

mod core_ffi;
mod error;
mod font_detect;
mod fonts;
mod ingame;
mod overlay;
mod settings;
mod shape;
mod styles;
mod translate;
mod translation;

use std::ffi::{c_char, c_void, CStr};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::Serialize;
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::{AppHandle, Emitter, Manager, State};

use core_ffi::{Core, SqRect, WindowEntry};
use error::AppError;
use settings::{Settings, TranslatorSettings};
use translate::similar;
use translation::TranslationState;

const MIN_WIDTH: f64 = 960.0; // same as minWidth in tauri.conf.json
const MIN_HEIGHT: f64 = 640.0; // same as minHeight in tauri.conf.json
const MAX_LINE_SHARE: f32 = 0.1; // an OCR line taller than this share of the window is artwork, not text
const THUMBNAIL_WIDTH: i32 = 360; // game list pictures: enough to recognize a game, cheap to send

/// Channel the preview reaches the UI through, as raw bytes (no JSON).
type PreviewChannel = Channel<InvokeResponseBody>;

/// State shared across commands.
struct AppState {
    core: Result<Core, String>, // Err = DLL missing or incompatible (the app opens anyway)
    settings: Mutex<Settings>,
    config_dir: PathBuf,
    data_dir: PathBuf, // app data folder (imported fonts live here)
    session_user_data: Mutex<Option<*mut AppHandle>>, // AppHandle lent to the core during a session
    session_window: Mutex<Option<u64>>,                 // game being translated (None = no session)
    preview_user_data: Mutex<Option<*mut PreviewChannel>>, // channel lent to the core during the preview
    translation: TranslationState,
    overlay_running: Mutex<Option<Arc<AtomicBool>>>, // stops the loop that follows the game window
    active_profile: Mutex<Option<String>>, // profile key of the game being translated (the overlay styles itself from it)
    font_finder: font_detect::FontFinder,   // fonts suggested for a game, waiting for the user's choice
    styles: styles::StyleStore,             // text styles taken by the user, with their prints
    paused: AtomicBool,                     // session paused from the toolbar (core stopped)
    overlay_hidden: AtomicBool,             // translations hidden from the toolbar
    toolbar_wanted: AtomicBool,             // toolbar asked for (Ctrl+Shift+B); shown only over the game
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
    area: u32,            // index of the profile area this text came from
    same_as: Option<u64>, // this is that earlier line read again: keep its translation
}

/// One recognized line with its place and colors, for drawing its translation in place.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OcrLineEvent {
    text: String,
    rect: [i32; 4], // [x, y, width, height] in window pixels
    box_color: String,
    text_color: String,
    style: Option<String>, // id of the game's text style this line belongs to
}

/// Tells the UI the styles changed, and restyles the lines on screen now
/// ("line-styles": event id + style per line, for each area), so a new font
/// shows at once even on a menu that is never read again.
pub(crate) fn styles_changed(app: &AppHandle) {
    let _ = app.emit("styles-changed", ());
    let _ = app.emit("line-styles", app.state::<AppState>().styles.rematch());
}

/// Shape of a line's letters, measured on the event's area pixels (BGRA) with
/// the line's own letter and background colors. None when the core sent no
/// picture or the line is too small to measure.
unsafe fn line_shape(e: &core_ffi::SqOcrEvent, line: &core_ffi::SqLine) -> Option<shape::Shape> {
    if e.image_bgra.is_null() || e.image_stride <= 0 {
        return None;
    }
    let image = std::slice::from_raw_parts(e.image_bgra, (e.image_stride * e.image_height) as usize);
    // Line box relative to the area, clipped to it.
    let x0 = (line.rect.x - e.image_x).clamp(0, e.image_width) as usize;
    let y0 = (line.rect.y - e.image_y).clamp(0, e.image_height) as usize;
    let x1 = (line.rect.x - e.image_x + line.rect.width).clamp(0, e.image_width) as usize;
    let y1 = (line.rect.y - e.image_y + line.rect.height).clamp(0, e.image_height) as usize;
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let stride = e.image_stride as usize;
    let pixel = |x: usize, y: usize| {
        let i = (y0 + y) * stride + (x0 + x) * 4;
        u32::from(image[i + 2]) << 16 | u32::from(image[i + 1]) << 8 | u32::from(image[i])
    };
    shape::Shape::measure(x1 - x0, y1 - y0, pixel, line.text_rgb, line.background_rgb)
}

/// Formats 0xRRGGBB as a CSS color.
fn css_color(rgb: u32) -> String {
    format!("#{:06x}", rgb & 0xFF_FFFF)
}

/// Copies a C string into an owned String (lossy on invalid UTF-8).
unsafe fn owned(text: *const c_char) -> String {
    CStr::from_ptr(text).to_string_lossy().into_owned()
}

/// Tray icon (next to the clock): click it to bring Subquest back; its menu
/// opens Subquest or quits for good. Labels follow the UI language.
fn tray(app: &AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    let language = app.state::<AppState>().settings.lock().unwrap().ui_language.clone();
    let (open_label, quit_label) = match language.as_str() {
        "en" => ("Open Subquest", "Quit"),
        "es" => ("Abrir Subquest", "Salir"),
        _ => ("Abrir o Subquest", "Sair"),
    };
    let open = MenuItem::with_id(app, "open", open_label, true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", quit_label, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit_item])?;
    let mut builder = TrayIconBuilder::with_id("subquest")
        .tooltip("Subquest")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app.clone(), None),
            "quit" => quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main(tray.app_handle().clone(), None);
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}

/// Quits for good: ends the session (capture, overlay, shortcuts) and closes
/// every window, the hidden ones included.
fn quit(app: &AppHandle) {
    let _ = end_session(app);
    app.exit(0);
}

/// Opens the main window big enough for the screen it is on: 80% of it,
/// at most 1440 × 920 and never below the minimum size in tauri.conf.json.
/// A fixed size is either cramped on big screens or too tall for laptops.
fn fit_main_window(app: &AppHandle) {
    let Some(main) = app.get_webview_window("main") else { return };
    let Ok(Some(monitor)) = main.current_monitor() else { return };
    let scale = monitor.scale_factor();
    let screen = monitor.size().to_logical::<f64>(scale);
    let width = (screen.width * 0.8).clamp(MIN_WIDTH, 1440.0);
    let height = (screen.height * 0.8).clamp(MIN_HEIGHT, 920.0);
    let _ = main.set_size(tauri::LogicalSize::new(width, height));
    let _ = main.center();
}

/// Appends a line from one of the UI pages (errors, and a few diagnostics) to
/// `<app data>/ui.log`. The installed app has no developer console, so this is
/// where its JavaScript problems become visible. Kept small: past 512 KB the
/// file starts over.
#[tauri::command]
fn ui_log(state: State<'_, AppState>, source: String, message: String) {
    use std::io::Write;
    const MAX_BYTES: u64 = 512 * 1024;
    let path = state.data_dir.join("ui.log");
    let _ = std::fs::create_dir_all(&state.data_dir);
    let too_big = std::fs::metadata(&path).map(|m| m.len() > MAX_BYTES).unwrap_or(false);
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(!too_big)
        .truncate(too_big)
        .open(&path);
    if let Ok(mut file) = file {
        let when = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let _ = writeln!(file, "[{when}] {source}: {message}");
    }
}

/// Writes every panic (with where it happened) to `<app data>/crash.log`, so a
/// crash during a test leaves a trace. A panic inside a core callback aborts
/// the whole app (it can't unwind into C++), so this is often the only clue.
fn log_panics(data_dir: &std::path::Path) {
    let path = data_dir.join("crash.log");
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        use std::io::Write;
        let when = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let thread = std::thread::current().name().unwrap_or("unnamed").to_string();
        let trace = std::backtrace::Backtrace::force_capture();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(file, "[unix {when}] panic on thread '{thread}': {info}\n{trace}\n");
        }
        previous(info);
    }));
}

/// Returns the loaded core or the `coreMissing` error.
pub(crate) fn core(state: &AppState) -> Result<&Core, AppError> {
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

/// Callback called by the core (OCR thread). A panic must not cross into C++
/// (that aborts the app): it is caught here, logged (see log_panics), and only
/// this one reading is lost.
unsafe extern "C" fn on_ocr_text(event: *const core_ffi::SqOcrEvent, user: *mut c_void) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handle_ocr_text(event, user)));
}

/// Forwards one OCR reading to the UI and hands it to the translator on the
/// async runtime, so the OCR thread never waits on the network. The overlay
/// window listens to the same events.
unsafe fn handle_ocr_text(event: *const core_ffi::SqOcrEvent, user: *mut c_void) {
    let app = &*(user as *const AppHandle);
    let e = &*event;
    let all_lines = if e.lines.is_null() || e.line_count == 0 {
        &[][..]
    } else {
        std::slice::from_raw_parts(e.lines, e.line_count as usize)
    };
    // Huge "lines" are artwork read as letters (a logo, big numbers on a
    // menu): translated, they came back as giant subtitles over the screen.
    let tallest = e.frame_height as f32 * MAX_LINE_SHARE;
    let kept: Vec<core_ffi::SqLine> = all_lines
        .iter()
        .filter(|l| e.frame_height <= 0 || (l.rect.height as f32) <= tallest)
        .copied()
        .collect();
    let raw_lines = &kept[..];
    let mut text = if kept.len() == all_lines.len() {
        owned(e.utf8_text)
    } else {
        raw_lines.iter().map(|l| owned(l.utf8_text)).collect::<Vec<_>>().join("\n")
    };
    let meaningful = similar::meaningful(&text);

    // Text styles: each line takes the user's style its letters look like
    // (shape measured on the area's pixels, only valid during this callback).
    let found: Vec<(u32, [i32; 4], String)> = raw_lines
        .iter()
        .map(|l| (l.text_rgb, [l.rect.x, l.rect.y, l.rect.width, l.rect.height], owned(l.utf8_text)))
        .collect();
    let samples: Vec<(u32, Option<shape::Shape>)> = raw_lines
        .iter()
        .map(|l| (l.text_rgb, if meaningful { line_shape(e, l) } else { None }))
        .collect();
    let style_ids = app.state::<AppState>().styles.match_lines(e.area_index, e.frame_id, samples);
    let mut lines: Vec<OcrLineEvent> = found
        .into_iter()
        .zip(raw_lines)
        .enumerate()
        .map(|(i, ((_, rect, text), l))| OcrLineEvent {
            text,
            rect,
            box_color: css_color(l.background_rgb),
            text_color: css_color(l.text_rgb),
            style: style_ids.get(i).cloned().flatten(),
        })
        .collect();
    if !meaningful {
        // The text left the screen (or only stray marks are left): tell the UI, translate nothing.
        text.clear();
        lines = Vec::new();
    }
    let plan = meaningful.then(|| app.state::<AppState>().translation.plan(&text, e.frame_id, e.area_index));
    let same_as = match &plan {
        Some(translation::Plan::Reuse { of, .. }) => Some(*of),
        _ => None,
    };
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
            area: e.area_index,
            same_as,
        },
    );
    if let Some(plan) = plan {
        let app = app.clone();
        tauri::async_runtime::spawn(translation::translate_line(app, text, e.frame_id, plan));
    }
}

/// Screen areas of a profile as the core wants them: a marked rectangle, or an
/// empty one for "whole window". No profile (or no areas) = the whole window.
fn session_areas(profile: Option<&settings::GameProfile>) -> Vec<SqRect> {
    let mut areas: Vec<SqRect> = profile
        .map(|p| p.areas.as_slice())
        .unwrap_or_default()
        .iter()
        .map(|a| match a.region {
            Some([x, y, width, height]) if a.mode != "screen" => SqRect {
                x,
                y,
                width,
                height,
            },
            _ => SqRect::default(),
        })
        .take(core_ffi::MAX_AREAS)
        .collect();
    if areas.is_empty() {
        areas.push(SqRect::default());
    }
    areas
}

/// Starts the core reading the profile's areas of a window. Only the core:
/// the overlay and the translation context are left as they are.
fn start_core(app: &AppHandle, state: &AppState, window_handle: u64, profile_key: &str) -> Result<(), AppError> {
    let core = core(state)?;
    let (language, areas) = {
        let settings = state.settings.lock().unwrap();
        let areas = session_areas(settings.profiles.get(profile_key));
        (settings.source_language.clone(), areas)
    };
    let mut slot = state.session_user_data.lock().unwrap();
    let user = Box::into_raw(Box::new(app.clone()));
    let result = core.start(
        window_handle,
        &areas,
        &language,
        on_ocr_text,
        user as *mut c_void,
    );
    match result {
        Ok(()) => *slot = Some(user),
        // SAFETY: the core refused the session, so it didn't keep the pointer.
        Err(_) => drop(unsafe { Box::from_raw(user) }),
    }
    result
}

/// Stops the core and takes back the AppHandle lent to it.
pub(crate) fn stop_core(state: &AppState) -> Result<(), AppError> {
    let result = core(state)?.stop();
    if let Some(user) = state.session_user_data.lock().unwrap().take() {
        // SAFETY: sq_stop already waited for the OCR thread; nobody uses the pointer anymore.
        drop(unsafe { Box::from_raw(user) });
    }
    result
}

/// Starts translating the chosen window, reading every area of its profile.
#[tauri::command]
async fn start_session(
    app: AppHandle,
    state: State<'_, AppState>,
    window_handle: u64,
    profile_key: String,
) -> Result<(), AppError> {
    *state.active_profile.lock().unwrap() = Some(profile_key.clone());
    let _ = app.emit("session-started", profile_key.clone()); // the overlay loads this game's look
    state.translation.reset_context();
    state.styles.open(&profile_key);
    start_core(&app, &state, window_handle, &profile_key)?;
    *state.session_window.lock().unwrap() = Some(window_handle);
    tauri::async_runtime::spawn(translation::keep_warm(app.clone(), window_handle));

    // The session works even if the overlay fails; the translations still show in the app.
    if let Err(e) = overlay::open(&app, window_handle) {
        eprintln!("[subquest] overlay: {}", e.detail);
    }
    ingame::on_session_start(&app);
    Ok(())
}

/// Applies the profile's areas (just saved by the UI) to the running session:
/// the core restarts reading the new areas in a fraction of a second, while
/// the overlay and the translation context stay. Nothing to do without a session.
#[tauri::command]
async fn apply_areas(app: AppHandle) -> Result<(), AppError> {
    // Stop the editor's preview first: two captures of the same window being
    // torn down and started together is what froze "Apply" mid-session.
    {
        let state = app.state::<AppState>();
        if let Ok(core) = core(&state) {
            release_preview(core, &mut state.preview_user_data.lock().unwrap());
        }
    }
    // Stopping waits for the OCR thread: keep that off the async runtime's workers.
    tauri::async_runtime::spawn_blocking(move || restart_reading(&app))
        .await
        .map_err(|e| AppError::other(e.to_string()))?
}

/// Restarts the core on the session's game with the profile's current areas
/// (also used to resume after a pause). Nothing to do without a session.
pub(crate) fn restart_reading(app: &AppHandle) -> Result<(), AppError> {
    let state = app.state::<AppState>();
    let Some(window_handle) = *state.session_window.lock().unwrap() else {
        return Ok(());
    };
    let Some(profile_key) = state.active_profile.lock().unwrap().clone() else {
        return Ok(());
    };
    if !state.paused.load(Ordering::SeqCst) {
        match stop_core(&state) {
            Ok(()) => {}
            Err(e) if e.kind == "notRunning" => {} // already stopped (game closed, a failed start): just start again
            Err(e) => return Err(e),
        }
    }
    state.paused.store(false, Ordering::SeqCst);
    // Area indexes may have changed: the overlay starts the game's areas over.
    let _ = app.emit("session-started", profile_key.clone());
    start_core(app, &state, window_handle, &profile_key)
}

/// Saved translations: (how many, file size in bytes), for the Settings screen.
#[tauri::command]
fn cache_usage(state: State<'_, AppState>) -> (usize, u64) {
    state.translation.cache_usage()
}

/// Forgets every saved translation (memory and file).
#[tauri::command]
fn clear_translation_cache(state: State<'_, AppState>) {
    state.translation.clear_cache();
}

/// Ends editing the areas over the game: the new list, or None to cancel.
#[tauri::command]
fn finish_area_edit(app: AppHandle, areas: Option<Vec<settings::GameArea>>) -> Result<(), AppError> {
    ingame::finish_area_edit(&app, areas)
}

/// Starts drawing a rectangle over the game: "area" or "style".
#[tauri::command]
fn begin_selection(app: AppHandle, purpose: ingame::Purpose) -> Result<(), AppError> {
    ingame::begin_selection(&app, purpose)
}

/// The rectangle drawn over the game (window pixels), or null when cancelled.
#[tauri::command]
async fn finish_selection(app: AppHandle, purpose: ingame::Purpose, rect: Option<[i32; 4]>) -> Result<(), AppError> {
    ingame::finish_selection(app, purpose, rect).await
}

/// Pauses or resumes reading; returns true when now paused.
#[tauri::command]
async fn toggle_pause(app: AppHandle) -> Result<bool, AppError> {
    ingame::toggle_pause(&app)
}

/// Hides or shows the translations; returns true when now hidden.
#[tauri::command]
fn toggle_hidden(app: AppHandle) -> bool {
    ingame::toggle_hidden(&app)
}

/// Shows or hides the in-game toolbar; returns true when it is now shown.
#[tauri::command]
fn toggle_toolbar(app: AppHandle) -> Result<bool, AppError> {
    ingame::toggle_toolbar(&app)
}

/// Brings Subquest's main window to the front (the toolbar's "open" button),
/// optionally on a screen.
#[tauri::command]
fn show_main(app: AppHandle, screen: Option<String>) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.unminimize();
        let _ = main.show();
        let _ = main.set_focus();
    }
    if let Some(screen) = screen {
        let _ = app.emit("navigate", screen);
    }
}

/// Paused / hidden flags, for the toolbar when it opens.
#[tauri::command]
fn session_flags(state: State<'_, AppState>) -> (bool, bool) {
    (state.paused.load(Ordering::SeqCst), state.overlay_hidden.load(Ordering::SeqCst))
}

/// Ends the session: hides the overlay, saves what was learned (cache, styles)
/// and stops the core. Used by the Stop button and when the game closes.
pub(crate) fn end_session(app: &AppHandle) -> Result<(), AppError> {
    let state = app.state::<AppState>();
    overlay::close(app);
    ingame::on_session_end(app);
    *state.session_window.lock().unwrap() = None;
    if state.paused.swap(false, Ordering::SeqCst) {
        state.translation.save_cache();
        return Ok(()); // paused: the core is already stopped
    }
    state.translation.save_cache();
    stop_core(&state)
}

/// Stops the session and hides the overlay.
#[tauri::command]
async fn stop_session(app: AppHandle) -> Result<(), AppError> {
    end_session(&app)
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
pub(crate) fn release_preview(core: &Core, slot: &mut Option<*mut PreviewChannel>) {
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

/// Suggests Google Fonts close to the game's font, from a PNG crop of its text
/// (sent as raw bytes). Uses the translator's model, which must accept images.
#[tauri::command]
async fn detect_font(
    state: State<'_, AppState>,
    request: tauri::ipc::Request<'_>,
) -> Result<font_detect::FontDetection, AppError> {
    let tauri::ipc::InvokeBody::Raw(png) = request.body() else {
        return Err(AppError::other("expected the image as raw bytes"));
    };
    let settings = state.settings.lock().unwrap().clone();
    let http = &state.translation.http;
    // A stronger model can be set just for this one-off job (typography is hard).
    let mut translator = settings.translator.clone();
    if !translator.vision_model.trim().is_empty() {
        translator.model = translator.vision_model.clone();
    }
    let model = translation::backend(http, &translator);
    state
        .font_finder
        .detect(&model, http, png, &settings.ui_language)
        .await
}

/// Bytes of a suggested font, so the UI can show a sample before keeping it.
#[tauri::command]
fn candidate_font(state: State<'_, AppState>, file: String) -> Result<tauri::ipc::Response, AppError> {
    let bytes = state
        .font_finder
        .bytes(&file)
        .ok_or_else(|| AppError::other("unknown font candidate"))?;
    Ok(tauri::ipc::Response::new(bytes))
}

/// Keeps a suggested font: it goes to the Fonts folder like an imported file.
#[tauri::command]
fn keep_candidate_font(state: State<'_, AppState>, file: String) -> Result<fonts::FontEntry, AppError> {
    let bytes = state
        .font_finder
        .bytes(&file)
        .ok_or_else(|| AppError::other("unknown font candidate"))?;
    fonts::import(&state.data_dir, &file, &bytes)
}

/// Text styles seen in a game, with their chosen and automatic fonts.
#[tauri::command]
fn list_styles(state: State<'_, AppState>, profile_key: String) -> Vec<styles::TextStyle> {
    state.styles.list(&profile_key)
}

/// PNG thumbnail of a window, for the game list (a short capture of it).
#[tauri::command]
async fn window_thumbnail(app: AppHandle, window_handle: u64) -> Result<tauri::ipc::Response, AppError> {
    tauri::async_runtime::spawn_blocking(move || {
        let picture = ingame::thumbnail(&app, window_handle, THUMBNAIL_WIDTH)?;
        picture.png().map(tauri::ipc::Response::new).ok_or_else(|| AppError::other("could not encode the thumbnail"))
    })
    .await
    .map_err(|e| AppError::other(e.to_string()))?
}

/// PNG of a style's print (the phrase as the game drew it).
#[tauri::command]
fn style_print(
    state: State<'_, AppState>,
    profile_key: String,
    id: String,
) -> Result<tauri::ipc::Response, AppError> {
    std::fs::read(state.styles.print_path(&profile_key, &id))
        .map(tauri::ipc::Response::new)
        .map_err(|e| AppError::other(e.to_string()))
}

/// Changes a style: its name, or the font and letter case chosen for it.
/// Fields left out stay as they are. The overlay restyles itself right away.
#[tauri::command]
fn update_style(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_key: String,
    id: String,
    name: Option<String>,
    font: Option<String>,
    caps: Option<String>,
) -> Result<(), AppError> {
    state.styles.update(&profile_key, |book| {
        if let Some(style) = book.styles.iter_mut().find(|s| s.id == id) {
            if let Some(name) = name {
                style.name = name;
            }
            if let Some(font) = font {
                style.font = font;
            }
            if let Some(caps) = caps {
                style.caps = caps;
            }
        }
    })?;
    styles_changed(&app);
    Ok(())
}

/// Forgets a style and its print.
#[tauri::command]
fn remove_style(app: AppHandle, state: State<'_, AppState>, profile_key: String, id: String) -> Result<(), AppError> {
    state.styles.update(&profile_key, |book| book.styles.retain(|s| s.id != id))?;
    let _ = std::fs::remove_file(state.styles.print_path(&profile_key, &id));
    styles_changed(&app);
    Ok(())
}

/// Deletes all of a game's styles and prints (its profile was forgotten).
#[tauri::command]
fn forget_styles(app: AppHandle, state: State<'_, AppState>, profile_key: String) {
    state.styles.forget(&profile_key);
    styles_changed(&app);
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
    let keys_changed = state.settings.lock().unwrap().shortcuts != settings.shortcuts;
    *state.settings.lock().unwrap() = settings.clone();
    if keys_changed && state.session_window.lock().unwrap().is_some() {
        ingame::register_shortcuts(&app); // new keys work right away, mid-game
    }
    let _ = app.emit("settings-changed", settings); // the overlay and toolbar update live
    Ok(())
}

/// Builds and runs the app: loads the core and settings and registers the commands.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Opening Subquest again (a second click on its shortcut) brings the
        // running one to the front instead of starting a second copy, which
        // would fight the first over the overlay, shortcuts and settings.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.unminimize();
                let _ = main.show();
                let _ = main.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| ingame::on_shortcut(app, shortcut, event.state))
                .build(),
        )
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let data_dir = app.path().app_data_dir()?;
            log_panics(&data_dir);
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
                data_dir: data_dir.clone(),
                session_user_data: Mutex::new(None),
                session_window: Mutex::new(None),
                preview_user_data: Mutex::new(None),
                translation: TranslationState::new(&data_dir),
                overlay_running: Mutex::new(None),
                font_finder: font_detect::FontFinder::default(),
                styles: styles::StyleStore::new(&data_dir),
                paused: AtomicBool::new(false),
                overlay_hidden: AtomicBool::new(false),
                toolbar_wanted: AtomicBool::new(false),
                active_profile: Mutex::new(None),
            });
            fit_main_window(app.handle());
            tray(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            core_status,
            list_windows,
            start_session,
            stop_session,
            apply_areas,
            begin_selection,
            finish_selection,
            finish_area_edit,
            cache_usage,
            clear_translation_cache,
            toggle_pause,
            toggle_hidden,
            toggle_toolbar,
            forget_styles,
            window_thumbnail,
            ui_log,
            session_flags,
            show_main,
            start_preview,
            stop_preview,
            test_translator,
            list_models,
            active_profile,
            import_font,
            list_fonts,
            read_font,
            remove_font,
            detect_font,
            candidate_font,
            keep_candidate_font,
            list_styles,
            style_print,
            update_style,
            remove_style,
            get_settings,
            save_settings
        ])
        .on_window_event(|window, event| {
            // The main window's X: during a translation the game keeps being
            // translated and Subquest waits in the tray (next to the clock);
            // otherwise it really quits. Before, the hidden overlay window kept
            // the process alive, and installing again said it was still running.
            if window.label() != "main" {
                return;
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                if app.state::<AppState>().session_window.lock().unwrap().is_some() {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    quit(app);
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("failed to start Subquest")
        .run(|app, event| {
            // Closing the app also saves what was translated this time.
            if let tauri::RunEvent::Exit = event {
                app.state::<AppState>().translation.save_cache();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_areas_become_core_areas() {
        let mut profile = settings::GameProfile::default();
        let mut dialogue = settings::GameArea::default();
        dialogue.region = Some([10, 20, 300, 80]);
        let mut menu = settings::GameArea::default();
        menu.mode = "screen".into();
        menu.region = Some([1, 2, 3, 4]); // ignored: screen mode reads the whole window
        profile.areas = vec![dialogue, menu];

        let areas = session_areas(Some(&profile));
        assert_eq!(areas.len(), 2);
        assert_eq!((areas[0].x, areas[0].width), (10, 300));
        assert_eq!(areas[1].width, 0);
    }

    #[test]
    fn no_profile_reads_the_whole_window() {
        let areas = session_areas(None);
        assert_eq!(areas.len(), 1);
        assert_eq!(areas[0].width, 0);
    }
}
