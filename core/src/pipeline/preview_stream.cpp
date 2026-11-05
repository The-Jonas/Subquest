#include "pipeline/preview_stream.h"

namespace subquest {

// Recebe a fonte de captura (no app real, uma WgcCapture com FPS baixo).
PreviewStream::PreviewStream(std::unique_ptr<ICaptureSource> capture) : capture_(std::move(capture)) {}

// Para a captura antes de destruir.
PreviewStream::~PreviewStream() { stop(); }

// Captura a janela inteira (região vazia) e entrega cada quadro já reduzido.
bool PreviewStream::start(uint64_t window_handle, int32_t max_width, PreviewCallback on_image) {
    if (running_) return false;
    const bool ok = capture_->start(window_handle, Rect{}, [max_width, on_image = std::move(on_image)](Frame&& frame) {
        on_image(downscale_to_rgba(frame, max_width), frame.width, frame.height);
    });
    running_ = ok;
    return ok;
}

// Encerra a captura; depois disso o callback não é mais chamado.
void PreviewStream::stop() {
    if (!running_.exchange(false)) return;
    capture_->stop();
}

} // namespace subquest
