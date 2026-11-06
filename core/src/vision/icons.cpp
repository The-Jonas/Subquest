#include "vision/icons.h"

#include <algorithm>
#include <climits>
#include <cctype>
#include <cmath>

namespace subquest {

namespace {

constexpr float OTHER_WORD = 0.30f;  // share of "other" pixels that makes a word a picture
constexpr float OTHER_GAP = 0.12f;   // ...and a gap between words (mostly background around it)
constexpr float TALL_WORD = 1.35f;   // a short word this much taller than the line's words: a drawn key
constexpr size_t SHORT_WORD = 3;     // characters at most for the "tall word" rule
constexpr size_t ICON_READING = 2;   // OCR reads a picture as junk this short ("A", "®", "(Y)" without brackets)
constexpr float WIDE_GAP = 0.9f;     // a gap wider than this share of the line height can hold a picture
constexpr float TALLER = 0.75f;      // pictures may reach this share of the line height above and below the letters
constexpr float OTHER_INK = 0.5f;    // short junk whose ink is mostly another color than the text's (a white note in a green line)
constexpr float FILLED_KEY = 0.6f;   // short junk whose box is at least this full is a key's face (letters fill 20-40%)
constexpr float EDGE_REACH = 3.0f;   // pictures before the first word / after the last: within this many line heights
constexpr float SMALLEST = 0.4f;     // a picture is at least this share of the line height, both ways

// RGB distance between a BGRA pixel and a 0xRRGGBB color.
float distance(const uint8_t* px, uint32_t rgb) {
    const float db = float(px[0]) - float(rgb & 0xFF);
    const float dg = float(px[1]) - float((rgb >> 8) & 0xFF);
    const float dr = float(px[2]) - float((rgb >> 16) & 0xFF);
    return std::sqrt(db * db + dg * dg + dr * dr);
}

// How far a BGRA pixel is from every mix of two colors (0xRRGGBB): about 0
// for a letter's anti-aliased edge (part text, part background), large for a
// color of its own (a green button, a white note in a green line).
float off_mix(const uint8_t* px, uint32_t a, uint32_t b) {
    const float p[3] = {float(px[2]), float(px[1]), float(px[0])};
    const float ca[3] = {float((a >> 16) & 0xFF), float((a >> 8) & 0xFF), float(a & 0xFF)};
    const float cb[3] = {float((b >> 16) & 0xFF), float((b >> 8) & 0xFF), float(b & 0xFF)};
    float axis2 = 0, along = 0;
    for (int i = 0; i < 3; ++i) {
        axis2 += (cb[i] - ca[i]) * (cb[i] - ca[i]);
        along += (p[i] - ca[i]) * (cb[i] - ca[i]);
    }
    const float t = std::clamp(along / std::max(axis2, 1.0f), 0.0f, 1.0f);
    float d2 = 0;
    for (int i = 0; i < 3; ++i) d2 += (p[i] - (ca[i] + (cb[i] - ca[i]) * t)) * (p[i] - (ca[i] + (cb[i] - ca[i]) * t));
    return std::sqrt(d2);
}

// Box clipped to the frame.
Rect clipped(Rect r, const Frame& frame) {
    const int32_t x0 = std::clamp(r.x, 0, frame.width);
    const int32_t y0 = std::clamp(r.y, 0, frame.height);
    const int32_t x1 = std::clamp(r.x + r.width, 0, frame.width);
    const int32_t y1 = std::clamp(r.y + r.height, 0, frame.height);
    return Rect{x0, y0, x1 - x0, y1 - y0};
}

// Pixels of a box: how many are neither the text nor the background color
// ("other"), and the box around the pixels that aren't background.
struct Look {
    float other = 0;  // share of "other" pixels: off every mix of the text and background colors
    float ink_other = 0; // the same, among the pixels that aren't background (the drawing's own colors)
    float filled = 0; // share of pixels that aren't background (a key's face fills its box; a letter doesn't)
    Rect drawn;       // around everything that isn't background (empty: nothing)
};

Look look_at(const Frame& frame, Rect box, uint32_t text, uint32_t background) {
    Look out;
    box = clipped(box, frame);
    if (box.empty()) return out;
    // Colors count as the text's or the background's when they are within a
    // third of the distance between the two (anti-aliased edges included).
    const uint8_t t[4] = {uint8_t(text & 0xFF), uint8_t((text >> 8) & 0xFF), uint8_t((text >> 16) & 0xFF), 255};
    const float contrast = std::max(60.0f, distance(t, background));
    const float close = contrast * 0.33f;
    // A letter's soft edge sits right on the text/background mix; a color of
    // its own (a green button on a gray line) stands well off it.
    const float off = std::max(30.0f, contrast * 0.15f);
    size_t others = 0;
    size_t drawn = 0;
    int32_t x0 = INT32_MAX, y0 = INT32_MAX, x1 = INT32_MIN, y1 = INT32_MIN;
    for (int32_t y = box.y; y < box.y + box.height; ++y) {
        const uint8_t* row = frame.bgra.data() + size_t(y) * frame.stride;
        for (int32_t x = box.x; x < box.x + box.width; ++x) {
            const uint8_t* px = row + size_t(x) * 4;
            const float to_bg = distance(px, background);
            if (to_bg < close) continue; // background
            ++drawn;
            x0 = std::min(x0, x);
            y0 = std::min(y0, y);
            x1 = std::max(x1, x + 1);
            y1 = std::max(y1, y + 1);
            if (distance(px, text) >= close && off_mix(px, text, background) >= off) ++others;
        }
    }
    out.other = float(others) / float(size_t(box.width) * box.height);
    out.filled = float(drawn) / float(size_t(box.width) * box.height);
    out.ink_other = drawn ? float(others) / float(drawn) : 0.0f;
    if (x1 > x0) out.drawn = Rect{x0, y0, x1 - x0, y1 - y0};
    return out;
}

// Characters in UTF-8 text.
size_t char_count(const std::string& text) {
    size_t count = 0;
    for (unsigned char c : text) count += (c & 0xC0) != 0x80;
    return count;
}

// Whether the text has a letter or digit (anything beyond ASCII counts, so
// accented letters and other scripts do).
bool has_letters(const std::string& text) {
    for (unsigned char c : text) {
        if (c >= 0x80 || std::isalnum(c)) return true;
    }
    return false;
}

// Usual height of a line's words (median), the size of its letters.
int32_t usual_height(const std::vector<OcrWord>& words) {
    std::vector<int32_t> heights;
    for (const OcrWord& w : words) heights.push_back(w.box.height);
    if (heights.empty()) return 0;
    std::nth_element(heights.begin(), heights.begin() + heights.size() / 2, heights.end());
    return heights[heights.size() / 2];
}

// Usual height of a line's words other than word `skip` (0 when there are none).
int32_t usual_height_without(const std::vector<OcrWord>& words, size_t skip) {
    std::vector<OcrWord> others;
    for (size_t i = 0; i < words.size(); ++i)
        if (i != skip) others.push_back(words[i]);
    return usual_height(others);
}

// Whether two boxes share any pixel.
bool overlaps(Rect a, Rect b) {
    return a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height;
}

// A picture right before the line's first word or after its last one (song
// notes around lyrics, a key before its label), or an empty box. It must be
// small and stand on its own: something touching the far edge of the search
// (a portrait, the box's frame) or another line is not a picture of this line.
Rect edge_picture(const Frame& frame, const OcrLine& line, const std::vector<OcrLine>& lines, bool before) {
    const OcrWord& word = before ? line.words.front() : line.words.back();
    const int32_t height = std::max(1, usual_height(line.words));
    const int32_t inset = std::max(1, height / 6);
    const int32_t reach_x = int32_t(height * EDGE_REACH);
    const int32_t reach_y = int32_t(height * TALLER);
    const int32_t x = before ? word.box.x - reach_x : word.box.x + word.box.width + inset;
    const Rect area = clipped({x, line.box.y - reach_y, reach_x - inset, line.box.height + 2 * reach_y}, frame);
    if (area.empty()) return {};
    const Rect d = look_at(frame, area, line.text_rgb, line.background_rgb).drawn;
    if (d.empty() || d.width < height * SMALLEST || d.height < height * SMALLEST) return {};
    const bool far_edge = before ? d.x <= area.x : d.x + d.width >= area.x + area.width;
    const bool cut = d.y <= area.y || d.y + d.height >= area.y + area.height;
    if (far_edge || cut) return {}; // runs on past the search: not a small picture of its own
    for (const OcrLine& other : lines)
        if (&other != &line && overlaps(other.box, d)) return {};
    return d;
}

} // namespace

std::string icon_placeholder(size_t index) { return "{" + std::to_string(index + 1) + "}"; }

// Goes through each line's words and the gaps between them; pictures become
// placeholders, and the line's text is rebuilt from its words when it changed.
std::vector<Rect> find_icons(const Frame& frame, std::vector<OcrLine>& lines) {
    std::vector<Rect> icons;
    for (OcrLine& line : lines) {
        if (line.words.empty()) continue;
        const int32_t usual = usual_height(line.words);
        const int32_t line_height = std::max(usual, 1);
        std::vector<OcrWord> words;
        bool changed = false;
        Rect grown_box = line.box; // the line's box with its edge pictures
        const auto add_edge = [&](Rect picture) {
            words.push_back({icon_placeholder(icons.size()), picture});
            icons.push_back(picture);
            const int32_t x0 = std::min(grown_box.x, picture.x), y0 = std::min(grown_box.y, picture.y);
            const int32_t x1 = std::max(grown_box.x + grown_box.width, picture.x + picture.width);
            const int32_t y1 = std::max(grown_box.y + grown_box.height, picture.y + picture.height);
            grown_box = {x0, y0, x1 - x0, y1 - y0};
            changed = true;
        };
        if (const Rect before = edge_picture(frame, line, lines, true); !before.empty()) add_edge(before);
        for (size_t i = 0; i < line.words.size(); ++i) {
            const OcrWord& w = line.words[i];
            // A gap before this word, wide enough for a picture.
            if (i > 0) {
                const OcrWord& prev = line.words[i - 1];
                const int32_t gap_x = prev.box.x + prev.box.width;
                const int32_t gap = w.box.x - gap_x;
                if (gap > line_height * WIDE_GAP) {
                    // Inside the gap, a little in from the words; buttons are often
                    // taller than the letters, so it reaches above and below them.
                    const int32_t inset = std::max(1, line_height / 6);
                    const int32_t reach = int32_t(line_height * TALLER);
                    const Rect area{gap_x + inset, line.box.y - reach, gap - 2 * inset, line.box.height + 2 * reach};
                    const Look seen = look_at(frame, area, line.text_rgb, line.background_rgb);
                    if (seen.other >= OTHER_GAP && !seen.drawn.empty()) {
                        words.push_back({icon_placeholder(icons.size()), seen.drawn});
                        icons.push_back(seen.drawn);
                        changed = true;
                    }
                }
            }
            // The word itself drawn as a picture.
            const Look seen = look_at(frame, w.box, line.text_rgb, line.background_rgb);
            // Only short junk: a whole word in another color is a highlighted
            // word ("use a Fireball" in red), and must be translated.
            const bool junk = char_count(w.text) <= ICON_READING || !has_letters(w.text);
            const bool colored = junk && (seen.other >= OTHER_WORD || (seen.filled > 0.05f && seen.ink_other >= OTHER_INK));
            // A key drawn as a box with its letter on it: the face fills the box.
            const bool squarish = w.box.width * 10 >= w.box.height * 6; // keys are about square; an "I" is a thin bar
            const bool key_face = junk && squarish && line.words.size() > 1 && seen.filled >= FILLED_KEY;
            // ...or a short word much taller than the line's other words.
            const int32_t others = usual_height_without(line.words, i);
            const bool tall_key = char_count(w.text) <= SHORT_WORD && others > 0 && w.box.height > others * TALL_WORD;
            if (colored || key_face || tall_key) {
                // The whole picture: OCR boxes it like letters and may clip it above or below.
                const int32_t reach = int32_t(line_height * TALLER);
                const Look whole = look_at(frame, {w.box.x, w.box.y - reach, w.box.width, w.box.height + 2 * reach},
                                           line.text_rgb, line.background_rgb);
                const Rect box = !whole.drawn.empty() ? whole.drawn : !seen.drawn.empty() ? seen.drawn : w.box;
                words.push_back({icon_placeholder(icons.size()), box});
                icons.push_back(box);
                changed = true;
            } else {
                words.push_back(w);
            }
        }
        if (const Rect after = edge_picture(frame, line, lines, false); !after.empty()) add_edge(after);
        if (!changed) continue;
        line.box = grown_box; // the pictures are part of the line: their place gets cleaned too
        std::string text;
        for (const OcrWord& w : words) {
            if (!text.empty()) text += ' ';
            text += w.text;
        }
        line.text = text;
        line.words = std::move(words);
    }
    return icons;
}

} // namespace subquest
