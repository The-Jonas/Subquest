// win_ocr_engine.h — native Windows OCR (Windows.Media.Ocr).
//
// WinRT types stay in the .cpp (pimpl), like in WgcCapture.
#pragma once

#include <memory>

#include "ocr/ocr_engine.h"

namespace subquest {

class WinOcrEngine final : public IOcrEngine {
public:
    WinOcrEngine();
    ~WinOcrEngine() override;

    OcrLoadError load(const std::string& language) override;
    OcrResult recognize(const Frame& frame) override;
    const char* name() const override { return "Windows OCR"; }
    const std::string& error() const override { return error_; }

private:
    struct Impl;                 // holds the WinRT OcrEngine
    std::unique_ptr<Impl> impl_;
    std::string error_;
};

} // namespace subquest
