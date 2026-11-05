// image_ops.h — simple image operations with no dependencies (portable and testable).
#pragma once

#include <cstdint>
#include <vector>

#include "common/types.h"

namespace subquest {

struct RgbaImage {
    std::vector<uint8_t> pixels; // RGBA 8 bits, contiguous rows (stride = width * 4)
    int32_t width = 0;
    int32_t height = 0;
};

RgbaImage downscale_to_rgba(const Frame& src, int32_t max_width); // shrinks keeping aspect ratio (never enlarges) and swaps BGRA -> RGBA
bool fit_within(const Frame& src, int32_t max_dim, Frame& out);    // shrinks (still BGRA) so no side exceeds max_dim; false = already fits

} // namespace subquest
