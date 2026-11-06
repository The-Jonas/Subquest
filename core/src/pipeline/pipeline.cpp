#include "pipeline/pipeline.h"

#include <algorithm>
#include <chrono>
#include <climits>

#include "ocr/line_cleanup.h"
#include "vision/image_ops.h"
#include "vision/palette.h"

namespace subquest {

// Id of the next OCR result, for the whole life of the process: a session that
// restarts (new areas, pause and resume) must never reuse an id, because the
// app keys its lines, translations and history by it.
static std::atomic<uint64_t> g_next_event_id{1};

// Takes the concrete implementations (dependency injection makes testing easier).
Pipeline::Pipeline(std::unique_ptr<ICaptureSource> capture, std::unique_ptr<IOcrEngine> ocr)
    : capture_(std::move(capture)), ocr_(std::move(ocr)) {}

// Stops everything before being destroyed so no thread is left dangling.
Pipeline::~Pipeline() { stop(); }

// Smallest rectangle containing every area; empty when an area wants the
// whole window (then the capture must take all of it).
Rect bounding_rect(const std::vector<Rect>& areas) {
    if (areas.empty()) return Rect{};
    int32_t x0 = INT32_MAX, y0 = INT32_MAX, x1 = INT32_MIN, y1 = INT32_MIN;
    for (const Rect& a : areas) {
        if (a.empty()) return Rect{};
        x0 = std::min(x0, a.x);
        y0 = std::min(y0, a.y);
        x1 = std::max(x1, a.x + a.width);
        y1 = std::max(y1, a.y + a.height);
    }
    return Rect{x0, y0, x1 - x0, y1 - y0};
}

// Loads OCR, starts the OCR thread and only then starts capture.
// Order matters: capture must never deliver frames to nobody.
StartError Pipeline::start(const PipelineConfig& config, TextCallback on_text) {
    if (running_) return StartError::AlreadyRunning;
    switch (ocr_->load(config.ocr_language)) {
        case OcrLoadError::None: break;
        case OcrLoadError::LanguageMissing: return StartError::OcrLanguage;
        case OcrLoadError::Platform: return StartError::Ocr;
    }

    // Areas are kept relative to the captured rectangle, which is what frames show.
    const std::vector<Rect> areas = config.areas.empty() ? std::vector<Rect>{Rect{}} : config.areas;
    const Rect captured = bounding_rect(areas);
    areas_.clear();
    for (const Rect& a : areas)
        areas_.push_back(a.empty() ? Rect{} : Rect{a.x - captured.x, a.y - captured.y, a.width, a.height});
    detectors_.assign(areas_.size(), ChangeDetector(config.detector));
    last_texts_.assign(areas_.size(), std::string());
    ocr_slots_.reset(areas_.size());
    on_text_ = std::move(on_text);
    running_ = true;
    ocr_thread_ = std::thread(&Pipeline::ocr_loop, this);

    const bool ok = capture_->start(config.window_handle, captured,
                                    [this](Frame&& f) { on_frame(std::move(f)); });
    if (!ok) {
        stop();
        return StartError::Capture;
    }
    return StartError::None;
}

// Stops capture first (no more producing), then closes the slots and waits for OCR.
void Pipeline::stop() {
    if (!running_.exchange(false)) return;
    capture_->stop();
    ocr_slots_.close();
    if (ocr_thread_.joinable()) ocr_thread_.join();
}

// Cuts each area out of the frame and forwards it to OCR when its detector approves.
void Pipeline::on_frame(Frame&& frame) {
    if (!running_) return;
    for (size_t i = 0; i < areas_.size(); ++i) {
        if (areas_[i].empty()) {
            if (detectors_[i].feed(frame)) ocr_slots_.put(i, frame); // whole capture: a copy per area
            continue;
        }
        Frame piece;
        if (crop(frame, areas_[i], piece) && detectors_[i].feed(piece)) ocr_slots_.put(i, std::move(piece));
    }
}

// Drains the slots until the session closes; tidies what the engine read and
// skips repeated text per area.
// When an area's text disappears, sends one empty result so the UI can hide
// the old translation. Line boxes leave in WINDOW pixels, so the overlay
// (which covers the whole window) can place every area's lines directly.
// total_ms uses the same steady clock as the capture timestamp.
void Pipeline::ocr_loop() {
    while (auto job = ocr_slots_.take()) {
        const size_t area = job->first;
        const Frame& frame = job->second;
        OcrResult result = ocr_->recognize(frame);
        tidy_lines(result); // the same cleanup whatever the engine
        const auto now_us = std::chrono::duration_cast<std::chrono::microseconds>(
                                std::chrono::steady_clock::now().time_since_epoch()).count();
        result.total_ms = double(now_us - int64_t(frame.timestamp_us)) / 1000.0;
        if (result.full_text == last_texts_[area]) continue; // nothing new (still the same, or still empty)
        last_texts_[area] = result.full_text;
        if (!result.full_text.empty()) {
            // Colors only for new text: cheap enough (a few thousand pixels per line).
            const TextColors colors = sample_colors(frame, result.lines);
            result.background_rgb = colors.background;
            result.text_rgb = colors.text;
            for (OcrLine& line : result.lines) {
                const TextColors c = sample_line_colors(frame, line.box);
                line.background_rgb = c.background;
                line.text_rgb = c.text;
                line.box.x += frame.origin_x; // from the crop to the window
                line.box.y += frame.origin_y;
            }
        }
        result.frame_id = g_next_event_id++; // areas share frames, so frame ids aren't unique per result
        result.area_index = uint32_t(area);
        result.frame_width = frame.source_width > 0 ? frame.source_width : frame.width;
        result.frame_height = frame.source_height > 0 ? frame.source_height : frame.height;
        result.image = &frame; // lets the app cut a picture of a line (new text styles)
        if (on_text_) on_text_(result);
    }
}

} // namespace subquest
