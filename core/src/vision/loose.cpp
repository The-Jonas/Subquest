#include "vision/loose.h"

#include <algorithm>
#include <climits>

namespace subquest {

namespace {

constexpr float MIN_TALL = 0.4f;  // a leftover is at least this share of the usual line height (a "?" is)
constexpr float MAX_TALL = 1.8f;  // and at most this much (taller: artwork, not text)
constexpr float JOIN_GAP = 1.5f;  // pieces on one row closer than this many line heights are one leftover
constexpr int MIN_GAP = 40;       // ink must stand this far (RGB) from the backgrounds
constexpr float STRONG = 0.6f;    // ...and somewhere this share of the letters' usual contrast
constexpr float MAX_FILL = 0.6f;  // letters fill at most this share of their box

// Squared RGB distance between a BGRA pixel and a 0xRRGGBB color.
int distance2(const uint8_t* px, uint32_t rgb) {
    const int db = int(px[0]) - int(rgb & 0xFF);
    const int dg = int(px[1]) - int((rgb >> 8) & 0xFF);
    const int dr = int(px[2]) - int((rgb >> 16) & 0xFF);
    return db * db + dg * dg + dr * dr;
}

// The rectangle grown by `by` on every side.
Rect grown(Rect r, int32_t by) { return {r.x - by, r.y - by, r.width + 2 * by, r.height + 2 * by}; }

bool inside(const Rect& r, int32_t x, int32_t y) { return x >= r.x && y >= r.y && x < r.x + r.width && y < r.y + r.height; }

struct Piece {
    int32_t x0, y0, x1, y1; // inclusive
    size_t count;
};

} // namespace

// Ink of the text's colors left outside the lines, split in connected pieces,
// then pieces of one row joined; what has a line's size is kept.
std::vector<Rect> find_loose_text(const Frame& frame, const std::vector<OcrLine>& lines, const std::vector<Rect>& pictures) {
    std::vector<Rect> out;
    if (lines.empty() || frame.width <= 0 || frame.height <= 0) return out;
    std::vector<int32_t> heights;
    int32_t x0 = INT32_MAX, y0 = INT32_MAX, x1 = INT32_MIN, y1 = INT32_MIN;
    for (const OcrLine& l : lines) {
        heights.push_back(l.box.height);
        x0 = std::min(x0, l.box.x), y0 = std::min(y0, l.box.y);
        x1 = std::max(x1, l.box.x + l.box.width), y1 = std::max(y1, l.box.y + l.box.height);
    }
    std::nth_element(heights.begin(), heights.begin() + long(heights.size() / 2), heights.end());
    const int32_t usual = std::max(4, heights[heights.size() / 2]);
    // Where text can be: the lines' span, a line beyond it on every side.
    const int32_t sx0 = std::max(0, x0 - usual), sy0 = std::max(0, y0 - usual);
    const int32_t sx1 = std::min(frame.width, x1 + usual), sy1 = std::min(frame.height, y1 + usual);
    const Rect span{sx0, sy0, sx1 - sx0, sy1 - sy0};
    if (span.width <= 0 || span.height <= 0) return out;

    std::vector<Rect> taken;
    for (const OcrLine& l : lines) taken.push_back(grown(l.box, std::max(2, l.box.height / 3)));
    for (const Rect& p : pictures) taken.push_back(grown(p, 2));

    const int32_t w = span.width, h = span.height;
    std::vector<uint8_t> ink(size_t(w) * h, 0);
    for (int32_t y = 0; y < h; ++y) {
        const uint8_t* row = frame.bgra.data() + size_t(span.y + y) * frame.stride;
        for (int32_t x = 0; x < w; ++x) {
            const int32_t fx = span.x + x, fy = span.y + y;
            if (std::any_of(taken.begin(), taken.end(), [&](const Rect& r) { return inside(r, fx, fy); })) continue;
            const uint8_t* px = row + size_t(fx) * 4;
            int to_text = INT_MAX, to_back = INT_MAX;
            for (const OcrLine& l : lines) {
                to_text = std::min(to_text, distance2(px, l.text_rgb));
                to_back = std::min(to_back, distance2(px, l.background_rgb));
            }
            if (to_text < to_back && to_back >= MIN_GAP * MIN_GAP) ink[size_t(y) * w + x] = 1;
        }
    }

    // Connected pieces (8 neighbors); specks left out.
    std::vector<Piece> pieces;
    std::vector<uint8_t> seen(ink.size(), 0);
    std::vector<int32_t> stack;
    for (int32_t start = 0; start < w * h; ++start) {
        if (!ink[start] || seen[start]) continue;
        Piece p{w, h, -1, -1, 0};
        stack.assign(1, start);
        seen[start] = 1;
        while (!stack.empty()) {
            const int32_t i = stack.back();
            stack.pop_back();
            const int32_t x = i % w, y = i / w;
            p.x0 = std::min(p.x0, x), p.y0 = std::min(p.y0, y), p.x1 = std::max(p.x1, x), p.y1 = std::max(p.y1, y);
            ++p.count;
            for (int32_t dy = -1; dy <= 1; ++dy)
                for (int32_t dx = -1; dx <= 1; ++dx) {
                    const int32_t nx = x + dx, ny = y + dy;
                    if (nx < 0 || ny < 0 || nx >= w || ny >= h) continue;
                    const int32_t j = ny * w + nx;
                    if (ink[j] && !seen[j]) {
                        seen[j] = 1;
                        stack.push_back(j);
                    }
                }
        }
        if (p.count >= 4) pieces.push_back(p);
    }

    // Pieces of one row ("·", "?", "?", "?") joined.
    std::sort(pieces.begin(), pieces.end(), [](const Piece& a, const Piece& b) { return a.x0 < b.x0; });
    const int32_t join = int32_t(usual * JOIN_GAP);
    std::vector<Piece> rows;
    for (const Piece& p : pieces) {
        bool merged = false;
        for (Piece& r : rows) {
            const bool same_row = p.y0 <= r.y1 && p.y1 >= r.y0;
            if (same_row && p.x0 - r.x1 <= join) {
                r.x0 = std::min(r.x0, p.x0), r.y0 = std::min(r.y0, p.y0);
                r.x1 = std::max(r.x1, p.x1), r.y1 = std::max(r.y1, p.y1);
                r.count += p.count;
                merged = true;
                break;
            }
        }
        if (!merged) rows.push_back(p);
    }
    // How strong the area's letters are against their background: a leftover
    // must reach that much somewhere (a soft glow of the scenery doesn't).
    std::vector<float> contrasts;
    for (const OcrLine& l : lines) {
        const uint8_t t[4] = {uint8_t(l.text_rgb & 0xFF), uint8_t((l.text_rgb >> 8) & 0xFF), uint8_t((l.text_rgb >> 16) & 0xFF), 255};
        contrasts.push_back(float(distance2(t, l.background_rgb)));
    }
    std::sort(contrasts.begin(), contrasts.end());
    const float strong = contrasts[contrasts.size() / 2] * STRONG * STRONG; // squared
    for (const Piece& r : rows) {
        const int32_t rw = r.x1 - r.x0 + 1, rh = r.y1 - r.y0 + 1;
        // Letters leave room between their strokes; a filled block is a shape, not text.
        if (float(r.count) > float(rw) * float(rh) * MAX_FILL) continue;
        int best = 0;
        for (int32_t y = r.y0; y <= r.y1; ++y)
            for (int32_t x = r.x0; x <= r.x1; ++x) {
                if (!ink[size_t(y) * w + x]) continue;
                const uint8_t* px = frame.bgra.data() + size_t(span.y + y) * frame.stride + size_t(span.x + x) * 4;
                int to_back = INT_MAX;
                for (const OcrLine& l : lines) to_back = std::min(to_back, distance2(px, l.background_rgb));
                best = std::max(best, to_back);
            }
        if (float(best) < strong) continue;
        if (rh < usual * MIN_TALL || rh > usual * MAX_TALL || rw < usual * MIN_TALL) continue;
        // Cut by the edge of where text can be: part of something bigger (the box's frame, scenery).
        if (r.x0 == 0 || r.y0 == 0 || r.x1 == w - 1 || r.y1 == h - 1) continue;
        const Rect box{span.x + r.x0, span.y + r.y0, rw, rh};
        // Within the lines' own left and right edges: text lines up with
        // them; the tip of an ornament beside them doesn't.
        if (box.x < x0 - usual / 3 || box.x + box.width > x1 + usual / 3) continue;
        out.push_back(grown(box, 1));
    }
    return out;
}

} // namespace subquest
