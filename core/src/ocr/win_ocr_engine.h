// win_ocr_engine.h — native Windows OCR (Windows.Media.Ocr).
#pragma once

#include "ocr/ocr_engine.h"

namespace subquest {

class WinOcrEngine final : public IOcrEngine {
public:
    bool load(const std::string& language) override;
    OcrResult recognize(const Frame& frame) override;
    const char* name() const override { return "Windows OCR"; }

private:
    std::string language_; // loaded language; empty = not loaded
};

} // namespace subquest
