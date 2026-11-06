// line_cleanup.h — tidies the words of an OCR line before it is used.
//
// Game art often sits right next to the text (a crescent before a menu item,
// a frame corner, a cursor) and OCR reads it as a stray "(", "|" or "_" at the
// start or end of the line, either as a word of its own or glued to the first
// or last word ("(PLAY"). Those are removed, so they aren't translated
// ("(Jogar") and don't widen the line's box (which shifts the translation).
#pragma once

#include <cstddef>
#include <string>
#include <utility>
#include <vector>

namespace subquest {

bool has_letter_or_digit(const std::wstring& word); // any letter/digit (non-ASCII counts, e.g. "à", "♫")
std::pair<size_t, size_t> kept_words(const std::vector<std::wstring>& words); // [first, last) without symbol-only words at the edges
std::pair<size_t, size_t> stray_marks(const std::vector<std::wstring>& words); // characters to cut from the front of the first word and the back of the last one

} // namespace subquest
