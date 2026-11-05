// types.h — tipos compartilhados entre os módulos do núcleo.
#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace subquest {

struct Rect {
    int32_t x = 0;
    int32_t y = 0;
    int32_t width = 0;
    int32_t height = 0;

    bool empty() const { return width <= 0 || height <= 0; } // true quando a área não foi definida
};

// Um quadro já recortado na região de interesse, em BGRA 8 bits por canal.
struct Frame {
    std::vector<uint8_t> bgra; // pixels, linha a linha
    int32_t width = 0;
    int32_t height = 0;
    int32_t stride = 0;        // bytes por linha (pode ser > width * 4 por alinhamento)
    uint64_t id = 0;           // contador crescente, útil para medir latência
    uint64_t timestamp_us = 0; // instante da captura (relógio monotônico)
};

struct OcrLine {
    std::string text; // UTF-8
    Rect box;         // posição da linha dentro do quadro
};

struct OcrResult {
    std::vector<OcrLine> lines;
    std::string full_text;  // linhas unidas por '\n'
    uint64_t frame_id = 0;
    double elapsed_ms = 0;  // tempo gasto só no reconhecimento
};

} // namespace subquest
