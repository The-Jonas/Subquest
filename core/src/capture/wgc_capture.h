// wgc_capture.h — captura de janela via Windows.Graphics.Capture (GPU).
//
// Os tipos do D3D11/WinRT ficam escondidos no .cpp (idioma "pimpl"), para que
// quem inclui este header não precise puxar os headers pesados do Windows.
#pragma once

#include <memory>
#include <string>

#include "capture/capture_source.h"

namespace subquest {

class WgcCapture final : public ICaptureSource {
public:
    explicit WgcCapture(uint32_t max_fps = 30); // limita quantos quadros por segundo são entregues
    ~WgcCapture() override;

    bool start(uint64_t window_handle, Rect region, FrameCallback on_frame) override;
    void stop() override;
    void set_region(Rect region) override;
    const std::string& error() const override { return error_; }

private:
    struct Impl;                 // definido no .cpp (Windows) ou vazio (outros SOs)
    std::unique_ptr<Impl> impl_;
    uint32_t max_fps_;
    std::string error_;
};

} // namespace subquest
