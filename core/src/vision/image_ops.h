// image_ops.h — operações simples de imagem, sem dependências (portáveis e testáveis).
#pragma once

#include <cstdint>
#include <vector>

#include "common/types.h"

namespace subquest {

struct RgbaImage {
    std::vector<uint8_t> pixels; // RGBA 8 bits, linhas contíguas (stride = width * 4)
    int32_t width = 0;
    int32_t height = 0;
};

RgbaImage downscale_to_rgba(const Frame& src, int32_t max_width); // reduz mantendo a proporção (nunca amplia) e troca BGRA -> RGBA

} // namespace subquest
