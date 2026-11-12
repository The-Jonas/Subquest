// Test for second_read (lines OCR could not read are read again in black and
// white), no framework. The junk words and boxes are the ones the Windows OCR
// returned on a real Wesnoth help page, light text over a striped map.
#include <cstdio>

#include "ocr/second_read.h"

using namespace subquest;

static int failures = 0;

// Checks a condition and records a failure with a description.
static void check(bool ok, const char* what) {
    std::printf("[%s] %s\n", ok ? " ok " : "FAIL", what);
    if (!ok) ++failures;
}

// A line of words, each at a box on a row at y = 20, 16 px tall.
static OcrLine make_line(std::vector<OcrWord> words) {
    OcrLine line;
    line.words = std::move(words);
    int x0 = line.words.front().box.x, x1 = line.words.back().box.x + line.words.back().box.width;
    line.box = {x0, 20, x1 - x0, 16};
    for (const OcrWord& w : line.words) line.text += (line.text.empty() ? "" : " ") + w.text;
    return line;
}

// An OCR engine that answers the same words to any picture, and counts the readings.
class CannedOcr final : public IOcrEngine {
public:
    std::vector<OcrWord> words; // in the crop's pixels (the crop is enlarged: the caller divides)
    int readings = 0;
    int last_width = 0, last_height = 0;
    OcrLoadError load(const std::string&) override { return OcrLoadError::None; }
    OcrResult recognize(const Frame& frame) override {
        ++readings;
        last_width = frame.width, last_height = frame.height;
        OcrResult result;
        OcrLine line;
        line.words = words;
        result.lines.push_back(line);
        return result;
    }
    const char* name() const override { return "canned"; }
    const std::string& error() const override { return error_; }

private:
    std::string error_;
};

