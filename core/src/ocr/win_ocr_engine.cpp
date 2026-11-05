#include "ocr/win_ocr_engine.h"

namespace subquest {

// TODO(phase 1): OcrEngine::TryCreateFromLanguage(Language(lang)).
// Returns false if the Windows language pack isn't installed; the UI should
// then explain to the user how to install it.
bool WinOcrEngine::load(const std::string& language) {
    language_ = language;
    return false;
}

// TODO(phase 1): copy Frame::bgra into a SoftwareBitmap (BGRA8, premultiplied),
// call RecognizeAsync and convert each OcrLine (text + rectangle) to OcrResult.
// Measure only the recognition in elapsed_ms (std::chrono::steady_clock).
OcrResult WinOcrEngine::recognize(const Frame& frame) {
    OcrResult result;
    result.frame_id = frame.id;
    return result;
}

} // namespace subquest
