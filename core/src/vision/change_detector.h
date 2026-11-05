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
    int32_t thumb_width = 96;        // wide, short thumbnail: dialogue boxes look like that
    int32_t thumb_height = 24;
    float change_threshold = 0.02f;  // mean difference (0..1) that counts as "changed"
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

private:
    std::vector<uint8_t> make_thumbnail(const Frame& frame) const; // shrinks the frame to thumb_width x thumb_height grayscale

    ChangeDetectorConfig config_;
    std::vector<uint8_t> previous_;  // thumbnail of the previous frame
    std::vector<uint8_t> last_fired_; // thumbnail of the last frame that fired OCR
    uint32_t still_count_ = 0;       // still frames in a row
};

} // namespace subquest
