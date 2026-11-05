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

// Converte UTF-16 (API do Windows) para UTF-8 (o que atravessa a DLL).
static std::string to_utf8(const wchar_t* text, int length) {
    if (length <= 0) return {};
    const int size = WideCharToMultiByte(CP_UTF8, 0, text, length, nullptr, 0, nullptr, nullptr);
    std::string out(size_t(size), '\0');
    WideCharToMultiByte(CP_UTF8, 0, text, length, out.data(), size, nullptr, nullptr);
    return out;
}

// Descobre o nome do executável dono da janela (só o nome, sem a pasta).
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

// Filtra o que o usuário reconheceria como "uma janela de jogo/app":
// visível, com título, não minimizada, não ferramenta, não "cloaked" (UWP escondido)
// e que não seja do próprio Subquest (a DLL roda dentro do processo do app).
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

// Callback do EnumWindows: acumula as janelas aprovadas no vetor recebido em lparam.
static BOOL CALLBACK collect_window(HWND hwnd, LPARAM lparam) {
    if (!is_capturable(hwnd)) return TRUE;
    auto* out = reinterpret_cast<std::vector<WindowInfo>*>(lparam);
    wchar_t title[512];
    const int len = GetWindowTextW(hwnd, title, 512);
    out->push_back({uint64_t(reinterpret_cast<uintptr_t>(hwnd)), to_utf8(title, len), process_name(hwnd)});
    return TRUE;
}

// Percorre as janelas de nível superior em ordem Z (a mais à frente primeiro).
std::vector<WindowInfo> list_capturable_windows() {
    std::vector<WindowInfo> windows;
    EnumWindows(collect_window, reinterpret_cast<LPARAM>(&windows));
    return windows;
}

#else

// Fora do Windows não há o que listar (o núcleo só compila de verdade no Windows).
std::vector<WindowInfo> list_capturable_windows() { return {}; }

#endif

} // namespace subquest
