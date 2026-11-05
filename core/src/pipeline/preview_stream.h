// preview_stream.h — miniatura ao vivo da janela do jogo, para a interface.
//
// Usa a mesma captura da sessão, mas com FPS baixo e a imagem reduzida e já
// convertida para RGBA, pronta para desenhar num <canvas>.
#pragma once

#include <atomic>
#include <functional>
#include <memory>
#include <string>

#include "capture/capture_source.h"
#include "vision/image_ops.h"

namespace subquest {

// Recebe a imagem reduzida e o tamanho real da captura (para converter coordenadas).
using PreviewCallback = std::function<void(const RgbaImage&, int32_t source_width, int32_t source_height)>;

class PreviewStream {
public:
    explicit PreviewStream(std::unique_ptr<ICaptureSource> capture);
    ~PreviewStream();

    bool start(uint64_t window_handle, int32_t max_width, PreviewCallback on_image); // false => ver error()
    void stop();                                                                      // idempotente
    bool running() const { return running_; }
    const std::string& error() const { return capture_->error(); }

private:
    std::unique_ptr<ICaptureSource> capture_;
    std::atomic<bool> running_{false};
};

} // namespace subquest