int main() {
    // What the OCR returned on the page.
    check(is_junk_word("movêiñentpóintsewhiChdiCtàtezhow"), "a 31-letter run is junk");
    check(is_junk_word("thCeri&0Ñir"), "capitals and symbols inside a word are junk");
    check(is_junk_word("NormallY,"), "a capital after a lowercase letter is junk");
    check(is_junk_word("hexoon.flathterrain;uses"), "punctuation between letters is junk");
    check(!is_junk_word("number") && !is_junk_word("Konrad,") && !is_junk_word("(You") && !is_junk_word("point."),
          "ordinary words are not");
    check(!is_junk_word("10:30") && !is_junk_word("3.5") && !is_junk_word("don't") && !is_junk_word("well-known"),
          "times, decimals, apostrophes and hyphens are not");
    check(!is_junk_word("—") && !is_junk_word("..."), "marks alone are not words");

    {
        // Light text on dark: ink is black on white, enlarged.
        Frame f;
        f.width = 40, f.height = 20, f.stride = 160;
        f.bgra.assign(size_t(f.stride) * f.height, 0x20); // dark background
        for (int y = 5; y < 10; ++y)
            for (int x = 10; x < 20; ++x) {
                uint8_t* px = f.bgra.data() + size_t(y) * f.stride + size_t(x) * 4;
                px[0] = px[1] = px[2] = 0xD0; // light letters
            }
        Binarized b;
        check(binarize_crop(f, {0, 0, 40, 20}, 0xD0D0D0, 0x202020, 0.46f, b), "a crop with contrast is cut");
        check(b.frame.width == 80 && b.frame.height == 40 && b.scale == 2, "and enlarged twice");
        const auto at = [&](int x, int y) { return b.frame.bgra[size_t(y) * b.frame.stride + size_t(x) * 4]; };
        check(at(30, 15) == 0 && at(5, 5) == 255, "letters turn black and the background white");
        Binarized flat;
        check(!binarize_crop(f, {0, 0, 40, 20}, 0x303030, 0x202020, 0.46f, flat), "no crop without contrast to cut by");
    }
    {
        // Line 3 of the page: the whole end of the line came back as one 31-letter word.
        OcrLine line = make_line({{"certain", {100, 20, 50, 16}},
                                  {"nuiñbeijof", {160, 20, 90, 16}},
                                  {"movêiñentpóintsewhiChdiCtàtezhow", {256, 20, 360, 16}}});
        const std::vector<OcrWord> second = {{"movement", {262, 20, 80, 16}},
                                             {"points.", {350, 20, 60, 16}},
                                             {"which", {420, 20, 50, 16}},
                                             {"dictate", {480, 20, 70, 16}},
                                             {"how", {560, 20, 40, 16}},
                                             {"nurnber", {162, 20, 70, 16}}}; // over a word the first reading got "right"
        merge_second_read(line, second);
        check(line.text == "certain nurnber movement points. which dictate how",
              "the junk word and its half-right neighbour are replaced by the clean ones over their stretch");
        check(line.box.x == 100 && line.box.width == 500, "the line's box follows its words");
    }
    {
        // Line 5: OCR lost "click on Konrad" between two words, over the map.
        OcrLine line = make_line({{"simply", {200, 20, 60, 16}}, {"thCeri&0Ñir", {420, 20, 120, 16}}});
        const std::vector<OcrWord> second = {{"click", {290, 20, 50, 16}},
                                             {"on", {348, 20, 24, 16}},
                                             {"Konrad", {380, 20, 70, 16}},
                                             {"(You", {470, 20, 50, 16}}};
        merge_second_read(line, second);
        check(line.text == "simply click on Konrad (You", "words over a stretch the first reading skipped are added");
    }
    {
        // Line 4 of the page: the black-and-white reading glued "Normally", "moving" and "one" with
        // stray marks ("turn..Normally„moving.one"); the first reading had "NormallY," and "movin-gone".
        OcrLine line = make_line({{"turn.", {10, 20, 40, 16}},
                                  {"NormallY,", {60, 20, 80, 16}},
                                  {"movin-gone", {150, 20, 100, 16}},
                                  {"flat", {260, 20, 40, 16}}});
        merge_second_read(line, {{"turn..Normally„moving.one", {10, 20, 240, 16}}});
        check(line.text == "turn. Normally moving. one flat", "words glued by stray marks are cut, and a hyphen next to junk is replaced");
    }
    {
        // A second reading of another row (the next line's words) adds nothing.
        OcrLine line = make_line({{"fine", {20, 20, 40, 16}}, {"NormallY,", {70, 20, 80, 16}}});
        merge_second_read(line, {{"Normally,", {72, 60, 80, 16}}});
        check(line.text == "fine NormallY,", "words of another row are left out");
    }
    {
        // Clean lines are not read again; a line with junk is read, and its text is rebuilt.
        Frame f;
        f.width = 300, f.height = 60, f.stride = 1200;
        f.bgra.assign(size_t(f.stride) * f.height, 0x20);
        for (int y = 20; y < 36; ++y)
            for (int x = 10; x < 200; x += 4) {
                uint8_t* px = f.bgra.data() + size_t(y) * f.stride + size_t(x) * 4;
                px[0] = px[1] = px[2] = 0xE0;
            }
        OcrResult result;
        result.lines.push_back(make_line({{"Hello", {10, 20, 40, 16}}}));
        result.lines.push_back(make_line({{"fine", {10, 20, 40, 16}}, {"thCeri&0Ñir", {60, 20, 120, 16}}}));
        result.full_text = "Hello\nfine thCeri&0Ñir";
        CannedOcr ocr;
        ocr.words = {{"click", {140, 12, 60, 32}}, {"here", {220, 12, 60, 32}}}; // in the enlarged crop: halved by the caller
        refine_lines(f, result, ocr);
        check(ocr.readings >= 1 && ocr.readings <= 2, "only the line with junk is read again");
        check(ocr.last_height > 16 * 2, "in an enlarged crop around the line");
        check(result.lines[0].text == "Hello", "a clean line stays");
        check(result.full_text.find("click here") != std::string::npos && result.full_text.find("thCeri") == std::string::npos,
              "the result's text is rebuilt with the second reading");
    }

    std::printf(failures ? "%d FAILURE(S)\n" : "ALL OK\n", failures);
    return failures ? 1 : 0;
}
