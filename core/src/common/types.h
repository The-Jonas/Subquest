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
    int32_t origin_x = 0;      // where this crop starts in the window, in window pixels
    int32_t origin_y = 0;
    int32_t source_width = 0;  // whole window size (0 = same as width/height)
    int32_t source_height = 0;
};

struct OcrLine {
    std::string text;                   // UTF-8
    Rect box;                           // line position inside the frame
    uint32_t background_rgb = 0x000000; // color right around this line (0xRRGGBB)
    uint32_t text_rgb = 0xFFFFFF;       // color of this line's letters (0xRRGGBB)
};

struct OcrResult {
    std::vector<OcrLine> lines;
    std::string full_text;  // lines joined by '\n'
    uint64_t frame_id = 0;
    uint32_t area_index = 0; // which of the session's areas this text came from
    double elapsed_ms = 0;  // time spent on recognition only
    double total_ms = 0;    // from frame capture until the text was ready
    int32_t frame_width = 0;  // size of the captured area, so line boxes can be scaled
    int32_t frame_height = 0;
    uint32_t background_rgb = 0x000000; // dialogue box color (0xRRGGBB), sampled from the frame
    uint32_t text_rgb = 0xFFFFFF;       // text color (0xRRGGBB), sampled from the letters
};

} // namespace subquest
