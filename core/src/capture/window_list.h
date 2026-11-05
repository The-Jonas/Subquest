// window_list.h — lists the windows that capture can target.
#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace subquest {

struct WindowInfo {
    uint64_t handle = 0;  // HWND cast to an integer
    std::string title;    // UTF-8
    std::string process;  // executable name, e.g. "Hollow Knight.exe"
};

std::vector<WindowInfo> list_capturable_windows(); // visible, titled, non-minimized, non-tool windows

struct WindowBounds {
    int32_t x = 0, y = 0, width = 0, height = 0; // on screen, physical pixels (same space as the capture)
    bool minimized = false;
};

bool window_bounds(uint64_t handle, WindowBounds& out); // false if the window no longer exists

} // namespace subquest
