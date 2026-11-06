#include "vision/icons.h"

#include <algorithm>
#include <climits>
#include <cctype>
#include <cmath>

namespace subquest {

namespace {

constexpr float OTHER_WORD = 0.30f;  // share of "other" pixels that makes a word a picture
constexpr float OTHER_GAP_INK = 0.3f; // a drawing in a gap between words: this much of its ink in a color of its own
constexpr float TALL_WORD = 1.35f;   // a short word this much taller than the line's words: a drawn key
constexpr size_t SHORT_WORD = 3;     // characters at most for the "tall word" rule
constexpr size_t ICON_READING = 1;   // OCR reads a picture as one character ("A", "®"); two letters can be a word ("on")
constexpr float WIDE_GAP = 0.9f;     // a gap wider than this share of the line height can hold a picture
constexpr float TALLER = 0.75f;      // pictures may reach this share of the line height above and below the letters
constexpr float OTHER_INK = 0.5f;    // short junk whose ink is mostly another color than the text's (a white note in a green line)
constexpr float FILLED_KEY = 0.6f;   // short junk whose box is at least this full is a key's face (letters fill 20-40%)
constexpr float EDGE_REACH = 3.0f;   // pictures before the first word / after the last: within this many line heights
constexpr float SMALLEST = 0.4f;     // a picture is at least this share of the line height, both ways
constexpr float GAP_TALL = 0.8f;     // a drawing alone in a wide gap this tall (share of the line height) is a picture
constexpr float ICON_SPAN = 1.4f;    // pieces of one picture fit in this many line heights, both ways
constexpr int VIVID = 30;            // colorfulness above the text's and background's that only a picture has

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
    // Past `a` on the far side from `b` still counts as on the mix: the
    // letters' cores are brighter than their sampled color, which averages in
    // their soft edges (Hades' dim purple italics; a plain "a" there read as
    // a picture). A color of its own lies off the axis, not along it.
    const float t = std::clamp(along / std::max(axis2, 1.0f), -2.0f, 1.0f);
    float d2 = 0;
    for (int i = 0; i < 3; ++i) d2 += (p[i] - (ca[i] + (cb[i] - ca[i]) * t)) * (p[i] - (ca[i] + (cb[i] - ca[i]) * t));
    return std::sqrt(d2);
}

// How colorful a 0xRRGGBB color is: its strongest channel minus its weakest.
int chroma(uint32_t rgb) {
    const int r = int((rgb >> 16) & 0xFF), g = int((rgb >> 8) & 0xFF), b = int(rgb & 0xFF);
    return std::max({r, g, b}) - std::min({r, g, b});
}

// Whether a BGRA pixel is more colorful than any mix of the text and the
// background can be (a mix is never more colorful than the more colorful of
// the two): the green of a clock on white-on-black text, even when that green
// is dark enough to pass for background.
bool vivid(const uint8_t* px, int limit) {
    return std::max({px[0], px[1], px[2]}) - std::min({px[0], px[1], px[2]}) > limit;
}

// Colorfulness a pixel must pass to be vivid, for a line's two colors.
int vivid_limit(uint32_t text, uint32_t background) { return std::max(chroma(text), chroma(background)) + VIVID; }

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
    const int colorful = vivid_limit(text, background);
    size_t others = 0;
    size_t drawn = 0;
    int32_t x0 = INT32_MAX, y0 = INT32_MAX, x1 = INT32_MIN, y1 = INT32_MIN;
    for (int32_t y = box.y; y < box.y + box.height; ++y) {
        const uint8_t* row = frame.bgra.data() + size_t(y) * frame.stride;
        for (int32_t x = box.x; x < box.x + box.width; ++x) {
            const uint8_t* px = row + size_t(x) * 4;
            const bool lively = vivid(px, colorful);
            if (distance(px, background) < close && !lively) continue; // background
            ++drawn;
            x0 = std::min(x0, x);
            y0 = std::min(y0, y);
            x1 = std::max(x1, x + 1);
            y1 = std::max(y1, y + 1);
            if (lively || (distance(px, text) >= close && off_mix(px, text, background) >= off)) ++others;
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

// What is drawn on its own inside a search area: the pixels that aren't the
// line's background (only those of a color of their own, with `own_only`),
// split in connected pieces; pieces that touch the edge of
// the area (scenery around a thin subtitle strip, a portrait, the next word)
// or that don't sit at the line's height are left out, and so are specks.
// Returns the box around the pieces kept, and how much of their ink isn't the
// text's own color (empty box: nothing on its own there).
struct Drawing {
    Rect box;
    float ink_other = 0;
    int pieces = 0; // how many connected pieces were kept
};

Drawing isolated_drawing(const Frame& frame, Rect area, const OcrLine& line, bool own_only = false,
                         bool largest_only = false, bool vivid_ink = false, bool gather = false) {
    Drawing out;
    area = clipped(area, frame);
    if (area.width < 3 || area.height < 3) return out;
    const uint8_t t[4] = {uint8_t(line.text_rgb & 0xFF), uint8_t((line.text_rgb >> 8) & 0xFF),
                          uint8_t((line.text_rgb >> 16) & 0xFF), 255};
    const float contrast = std::max(60.0f, distance(t, line.background_rgb));
    const float close = contrast * 0.33f;
    const float off = std::max(30.0f, contrast * 0.15f);
    const int colorful = vivid_limit(line.text_rgb, line.background_rgb);
    const int32_t w = area.width, h = area.height;
    std::vector<uint8_t> ink(size_t(w) * h, 0); // 1 = drawn, 2 = drawn in a color of its own
    for (int32_t y = 0; y < h; ++y) {
        const uint8_t* row = frame.bgra.data() + size_t(area.y + y) * frame.stride;
        for (int32_t x = 0; x < w; ++x) {
            const uint8_t* px = row + size_t(area.x + x) * 4;
            const bool lively = vivid_ink && vivid(px, colorful);
            if (distance(px, line.background_rgb) < close && !lively) continue;
            const bool own =
                lively || (distance(px, line.text_rgb) >= close && off_mix(px, line.text_rgb, line.background_rgb) >= off);
            if (own_only && !own) continue; // looking for something not drawn in the text's color
            ink[size_t(y) * w + x] = own ? 2 : 1;
        }
    }
    const int32_t letters = std::max(1, line.box.height);
    const int32_t top = line.box.y - area.y - letters / 2;          // the line's height band, inside the area
    const int32_t bottom = line.box.y + line.box.height - area.y + letters / 2;
    std::vector<uint8_t> seen(ink.size(), 0);
    std::vector<int32_t> stack;
    int32_t x0 = INT32_MAX, y0 = INT32_MAX, x1 = INT32_MIN, y1 = INT32_MIN;
    size_t kept = 0, kept_own = 0;
    struct Piece {
        int32_t x0, y0, x1, y1;
        size_t count, own;
    };
    std::vector<Piece> pieces;
    for (int32_t start = 0; start < w * h; ++start) {
        if (!ink[start] || seen[start]) continue;
        // One piece: every drawn pixel connected to this one (8 neighbors).
        int32_t px0 = w, py0 = h, px1 = -1, py1 = -1;
        size_t count = 0, own = 0;
        stack.assign(1, start);
        seen[start] = 1;
        while (!stack.empty()) {
            const int32_t i = stack.back();
            stack.pop_back();
            const int32_t x = i % w, y = i / w;
            px0 = std::min(px0, x), py0 = std::min(py0, y), px1 = std::max(px1, x), py1 = std::max(py1, y);
            ++count;
            own += ink[i] == 2;
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
        const bool touches = px0 == 0 || py0 == 0 || px1 == w - 1 || py1 == h - 1;
        const int32_t middle = (py0 + py1) / 2;
        const bool speck = count < size_t(std::max(4, letters / 2));
        if (touches || speck || middle < top || middle > bottom) continue;
        ++out.pieces;
        if (largest_only) {
            pieces.push_back({px0, py0, px1, py1, count, own});
            continue;
        }
        x0 = std::min(x0, px0), y0 = std::min(y0, py0), x1 = std::max(x1, px1), y1 = std::max(y1, py1);
        kept += count;
        kept_own += own;
    }
    if (largest_only && !pieces.empty()) {
        // The biggest piece (a key's face, not the letters boxed with it), with
        // the pieces right around it that still make one picture: a clock's
        // light rim comes in several arcs around its dark face. A piece joins
        // when it is close and the picture stays about icon-sized (a key with
        // the "S" beside it would get too wide).
        std::sort(pieces.begin(), pieces.end(), [](const Piece& a, const Piece& b) { return a.count > b.count; });
        const Piece& big = pieces.front();
        x0 = big.x0, y0 = big.y0, x1 = big.x1, y1 = big.y1, kept = big.count, kept_own = big.own;
        const int32_t near = std::max(2, letters / 4);
        const int32_t widest = std::max(x1 - x0 + 1, int32_t(letters * ICON_SPAN));
        const int32_t tallest = std::max(y1 - y0 + 1, int32_t(letters * ICON_SPAN));
        std::vector<bool> used(pieces.size(), false);
        used[0] = true;
        for (bool grew = gather; grew;) { // without `gather`, the biggest piece alone
            grew = false;
            for (size_t k = 1; k < pieces.size(); ++k) {
                const Piece& p = pieces[k];
                if (used[k]) continue;
                const bool close_by = p.x0 <= x1 + near && p.x1 >= x0 - near && p.y0 <= y1 + near && p.y1 >= y0 - near;
                const int32_t nx0 = std::min(x0, p.x0), ny0 = std::min(y0, p.y0);
                const int32_t nx1 = std::max(x1, p.x1), ny1 = std::max(y1, p.y1);
                if (!close_by || nx1 - nx0 + 1 > widest || ny1 - ny0 + 1 > tallest) continue;
                x0 = nx0, y0 = ny0, x1 = nx1, y1 = ny1;
                kept += p.count;
                kept_own += p.own;
                used[k] = grew = true;
            }
        }
    }
    if (kept == 0) return out;
    out.box = {area.x + x0, area.y + y0, x1 - x0 + 1, y1 - y0 + 1};
    out.ink_other = float(kept_own) / float(kept);
    return out;
}

// A picture right before the line's first word or after its last one (song
// notes around lyrics, a key before its label), or an empty box: something
// drawn on its own there (isolated_drawing), big enough, not another line.
Rect edge_picture(const Frame& frame, const OcrLine& line, const std::vector<OcrLine>& lines, bool before) {
    const OcrWord& word = before ? line.words.front() : line.words.back();
    const int32_t height = std::max(1, usual_height(line.words));
    const int32_t inset = std::max(1, height / 6);
    const int32_t reach_x = int32_t(height * EDGE_REACH);
    const int32_t reach_y = int32_t(height * TALLER);
    const int32_t x = before ? word.box.x - reach_x : word.box.x + word.box.width + inset;
    const Rect area = clipped({x, line.box.y - reach_y, reach_x - inset, line.box.height + 2 * reach_y}, frame);
    if (area.empty()) return {};
    // The biggest piece only: a note, not the scenery specks beside it.
    const Rect d = isolated_drawing(frame, area, line, false, true, false, true).box;
    if (d.empty() || d.width < height * SMALLEST || d.height < height * SMALLEST) return {};
    // Nor part of another line: its letters or its own pictures (the line's
    // box may have been cut to its letters already).
    for (const OcrLine& other : lines) {
        if (&other == &line) continue;
        if (overlaps(other.box, d)) return {};
        for (const OcrWord& w : other.words)
            if (overlaps(w.box, d)) return {};
    }
    return d;
}

// A picture OCR boxed together with the first (or last) word but left out of
// its text ("♫Mares" read as "Mares"): something of a color of its own at that
// end of the word's box. Empty box when there is none.
Rect inner_picture(const Frame& frame, const OcrLine& line, const OcrWord& word, bool before) {
    const int32_t height = std::max(1, usual_height(line.words));
    const int32_t inset = std::max(1, height / 6);
    const int32_t reach_y = int32_t(height * TALLER);
    const int32_t span = std::min(word.box.width, int32_t(height * 1.5f));
    const int32_t x = before ? word.box.x - inset : word.box.x + word.box.width - span;
    const Rect area{x, line.box.y - reach_y, span + inset, line.box.height + 2 * reach_y};
    const Drawing found = isolated_drawing(frame, area, line, true, true);
    const Rect d = found.box;
    // One piece (a note, a key), not the letters of a word in another color.
    if (d.empty() || found.pieces != 1 || d.width < height * SMALLEST || d.height < height * SMALLEST) return {};
    // The rest of the word must be letters in the text's color ("Mares" after
    // the note), or the word is a highlighted one, not a picture plus letters.
    const Rect rest = before ? Rect{d.x + d.width, word.box.y, word.box.x + word.box.width - (d.x + d.width), word.box.height}
                             : Rect{word.box.x, word.box.y, d.x - word.box.x, word.box.height};
    if (rest.width < height / 2) return {};
    const Look letters = look_at(frame, rest, line.text_rgb, line.background_rgb);
    if (letters.filled < 0.05f || letters.ink_other >= 0.3f) return {};
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
        const auto add_edge = [&](Rect picture) {
            words.push_back({icon_placeholder(icons.size()), picture});
            icons.push_back(picture);
            changed = true;
        };
        // Pictures at the ends: beside the words, or boxed into the first/last
        // word by OCR (then that word's box is trimmed to its letters).
        std::vector<OcrWord> source = line.words;
        Rect before = edge_picture(frame, line, lines, true);
        // (A word that is itself junk, "J" for a note, is left to the word rules below.)
        const auto real_word = [](const OcrWord& w) { return char_count(w.text) > ICON_READING && has_letters(w.text); };
        if (before.empty() && source.size() > 1 && real_word(source.front())) {
            before = inner_picture(frame, line, source.front(), true);
            if (!before.empty()) {
                OcrWord& first = source.front();
                const int32_t end = first.box.x + first.box.width;
                first.box.x = std::min(end - 1, before.x + before.width);
                first.box.width = end - first.box.x;
            }
        }
        Rect after = edge_picture(frame, line, lines, false);
        if (after.empty() && source.size() > 1 && real_word(source.back())) {
            after = inner_picture(frame, line, source.back(), false);
            if (!after.empty()) {
                OcrWord& last = source.back();
                last.box.width = std::max(1, after.x - last.box.x);
            }
        }
        if (!before.empty()) add_edge(before);
        for (size_t i = 0; i < source.size(); ++i) {
            const OcrWord& w = source[i];
            // A gap before this word, wide enough for a picture.
            if (i > 0) {
                const OcrWord& prev = source[i - 1];
                const int32_t gap_x = prev.box.x + prev.box.width;
                const int32_t gap = w.box.x - gap_x;
                if (gap > line_height * WIDE_GAP) {
                    // Inside the gap, a little in from the words; buttons are often
                    // taller than the letters, so it reaches above and below them.
                    const int32_t inset = std::max(1, line_height / 6);
                    const int32_t reach = int32_t(line_height * TALLER);
                    const Rect area{gap_x + inset, line.box.y - reach, gap - 2 * inset, line.box.height + 2 * reach};
                    const Drawing seen = isolated_drawing(frame, area, line, false, false, true);
                    const bool big = seen.box.width >= line_height * SMALLEST && seen.box.height >= line_height * SMALLEST;
                    // OCR's words hold every letter, so a drawing as tall as
                    // the letters standing alone in a wide gap is a picture
                    // even in the text's own colors (Hades' gray pickaxe in a
                    // line of white and purple words).
                    const bool letter_tall = seen.box.height >= line_height * GAP_TALL;
                    if (!seen.box.empty() && big && (seen.ink_other >= OTHER_GAP_INK || letter_tall)) {
                        words.push_back({icon_placeholder(icons.size()), seen.box});
                        icons.push_back(seen.box);
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
            // Keys carry capitals: a lone lowercase letter is a word ("a"), even
            // a bold one that fills its tight box (Hades' italic "a", 62%).
            const bool lowercase = w.text.size() == 1 && std::islower(static_cast<unsigned char>(w.text[0]));
            const bool key_face = junk && squarish && !lowercase && line.words.size() > 1 && seen.filled >= FILLED_KEY;
            // ...or a short word much taller than the line's other words.
            const int32_t others = usual_height_without(source, i);
            const bool tall_key = junk && others > 0 && w.box.height > others * TALL_WORD;
            if (colored || key_face || tall_key) {
                // Just the picture: OCR may clip it above or below, or box it
                // together with the space and the start of the next word.
                const int32_t reach = int32_t(line_height * TALLER);
                const int32_t inset = std::max(1, line_height / 6);
                const Drawing whole = isolated_drawing(
                    frame, {w.box.x - inset, w.box.y - reach, w.box.width + 2 * inset, w.box.height + 2 * reach}, line,
                    false, true, false, true);
                const Rect box = !whole.box.empty() ? whole.box : !seen.drawn.empty() ? seen.drawn : w.box;
                // A bullet ("·") or a dot is too small to be a picture: it stays text.
                const bool big = box.width >= line_height * SMALLEST && box.height >= line_height * SMALLEST;
                if (big) {
                    words.push_back({icon_placeholder(icons.size()), box});
                    icons.push_back(box);
                    changed = true;
                } else {
                    words.push_back(w);
                }
            } else {
                words.push_back(w);
            }
        }
        if (!after.empty()) add_edge(after);
        if (!changed) continue;
        // The line's box keeps only its letters: their size and case are
        // measured on it (a key taller than the letters made translations
        // come out huge). The pictures' places are cleaned on their own.
        Rect letters_box;
        for (const OcrWord& w : words) {
            if (!w.text.empty() && w.text.front() == '{' && w.text.back() == '}') continue;
            if (letters_box.empty()) {
                letters_box = w.box;
                continue;
            }
            const int32_t x0 = std::min(letters_box.x, w.box.x), y0 = std::min(letters_box.y, w.box.y);
            const int32_t x1 = std::max(letters_box.x + letters_box.width, w.box.x + w.box.width);
            const int32_t y1 = std::max(letters_box.y + letters_box.height, w.box.y + w.box.height);
            letters_box = {x0, y0, x1 - x0, y1 - y0};
        }
        if (!letters_box.empty()) line.box = letters_box;
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
