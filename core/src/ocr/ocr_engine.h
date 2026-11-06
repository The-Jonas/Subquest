// ocr_engine.h — interface for an OCR engine.
//
// An engine only reports what it read; everything after that is shared and
// lives in the pipeline: stray marks at the line edges are cut (tidy_lines),
// colors are sampled, boxes are moved to window pixels, and repeated text is
// skipped. So a new engine is just this interface plus a name in
// ocr_factory.cpp.
//
// Implementations:
//  - WinOcrEngine     -> Windows.Media.Ocr ("windows": fast, great for English)
// Planned:
//  - PaddleOcrEngine  -> PaddleOCR via ONNX Runtime ("paddle": Japanese, Chinese, Korean)
#pragma once

#include <string>

#include "common/types.h"

namespace subquest {

enum class OcrLoadError { None, LanguageMissing, Platform }; // why load() failed

class IOcrEngine {
public:
    virtual ~IOcrEngine() = default;

    // Prepares the engine for a language (BCP-47 tag). Called on the thread
    // that starts the session, before any recognize().
    virtual OcrLoadError load(const std::string& language) = 0;

    // Reads the text of the whole frame. Called on the OCR thread only, one
    // frame at a time. Returns one OcrLine per line in reading order, boxes in
    // frame pixels, with:
    //  - text: the line as the engine spaces it (or empty: words joined by spaces);
    //  - words: each word with its box, when the engine has them (otherwise
    //    the line is split on spaces and each word gets its share of the box);
    //  - elapsed_ms: time spent recognizing.
    // Other fields (full_text, colors, ids) are filled in by the pipeline.
    // Must not throw: a frame that fails yields no lines.
    virtual OcrResult recognize(const Frame& frame) = 0;

    virtual const char* name() const = 0;          // name shown in the UI and logs
    virtual const std::string& error() const = 0;  // details of the last load() failure
};

} // namespace subquest
