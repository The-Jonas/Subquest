#include "ocr/win_ocr_engine.h"

namespace subquest {

// TODO(fase 1): OcrEngine::TryCreateFromLanguage(Language(lang)).
// Retorna false se o pacote de idioma do Windows não estiver instalado; a
// interface deve então explicar ao usuário como instalar.
bool WinOcrEngine::load(const std::string& language) {
    language_ = language;
    return false;
}

// TODO(fase 1): copiar Frame::bgra para um SoftwareBitmap (BGRA8, premultiplied),
// chamar RecognizeAsync e converter cada OcrLine (texto + retângulo) para OcrResult.
// Medir só o reconhecimento em elapsed_ms (std::chrono::steady_clock).
OcrResult WinOcrEngine::recognize(const Frame& frame) {
    OcrResult result;
    result.frame_id = frame.id;
    return result;
}

} // namespace subquest
