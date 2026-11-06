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

    // --- tidy_line: what every engine's lines go through in the pipeline ---
    {
        OcrLine line{"( PLAY", {}, {}, {}, {{"(", {100, 10, 10, 20}}, {"PLAY", {120, 10, 60, 20}}}};
        check(tidy_line(line) && line.text == "PLAY", "a lone '(' before a menu item leaves the text...");
        check(line.box.x == 120 && line.box.width == 60, "...and the box");
        check(line.words.size() == 1 && line.words[0].text == "PLAY", "...and the word list");
    }
    {
        OcrLine line{"(PLAY", {}, {}, {}, {{"(PLAY", {100, 10, 50, 20}}}};
        check(tidy_line(line) && line.text == "PLAY", "a '(' glued to the word is cut from the text...");
        check(line.box.x == 110 && line.box.width == 40, "...and its share of the word's width from the box");
        check(line.words.size() == 1 && line.words[0].text == "PLAY" && line.words[0].box.x == 110,
              "...and from the word, so it can't be picked over the game");
    }
    {
        // Japanese: no spaces between words; the engine's text is kept, not rebuilt with spaces.
        OcrLine line{"|\xe3\x81\xaf\xe3\x81\x84", {}, {}, {}, {{"|", {0, 0, 5, 20}}, {"\xe3\x81\xaf", {5, 0, 20, 20}}, {"\xe3\x81\x84", {25, 0, 20, 20}}}};
        check(tidy_line(line) && line.text == "\xe3\x81\xaf\xe3\x81\x84", "a line without spaces stays without spaces");
    }
    {
        OcrLine line{"Hello  there", {}, {}, {}, {{"Hello", {0, 0, 50, 20}}, {"there", {60, 0, 50, 20}}}};
        check(tidy_line(line) && line.text == "Hello  there", "an untouched line keeps the engine's spacing");
        check(line.box.x == 0 && line.box.width == 110, "its box surrounds all its words");
    }
    {
        OcrLine line{"... ?", {}, {}, {}, {{"...", {0, 0, 10, 20}}, {"?", {20, 0, 5, 20}}}};
        check(!tidy_line(line), "a line of symbols only is dropped");
    }
    {
        // An engine that only boxes whole lines: words get their share of the box.
        const std::vector<OcrWord> words = split_words("ab cd", {0, 0, 50, 10});
        check(words.size() == 2 && words[1].text == "cd" && words[1].box.x == 30 && words[1].box.width == 20,
              "split_words gives each word its share of the line's box");
        OcrLine line{"| Quit", {0, 0, 60, 10}};
        check(tidy_line(line) && line.text == "Quit" && line.box.x == 20, "a line without words is still tidied");
    }
    {
        OcrResult result;
        result.lines = {OcrLine{"Hi", {}, {}, {}, {{"Hi", {0, 0, 20, 10}}}}, OcrLine{"--", {}, {}, {}, {{"--", {0, 20, 20, 10}}}},
                        OcrLine{"Bye", {}, {}, {}, {{"Bye", {0, 40, 30, 10}}}}};
        tidy_lines(result);
        check(result.lines.size() == 2 && result.full_text == "Hi\nBye", "tidy_lines drops empty lines and rebuilds full_text");
    }

    {
        // Hades II item card: OCR split "Found using the | Crescent Pick | on" where the font changes.
        std::vector<OcrLine> lines{OcrLine{"Crescent Pick", {200, 102, 110, 20}, {}, {}, {{"Crescent", {200, 102, 70, 20}}, {"Pick", {275, 102, 35, 20}}}},
                                   OcrLine{"Found using the", {80, 104, 110, 17}, {}, {}, {{"Found", {80, 104, 40, 17}}}},
                                   OcrLine{"on", {318, 104, 18, 17}, {}, {}, {{"on", {318, 104, 18, 17}}}},
                                   OcrLine{"Outcroppings in Erebus", {80, 132, 200, 20}, {}, {}, {}}};
        join_split_lines(lines);
        check(lines.size() == 2, "pieces of one row are joined, the next row stays apart");
        check(!lines.empty() && lines[0].text == "Found using the Crescent Pick on" && lines[0].box.x == 80 &&
                  lines[0].box.x + lines[0].box.width == 336 && lines[0].words.size() == 4,
              "in reading order, with one box and all their words");
    }
    {
        // Two columns far apart on the same row stay two lines.
        std::vector<OcrLine> lines{OcrLine{"Gold", {20, 20, 40, 16}}, OcrLine{"120", {200, 20, 30, 16}}};
        join_split_lines(lines);
        check(lines.size() == 2, "columns far apart stay apart");
    }

    std::printf(failures ? "%d FAILURE(S)\n" : "ALL OK\n", failures);
    return failures ? 1 : 0;
}
