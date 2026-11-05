// core_api.cpp — implementação da API em C: traduz chamadas C para objetos C++.
#include "subquest/core_api.h"

#include <memory>
#include <mutex>
#include <string>

#include "capture/wgc_capture.h"
#include "capture/window_list.h"
#include "ocr/win_ocr_engine.h"
#include "overlay/overlay_window.h"
#include "pipeline/pipeline.h"

using namespace subquest;

namespace {

// Estado global do núcleo: uma sessão por vez é suficiente para um jogo por vez.
struct CoreState {
    std::mutex mutex;
    std::unique_ptr<Pipeline> pipeline;
    std::unique_ptr<OverlayWindow> overlay;
    std::string last_error;
};

// Instância única criada no primeiro uso (evita problemas de ordem de inicialização).
CoreState& state() {
    static CoreState s;
    return s;
}

// Registra a mensagem de erro e devolve o código, para usar em "return fail(...)".
SqStatus fail(SqStatus code, const char* message) {
    state().last_error = message;
    return code;
}

} // namespace

// Versão da ABI; o Rust recusa carregar uma DLL com versão diferente.
extern "C" SQ_API uint32_t sq_api_version(void) { return SQ_API_VERSION; }

// Chama o callback uma vez por janela capturável.
extern "C" SQ_API SqStatus sq_list_windows(SqWindowCallback callback, void* user_data) {
    if (!callback) return fail(SQ_ERR_INVALID_ARG, "callback nulo");
    for (const WindowInfo& w : list_capturable_windows())
        callback(w.handle, w.title.c_str(), w.process.c_str(), user_data);
    return SQ_OK;
}

// Monta o pipeline com as implementações do Windows e inicia a sessão.
extern "C" SQ_API SqStatus sq_start(const SqSessionConfig* config, SqTextCallback callback, void* user_data) {
    if (!config || !callback || !config->ocr_language) return fail(SQ_ERR_INVALID_ARG, "config ou callback nulo");

    CoreState& s = state();
    std::lock_guard lock(s.mutex);
    if (s.pipeline && s.pipeline->running()) return fail(SQ_ERR_ALREADY_RUNNING, "sessao ja ativa");

    PipelineConfig pc;
    pc.window_handle = config->window_handle;
    pc.region = {config->region.x, config->region.y, config->region.width, config->region.height};
    pc.ocr_language = config->ocr_language;
    if (config->settle_frames > 0) pc.detector.settle_frames = config->settle_frames;
    if (config->change_threshold > 0.0f) pc.detector.change_threshold = config->change_threshold;

    s.pipeline = std::make_unique<Pipeline>(std::make_unique<WgcCapture>(), std::make_unique<WinOcrEngine>());
    if (!s.overlay) s.overlay = std::make_unique<OverlayWindow>();

    const bool ok = s.pipeline->start(pc, [callback, user_data](const OcrResult& r) {
        callback(r.full_text.c_str(), r.frame_id, r.elapsed_ms, user_data);
    });
    if (!ok) return fail(SQ_ERR_NOT_IMPLEMENTED, "captura/OCR ainda nao implementados (fase 1)");
    return SQ_OK;
}

// Encerra a sessão ativa.
extern "C" SQ_API SqStatus sq_stop(void) {
    CoreState& s = state();
    std::lock_guard lock(s.mutex);
    if (!s.pipeline || !s.pipeline->running()) return fail(SQ_ERR_NOT_RUNNING, "nenhuma sessao ativa");
    s.pipeline->stop();
    return SQ_OK;
}

// Repassa a tradução pronta para o overlay.
extern "C" SQ_API SqStatus sq_overlay_set_text(const char* utf8_text) {
    if (!utf8_text) return fail(SQ_ERR_INVALID_ARG, "texto nulo");
    CoreState& s = state();
    std::lock_guard lock(s.mutex);
    if (!s.overlay) s.overlay = std::make_unique<OverlayWindow>();
    s.overlay->set_text(utf8_text);
    return SQ_OK;
}

// Mostra ou esconde o overlay (atalho global do usuário).
extern "C" SQ_API SqStatus sq_overlay_set_visible(int32_t visible) {
    CoreState& s = state();
    std::lock_guard lock(s.mutex);
    if (!s.overlay) return fail(SQ_ERR_NOT_RUNNING, "overlay ainda nao criado");
    s.overlay->set_visible(visible != 0);
    return SQ_OK;
}

// Última mensagem de erro registrada por fail().
extern "C" SQ_API const char* sq_last_error(void) { return state().last_error.c_str(); }
