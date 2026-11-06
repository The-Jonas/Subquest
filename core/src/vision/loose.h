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

} // namespace subquest
