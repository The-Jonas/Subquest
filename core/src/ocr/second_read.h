// second_read.h — reads again, in black and white, the lines OCR could not read.
//
// Light letters over a busy background (Wesnoth's help over its map: stripes
// and foliage) come back as junk ("nuiñbeijof movêiñentpóintsewhiChdiCtàtezhow")
// or are not read at all, and the translation loses the sentence. The same
// crop turned to pure black on white by the line's text color is read well
// there (measured on a real page: "which dictate how", "simply click on
// Konrad" came back; they were junk or missing in the first reading).
//
// Only the lines with junk words are read again, and only their bad words are
// replaced (or the missing words added) from the second reading; what the
// first reading got right stays, since the black-and-white one is worse for
// other words ("many" came back "ITjany").
#pragma once

#include <string>
#include <vector>

#include "common/types.h"
#include "ocr/ocr_engine.h"

namespace subquest {

// Whether a word looks like OCR junk: very long, punctuation between letters
// ("hexoon.flathterrain;uses"), a capital after a lowercase letter ("thCeri",
// "NormallY") or letters mixed with digits ("0Ñir"). Edge punctuation is ignored.
bool is_junk_word(const std::string& word);

// Whether any word of the line looks like junk.
bool has_junk(const OcrLine& line);

// A crop of the frame in pure black (ink) and white, doubled in size, and
// where it came from. `level`: how near the text color a pixel must be to
// count as ink, as a share of the distance between the text and background
// colors (0.46 read best). Color, not brightness: light green grass behind
// white letters is as bright as they are, but far from their color.
struct Binarized {
    Frame frame;
    int32_t x = 0; // top left of the crop in the frame
    int32_t y = 0;
    int32_t scale = 1; // the crop was enlarged this many times
};
bool binarize_crop(const Frame& frame, Rect region, uint32_t text, uint32_t background, float level, Binarized& out);

// Puts the words of a second reading (frame pixels) into the line: its junk
// words are replaced by the second reading's clean words over the same
// stretch, and clean words over a stretch the first reading skipped are
// added. Words the first reading got right stay as they are.
void merge_second_read(OcrLine& line, const std::vector<OcrWord>& second);

// Reads again the lines of `result` that have junk words, and rebuilds the
// result's text. `frame` is what `result` was read from.
void refine_lines(const Frame& frame, OcrResult& result, IOcrEngine& ocr);

} // namespace subquest
