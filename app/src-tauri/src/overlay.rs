//! The overlay: a second, transparent app window that sits on top of the whole
//! game window and shows the latest translation of every area.
//!
//! It is created with `focusable(false)` (WS_EX_NOACTIVATE: clicking or showing
//! it never takes focus from the game) and ignores the mouse, so clicks go
//! through to the game. A small loop follows the game window around: if the
//! game moves, resizes or minimizes, the overlay follows; if it closes, the
//! overlay hides.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::core_ffi::SqRect;
use crate::error::AppError;
use crate::AppState;

pub const LABEL: &str = "overlay";
const FOLLOW_INTERVAL: Duration = Duration::from_millis(250); // how often the game's position is checked
const MISSES_BEFORE_ENDING: u32 = 2; // failed position reads in a row = the game closed (~0.5 s)

/// WebView2 flags, the same for BOTH windows (WebView2 refuses two windows of
/// one app with different flags). The first three disabled features are wry's
/// defaults. The rest stop Chromium from pausing a window it believes is
/// covered: over a fullscreen game the overlay counts as "covered", so new
/// translations were only painted after an Alt+Tab. Keep in sync with
/// `additionalBrowserArgs` in tauri.conf.json.
pub const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,CalculateNativeWinOcclusion \
    --disable-backgrounding-occluded-windows --disable-renderer-backgrounding --disable-background-timer-throttling";

/// Where the overlay goes, in screen pixels: [x, y, width, height].
pub type Placement = (i32, i32, u32, u32);

/// Overlay placement for a game window: always the whole window, in screen
/// pixels. Every area's lines come in window pixels, so they map 1:1 onto it;
/// classic subtitle boxes are drawn inside it, at their own area.
pub fn placement(window: SqRect) -> Placement {
    (
        window.x,
        window.y,
        window.width.max(1) as u32,
        window.height.max(1) as u32,
    )
}

/// Returns the overlay window, creating it (hidden) the first time.
fn window(app: &AppHandle) -> Result<WebviewWindow, AppError> {
    if let Some(existing) = app.get_webview_window(LABEL) {
        return Ok(existing);
    }
    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("overlay.html".into()))
        .title("Subquest overlay")
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focusable(false)
        .focused(false)
        .resizable(false)
        .visible(false)
        .additional_browser_args(BROWSER_ARGS)
        .build()
        .map_err(|e| AppError::other(format!("could not create the overlay: {e}")))?;
    window
        .set_ignore_cursor_events(true)
        .map_err(|e| AppError::other(e.to_string()))?;
    Ok(window)
}

/// Puts the overlay back at the top of the topmost windows, without taking
/// focus. A game that gets the foreground (or toggles fullscreen) can end up
/// above it even though the overlay is "always on top"; this undoes that.
#[cfg(windows)]
fn raise(overlay: &WebviewWindow) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE,
    };
    if let Ok(hwnd) = overlay.hwnd() {
        // SAFETY: a valid window handle of ours; only the z-order changes.
        unsafe {
            SetWindowPos(
                hwnd.0 as _,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
            );
        }
    }
}

#[cfg(not(windows))]
fn raise(_overlay: &WebviewWindow) {}

/// True when the window in front belongs to the game or to Subquest itself.
/// Compared by process, so the game's own popups and Subquest's windows
/// (main, overlay) count too.
#[cfg(windows)]
fn game_or_us_in_front(game: u64) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    // SAFETY: plain queries on window handles; a stale handle just gives process 0.
    unsafe {
        let front = GetForegroundWindow();
        if front.is_null() {
            return true; // nothing in front (e.g. switching desktops): don't blink
        }
        let mut front_pid = 0u32;
        let mut game_pid = 0u32;
        GetWindowThreadProcessId(front, &mut front_pid);
        GetWindowThreadProcessId(game as usize as _, &mut game_pid);
        front_pid == std::process::id() || (game_pid != 0 && front_pid == game_pid)
    }
}

#[cfg(not(windows))]
fn game_or_us_in_front(_game: u64) -> bool {
    true
}

/// Shows the overlay over the game and starts following the game window.
/// Any previous follow loop is stopped first.
pub fn open(app: &AppHandle, game: u64) -> Result<(), AppError> {
    close(app);
    let overlay = window(app)?;
    let running = Arc::new(AtomicBool::new(true));
    *app.state::<AppState>().overlay_running.lock().unwrap() = Some(running.clone());

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut last: Option<Placement> = None;
        let mut visible = false;
        let mut misses = 0;
        while running.load(Ordering::SeqCst) {
            let state = app.state::<AppState>();
            let bounds = match state.core.as_ref() {
                Ok(core) => core.window_bounds(game),
                Err(e) => Err(AppError::core_missing(e.clone())),
            };
            match bounds {
                Ok((rect, false)) => {
                    misses = 0;
                    let place = placement(rect);
                    if last != Some(place) {
                        let _ = overlay.set_position(PhysicalPosition::new(place.0, place.1));
                        let _ = overlay.set_size(PhysicalSize::new(place.2, place.3));
                        last = Some(place);
                    }
                    // Only over the game: Alt+Tab to a browser hides the
                    // translation; the game or Subquest in front shows it.
                    let wanted = game_or_us_in_front(game);
                    if wanted && !visible {
                        let _ = overlay.show();
                        visible = true;
                    } else if !wanted && visible {
                        let _ = overlay.hide();
                        visible = false;
                    }
                    if visible {
                        raise(&overlay);
                    }
                }
                Ok((_, true)) => {
                    // Minimized: get out of the way until it comes back.
                    if visible {
                        let _ = overlay.hide();
                        visible = false;
                    }
                }
                Err(_) => {
                    // The game's window is gone (a single failed read could be a
                    // hiccup, so it takes two in a row): end the whole session,
                    // so nothing keeps capturing or translating, and tell the UI.
                    misses += 1;
                    if misses >= MISSES_BEFORE_ENDING {
                        let _ = crate::end_session(&app);
                        let _ = app.emit("session-ended", ());
                        break;
                    }
                }
            }
            tokio::time::sleep(FOLLOW_INTERVAL).await;
        }
        let _ = overlay.hide();
    });
    Ok(())
}

/// Stops following the game and hides the overlay (the window is kept for reuse).
pub fn close(app: &AppHandle) {
    if let Some(running) = app
        .state::<AppState>()
        .overlay_running
        .lock()
        .unwrap()
        .take()
    {
        running.store(false, Ordering::SeqCst);
    }
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.hide();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covers_the_whole_game_window() {
        let game = SqRect {
            x: 100,
            y: 50,
            width: 1920,
            height: 1080,
        };
        assert_eq!(placement(game), (100, 50, 1920, 1080));
    }
}
