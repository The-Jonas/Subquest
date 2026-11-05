// palette.h — the colors of a dialogue box, so the overlay can look like the game.
#pragma once

#include <cstdint>
#include <vector>

#include "common/types.h"

namespace subquest {

struct TextColors {
    uint32_t background = 0x000000; // 0xRRGGBB
    uint32_t text = 0xFFFFFF;       // 0xRRGGBB
};

// Background = median of the frame's border (the box frame around the text).
// Text = average of the pixels most different from the background inside the
// recognized lines (the letters). Falls back to black/white when the two end up
// too close to read.
TextColors sample_colors(const Frame& frame, const std::vector<OcrLine>& lines);

// Colors of one line: background from a thin ring just outside its box (what
// surrounds those letters), text from the ink inside it. This is what lets the
// overlay "erase" a line with the right color instead of a generic box.
TextColors sample_line_colors(const Frame& frame, Rect box);

float luminance(uint32_t rgb); // relative luminance 0..1 (sRGB-ish), used for the contrast check

} // namespace subquest
