//! Controls used from inside the game, without going back to Subquest's window:
//! a small floating toolbar (like MORT's) and global shortcuts.
//!
//! Default keys (all can be changed in the Shortcuts screen):
//!   Ctrl+Shift+A  mark a new area: drag a rectangle right over the game
//!   Ctrl+Shift+S  take a style print: drag over a phrase, then pick its font
//!   Ctrl+Shift+G  glossary: click a word in the game, say how to translate it
//!   Ctrl+Shift+T  pause / resume translating
//!   Ctrl+Shift+H  hide / show the translations
//!   Ctrl+Shift+B  show / hide the toolbar (hidden until asked for, so it
//!                 doesn't sit on the game the whole time)
//!
//! Marking happens on the overlay: it starts taking the mouse, shows a dimmed
//! layer and reports the rectangle in window pixels.

use std::sync::atomic::Ordering;
use std::sync::{Condvar, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::error::AppError;
use crate::profiles::GameArea;
use crate::styles::Rgba;
use crate::{overlay, AppState};

pub const TOOLBAR: &str = "toolbar";
const GRAB_WAIT: Duration = Duration::from_secs(2); // the first full-size frame usually comes in ~100 ms
const THUMBNAIL_WAIT: Duration = Duration::from_millis(900); // a minimized window never sends one

/// What a rectangle drawn over the game is for.
#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Purpose {
    Area,  // a new area to read
    Style, // a print of a phrase, to pick its font
    Edit,  // move, resize, rename or remove the areas already marked
    Term,  // click a word in the game to add it to the glossary
}

/// Something a shortcut does.
#[derive(Clone, Copy)]
enum Action {
    Area,
    Style,
    Pause,
    Hide,
    Toolbar,
    Edit,
    Term,
}

/// The configured shortcuts that parse, with their action. A key written in
/// a way the plugin doesn't understand is skipped (and logged).
fn shortcuts(app: &AppHandle) -> Vec<(Shortcut, Action)> {
    let keys = app.state::<AppState>().settings.lock().unwrap().shortcuts.clone();
    [
        (keys.area, Action::Area),
        (keys.style, Action::Style),
        (keys.pause, Action::Pause),
        (keys.hide, Action::Hide),
        (keys.toolbar, Action::Toolbar),
        (keys.edit, Action::Edit),
        (keys.term, Action::Term),
    ]
    .into_iter()
    .filter(|(text, _)| !text.trim().is_empty())
    .filter_map(|(text, action)| match text.parse::<Shortcut>() {
        Ok(shortcut) => Some((shortcut, action)),
        Err(e) => {
            eprintln!("[subquest] shortcut \"{text}\" not understood: {e}");
            None
        }
    })
    .collect()
}

/// Runs a shortcut (only during a session; it is only registered then anyway).
pub fn on_shortcut(app: &AppHandle, shortcut: &Shortcut, state: ShortcutState) {
    if state != ShortcutState::Pressed {
        return;
    }
    let Some(action) = shortcuts(app).into_iter().find(|(s, _)| s == shortcut).map(|(_, a)| a) else {
        return;
    };
    let result = match action {
        Action::Area => begin_selection(app, Purpose::Area),
        Action::Style => begin_selection(app, Purpose::Style),
        Action::Pause => toggle_pause(app).map(|_| ()),
        Action::Hide => {
            toggle_hidden(app);
            Ok(())
        }
        Action::Toolbar => toggle_toolbar(app).map(|_| ()),
        Action::Edit => begin_selection(app, Purpose::Edit),
        Action::Term => begin_selection(app, Purpose::Term),
    };
    if let Err(e) = result {
        eprintln!("[subquest] shortcut: {}", e.detail);
    }
}

/// (Re)registers the configured shortcuts: at the start of a session, and when
/// they are changed during one. Keys taken by another program are skipped (the
/// toolbar still does everything).
pub fn register_shortcuts(app: &AppHandle) {
    let keys = app.global_shortcut();
    let _ = keys.unregister_all();
    for (shortcut, _) in shortcuts(app) {
        if let Err(e) = keys.register(shortcut) {
            eprintln!("[subquest] shortcut taken by another app: {e}");
        }
    }
}

