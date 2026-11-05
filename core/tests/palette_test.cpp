// Minimal palette test, no framework.
#include <cstdio>
#include <cstdlib>

#include "vision/palette.h"

using namespace subquest;

static int failures = 0;

// Checks a condition and records a failure with a description.
static void check(bool ok, const char* what) {
    std::printf("[%s] %s\n", ok ? " ok " : "FAIL", what);
    if (!ok) ++failures;
}

// Builds a dialogue-box-like frame: solid background with "letters" (thin
// vertical bars) of another color inside a text line.
static Frame make_box(uint32_t bg, uint32_t fg, Rect line) {
    Frame f;
    f.width = 400;
    f.height = 100;
    f.stride = f.width * 4;
    f.bgra.resize(size_t(f.stride) * f.height);
    for (int y = 0; y < f.height; ++y)
        for (int x = 0; x < f.width; ++x) {
            const bool letter = x >= line.x && x < line.x + line.width && y >= line.y && y < line.y + line.height &&
                                (x / 3) % 3 == 0; // ~1/3 of the line area is "ink"
            const uint32_t c = letter ? fg : bg;
            uint8_t* p = f.bgra.data() + size_t(y) * f.stride + size_t(x) * 4;
            p[0] = uint8_t(c & 0xFF);
            p[1] = uint8_t((c >> 8) & 0xFF);
            p[2] = uint8_t((c >> 16) & 0xFF);
            p[3] = 255;
        }
    return f;
}

int main() {
    const Rect line{20, 30, 300, 30};

    Frame green_on_dark = make_box(0x101418, 0x3FA34D, line);
    TextColors c = sample_colors(green_on_dark, {{"x", line}});
    check(c.background == 0x101418, "background comes from the border");
    check(c.text == 0x3FA34D, "text color is the ink inside the line");

    Frame same = make_box(0x808080, 0x828282, line);
    TextColors low = sample_colors(same, {{"x", line}});
    check(low.text == 0xFFFFFF || low.text == 0x000000, "low contrast falls back to black or white");

    Frame dark_on_light = make_box(0xE8E0C8, 0x202020, line);
    TextColors paper = sample_colors(dark_on_light, {});
    check(paper.text == 0x202020, "works without line boxes (whole frame)");

    // Menu case: one highlighted row (red band) inside a dark screen. The line's
    // own background must be the red band, not the dark screen around it.
    Frame menu = make_box(0x111111, 0xEEEEEE, line);
    for (int y = 20; y < 70; ++y)
        for (int x = 0; x < 400; ++x) {
            uint8_t* p = menu.bgra.data() + size_t(y) * menu.stride + size_t(x) * 4;
            const bool letter = x >= line.x && x < line.x + line.width && y >= line.y && y < line.y + line.height &&
                                (x / 3) % 3 == 0;
            if (!letter) { p[0] = 0x20; p[1] = 0x20; p[2] = 0xE0; } // red band (BGRA)
        }
    TextColors row = sample_line_colors(menu, line);
    check(row.background == 0xE02020, "line background comes from around that line (red band)");
    check(row.text == 0xEEEEEE, "line text color is that line's ink");

    std::printf("%s\n", failures ? "FAILED" : "ALL OK");
    return failures ? EXIT_FAILURE : EXIT_SUCCESS;
}
