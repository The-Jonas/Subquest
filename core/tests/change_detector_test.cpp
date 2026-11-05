// Minimal ChangeDetector test, no framework: runs on any OS.
#include <cstdio>
#include <cstdlib>

#include "vision/change_detector.h"

using namespace subquest;

// Builds a uniform gray frame with a bright rectangle standing in for text.
static Frame make_frame(int w, int h, int text_width, uint8_t bg = 30) {
    Frame f;
    f.width = w;
    f.height = h;
    f.stride = w * 4;
    f.bgra.assign(size_t(f.stride) * h, bg);
    for (int y = h / 3; y < 2 * h / 3; ++y)
        for (int x = 10; x < 10 + text_width && x < w; ++x)
            for (int c = 0; c < 3; ++c) f.bgra[size_t(y) * f.stride + x * 4 + c] = 230;
    return f;
}

static int failures = 0;

// Checks a condition and records a failure with a description.
static void check(bool ok, const char* what) {
    std::printf("[%s] %s\n", ok ? " ok " : "FAIL", what);
    if (!ok) ++failures;
}

int main() {
    ChangeDetector det; // settle_frames = 3

    Frame a = make_frame(640, 120, 200);
    check(!det.feed(a), "1st frame does not fire (no history)");
    check(!det.feed(a), "2nd identical frame still does not fire");
    check(det.feed(a), "3rd identical frame fires (settled)");
    check(!det.feed(a), "4th identical frame does not fire again");

    // Typewriter effect: the text grows every frame.
    check(!det.feed(make_frame(640, 120, 260)), "growing text does not fire");
    check(!det.feed(make_frame(640, 120, 320)), "growing text does not fire (2)");
    Frame b = make_frame(640, 120, 400);
    check(!det.feed(b), "text stopped: 1st still frame");
    check(!det.feed(b), "text stopped: 2nd still frame");
    check(det.feed(b), "text stopped: 3rd still frame fires");

    // The same text coming back after a blink (e.g. a cursor) doesn't fire again.
    det.feed(make_frame(640, 120, 400, 60));
    det.feed(b); det.feed(b);
    check(!det.feed(b), "same text after a blink does not repeat OCR");

    std::printf("%s\n", failures ? "FAILED" : "ALL OK");
    return failures ? EXIT_FAILURE : EXIT_SUCCESS;
}
