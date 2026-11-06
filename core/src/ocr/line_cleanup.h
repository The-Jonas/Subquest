// line_cleanup.h — tidies the words of an OCR line before it is used.
//
// Game art often sits right next to the text (a crescent before a menu item,
// a frame corner, a cursor) and OCR reads it as a stray "(", "|" or "_" at the
// start or end of the line, either as a word of its own or glued to the first
// or last word ("(PLAY"). Those are removed, so they aren't translated
// ("(Jogar") and don't widen the line's box (which shifts the translation).
//
// This runs in the pipeline for every OCR engine, so each new engine only has
// to report what it read (see ocr_engine.h).
#pragma once

#include <cstddef>
#include <string>
#include <utility>
#include <vector>

#include "common/types.h"

namespace subquest {

bool has_letter_or_digit(const std::wstring& word); // any letter/digit (non-ASCII counts, e.g. "à", "♫")
std::pair<size_t, size_t> kept_words(const std::vector<std::wstring>& words); // [first, last) without symbol-only words at the edges
std::pair<size_t, size_t> stray_marks(const std::vector<std::wstring>& words); // characters to cut from the front of the first word and the back of the last one

std::vector<OcrWord> split_words(const std::string& text, Rect box); // words of a line read without word boxes, each given its share of the box
bool tidy_line(OcrLine& line);    // cleans one line and its words; false when nothing but symbols was left
void tidy_lines(OcrResult& result); // cleans every line, drops empty ones and rebuilds full_text

} // namespace subquest
