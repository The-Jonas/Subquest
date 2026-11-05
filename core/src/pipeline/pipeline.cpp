#include "pipeline/pipeline.h"

#include <chrono>

namespace subquest {

// Takes the concrete implementations (dependency injection makes testing easier).
Pipeline::Pipeline(std::unique_ptr<ICaptureSource> capture, std::unique_ptr<IOcrEngine> ocr)
    : capture_(std::move(capture)), ocr_(std::move(ocr)) {}

// Stops everything before being destroyed so no thread is left dangling.
Pipeline::~Pipeline() { stop(); }

// Loads OCR, starts the OCR thread and only then starts capture.
// Order matters: capture must never deliver frames to nobody.
StartError Pipeline::start(const PipelineConfig& config, TextCallback on_text) {
    if (running_) return StartError::AlreadyRunning;
    switch (ocr_->load(config.ocr_language)) {
        case OcrLoadError::None: break;
        case OcrLoadError::LanguageMissing: return StartError::OcrLanguage;
        case OcrLoadError::Platform: return StartError::Ocr;
    }

    detector_ = ChangeDetector(config.detector);
    ocr_slot_.reset();
    on_text_ = std::move(on_text);
    last_text_.clear();
    running_ = true;
    ocr_thread_ = std::thread(&Pipeline::ocr_loop, this);

    const bool ok = capture_->start(config.window_handle, config.region,
                                    [this](Frame&& f) { on_frame(std::move(f)); });
    if (!ok) {
        stop();
        return StartError::Capture;
    }
    return StartError::None;
}

// Stops capture first (no more producing), then closes the slot and waits for OCR.
void Pipeline::stop() {
    if (!running_.exchange(false)) return;
    capture_->stop();
    ocr_slot_.close();
    if (ocr_thread_.joinable()) ocr_thread_.join();
}

// Forwards to OCR only the frames the detector approves.
void Pipeline::on_frame(Frame&& frame) {
    if (!running_) return;
    if (detector_.feed(frame)) ocr_slot_.put(std::move(frame));
}

// Drains the slot until the session closes; skips empty or repeated text.
// total_ms uses the same steady clock as the capture timestamp.
void Pipeline::ocr_loop() {
    while (auto frame = ocr_slot_.take()) {
        OcrResult result = ocr_->recognize(*frame);
        const auto now_us = std::chrono::duration_cast<std::chrono::microseconds>(
                                std::chrono::steady_clock::now().time_since_epoch()).count();
        result.total_ms = double(now_us - int64_t(frame->timestamp_us)) / 1000.0;
        if (result.full_text.empty() || result.full_text == last_text_) continue;
        last_text_ = result.full_text;
        if (on_text_) on_text_(result);
    }
}

} // namespace subquest
