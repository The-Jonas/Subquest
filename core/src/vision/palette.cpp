#include "vision/palette.h"

#include <algorithm>
#include <cmath>

namespace subquest {

namespace {

constexpr int kBorder = 3;               // border ring thickness, in pixels
constexpr float kInkThreshold = 0.5f;    // a pixel is "ink" if at least this far from the background, relative to the farthest one
constexpr int kMinGap = 40;              // below this RGB distance text and background can't be told apart: black/white

// Packs channels into 0xRRGGBB.
uint32_t pack(uint32_t r, uint32_t g, uint32_t b) { return (r << 16) | (g << 8) | b; }

// Channel helpers for 0xRRGGBB.
int red(uint32_t c) { return int((c >> 16) & 0xFF); }
int green(uint32_t c) { return int((c >> 8) & 0xFF); }
int blue(uint32_t c) { return int(c & 0xFF); }

// Reads one BGRA pixel as 0xRRGGBB.
uint32_t pixel(const Frame& f, int x, int y) {
    const uint8_t* p = f.bgra.data() + size_t(y) * f.stride + size_t(x) * 4;
    return pack(p[2], p[1], p[0]);
}

// Per-channel median, robust against a few odd pixels (a corner ornament, a sparkle).
uint32_t median(std::vector<uint32_t>& colors) {
    if (colors.empty()) return 0;
    const size_t mid = colors.size() / 2;
    auto channel = [&](int (*get)(uint32_t)) {
        std::vector<int> values(colors.size());
        std::transform(colors.begin(), colors.end(), values.begin(), get);
        std::nth_element(values.begin(), values.begin() + long(mid), values.end());
        return uint32_t(values[mid]);
    };
    return pack(channel(red), channel(green), channel(blue));
}

// Squared RGB distance; enough to rank pixels by "how unlike the background".
int distance2(uint32_t a, uint32_t b) {
    const int dr = red(a) - red(b), dg = green(a) - green(b), db = blue(a) - blue(b);
    return dr * dr + dg * dg + db * db;
}

} // namespace

static uint32_t ink_color(const Frame& frame, const std::vector<Rect>& areas, uint32_t background);

// Approximate relative luminance with the Rec. 709 weights on gamma-encoded values.
float luminance(uint32_t rgb) {
    return (0.2126f * float(red(rgb)) + 0.7152f * float(green(rgb)) + 0.0722f * float(blue(rgb))) / 255.0f;
}

// See the header: border median for the background, farthest pixels inside the
// lines for the text, and a contrast safety net.
TextColors sample_colors(const Frame& frame, const std::vector<OcrLine>& lines) {
    TextColors out;
    if (frame.width <= 2 * kBorder || frame.height <= 2 * kBorder) return out;

    std::vector<uint32_t> border;
    for (int y = 0; y < frame.height; ++y) {
        const bool edge_row = y < kBorder || y >= frame.height - kBorder;
        for (int x = 0; x < frame.width; ++x) {
            const bool edge_col = x < kBorder || x >= frame.width - kBorder;
            if (edge_row || edge_col) border.push_back(pixel(frame, x, y));
        }
    }
    out.background = median(border);

    // Text candidates: inside the OCR line boxes, or the whole frame if there are none.
    std::vector<Rect> areas;
    for (const OcrLine& line : lines) areas.push_back(line.box);
    if (areas.empty()) areas.push_back({0, 0, frame.width, frame.height});
    out.text = ink_color(frame, areas, out.background);
    return out;
}

// Colors for a single line: ring around the box for the background, ink inside for the text.
TextColors sample_line_colors(const Frame& frame, Rect box) {
    TextColors out;
    constexpr int kGap = 2;  // skip anti-aliased edges right next to the letters
    constexpr int kRing = 3; // ring thickness
    const int x0 = std::max(0, box.x - kGap - kRing), x1 = std::min(frame.width, box.x + box.width + kGap + kRing);
    const int y0 = std::max(0, box.y - kGap - kRing), y1 = std::min(frame.height, box.y + box.height + kGap + kRing);
    const int ix0 = box.x - kGap, ix1 = box.x + box.width + kGap;
    const int iy0 = box.y - kGap, iy1 = box.y + box.height + kGap;

    std::vector<uint32_t> ring;
    for (int y = y0; y < y1; ++y)
        for (int x = x0; x < x1; ++x)
            if (x < ix0 || x >= ix1 || y < iy0 || y >= iy1) ring.push_back(pixel(frame, x, y));
    if (ring.empty()) return out;
    out.background = median(ring);
    out.text = ink_color(frame, {box}, out.background);
    return out;
}

// Average color of the "ink" pixels inside the given areas: the ones at least
// kInkThreshold as far from the background as the farthest pixel. Falls back
// to black/white when the result is too close to the background to read.
static uint32_t ink_color(const Frame& frame, const std::vector<Rect>& areas, uint32_t background) {
    std::vector<std::pair<int, uint32_t>> candidates;
    auto collect = [&](int x0, int y0, int x1, int y1) {
        x0 = std::clamp(x0, 0, frame.width);
        x1 = std::clamp(x1, 0, frame.width);
        y0 = std::clamp(y0, 0, frame.height);
        y1 = std::clamp(y1, 0, frame.height);
        for (int y = y0; y < y1; ++y)
            for (int x = x0; x < x1; ++x) {
                const uint32_t c = pixel(frame, x, y);
                candidates.emplace_back(distance2(c, background), c);
            }
    };
    for (const Rect& a : areas) collect(a.x, a.y, a.x + a.width, a.y + a.height);
    const uint32_t fallback = luminance(background) > 0.5f ? 0x000000 : 0xFFFFFF;
    if (candidates.empty()) return fallback;

    // Relative threshold instead of a fixed share: works whether the ink covers
    // 5% of the area (thin font, whole frame) or 40% (bold font, tight box).
    // Distances are squared, so the threshold is squared too.
    int farthest = 0;
    for (const auto& [d, c] : candidates) farthest = std::max(farthest, d);
    const float cut = float(farthest) * kInkThreshold * kInkThreshold;
    uint64_t r = 0, g = 0, b = 0, count = 0;
    for (const auto& [d, c] : candidates) {
        if (float(d) < cut) continue;
        r += uint64_t(red(c));
        g += uint64_t(green(c));
        b += uint64_t(blue(c));
        ++count;
    }
    if (count == 0) return fallback;
    const uint32_t ink = pack(uint32_t(r / count), uint32_t(g / count), uint32_t(b / count));

    // Safety net: only when text and background really can't be told apart,
    // pick whichever of white/black reads better on that background. A dim
    // color is still the game's (Hades' purple flavor text on a dark purple
    // scroll is only 0.14 apart in luminance): replacing it drew the
    // translation in white and left the original under the cleaned picture.
    if (distance2(ink, background) < kMinGap * kMinGap) return fallback;
    return ink;
}

} // namespace subquest
