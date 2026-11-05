// types.h — types shared across the core modules.
#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace subquest {

struct Rect {
    int32_t x = 0;
    int32_t y = 0;
    int32_t width = 0;
    int32_t height = 0;

    bool empty() const { return width <= 0 || height <= 0; } // true when no area was set
};

// A frame already cropped to the region of interest, BGRA 8 bits per channel.
struct Frame {
    std::vector<uint8_t> bgra; // pixels, row by row
    int32_t width = 0;
    int32_t height = 0;
    int32_t stride = 0;        // bytes per row (may be > width * 4 due to alignment)
    uint64_t id = 0;           // increasing counter, useful to measure latency
    uint64_t timestamp_us = 0; // capture time (monotonic clock)
};

struct OcrLine {
    std::string text; // UTF-8
    Rect box;         // line position inside the frame
};

struct OcrResult {
    std::vector<OcrLine> lines;
    std::string full_text;  // lines joined by '\n'
    uint64_t frame_id = 0;
    double elapsed_ms = 0;  // time spent on recognition only
    double total_ms = 0;    // from frame capture until the text was ready
};

} // namespace subquest
