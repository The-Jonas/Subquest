#include "vision/image_ops.h"

#include <algorithm>

namespace subquest {

namespace {

// Box-filter resample of `src` into an `out_w` x `out_h` buffer with tight rows:
// each output pixel is the average of the matching input block. With
// `swap_rb`, B and R are swapped in the same pass (BGRA -> RGBA).
void box_resample(const Frame& src, int32_t out_w, int32_t out_h, bool swap_rb, uint8_t* out) {
    for (int32_t oy = 0; oy < out_h; ++oy) {
        const int32_t y0 = int32_t(int64_t(oy) * src.height / out_h);
        const int32_t y1 = std::max(y0 + 1, int32_t(int64_t(oy + 1) * src.height / out_h));
        for (int32_t ox = 0; ox < out_w; ++ox) {
            const int32_t x0 = int32_t(int64_t(ox) * src.width / out_w);
            const int32_t x1 = std::max(x0 + 1, int32_t(int64_t(ox + 1) * src.width / out_w));
            uint32_t c0 = 0, c1 = 0, c2 = 0, count = 0;
            for (int32_t y = y0; y < y1; ++y) {
                const uint8_t* row = src.bgra.data() + size_t(y) * src.stride;
                for (int32_t x = x0; x < x1; ++x) {
                    const uint8_t* px = row + size_t(x) * 4;
                    c0 += px[0];
                    c1 += px[1];
                    c2 += px[2];
                    ++count;
                }
            }
            uint8_t* dst = out + (size_t(oy) * out_w + ox) * 4;
            dst[0] = uint8_t((swap_rb ? c2 : c0) / count);
            dst[1] = uint8_t(c1 / count);
            dst[2] = uint8_t((swap_rb ? c0 : c2) / count);
            dst[3] = 255; // the capture has no meaningful transparency
        }
    }
}

} // namespace

// Shrinks to at most `max_width` wide, keeping the aspect ratio, as RGBA for the browser canvas.
RgbaImage downscale_to_rgba(const Frame& src, int32_t max_width) {
    RgbaImage out;
    if (src.width <= 0 || src.height <= 0 || max_width <= 0) return out;

    out.width = std::min(src.width, max_width);
    out.height = std::max(1, int32_t(int64_t(src.height) * out.width / src.width));
    out.pixels.resize(size_t(out.width) * out.height * 4);
    box_resample(src, out.width, out.height, true, out.pixels.data());
    return out;
}

// Shrinks the frame (still BGRA) so neither side exceeds `max_dim`. Returns
// false, leaving `out` untouched, when the frame already fits.
bool fit_within(const Frame& src, int32_t max_dim, Frame& out) {
    if (src.width <= max_dim && src.height <= max_dim) return false;

    const double scale = double(max_dim) / double(std::max(src.width, src.height));
    out.width = std::max(1, int32_t(src.width * scale));
    out.height = std::max(1, int32_t(src.height * scale));
    out.stride = out.width * 4;
    out.bgra.resize(size_t(out.stride) * out.height);
    out.id = src.id;
    out.timestamp_us = src.timestamp_us;
    box_resample(src, out.width, out.height, false, out.bgra.data());
    return true;
}

// Copies a rectangle of the frame into a new tight frame. The crop keeps the
// frame's id and timestamp and records where it sits in the window.
bool crop(const Frame& src, Rect area, Frame& out) {
    const int32_t x0 = std::clamp(area.x, 0, src.width);
    const int32_t y0 = std::clamp(area.y, 0, src.height);
    const int32_t x1 = std::clamp(area.x + area.width, 0, src.width);
    const int32_t y1 = std::clamp(area.y + area.height, 0, src.height);
    if (x1 <= x0 || y1 <= y0) return false;

    out.width = x1 - x0;
    out.height = y1 - y0;
    out.stride = out.width * 4;
    out.bgra.resize(size_t(out.stride) * out.height);
    for (int32_t y = 0; y < out.height; ++y) {
        const uint8_t* row = src.bgra.data() + size_t(y0 + y) * src.stride + size_t(x0) * 4;
        std::copy(row, row + out.stride, out.bgra.data() + size_t(y) * out.stride);
    }
    out.id = src.id;
    out.timestamp_us = src.timestamp_us;
    out.origin_x = src.origin_x + x0;
    out.origin_y = src.origin_y + y0;
    out.source_width = src.source_width;
    out.source_height = src.source_height;
    return true;
}

} // namespace subquest
