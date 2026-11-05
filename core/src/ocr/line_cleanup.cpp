#include "ocr/line_cleanup.h"

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

} // namespace subquest
