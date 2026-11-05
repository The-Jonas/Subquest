// Minimal test for LatestSlots (one newest item per area, areas served in turn), no framework.
#include <cstdio>

#include "pipeline/latest_slot.h"

using namespace subquest;

static int failures = 0;

// Checks a condition and records a failure with a description.
static void check(bool ok, const char* what) {
    std::printf("[%s] %s\n", ok ? " ok " : "FAIL", what);
    if (!ok) ++failures;
}

int main() {
    LatestSlots<int> slots;
    slots.reset(3);

    slots.put(0, 1);
    slots.put(0, 2); // replaces 1: only the newest frame of an area waits
    slots.put(2, 7);
    auto a = slots.take();
    check(a && a->first == 0 && a->second == 2, "keeps only the newest item of an area");

    slots.put(0, 3); // area 0 is busy again...
    auto b = slots.take();
    check(b && b->first == 2 && b->second == 7, "...but the next area gets its turn first");
    auto c = slots.take();
    check(c && c->first == 0 && c->second == 3, "then the busy area again");

    slots.put(5, 9); // out of range: ignored
    slots.close();
    check(!slots.take().has_value(), "close() releases take()");

    std::printf(failures ? "%d FAILURE(S)\n" : "ALL OK\n", failures);
    return failures ? 1 : 0;
}
