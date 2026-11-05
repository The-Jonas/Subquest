#include "capture/window_list.h"

#ifdef _WIN32
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#include <dwmapi.h>
#endif

namespace subquest {

#ifdef _WIN32

// Converts UTF-16 (Windows API) to UTF-8 (what crosses the DLL).
static std::string to_utf8(const wchar_t* text, int length) {
    if (length <= 0) return {};
    const int size = WideCharToMultiByte(CP_UTF8, 0, text, length, nullptr, 0, nullptr, nullptr);
    std::string out(size_t(size), '\0');
    WideCharToMultiByte(CP_UTF8, 0, text, length, out.data(), size, nullptr, nullptr);
    return out;
}

// Finds the name of the executable that owns the window (name only, no folder).
static std::string process_name(HWND hwnd) {
    DWORD pid = 0;
    GetWindowThreadProcessId(hwnd, &pid);
    HANDLE proc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
    if (!proc) return {};
    wchar_t path[MAX_PATH];
    DWORD len = MAX_PATH;
    std::string name;
    if (QueryFullProcessImageNameW(proc, 0, path, &len)) {
        const wchar_t* base = path;
        for (DWORD i = 0; i < len; ++i)
            if (path[i] == L'\\') base = path + i + 1;
        name = to_utf8(base, int(len - DWORD(base - path)));
    }
    CloseHandle(proc);
    return name;
}

// Keeps what a user would recognize as "a game/app window": visible, titled,
// not minimized, not a tool window, not "cloaked" (hidden UWP) and not Subquest
// itself (the DLL runs inside the app's process).
static bool is_capturable(HWND hwnd) {
    if (!IsWindowVisible(hwnd) || IsIconic(hwnd)) return false;
    DWORD pid = 0;
    GetWindowThreadProcessId(hwnd, &pid);
    if (pid == GetCurrentProcessId()) return false;
    if (GetWindowTextLengthW(hwnd) == 0) return false;
    if (GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & WS_EX_TOOLWINDOW) return false;
    BOOL cloaked = FALSE;
    DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &cloaked, sizeof(cloaked));
    return !cloaked;
}

// EnumWindows callback: collects accepted windows into the vector passed in lparam.
static BOOL CALLBACK collect_window(HWND hwnd, LPARAM lparam) {
    if (!is_capturable(hwnd)) return TRUE;
    auto* out = reinterpret_cast<std::vector<WindowInfo>*>(lparam);
    wchar_t title[512];
    const int len = GetWindowTextW(hwnd, title, 512);
    out->push_back({uint64_t(reinterpret_cast<uintptr_t>(hwnd)), to_utf8(title, len), process_name(hwnd)});
    return TRUE;
}

// Walks top-level windows in Z order (front-most first).
std::vector<WindowInfo> list_capturable_windows() {
    std::vector<WindowInfo> windows;
    EnumWindows(collect_window, reinterpret_cast<LPARAM>(&windows));
    return windows;
}

#else

// Nothing to list outside Windows (the core only really builds on Windows).
std::vector<WindowInfo> list_capturable_windows() { return {}; }

#endif

} // namespace subquest
