// pipeline.h — wires capture -> change detector -> OCR -> callback.
//
//   [capture thread]                   [OCR thread]
//   frame -> ChangeDetector --put-->  LatestSlot --take--> OCR -> callback(text)
//
// Translation does NOT happen here: the text goes to Rust through the callback,
// and Rust sends the translation back via sq_overlay_set_text.
#pragma once

#include <atomic>
#include <functional>
#include <memory>
#include <string>
#include <thread>

#include "capture/capture_source.h"
#include "ocr/ocr_engine.h"
#include "pipeline/latest_slot.h"
#include "vision/change_detector.h"

namespace subquest {

struct PipelineConfig {
    uint64_t window_handle = 0;
    Rect region;
    std::string ocr_language = "en";
    ChangeDetectorConfig detector;
};

using TextCallback = std::function<void(const OcrResult&)>; // called on the OCR thread

enum class StartError { None, AlreadyRunning, Ocr, Capture }; // which stage kept the session from starting

class Pipeline {
public:
    Pipeline(std::unique_ptr<ICaptureSource> capture, std::unique_ptr<IOcrEngine> ocr);
    ~Pipeline();

    StartError start(const PipelineConfig& config, TextCallback on_text); // None = session running
    void stop();                                                    // idempotent; waits for the threads
    bool running() const { return running_; }
    const std::string& capture_error() const { return capture_->error(); }

private:
    void on_frame(Frame&& frame); // runs on the capture thread: only decides and forwards, never blocks
    void ocr_loop();              // runs on the OCR thread

    std::unique_ptr<ICaptureSource> capture_;
    std::unique_ptr<IOcrEngine> ocr_;
    ChangeDetector detector_;
    LatestSlot<Frame> ocr_slot_;
    TextCallback on_text_;
    std::thread ocr_thread_;
    std::atomic<bool> running_{false};
    std::string last_text_; // only touched on the OCR thread
};

} // namespace subquest
