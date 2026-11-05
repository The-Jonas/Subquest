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

    std::printf("%s\n", failures ? "FAILED" : "ALL OK");
    return failures ? EXIT_FAILURE : EXIT_SUCCESS;
}
