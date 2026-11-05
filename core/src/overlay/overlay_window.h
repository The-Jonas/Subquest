// overlay_window.h — janela transparente, sempre no topo e "clicável através".
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

    void set_text(std::string utf8_text); // agenda um redesenho com o novo texto
    void set_bounds(Rect screen_rect);     // posição/tamanho em coordenadas de tela
    void set_visible(bool visible);        // mostra ou esconde sem destruir a janela

private:
    void ui_thread_main(); // dona da janela Win32 e do loop de mensagens

    std::thread ui_thread_;
    std::mutex state_mutex_;
    std::string text_;        // protegido por state_mutex_
    Rect bounds_;             // protegido por state_mutex_
    std::atomic<bool> visible_{true};
    std::atomic<bool> running_{false};
};

} // namespace subquest
