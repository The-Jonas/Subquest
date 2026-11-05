// change_detector.h — decide QUANDO vale a pena rodar o OCR.
//
// Ideia: reduzir o quadro a uma miniatura em tons de cinza e comparar com a
// anterior. Só dispara quando o texto MUDOU em relação ao último disparo e
// depois ficou PARADO por alguns quadros (efeito máquina de escrever acabou).
#pragma once

#include <cstdint>
#include <vector>

#include "common/types.h"

namespace subquest {

struct ChangeDetectorConfig {
    int32_t thumb_width = 96;        // miniatura larga e baixa: caixas de diálogo são assim
    int32_t thumb_height = 24;
    float change_threshold = 0.02f;  // diferença média (0..1) que conta como "mudou"
    float still_threshold = 0.004f;  // diferença abaixo disso conta como "parado"
    uint32_t settle_frames = 3;      // quadros parados seguidos antes de disparar
};

class ChangeDetector {
public:
    explicit ChangeDetector(ChangeDetectorConfig config = {});

    bool feed(const Frame& frame); // true => quadro estável e diferente do último disparo: rode o OCR
    void reset();                  // esquece o histórico (nova sessão ou região trocada)

    // Exposto para testes: diferença média absoluta entre duas miniaturas (0..1).
    static float mean_abs_diff(const std::vector<uint8_t>& a, const std::vector<uint8_t>& b);

private:
    std::vector<uint8_t> make_thumbnail(const Frame& frame) const; // reduz o quadro para thumb_width x thumb_height em cinza

    ChangeDetectorConfig config_;
    std::vector<uint8_t> previous_;  // miniatura do quadro anterior
    std::vector<uint8_t> last_fired_; // miniatura do último quadro que disparou o OCR
    uint32_t still_count_ = 0;       // quadros parados seguidos
};

} // namespace subquest
