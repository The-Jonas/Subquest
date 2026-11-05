// overlay_window.h — transparent, always-on-top, click-through window.
#pragma once

#include <atomic>
#include <mutex>
#include <string>
#include <thread>

#include "common/types.h"

namespace subquest {

class OverlayWindow {
public:
    OverlayWindow();
    ~OverlayWindow();

    void set_text(std::string utf8_text); // schedules a redraw with the new text
    void set_bounds(Rect screen_rect);     // position/size in screen coordinates
    void set_visible(bool visible);        // shows or hides without destroying the window

private:
    void ui_thread_main(); // owns the Win32 window and the message loop

    std::thread ui_thread_;
    std::mutex state_mutex_;
    std::string text_;        // guarded by state_mutex_
    Rect bounds_;             // guarded by state_mutex_
    std::atomic<bool> visible_{true};
    std::atomic<bool> running_{false};
};

} // namespace subquest
