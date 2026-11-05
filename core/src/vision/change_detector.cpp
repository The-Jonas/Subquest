#include "vision/change_detector.h"

#include <algorithm>
#include <cstdlib>

namespace subquest {

// Stores the config; history starts empty.
ChangeDetector::ChangeDetector(ChangeDetectorConfig config) : config_(config) {}

// Clears history so the next stable frame always fires.
void ChangeDetector::reset() {
    previous_.clear();
    last_fired_.clear();
    still_count_ = 0;
}

// Mean of the per-pixel absolute difference, normalized to 0..1.
float ChangeDetector::mean_abs_diff(const std::vector<uint8_t>& a, const std::vector<uint8_t>& b) {
    if (a.size() != b.size() || a.empty()) return 1.0f;
    uint64_t sum = 0;
    for (size_t i = 0; i < a.size(); ++i) {
        sum += static_cast<uint64_t>(std::abs(int(a[i]) - int(b[i])));
    }
    return float(sum) / (float(a.size()) * 255.0f);
}

// Shrinks the frame with a box filter and converts BGRA to luminance.
// Block averaging, rather than sampling 1 pixel, so noise and anti-aliasing
// don't look like a "change".
std::vector<uint8_t> ChangeDetector::make_thumbnail(const Frame& frame) const {
    const int tw = config_.thumb_width;
    const int th = config_.thumb_height;
    std::vector<uint8_t> thumb(size_t(tw) * th, 0);
    if (frame.width <= 0 || frame.height <= 0) return thumb;

    for (int ty = 0; ty < th; ++ty) {
        const int y0 = ty * frame.height / th;
        const int y1 = std::max(y0 + 1, (ty + 1) * frame.height / th);
        for (int tx = 0; tx < tw; ++tx) {
            const int x0 = tx * frame.width / tw;
            const int x1 = std::max(x0 + 1, (tx + 1) * frame.width / tw);
            uint64_t acc = 0;
            uint32_t count = 0;
            for (int y = y0; y < y1 && y < frame.height; ++y) {
                const uint8_t* row = frame.bgra.data() + size_t(y) * frame.stride;
                for (int x = x0; x < x1 && x < frame.width; ++x) {
                    const uint8_t* px = row + size_t(x) * 4;
                    // Approximate luminance (BT.601) in integers: 0.114 B + 0.587 G + 0.299 R
                    acc += (29u * px[0] + 150u * px[1] + 77u * px[2]) >> 8;
                    ++count;
                }
            }
            thumb[size_t(ty) * tw + tx] = count ? uint8_t(acc / count) : 0;
        }
    }
    return thumb;
}

// State machine: counts still frames; on reaching settle_frames, fires only if
// the stable image differs from the one that fired last time.
bool ChangeDetector::feed(const Frame& frame) {
    std::vector<uint8_t> thumb = make_thumbnail(frame);

    if (previous_.empty()) {
        previous_ = std::move(thumb);
        still_count_ = 1;
        return false;
    }

    const float diff_prev = mean_abs_diff(thumb, previous_);
    still_count_ = (diff_prev <= config_.still_threshold) ? still_count_ + 1 : 1;
    previous_ = thumb;

    if (still_count_ != config_.settle_frames) return false; // fires only once per still period

    const bool first_fire = last_fired_.empty();
    if (!first_fire && mean_abs_diff(thumb, last_fired_) < config_.change_threshold) return false;

    last_fired_ = std::move(thumb);
    return true;
}

} // namespace subquest