/// Session started: registers the shortcuts. The toolbar stays hidden until
/// Ctrl+Shift+B (or the button in Subquest) asks for it.
pub fn on_session_start(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.paused.store(false, Ordering::SeqCst);
    state.overlay_hidden.store(false, Ordering::SeqCst);
    state.toolbar_wanted.store(false, Ordering::SeqCst);
    register_shortcuts(app);
}

/// Shows or hides the toolbar; returns true when it is now asked for. The
/// overlay's follow loop does the actual showing, and only while the game (or
/// Subquest) is in front. The first time, it is placed at the game's top-right.
pub fn toggle_toolbar(app: &AppHandle) -> Result<bool, AppError> {
    let state = app.state::<AppState>();
    let Some(game) = *state.session_window.lock().unwrap() else {
        return Ok(false); // no game, no toolbar
    };
    let wanted = !state.toolbar_wanted.load(Ordering::SeqCst);
    if wanted && app.get_webview_window(TOOLBAR).is_none() {
        let bounds = crate::core(&state)?.window_bounds(game).ok().map(|(r, _)| (r.x, r.y, r.width));
        create_toolbar(app, bounds)?;
    }
    state.toolbar_wanted.store(wanted, Ordering::SeqCst);
    if let Some(toolbar) = app.get_webview_window(TOOLBAR) {
        let _ = if wanted { toolbar.show() } else { toolbar.hide() };
    }
    let _ = app.emit("toolbar-shown", wanted);
    Ok(wanted)
}

/// Session ended: frees the shortcuts and hides the toolbar.
pub fn on_session_end(app: &AppHandle) {
    let _ = app.global_shortcut().unregister_all();
    app.state::<AppState>().toolbar_wanted.store(false, Ordering::SeqCst);
    if let Some(toolbar) = app.get_webview_window(TOOLBAR) {
        let _ = toolbar.hide();
    }
    let _ = app.emit("toolbar-shown", false);
    if let Some(overlay) = app.get_webview_window(overlay::LABEL) {
        let _ = overlay.set_ignore_cursor_events(true);
    }
}

/// Creates the toolbar (hidden) at the game's top-right corner. `game_bounds`
/// = (x, y, width) of the game window in screen pixels.
fn create_toolbar(app: &AppHandle, game_bounds: Option<(i32, i32, i32)>) -> Result<(), AppError> {
    let toolbar = WebviewWindowBuilder::new(app, TOOLBAR, WebviewUrl::App("toolbar.html".into()))
        .title("Subquest")
        .inner_size(392.0, 84.0) // the bar, plus room below it for the button labels
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focusable(false) // clicking it never takes the focus from the game
        .resizable(false)
        .visible(false)
        .additional_browser_args(overlay::BROWSER_ARGS)
        .build()
        .map_err(|e| AppError::other(format!("could not create the toolbar: {e}")))?;
    if let Some((x, y, width)) = game_bounds {
        let size = toolbar.outer_size().map(|s| s.width as i32).unwrap_or(392);
        let _ = toolbar.set_position(tauri::PhysicalPosition::new(x + width - size - 24, y + 24));
    }
    Ok(())
}

/// Starts marking a rectangle over the game: the overlay takes the mouse.
pub fn begin_selection(app: &AppHandle, purpose: Purpose) -> Result<(), AppError> {
    let state = app.state::<AppState>();
    if state.session_window.lock().unwrap().is_none() {
        return Ok(()); // nothing to mark without a game
    }
    let overlay = app
        .get_webview_window(overlay::LABEL)
        .ok_or_else(|| AppError::other("overlay not open"))?;
    state.overlay_hidden.store(false, Ordering::SeqCst);
    overlay
        .set_ignore_cursor_events(false)
        .map_err(|e| AppError::other(e.to_string()))?;
    let _ = overlay.show();
    if matches!(purpose, Purpose::Edit | Purpose::Term) {
        // Renaming an area or typing a term's translation needs the keyboard:
        // the overlay may take the focus until done.
        let _ = overlay.set_focusable(true);
        let _ = overlay.set_focus();
    }
    let _ = app.emit("select-region", purpose);
    Ok(())
}

