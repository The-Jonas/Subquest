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

} // namespace subquest
