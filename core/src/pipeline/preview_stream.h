// preview_stream.h — live thumbnail of the game window, for the UI.
//
// Uses the same capture as a session, but at low FPS and with the image
// downscaled and already converted to RGBA, ready to draw on a <canvas>.
#pragma once

#include <atomic>
#include <functional>
#include <memory>
#include <string>

#include "capture/capture_source.h"
#include "vision/image_ops.h"

namespace subquest {

// Receives the downscaled image and the real capture size (to convert coordinates).
using PreviewCallback = std::function<void(const RgbaImage&, int32_t source_width, int32_t source_height)>;

class PreviewStream {
public:
    explicit PreviewStream(std::unique_ptr<ICaptureSource> capture);
    ~PreviewStream();

    bool start(uint64_t window_handle, int32_t max_width, PreviewCallback on_image); // false => see error()
    void stop();                                                                      // idempotent
    bool running() const { return running_; }
    const std::string& error() const { return capture_->error(); }

private:
    std::unique_ptr<ICaptureSource> capture_;
    std::atomic<bool> running_{false};
};

} // namespace subquest
