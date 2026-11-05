// pipeline.h — liga captura -> detector de mudança -> OCR -> callback.
//
//   [thread de captura]                 [thread de OCR]
//   quadro -> ChangeDetector --put-->  LatestSlot --take--> OCR -> callback(texto)
//
// A tradução NÃO acontece aqui: o texto vai para o Rust pelo callback, e o Rust
// devolve a tradução via sq_overlay_set_text.
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

using TextCallback = std::function<void(const OcrResult&)>; // chamado na thread de OCR

class Pipeline {
public:
    Pipeline(std::unique_ptr<ICaptureSource> capture, std::unique_ptr<IOcrEngine> ocr);
    ~Pipeline();

    bool start(const PipelineConfig& config, TextCallback on_text); // false se captura ou OCR não iniciarem
    void stop();                                                    // idempotente; espera as threads
    bool running() const { return running_; }

private:
    void on_frame(Frame&& frame); // roda na thread de captura: só decide e repassa, nunca bloqueia
    void ocr_loop();              // roda na thread de OCR

    std::unique_ptr<ICaptureSource> capture_;
    std::unique_ptr<IOcrEngine> ocr_;
    ChangeDetector detector_;
    LatestSlot<Frame> ocr_slot_;
    TextCallback on_text_;
    std::thread ocr_thread_;
    std::atomic<bool> running_{false};
    std::string last_text_; // só acessado na thread de OCR
};

} // namespace subquest
