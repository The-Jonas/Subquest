// Test for inpaint (taking the text out of the game's picture), no framework:
// letters drawn over a gradient must leave the gradient behind them, and
// nothing outside the letters may change.
#include <cmath>
#include <cstdio>
#include <cstdlib>

#include "vision/inpaint.h"

using namespace subquest;

static int failures = 0;

// Checks a condition and records a failure with a description.
static void check(bool ok, const char* what) {
    std::printf("[%s] %s\n", ok ? " ok " : "FAIL", what);
    if (!ok) ++failures;
}

// Background of the test picture: blue rising from left to right, red from top to bottom.
static void background(int x, int y, uint8_t* px) {
    px[0] = uint8_t(60 + x / 2);  // B
    px[1] = 40;                   // G
    px[2] = uint8_t(30 + y * 2);  // R
    px[3] = 255;
}

// A 200 x 60 frame with a gradient and, inside the line box, "letters": white
// vertical strokes 2 px wide every 6 px (like text, thin and many).
static Frame text_on_gradient(OcrLine& line) {
    Frame f;
    f.width = 200;
    f.height = 60;
    f.stride = f.width * 4;
    f.bgra.resize(size_t(f.stride) * f.height);
    line.box = {20, 20, 150, 20};
    line.text_rgb = 0xFFFFFF;
    line.background_rgb = 0x5A2896; // about the gradient's color there
    for (int y = 0; y < f.height; ++y) {
        for (int x = 0; x < f.width; ++x) {
            uint8_t* px = f.bgra.data() + size_t(y) * f.stride + size_t(x) * 4;
            background(x, y, px);
            const bool in_line = x >= 22 && x < 168 && y >= 23 && y < 37;
            if (in_line && x % 6 < 2) px[0] = px[1] = px[2] = 255;
        }
    }
    return f;
}

int main() {
    OcrLine line;
    const Frame frame = text_on_gradient(line);

    const std::vector<uint8_t> mask = text_mask(frame, {0, 0, 200, 60}, {line});
    check(mask[size_t(30) * 200 + 24] == 255, "a letter stroke is in the mask");
    check(mask[size_t(30) * 200 + 27] == 255, "pixels right next to it too (anti-aliased edges)");
    check(mask[size_t(5) * 200 + 100] == 0, "the background away from the line is not");

    CleanImage clean;
    check(remove_text(frame, {line}, true, clean), "a frame with a line can be cleaned");
    check(clean.rect.x == 0 && clean.rect.width == 200, "a marked area is cleaned whole");

    // Behind the letters: the gradient again, within a few levels.
    int worst = 0;
    for (int y = 23; y < 37; ++y) {
        for (int x = 22; x < 168; ++x) {
            uint8_t expected[4];
            background(x, y, expected);
            const uint8_t* got = clean.bgra.data() + (size_t(y) * clean.rect.width + x) * 4;
            for (int c = 0; c < 3; ++c) worst = std::max(worst, std::abs(int(got[c]) - int(expected[c])));
        }
    }
    std::printf("       (largest difference from the gradient behind the letters: %d levels)\n", worst);
    check(worst <= 6, "the gradient continues where the letters were");

    bool untouched = true;
    for (int y = 0; y < 60 && untouched; ++y) {
        for (int x = 0; x < 200; ++x) {
            if (mask[size_t(y) * 200 + x]) continue;
            const uint8_t* a = frame.bgra.data() + size_t(y) * frame.stride + size_t(x) * 4;
            const uint8_t* b = clean.bgra.data() + (size_t(y) * 200 + x) * 4;
            if (a[0] != b[0] || a[1] != b[1] || a[2] != b[2]) {
                untouched = false;
                break;
            }
        }
    }
    check(untouched, "pixels that are not letters stay exactly the game's");

    CleanImage around;
    check(remove_text(frame, {line}, false, around), "the whole window is cleaned only around the lines");
    check(around.rect.x == 0 && around.rect.y == 0 && around.rect.width == 190 && around.rect.height == 60,
          "with one line height of margin, inside the frame");

    CleanImage none;
    check(!remove_text(frame, {}, true, none), "no lines, nothing to clean");

    std::printf(failures ? "%d FAILURE(S)\n" : "ALL OK\n", failures);
    return failures ? 1 : 0;
}