/// Ends editing the areas over the game. With `areas` (the whole new list,
/// in window pixels), they replace the game's areas and are read right away;
/// None cancels. Either way the overlay lets the mouse and keyboard through
/// again and the game gets the focus back.
pub fn finish_area_edit(app: &AppHandle, areas: Option<Vec<GameArea>>) -> Result<(), AppError> {
    let state = app.state::<AppState>();
    if let Some(overlay) = app.get_webview_window(overlay::LABEL) {
        let _ = overlay.set_ignore_cursor_events(true);
        let _ = overlay.set_focusable(false);
    }
    if let Some(game) = *state.session_window.lock().unwrap() {
        overlay::focus_game(game);
    }
    let (Some(areas), Some(key)) = (areas, state.active_profile.lock().unwrap().clone()) else {
        return Ok(());
    };
    state.profiles.update(&key, |p| p.areas = areas)?;
    crate::profiles_changed(app);
    crate::restart_reading(app)
}

/// The rectangle drawn over the game (window pixels), or None when cancelled.
/// An area is added to the game's profile and read right away; a style print
/// is saved and Subquest opens on the Styles screen to pick its font.
pub async fn finish_selection(app: AppHandle, purpose: Purpose, rect: Option<[i32; 4]>) -> Result<(), AppError> {
    if let Some(overlay) = app.get_webview_window(overlay::LABEL) {
        let _ = overlay.set_ignore_cursor_events(true);
        if purpose == Purpose::Term {
            let _ = overlay.set_focusable(false);
        }
    }
    if purpose == Purpose::Term {
        // The overlay saved the term itself (save_glossary): the game gets the keyboard back.
        if let Some(game) = *app.state::<AppState>().session_window.lock().unwrap() {
            overlay::focus_game(game);
        }
        return Ok(());
    }
    let Some(rect) = rect.filter(|r| r[2] >= 8 && r[3] >= 8) else {
        return Ok(());
    };
    let state = app.state::<AppState>();
    let Some(key) = state.active_profile.lock().unwrap().clone() else {
        return Ok(());
    };
    match purpose {
        Purpose::Edit | Purpose::Term => Ok(()), // edits end through finish_area_edit; terms returned above
        Purpose::Area => {
            state.profiles.update(&key, |profile| {
                // A game is read through marked areas or as a whole window, never
                // both: marking an area over a whole-window game switches it to areas.
                profile.areas.retain(|a| a.mode != "screen");
                profile.areas.push(GameArea {
                    region: Some(rect),
                    ..GameArea::default()
                });
            })?;
            crate::profiles_changed(&app);
            crate::restart_reading(&app)
        }
        Purpose::Style => {
            let Some(handle) = *state.session_window.lock().unwrap() else {
                return Ok(());
            };
            // The capture waits for a frame: off the async threads.
            let grabber = app.clone();
            let frame = tauri::async_runtime::spawn_blocking(move || grab_window(&grabber, handle))
                .await
                .map_err(|e| AppError::other(e.to_string()))??;
            let print = frame.crop(rect).ok_or_else(|| AppError::other("the rectangle is outside the game"))?;
            let id = state.styles.add_from_print(&key, &print)?;
            crate::styles_changed(&app);
            if let Some(main) = app.get_webview_window("main") {
                let _ = main.unminimize();
                let _ = main.show();
                let _ = main.set_focus();
            }
            let _ = app.emit("style-captured", id);
            Ok(())
        }
    }
}

/// Pauses or resumes reading the game; returns true when now paused.
pub fn toggle_pause(app: &AppHandle) -> Result<bool, AppError> {
    let state = app.state::<AppState>();
    let paused = !state.paused.load(Ordering::SeqCst);
    if paused {
        crate::stop_core(&state)?;
    } else {
        crate::restart_reading(app)?;
    }
    state.paused.store(paused, Ordering::SeqCst);
    let _ = app.emit("session-paused", paused);
    Ok(paused)
}

