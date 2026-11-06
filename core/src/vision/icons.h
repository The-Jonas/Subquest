// icons.h — pictures inside a line of text (a gamepad button, a key, a coin).
//
// "Press (A) to skip": the (A) is drawn by the game, not written. OCR reads it
// as junk ("®", "(A)") or skips it, the translation loses it, and the patch
// over the line hides it. Here such pictures are found and replaced in the
// line's text by a placeholder, "{1}", that the translator keeps in place
// ("Aperte {1} para pular"); the overlay then draws the picture there.
//
// A picture is:
//  - a "word" read as short junk ("A", "®") whose pixels are mostly neither
//    the line's text color nor its background (a green button with a white A
//    on it), or a short word much taller than the line's letters (a key drawn
//    as a box). A whole word in another color is a highlighted word, not a picture;
//  - a one-character junk "word" whose box is mostly filled (a key's face with
//    its letter); two letters can be a real word ("on"), so they never count;
//  - a gap between two words, wider than a space, with something drawn in it;
//  - or something small drawn right before the first word or after the last
//    one (song notes around lyrics, a key before its label), on its own: not
//    touching a portrait, the box's frame or another line; or boxed by OCR
//    into that word ("♫Mares" read as "Mares") in a color of its own.
#pragma once

#include <cstdint>
#include <string>
#include <vector>

#include "common/types.h"

namespace subquest {

// Finds the pictures in the lines (frame pixels), puts a placeholder "{n}" in
// their place in each line's text and words, and returns their boxes: the
// first is {1}, the second {2}... Lines must have their colors sampled.
std::vector<Rect> find_icons(const Frame& frame, std::vector<OcrLine>& lines);

std::string icon_placeholder(size_t index); // "{1}" for index 0

} // namespace subquest
