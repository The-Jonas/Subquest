#include "pipeline/preview_stream.h"

namespace subquest {

// Takes the capture source (in the real app, a low-FPS WgcCapture).
PreviewStream::PreviewStream(std::unique_ptr<ICaptureSource> capture) : capture_(std::move(capture)) {}

// Stops capture before being destroyed.
PreviewStream::~PreviewStream() { stop(); }

// Captures the whole window (empty region) and delivers each frame downscaled.
bool PreviewStream::start(uint64_t window_handle, int32_t max_width, PreviewCallback on_image) {
    if (running_) return false;
    const bool ok = capture_->start(window_handle, Rect{}, [max_width, on_image = std::move(on_image)](Frame&& frame) {
        on_image(downscale_to_rgba(frame, max_width), frame.width, frame.height);
    });
    running_ = ok;
    return ok;
}

// Stops capture; after this the callback is no longer called.
void PreviewStream::stop() {
    if (!running_.exchange(false)) return;
    capture_->stop();
}

} // namespace subquest
