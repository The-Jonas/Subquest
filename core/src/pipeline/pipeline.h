// pipeline.h — wires capture -> change detector -> OCR -> callback.
//
//   [capture thread]                                   [OCR thread]
//   frame -> crop area i -> ChangeDetector i --put i--> LatestSlots --take--> OCR -> callback(text, i)
//
// One capture covers all the areas (the smallest rectangle around them); each
// area is cropped on the CPU and has its own change detector, so a busy area
// never makes a quiet one run OCR. Translation does NOT happen here: the text
// goes to Rust through the callback.
#pragma once

#include <atomic>
#include <functional>
#include <memory>
#include <string>
#include <thread>
#include <vector>

#include "capture/capture_source.h"
#include "ocr/ocr_engine.h"
#include "pipeline/latest_slot.h"
#include "vision/change_detector.h"

namespace subquest {

struct PipelineConfig {
    uint64_t window_handle = 0;
    std::vector<Rect> areas;           // in window pixels; an empty Rect = the whole window
    std::string ocr_language = "en";
    ChangeDetectorConfig detector;
};

using TextCallback = std::function<void(const OcrResult&)>; // called on the OCR thread

enum class StartError { None, AlreadyRunning, OcrLanguage, Ocr, Capture }; // which stage kept the session from starting

class Pipeline {
public:
    Pipeline(std::unique_ptr<ICaptureSource> capture, std::unique_ptr<IOcrEngine> ocr);
    ~Pipeline();

    StartError start(const PipelineConfig& config, TextCallback on_text); // None = session running
    void stop();                                                    // idempotent; waits for the threads
    bool running() const { return running_; }
    const std::string& capture_error() const { return capture_->error(); }
    const std::string& ocr_error() const { return ocr_->error(); }

private:
    void on_frame(Frame&& frame); // runs on the capture thread: only decides and forwards, never blocks
    void ocr_loop();              // runs on the OCR thread

    std::unique_ptr<ICaptureSource> capture_;
    std::unique_ptr<IOcrEngine> ocr_;
    std::vector<Rect> areas_;              // relative to the captured rectangle (empty = all of it)
    std::vector<ChangeDetector> detectors_; // one per area, only touched on the capture thread
    LatestSlots<Frame> ocr_slots_;          // one per area
    TextCallback on_text_;
    std::thread ocr_thread_;
    std::atomic<bool> running_{false};
    std::vector<std::string> last_texts_; // one per area, only touched on the OCR thread
};

Rect bounding_rect(const std::vector<Rect>& areas); // rectangle the capture must cover (empty = whole window)

} // namespace subquest
