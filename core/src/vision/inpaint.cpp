#include "vision/inpaint.h"

#include <algorithm>
#include <climits>
#include <cmath>

namespace subquest {

namespace {

// Squared distance between a BGRA pixel and a 0xRRGGBB color.
int distance2(const uint8_t* px, uint32_t rgb) {
    const int db = int(px[0]) - int(rgb & 0xFF);
    const int dg = int(px[1]) - int((rgb >> 8) & 0xFF);
    const int dr = int(px[2]) - int((rgb >> 16) & 0xFF);
    return db * db + dg * dg + dr * dr;
}

// Box `r` grown by `by` pixels on every side, kept inside `bounds`.
Rect grown(Rect r, int32_t by, Rect bounds) {
    const int32_t x0 = std::max(bounds.x, r.x - by);
    const int32_t y0 = std::max(bounds.y, r.y - by);
    const int32_t x1 = std::min(bounds.x + bounds.width, r.x + r.width + by);
    const int32_t y1 = std::min(bounds.y + bounds.height, r.y + r.height + by);
    return Rect{x0, y0, std::max(0, x1 - x0), std::max(0, y1 - y0)};
}

// Widens the mask by `radius` pixels (square neighborhood), done as two
// passes (rows, then columns) so it stays cheap for larger radii.
void dilate(std::vector<uint8_t>& mask, int32_t width, int32_t height, int32_t radius) {
    if (radius <= 0) return;
    std::vector<uint8_t> rows(mask.size(), 0);
    for (int32_t y = 0; y < height; ++y) {
        const uint8_t* in = mask.data() + size_t(y) * width;
        uint8_t* out = rows.data() + size_t(y) * width;
        for (int32_t x = 0; x < width; ++x) {
            if (!in[x]) continue;
            for (int32_t k = std::max(0, x - radius); k <= std::min(width - 1, x + radius); ++k) out[k] = 255;
        }
    }
    std::fill(mask.begin(), mask.end(), uint8_t(0));
    for (int32_t y = 0; y < height; ++y) {
        const uint8_t* in = rows.data() + size_t(y) * width;
        for (int32_t x = 0; x < width; ++x) {
            if (!in[x]) continue;
            for (int32_t k = std::max(0, y - radius); k <= std::min(height - 1, y + radius); ++k)
                mask[size_t(k) * width + x] = 255;
        }
    }
}

// One level of the pyramid: color sums and how many known pixels went into them.
struct Level {
    int32_t width = 0;
    int32_t height = 0;
    std::vector<float> color;  // 3 floats (B, G, R) per pixel, already averaged
    std::vector<uint8_t> known; // 1 where the color is real (or averaged from real ones)
};

// Half-size level: each pixel is the average of the known pixels of its 2x2 block.
Level halve(const Level& in) {
    Level out;
    out.width = std::max(1, (in.width + 1) / 2);
    out.height = std::max(1, (in.height + 1) / 2);
    out.color.assign(size_t(out.width) * out.height * 3, 0.0f);
    out.known.assign(size_t(out.width) * out.height, 0);
    for (int32_t y = 0; y < out.height; ++y) {
        for (int32_t x = 0; x < out.width; ++x) {
            float sum[3] = {0, 0, 0};
            int count = 0;
            for (int32_t dy = 0; dy < 2; ++dy) {
                for (int32_t dx = 0; dx < 2; ++dx) {
                    const int32_t sx = x * 2 + dx;
                    const int32_t sy = y * 2 + dy;
                    if (sx >= in.width || sy >= in.height) continue;
                    const size_t i = size_t(sy) * in.width + sx;
                    if (!in.known[i]) continue;
                    for (int c = 0; c < 3; ++c) sum[c] += in.color[i * 3 + c];
                    ++count;
                }
            }
            if (count == 0) continue;
            const size_t o = size_t(y) * out.width + x;
            for (int c = 0; c < 3; ++c) out.color[o * 3 + c] = sum[c] / float(count);
            out.known[o] = 1;
        }
    }
    return out;
}

// Fills the unknown pixels of `fine` from `coarse` (bilinear), then marks them known.
void fill_from(Level& fine, const Level& coarse) {
    for (int32_t y = 0; y < fine.height; ++y) {
        for (int32_t x = 0; x < fine.width; ++x) {
            const size_t i = size_t(y) * fine.width + x;
            if (fine.known[i]) continue;
            // Position of this pixel's center in the coarse level.
            const float cx = std::clamp((x + 0.5f) / 2.0f - 0.5f, 0.0f, float(coarse.width - 1));
            const float cy = std::clamp((y + 0.5f) / 2.0f - 0.5f, 0.0f, float(coarse.height - 1));
            const int32_t x0 = int32_t(cx);
            const int32_t y0 = int32_t(cy);
            const int32_t x1 = std::min(x0 + 1, coarse.width - 1);
            const int32_t y1 = std::min(y0 + 1, coarse.height - 1);
            const float fx = cx - float(x0);
            const float fy = cy - float(y0);
            for (int c = 0; c < 3; ++c) {
                const auto at = [&](int32_t px, int32_t py) { return coarse.color[(size_t(py) * coarse.width + px) * 3 + c]; };
                const float top = at(x0, y0) * (1 - fx) + at(x1, y0) * fx;
                const float bottom = at(x0, y1) * (1 - fx) + at(x1, y1) * fx;
                fine.color[i * 3 + c] = top * (1 - fy) + bottom * fy;
            }
        }
    }
    std::fill(fine.known.begin(), fine.known.end(), uint8_t(1));
}

} // namespace

// Letters of each line: inside its box (a little larger, for letters the OCR
// box clips), the pixels closer to its text color than to its background,
// then widened by a fifth of the line height for edges, shadows and outlines.
std::vector<uint8_t> text_mask(const Frame& frame, Rect region, const std::vector<OcrLine>& lines,
                               const std::vector<Rect>& pictures) {
    std::vector<uint8_t> mask(size_t(std::max(0, region.width)) * std::max(0, region.height), 0);
    if (region.empty()) return mask;
    int32_t widest_pad = 1;
    for (const OcrLine& line : lines) {
        if (line.box.empty()) continue;
        const int32_t pad = std::max(2, line.box.height / 6);
        // Wide enough to swallow the shadow or outline many games draw around
        // their letters: left behind, it would fill the hole with a dark letter.
        widest_pad = std::max(widest_pad, std::max(2, line.box.height / 5));
        const Rect box = grown(line.box, pad, region);
        for (int32_t y = box.y; y < box.y + box.height; ++y) {
            const uint8_t* row = frame.bgra.data() + size_t(y) * frame.stride;
            uint8_t* out = mask.data() + size_t(y - region.y) * region.width;
            for (int32_t x = box.x; x < box.x + box.width; ++x) {
                const uint8_t* px = row + size_t(x) * 4;
                if (distance2(px, line.text_rgb) < distance2(px, line.background_rgb)) out[x - region.x] = 255;
            }
        }
    }
    // Pictures go whole (their colors are not the text's), with a pixel of margin.
    for (const Rect& p : pictures) {
        const Rect box = grown(p, 1, region);
        for (int32_t y = box.y; y < box.y + box.height; ++y)
            std::fill_n(mask.begin() + size_t(y - region.y) * region.width + (box.x - region.x), box.width, uint8_t(255));
    }
    dilate(mask, region.width, region.height, widest_pad);
    return mask;
}

// Pyramid fill: halve until every pixel is known (or one pixel is left), then
// come back up filling holes from the level above, and smooth the filled
// pixels a few times so no blocks show.
void fill_holes(std::vector<uint8_t>& bgra, int32_t width, int32_t height, const std::vector<uint8_t>& mask) {
    if (width <= 0 || height <= 0) return;
    std::vector<Level> levels(1);
    Level& base = levels[0];
    base.width = width;
    base.height = height;
    base.color.resize(size_t(width) * height * 3);
    base.known.resize(size_t(width) * height);
    bool any_hole = false;
    bool any_known = false;
    for (size_t i = 0; i < size_t(width) * height; ++i) {
        for (int c = 0; c < 3; ++c) base.color[i * 3 + c] = bgra[i * 4 + c];
        base.known[i] = mask[i] ? 0 : 1;
        any_hole |= mask[i] != 0;
        any_known |= mask[i] == 0;
    }
    if (!any_hole || !any_known) return; // nothing to fill, or nothing to fill it from

    while (true) {
        const Level& last = levels.back();
        if (std::all_of(last.known.begin(), last.known.end(), [](uint8_t k) { return k != 0; })) break;
        if (last.width == 1 && last.height == 1) break;
        levels.push_back(halve(last));
    }
    for (size_t l = levels.size() - 1; l > 0; --l) fill_from(levels[l - 1], levels[l]);

    // Smoothing inside the holes only: each filled pixel takes the average of
    // its four neighbors, a few times (the known pixels never change).
    std::vector<float>& color = levels[0].color;
    for (int pass = 0; pass < 6; ++pass) {
        for (int32_t y = 0; y < height; ++y) {
            for (int32_t x = 0; x < width; ++x) {
                const size_t i = size_t(y) * width + x;
                if (!mask[i]) continue;
                float sum[3] = {0, 0, 0};
                int count = 0;
                const int32_t nx[4] = {x - 1, x + 1, x, x};
                const int32_t ny[4] = {y, y, y - 1, y + 1};
                for (int k = 0; k < 4; ++k) {
                    if (nx[k] < 0 || ny[k] < 0 || nx[k] >= width || ny[k] >= height) continue;
                    const size_t j = size_t(ny[k]) * width + nx[k];
                    for (int c = 0; c < 3; ++c) sum[c] += color[j * 3 + c];
                    ++count;
                }
                for (int c = 0; c < 3; ++c) color[i * 3 + c] = sum[c] / float(count);
            }
        }
    }
    for (size_t i = 0; i < size_t(width) * height; ++i) {
        if (!mask[i]) continue;
        for (int c = 0; c < 3; ++c) bgra[i * 4 + c] = uint8_t(std::clamp(std::lround(color[i * 3 + c]), 0L, 255L));
        bgra[i * 4 + 3] = 255;
    }
}

// Cuts the region out of the frame, masks the letters and fills them in.
bool remove_text(const Frame& frame, const std::vector<OcrLine>& lines, bool whole, CleanImage& out,
                 const std::vector<Rect>& pictures) {
    if (lines.empty() || frame.width <= 0 || frame.height <= 0) return false;
    const Rect bounds{0, 0, frame.width, frame.height};
    Rect region = bounds;
    if (!whole) {
        // Around the lines, one line height of margin: room for the patches
        // and for a translation that runs a little longer.
        int32_t x0 = INT32_MAX, y0 = INT32_MAX, x1 = INT32_MIN, y1 = INT32_MIN, tallest = 0;
        for (const OcrLine& line : lines) {
            x0 = std::min(x0, line.box.x);
            y0 = std::min(y0, line.box.y);
            x1 = std::max(x1, line.box.x + line.box.width);
            y1 = std::max(y1, line.box.y + line.box.height);
            tallest = std::max(tallest, line.box.height);
        }
        for (const Rect& p : pictures) {
            x0 = std::min(x0, p.x);
            y0 = std::min(y0, p.y);
            x1 = std::max(x1, p.x + p.width);
            y1 = std::max(y1, p.y + p.height);
        }
        region = grown(Rect{x0, y0, x1 - x0, y1 - y0}, tallest, bounds);
    }
    if (region.empty()) return false;

    out.rect = region;
    out.bgra.resize(size_t(region.width) * region.height * 4);
    for (int32_t y = 0; y < region.height; ++y) {
        const uint8_t* src = frame.bgra.data() + size_t(region.y + y) * frame.stride + size_t(region.x) * 4;
        std::copy(src, src + size_t(region.width) * 4, out.bgra.data() + size_t(y) * region.width * 4);
    }
    const std::vector<uint8_t> mask = text_mask(frame, region, lines, pictures);
    fill_holes(out.bgra, region.width, region.height, mask);

    // Only the filled holes are kept, fading out over a few pixels; the rest is
    // see-through, so the overlay shows the live game there (an animated
    // scene behind a lyric no longer freezes in a box around it).
    int32_t tallest = 0;
    for (const OcrLine& line : lines) tallest = std::max(tallest, line.box.height);
    const int32_t feather = std::max(1, tallest / 10);
    std::vector<uint8_t> near = mask, far;
    dilate(near, region.width, region.height, feather);
    far = near;
    dilate(far, region.width, region.height, feather);
    for (size_t i = 0; i < mask.size(); ++i)
        out.bgra[i * 4 + 3] = mask[i] ? 255 : near[i] ? 170 : far[i] ? 85 : 0;
    return true;
}

} // namespace subquest