/// Hides or shows the translations (the overlay follow loop applies it).
pub fn toggle_hidden(app: &AppHandle) -> bool {
    let state = app.state::<AppState>();
    let hidden = !state.overlay_hidden.load(Ordering::SeqCst);
    state.overlay_hidden.store(hidden, Ordering::SeqCst);
    let _ = app.emit("overlay-hidden", hidden);
    hidden
}

/// Where the core's preview callback leaves the first frame wanted.
#[derive(Default)]
struct GrabSlot {
    frame: Mutex<Option<Rgba>>,
    ready: Condvar,
    any_size: bool, // a thumbnail takes the first frame; a grab waits for a full-size one
}

/// Preview callback for `grab`: keeps the first frame wanted.
unsafe extern "C" fn on_grab_frame(
    rgba: *const u8,
    width: i32,
    height: i32,
    source_width: i32,
    source_height: i32,
    user: *mut std::ffi::c_void,
) {
    let slot = &*(user as *const GrabSlot);
    if !slot.any_size && (width != source_width || height != source_height) {
        return;
    }
    let mut frame = slot.frame.lock().unwrap();
    if frame.is_none() {
        let len = width as usize * height as usize * 4;
        *frame = Some(Rgba {
            pixels: std::slice::from_raw_parts(rgba, len).to_vec(),
            width,
            height,
        });
        slot.ready.notify_all();
    }
}

/// One full-size picture of the game window, through a short preview capture
/// (the in-app preview, if open, stops: only one preview runs at a time).
pub fn grab_window(app: &AppHandle, window_handle: u64) -> Result<Rgba, AppError> {
    grab(app, window_handle, 16384, false, GRAB_WAIT)
}

/// A small picture of a window (`max_width` wide at most), for the game list.
/// Minimized windows send no frames: they time out quickly and get no picture.
pub fn thumbnail(app: &AppHandle, window_handle: u64, max_width: i32) -> Result<Rgba, AppError> {
    grab(app, window_handle, max_width, true, THUMBNAIL_WAIT)
}

/// One picture of a window through a short preview capture (the in-app
/// preview, if open, stops: only one preview runs at a time).
fn grab(app: &AppHandle, window_handle: u64, max_width: i32, any_size: bool, wait: Duration) -> Result<Rgba, AppError> {
    let state = app.state::<AppState>();
    let core = crate::core(&state)?;
    let mut preview = state.preview_user_data.lock().unwrap();
    crate::release_preview(core, &mut preview);

    let slot = Box::new(GrabSlot { any_size, ..GrabSlot::default() });
    let user = &*slot as *const GrabSlot as *mut std::ffi::c_void;
    core.preview_start(window_handle, max_width, on_grab_frame, user)?;
    let frame = {
        let guard = slot.frame.lock().unwrap();
        let (mut guard, _) = slot
            .ready
            .wait_timeout_while(guard, wait, |frame| frame.is_none())
            .unwrap();
        guard.take()
    };
    let _ = core.preview_stop(); // after this the callback is never called again, so `slot` may go
    drop(slot);
    frame.ok_or_else(|| AppError::other("no picture of the game came in time"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::ShortcutSettings;

    #[test]
    fn the_keys_the_shortcuts_screen_writes_are_understood() {
        let defaults = ShortcutSettings::default();
        for text in [defaults.area, defaults.style, defaults.pause, defaults.hide, defaults.toolbar, defaults.edit] {
            assert!(text.parse::<Shortcut>().is_ok(), "{text}");
        }
        for text in ["Ctrl+Alt+F5", "Ctrl+Shift+1", "F9", "Alt+Space", "Super+Shift+PageDown", "Ctrl+Shift+ArrowUp"] {
            assert!(text.parse::<Shortcut>().is_ok(), "{text}");
        }
    }
}
