//! The overlay: a second, transparent app window that sits on top of the game
//! and shows the latest translation.
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
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::core_ffi::SqRect;
use crate::error::AppError;
use crate::AppState;

pub const LABEL: &str = "overlay";
const FOLLOW_INTERVAL: Duration = Duration::from_millis(250); // how often the game's position is checked

/// Where the overlay goes, in screen pixels: [x, y, width, height].
pub type Placement = (i32, i32, u32, u32);

/// Overlay placement for a game window. With a dialogue area, the overlay
/// covers exactly that area (the translation replaces the original text).
/// Without one, it's a subtitle band near the bottom of the window.
pub fn placement(window: SqRect, region: Option<SqRect>) -> Placement {
    if let Some(r) = region.filter(|r| r.width > 0 && r.height > 0) {
        return (
            window.x + r.x,
            window.y + r.y,
            r.width as u32,
            r.height as u32,
        );
    }
    let width = window.width * 7 / 10;
    let height = (window.height * 22 / 100).max(80);
    let x = window.x + (window.width - width) / 2;
    let y = window.y + window.height - height - window.height * 4 / 100;
    (x, y, width.max(1) as u32, height as u32)
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
        .build()
        .map_err(|e| AppError::other(format!("could not create the overlay: {e}")))?;
    window
        .set_ignore_cursor_events(true)
        .map_err(|e| AppError::other(e.to_string()))?;
    Ok(window)
}

/// Shows the overlay over the game and starts following the game window.
/// Any previous follow loop is stopped first.
pub fn open(app: &AppHandle, game: u64, region: Option<SqRect>) -> Result<(), AppError> {
    close(app);
    let overlay = window(app)?;
    let running = Arc::new(AtomicBool::new(true));
    *app.state::<AppState>().overlay_running.lock().unwrap() = Some(running.clone());

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut last: Option<Placement> = None;
        let mut visible = false;
        while running.load(Ordering::SeqCst) {
            let state = app.state::<AppState>();
            let bounds = match state.core.as_ref() {
                Ok(core) => core.window_bounds(game),
                Err(e) => Err(AppError::core_missing(e.clone())),
            };
            match bounds {
                Ok((rect, false)) => {
                    let place = placement(rect, region);
                    if last != Some(place) {
                        let _ = overlay.set_position(PhysicalPosition::new(place.0, place.1));
                        let _ = overlay.set_size(PhysicalSize::new(place.2, place.3));
                        last = Some(place);
                    }
                    if !visible {
                        let _ = overlay.show();
                        visible = true;
                    }
                }
                Ok((_, true)) | Err(_) => {
                    // Minimized or closed: get out of the way until it comes back.
                    if visible {
                        let _ = overlay.hide();
                        visible = false;
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

    fn game() -> SqRect {
        SqRect {
            x: 100,
            y: 50,
            width: 1920,
            height: 1080,
        }
    }

    #[test]
    fn covers_the_dialogue_area_in_screen_coordinates() {
        let area = SqRect {
            x: 300,
            y: 820,
            width: 1200,
            height: 200,
        };
        assert_eq!(placement(game(), Some(area)), (400, 870, 1200, 200));
    }

    #[test]
    fn without_an_area_it_is_a_band_at_the_bottom() {
        let (x, y, w, h) = placement(game(), None);
        assert_eq!((w, h), (1344, 237));
        assert_eq!(x, 100 + (1920 - 1344) / 2);
        assert!(
            y + h as i32 <= 50 + 1080,
            "band must stay inside the window"
        );
    }
}
