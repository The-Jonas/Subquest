// window_list.h — lista as janelas em que dá para "mirar" a captura.
#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace subquest {

struct WindowInfo {
    uint64_t handle = 0;  // HWND convertido para inteiro
    std::string title;    // UTF-8
    std::string process;  // nome do executável, ex.: "Hollow Knight.exe"
};

std::vector<WindowInfo> list_capturable_windows(); // janelas visíveis, com título, não minimizadas e não ferramentas

} // namespace subquest
