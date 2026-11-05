// ocr_engine.h — interface de um motor de OCR.
//
// Implementações planejadas:
//  - WinOcrEngine     -> Windows.Media.Ocr (rápido, ótimo para inglês)
//  - PaddleOcrEngine  -> PaddleOCR via ONNX Runtime + DirectML (japonês, chinês, coreano)
#pragma once

#include <string>

#include "common/types.h"

namespace subquest {

class IOcrEngine {
public:
    virtual ~IOcrEngine() = default;

    virtual bool load(const std::string& language) = 0;     // prepara o motor para um idioma (tag BCP-47)
    virtual OcrResult recognize(const Frame& frame) = 0;     // reconhece o texto do quadro inteiro
    virtual const char* name() const = 0;                     // nome exibido na interface e nos logs
};

} // namespace subquest
