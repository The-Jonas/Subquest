// Tests for loose.cpp: text the OCR left out, found next to the lines it read.
#include <cstdio>

#include "vision/loose.h"

using namespace subquest;

static int failures = 0;

static void check(bool ok, const char* what) {
    if (!ok) {
        std::printf("[FAIL] %s\n", what);
        ++failures;
    }
}

static const uint32_t BG = 0x19182A, PURPLE = 0x8A7299, FRAME = 0x9AA4A8;

static Frame blank() {
    Frame f;
    f.width = 300;
    f.height = 200;
    f.stride = f.width * 4;
    f.bgra.resize(size_t(f.stride) * f.height);
    for (size_t i = 0; i < f.bgra.size(); i += 4) {
        f.bgra[i] = BG & 0xFF, f.bgra[i + 1] = (BG >> 8) & 0xFF, f.bgra[i + 2] = (BG >> 16) & 0xFF, f.bgra[i + 3] = 255;
    }
    return f;
}

static void paint(Frame& f, Rect r, uint32_t rgb) {
    for (int y = r.y; y < r.y + r.height; ++y)
        for (int x = r.x; x < r.x + r.width; ++x) {
            uint8_t* px = f.bgra.data() + size_t(y) * f.stride + size_t(x) * 4;
            px[0] = rgb & 0xFF, px[1] = (rgb >> 8) & 0xFF, px[2] = (rgb >> 16) & 0xFF;
        }
}

static OcrLine line(Rect box) {
    OcrLine l;
    l.text = "x";
    l.box = box;
    l.text_rgb = PURPLE;
    l.background_rgb = BG;
    return l;
}

int main() {
    // Hades II: two list items read, a "· ? ? ?" under them not read, the
    // flavor text read further down, and the scroll's metal frame along the left edge.
    Frame f = blank();
    const Rect a{40, 30, 200, 16}, b{40, 70, 180, 16};
    for (int x = a.x; x < a.x + a.width; x += 5) paint(f, {x, a.y, 2, a.height}, PURPLE);
    for (int x = b.x; x < b.x + b.width; x += 5) paint(f, {x, b.y, 2, b.height}, PURPLE);
    const Rect flavor{40, 170, 200, 16};
    for (int x = flavor.x; x < flavor.x + flavor.width; x += 5) paint(f, {x, flavor.y, 2, flavor.height}, PURPLE);
    paint(f, {40, 120, 3, 3}, PURPLE);                                // the bullet
    for (int k = 0; k < 3; ++k) paint(f, {50 + k * 12, 110, 6, 14}, PURPLE); // three "?"
    paint(f, {0, 0, 8, 200}, FRAME);                                  // the frame
    paint(f, {150, 150, 1, 1}, PURPLE);                               // a speck
    paint(f, {200, 105, 20, 16}, PURPLE);                             // a filled block, not letters
    for (int k = 0; k < 4; ++k) paint(f, {26 + k * 2, 150 + k * 3, 6, 3}, PURPLE); // an ornament's tip, left of the text
    paint(f, {120, 135, 30, 14}, 0x5A4A70);                           // a soft glow, faint

    const std::vector<Rect> loose = find_loose_text(f, {line(a), line(b), line(flavor)}, {});
    check(loose.size() == 1, "the unread line is found, alone (not the frame, not the speck)");
    if (!loose.empty()) {
        check(loose[0].x <= 40 && loose[0].x + loose[0].width >= 80, "it spans the bullet and the marks");
        check(loose[0].y <= 110 && loose[0].y + loose[0].height >= 124, "and their height");
    }
    check(find_loose_text(f, {line(a), line(b), line(flavor), line({38, 108, 70, 18})}, {}).empty(), "once read, it is no longer loose");

    if (failures == 0) std::printf("all loose tests passed\n");
    return failures == 0 ? 0 : 1;
}
