// inpaint.h — the game's picture with the text taken out.
//
// The overlay draws each translation where the original line was. To hide the
// original, it used to paint a patch of the color around the line; on a
// gradient, a texture or a picture behind a see-through box that patch shows.
// Here the letters are removed from the picture itself and the hole is filled
// from what surrounds it, so the overlay can put back the game's own pixels,
// just without the letters.
//
// How:
//  1. mask: inside each line's box, pixels closer to the line's text color than
//     to its background color are letters; the mask is widened by a fifth of
//     the line height to take the anti-aliased edges, shadows and outlines too;
//  2. fill: a pyramid. The picture is halved again and again, each level's
//     pixels being the average of the known ones below them, until nothing is
//     unknown; then holes are filled from the coarser level on the way back
//     up, and smoothed. Holes are thin (strokes), so they get the colors right
//     next to them: gradients and soft textures continue through.
#pragma once

#include <cstdint>
#include <vector>

#include "common/types.h"

namespace subquest {

// A piece of the frame without its text, BGRA like Frame.
struct CleanImage {
    Rect rect;                 // where it is in the frame
    std::vector<uint8_t> bgra; // rect.width * rect.height * 4 bytes, rows packed
};

// Which pixels of `region` (frame pixels) are letters of `lines`: one byte per
// pixel, 255 = letter. Exposed for tests.
std::vector<uint8_t> text_mask(const Frame& frame, Rect region, const std::vector<OcrLine>& lines);

// Fills the masked pixels of a packed BGRA picture from the others. Exposed for tests.
void fill_holes(std::vector<uint8_t>& bgra, int32_t width, int32_t height, const std::vector<uint8_t>& mask);

// The frame without the text of `lines`: the whole frame when `whole` (a marked
// area: the translation may be laid out anywhere in it), otherwise just around
// the lines (the whole window: only what the lines cover is needed). False when
// there is nothing to clean.
bool remove_text(const Frame& frame, const std::vector<OcrLine>& lines, bool whole, CleanImage& out);

} // namespace subquest
