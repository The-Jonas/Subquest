#include "ocr/line_cleanup.h"

#include <algorithm>
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

} // namespace subquest
