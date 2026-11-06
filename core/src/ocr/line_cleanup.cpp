#include "ocr/line_cleanup.h"

#include <algorithm>
#include <cstddef>
#include <cstdlib>
#include <cstdint>
#include <cwctype>

namespace subquest {

// True when the word has a letter or digit. Anything outside ASCII counts as
// one, so accented letters and other scripts are always kept whatever the
// C library's locale knows; only ASCII punctuation/symbols are "not letters".
bool has_letter_or_digit(const std::wstring& word) {
    for (wchar_t c : word) {
        if (c > 0x7F || std::iswalnum(static_cast<wint_t>(c))) return true;
    }
    return false;
}

// Range of words to keep: symbol-only words are dropped from both ends (in the
// middle they stay, e.g. "Yes - no"). All symbols: an empty range.
std::pair<size_t, size_t> kept_words(const std::vector<std::wstring>& words) {
    size_t first = 0, last = words.size();
    while (first < last && !has_letter_or_digit(words[first])) ++first;
    while (last > first && !has_letter_or_digit(words[last - 1])) --last;
    return {first, last};
}

// Stray marks glued to the line's ends: an opening bracket at the very start
// whose closing one appears nowhere in the line (and the reverse at the end),
// and bars/underscores, which no sentence starts or ends with. A real "(Play)"
// keeps both brackets. The first and last word keep at least one character.
std::pair<size_t, size_t> stray_marks(const std::vector<std::wstring>& words) {
    if (words.empty()) return {0, 0};
    std::wstring line;
    for (const auto& w : words) line += w;
    const auto count = [&](wchar_t c) { return std::count(line.begin(), line.end(), c); };
    const auto unmatched_open = [&](wchar_t c) {
        switch (c) {
            case L'(': return count(L')') < count(L'(');
            case L'[': return count(L']') < count(L'[');
            case L'{': return count(L'}') < count(L'{');
            case L'<': return count(L'>') < count(L'<');
            case L'|': case L'_': return true;
            default: return false;
        }
    };
    const auto unmatched_close = [&](wchar_t c) {
        switch (c) {
            case L')': return count(L'(') < count(L')');
            case L']': return count(L'[') < count(L']');
            case L'}': return count(L'{') < count(L'}');
            case L'>': return count(L'<') < count(L'>');
            case L'|': case L'_': return true;
            default: return false;
        }
    };
    const std::wstring& first = words.front();
    const std::wstring& last = words.back();
    size_t front = 0;
    while (front + 1 < first.size() && unmatched_open(first[front])) ++front;
    size_t back = 0;
    const size_t last_room = words.size() == 1 ? last.size() - front : last.size();
    while (back + 1 < last_room && unmatched_close(last[last.size() - 1 - back])) ++back;
    return {front, back};
}

namespace {

// UTF-8 to wide characters, enough to tell letters from symbols and to count
// characters (a character beyond 16 bits becomes U+FFFD where wchar_t is 16-bit).
std::wstring widen(const std::string& text) {
    std::wstring out;
    for (size_t i = 0; i < text.size();) {
        const unsigned char c = static_cast<unsigned char>(text[i]);
        const size_t length = c < 0x80 ? 1 : (c >> 5) == 0x6 ? 2 : (c >> 4) == 0xE ? 3 : (c >> 3) == 0x1E ? 4 : 1;
        uint32_t code = length == 1 ? c : c & (0x7F >> length);
        for (size_t k = 1; k < length && i + k < text.size(); ++k)
            code = (code << 6) | (static_cast<unsigned char>(text[i + k]) & 0x3F);
        if (code > 0xFFFF && sizeof(wchar_t) == 2) code = 0xFFFD;
        out.push_back(static_cast<wchar_t>(code));
        i += length;
    }
    return out;
}

// Number of characters (not bytes) in UTF-8 text.
size_t char_count(const std::string& text) {
    size_t count = 0;
    for (unsigned char c : text) count += (c & 0xC0) != 0x80;
    return count;
}

// Smallest rectangle around the boxes of words [first, last).
Rect union_box(const std::vector<OcrWord>& words, size_t first, size_t last) {
    int32_t x0 = 0, y0 = 0, x1 = 0, y1 = 0;
    for (size_t i = first; i < last; ++i) {
        const Rect& r = words[i].box;
        if (i == first || r.x < x0) x0 = r.x;
        if (i == first || r.y < y0) y0 = r.y;
        if (i == first || r.x + r.width > x1) x1 = r.x + r.width;
        if (i == first || r.y + r.height > y1) y1 = r.y + r.height;
    }
    return Rect{x0, y0, x1 - x0, y1 - y0};
}

// The line's own text without the dropped words and marks. Taken out of the
// engine's text rather than rebuilt from the words, because engines space
// words their own way (Japanese has no spaces at all). Words joined by spaces
// when the engine's text doesn't match its words.
std::string trimmed_text(const OcrLine& line, size_t first, size_t last, size_t front, size_t back) {
    const std::string& text = line.text;
    const auto& words = line.words;
    size_t start = 0, end = text.size();
    bool matches = true;
    const auto skip_front = [&] { while (start < end && text[start] == ' ') ++start; };
    const auto skip_back = [&] { while (end > start && text[end - 1] == ' ') --end; };
    for (size_t i = 0; i < first && matches; ++i) {
        skip_front();
        matches = text.compare(start, words[i].text.size(), words[i].text) == 0;
        start += words[i].text.size();
    }
    for (size_t i = words.size(); i > last && matches; --i) {
        skip_back();
        const std::string& w = words[i - 1].text;
        matches = end >= start + w.size() && text.compare(end - w.size(), w.size(), w) == 0;
        end -= w.size();
    }
    if (matches) {
        skip_front();
        skip_back();
        // Stray marks are ASCII: one character is one byte.
        if (end >= start + front + back) return text.substr(start + front, end - start - front - back);
    }
    std::string joined;
    for (size_t i = first; i < last; ++i) {
        std::string word = words[i].text;
        if (i + 1 == last && back > 0) word.erase(word.size() - std::min(back, word.size()));
        if (i == first && front > 0) word.erase(0, std::min(front, word.size()));
        if (!joined.empty()) joined += ' ';
        joined += word;
    }
    return joined;
}

} // namespace

// Splits a line on spaces and gives each word the share of the box its
// characters take (engines that only box whole lines).
std::vector<OcrWord> split_words(const std::string& text, Rect box) {
    std::vector<OcrWord> words;
    const size_t total = char_count(text);
    if (total == 0) return words;
    size_t done = 0; // characters before the current position
    for (size_t i = 0; i < text.size();) {
        if (text[i] == ' ') {
            ++i;
            ++done;
            continue;
        }
        const size_t end = std::min(text.find(' ', i), text.size());
        std::string word = text.substr(i, end - i);
        const size_t count = char_count(word);
        const int32_t x0 = box.x + int32_t(int64_t(box.width) * int64_t(done) / int64_t(total));
        const int32_t x1 = box.x + int32_t(int64_t(box.width) * int64_t(done + count) / int64_t(total));
        words.push_back({std::move(word), Rect{x0, box.y, x1 - x0, box.height}});
        done += count;
        i = end;
    }
    return words;
}

// Drops symbol-only words at the line's edges and stray marks glued to its
// ends, from the text and from the box (a mark takes its share of its word's
// width: engines don't box single letters). Untouched lines keep the engine's
// text as it is.
bool tidy_line(OcrLine& line) {
    if (line.words.empty()) line.words = split_words(line.text, line.box);
    if (line.words.empty()) return false;
    std::vector<std::wstring> wide;
    for (const OcrWord& w : line.words) wide.push_back(widen(w.text));
    const auto [first, last] = kept_words(wide);
    if (first == last) return false;
    const std::vector<std::wstring> kept(wide.begin() + first, wide.begin() + last);
    const auto [front, back] = stray_marks(kept);

    Rect box = union_box(line.words, first, last);
    if (front > 0) {
        const Rect& r = line.words[first].box;
        const int32_t cut = int32_t(int64_t(r.width) * int64_t(front) / int64_t(kept.front().size()));
        box.x += cut;
        box.width -= cut;
    }
    if (back > 0) {
        const Rect& r = line.words[last - 1].box;
        box.width -= int32_t(int64_t(r.width) * int64_t(back) / int64_t(kept.back().size()));
    }

    const bool untouched = first == 0 && last == line.words.size() && front == 0 && back == 0;
    if (!untouched || line.text.empty()) line.text = trimmed_text(line, first, last, front, back);
    line.box = box;

    // The words follow the cleaned line: only the kept ones, without the marks
    // (their boxes lose the same share as the line's), so picking a word over
    // the game never picks a stray "(".
    std::vector<OcrWord> words(line.words.begin() + first, line.words.begin() + last);
    if (front > 0) {
        OcrWord& w = words.front();
        const int32_t cut = int32_t(int64_t(w.box.width) * int64_t(front) / int64_t(kept.front().size()));
        w.box.x += cut;
        w.box.width -= cut;
        w.text.erase(0, std::min(front, w.text.size()));
    }
    if (back > 0) {
        OcrWord& w = words.back();
        w.box.width -= int32_t(int64_t(w.box.width) * int64_t(back) / int64_t(kept.back().size()));
        w.text.erase(w.text.size() - std::min(back, w.text.size()));
    }
    line.words = std::move(words);
    return true;
}

// Pieces of one visual line that OCR returned as separate lines: on the same
// row (their middles within a third of the letters' height) and close
// sideways (less than 1.5 letter heights apart). OCR splits a line where the
// font changes ("Found using the" / "Crescent Pick" in bold / "on"); each piece
// would then get a piece of the translation, drawn over each other. Columns
// further apart ("Gold        120") stay separate lines.
void join_split_lines(std::vector<OcrLine>& lines) {
    const auto middle = [](const Rect& r) { return r.y + r.height / 2; };
    bool joined = true;
    while (joined) {
        joined = false;
        for (size_t i = 0; i < lines.size() && !joined; ++i) {
            for (size_t j = 0; j < lines.size() && !joined; ++j) {
                if (i == j) continue;
                OcrLine& a = lines[i]; // the one on the left
                const OcrLine& b = lines[j];
                const int32_t height = std::max(a.box.height, b.box.height);
                const int32_t gap = b.box.x - (a.box.x + a.box.width);
                const bool same_row = std::abs(middle(a.box) - middle(b.box)) * 3 <= height;
                if (!same_row || gap < -height / 2 || gap * 2 > height * 3) continue;
                a.text += " " + b.text;
                a.words.insert(a.words.end(), b.words.begin(), b.words.end());
                const int32_t x0 = std::min(a.box.x, b.box.x), y0 = std::min(a.box.y, b.box.y);
                const int32_t x1 = std::max(a.box.x + a.box.width, b.box.x + b.box.width);
                const int32_t y1 = std::max(a.box.y + a.box.height, b.box.y + b.box.height);
                a.box = {x0, y0, x1 - x0, y1 - y0};
                lines.erase(lines.begin() + std::ptrdiff_t(j));
                joined = true;
            }
        }
    }
}

// Tidies every line of a result, joins lines OCR split, and joins what is left into full_text.
void tidy_lines(OcrResult& result) {
    std::vector<OcrLine> kept;
    for (OcrLine& line : result.lines) {
        if (tidy_line(line)) kept.push_back(std::move(line));
    }
    join_split_lines(kept);
    result.full_text.clear();
    for (const OcrLine& line : kept) {
        if (!result.full_text.empty()) result.full_text += '\n';
        result.full_text += line.text;
    }
    result.lines = std::move(kept);
}

} // namespace subquest
