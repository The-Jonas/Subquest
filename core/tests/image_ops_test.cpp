// Minimal test for downscale_to_rgba, no framework.
#include <cstdio>
#include <cstdlib>

#include "vision/image_ops.h"

using namespace subquest;

static int failures = 0;

// Checks a condition and records a failure with a description.
static void check(bool ok, const char* what) {
    std::printf("[%s] %s\n", ok ? " ok " : "FAIL", what);
    if (!ok) ++failures;
}

int main() {
    // 400x200 BGRA frame: left half pure blue, right half pure red.
    Frame f;
    f.width = 400;
    f.height = 200;
    f.stride = f.width * 4 + 64; // stride wider than the row, as happens on the GPU
    f.bgra.assign(size_t(f.stride) * f.height, 0);
    for (int y = 0; y < f.height; ++y)
        for (int x = 0; x < f.width; ++x) {
            uint8_t* px = f.bgra.data() + size_t(y) * f.stride + x * 4;
            if (x < 200) px[0] = 255; else px[2] = 255; // B ou R
            px[3] = 255;
        }

    RgbaImage img = downscale_to_rgba(f, 100);
    check(img.width == 100 && img.height == 50, "shrinks to 100x50 keeping the aspect ratio");

    const uint8_t* left = img.pixels.data() + (25 * 100 + 10) * 4;
    const uint8_t* right = img.pixels.data() + (25 * 100 + 90) * 4;
    check(left[2] == 255 && left[0] == 0, "blue (B) lands in the RGBA B channel");
    check(right[0] == 255 && right[2] == 0, "red (R) lands in the RGBA R channel");
    check(left[3] == 255, "opaque alpha");

    RgbaImage same = downscale_to_rgba(f, 4000);
    check(same.width == 400 && same.height == 200, "never enlarges");

    Frame fitted;
    check(!fit_within(f, 400, fitted), "fit_within leaves a frame that already fits");
    check(fit_within(f, 100, fitted) && fitted.width == 100 && fitted.height == 50, "fit_within shrinks the longest side");
    const uint8_t* fl = fitted.bgra.data() + (25 * 100 + 10) * 4;
    check(fl[0] == 255 && fl[2] == 0, "fit_within keeps BGRA order");

    // crop: a piece of the red half, offset inside a frame that itself starts at (10, 20).
    f.origin_x = 10;
    f.origin_y = 20;
    Frame piece;
    check(crop(f, Rect{300, 50, 50, 40}, piece) && piece.width == 50 && piece.height == 40,
          "crop takes the requested size");
    check(piece.bgra[2] == 255 && piece.bgra[0] == 0, "crop copies the right pixels (red half)");
    check(piece.origin_x == 310 && piece.origin_y == 70, "crop records its place in the window");
    check(crop(f, Rect{380, 190, 100, 100}, piece) && piece.width == 20 && piece.height == 10,
          "crop is clipped to the frame");
    check(!crop(f, Rect{500, 0, 10, 10}, piece), "crop outside the frame gives nothing");

    std::printf("%s\n", failures ? "FAILED" : "ALL OK");
    return failures ? EXIT_FAILURE : EXIT_SUCCESS;
}
