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

// A box with three lines of "words" whose widths and gaps come
// from `seed`. Two seeds give two different sentences of about the same length.
static Frame make_sentence(int w, int h, unsigned seed) {
    Frame f;
    f.width = w;
    f.height = h;
    f.stride = w * 4;
    f.bgra.assign(size_t(f.stride) * h, 30);
    unsigned state = seed;
    for (int line = 0; line < 3; ++line) {
        const int top = 20 + line * 40;
        int x = 12;
        while (x < w - 40) {
            state = state * 1103515245u + 12345u;
            const int word = 6 + int((state >> 16) % 7) * 6; // 6..42 px
            // Letters: thin strokes (2 px every 5 px), like real anti-aliased text.
            // Seen from far away every word is the same gray: only where the
            // gaps between words fall tells two sentences apart.
            for (int y = top; y < top + 16 && y < h; ++y)
                for (int i = x; i < x + word && i < w; ++i)
                    if ((i - x) % 5 < 2)
                        for (int c = 0; c < 3; ++c) f.bgra[size_t(y) * f.stride + i * 4 + c] = 230;
            x += word + 8;
        }
    }
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

    // A new sentence of about the same length in the same box (game tips,
    // dialogue): the average barely changes, but it is another text.
    ChangeDetector det2;
    Frame s1 = make_sentence(727, 160, 1);
    Frame s2 = make_sentence(727, 160, 2);
    det2.feed(s1); det2.feed(s1);
    check(det2.feed(s1), "first sentence is read");
    det2.feed(s2); det2.feed(s2);
    check(det2.feed(s2), "another sentence of the same length is read too");
    det2.feed(s2); det2.feed(s2);
    check(!det2.feed(s2), "the same sentence staying on screen is not read again");

    // The wait to settle is measured from the frame where the screen stopped changing.
    ChangeDetector det3;
    Frame t1 = make_frame(640, 120, 200);
    Frame t2 = make_frame(640, 120, 300);
    t1.timestamp_us = 1'000'000;
    det3.feed(t1); // still typing...
    t2.timestamp_us = 1'033'000; // ...the last letter appears at 1.033 s
    det3.feed(t2);
    t2.timestamp_us = 1'066'000;
    det3.feed(t2);
    t2.timestamp_us = 1'100'000;
    check(det3.feed(t2), "the text that stopped changing is read on its 3rd still frame");
    check(det3.still_since_us() == 1'033'000, "and it is known since when it stood still (67 ms of waiting)");

    std::printf("%s\n", failures ? "FAILED" : "ALL OK");
    return failures ? EXIT_FAILURE : EXIT_SUCCESS;
}
