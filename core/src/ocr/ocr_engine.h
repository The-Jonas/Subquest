// ocr_engine.h — interface for an OCR engine.
//
// Planned implementations:
//  - WinOcrEngine     -> Windows.Media.Ocr (fast, great for English)
//  - PaddleOcrEngine  -> PaddleOCR via ONNX Runtime + DirectML (Japanese, Chinese, Korean)
#pragma once

#include <string>

#include "common/types.h"

namespace subquest {

class IOcrEngine {
public:
    virtual ~IOcrEngine() = default;

    virtual bool load(const std::string& language) = 0;     // prepares the engine for a language (BCP-47 tag)
    virtual OcrResult recognize(const Frame& frame) = 0;     // recognizes the text of the whole frame
    virtual const char* name() const = 0;                     // name shown in the UI and logs
};

} // namespace subquest
