#include "ocr/second_read.h"

#include <algorithm>
#include <climits>
#include <cmath>
#include <cwctype>

#include "vision/palette.h"

namespace subquest {

namespace {

constexpr size_t MAX_WORD = 18;     // a longer "word" is junk (English: "internationalization" is 20, and rare in games)
constexpr size_t MAX_LINES = 10;    // lines read again per frame, to bound the wait on a page of junk
constexpr int32_t SCALE = 2;        // the crop is enlarged: the letters become bigger for the engine
const float LEVELS[] = {0.46f, 0.54f}; // how near the text color a pixel must be to count as ink, as a share of the text/background distance
                                       // (measured on a real page: 0.46 read best; brightness instead of color was fooled by light green grass)

// UTF-8 to wide characters (invalid bytes are skipped).
std::wstring widen(const std::string& text) {
    std::wstring out;
    for (size_t i = 0; i < text.size();) {
        const unsigned char c = static_cast<unsigned char>(text[i]);
        const size_t extra = c >= 0xF0 ? 3 : c >= 0xE0 ? 2 : c >= 0xC0 ? 1 : 0;
        if (c >= 0x80 && extra == 0) {
            ++i;
            continue;
        }
        uint32_t code = extra == 0 ? c : (c & (0x3F >> extra));
        for (size_t k = 1; k <= extra && i + k < text.size(); ++k) code = (code << 6) | (static_cast<unsigned char>(text[i + k]) & 0x3F);
        out.push_back(wchar_t(code));
        i += extra + 1;
    }
    return out;
}

bool ascii_letter(wchar_t c) { return (c >= L'a' && c <= L'z') || (c >= L'A' && c <= L'Z'); }
bool ascii_digit(wchar_t c) { return c >= L'0' && c <= L'9'; }
bool is_alnum(wchar_t c) { return ascii_letter(c) || ascii_digit(c) || c >= 0xC0; }
bool has_non_ascii(const std::string& word) {
    for (unsigned char c : word)
        if (c >= 0x80) return true;
    return false;
}
bool has_inner_hyphen(const std::string& word) { // "movin-gone": two words glued by a misread hyphen
    const size_t at = word.find('-');
    return at != std::string::npos && at > 0 && at + 1 < word.size();
}
bool has_alnum(const std::string& word) {
    for (wchar_t c : widen(word))
        if (is_alnum(c)) return true;
    return false;
}

float luma_of(uint32_t rgb) { return 0.299f * float((rgb >> 16) & 0xFF) + 0.587f * float((rgb >> 8) & 0xFF) + 0.114f * float(rgb & 0xFF); }

} // namespace

// Whether a word looks like OCR junk (see second_read.h).
bool is_junk_word(const std::string& word) {
    std::wstring w = widen(word);
    size_t from = 0, to = w.size();
    while (from < to && !is_alnum(w[from])) ++from;
    while (to > from && !is_alnum(w[to - 1])) --to;
    w = w.substr(from, to - from);
    if (w.size() > MAX_WORD) return true;
    bool letter = false, digit = false;
    for (size_t i = 0; i < w.size(); ++i) {
        const wchar_t c = w[i];
        letter = letter || ascii_letter(c);
        digit = digit || ascii_digit(c);
        if (i == 0) continue;
        if (c >= L'A' && c <= L'Z' && w[i - 1] >= L'a' && w[i - 1] <= L'z') return true; // "thCeri", "NormallY"
        if (i + 1 >= w.size()) continue;
        if (std::wstring(L".;:,_&@#$%^*=+/\\|<>~").find(c) != std::wstring::npos) {
            const bool between_digits = ascii_digit(w[i - 1]) && ascii_digit(w[i + 1]); // 3.5, 10:30
            if (!between_digits) return true;
        }
    }
    // Letters mixed with digits ("0Ñir"): "1st", "3D" and the like also count, and cost one more reading at most.
    return letter && digit;
}

// Whether any word of the line looks like junk.
bool has_junk(const OcrLine& line) {
    for (const OcrWord& w : line.words)
        if (is_junk_word(w.text)) return true;
    return false;
}

// A crop of the frame in pure black and white, enlarged (see second_read.h).
bool binarize_crop(const Frame& frame, Rect region, uint32_t text, uint32_t background, float level, Binarized& out) {
    const int32_t x0 = std::max(0, region.x), y0 = std::max(0, region.y);
    const int32_t x1 = std::min(frame.width, region.x + region.width), y1 = std::min(frame.height, region.y + region.height);
    if (x1 - x0 < 4 || y1 - y0 < 4) return false;
    if (std::abs(luma_of(text) - luma_of(background)) < 40.0f) return false; // no contrast to cut by
    const auto channel = [](uint32_t rgb, int shift) { return float((rgb >> shift) & 0xFF); };
    const float range = std::sqrt(std::pow(channel(text, 16) - channel(background, 16), 2.0f) + std::pow(channel(text, 8) - channel(background, 8), 2.0f) +
                                  std::pow(channel(text, 0) - channel(background, 0), 2.0f));
    const float reach = level * range; // ink: no farther than this from the text color
    const int32_t w = x1 - x0, h = y1 - y0;
    out.frame = Frame{};
    out.frame.width = w * SCALE;
    out.frame.height = h * SCALE;
    out.frame.stride = out.frame.width * 4;
    out.frame.id = frame.id;
    out.frame.bgra.assign(size_t(out.frame.stride) * out.frame.height, 255); // white
    for (int32_t y = 0; y < h; ++y) {
        const uint8_t* row = frame.bgra.data() + size_t(y0 + y) * frame.stride;
        for (int32_t x = 0; x < w; ++x) {
            const uint8_t* src = row + size_t(x0 + x) * 4;
            const float dr = float(src[2]) - channel(text, 16), dg = float(src[1]) - channel(text, 8), db = float(src[0]) - channel(text, 0);
            if (std::sqrt(dr * dr + dg * dg + db * db) > reach) continue;
            for (int32_t dy = 0; dy < SCALE; ++dy)
                for (int32_t dx = 0; dx < SCALE; ++dx) {
                    uint8_t* px = out.frame.bgra.data() + size_t(y * SCALE + dy) * out.frame.stride + size_t(x * SCALE + dx) * 4;
                    px[0] = px[1] = px[2] = 0;
                }
        }
    }
    out.x = x0;
    out.y = y0;
    out.scale = SCALE;
    return true;
}

// A word of the second reading cut where junk punctuation sits between letters
// ("Normally,tmoving.one" -> "Normally,", "tmoving", "one"; "-and•then" -> "and", "then"),
// each piece with its share of the box. Pieces keep their trailing punctuation.
std::vector<OcrWord> pieces_of(const OcrWord& word) {
    const std::wstring w = widen(word.text);
    if (w.empty()) return {};
    const std::wstring separators = L".,;:_&@#$%^*=+/|<>~•„‚";
    std::vector<std::pair<size_t, size_t>> parts; // [from, to) in characters
    size_t from = 0;
    for (size_t i = 1; i + 1 < w.size();) {
        if (separators.find(w[i]) == std::wstring::npos || !is_alnum(w[i - 1])) {
            ++i;
            continue;
        }
        size_t run = i; // a run of separators ("..", ",,") is one cut
        while (run < w.size() && separators.find(w[run]) != std::wstring::npos) ++run;
        if (run < w.size() && is_alnum(w[run])) {
            const bool keeps = w[i] == L',' || w[i] == L'.' || w[i] == L';' || w[i] == L':'; // trailing punctuation stays
            parts.push_back({from, keeps ? i + 1 : i});
            from = run;
        }
        i = run + 1;
    }
    parts.push_back({from, w.size()});
    std::vector<OcrWord> out;
    for (const auto& [a, b] : parts) {
        std::string text;
        size_t start = a;
        while (start < b && (w[start] == L'-' || w[start] == L'•')) ++start; // a stray dash or bullet in front
        for (size_t i = start; i < b; ++i) {
            const uint32_t c = uint32_t(w[i]);
            if (c < 0x80) text += char(c);
            else if (c < 0x800) text += char(0xC0 | (c >> 6)), text += char(0x80 | (c & 0x3F));
            else text += char(0xE0 | (c >> 12)), text += char(0x80 | ((c >> 6) & 0x3F)), text += char(0x80 | (c & 0x3F));
        }
        if (text.empty()) continue;
        const int32_t x = word.box.x + int32_t(int64_t(word.box.width) * int64_t(a) / int64_t(w.size()));
        const int32_t x2 = word.box.x + int32_t(int64_t(word.box.width) * int64_t(b) / int64_t(w.size()));
        out.push_back({text, {x, word.box.y, std::max(1, x2 - x), word.box.height}});
    }
    return out;
}

// Puts a second reading's clean words into the line (see second_read.h).
void merge_second_read(OcrLine& line, const std::vector<OcrWord>& second) {
    const std::vector<OcrWord> first = line.words;
    std::vector<bool> bad(first.size());
    for (size_t i = 0; i < first.size(); ++i) bad[i] = is_junk_word(first[i].text);
    // A word with an odd letter inside ("nuiñbeijof", for "number of") next to junk is junk read half right.
    const std::vector<bool> junk = bad;
    for (size_t i = 0; i < first.size(); ++i)
        if (!junk[i] && (has_non_ascii(first[i].text) || has_inner_hyphen(first[i].text)) && ((i > 0 && junk[i - 1]) || (i + 1 < first.size() && junk[i + 1]))) bad[i] = true;
    std::vector<std::vector<OcrWord>> replacing(first.size()); // second-reading words over each bad first word
    std::vector<OcrWord> added;                                // ...and over stretches the first reading skipped
    std::vector<OcrWord> split;
    for (const OcrWord& word : second) {
        const std::vector<OcrWord> cut = pieces_of(word);
        split.insert(split.end(), cut.begin(), cut.end());
    }
    for (const OcrWord& s : split) {
        if (is_junk_word(s.text) || !has_alnum(s.text)) continue;
        const int32_t overlap = std::min(s.box.y + s.box.height, line.box.y + line.box.height) - std::max(s.box.y, line.box.y);
        if (overlap * 2 < std::min(s.box.height, line.box.height)) continue; // another line
        const int32_t centre = s.box.x + s.box.width / 2;
        bool inside = false;
        for (size_t i = 0; i < first.size(); ++i) {
            if (centre < first[i].box.x || centre >= first[i].box.x + first[i].box.width) continue;
            inside = true;
            if (bad[i]) replacing[i].push_back(s);
            break;
        }
        if (!inside) added.push_back(s);
    }
    std::vector<OcrWord> merged;
    for (size_t i = 0; i < first.size(); ++i) {
        if (bad[i] && !replacing[i].empty()) merged.insert(merged.end(), replacing[i].begin(), replacing[i].end());
        else merged.push_back(first[i]);
    }
    merged.insert(merged.end(), added.begin(), added.end());
    if (merged.size() == first.size() && added.empty() && std::equal(merged.begin(), merged.end(), first.begin(), [](const OcrWord& a, const OcrWord& b) { return a.text == b.text; })) return;
    std::stable_sort(merged.begin(), merged.end(), [](const OcrWord& a, const OcrWord& b) { return a.box.x < b.box.x; });
    int32_t left = INT32_MAX, right = INT32_MIN;
    std::string text;
    for (const OcrWord& w : merged) {
        left = std::min(left, w.box.x), right = std::max(right, w.box.x + w.box.width);
        text += (text.empty() ? "" : " ") + w.text;
    }
    line.words = std::move(merged);
    line.text = std::move(text);
    line.box.x = left;
    line.box.width = right - left;
}

// Reads the lines with junk again, in black and white (see second_read.h).
void refine_lines(const Frame& frame, OcrResult& result, IOcrEngine& ocr) {
    size_t reread = 0;
    bool changed = false;
    for (OcrLine& line : result.lines) {
        if (reread >= MAX_LINES) break;
        if (line.words.empty() || !has_junk(line)) continue;
        ++reread;
        const TextColors colors = sample_line_colors(frame, line.box);
        const int32_t margin_x = std::max(6, line.box.height / 2), margin_y = std::max(2, line.box.height / 3);
        const Rect region{line.box.x - margin_x, line.box.y - margin_y, line.box.width + 2 * margin_x, line.box.height + 2 * margin_y};
        for (const float level : LEVELS) {
            Binarized crop;
            if (!binarize_crop(frame, region, colors.text, colors.background, level, crop)) break;
            std::vector<OcrWord> words;
            for (const OcrLine& read : ocr.recognize(crop.frame).lines)
                for (OcrWord w : read.words) {
                    w.box = {crop.x + w.box.x / crop.scale, crop.y + w.box.y / crop.scale, w.box.width / crop.scale, w.box.height / crop.scale};
                    words.push_back(std::move(w));
                }
            const std::string before = line.text;
            merge_second_read(line, words);
            changed = changed || line.text != before;
            if (!has_junk(line)) break; // nothing bad left to fix with another level
        }
    }
    if (!changed) return;
    result.full_text.clear();
    for (const OcrLine& line : result.lines) result.full_text += (result.full_text.empty() ? "" : "\n") + line.text;
}

} // namespace subquest
