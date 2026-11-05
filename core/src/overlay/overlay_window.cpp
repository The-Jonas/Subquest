#include "overlay/overlay_window.h"

namespace subquest {

// Doesn't create the window yet: that happens on its own thread (phase 1).
OverlayWindow::OverlayWindow() = default;

// Stops the window thread, if running.
OverlayWindow::~OverlayWindow() {
    running_ = false;
    if (ui_thread_.joinable()) ui_thread_.join();
}

// Stores the text; the window thread redraws when it gets the message.
// TODO(phase 1): PostMessage(hwnd, WM_APP_REDRAW) after swapping the text.
void OverlayWindow::set_text(std::string utf8_text) {
    std::lock_guard lock(state_mutex_);
    text_ = std::move(utf8_text);
}

// Stores the new position; the window thread applies it with SetWindowPos.
void OverlayWindow::set_bounds(Rect screen_rect) {
    std::lock_guard lock(state_mutex_);
    bounds_ = screen_rect;
}

// Toggles visibility (ShowWindow on the window thread).
void OverlayWindow::set_visible(bool visible) {
    visible_ = visible;
}

// TODO(phase 1): window plan:
//  1. RegisterClassEx + CreateWindowEx with WS_EX_LAYERED | WS_EX_TRANSPARENT |
//     WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE (never steals game focus).
//  2. Direct2D (ID2D1HwndRenderTarget or DComposition) + DirectWrite for the text,
//     with a translucent background and an outline so it reads over any scene.
//  3. GetMessage loop; redraw on WM_APP_REDRAW.
void OverlayWindow::ui_thread_main() {}

} // namespace subquest
