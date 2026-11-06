// change_detector.h — decides WHEN it is worth running OCR.
//
// Idea: shrink the frame to a grayscale thumbnail and compare it with the
// previous one. Fires only when the text CHANGED since the last fire and then
// stayed STILL for a few frames (the typewriter effect is over).
#pragma once

#include <cstdint>
#include <vector>

#include "common/types.h"

namespace subquest {

struct ChangeDetectorConfig {
    // Fine enough that a letter covers about one thumbnail pixel in a dialogue
    // box: at 96 x 24 two different sentences of the same length blurred into
    // the same gray smear, and the second one was taken for the first.
    int32_t thumb_width = 256;
    int32_t thumb_height = 64;
    float change_threshold = 0.02f;  // mean difference (0..1) that counts as "changed"...
    uint8_t pixel_delta = 24;        // ...or this many gray levels of difference...
    float changed_share = 0.01f;     // ...in at least this share of the pixels (other words, same length)
    float still_threshold = 0.004f;  // difference below this counts as "still"
    uint32_t settle_frames = 3;      // still frames in a row before firing
};

class ChangeDetector {
public:
    explicit ChangeDetector(ChangeDetectorConfig config = {});

    bool feed(const Frame& frame); // true => stable frame, different from the last fire: run OCR
    void reset();                  // forgets history (new session or region changed)

    // Exposed for tests: mean absolute difference between two thumbnails (0..1).
    static float mean_abs_diff(const std::vector<uint8_t>& a, const std::vector<uint8_t>& b);
    // Exposed for tests: share of pixels (0..1) that differ by more than `delta` gray levels.
    static float changed_pixels(const std::vector<uint8_t>& a, const std::vector<uint8_t>& b, uint8_t delta);

private:
    std::vector<uint8_t> make_thumbnail(const Frame& frame) const; // shrinks the frame to thumb_width x thumb_height grayscale
    bool differs(const std::vector<uint8_t>& a, const std::vector<uint8_t>& b) const; // other text than last time?

    ChangeDetectorConfig config_;
    std::vector<uint8_t> previous_;  // thumbnail of the previous frame
    std::vector<uint8_t> last_fired_; // thumbnail of the last frame that fired OCR
    uint32_t still_count_ = 0;       // still frames in a row
};

} // namespace subquest
