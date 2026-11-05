// ocr_engine.h — interface for an OCR engine.
//
// Planned implementations:
//  - WinOcrEngine     -> Windows.Media.Ocr (fast, great for English)
//  - PaddleOcrEngine  -> PaddleOCR via ONNX Runtime + DirectML (Japanese, Chinese, Korean)
#pragma once

#include <string>

#include "common/types.h"

namespace subquest {

enum class OcrLoadError { None, LanguageMissing, Platform }; // why load() failed

class IOcrEngine {
public:
    virtual ~IOcrEngine() = default;

    virtual OcrLoadError load(const std::string& language) = 0; // prepares the engine for a language (BCP-47 tag)
    virtual OcrResult recognize(const Frame& frame) = 0;         // recognizes the text of the whole frame
    virtual const char* name() const = 0;                         // name shown in the UI and logs
    virtual const std::string& error() const = 0;                 // details of the last load() failure
};

} // namespace subquest
