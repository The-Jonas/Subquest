// Minimal test for kept_words (stray symbols at the edges of OCR lines), no framework.
#include <cstdio>

#include "ocr/line_cleanup.h"

using namespace subquest;

static int failures = 0;

// Checks a condition and records a failure with a description.
static void check(bool ok, const char* what) {
    std::printf("[%s] %s\n", ok ? " ok " : "FAIL", what);
    if (!ok) ++failures;
}

int main() {
    using W = std::vector<std::wstring>;
    check(kept_words(W{L"(", L"PLAY"}) == std::pair<size_t, size_t>{1, 2}, "a crescent read as '(' before a menu item goes");
    check(kept_words(W{L"Quit", L"|"}) == std::pair<size_t, size_t>{0, 1}, "a frame edge read as '|' after it goes");
    check(kept_words(W{L"Yes", L"-", L"no"}) == std::pair<size_t, size_t>{0, 3}, "symbols between words stay");
    check(kept_words(W{L"(Play)"}) == std::pair<size_t, size_t>{0, 1}, "brackets glued to a word stay");
    check(kept_words(W{L"...", L"?"}).first == kept_words(W{L"...", L"?"}).second, "a line of symbols only is empty");
    check(has_letter_or_digit(L"\u00e0") && has_letter_or_digit(L"42"), "accents and digits count as letters");

    using P = std::pair<size_t, size_t>;
    check(stray_marks(W{L"(PLAY"}) == P{1, 0}, "a crescent glued to the word ('(PLAY') is cut");
    check(stray_marks(W{L"(Play)"}) == P{0, 0}, "matched brackets stay");
    check(stray_marks(W{L"|Quit|"}) == P{1, 1}, "bars on both ends go");
    check(stray_marks(W{L"Options", L"(more"}) == P{0, 0}, "only the line's own ends are looked at");
    check(stray_marks(W{L"Yes", L"no)"}) == P{0, 1}, "an unmatched closing bracket at the end goes");

    std::printf(failures ? "%d FAILURE(S)\n" : "ALL OK\n", failures);
    return failures ? 1 : 0;
}
