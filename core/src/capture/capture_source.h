// capture_source.h — interface for any frame source.
//
// The real implementation is WgcCapture (Windows.Graphics.Capture). Having an
// interface lets tests swap in a fake source (images from disk).
#pragma once

#include <cstdint>
#include <functional>
#include <string>

#include "common/types.h"

namespace subquest {

using FrameCallback = std::function<void(Frame&&)>; // called on the capture thread

class ICaptureSource {
public:
    virtual ~ICaptureSource() = default;

    virtual bool start(uint64_t window_handle, Rect region, FrameCallback on_frame) = 0; // starts delivering frames already cropped to `region`
    virtual void stop() = 0;                                                         // stops and waits for the capture thread to finish
    virtual void set_region(Rect region) = 0;                                        // changes the region without restarting capture
    virtual const std::string& error() const = 0;                                    // reason for the last start() failure, as text
};

} // namespace subquest
