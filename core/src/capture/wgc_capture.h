// wgc_capture.h — window capture through Windows.Graphics.Capture (GPU).
//
// D3D11/WinRT types are hidden in the .cpp (the "pimpl" idiom), so code that
// includes this header doesn't pull in the heavy Windows headers.
#pragma once

#include <memory>
#include <string>

#include "capture/capture_source.h"

namespace subquest {

class WgcCapture final : public ICaptureSource {
public:
    explicit WgcCapture(uint32_t max_fps = 30); // caps how many frames per second are delivered
    ~WgcCapture() override;

    bool start(uint64_t window_handle, Rect region, FrameCallback on_frame) override;
    void stop() override;
    void set_region(Rect region) override;
    const std::string& error() const override { return error_; }

private:
    struct Impl;                 // defined in the .cpp (Windows) or empty (other OSes)
    std::unique_ptr<Impl> impl_;
    uint32_t max_fps_;
    std::string error_;
};

} // namespace subquest
