// loose.h — text the OCR left out (Hades' "· ? ? ?" line, a stylized number).
//
// A marked area is rewritten paragraph by paragraph, and a paragraph that
// grows in translation pushes the ones below it down. Something OCR never
// read stayed where it was, and the pushed text landed on top of it. Here such
// leftovers are found, so the overlay can move them along with the text (as
// pictures cut from the game) and the clean background no longer has them.
//
// A leftover is ink in the colors of the area's text (closer to some line's
// text color than to every line's background), outside every line and
// picture, within the span of the area's lines, and about the size of a line
// of text: specks, frame ornaments and scenery are left out.
#pragma once

#include <vector>

#include "common/types.h"

namespace subquest {

// Boxes (frame pixels) of the text-like things in `frame` that no line in
// `lines` and no picture in `pictures` covers.
std::vector<Rect> find_loose_text(const Frame& frame, const std::vector<OcrLine>& lines, const std::vector<Rect>& pictures);

// Whether a leftover is a mark ("· ? ? ?", a number) rather than a run of
// unread words: marks are a few glyphs wide (Hades: about 2.5 times their
// height), a run of words is much longer (the rest of a line OCR couldn't read
// over a busy background: 15 times). Only marks are worth drawing again as
// pictures; a run would land on top of the translation.
bool looks_like_mark(const Rect& leftover);

// Whether a leftover sits on the row of a line OCR read: then it is what OCR
// left out of that line (a word lost over a textured background: "number",
// "on", "Konr"), not a mark standing on its own row. Redrawn as a picture, it
// would sit on top of the translation of the line it belongs to.
bool shares_a_row(const Rect& leftover, const std::vector<OcrLine>& lines);

} // namespace subquest
