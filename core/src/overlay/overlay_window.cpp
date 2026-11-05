#include "overlay/overlay_window.h"

namespace subquest {

// Não cria a janela ainda: isso acontece na thread própria (fase 1).
OverlayWindow::OverlayWindow() = default;

// Encerra a thread da janela, se estiver rodando.
OverlayWindow::~OverlayWindow() {
    running_ = false;
    if (ui_thread_.joinable()) ui_thread_.join();
}

// Guarda o texto; a thread da janela redesenha ao receber a mensagem.
// TODO(fase 1): PostMessage(hwnd, WM_APP_REDRAW) depois de trocar o texto.
void OverlayWindow::set_text(std::string utf8_text) {
    std::lock_guard lock(state_mutex_);
    text_ = std::move(utf8_text);
}

// Guarda a nova posição; a thread da janela aplica com SetWindowPos.
void OverlayWindow::set_bounds(Rect screen_rect) {
    std::lock_guard lock(state_mutex_);
    bounds_ = screen_rect;
}

// Alterna a visibilidade (ShowWindow na thread da janela).
void OverlayWindow::set_visible(bool visible) {
    visible_ = visible;
}

// TODO(fase 1): roteiro da janela:
//  1. RegisterClassEx + CreateWindowEx com WS_EX_LAYERED | WS_EX_TRANSPARENT |
//     WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE (não rouba o foco do jogo).
//  2. Direct2D (ID2D1HwndRenderTarget ou DComposition) + DirectWrite para o texto,
//     com fundo semitransparente e contorno para ler sobre qualquer cenário.
//  3. Loop GetMessage; redesenhar em WM_APP_REDRAW.
void OverlayWindow::ui_thread_main() {}

} // namespace subquest
