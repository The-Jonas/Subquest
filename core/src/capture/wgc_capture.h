// wgc_capture.h — captura de janela via Windows.Graphics.Capture (GPU).
#pragma once

#include <atomic>
#include <mutex>

#include "capture/capture_source.h"

namespace subquest {

class WgcCapture final : public ICaptureSource {
public:
    WgcCapture();
    ~WgcCapture() override;

    bool start(uint64_t window_handle, Rect region, FrameCallback on_frame) override;
    void stop() override;
    void set_region(Rect region) override;

private:
    std::mutex region_mutex_;
    Rect region_;               // protegida por region_mutex_ (lida na thread de captura)
    FrameCallback on_frame_;
    std::atomic<bool> running_{false};
    std::atomic<uint64_t> next_frame_id_{1};
};

} // namespace subquest
