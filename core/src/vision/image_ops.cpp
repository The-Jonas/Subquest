#include "vision/image_ops.h"

#include <algorithm>

namespace subquest {

// Box-filter downscale: each output pixel is the average of the matching input
// block. The same pass swaps B and R, since the browser canvas expects RGBA.
RgbaImage downscale_to_rgba(const Frame& src, int32_t max_width) {
    RgbaImage out;
    if (src.width <= 0 || src.height <= 0 || max_width <= 0) return out;

    out.width = std::min(src.width, max_width);
    out.height = std::max(1, int32_t(int64_t(src.height) * out.width / src.width));
    out.pixels.resize(size_t(out.width) * out.height * 4);

    for (int32_t oy = 0; oy < out.height; ++oy) {
        const int32_t y0 = int32_t(int64_t(oy) * src.height / out.height);
        const int32_t y1 = std::max(y0 + 1, int32_t(int64_t(oy + 1) * src.height / out.height));
        for (int32_t ox = 0; ox < out.width; ++ox) {
            const int32_t x0 = int32_t(int64_t(ox) * src.width / out.width);
            const int32_t x1 = std::max(x0 + 1, int32_t(int64_t(ox + 1) * src.width / out.width));
            uint32_t b = 0, g = 0, r = 0, count = 0;
            for (int32_t y = y0; y < y1; ++y) {
                const uint8_t* row = src.bgra.data() + size_t(y) * src.stride;
                for (int32_t x = x0; x < x1; ++x) {
                    const uint8_t* px = row + size_t(x) * 4;
                    b += px[0];
                    g += px[1];
                    r += px[2];
                    ++count;
                }
            }
            uint8_t* dst = out.pixels.data() + (size_t(oy) * out.width + ox) * 4;
            dst[0] = uint8_t(r / count);
            dst[1] = uint8_t(g / count);
            dst[2] = uint8_t(b / count);
            dst[3] = 255; // the capture has no meaningful transparency
        }
    }
    return out;
}

} // namespace subquest
